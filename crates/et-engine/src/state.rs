use serde::{Deserialize, Serialize};

/// Which seat a participant occupies. The game is strictly two-player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Player {
    First,
    Second,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::First => Player::Second,
            Player::Second => Player::First,
        }
    }
}

/// Terminal result of a game, from the perspective of the whole game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Win(Player),
    Draw,
}

/// A legal move. The concrete variants will be defined once the rules are
/// written down; the placeholder exists so dependent crates compile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Pass,
}

/// Full game state, including hidden information for both players.
/// Only the engine, the search's determinizer, and AI-vs-AI spectators
/// should ever hold one of these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub to_move: Player,
    pub turn: u32,
}

/// What a single player can see: their own hand, public zones, and
/// counts (not contents) of the opponent's hidden zones. This is the
/// information set that ISMCTS searches over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub viewer: Player,
    pub to_move: Player,
    pub turn: u32,
}

impl GameState {
    pub fn new() -> Self {
        Self { to_move: Player::First, turn: 0 }
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        vec![Action::Pass]
    }

    pub fn apply(&mut self, _action: &Action) {
        self.to_move = self.to_move.opponent();
        self.turn += 1;
    }

    pub fn outcome(&self) -> Option<Outcome> {
        None
    }

    pub fn observe(&self, viewer: Player) -> Observation {
        Observation { viewer, to_move: self.to_move, turn: self.turn }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_alternates_players() {
        let mut s = GameState::new();
        s.apply(&Action::Pass);
        assert_eq!(s.to_move, Player::Second);
        assert_eq!(s.turn, 1);
    }
}
