//! File import on the `tpt-cadence` codec foundation (spec §25 step 2).
//!
//! Decodes a container into the mono `f32` PCM the speech engine consumes:
//! channels are mixed down (equal power), and the recording metadata the
//! project model needs (sample rate, channels, duration) travels with it.
//!
//! Supported import formats: **WAV** (8/16/24/32-bit integer PCM and
//! 32/64-bit IEEE float, any channel count, `WAVE_FORMAT_EXTENSIBLE`),
//! **MP3**, **AAC** (ADTS framing), and **FLAC**. The format is detected from
//! the file's leading bytes, not its extension (spec §16, §25 step 2).

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tpt_av_cadence_aac::AacReader;
use tpt_av_cadence_core::FormatReader;
use tpt_av_cadence_flac::FlacReader;
use tpt_av_cadence_mp3::Mp3Reader;
use tpt_av_cadence_wav::WavReader;

use crate::error::TranscribeError;

/// The import formats this stage decodes. Kept in one place so the UI, CLI,
/// and error messages agree (spec §25 step 2).
pub const SUPPORTED_IMPORT_FORMATS: &str = "WAV, MP3, AAC (ADTS), FLAC";

/// A container format the import path can decode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImportFormat {
    /// RIFF/WAVE PCM or IEEE float.
    Wav,
    /// MPEG-1/2 audio layer III, or an ID3-tagged MP3 stream.
    Mp3,
    /// AAC in ADTS framing.
    Aac,
    /// Free Lossless Audio Codec.
    Flac,
}

impl ImportFormat {
    /// Identifies the format from the first bytes of a file. Returns `None`
    /// for anything unrecognised, so the caller can reject it cleanly.
    #[must_use]
    pub fn sniff(head: &[u8]) -> Option<Self> {
        if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WAVE" {
            return Some(Self::Wav);
        }
        if head.starts_with(b"fLaC") {
            return Some(Self::Flac);
        }
        // ADTS sync word (12 bits set) with the layer bits (2 bits) equal to 0.
        if head.len() >= 2 && head[0] == 0xFF && (head[1] & 0xF6) == 0xF0 {
            return Some(Self::Aac);
        }
        if head.starts_with(b"ID3") {
            return Some(Self::Mp3);
        }
        // MPEG audio sync word with a non-zero layer.
        if head.len() >= 2 && head[0] == 0xFF && (head[1] & 0xE0) == 0xE0 && (head[1] & 0x06) != 0 {
            return Some(Self::Mp3);
        }
        None
    }
}

/// Decoded, mono-downmixed audio ready for the speech engine or cleanup.
#[derive(Clone, PartialEq, Debug)]
pub struct DecodedAudio {
    /// Mono samples in `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count of the *source* file (before the mono mixdown).
    pub channels: u16,
    /// Source bit depth as reported by the container.
    pub bit_depth: u16,
    /// Duration of the decoded audio.
    pub duration: std::time::Duration,
}

impl DecodedAudio {
    /// Number of mono samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// True if the audio contains no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Reads and decodes an audio file in any [`SUPPORTED_IMPORT_FORMATS`]
/// format, mixing down to mono. The format is detected from the content.
///
/// # Errors
/// Returns [`TranscribeError::Io`] for unreadable files,
/// [`TranscribeError::UnsupportedFormat`] for unrecognised content,
/// [`TranscribeError::Codec`] for corrupt streams, [`TranscribeError::InvalidAudio`]
/// for unusable sample data, and [`TranscribeError::EmptyAudio`] for a
/// successfully decoded but empty stream.
pub fn decode_file(path: &Path) -> Result<DecodedAudio, TranscribeError> {
    let mut file = std::fs::File::open(path)?;
    let mut head = Vec::with_capacity(12);
    (&mut file).take(12).read_to_end(&mut head)?;
    let format = ImportFormat::sniff(&head).ok_or_else(|| {
        let label = path.extension().map_or_else(
            || "unknown".to_string(),
            |e| e.to_string_lossy().into_owned(),
        );
        TranscribeError::UnsupportedFormat(label, SUPPORTED_IMPORT_FORMATS)
    })?;
    file.seek(SeekFrom::Start(0))?;

    let mut reader: Box<dyn FormatReader> = match format {
        ImportFormat::Wav => Box::new(WavReader::open(Box::new(file))?),
        ImportFormat::Mp3 => Box::new(Mp3Reader::open(Box::new(file))?),
        ImportFormat::Aac => Box::new(AacReader::open(Box::new(file))?),
        ImportFormat::Flac => Box::new(FlacReader::open(Box::new(file))?),
    };
    decode_reader(reader.as_mut())
}

/// Decodes every sample from an opened reader and mixes it down to mono.
fn decode_reader(reader: &mut dyn FormatReader) -> Result<DecodedAudio, TranscribeError> {
    let decoder = reader.decoder();
    // Copy the metadata out: `info()` borrows the decoder immutably and the
    // decode loop below needs it mutably.
    let (sample_rate, channels, bit_depth) = {
        let info = decoder.info();
        (info.sample_rate, u16::max(info.channels, 1), info.bit_depth)
    };
    if sample_rate == 0 {
        return Err(TranscribeError::InvalidAudio(
            "sample rate is zero".to_string(),
        ));
    }

    let mut interleaved: Vec<f32> = Vec::new();
    let chunk_frames = 8192usize;
    let mut buf = vec![0.0_f32; chunk_frames * channels as usize];
    loop {
        let frames = decoder.decode(&mut buf)?;
        if frames == 0 {
            break;
        }
        interleaved.extend_from_slice(&buf[..frames * channels as usize]);
    }

    if interleaved.iter().any(|s| !s.is_finite()) {
        return Err(TranscribeError::InvalidAudio(
            "float samples contain NaN or infinity".to_string(),
        ));
    }

    let samples = downmix_to_mono(&interleaved, channels as usize);
    #[allow(clippy::cast_precision_loss)] // sample counts stay far below f64 precision limits
    let secs = samples.len() as f64 / f64::from(sample_rate);
    let duration = std::time::Duration::from_secs_f64(secs);

    if samples.is_empty() {
        return Err(TranscribeError::EmptyAudio);
    }

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
        bit_depth,
        duration,
    })
}

