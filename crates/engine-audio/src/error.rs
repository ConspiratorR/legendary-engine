use thiserror::Error;

/// Errors that can occur in the audio module.
#[derive(Error, Debug)]
pub enum AudioError {
    /// Failed to decode an audio file.
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    /// No audio output device was found.
    #[error("Audio device not found")]
    DeviceNotFound,

    /// The audio file format is not supported.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// An error occurred during playback.
    #[error("Playback error: {0}")]
    PlaybackError(String),

    /// Failed to open or manage the audio output stream.
    #[error("Stream error: {0}")]
    StreamError(String),
}
