# Game Framework & State Management

RustEngine provides a high-level game framework with state management for common game flows.

## Overview

The framework crate (`engine-framework`) provides:

- **GameState trait** — Define discrete game states
- **StateStack** — Manage state transitions (push/pop/replace)
- **GameFlowPlugin** — Standard title → menu → game → pause → game-over flow
- **SaveManager** — Persist game progress

## Defining Game States

Implement the `GameState` trait for each state:

```rust
use engine_framework::{GameState, StateCtx};

struct GameplayState {
    score: i32,
}

impl GameState for GameplayState {
    fn on_enter(&mut self, ctx: &mut StateCtx) {
        println!("Entering gameplay");
    }

    fn on_exit(&mut self, ctx: &mut StateCtx) {
        println!("Exiting gameplay, score: {}", self.score);
    }

    fn update(&mut self, ctx: &mut StateCtx, dt: f32) {
        // Game logic here
        self.score += 1;
    }
}
```

## State Stack

The `StateStack` manages a stack of states where only the topmost receives updates:

```rust
use engine_framework::StateStack;

let mut stack = StateStack::new();

// Push a state
stack.push(Box::new(GameplayState { score: 0 }));

// Pop the top state
stack.pop();

// Replace the top state
stack.replace(Box::new(PauseState));
```

Operations are deferred until `flush()` is called, making mid-frame transitions safe.

## Standard Game Flow

The `GameFlowPlugin` provides a standard flow:

```
TitleState → MenuState → GameplayState → PauseState
                                    ↓
                              GameOverState → MenuState
```

Use `GameStateAction` to drive transitions:

```rust
use engine_framework::GameStateAction;

// Start a new game
resources.insert(GameStateAction::StartGame);

// Push pause menu
resources.insert(GameStateAction::PushPause);

// Pop back from pause
resources.insert(GameStateAction::Pop);

// Game over with score
resources.insert(GameStateAction::PushGameOver { score: 1000 });

// Return to title
resources.insert(GameStateAction::PushTitle);

// Quit
resources.insert(GameStateAction::Quit);
```

## Game Session

Track the current game session:

```rust
use engine_framework::GameSession;

let session = world.get_resource::<GameSession>().unwrap();
println!("Score: {}, Running: {}", session.score, session.is_running);
```

## Save System

Persist game data with `SaveManager`:

```rust
use engine_framework::save::{SaveManager, SaveData, SaveValue};

let mut manager = SaveManager::new("saves");

// Create save data
let mut data = SaveData::new();
data.set("player", "score", SaveValue::Int(1000));
data.set("player", "name", SaveValue::String("Hero".to_string()));

// Save to slot 0
manager.save(0, &data)?;

// Load from slot 0
let loaded = manager.load(0)?;

// List available slots
let slots = manager.list_slots();
```

## Custom Framework

Build your own state machine:

```rust
use engine_core::app::AppBuilder;
use engine_framework::{FrameworkPlugin, StateStack};

let mut app = AppBuilder::new();
app.add_plugin(FrameworkPlugin);

// Insert your custom initial state
if let Some(stack) = app.resources_mut().get_mut::<StateStack>() {
    stack.push(Box::new(MyCustomState));
}
```
