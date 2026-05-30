//! 2D sprite animation system.
//!
//! Provides sprite sheet frame lookup, named frame sequences with playback
//! modes, and an ECS update system that advances `SpriteAnimation` state
//! and writes the corresponding UV region into the entity's `Sprite`.

use engine_ecs::world::World;

/// Playback behavior when reaching the end of a frame sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Loop back to the first frame.
    Loop,
    /// Stop on the last frame.
    Once,
    /// Reverse direction at both ends.
    PingPong,
}

/// A texture split into a uniform grid of frames.
#[derive(Debug, Clone)]
pub struct SpriteSheet {
    /// Total texture width in pixels.
    pub texture_width: u32,
    /// Total texture height in pixels.
    pub texture_height: u32,
    /// Width of a single frame in pixels.
    pub frame_width: u32,
    /// Height of a single frame in pixels.
    pub frame_height: u32,
    /// Number of columns in the grid.
    pub columns: u32,
    /// Number of rows in the grid.
    pub rows: u32,
}

impl SpriteSheet {
    /// Creates a new sprite sheet, deriving grid dimensions from texture and
    /// frame sizes.
    pub fn new(
        texture_width: u32,
        texture_height: u32,
        frame_width: u32,
        frame_height: u32,
    ) -> Self {
        assert!(
            frame_width > 0 && frame_height > 0,
            "frame size must be > 0"
        );
        Self {
            texture_width,
            texture_height,
            frame_width,
            frame_height,
            columns: texture_width / frame_width,
            rows: texture_height / frame_height,
        }
    }

    /// Total number of frames in the sheet.
    pub fn frame_count(&self) -> u32 {
        self.columns * self.rows
    }

    /// Computes the UV region `[u_min, v_min, u_max, v_max]` for the given
    /// frame index. Returns `(0,0)-(1,1)` if the index is out of bounds.
    pub fn frame_uv(&self, index: usize) -> [f32; 4] {
        let idx = index as u32;
        if idx >= self.frame_count() {
            return [0.0, 0.0, 1.0, 1.0];
        }
        let col = idx % self.columns;
        let row = idx / self.columns;
        let u_min = col as f32 * self.frame_width as f32 / self.texture_width as f32;
        let v_min = row as f32 * self.frame_height as f32 / self.texture_height as f32;
        let u_max = (col + 1) as f32 * self.frame_width as f32 / self.texture_width as f32;
        let v_max = (row + 1) as f32 * self.frame_height as f32 / self.texture_height as f32;
        [u_min, v_min, u_max, v_max]
    }
}

/// A named sequence of frames with playback configuration.
#[derive(Debug, Clone)]
pub struct FrameSequence {
    /// Ordered list of frame indices into the sprite sheet.
    pub frames: Vec<usize>,
    /// Playback speed in frames per second.
    pub fps: f32,
    /// What to do when the sequence ends.
    pub mode: PlaybackMode,
}

impl FrameSequence {
    /// Creates a looping sequence.
    pub fn looping(frames: Vec<usize>, fps: f32) -> Self {
        Self {
            frames,
            fps,
            mode: PlaybackMode::Loop,
        }
    }

    /// Creates a play-once sequence.
    pub fn once(frames: Vec<usize>, fps: f32) -> Self {
        Self {
            frames,
            fps,
            mode: PlaybackMode::Once,
        }
    }

    /// Creates a ping-pong sequence.
    pub fn ping_pong(frames: Vec<usize>, fps: f32) -> Self {
        Self {
            frames,
            fps,
            mode: PlaybackMode::PingPong,
        }
    }
}

/// Animation state component. Attach to entities alongside `Sprite` to
/// drive automatic frame animation.
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    /// Index into `SpriteSheetStore::sheets` identifying the sprite sheet.
    pub sheet_index: usize,
    /// Name of the current frame sequence (looked up in `sequences` map).
    pub sequence_name: String,
    /// Index within the current sequence's `frames` vector.
    pub current_frame: usize,
    /// Accumulated time in seconds since the last frame change.
    pub elapsed: f32,
    /// Whether the animation is currently playing.
    pub playing: bool,
    /// Current playback direction: `1` = forward, `-1` = reverse (for PingPong).
    pub direction: i32,
}

