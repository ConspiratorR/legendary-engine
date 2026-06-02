//! Matchmaking system: room management, player matching, and lobby.
//!
//! Provides room creation/joining, skill-based matching, and pre-game lobby
//! management for multiplayer games.

use std::collections::HashMap;

use thiserror::Error;

/// Errors specific to matchmaking.
#[derive(Debug, Error)]
pub enum MatchmakingError {
    #[error("room not found: {0}")]
    RoomNotFound(u64),
    #[error("room is full")]
    RoomFull,
    #[error("room is closed")]
    RoomClosed,
    #[error("player already in a room")]
    AlreadyInRoom,
    #[error("player not in a room")]
    NotInRoom,
    #[error("player not found: {0}")]
    PlayerNotFound(u64),
    #[error("invalid room configuration: {0}")]
    InvalidConfig(String),
}

/// State of a game room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Waiting,
    InGame,
    Closed,
}

/// Game mode identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameMode {
    /// Quick match — automatic matchmaking.
    #[default]
    QuickMatch,
    /// Custom room — player-created.
    Custom,
    /// Specific named mode.
    Named(String),
}

/// A game room that players can join.
#[derive(Debug, Clone)]
pub struct Room {
    pub id: u64,
    pub name: String,
    pub host_id: u64,
    pub players: Vec<u64>,
    pub max_players: u32,
    pub state: RoomState,
    pub game_mode: GameMode,
    pub map: String,
}

impl Room {
    pub fn is_full(&self) -> bool {
        self.players.len() >= self.max_players as usize
    }

    pub fn is_waiting(&self) -> bool {
        self.state == RoomState::Waiting
    }

    pub fn contains_player(&self, player_id: u64) -> bool {
        self.players.contains(&player_id)
    }
}

