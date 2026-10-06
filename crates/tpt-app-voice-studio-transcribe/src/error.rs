//! Errors from the transcription stage.

/// Transcription/decoding failures.
#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    /// The audio file could not be read.
    #[error("could not read audio file: {0}")]
    Io(#[from] std::io::Error),
    /// The codec foundation failed to parse or decode the file.
    #[error("audio decoding failed: {0}")]
    Codec(#[from] tpt_av_cadence_core::CadenceError),
    /// The file's container/codec is not one of the supported import
    /// formats.
    #[error("unsupported input format {0:?}; supported import formats: {1}")]
    UnsupportedFormat(String, &'static str),
    /// The speech engine failed (model missing/corrupt, inference error).
    #[error("speech engine failed: {0}")]
    Engine(#[from] tpt_voice::utils::VoiceError),
    /// The recording contained no usable audio.
    #[error("recording contains no audio samples")]
    EmptyAudio,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_actionable() {
        let err = TranscribeError::UnsupportedFormat("mp4".to_string(), "WAV, MP3, AAC, FLAC");
        assert_eq!(
            err.to_string(),
            "unsupported input format \"mp4\"; supported import formats: WAV, MP3, AAC, FLAC"
        );
        let err = TranscribeError::EmptyAudio;
        assert_eq!(err.to_string(), "recording contains no audio samples");
    }
}
