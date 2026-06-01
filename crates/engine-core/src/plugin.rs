use crate::app::AppBuilder;

/// Trait for engine plugins.
///
/// A plugin receives mutable access to the [`AppBuilder`] during
/// initialization and can register systems, resources, and hooks.
pub trait Plugin {
    /// Configure the application by adding systems, resources, and hooks.
    fn build(&self, app: &mut AppBuilder);
}
