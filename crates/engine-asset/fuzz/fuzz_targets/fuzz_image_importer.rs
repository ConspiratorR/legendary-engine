#![no_main]
use libfuzzer_sys::fuzz_target;
use engine_asset::format::importers::ImageImporter;
use engine_asset::pipeline::{AssetImporter, ImportContext};

fuzz_target!(|data: &[u8]| {
    // Fuzz the image importer with arbitrary bytes.
    // Must never panic — errors should be returned as Err.
    let imp = ImageImporter;
    let mut ctx = ImportContext::new("fuzz.png");
    let _ = imp.import(data, &mut ctx);

    // Also test with different extensions
    let mut ctx = ImportContext::new("fuzz.jpg");
    let _ = imp.import(data, &mut ctx);

    let mut ctx = ImportContext::new("fuzz.bmp");
    let _ = imp.import(data, &mut ctx);

    let mut ctx = ImportContext::new("fuzz.tga");
    let _ = imp.import(data, &mut ctx);
});
