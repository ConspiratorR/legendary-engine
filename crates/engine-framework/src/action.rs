/// An action that drives state transitions in the game flow.
#[derive(Default)]
pub enum GameStateAction {
    /// No action.
    #[default]
    None,
    /// Replace the current state with the menu.
    PushMenu,
    /// Push the pause state on top of the current state.
    PushPause,
    /// Push the game-over state with the final score.
    PushGameOver {
        /// The player's final score.
        score: i32,
    },
    /// Replace the current state with the title screen.
    PushTitle,
    /// Pop the topmost state.
    Pop,
    /// Start a new game session.
    StartGame,
    /// Request the application to quit.
    Quit,
}

/// Tracks the current game session (score, running state).
pub struct GameSession {
    /// The current score.
    pub score: i32,
    /// Whether a game session is currently active.
    pub is_running: bool,
    /// Whether a quit has been requested.
    pub quit_requested: bool,
}

impl Default for GameSession {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSession {
    /// Create a new game session with default values.
    pub fn new() -> Self {
        Self {
            score: 0,
            is_running: false,
            quit_requested: false,
        }
    }

    /// Reset the session to its initial state.
    pub fn reset(&mut self) {
        self.score = 0;
        self.is_running = false;
        self.quit_requested = false;
    }
}

impl GameStateAction {
    /// Return a human-readable name for this action variant.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::PushMenu => "PushMenu",
            Self::PushPause => "PushPause",
            Self::PushGameOver { .. } => "PushGameOver",
            Self::PushTitle => "PushTitle",
            Self::Pop => "Pop",
            Self::StartGame => "StartGame",
            Self::Quit => "Quit",
        }
    }
}
