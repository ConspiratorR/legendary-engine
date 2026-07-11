# Animation System

RustEngine provides a multi-layered animation system covering 2D sprite animation, 3D keyframe animation, skeletal animation with skinning, animation state machines, IK/FK solvers, and UI widget animation.

## Architecture Overview

| Layer | Crate | Module | Purpose |
|-------|-------|--------|---------|
| 2D Sprite | `engine-render` | `animation` | Sprite sheet frame playback |
| 3D Keyframe | `engine-scene` | `keyframe` | Transform animation clips (position/rotation/scale) |
| Skeletal | `engine-scene` | `skeleton` | Joint hierarchies, skinning, matrix palettes |
| State Machine | `engine-scene` | `animation_state` | State transitions with blend support |
| IK/FK | `engine-scene` | `ik` | CCD inverse kinematics and forward kinematics |
| UI | `engine-ui` | `animation` | Tweens, easing, widget property animation |

---

## 1. Sprite Sheet Animation (2D)

**Module:** `engine_render::animation`

Sprite animation drives UV region changes on a `Sprite` component based on a grid-based sprite sheet and named frame sequences.

### Core Types

- **`SpriteSheet`** — A texture divided into a uniform grid of frames.
- **`FrameSequence`** — A named list of frame indices with FPS and playback mode.
- **`SpriteAnimation`** — ECS component tracking current sequence, frame, and timing.
- **`SpriteSheetStore`** — Resource holding all sprite sheets and their sequences.
- **`AnimationTime`** — Resource providing delta time each frame.

### Playback Modes

```rust
pub enum PlaybackMode {
    Loop,     // Loop back to first frame
    Once,     // Stop on last frame
    PingPong, // Reverse direction at both ends
}
```

### Example

```rust
use engine_render::animation::*;
use std::collections::HashMap;

// Create a sprite sheet from a 256x256 texture with 64x64 frames (4x4 grid).
let sheet = SpriteSheet::new(256, 256, 64, 64);

// Define named sequences.
let mut sequences = HashMap::new();
sequences.insert(
    "idle".into(),
    FrameSequence::looping(vec![0, 1, 2, 3], 8.0),
);
sequences.insert(
    "run".into(),
    FrameSequence::looping(vec![4, 5, 6, 7], 12.0),
);
sequences.insert(
    "explode".into(),
    FrameSequence::once(vec![8, 9, 10, 11], 10.0),
);

// Register the sheet store as an ECS resource.
let mut store = SpriteSheetStore::new();
let sheet_index = store.add_sheet(sheet, sequences);
world.insert_resource(store);

// Spawn an entity with SpriteAnimation + Sprite.
let entity = world.spawn();
world.add_component(entity, SpriteAnimation::new(sheet_index, "idle"));
world.add_component(entity, sprite_component); // requires Sprite with uv_region

// Set delta time each frame.
world.insert_resource(AnimationTime { dt: 1.0 / 60.0 });

// Run the update system in your schedule.
animation_update_system(&mut world);
```

### Switching Sequences

```rust
let anim = world.get_mut::<SpriteAnimation>(entity).unwrap();
anim.play("run");   // switches to "run", resets to frame 0
anim.pause();       // freezes at current frame
anim.resume();      // continues playback
```

---

## 2. Keyframe Animation (3D)

**Module:** `engine_scene::keyframe`

Provides transform animation clips with position, rotation, and scale tracks. Supports linear, step, and cubic spline interpolation.

### Keyframe Types

```rust
pub enum Interpolation {
    Linear, // lerp / slerp
    Step,   // snap to next value
    Cubic,  // Hermite spline (uses tangent values)
}

pub struct FloatKeyframe  { time, value, interpolation, tangent_in, tangent_out }
pub struct Vec3Keyframe   { time, value, interpolation, tangent_in, tangent_out }
pub struct RotationKeyframe { time, value (Quat), interpolation }
```

### AnimationClip

An `AnimationClip` holds optional position, rotation, and scale tracks. Only tracks that are present get applied.

