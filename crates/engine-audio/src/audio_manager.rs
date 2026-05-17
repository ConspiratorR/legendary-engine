use rodio::{OutputStream, OutputStreamHandle, Source};

pub struct AudioManager {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioManager {
    pub fn new() -> Self {
        let (_stream, _stream_handle) =
            OutputStream::try_default().expect("Failed to initialize audio output");
        Self {
            _stream,
            _stream_handle,
        }
    }

    pub fn play(&self, path: &str) -> Result<(), String> {
        let file =
            std::fs::File::open(path).map_err(|e| format!("Cannot open '{}': {}", path, e))?;
        let source = rodio::Decoder::new(std::io::BufReader::new(file))
            .map_err(|e| format!("Cannot decode '{}': {}", path, e))?;
        self._stream_handle
            .play_raw(source.convert_samples())
            .map_err(|e| format!("Playback error: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::audio_manager::AudioManager;

    #[test]
    fn test_audio_manager_create() {
        let _audio = AudioManager::new();
    }
}
