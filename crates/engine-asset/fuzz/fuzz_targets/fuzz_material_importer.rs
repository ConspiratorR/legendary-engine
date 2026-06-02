#![no_main]
use libfuzzer_sys::fuzz_target;
use engine_asset::format::importers::{MaterialImporter, ScriptImporter};
use engine_asset::pipeline::{AssetImporter, ImportContext};
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    // Fuzz the material importer with arbitrary bytes
    let imp = MaterialImporter;
    let mut ctx = ImportContext::new("fuzz.mat");
    let _ = imp.import(data, &mut ctx);

    // Fuzz the script importer with arbitrary bytes
    for ext in &["lua", "rs", "py"] {
        let imp = ScriptImporter;
        let path = format!("fuzz.{ext}");
        let mut ctx = ImportContext::new(Path::new(&path));
        let _ = imp.import(data, &mut ctx);
    }
});
