use crate::{FrameworkResource, StateStack};
use engine_core::app::{App, AppBuilder};
use engine_core::plugin::Plugin;

/// Plugin that registers the [`StateStack`] and [`FrameworkResource`]
/// and hooks state updates into the engine lifecycle.
pub struct FrameworkPlugin;

impl Plugin for FrameworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.resources_mut().insert(StateStack::new());
        app.resources_mut().insert(FrameworkResource::new());
        app.add_pre_update_hook(Box::new(|app: &mut App| {
            let App {
                world, resources, ..
            } = app;
            if let Some(fw) = resources.get_mut::<FrameworkResource>() {
                fw.frame_count += 1;
            }
            let stack_ptr = resources
                .get_mut::<StateStack>()
                .map(|s| s as *mut StateStack);
            if let Some(stack_ptr) = stack_ptr {
                let stack = unsafe { &mut *stack_ptr };
                stack.update_top(world, resources, 0.016);
            }
        }));
        app.add_post_update_hook(Box::new(|app: &mut App| {
            let App {
                world, resources, ..
            } = app;
            let stack_ptr = resources
                .get_mut::<StateStack>()
                .map(|s| s as *mut StateStack);
            if let Some(stack_ptr) = stack_ptr {
                let stack = unsafe { &mut *stack_ptr };
                stack.flush(world, resources);
            }
        }));
    }
}