/// Equal-power mixdown: every channel contributes `1/channels`.
fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let frames = interleaved.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    #[allow(clippy::cast_precision_loss)] // channel counts are tiny
    let scale = 1.0 / channels as f32;
    for frame in 0..frames {
        let start = frame * channels;
        let sum: f32 = interleaved[start..start + channels].iter().sum();
        mono.push(sum * scale);
    }
    mono
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    /// Writes a minimal 16-bit PCM WAV so decoder tests need no binary
    /// fixtures and no encoder dependency.
    fn write_wav(path: &Path, samples: &[i16], sample_rate: u16, channels: u16) {
        let data_len = u32::try_from(samples.len() * 2).expect("data fits in a WAV chunk");
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&u32::from(sample_rate).to_le_bytes());
        wav.extend_from_slice(&(u32::from(sample_rate) * u32::from(channels) * 2).to_le_bytes());
        wav.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
        wav.extend_from_slice(&16_u16.to_le_bytes()); // bits per sample
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::File::create(path)
            .and_then(|mut f| f.write_all(&wav))
            .expect("write wav");
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tvs-decode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn decodes_mono_wav_with_correct_metadata() {
        let path = temp_path("mono.wav");
        write_wav(&path, &[0, 16384, -16384, 32767], 16_000, 1);

        let audio = decode_file(&path).expect("decodes");
        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.bit_depth, 16);
        assert_eq!(audio.len(), 4);
        assert_eq!(audio.duration, std::time::Duration::from_micros(250));
        // 16-bit PCM normalises to [-1, 1].
        assert!((audio.samples[1] - 0.5).abs() < 1e-3);
        assert!((audio.samples[2] + 0.5).abs() < 1e-3);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn stereo_is_downmixed_to_mono_average() {
        let path = temp_path("stereo.wav");
        // L = 0.5, R = -0.5 → mono 0.0; L = 1.0, R = 1.0 → mono 1.0.
        write_wav(&path, &[16_384, -16_384, 32_767, 32_767], 16_000, 2);

        let audio = decode_file(&path).expect("decodes");
        assert_eq!(audio.channels, 2, "source channel count is preserved");
        assert_eq!(audio.len(), 2, "mono mixdown halves the sample count");
        assert!(audio.samples[0].abs() < 1e-3);
        assert!((audio.samples[1] - 1.0).abs() < 1e-3);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn corrupt_and_missing_files_report_clean_errors() {
        let missing =
            decode_file(Path::new("Z:/definitely/not/here.wav")).expect_err("missing file");
        assert!(matches!(missing, TranscribeError::Io(_)));

        // Unrecognised content is rejected by sniffing before any codec runs.
        let path = temp_path("corrupt.wav");
        std::fs::write(&path, b"this is not a RIFF file at all").expect("write junk");
        let err = decode_file(&path).expect_err("unrecognised file");
        assert!(matches!(err, TranscribeError::UnsupportedFormat(_, _)));
        std::fs::remove_file(&path).ok();

        // A file that identifies as WAV but has a broken body reaches the codec.
        let truncated = temp_path("truncated.wav");
        std::fs::write(&truncated, b"RIFF\x10\0\0\0WAVEfmt ").expect("write truncated");
        let err = decode_file(&truncated).expect_err("truncated wav");
        assert!(matches!(err, TranscribeError::Codec(_)));
        std::fs::remove_file(&truncated).ok();
    }

    #[test]
    fn zero_sample_rate_is_rejected_as_invalid_audio() {
        let path = temp_path("zero-rate.wav");
        write_wav(&path, &[0, 1, 2, 3], 0, 1);
        // The WAV reader may reject a zero rate while parsing the header; the
        // explicit guard covers containers that do not. Either way it must fail.
        let err = decode_file(&path).expect_err("zero rate");
        assert!(matches!(
            err,
            TranscribeError::InvalidAudio(_) | TranscribeError::Codec(_)
        ));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn non_finite_float_samples_are_rejected() {
        let path = temp_path("nan.wav");
        // IEEE float WAV (format tag 3) with a NaN sample.
        let samples = [0.25_f32, f32::NAN, 0.5];
        let data_len = u32::try_from(samples.len() * 4).expect("data fits in a WAV chunk");
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&3_u16.to_le_bytes()); // IEEE float
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&16_000_u32.to_le_bytes());
        wav.extend_from_slice(&(16_000_u32 * 4).to_le_bytes());
        wav.extend_from_slice(&4_u16.to_le_bytes());
        wav.extend_from_slice(&32_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(&path, &wav).expect("write float wav");

        let err = decode_file(&path).expect_err("NaN sample");
        assert!(matches!(err, TranscribeError::InvalidAudio(_)));
        std::fs::remove_file(&path).ok();
    }

    #[allow(clippy::cast_precision_loss)] // test tone; indices stay well within f32 precision
    fn tone(frames: usize) -> Vec<f32> {
        (0..frames).map(|i| (i as f32 * 0.05).sin() * 0.5).collect()
    }

    #[test]
    fn sniff_detects_each_supported_format_from_content() {
        assert_eq!(
            ImportFormat::sniff(b"RIFF\0\0\0\0WAVEfmt "),
            Some(ImportFormat::Wav)
        );
        assert_eq!(
            ImportFormat::sniff(b"fLaC\0\0\0\x22"),
            Some(ImportFormat::Flac)
        );
        // ADTS: sync 0xFFF1 (layer 00, no CRC).
        assert_eq!(
            ImportFormat::sniff(&[0xFF, 0xF1, 0x50, 0x80]),
            Some(ImportFormat::Aac)
        );
        // MPEG audio layer III: sync 0xFFFB.
        assert_eq!(
            ImportFormat::sniff(&[0xFF, 0xFB, 0x90, 0x64]),
            Some(ImportFormat::Mp3)
        );
        assert_eq!(ImportFormat::sniff(b"ID3\x04\0\0"), Some(ImportFormat::Mp3));
        assert_eq!(
            ImportFormat::sniff(b"ftypM4A "),
            None,
            "mp4 container is not supported"
        );
        assert_eq!(ImportFormat::sniff(b""), None);
    }

    #[test]
    fn unrecognised_content_is_rejected_regardless_of_extension() {
        let path = temp_path("not-audio.wav");
        std::fs::write(&path, b"plain text pretending to be audio").expect("write");
        let err = decode_file(&path).expect_err("unsupported");
        assert!(matches!(err, TranscribeError::UnsupportedFormat(_, _)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn flac_file_decodes_through_the_same_entry_point() {
        use tpt_av_cadence_core::Encoder;
        use tpt_av_cadence_flac::FlacEncoder;

        let path = temp_path("tone.flac");
        let sink = std::fs::File::create(&path).expect("create");
        let mut enc = FlacEncoder::new(sink, 16_000, 1, 16).expect("encoder");
        let samples = tone(16_000);
        enc.encode(&samples).expect("encode");
        enc.finish().expect("finish");
        drop(enc);

        let audio = decode_file(&path).expect("decodes flac");
        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.len(), samples.len(), "lossless round-trip length");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn mp3_file_decodes_within_encoder_padding_tolerance() {
        use tpt_av_cadence_core::Encoder;
        use tpt_av_cadence_mp3::Mp3Encoder;

        let path = temp_path("tone.mp3");
        let sink = std::fs::File::create(&path).expect("create");
        let mut enc = Mp3Encoder::new(sink, 44_100, 1, 128).expect("encoder");
        let samples = tone(44_100);
        let mut offset = 0;
        while offset < samples.len() {
            let consumed = enc.encode(&samples[offset..]).expect("encode");
            assert!(consumed > 0, "encoder must make progress");
            offset += consumed;
        }
        enc.finish().expect("finish");
        drop(enc);

        let audio = decode_file(&path).expect("decodes mp3");
        assert_eq!(audio.sample_rate, 44_100);
        // Encoder delay and padding shift the length slightly; allow 0.1 s.
        let secs = audio.duration.as_secs_f64();
        assert!((secs - 1.0).abs() < 0.1, "decoded {secs} s");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn aac_fixture_decodes_from_real_adts_stream() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/formats/test.aac");
        let audio = decode_file(&fixture).expect("decodes aac");
        assert!(audio.sample_rate > 0);
        assert!(!audio.is_empty());
        assert!(audio.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_wav_reports_empty_audio() {
        let path = temp_path("empty.wav");
        write_wav(&path, &[], 16_000, 1);
        let err = decode_file(&path).expect_err("empty");
        assert!(matches!(err, TranscribeError::EmptyAudio));
        std::fs::remove_file(&path).ok();
    }
}