impl SpriteAnimation {
    /// Creates a new animation that starts playing from frame 0.
    pub fn new(sheet_index: usize, sequence_name: impl Into<String>) -> Self {
        Self {
            sheet_index,
            sequence_name: sequence_name.into(),
            current_frame: 0,
            elapsed: 0.0,
            playing: true,
            direction: 1,
        }
    }

    /// Switches to a different sequence, resetting playback state.
    pub fn play(&mut self, sequence_name: impl Into<String>) {
        self.sequence_name = sequence_name.into();
        self.current_frame = 0;
        self.elapsed = 0.0;
        self.direction = 1;
        self.playing = true;
    }

    /// Pauses the animation at the current frame.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resumes playback.
    pub fn resume(&mut self) {
        self.playing = true;
    }
}

/// Collection of sprite sheets and their named frame sequences.
/// Insert as an ECS resource via `world.insert_resource(SpriteSheetStore::new())`.
pub struct SpriteSheetStore {
    pub sheets: Vec<SpriteSheet>,
    /// Named sequences indexed by sheet index.
    pub sequences: Vec<std::collections::HashMap<String, FrameSequence>>,
}

impl Default for SpriteSheetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteSheetStore {
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            sequences: Vec::new(),
        }
    }

    /// Adds a sprite sheet with its sequences. Returns the sheet index.
    pub fn add_sheet(
        &mut self,
        sheet: SpriteSheet,
        sequences: std::collections::HashMap<String, FrameSequence>,
    ) -> usize {
        let index = self.sheets.len();
        self.sheets.push(sheet);
        self.sequences.push(sequences);
        index
    }
}

/// The `Sprite` type used by the animation system. Re-exports the UV fields
/// needed for frame updates.
///
/// The animation system expects entities to have both a `SpriteAnimation`
/// and a `Sprite` (from `crate::sprite`). The `Sprite::uv_region` field
/// is written to by the animation system.
///
/// ECS update system — call from your `Schedule`:
/// ```ignore
/// use engine_render::animation::animation_update_system;
/// schedule.add_system(animation_update_system.system());
/// ```
pub fn animation_update_system(world: &mut World) {
    use crate::sprite::Sprite;

    let indices: Vec<u32> = {
        let anim_ents = world.component_entities::<SpriteAnimation>();
        let sprite_ents = world.component_entities::<Sprite>();
        anim_ents
            .into_iter()
            .filter(|idx| sprite_ents.contains(idx))
            .collect()
    };

    // Snapshot dt and sheet data from resources to avoid borrow conflicts.
    let dt = world
        .get_resource::<AnimationTime>()
        .map(|t| t.dt)
        .unwrap_or(1.0 / 60.0);

    let (sheets_snapshot, sequences_snapshot) = {
        match world.get_resource::<SpriteSheetStore>() {
            Some(store) => (store.sheets.clone(), store.sequences.clone()),
            None => return,
        }
    };

    // Phase 1: collect per-entity work (reads only).
    struct EntityWork {
        idx: u32,
        uv: [f32; 4],
        next_frame: usize,
        elapsed: f32,
        playing: bool,
        direction: i32,
    }
    let mut work_list: Vec<EntityWork> = Vec::new();

    for &idx in &indices {
        let anim = match world.get_by_index::<SpriteAnimation>(idx) {
            Some(a) => a,
            None => continue,
        };
        if !anim.playing {
            continue;
        }
        let sheet_idx = anim.sheet_index;
        let sheet = match sheets_snapshot.get(sheet_idx) {
            Some(s) => s,
            None => continue,
        };
        let seqs = match sequences_snapshot.get(sheet_idx) {
            Some(s) => s,
            None => continue,
        };
        let seq = match seqs.get(&anim.sequence_name) {
            Some(s) => s,
            None => continue,
        };
        if seq.frames.is_empty() {
            continue;
        }

        // Advance timing first, then compute UV for the new frame.
        let mut elapsed = anim.elapsed + dt;
        let mut current_frame = anim.current_frame;
        let mut playing = anim.playing;
        let mut direction = anim.direction;

        let frame_duration = if seq.fps > 0.0 {
            1.0 / seq.fps
        } else {
            f32::MAX
        };

        while elapsed >= frame_duration && frame_duration < f32::MAX {
            elapsed -= frame_duration;

            match seq.mode {
                PlaybackMode::Loop => {
                    current_frame = (current_frame + 1) % seq.frames.len();
                }
                PlaybackMode::Once => {
                    if current_frame + 1 < seq.frames.len() {
                        current_frame += 1;
                    } else {
                        playing = false;
                        break;
                    }
                }
                PlaybackMode::PingPong => {
                    let next = current_frame as i32 + direction;
                    if next < 0 {
                        direction = 1;
                        current_frame = if seq.frames.len() > 1 { 1 } else { 0 };
                    } else if next >= seq.frames.len() as i32 {
                        direction = -1;
                        current_frame = if seq.frames.len() > 1 {
                            seq.frames.len() - 2
                        } else {
                            0
                        };
                    } else {
                        current_frame = next as usize;
                    }
                }
            }
        }

        // Compute UV for the (possibly advanced) current frame.
        let frame_idx = current_frame.min(seq.frames.len() - 1);
        let sheet_frame = seq.frames[frame_idx];
        let uv = sheet.frame_uv(sheet_frame);

        work_list.push(EntityWork {
            idx,
            uv,
            next_frame: current_frame,
            elapsed,
            playing,
            direction,
        });
    }

    // Phase 2: apply all changes (mutable access, one entity at a time).
    for work in work_list {
        if let Some(sprite) = world.get_by_index_mut::<Sprite>(work.idx) {
            sprite.uv_region = work.uv;
        }
        if let Some(anim) = world.get_by_index_mut::<SpriteAnimation>(work.idx) {
            anim.current_frame = work.next_frame;
            anim.elapsed = work.elapsed;
            anim.playing = work.playing;
            anim.direction = work.direction;
        }
    }
}

