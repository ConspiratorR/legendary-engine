# Audio System Usage

RustEngine provides audio playback through `rodio` with mixing, spatial audio, and streaming support.

## Setup

The audio system is available through `AudioManager`:

```rust
use engine_audio::audio_manager::{AudioManager, AudioChannel};

let audio = AudioManager::new();
```

## Playing Sounds

Load and play audio clips:

```rust
// Play on the SFX channel
let handle = audio.play(sfx_bytes, AudioChannel::Sfx)?;

// Play background music
let music_handle = audio.play(music_bytes, AudioChannel::Music)?;

// Play with volume control
audio.play_detached(bytes, AudioChannel::Sfx, 0.8)?;
```

## Volume Control

Control volume per-channel and globally:

```rust
// Master volume (0.0 – 1.0)
audio.set_master_volume(0.8);

// Per-channel volume
audio.set_channel_volume(AudioChannel::Music, 0.5);
audio.set_channel_volume(AudioChannel::Sfx, 1.0);

// Query current volumes
let master = audio.master_volume();
let music_vol = audio.channel_volume(AudioChannel::Music);
```

## Playback Control

```rust
audio.pause(handle)?;
audio.resume(handle)?;
audio.stop(handle)?;

if audio.is_playing(handle) {
    println!("Still playing");
}
```

## Audio Mixer

The `AudioMixer` provides bus-based mixing:

```rust
use engine_audio::mixer::AudioMixer;

let mut mixer = AudioMixer::new();
mixer.add_bus("music");
mixer.add_bus("sfx");
mixer.add_bus("voice");

mixer.set_bus_volume("music", 0.5);
mixer.set_bus_muted("sfx", true);

let effective = mixer.effective_volume("music"); // master * bus volume
```

## Spatial Audio

3D positional audio for immersive sound:

```rust
use engine_audio::spatial::{
    AudioListener, SpatialAudioSource, SpatialAudioConfig, DistanceModel,
};

// Set up the listener (usually the camera/player)
let listener = AudioListener {
    position: Vec3::new(0.0, 0.0, 0.0),
    forward: Vec3::new(0.0, 0.0, -1.0),
    up: Vec3::new(0.0, 1.0, 0.0),
    velocity: Vec3::ZERO,
};

// Create a spatial sound source
let source = SpatialAudioSource {
    position: Vec3::new(5.0, 0.0, 0.0),
    velocity: Vec3::ZERO,
    volume: 1.0,
    pitch: 1.0,
    min_distance: 1.0,
    max_distance: 50.0,
};

// Configure distance attenuation
let config = SpatialAudioConfig {
    model: DistanceModel::Inverse,
    doppler_factor: 1.0,
    speed_of_sound: 343.0,
};

// Compute volume based on distance
let vol = compute_volume(&source, &listener, &config);
```

## Streaming

Stream large audio files without loading entirely into memory:

```rust
use engine_audio::streaming::{AudioStream, StreamingConfig, AudioFormat};

// Probe file metadata
let stream = AudioStream::probe("music/background.ogg")?;
println!("Duration: {}s", stream.duration_secs());
println!("Format: {:?}", stream.format());

// Configure streaming
let config = StreamingConfig {
    format: AudioFormat::Ogg,
    sample_rate: 44100,
    channels: 2,
    buffer_size: 8192,
}.with_looping(true);
```
