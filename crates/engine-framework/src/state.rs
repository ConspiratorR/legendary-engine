use crate::StateCtx;

pub trait GameState {
    fn on_enter(&mut self, _: &mut StateCtx) {}
    fn on_exit(&mut self, _: &mut StateCtx) {}
    fn update(&mut self, _: &mut StateCtx, _: f32) {}
}