/// Frame delta-time resource. Set this each frame before running the schedule.
pub struct AnimationTime {
    pub dt: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sprite_sheet_new() {
        let sheet = SpriteSheet::new(256, 256, 64, 64);
        assert_eq!(sheet.columns, 4);
        assert_eq!(sheet.rows, 4);
        assert_eq!(sheet.frame_count(), 16);
    }

    #[test]
    fn test_sprite_sheet_uv_first_frame() {
        let sheet = SpriteSheet::new(128, 32, 64, 32);
        let uv = sheet.frame_uv(0);
        // Frame 0: col=0, row=0
        assert!((uv[0] - 0.0).abs() < 1e-6); // u_min
        assert!((uv[1] - 0.0).abs() < 1e-6); // v_min
        assert!((uv[2] - 0.5).abs() < 1e-6); // u_max = 64/128
        assert!((uv[3] - 1.0).abs() < 1e-6); // v_max = 32/32
    }

    #[test]
    fn test_sprite_sheet_uv_middle_frame() {
        let sheet = SpriteSheet::new(128, 32, 64, 32);
        // Frame 1: col=1, row=0
        let uv = sheet.frame_uv(1);
        assert!((uv[0] - 0.5).abs() < 1e-6); // u_min = 64/128
        assert!((uv[1] - 0.0).abs() < 1e-6); // v_min
        assert!((uv[2] - 1.0).abs() < 1e-6); // u_max = 128/128
        assert!((uv[3] - 1.0).abs() < 1e-6); // v_max = 32/32
    }

