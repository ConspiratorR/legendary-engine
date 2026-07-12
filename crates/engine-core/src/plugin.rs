use crate::app::AppBuilder;

/// Trait for engine plugins.
///
/// A plugin receives mutable access to the [`AppBuilder`] during
/// initialization and can register systems, resources, and hooks.
pub trait Plugin: Send + Sync {
    /// Configure the application by adding systems, resources, and hooks.
    fn build(&self, app: &mut AppBuilder);

    /// Get the plugin name (for debugging).
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Blanket implementation for closures.
impl<F: Fn(&mut AppBuilder) + Send + Sync> Plugin for F {
    fn build(&self, app: &mut AppBuilder) {
        self(app);
    }
}
