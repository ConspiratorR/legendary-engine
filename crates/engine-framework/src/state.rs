use crate::StateCtx;

/// A discrete game state (title screen, menu, gameplay, pause, etc.).
///
/// States are managed by the [`StateStack`](crate::StateStack). Override
/// the lifecycle hooks as needed; the default implementations are no-ops.
///
/// # Lifecycle
///
/// ```text
///              push(B)
///   [A] ──────────────► [B, A]
///                          │
///              push(C)     │  A receives on_pause
///   [B, A] ────────────► [C, B, A]
///                          │
///              pop()       │  C receives on_exit
///   [C, B, A] ──────────► [B, A]
///                          │
///                          │  B receives on_resume
///              pop()       │
///   [B, A] ──────────────► [A]
///                          │
///                          │  A receives on_resume
/// ```
///
/// | Transition | Outgoing hook | Incoming hook |
/// |---|---|---|
/// | Push new state on top | old top → `on_pause` | new state → `on_enter` |
/// | Pop top state | popped state → `on_exit` | new top → `on_resume` |
/// | Replace top state | old top → `on_exit` | new state → `on_enter` |
pub trait GameState {
    /// Called when this state becomes the top of the stack.
    fn on_enter(&mut self, _: &mut StateCtx) {}
    /// Called when this state is popped from the stack.
    fn on_exit(&mut self, _: &mut StateCtx) {}
    /// Called when another state is pushed on top of this one.
    fn on_pause(&mut self, _: &mut StateCtx) {}
    /// Called when the state above this one is popped, making this one top again.
    fn on_resume(&mut self, _: &mut StateCtx) {}
    /// Called every frame while this state is on top of the stack.
    fn update(&mut self, _: &mut StateCtx, _: f32) {}
}
