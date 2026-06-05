use thiserror::Error;

/// Errors that can occur in the audio module.
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Audio device not found")]
    DeviceNotFound,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Playback error: {0}")]
    PlaybackError(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}
