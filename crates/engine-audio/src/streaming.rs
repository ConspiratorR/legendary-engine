use std::path::Path;

/// Streaming audio source that reads from a file in chunks.
///
/// Unlike `play()` which loads the entire file into memory, streaming
/// reads small chunks on the fly — suitable for long background music.
pub struct AudioStream {
    path: String,
    format: AudioFormat,
    sample_rate: u32,
    channels: u16,
    duration_secs: f32,
}

/// Audio file format hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Ogg,
    Mp3,
    Wav,
    Flac,
}

impl AudioStream {
    /// Probe an audio file to get its format and basic metadata.
    pub fn probe(path: &str) -> Result<Self, String> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let format = match ext.as_str() {
            "ogg" => AudioFormat::Ogg,
            "mp3" => AudioFormat::Mp3,
            "wav" => AudioFormat::Wav,
            "flac" => AudioFormat::Flac,
            _ => return Err(format!("Unsupported audio format: {}", ext)),
        };

        // Estimate duration from file size (rough approximation)
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read file metadata: {}", e))?;
        let file_size = metadata.len();

        // Rough estimates by format (bytes per second)
        let bytes_per_sec = match format {
            AudioFormat::Ogg => 20_000,  // ~160kbps
            AudioFormat::Mp3 => 19_200,  // ~128kbps
            AudioFormat::Wav => 176_400, // 44100 Hz * 2 channels * 2 bytes
            AudioFormat::Flac => 70_000, // ~560kbps
        };

        let duration_secs = if bytes_per_sec > 0 {
            file_size as f32 / bytes_per_sec as f32
        } else {
            0.0
        };

        Ok(Self {
            path: path.to_string(),
            format,
            sample_rate: 44100,
            channels: 2,
            duration_secs,
        })
    }

    /// The file path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Detected format.
    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Estimated duration in seconds.
    pub fn duration_secs(&self) -> f32 {
        self.duration_secs
    }

    /// Sample rate (default 44100).
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// Configuration for streaming playback.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Buffer size in bytes (default 8192).
    pub buffer_size: usize,
    /// Number of buffers to keep filled ahead (default 3).
    pub buffer_count: usize,
    /// Whether to loop the stream.
    pub looping: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            buffer_count: 3,
            looping: false,
        }
    }
}

impl StreamingConfig {
    /// Enable or disable looping for the stream.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Set the buffer size in bytes.
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.buffer_size, 8192);
        assert_eq!(config.buffer_count, 3);
        assert!(!config.looping);
    }

    #[test]
    fn test_streaming_config_builder() {
        let config = StreamingConfig::default()
            .with_looping(true)
            .with_buffer_size(16384);
        assert!(config.looping);
        assert_eq!(config.buffer_size, 16384);
    }

    #[test]
    fn test_audio_format_detection() {
        // We can't easily test file probing without actual files,
        // but we can test the format enum
        assert_eq!(AudioFormat::Ogg, AudioFormat::Ogg);
        assert_ne!(AudioFormat::Ogg, AudioFormat::Mp3);
    }

    #[test]
    fn test_probe_nonexistent_file() {
        let result = AudioStream::probe("nonexistent.ogg");
        assert!(result.is_err());
    }

    #[test]
    fn test_probe_unsupported_format() {
        let result = AudioStream::probe("test.xyz");
        assert!(result.is_err());
    }
}
