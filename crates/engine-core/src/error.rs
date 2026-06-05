//! Top-level engine error type that aggregates subsystem errors.

use thiserror::Error;

/// Top-level engine error aggregating all subsystem errors.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error(transparent)]
    Math(#[from] engine_math::MathError),

    #[error(transparent)]
    Ecs(#[from] engine_ecs::EcsError),

    #[error(transparent)]
    Window(#[from] engine_window::WindowError),

    #[error(transparent)]
    Input(#[from] engine_input::InputError),

    #[error(transparent)]
    Asset(#[from] engine_asset::AssetError),

    #[error(transparent)]
    Scene(#[from] engine_scene::SceneError),

    #[error(transparent)]
    Render(#[from] engine_render::RenderError),

    #[cfg(feature = "audio")]
    #[error(transparent)]
    Audio(#[from] engine_audio::AudioError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("plugin error: {0}")]
    Plugin(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("initialization failed: {0}")]
    InitFailed(String),
}
