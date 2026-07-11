//! Plugin for loading and running WASM mods.

use crate::mod_system::ModLoader;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Plugin that loads and manages WASM mods.
///
/// # Example
///
/// ```rust,no_run
/// use engine_core::app::AppBuilder;
/// use engine_script::ModPlugin;
///
/// let mut app = AppBuilder::new();
/// app.add_plugin(ModPlugin::new("mods"));
/// ```
pub struct ModPlugin {
    mods_dir: PathBuf,
}

impl ModPlugin {
    /// Create a new ModPlugin that loads mods from the given directory.
    pub fn new(mods_dir: impl Into<PathBuf>) -> Self {
        Self {
            mods_dir: mods_dir.into(),
        }
    }
}

impl Plugin for ModPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match ModLoader::new() {
            Ok(loader) => {
                let mut loader = loader;
                loader.add_mod_dir(self.mods_dir.clone());
                if let Err(e) = loader.load_all() {
                    log::warn!("Failed to load some mods: {e}");
                }
                log::info!(
                    "ModPlugin: loaded {} mods from {:?}",
                    loader.mods().len(),
                    self.mods_dir
                );
                app.insert_resource(Arc::new(RwLock::new(loader)));
            }
            Err(e) => {
                log::warn!("ModPlugin: failed to initialize ModLoader: {e}");
            }
        }
    }
}

/// System that runs all loaded WASM mods each frame.
pub fn mod_update_system(world: &mut engine_ecs::world::World) {
    // Extract the loader resource temporarily to avoid borrow conflicts
    let Some(loader) = world.remove_resource::<Arc<RwLock<ModLoader>>>() else {
        return;
    };

    {
        let Ok(loader) = loader.write() else {
            return;
        };
        loader.run_systems(world);
    }

    // Put the loader back
    world.insert_resource(loader);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_plugin_creation() {
        let plugin = ModPlugin::new("/tmp/test_mods");
        assert_eq!(plugin.mods_dir, std::path::PathBuf::from("/tmp/test_mods"));
    }

    #[test]
    fn test_mod_plugin_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = ModPlugin::new(dir.path());
        let mut app = AppBuilder::new();
        // Should not panic, just log a warning
        plugin.build(&mut app);
    }
}
