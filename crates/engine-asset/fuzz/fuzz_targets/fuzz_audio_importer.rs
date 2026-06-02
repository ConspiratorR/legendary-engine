#![no_main]
use libfuzzer_sys::fuzz_target;
use engine_asset::format::importers::AudioImporter;
use engine_asset::pipeline::{AssetImporter, ImportContext};
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    // Fuzz the audio importer with arbitrary bytes.
    // Must never panic — errors should be returned as Err.
    for ext in &["wav", "ogg", "mp3", "flac"] {
        let imp = AudioImporter;
        let path = format!("fuzz.{ext}");
        let mut ctx = ImportContext::new(Path::new(&path));
        let _ = imp.import(data, &mut ctx);
    }
});
