use crate::app::AppBuilder;
use crate::plugin::Plugin;
use engine_input::action::ActionMap;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActionMap::new());
    }
}
