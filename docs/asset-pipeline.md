# Asset Pipeline

RustEngine includes a complete asset pipeline for loading, managing, and serializing game assets.

## Overview

The asset pipeline consists of:

1. **Asset Store** — Central registry for all loaded assets
2. **Asset Handles** — `Arc`-based reference counting for automatic cleanup
3. **Asset Loaders** — File format-specific loading code
4. **Asset Meta** — `.meta` files with GUID and import settings
5. **File Watcher** — Hot-reload support for development

## Asset Types

| Type | Extensions | Description |
|------|-----------|-------------|
| **Texture** | `.png`, `.jpg`, `.dds` | 2D images and textures |
| **Mesh** | `.gltf`, `.glb` | 3D models with geometry |
| **Audio** | `.wav`, `.ogg`, `.mp3`, `.flac` | Sound files |
| **Scene** | `.json` | Scene files with entity hierarchy |
| **Prefab** | `.json` | Reusable scene templates |
| **Material** | `.json` | PBR material definitions |
| **Script** | `.lua` | Lua scripts |

## Asset Store

The `AssetStore` manages all loaded assets:

```rust
use engine_asset::store::AssetStore;

let mut store = AssetStore::new();

// Load an asset
let texture = store.load_texture("assets/textures/player.png")?;
let mesh = store.load_mesh("assets/models/character.gltf")?;
let audio = store.load_audio("assets/sounds/jump.wav")?;

// Get an asset by handle
if let Some(tex) = store.get_texture(&texture) {
    println!("Texture size: {}x{}", tex.width, tex.height);
}
```

## Asset Handles

Assets are referenced by handles (`Arc`-based):

```rust
use engine_asset::handle::AssetHandle;

// Handle automatically cleans up when dropped
let handle: AssetHandle<Texture> = store.load_texture("path/to/texture.png")?;

// Clone handle (increments reference count)
let handle2 = handle.clone();

// Drop handle (decrements reference count)
drop(handle2);
```

## Asset Meta Files

Each asset can have a companion `.meta` file:

```json
{
  "guid": "a1b2c3d4e5f6",
  "source_path": "assets/textures/player.png",
  "content_hash": "sha256:...",
  "modified_at_secs": 1623456789,
  "dependencies": [],
  "import_settings": {
    "Texture": {
      "max_size": 2048,
      "generate_mipmaps": true,
      "compression": "BC3"
    }
  }
}
```

### Import Settings

Different asset types have different import settings:

**Texture:**
- `max_size` — Maximum texture dimension
- `generate_mipmaps` — Generate mipmaps for LOD
- `compression` — Compression format (BC1-BC7, ASTC, etc.)

**Mesh:**
- `generate_lod` — Generate level-of-detail meshes
- `scale` — Import scale factor

**Audio:**
- `sample_rate` — Target sample rate
- `streaming` — Stream from disk vs load into memory

## File Watcher

The file watcher detects changes and triggers hot-reload:

```rust
use engine_asset::watcher::AssetWatcher;

let watcher = AssetWatcher::new("assets/")?;
watcher.on_change(|path| {
    println!("Asset changed: {}", path.display());
    // Reload the asset
});
```

## Asset Loading Pipeline

1. **Scan** — File watcher detects new/changed files
2. **Meta** — Load or create `.meta` file
3. **Import** — Apply import settings
4. **Load** — Parse file format
5. **Upload** — Send to GPU (textures, meshes)
6. **Register** — Add to asset store

## Best Practices

1. **Use relative paths** — Assets should be relative to the project root
2. **Commit `.meta` files** — They contain import settings and GUIDs
3. **Organize by type** — `assets/textures/`, `assets/models/`, etc.
4. **Use meaningful names** — `player_idle.png` not `img001.png`
5. **Optimize assets** — Compress textures, reduce polygon counts

## Example: Loading a 3D Model

```rust
use engine_asset::gltf::GltfImporter;

// Load glTF model
let model = GltfImporter::load("assets/models/character.gltf")?;

// Create entity with mesh
let entity = world.spawn();
world.add_component(entity, Transform::default());
world.add_component(entity, MeshRenderer {
    mesh: model.meshes[0].clone(),
    material: model.materials[0].clone(),
});
```

## Example: Loading Audio

```rust
use engine_audio::audio_manager::AudioManager;

let audio = AudioManager::new();

// Load and play sound
let handle = audio.play("assets/sounds/jump.wav", AudioChannel::Sfx)?;

// Load and play music
let music = audio.play("assets/music/background.ogg", AudioChannel::Music)?;
```

## See Also

- [Quick Start](quick-start.md) — Get started with RustEngine
- [Rendering Pipeline](rendering-pipeline.md) — Set up rendering
- [Audio System](audio-system.md) — Add audio