```rust
use engine_scene::keyframe::*;
use engine_math::{Vec3, Quat};

let clip = AnimationClip::new("walk", 2.0)
    .looping(true)
    .with_position_track(vec![
        Vec3Keyframe::linear(0.0, Vec3::ZERO),
        Vec3Keyframe::linear(0.5, Vec3::new(5.0, 0.0, 0.0)),
        Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
    ])
    .with_rotation_track(vec![
        RotationKeyframe::linear(0.0, Quat::IDENTITY),
        RotationKeyframe::linear(1.0, Quat::from_rotation_y(std::f32::consts::PI)),
    ])
    .with_scale_track(vec![
        Vec3Keyframe::linear(0.0, Vec3::ONE),
        Vec3Keyframe::linear(0.5, Vec3::splat(1.5)),
        Vec3Keyframe::linear(1.0, Vec3::ONE),
    ]);

// Sample at any time.
let position = clip.sample_position(0.5);   // Option<Vec3>
let rotation = clip.sample_rotation(0.5);   // Option<Quat>
let scale = clip.sample_scale(0.5);         // Option<Vec3>
```

### AnimationPlayer

Component that tracks playback state per entity.

```rust
let mut player = AnimationPlayer::new("walk");
player.speed = 1.5;

// Each frame:
player.advance(delta_time, &clip);

// Control
player.pause();
player.stop();    // resets time to 0
player.play();    // resume
```

---

## 3. Skeletal Animation

**Module:** `engine_scene::skeleton`

Full skeletal animation with joint hierarchies, skinning data, per-joint animation clips, pose blending, and GPU matrix palette generation.

### Skeleton

```rust
use engine_scene::skeleton::*;

let skeleton = Skeleton::new(vec![
    Joint {
        id: 0,
        name: "root".into(),
        parent_index: None,
        inverse_bind_pose: Mat4::IDENTITY,
        local_bind_pose: Mat4::IDENTITY,
    },
    Joint {
        id: 1,
        name: "spine".into(),
        parent_index: Some(0),
        inverse_bind_pose: Mat4::from_translation(Vec3::new(0.0, -10.0, 0.0)),
        local_bind_pose: Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0)),
    },
    Joint {
        id: 2,
        name: "head".into(),
        parent_index: Some(1),
        inverse_bind_pose: Mat4::from_translation(Vec3::new(0.0, -20.0, 0.0)),
        local_bind_pose: Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0)),
    },
]);

// Lookup joints by name.
assert_eq!(skeleton.find_joint("spine"), Some(1));

// Get parent chain.
let chain = skeleton.parent_chain(2); // [2, 1, 0]
```

### SkeletalAnimationClip

Per-joint animation tracks mapped by joint name.

```rust
let clip = SkeletalAnimationClip::new("walk", 2.0)
    .looping(true)
    .with_position_track("root", vec![
        Vec3Keyframe::linear(0.0, Vec3::ZERO),
        Vec3Keyframe::linear(1.0, Vec3::new(10.0, 0.0, 0.0)),
    ])
    .with_rotation_track("spine", vec![
        RotationKeyframe::linear(0.0, Quat::IDENTITY),
        RotationKeyframe::linear(0.5, Quat::from_rotation_x(0.3)),
        RotationKeyframe::linear(1.0, Quat::IDENTITY),
    ]);

// Sample a single joint's transform.
let spine_transform = clip.sample_joint("spine", 0.5);
```

### SkeletalAnimationPlayer

Drives playback and produces a matrix palette for GPU skinning.

```rust
let mut player = SkeletalAnimationPlayer::new("walk", &skeleton);

// Each frame:
player.advance(delta, &clip);
player.sample_pose(&skeleton, &clip);

// Get the matrix palette for GPU skinning.
// final_matrix[j] = world_transform[j] * inverse_bind_pose[j]
let palette: Vec<Mat4> = player.compute_matrix_palette(&skeleton);

// Or access per-joint local transforms directly.
let pose = player.local_pose();  // &[JointTransform]
```

### Skin (Vertex Weights)

```rust
let skin = Skin::new("humanoid", vertex_count);
// skin.joint_indices[i] = [u16; 4]  — up to 4 joint influences per vertex
// skin.joint_weights[i] = [f32; 4]  — corresponding weights
```

### Pose Blending

Blend two skeletal clips at arbitrary times with an alpha weight.

