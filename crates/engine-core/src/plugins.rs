use engine_input::action::ActionMap;
use crate::app::AppBuilder;
use crate::plugin::Plugin;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
    }
}
