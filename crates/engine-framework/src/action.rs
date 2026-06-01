#[derive(Default)]
pub enum GameStateAction {
    #[default]
    None,
    PushMenu,
    PushPause,
    PushGameOver {
        score: i32,
    },
    PushTitle,
    Pop,
    StartGame,
    Quit,
}

pub struct GameSession {
    pub score: i32,
    pub is_running: bool,
    pub quit_requested: bool,
}

impl Default for GameSession {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            score: 0,
            is_running: false,
            quit_requested: false,
        }
    }

    pub fn reset(&mut self) {
        self.score = 0;
        self.is_running = false;
        self.quit_requested = false;
    }
}

impl GameStateAction {
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
