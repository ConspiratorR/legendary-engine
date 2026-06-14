//! Editor plugin — registers the editor with the engine's plugin system
//! and hooks the editor UI into the post-update phase.

use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;

/// Plugin that registers the editor state and hooks the editor UI into
/// the post-update phase.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // EditorState is not Send+Sync (contains CommandManager with dyn Command),
        // so it cannot be inserted as a resource. The standalone editor binary
        // manages EditorState directly in main.rs instead.
        app.add_plugin(engine_terrain::plugin::TerrainPlugin);
    }
}
