use engine_audio::audio_manager::{AudioChannel, AudioManager};
use engine_audio::mixer::AudioMixer;

#[test]
fn test_audio_manager_creation() {
    let audio = AudioManager::new().unwrap();
    assert!((audio.master_volume() - 1.0).abs() < 1e-6);
    assert_eq!(audio.active_count(), 0);
}

#[test]
fn test_audio_manager_volume_control() {
    let mut audio = AudioManager::new().unwrap();

    audio.set_master_volume(0.5);
    assert!((audio.master_volume() - 0.5).abs() < 1e-6);

    audio.set_master_volume(1.5);
    assert!((audio.master_volume() - 1.0).abs() < 1e-6);

    audio.set_master_volume(-0.5);
    assert!(audio.master_volume().abs() < 1e-6);
}

#[test]
fn test_audio_manager_channel_volume() {
    let mut audio = AudioManager::new().unwrap();

    audio.set_channel_volume(AudioChannel::Music, 0.3);
    assert!((audio.channel_volume(AudioChannel::Music) - 0.3).abs() < 1e-6);
    assert!((audio.channel_volume(AudioChannel::Sfx) - 1.0).abs() < 1e-6);

    audio.set_channel_volume(AudioChannel::Sfx, 0.7);
    assert!((audio.channel_volume(AudioChannel::Sfx) - 0.7).abs() < 1e-6);
}

#[test]
fn test_mixer_bus_volume() {
    let mut mixer = AudioMixer::new();

    mixer.set_bus_volume("music", 0.5);
    assert!((mixer.bus_volume("music") - 0.5).abs() < 1e-6);

    mixer.set_bus_volume("sfx", 0.8);
    assert!((mixer.bus_volume("sfx") - 0.8).abs() < 1e-6);
}

#[test]
fn test_mixer_effective_volume() {
    let mut mixer = AudioMixer::new();

    mixer.set_master_volume(0.8);
    mixer.set_bus_volume("sfx", 0.5);
    assert!((mixer.effective_volume("sfx") - 0.4).abs() < 1e-6);

    mixer.set_bus_muted("sfx", true);
    assert!(mixer.effective_volume("sfx").abs() < 1e-6);
}

#[test]
fn test_mixer_invalid_bus_handling() {
    let mixer = AudioMixer::new();

    assert!((mixer.bus_volume("nonexistent") - 1.0).abs() < 1e-6);
    assert!(!mixer.is_bus_muted("nonexistent"));
    assert!((mixer.effective_volume("nonexistent") - 1.0).abs() < 1e-6);
}

#[test]
fn test_mixer_custom_bus() {
    let mut mixer = AudioMixer::new();

    mixer.add_bus(engine_audio::mixer::AudioBus::new("combat").with_volume(0.7));
    assert!(mixer.has_bus("combat"));
    assert!((mixer.bus_volume("combat") - 0.7).abs() < 1e-6);
}
