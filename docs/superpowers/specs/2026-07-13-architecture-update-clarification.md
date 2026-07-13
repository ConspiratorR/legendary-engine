# Architecture Documentation Update - Clarification

Before proceeding with the design, I need to clarify one point:

**Should the architecture documentation include code examples of the new Unity-like API?**

For example:
```rust
// Example: Creating a GameObject with MonoBehaviour
let mut app = AppBuilder::new();
app.add_startup_system(|ctx: &mut Context| {
    let mut go = GameObject::new("Player");
    go.add_component(Transform::from_xyz(0.0, 0.0, 0.0));
    go.add_component(PlayerController::new());
    ctx.world.spawn(go);
});
```

Options:
1. **Yes, include examples** - Show how to use the new API in practice
2. **No, keep it high-level** - Focus on architecture and concepts only
3. **Minimal examples** - Include one simple example for each major concept

Please let me know your preference.