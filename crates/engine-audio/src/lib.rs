//! # engine-audio
//!
//! Audio system for the RustEngine.
//!
//! Provides playback, volume control, 3D spatial audio, mixing,
//! and streaming capabilities. Built on rodio.
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

pub mod audio_manager;
pub mod error;
pub mod mixer;
pub mod spatial;
pub mod streaming;

pub use error::AudioError;