/// Manages all game rooms.
#[derive(Debug)]
pub struct RoomManager {
    rooms: HashMap<u64, Room>,
    next_room_id: u64,
    player_rooms: HashMap<u64, u64>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            next_room_id: 1,
            player_rooms: HashMap::new(),
        }
    }

    pub fn create_room(
        &mut self,
        name: String,
        host_id: u64,
        max_players: u32,
        game_mode: GameMode,
        map: String,
    ) -> Result<u64, MatchmakingError> {
        if max_players == 0 {
            return Err(MatchmakingError::InvalidConfig(
                "max_players must be > 0".to_string(),
            ));
        }
        if self.player_rooms.contains_key(&host_id) {
            return Err(MatchmakingError::AlreadyInRoom);
        }

        let room_id = self.next_room_id;
        self.next_room_id += 1;

        let room = Room {
            id: room_id,
            name,
            host_id,
            players: vec![host_id],
            max_players,
            state: RoomState::Waiting,
            game_mode,
            map,
        };

        self.rooms.insert(room_id, room);
        self.player_rooms.insert(host_id, room_id);
        Ok(room_id)
    }

    pub fn join_room(&mut self, room_id: u64, player_id: u64) -> Result<(), MatchmakingError> {
        if self.player_rooms.contains_key(&player_id) {
            return Err(MatchmakingError::AlreadyInRoom);
        }

        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(MatchmakingError::RoomNotFound(room_id))?;

        if room.state != RoomState::Waiting {
            return Err(MatchmakingError::RoomClosed);
        }
        if room.is_full() {
            return Err(MatchmakingError::RoomFull);
        }

        room.players.push(player_id);
        self.player_rooms.insert(player_id, room_id);
        Ok(())
    }

    pub fn leave_room(&mut self, player_id: u64) -> Result<(), MatchmakingError> {
        let room_id = self
            .player_rooms
            .remove(&player_id)
            .ok_or(MatchmakingError::NotInRoom)?;

        if let Some(room) = self.rooms.get_mut(&room_id) {
            room.players.retain(|&p| p != player_id);
            if room.players.is_empty() {
                room.state = RoomState::Closed;
                self.rooms.remove(&room_id);
                return Ok(());
            }
            if room.host_id == player_id {
                room.host_id = room.players[0];
            }
        }

        Ok(())
    }

    pub fn destroy_room(
        &mut self,
        room_id: u64,
        requester_id: u64,
    ) -> Result<(), MatchmakingError> {
        let room = self
            .rooms
            .get(&room_id)
            .ok_or(MatchmakingError::RoomNotFound(room_id))?;

        if room.host_id != requester_id {
            return Err(MatchmakingError::PlayerNotFound(requester_id));
        }

        for &player_id in &room.players {
            self.player_rooms.remove(&player_id);
        }
        self.rooms.remove(&room_id);
        Ok(())
    }

    pub fn get_room(&self, room_id: u64) -> Option<&Room> {
        self.rooms.get(&room_id)
    }

    pub fn get_room_mut(&mut self, room_id: u64) -> Option<&mut Room> {
        self.rooms.get_mut(&room_id)
    }

    pub fn get_player_room(&self, player_id: u64) -> Option<&Room> {
        let room_id = self.player_rooms.get(&player_id)?;
        self.rooms.get(room_id)
    }

    pub fn get_player_room_id(&self, player_id: u64) -> Option<u64> {
        self.player_rooms.get(&player_id).copied()
    }

    pub fn list_waiting_rooms(&self) -> Vec<&Room> {
        self.rooms
            .values()
            .filter(|r| r.state == RoomState::Waiting)
            .collect()
    }

    pub fn list_all_rooms(&self) -> Vec<&Room> {
        self.rooms.values().collect()
    }

    pub fn set_room_state(
        &mut self,
        room_id: u64,
        state: RoomState,
    ) -> Result<(), MatchmakingError> {
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(MatchmakingError::RoomNotFound(room_id))?;
        room.state = state;
        Ok(())
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn cleanup_closed(&mut self) {
        let closed: Vec<u64> = self
            .rooms
            .iter()
            .filter(|(_, r)| r.state == RoomState::Closed)
            .map(|(&id, _)| id)
            .collect();
        for id in closed {
            if let Some(room) = self.rooms.remove(&id) {
                for &player_id in &room.players {
                    self.player_rooms.remove(&player_id);
                }
            }
        }
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A player in the matchmaking queue.
#[derive(Debug, Clone)]
pub struct QueuedPlayer {
    pub player_id: u64,
    pub skill: f32,
    pub ping_ms: u32,
    pub preferred_mode: GameMode,
    pub queued_at: std::time::Instant,
}

/// Matchmaker that groups players by skill, ping, and mode preference.
#[derive(Debug)]
pub struct Matchmaker {
    queue: Vec<QueuedPlayer>,
    pub max_skill_diff: f32,
    pub max_ping_diff: u32,
    pub min_players: u32,
    pub max_players: u32,
}

impl Matchmaker {
    pub fn new(min_players: u32, max_players: u32) -> Self {
        Self {
            queue: Vec::new(),
            max_skill_diff: 500.0,
            max_ping_diff: 100,
            min_players,
            max_players,
        }
    }

    pub fn queue_player(&mut self, player: QueuedPlayer) {
        self.queue.push(player);
    }

    pub fn dequeue_player(&mut self, player_id: u64) -> bool {
        let len_before = self.queue.len();
        self.queue.retain(|p| p.player_id != player_id);
        self.queue.len() < len_before
    }

    /// Try to find a match for queued players.
    pub fn find_match(&mut self) -> Option<Vec<u64>> {
        if self.queue.len() < self.min_players as usize {
            return None;
        }

        self.queue.sort_by_key(|a| a.queued_at);

        if self.queue.is_empty() {
            return None;
        }

        let reference = self.queue[0].clone();
        let mut matched = vec![reference.player_id];

        for player in &self.queue[1..] {
            if matched.len() >= self.max_players as usize {
                break;
            }

            let skill_diff = (reference.skill - player.skill).abs();
            let ping_diff = (reference.ping_ms as i32 - player.ping_ms as i32).unsigned_abs();

            if skill_diff <= self.max_skill_diff && ping_diff <= self.max_ping_diff {
                matched.push(player.player_id);
            }
        }

        if matched.len() >= self.min_players as usize {
            for &id in &matched {
                self.queue.retain(|p| p.player_id != id);
            }
            Some(matched)
        } else {
            None
        }
    }

    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn queue(&self) -> &[QueuedPlayer] {
        &self.queue
    }
}

impl Default for Matchmaker {
    fn default() -> Self {
        Self::new(2, 16)
    }
}

/// Lobby state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyState {
    Lobby,
    Starting,
    InGame,
}

/// A pre-game lobby where players ready up.
#[derive(Debug)]
pub struct Lobby {
    pub room_id: u64,
    pub state: LobbyState,
    players: HashMap<u64, bool>,
    teams: HashMap<u64, u32>,
}

impl Lobby {
    pub fn new(room_id: u64) -> Self {
        Self {
            room_id,
            state: LobbyState::Lobby,
            players: HashMap::new(),
            teams: HashMap::new(),
        }
    }

    pub fn add_player(&mut self, player_id: u64) {
        self.players.insert(player_id, false);
    }

    pub fn remove_player(&mut self, player_id: u64) {
        self.players.remove(&player_id);
        self.teams.remove(&player_id);
    }

    pub fn set_ready(&mut self, player_id: u64, ready: bool) -> Result<(), MatchmakingError> {
        if !self.players.contains_key(&player_id) {
            return Err(MatchmakingError::PlayerNotFound(player_id));
        }
        self.players.insert(player_id, ready);
        Ok(())
    }

    pub fn is_ready(&self, player_id: u64) -> bool {
        self.players.get(&player_id).copied().unwrap_or(false)
    }

    pub fn all_ready(&self) -> bool {
        !self.players.is_empty() && self.players.values().all(|&r| r)
    }

    pub fn player_ids(&self) -> Vec<u64> {
        self.players.keys().copied().collect()
    }

    pub fn ready_states(&self) -> Vec<bool> {
        self.players.values().copied().collect()
    }

    pub fn assign_team(&mut self, player_id: u64, team_id: u32) {
        self.teams.insert(player_id, team_id);
    }

    pub fn get_team(&self, player_id: u64) -> Option<u32> {
        self.teams.get(&player_id).copied()
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn player_ready_list(&self) -> Vec<(u64, bool)> {
        self.players
            .iter()
            .map(|(&id, &ready)| (id, ready))
            .collect()
    }
}

/// Manages lobbies for all rooms.
#[derive(Debug)]
pub struct LobbyManager {
    lobbies: HashMap<u64, Lobby>,
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
        }
    }

    pub fn create_lobby(&mut self, room_id: u64) -> &mut Lobby {
        self.lobbies
            .entry(room_id)
            .or_insert_with(|| Lobby::new(room_id))
    }

    pub fn get_lobby(&self, room_id: u64) -> Option<&Lobby> {
        self.lobbies.get(&room_id)
    }

    pub fn get_lobby_mut(&mut self, room_id: u64) -> Option<&mut Lobby> {
        self.lobbies.get_mut(&room_id)
    }

    pub fn remove_lobby(&mut self, room_id: u64) {
        self.lobbies.remove(&room_id);
    }

    pub fn lobby_count(&self) -> usize {
        self.lobbies.len()
    }

    pub fn all_ready(&self, room_id: u64) -> bool {
        self.lobbies
            .get(&room_id)
            .map(|l| l.all_ready())
            .unwrap_or(false)
    }

    pub fn try_start(&mut self, room_id: u64) -> bool {
        if let Some(lobby) = self.lobbies.get_mut(&room_id)
            && lobby.all_ready()
        {
            lobby.state = LobbyState::Starting;
            return true;
        }
        false
    }
}