```rust
let blended_pose = blend_skeletal_poses(
    &skeleton,
    &clip_a, time_a,
    &clip_b, time_b,
    alpha, // 0.0 = clip_a, 1.0 = clip_b
);

// Compute palette from blended pose.
let palette = compute_palette_from_pose(&skeleton, &blended_pose);
```

---

## 4. Animation State Machine

**Module:** `engine_scene::animation_state`

Drives which animation clip plays based on states, transitions, and parameter conditions.

### States and Transitions

```rust
use engine_scene::animation_state::*;

let mut sm = AnimationStateMachine::new("idle");

sm.add_state(AnimationState::new("idle", "idle_clip").with_looping(true));
sm.add_state(AnimationState::new("run", "run_clip").with_speed(1.5));
sm.add_state(AnimationState::new("jump", "jump_clip").with_looping(false));

// Transition: idle → run when is_moving becomes true.
sm.add_transition(AnimationTransition {
    from: "idle".into(),
    to: "run".into(),
    blend_duration: 0.2, // 200ms blend
    condition: TransitionCondition::BoolTrue("is_moving".into()),
});

// Transition: run → idle when speed drops below 0.5.
sm.add_transition(AnimationTransition {
    from: "run".into(),
    to: "idle".into(),
    blend_duration: 0.3,
    condition: TransitionCondition::FloatLess("speed".into(), 0.5),
});

// Transition: idle → jump on trigger.
sm.add_transition(AnimationTransition {
    from: "idle".into(),
    to: "jump".into(),
    blend_duration: 0.0, // instant
    condition: TransitionCondition::Trigger("jump".into()),
});
```

### Condition Types

```rust
pub enum TransitionCondition {
    Always,                   // transition immediately
    BoolTrue(String),         // when parameter is true
    BoolFalse(String),        // when parameter is false
    FloatGreater(String, f32), // when float > threshold
    FloatLess(String, f32),    // when float < threshold
    Trigger(String),          // manual trigger
}
```

### Updating

```rust
// Set parameters each frame.
sm.parameters.set_bool("is_moving", input.is_moving);
sm.parameters.set_float("speed", velocity.length());
sm.parameters.set_trigger("jump"); // fire once

// Advance the state machine.
let current_state_name = sm.update(delta_time);
let state = sm.current().unwrap(); // AnimationState { clip_name, speed, looping }

// Blend weights during transitions.
let from_weight = sm.current_weight();
let to_weight = sm.target_weight();
```

---

## 5. IK / FK (Inverse / Forward Kinematics)

**Module:** `engine_scene::ik`

### Forward Kinematics (FK)

Computes world-space transforms from local joint poses by traversing root-to-leaf.

```rust
use engine_scene::ik::*;

let world_transforms = FKSolver::compute_world_transforms(&skeleton, &local_pose);

// Single joint.
let head_world = FKSolver::compute_world_transform(&skeleton, &local_pose, 2);
```

### Inverse Kinematics (IK)

CCD (Cyclic Coordinate Descent) solver adjusts joint rotations to reach a world-space target.

```rust
// Define an IK chain (root → mid → end-effector).
let chain = IKChain::from_names(
    &skeleton,
    &["root", "mid", "end"],
    50,    // max iterations
    0.1,   // convergence tolerance
).unwrap();

// Define the target.
let target = IKTarget::new(Vec3::new(15.0, 10.0, 0.0));

// Solve — modifies local_pose in place.
let converged = IKSolver::solve(&skeleton, &chain, &target, &mut local_pose);

// Or solve directly on a SkeletalAnimationPlayer.
let converged = IKSolver::solve_with_player(&skeleton, &chain, &target, &mut player);
```

### ECS IK System

Attach `IKChain`, `IKTarget`, `Skeleton`, and `SkeletalAnimationPlayer` to an entity, then run:

```rust
ik_solve_system(&mut world);
```

---

## 6. UI Animation

**Module:** `engine_ui::animation`

Tweens, easing curves, widget property animation, and gesture recognition for UI.

### Easing Curves

```rust
pub enum Easing {
    Linear,
    EaseIn, EaseOut, EaseInOut,
    EaseInCubic, EaseOutCubic, EaseInOutCubic,
    BackIn, BackOut,
    BounceOut,
}

let value = Easing::EaseInOut.apply(0.5); // 0.0..=1.0
```

