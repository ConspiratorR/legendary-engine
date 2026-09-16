//! `so_asset` — ScriptableObject asset pack/unpack helper.
//!
//! # Commands
//!
//! ```bash
//! # Scan a directory tree for *.asset and write missing .meta GUID sidecars
//! cargo run -p engine-core --bin so_asset -- pack Assets
//!
//! # List indexed assets (path, name, guid)
//! cargo run -p engine-core --bin so_asset -- list Assets
//! ```
//!
//! Corresponds to roadmap P3.5 (`so_asset pack/unpack`).

use engine_core::scriptable_asset::{AssetMeta, scan_asset_directory};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let root = args.get(1).map(String::as_str).unwrap_or("Assets");

    match cmd {
        "pack" => {
            let mut total = 0usize;
            match walk_pack(Path::new(root), 0, &mut total) {
                Ok(()) => {
                    println!("pack: ensured .meta for {total} .asset file(s) under {root}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("pack failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "list" => match walk_list(Path::new(root), 0) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("list failed: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Usage: so_asset <pack|list> [dir]");
            eprintln!("  pack  Ensure .meta sidecars for every *.asset under dir (recursive)");
            eprintln!("  list  Scan and print path\\tname\\tguid");
            ExitCode::from(2)
        }
    }
}

/// `scan_asset_directory` already calls `ensure_asset_meta` (writes missing `.meta`).
fn walk_pack(dir: &Path, depth: usize, count: &mut usize) -> Result<(), String> {
    if depth > 32 {
        return Err(format!("directory too deep: {}", dir.display()));
    }
    let found = scan_asset_directory(dir).map_err(|e| e.to_string())?;
    *count += found.len();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            walk_pack(&path, depth + 1, count)?;
        }
    }
    Ok(())
}

fn walk_list(dir: &Path, depth: usize) -> Result<(), String> {
    if depth > 32 {
        return Err(format!("directory too deep: {}", dir.display()));
    }
    let found: Vec<(std::path::PathBuf, AssetMeta)> =
        scan_asset_directory(dir).map_err(|e| e.to_string())?;
    for (path, meta) in &found {
        println!("{}\t{}\t{}", path.display(), meta.name, meta.guid);
    }
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            walk_list(&path, depth + 1)?;
        }
    }
    Ok(())
}
