use crate::error::AudioError;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

/// Unique handle for a playing sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle(pub u64);

/// Audio channel category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioChannel {
    /// Sound effects.
    Sfx,
    /// Background music.
    Music,
}

/// A playing sound tracked by the audio manager.
struct PlayingSound {
    sink: Sink,
    channel: AudioChannel,
}

/// Central audio manager with volume control and playback management.
///
/// Supports per-channel volume (SFX / Music) on top of a global master volume.
/// Each call to `play()` returns a [`SoundHandle`] that can be used to pause,
/// stop, or query the sound.
pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sounds: HashMap<SoundHandle, PlayingSound>,
    next_handle: u64,
    master_volume: f32,
    channel_volumes: HashMap<AudioChannel, f32>,
}

// SAFETY: OutputStream is !Send+!Sync on some platforms due to cpal's platform-specific types.
// However, `_stream` is only held to keep the audio device alive — it's never accessed after
// construction. All actual audio operations go through `stream_handle` which IS Send+Sync.
// The remaining fields (sounds, volumes) are standard Send+Sync types.
unsafe impl Send for AudioManager {}
unsafe impl Sync for AudioManager {}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| panic!("Failed to initialize audio: {e}"))
    }
}

impl AudioManager {
    pub fn new() -> Result<Self, AudioError> {
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| AudioError::StreamError(e.to_string()))?;
        Ok(Self {
            _stream,
            stream_handle,
            sounds: HashMap::new(),
            next_handle: 0,
            master_volume: 1.0,
            channel_volumes: HashMap::from([(AudioChannel::Sfx, 1.0), (AudioChannel::Music, 1.0)]),
        })
    }

    // ── Volume ────────────────────────────────────────────────────────

    /// Get the master volume (0.0 – 1.0).
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set the master volume (0.0 – 1.0).  Immediately updates all playing sounds.
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.apply_volumes();
    }

    /// Get the volume for a channel (0.0 – 1.0).
    pub fn channel_volume(&self, channel: AudioChannel) -> f32 {
        self.channel_volumes.get(&channel).copied().unwrap_or(1.0)
    }

    /// Set the volume for a channel (0.0 – 1.0).  Immediately updates affected sounds.
    pub fn set_channel_volume(&mut self, channel: AudioChannel, volume: f32) {
        self.channel_volumes.insert(channel, volume.clamp(0.0, 1.0));
        self.apply_volumes();
    }

    /// Compute the effective volume for a channel (master × channel).
    fn effective_volume(&self, channel: AudioChannel) -> f32 {
        self.master_volume * self.channel_volume(channel)
    }

    /// Re-apply volumes to all playing sinks.
    fn apply_volumes(&self) {
        for playing in self.sounds.values() {
            let vol = self.effective_volume(playing.channel);
            playing.sink.set_volume(vol);
        }
    }

    // ── Playback ──────────────────────────────────────────────────────

    /// Play an audio file on the given channel.  Returns a handle for later control.
    pub fn play(&mut self, path: &str, channel: AudioChannel) -> Result<SoundHandle, AudioError> {
        let file = File::open(path)
            .map_err(|e| AudioError::PlaybackError(format!("Cannot open '{}': {}", path, e)))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(format!("Cannot decode '{}': {}", path, e)))?;
        let converted = source.convert_samples::<f32>();
        self.play_f32_source(converted, channel)
    }

    /// Play an already-f32 source on the given channel.
    fn play_f32_source(
        &mut self,
        source: impl Source<Item = f32> + Send + 'static,
        channel: AudioChannel,
    ) -> Result<SoundHandle, AudioError> {
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(format!("Sink error: {}", e)))?;
        let vol = self.effective_volume(channel);
        sink.set_volume(vol);
        sink.append(source);

        let handle = SoundHandle(self.next_handle);
        self.next_handle += 1;
        self.sounds.insert(handle, PlayingSound { sink, channel });
        Ok(handle)
    }

    /// Play a sound with no tracking handle (fire-and-forget).
    pub fn play_detached(&self, path: &str, channel: AudioChannel) -> Result<(), AudioError> {
        let file = File::open(path)
            .map_err(|e| AudioError::PlaybackError(format!("Cannot open '{}': {}", path, e)))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(format!("Cannot decode '{}': {}", path, e)))?;
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(format!("Sink error: {}", e)))?;
        let vol = self.effective_volume(channel);
        sink.set_volume(vol);
        sink.append(source.convert_samples::<f32>());
        sink.detach();
        Ok(())
    }

    // ── Handle controls ───────────────────────────────────────────────

    /// Pause a playing sound.
    pub fn pause(&self, handle: SoundHandle) {
        if let Some(playing) = self.sounds.get(&handle) {
            playing.sink.pause();
        }
    }

    /// Resume a paused sound.
    pub fn resume(&self, handle: SoundHandle) {
        if let Some(playing) = self.sounds.get(&handle) {
            playing.sink.play();
        }
    }

    /// Stop a sound and remove it from tracking.
    pub fn stop(&mut self, handle: SoundHandle) {
        if let Some(playing) = self.sounds.remove(&handle) {
            playing.sink.stop();
        }
    }

    /// Check if a sound is still playing (not stopped and not finished).
    pub fn is_playing(&self, handle: SoundHandle) -> bool {
        self.sounds
            .get(&handle)
            .map(|p| !p.sink.is_paused() && !p.sink.empty())
            .unwrap_or(false)
    }

    /// Check if a sound is paused.
    pub fn is_paused(&self, handle: SoundHandle) -> bool {
        self.sounds
            .get(&handle)
            .map(|p| p.sink.is_paused())
            .unwrap_or(false)
    }

    /// Try to get the playback position.  Returns `None` if the sound is not tracked.
    pub fn position(&self, handle: SoundHandle) -> Option<Duration> {
        self.sounds.get(&handle).map(|p| p.sink.get_pos())
    }

    // ── Housekeeping ──────────────────────────────────────────────────

    /// Remove finished sounds from the tracking map.
    pub fn cleanup(&mut self) {
        self.sounds.retain(|_, p| !p.sink.empty());
    }

    /// Stop all sounds and clear tracking.
    pub fn stop_all(&mut self) {
        for (_, playing) in self.sounds.drain() {
            playing.sink.stop();
        }
    }

    /// Number of currently tracked sounds (including paused / finished).
    pub fn active_count(&self) -> usize {
        self.sounds.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_manager_create() {
        let audio = AudioManager::new().unwrap();
        assert!((audio.master_volume() - 1.0).abs() < 1e-6);
        assert!((audio.channel_volume(AudioChannel::Sfx) - 1.0).abs() < 1e-6);
        assert!((audio.channel_volume(AudioChannel::Music) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_volume_clamping() {
        let mut audio = AudioManager::new().unwrap();
        audio.set_master_volume(1.5);
        assert!((audio.master_volume() - 1.0).abs() < 1e-6);
        audio.set_master_volume(-0.5);
        assert!((audio.master_volume()).abs() < 1e-6);
        audio.set_master_volume(0.5);
        assert!((audio.master_volume() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_channel_volume() {
        let mut audio = AudioManager::new().unwrap();
        audio.set_channel_volume(AudioChannel::Music, 0.3);
        assert!((audio.channel_volume(AudioChannel::Music) - 0.3).abs() < 1e-6);
        assert!((audio.channel_volume(AudioChannel::Sfx) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_effective_volume() {
        let mut audio = AudioManager::new().unwrap();
        audio.set_master_volume(0.5);
        audio.set_channel_volume(AudioChannel::Sfx, 0.8);
        assert!((audio.effective_volume(AudioChannel::Sfx) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_stop_all_and_active_count() {
        let mut audio = AudioManager::new().unwrap();
        assert_eq!(audio.active_count(), 0);
        audio.stop_all();
        assert_eq!(audio.active_count(), 0);
    }

    #[test]
    fn test_handle_for_nonexistent_sound() {
        let audio = AudioManager::new().unwrap();
        let fake = SoundHandle(999);
        assert!(!audio.is_playing(fake));
        assert!(!audio.is_paused(fake));
        assert!(audio.position(fake).is_none());
    }

    #[test]
    fn test_cleanup_empty() {
        let mut audio = AudioManager::new().unwrap();
        audio.cleanup();
        assert_eq!(audio.active_count(), 0);
    }

    #[test]
    fn test_play_invalid_path() {
        let mut audio = AudioManager::new().unwrap();
        let result = audio.play("nonexistent_file_abc123.ogg", AudioChannel::Sfx);
        assert!(result.is_err(), "expected error for invalid path");
        // Verify it returns AudioError, not a panic
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("nonexistent_file_abc123.ogg"));
    }

    #[test]
    fn test_play_detached_invalid_path() {
        let audio = AudioManager::new().unwrap();
        let result = audio.play_detached("nonexistent_file_abc123.ogg", AudioChannel::Sfx);
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_nonexistent_handle() {
        let mut audio = AudioManager::new().unwrap();
        let fake = SoundHandle(999);
        // Should not panic
        audio.stop(fake);
        assert_eq!(audio.active_count(), 0);
    }

    #[test]
    fn test_pause_resume_nonexistent() {
        let audio = AudioManager::new().unwrap();
        let fake = SoundHandle(999);
        // Should not panic
        audio.pause(fake);
        audio.resume(fake);
    }

    #[test]
    fn test_channel_volume_clamping() {
        let mut audio = AudioManager::new().unwrap();
        audio.set_channel_volume(AudioChannel::Sfx, 2.0);
        assert!((audio.channel_volume(AudioChannel::Sfx) - 1.0).abs() < 1e-6);
        audio.set_channel_volume(AudioChannel::Sfx, -1.0);
        assert!(audio.channel_volume(AudioChannel::Sfx).abs() < 1e-6);
    }

    #[test]
    fn test_master_volume_zero_silences_all() {
        let mut audio = AudioManager::new().unwrap();
        audio.set_channel_volume(AudioChannel::Sfx, 0.8);
        audio.set_channel_volume(AudioChannel::Music, 0.6);
        audio.set_master_volume(0.0);
        assert!(audio.effective_volume(AudioChannel::Sfx).abs() < 1e-6);
        assert!(audio.effective_volume(AudioChannel::Music).abs() < 1e-6);
    }

    #[test]
    fn test_default_is_consistent_with_new() {
        let audio = AudioManager::default();
        assert!((audio.master_volume() - 1.0).abs() < 1e-6);
        assert_eq!(audio.active_count(), 0);
    }
}
