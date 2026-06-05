//! # engine-audio
//!
//! Audio system for the RustEngine.
//!
//! Provides playback, volume control, 3D spatial audio, mixing,
//! and streaming capabilities. Built on [rodio](https://docs.rs/rodio).
//!
//! ## Architecture
//!
//! The crate is organized into four subsystems:
//!
//! - **[`audio_manager`]** — High-level playback API. Load and play sounds,
//!   control volume per-channel (SFX / Music), pause/resume/stop by handle.
//! - **[`mixer`]** — Named bus architecture. The [`AudioMixer`] manages
//!   independent volume buses (master, sfx, music, ambient, voice, ui) with
//!   mute/unmute. Effective volume = `master × bus × mute`.
//! - **[`spatial`]** — 3D spatial audio. Distance attenuation (linear, inverse,
//!   exponential), doppler shift, and stereo panning relative to an
//!   [`AudioListener`].
//! - **[`streaming`]** — Streaming playback for long audio files. Reads
//!   chunks on demand instead of loading the entire file into memory.
//!
//! ## Mixer Bus Architecture
//!
//! The [`AudioMixer`] (`mixer::AudioMixer`) organizes audio into named buses:
//!
//! ```text
//! master ─┬─ sfx
//!         ├─ music
//!         ├─ ambient
//!         ├─ voice
//!         └─ ui
//! ```
//!
//! Each bus has its own volume (0.0–1.0) and mute flag. The effective volume
//! for any bus is computed as `master_volume × bus_volume × (0 if muted)`.
//! Custom buses can be added at runtime via [`AudioMixer::add_bus`].
//!
//! ## 3D Spatial Audio Model
//!
//! The spatial module (`spatial`) implements OpenAL-style 3D audio:
//!
//! - **Distance attenuation** — Three models: `Linear`, `Inverse`, `Exponential`.
//!   Sources closer than `min_distance` play at full volume; beyond `max_distance`
//!   they are silent.
//! - **Doppler shift** — Pitch changes based on the relative velocity between
//!   listener and source along the line connecting them.
//! - **Stereo panning** — Source position projected onto the listener's right
//!   vector determines left/right channel balance.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use engine_audio::audio_manager::{AudioManager, AudioChannel};
//!
//! let mut audio = AudioManager::new().unwrap();
//! audio.play("click.ogg", AudioChannel::Sfx).unwrap();
//! audio.set_master_volume(0.8);
//! ```
//!
//! ## Error Handling
//!
//! All fallible operations return [`AudioError`]. Audio decode and playback
//! failures propagate errors rather than panicking.

pub mod audio_manager;
pub mod error;
pub mod mixer;
pub mod spatial;
pub mod streaming;

pub use error::AudioError;
