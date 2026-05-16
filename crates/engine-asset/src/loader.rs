use crate::asset::Asset;
use crate::registry::Registry;

pub trait Loader {
    type AssetType: Asset;
    fn load(&self, path: &str, registry: &mut Registry);
}

pub fn load_asset<T: Asset>(registry: &mut Registry, path: &str, asset: T) {
    registry.store(path, asset);
}