impl Default for LobbyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_manager_create_join_leave() {
        let mut mgr = RoomManager::new();
        let room_id = mgr
            .create_room(
                "Test Room".to_string(),
                1,
                4,
                GameMode::QuickMatch,
                "map_01".to_string(),
            )
            .unwrap();

        assert_eq!(mgr.room_count(), 1);
        assert!(mgr.get_player_room(1).is_some());

        mgr.join_room(room_id, 2).unwrap();
        assert_eq!(mgr.get_room(room_id).unwrap().players.len(), 2);

        mgr.leave_room(2).unwrap();
        assert!(mgr.get_player_room(2).is_none());
    }

    #[test]
    fn test_room_full() {
        let mut mgr = RoomManager::new();
        let room_id = mgr
            .create_room(
                "Small".to_string(),
                1,
                2,
                GameMode::QuickMatch,
                "m".to_string(),
            )
            .unwrap();

        mgr.join_room(room_id, 2).unwrap();
        assert!(matches!(
            mgr.join_room(room_id, 3),
            Err(MatchmakingError::RoomFull)
        ));
    }

    #[test]
    fn test_room_already_in_room() {
        let mut mgr = RoomManager::new();
        mgr.create_room(
            "R1".to_string(),
            1,
            4,
            GameMode::QuickMatch,
            "m".to_string(),
        )
        .unwrap();
        let r2 = mgr
            .create_room(
                "R2".to_string(),
                2,
                4,
                GameMode::QuickMatch,
                "m".to_string(),
            )
            .unwrap();

        assert!(matches!(
            mgr.join_room(r2, 1),
            Err(MatchmakingError::AlreadyInRoom)
        ));
    }

    #[test]
    fn test_room_host_transfer() {
        let mut mgr = RoomManager::new();
        let room_id = mgr
            .create_room("R".to_string(), 1, 4, GameMode::QuickMatch, "m".to_string())
            .unwrap();
        mgr.join_room(room_id, 2).unwrap();

        mgr.leave_room(1).unwrap();
        assert_eq!(mgr.get_room(room_id).unwrap().host_id, 2);
    }

    #[test]
    fn test_room_empty_destroyed() {
        let mut mgr = RoomManager::new();
        let _ = mgr
            .create_room("R".to_string(), 1, 4, GameMode::QuickMatch, "m".to_string())
            .unwrap();

        mgr.leave_room(1).unwrap();
        assert_eq!(mgr.room_count(), 0);
    }

    #[test]
    fn test_room_destroy_by_host() {
        let mut mgr = RoomManager::new();
        let room_id = mgr
            .create_room("R".to_string(), 1, 4, GameMode::QuickMatch, "m".to_string())
            .unwrap();
        mgr.join_room(room_id, 2).unwrap();

        mgr.destroy_room(room_id, 1).unwrap();
        assert_eq!(mgr.room_count(), 0);
        assert!(mgr.get_player_room(2).is_none());
    }

    #[test]
    fn test_room_list_waiting() {
        let mut mgr = RoomManager::new();
        mgr.create_room(
            "R1".to_string(),
            1,
            4,
            GameMode::QuickMatch,
            "m".to_string(),
        )
        .unwrap();
        let r2 = mgr
            .create_room("R2".to_string(), 2, 4, GameMode::Custom, "m".to_string())
            .unwrap();
        mgr.set_room_state(r2, RoomState::InGame).unwrap();

        let waiting = mgr.list_waiting_rooms();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].name, "R1");
    }

    #[test]
    fn test_matchmaker_find_match() {
        let mut mm = Matchmaker::new(2, 4);
        let now = std::time::Instant::now();

        mm.queue_player(QueuedPlayer {
            player_id: 1,
            skill: 1000.0,
            ping_ms: 50,
            preferred_mode: GameMode::QuickMatch,
            queued_at: now,
        });
        mm.queue_player(QueuedPlayer {
            player_id: 2,
            skill: 1050.0,
            ping_ms: 60,
            preferred_mode: GameMode::QuickMatch,
            queued_at: now,
        });

        let result = mm.find_match();
        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.len(), 2);
        assert_eq!(mm.queue_size(), 0);
    }

    #[test]
    fn test_matchmaker_no_match_skill_diff() {
        let mut mm = Matchmaker::new(2, 4);
        mm.max_skill_diff = 100.0;

        let now = std::time::Instant::now();
        mm.queue_player(QueuedPlayer {
            player_id: 1,
            skill: 1000.0,
            ping_ms: 50,
            preferred_mode: GameMode::QuickMatch,
            queued_at: now,
        });
        mm.queue_player(QueuedPlayer {
            player_id: 2,
            skill: 2000.0,
            ping_ms: 50,
            preferred_mode: GameMode::QuickMatch,
            queued_at: now,
        });

        let result = mm.find_match();
        assert!(result.is_none());
    }

    #[test]
    fn test_matchmaker_dequeue() {
        let mut mm = Matchmaker::new(2, 4);
        let now = std::time::Instant::now();
        mm.queue_player(QueuedPlayer {
            player_id: 1,
            skill: 1000.0,
            ping_ms: 50,
            preferred_mode: GameMode::QuickMatch,
            queued_at: now,
        });

        assert!(mm.dequeue_player(1));
        assert_eq!(mm.queue_size(), 0);
        assert!(!mm.dequeue_player(99));
    }

    #[test]
    fn test_lobby_add_ready_all() {
        let mut lobby = Lobby::new(1);
        lobby.add_player(1);
        lobby.add_player(2);

        assert!(!lobby.all_ready());
        lobby.set_ready(1, true).unwrap();
        assert!(!lobby.all_ready());
        lobby.set_ready(2, true).unwrap();
        assert!(lobby.all_ready());
    }

    #[test]
    fn test_lobby_team_assignment() {
        let mut lobby = Lobby::new(1);
        lobby.add_player(1);
        lobby.add_player(2);

        lobby.assign_team(1, 0);
        lobby.assign_team(2, 1);

        assert_eq!(lobby.get_team(1), Some(0));
        assert_eq!(lobby.get_team(2), Some(1));
    }

    #[test]
    fn test_lobby_manager_create_and_start() {
        let mut mgr = LobbyManager::new();
        mgr.create_lobby(1);
        mgr.get_lobby_mut(1).unwrap().add_player(1);
        mgr.get_lobby_mut(1).unwrap().set_ready(1, true).unwrap();

        assert!(mgr.try_start(1));
        assert_eq!(mgr.get_lobby(1).unwrap().state, LobbyState::Starting);
    }

    #[test]
    fn test_lobby_manager_not_all_ready() {
        let mut mgr = LobbyManager::new();
        mgr.create_lobby(1);
        mgr.get_lobby_mut(1).unwrap().add_player(1);
        mgr.get_lobby_mut(1).unwrap().add_player(2);
        mgr.get_lobby_mut(1).unwrap().set_ready(1, true).unwrap();

        assert!(!mgr.try_start(1));
    }

    #[test]
    fn test_room_state_transitions() {
        let mut mgr = RoomManager::new();
        let room_id = mgr
            .create_room("R".to_string(), 1, 4, GameMode::QuickMatch, "m".to_string())
            .unwrap();

        assert_eq!(mgr.get_room(room_id).unwrap().state, RoomState::Waiting);
        mgr.set_room_state(room_id, RoomState::InGame).unwrap();
        assert_eq!(mgr.get_room(room_id).unwrap().state, RoomState::InGame);
        mgr.set_room_state(room_id, RoomState::Closed).unwrap();
        assert_eq!(mgr.get_room(room_id).unwrap().state, RoomState::Closed);
    }

    #[test]
    fn test_room_cleanup_closed() {
        let mut mgr = RoomManager::new();
        let r1 = mgr
            .create_room(
                "R1".to_string(),
                1,
                4,
                GameMode::QuickMatch,
                "m".to_string(),
            )
            .unwrap();
        mgr.create_room(
            "R2".to_string(),
            2,
            4,
            GameMode::QuickMatch,
            "m".to_string(),
        )
        .unwrap();

        mgr.set_room_state(r1, RoomState::Closed).unwrap();
        mgr.cleanup_closed();

        assert_eq!(mgr.room_count(), 1);
    }

    #[test]
    fn test_predicted_input_roundtrip() {
        let msg = crate::message::NetworkMessage::PredictedInput {
            client_tick: 42,
            input_data: vec![1, 2, 3],
            is_predicted: true,
        };
        let bytes = msg.serialize();
        let deserialized = crate::message::NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            crate::message::NetworkMessage::PredictedInput {
                client_tick,
                input_data,
                is_predicted,
            } => {
                assert_eq!(client_tick, 42);
                assert_eq!(input_data, vec![1, 2, 3]);
                assert!(is_predicted);
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_create_room_message_roundtrip() {
        let msg = crate::message::NetworkMessage::CreateRoom {
            name: "Test".to_string(),
            max_players: 16,
            game_mode: "deathmatch".to_string(),
        };
        let bytes = msg.serialize();
        let deserialized = crate::message::NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            crate::message::NetworkMessage::CreateRoom {
                name,
                max_players,
                game_mode,
            } => {
                assert_eq!(name, "Test");
                assert_eq!(max_players, 16);
                assert_eq!(game_mode, "deathmatch");
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_join_room_message_roundtrip() {
        let msg = crate::message::NetworkMessage::JoinRoom { room_id: 42 };
        let bytes = msg.serialize();
        let deserialized = crate::message::NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            crate::message::NetworkMessage::JoinRoom { room_id } => assert_eq!(room_id, 42),
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_leave_room_message_roundtrip() {
        let msg = crate::message::NetworkMessage::LeaveRoom;
        let bytes = msg.serialize();
        let deserialized = crate::message::NetworkMessage::deserialize(&bytes).unwrap();
        assert!(matches!(
            deserialized,
            crate::message::NetworkMessage::LeaveRoom
        ));
    }

    #[test]
    fn test_ready_up_message_roundtrip() {
        let msg = crate::message::NetworkMessage::ReadyUp { ready: true };
        let bytes = msg.serialize();
        let deserialized = crate::message::NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            crate::message::NetworkMessage::ReadyUp { ready } => assert!(ready),
            _ => panic!("wrong type"),
        }
    }
}
