//! Runtime SceneData Demo — author → JSON → SceneRuntime → lifecycle tick.
//!
//! No GPU / window required. Shows that Material / SpriteRenderer survive
//! SceneData serialization and that UnityPlayHost-style ticking works.
//!
//! Run: `cargo run -p engine-core --example runtime_scene_demo`

use engine_core::SceneRuntime;
use engine_core::components::{Material, SpriteRenderer};
use engine_core::event::EventBus;
use engine_core::scene_management::LoadSceneMode;
use engine_core::serialization::{SaveSceneJsonPrepared, SceneSerializer};
use engine_core::time::Time;
use engine_core::world::World;

fn main() {
    println!("=== Runtime SceneData Demo ===\n");

    // 1. Author a scene (what the editor would export)
    let mut author = World::new();
    let hero = author.CreateGameObject("Hero");
    author.AddComponent(
        hero,
        Material {
            base_color: [0.3, 0.9, 0.4, 1.0],
            metallic: 0.1,
            smoothness: 0.8,
            ..Default::default()
        },
    );
    author.AddComponent(
        hero,
        SpriteRenderer {
            sprite: "sprites/hero.png".into(),
            color: [1.0, 1.0, 1.0, 1.0],
            flip_x: false,
            flip_y: false,
            sorting_order: 1,
        },
    );
    let shield = author.CreateGameObject("Shield");
    author.SetParent(shield, Some(hero));
    let _ = author.with_transform_mut(shield, |t| {
        t.SetLocalPosition(engine_math::Vec3::new(0.5, 0.0, 0.0));
    });

    let json = SaveSceneJsonPrepared(&mut author, "DemoLevel").expect("serialize");
    println!("Scene JSON ({} bytes):\n{}", json.len(), json);

    // 2. Load into a SceneRuntime (standalone or editor Play host)
    let mut rt = SceneRuntime::new();
    rt.load_scene_json("DemoLevel", &json, LoadSceneMode::Single)
        .expect("load");
    rt.mark_needs_awake();

    let loaded = rt.world.Find("Hero").expect("Hero");
    let m = rt.world.GetComponent::<Material>(loaded).unwrap();
    println!(
        "\nLoaded Hero material = {:?}, sprite = {:?}",
        m.base_color,
        rt.world
            .GetComponent::<SpriteRenderer>(loaded)
            .map(|s| s.sprite.clone())
            .unwrap_or_default()
    );
    println!("Children of Hero = {}", rt.world.GetChildCount(loaded));

    // 3. Drive a few lifecycle frames
    let mut time = Time::new();
    let mut events = EventBus::new();
    for frame in 0..5 {
        time.update(0.016);
        rt.tick(&time, time.frameCount(), &mut events);
        println!(
            "  tick frame {} (unity frameCount={})",
            frame + 1,
            time.frameCount()
        );
    }

    // 4. Optional: direct serializer round-trip without App
    let serializer = SceneSerializer::new();
    let re = serializer.SavePrepared(&mut rt.world, "Echo");
    assert_eq!(re.game_objects.len(), 1);
    assert_eq!(re.game_objects[0].components.len(), 2);
    println!(
        "\nRe-export components = {}",
        re.game_objects[0].components.len()
    );
    println!("=== Demo Complete ===");
}
