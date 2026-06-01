use crate::StateCtx;

/// A discrete game state (title screen, menu, gameplay, pause, etc.).
///
/// States are managed by the [`StateStack`](crate::StateStack). Override
/// the lifecycle hooks as needed; the default implementations are no-ops.
pub trait GameState {
    /// Called when this state becomes the top of the stack.
    fn on_enter(&mut self, _: &mut StateCtx) {}
    /// Called when this state is popped from the stack.
    fn on_exit(&mut self, _: &mut StateCtx) {}
    /// Called every frame while this state is on top of the stack.
    fn update(&mut self, _: &mut StateCtx, _: f32) {}
}