    #[test]
    fn test_sprite_sheet_uv_out_of_bounds() {
        let sheet = SpriteSheet::new(64, 64, 32, 32);
        let uv = sheet.frame_uv(99);
        assert_eq!(uv, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_frame_sequence_constructors() {
        let seq = FrameSequence::looping(vec![0, 1, 2], 10.0);
        assert_eq!(seq.mode, PlaybackMode::Loop);
        assert_eq!(seq.frames.len(), 3);

        let seq = FrameSequence::once(vec![0, 1], 8.0);
        assert_eq!(seq.mode, PlaybackMode::Once);

        let seq = FrameSequence::ping_pong(vec![0, 1, 2], 12.0);
        assert_eq!(seq.mode, PlaybackMode::PingPong);
    }

    #[test]
    fn test_sprite_animation_play_pause() {
        let mut anim = SpriteAnimation::new(0, "idle");
        assert!(anim.playing);
        anim.pause();
        assert!(!anim.playing);
        anim.resume();
        assert!(anim.playing);
        anim.play("run");
        assert_eq!(anim.sequence_name, "run");
        assert_eq!(anim.current_frame, 0);
        assert!(anim.playing);
    }

    #[test]
    fn test_sprite_sheet_store_add() {
        let mut store = SpriteSheetStore::new();
        let sheet = SpriteSheet::new(64, 64, 32, 32);
        let mut seqs = HashMap::new();
        seqs.insert("idle".into(), FrameSequence::looping(vec![0, 1], 10.0));
        let idx = store.add_sheet(sheet, seqs);
        assert_eq!(idx, 0);
        assert_eq!(store.sheets.len(), 1);
        assert_eq!(store.sequences.len(), 1);
    }

    #[test]
    fn test_animation_system_advances_frame() {
        use crate::sprite::Sprite;
        use engine_math::Vec2;

        let mut world = World::new();

        // Setup sprite sheet store.
        let mut store = SpriteSheetStore::new();
        let sheet = SpriteSheet::new(64, 64, 32, 32);
        let mut seqs = HashMap::new();
        seqs.insert("walk".into(), FrameSequence::looping(vec![0, 1, 2, 3], 4.0));
        store.add_sheet(sheet, seqs);
        world.insert_resource(store);

        // Setup entity with SpriteAnimation + Sprite.
        let e = world.spawn();
        world.add_component(e, SpriteAnimation::new(0, "walk"));
        world.add_component(
            e,
            Sprite {
                texture: engine_asset::asset::Handle::new(engine_asset::types::Texture {
                    id: "test".into(),
                    width: 64,
                    height: 64,
                    data: vec![],
                    channels: 4,
                    asset_path: std::path::PathBuf::new(),
                }),
                color: [1.0; 4],
                size: Vec2::new(32.0, 32.0),
                transform: engine_math::Mat4::IDENTITY,
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );

        // dt = 0.3s → at 4 fps (0.25s per frame), should advance 1 frame.
        world.insert_resource(AnimationTime { dt: 0.3 });

        animation_update_system(&mut world);

        let anim = world.get::<SpriteAnimation>(e).unwrap();
        assert_eq!(anim.current_frame, 1);

        let sprite = world.get::<Sprite>(e).unwrap();
        // Frame 1 should have UV [0.5, 0.0, 1.0, 0.5]
        assert!((sprite.uv_region[0] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_animation_system_once_stops() {
        use crate::sprite::Sprite;
        use engine_math::Vec2;

        let mut world = World::new();

        let mut store = SpriteSheetStore::new();
        let sheet = SpriteSheet::new(32, 32, 32, 32);
        let mut seqs = HashMap::new();
        seqs.insert("explode".into(), FrameSequence::once(vec![0, 1, 2], 1.0));
        store.add_sheet(sheet, seqs);
        world.insert_resource(store);

        let e = world.spawn();
        world.add_component(e, SpriteAnimation::new(0, "explode"));
        world.add_component(
            e,
            Sprite {
                texture: engine_asset::asset::Handle::new(engine_asset::types::Texture {
                    id: "test".into(),
                    width: 32,
                    height: 32,
                    data: vec![],
                    channels: 4,
                    asset_path: std::path::PathBuf::new(),
                }),
                color: [1.0; 4],
                size: Vec2::new(32.0, 32.0),
                transform: engine_math::Mat4::IDENTITY,
                flip_x: false,
                flip_y: false,
                uv_region: [0.0, 0.0, 1.0, 1.0],
            },
        );

        // Run 3 frames worth of time.
        world.insert_resource(AnimationTime { dt: 1.0 });
        animation_update_system(&mut world);
        animation_update_system(&mut world);
        animation_update_system(&mut world);

        let anim = world.get::<SpriteAnimation>(e).unwrap();
        assert_eq!(anim.current_frame, 2);
        assert!(!anim.playing);
    }
}
