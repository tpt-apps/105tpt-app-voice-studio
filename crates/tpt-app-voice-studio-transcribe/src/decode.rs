//! File import on the `tpt-cadence` codec foundation (spec §25 step 2).
//!
//! Decodes a container into the mono `f32` PCM the speech engine consumes:
//! channels are mixed down (equal power), and the recording metadata the
//! project model needs (sample rate, channels, duration) travels with it.
//!
//! Supported import formats today: **WAV** (8/16/24/32-bit integer PCM and
//! 32/64-bit IEEE float, any channel count, `WAVE_FORMAT_EXTENSIBLE`).
//! MP3/AAC/FLAC decode lands with the remaining codec crates; the format
//! list is the single place callers enumerate (spec §25 step 2).

use std::path::Path;

use tpt_av_cadence_core::FormatReader;
use tpt_av_cadence_wav::WavReader;

use crate::error::TranscribeError;

/// The import formats this stage currently decodes. Kept in one place so
/// the UI, CLI, and error messages agree (spec §25 step 2).
pub const SUPPORTED_IMPORT_FORMATS: &str = "WAV";

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

/// Reads and decodes a WAV file, mixing down to mono.
///
/// # Errors
/// Returns [`TranscribeError::Io`] for unreadable files,
/// [`TranscribeError::Codec`] for corrupt/unsupported WAV content, and
/// [`TranscribeError::EmptyAudio`] for a successfully decoded but empty
/// stream.
pub fn decode_wav_file(path: &Path) -> Result<DecodedAudio, TranscribeError> {
    let file = std::fs::File::open(path)?;
    let mut reader = WavReader::open(Box::new(file))?;
    let decoder = reader.decoder();
    // Copy the metadata out: `info()` borrows the decoder immutably and the
    // decode loop below needs it mutably.
    let (sample_rate, channels, bit_depth) = {
        let info = decoder.info();
        (info.sample_rate, u16::max(info.channels, 1), info.bit_depth)
    };

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

    let samples = downmix_to_mono(&interleaved, channels as usize);
    let duration = if sample_rate == 0 {
        std::time::Duration::ZERO
    } else {
        #[allow(clippy::cast_precision_loss)] // sample counts stay far below f64 precision limits
        let secs = samples.len() as f64 / f64::from(sample_rate);
        std::time::Duration::from_secs_f64(secs)
    };

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

        let audio = decode_wav_file(&path).expect("decodes");
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

        let audio = decode_wav_file(&path).expect("decodes");
        assert_eq!(audio.channels, 2, "source channel count is preserved");
        assert_eq!(audio.len(), 2, "mono mixdown halves the sample count");
        assert!(audio.samples[0].abs() < 1e-3);
        assert!((audio.samples[1] - 1.0).abs() < 1e-3);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn corrupt_and_missing_files_report_clean_errors() {
        let missing =
            decode_wav_file(Path::new("Z:/definitely/not/here.wav")).expect_err("missing file");
        assert!(matches!(missing, TranscribeError::Io(_)));

        let path = temp_path("corrupt.wav");
        std::fs::write(&path, b"this is not a RIFF file at all").expect("write junk");
        let err = decode_wav_file(&path).expect_err("corrupt file");
        assert!(matches!(err, TranscribeError::Codec(_)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn empty_wav_reports_empty_audio() {
        let path = temp_path("empty.wav");
        write_wav(&path, &[], 16_000, 1);
        let err = decode_wav_file(&path).expect_err("empty");
        assert!(matches!(err, TranscribeError::EmptyAudio));
        std::fs::remove_file(&path).ok();
    }
}
