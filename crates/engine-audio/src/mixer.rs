use std::collections::HashMap;

/// A named audio bus (group) with its own volume control.
///
/// Buses allow organizing sounds into categories (e.g., "ambient", "voice", "ui")
/// with independent volume control on top of the global master volume.
#[derive(Debug, Clone)]
pub struct AudioBus {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

impl AudioBus {
    /// Create a new bus with the given name and default volume (1.0).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            muted: false,
        }
    }

    /// Set the initial volume (0.0 – 1.0) using the builder pattern.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// Audio mixer managing named buses with independent volume control.
///
/// The effective volume for a bus is: `master_volume × bus_volume × (0.0 if muted)`.
pub struct AudioMixer {
    master_volume: f32,
    buses: HashMap<String, AudioBus>,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMixer {
    /// Create a new mixer with default buses: master, sfx, music, ambient, voice, ui.
    pub fn new() -> Self {
        let mut mixer = Self {
            master_volume: 1.0,
            buses: HashMap::new(),
        };
        // Create default buses
        mixer.add_bus(AudioBus::new("master"));
        mixer.add_bus(AudioBus::new("sfx"));
        mixer.add_bus(AudioBus::new("music"));
        mixer.add_bus(AudioBus::new("ambient"));
        mixer.add_bus(AudioBus::new("voice"));
        mixer.add_bus(AudioBus::new("ui"));
        mixer
    }

    /// Get or create a bus by name.
    pub fn add_bus(&mut self, bus: AudioBus) {
        self.buses.insert(bus.name.clone(), bus);
    }

    /// Remove a bus by name.
    pub fn remove_bus(&mut self, name: &str) -> Option<AudioBus> {
        self.buses.remove(name)
    }

    /// Check if a bus exists.
    pub fn has_bus(&self, name: &str) -> bool {
        self.buses.contains_key(name)
    }

    /// List all bus names.
    pub fn bus_names(&self) -> Vec<&String> {
        self.buses.keys().collect()
    }

    /// Get the master volume (0.0 – 1.0).
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set the master volume (0.0 – 1.0).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Get the volume for a bus.
    pub fn bus_volume(&self, name: &str) -> f32 {
        self.buses.get(name).map(|b| b.volume).unwrap_or(1.0)
    }

    /// Set the volume for a bus.
    pub fn set_bus_volume(&mut self, name: &str, volume: f32) {
        if let Some(bus) = self.buses.get_mut(name) {
            bus.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// Mute or unmute a bus.
    pub fn set_bus_muted(&mut self, name: &str, muted: bool) {
        if let Some(bus) = self.buses.get_mut(name) {
            bus.muted = muted;
        }
    }

    /// Check if a bus is muted.
    pub fn is_bus_muted(&self, name: &str) -> bool {
        self.buses.get(name).map(|b| b.muted).unwrap_or(false)
    }

    /// Compute the effective volume for a bus: master × bus × mute.
    pub fn effective_volume(&self, name: &str) -> f32 {
        let bus_vol = self.bus_volume(name);
        let muted = self.is_bus_muted(name);
        if muted {
            0.0
        } else {
            self.master_volume * bus_vol
        }
    }

    /// Mute all buses.
    pub fn mute_all(&mut self) {
        for bus in self.buses.values_mut() {
            bus.muted = true;
        }
    }

    /// Unmute all buses.
    pub fn unmute_all(&mut self) {
        for bus in self.buses.values_mut() {
            bus.muted = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_default_buses() {
        let mixer = AudioMixer::new();
        assert!(mixer.has_bus("master"));
        assert!(mixer.has_bus("sfx"));
        assert!(mixer.has_bus("music"));
        assert!(mixer.has_bus("ambient"));
        assert!(mixer.has_bus("voice"));
        assert!(mixer.has_bus("ui"));
    }

    #[test]
    fn test_mixer_bus_volume() {
        let mut mixer = AudioMixer::new();
        mixer.set_bus_volume("music", 0.5);
        assert!((mixer.bus_volume("music") - 0.5).abs() < 1e-6);
        assert!((mixer.bus_volume("sfx") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mixer_effective_volume() {
        let mut mixer = AudioMixer::new();
        mixer.set_master_volume(0.8);
        mixer.set_bus_volume("sfx", 0.5);
        assert!((mixer.effective_volume("sfx") - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_mixer_mute_bus() {
        let mut mixer = AudioMixer::new();
        mixer.set_bus_muted("music", true);
        assert!(mixer.is_bus_muted("music"));
        assert!((mixer.effective_volume("music")).abs() < 1e-6);
    }

    #[test]
    fn test_mixer_mute_unmute_all() {
        let mut mixer = AudioMixer::new();
        mixer.mute_all();
        assert!(mixer.is_bus_muted("sfx"));
        assert!(mixer.is_bus_muted("music"));
        mixer.unmute_all();
        assert!(!mixer.is_bus_muted("sfx"));
        assert!(!mixer.is_bus_muted("music"));
    }

    #[test]
    fn test_mixer_custom_bus() {
        let mut mixer = AudioMixer::new();
        mixer.add_bus(AudioBus::new("combat").with_volume(0.7));
        assert!(mixer.has_bus("combat"));
        assert!((mixer.bus_volume("combat") - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_mixer_remove_bus() {
        let mut mixer = AudioMixer::new();
        mixer.add_bus(AudioBus::new("temp"));
        assert!(mixer.has_bus("temp"));
        mixer.remove_bus("temp");
        assert!(!mixer.has_bus("temp"));
    }

    #[test]
    fn test_mixer_bus_names() {
        let mixer = AudioMixer::new();
        let names = mixer.bus_names();
        assert!(names.len() >= 6); // master, sfx, music, ambient, voice, ui
    }

    #[test]
    fn test_mixer_nonexistent_bus() {
        let mixer = AudioMixer::new();
        assert!((mixer.bus_volume("nonexistent") - 1.0).abs() < 1e-6);
        assert!(!mixer.is_bus_muted("nonexistent"));
        assert!((mixer.effective_volume("nonexistent") - 1.0).abs() < 1e-6);
    }
}