### Tween

```rust
use engine_ui::animation::*;
use std::time::Duration;

let mut tween = Tween::new(0.0, 100.0, Duration::from_secs(1))
    .with_easing(Easing::EaseOutCubic)
    .with_looping(true)
    .with_ping_pong(true);

// Each frame.
let value = tween.tick(Duration::from_millis(16));
if tween.is_finished() { /* ... */ }
tween.reset();
```

### Widget Property Animation

```rust
use engine_ui::animation::*;

let mut manager = AnimationManager::new();

manager.animate(UiAnimation::new(
    widget_id,
    AnimProperty::Opacity,
    Tween::new(0.0, 1.0, Duration::from_millis(300))
        .with_easing(Easing::EaseInOut),
));

// Each frame.
let changes: HashMap<(u64, AnimProperty), f32> = manager.tick(dt);
// Apply changes to widget properties.

manager.cancel_for_widget(widget_id);
manager.cancel_all();
```

### Gesture Recognition

```rust
let mut recognizer = GestureRecognizer::default();

// Feed pointer events.
recognizer.on_pointer_down(x, y);
recognizer.on_pointer_move(x, y);
recognizer.tick(dt);
recognizer.on_pointer_up();

// Consume gesture.
if let Some(gesture) = recognizer.take_gesture() {
    match gesture {
        Gesture::Tap { x, y } => { /* ... */ }
        Gesture::LongPress { x, y } => { /* ... */ }
        Gesture::Swipe { dx, dy, velocity } => { /* ... */ }
        Gesture::Pinch { scale } => { /* ... */ }
    }
}
```

### Transition Helper

```rust
let mut transition = Transition::new(0.5); // 0.5s duration
let eased_progress = transition.tick(dt); // 0.0..=1.0
if transition.is_complete() { /* ... */ }
transition.reset();
```

---

## 7. ECS Integration

All animation systems are designed to run as ECS systems in the engine's `Schedule`.

### System Registration

```rust
// 2D sprite animation
schedule.add_system(animation_update_system.system());

// IK solving (after skeletal animation)
schedule.add_system(ik_solve_system.system());
```

### Required Components Per Feature

| Feature | Components |
|---------|-----------|
| Sprite Animation | `SpriteAnimation` + `Sprite` |
| Keyframe Animation | `AnimationPlayer` + entity transform |
| Skeletal Animation | `SkeletalAnimationPlayer` + `Skeleton` |
| IK | `IKChain` + `IKTarget` + `Skeleton` + `SkeletalAnimationPlayer` |
| State Machine | `AnimationStateMachine` (standalone or with clips) |
| UI Animation | `AnimationManager` (manual tick) |

### Resources

| Resource | Purpose |
|----------|---------|
| `SpriteSheetStore` | All sprite sheets and sequences |
| `AnimationTime` | Frame delta time for sprite animation |

### Update Order

1. Set `AnimationTime { dt }` each frame.
2. Run `animation_update_system` (2D sprites).
3. Run `ik_solve_system` (IK solving).
4. Run your own systems that read `AnimationPlayer` / `SkeletalAnimationPlayer` state and apply transforms.

---

## 8. File Reference

| File | Lines | Content |
|------|-------|---------|
| `crates/engine-render/src/animation.rs` | 539 | 2D sprite animation (SpriteSheet, FrameSequence, SpriteAnimation, animation_update_system) |
| `crates/engine-scene/src/keyframe.rs` | 505 | 3D keyframe animation (AnimationClip, AnimationPlayer, interpolation) |
| `crates/engine-scene/src/skeleton.rs` | 708 | Skeletal animation (Skeleton, Joint, Skin, SkeletalAnimationPlayer, pose blending) |
| `crates/engine-scene/src/animation_state.rs` | 412 | State machine (AnimationStateMachine, transitions, parameters) |
| `crates/engine-scene/src/ik.rs` | 521 | IK/FK solvers (FKSolver, IKChain, IKSolver, ik_solve_system) |
| `crates/engine-ui/src/animation.rs` | 736 | UI animation (Easing, Tween, UiAnimation, AnimationManager, GestureRecognizer) |
