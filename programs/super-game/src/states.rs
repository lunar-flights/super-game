use anchor_lang::prelude::*;

#[account]
pub struct SuperState {
    pub game_count: u32,
}

impl SuperState {
    pub const LEN: usize = 8 + 4;
}

#[account]
pub struct PlayerProfile {
    pub player: Pubkey,
    pub experience: u32,
    pub completed_games: u32,
    pub active_games: Vec<Pubkey>,
}

impl PlayerProfile {
    pub const MAX_ACTIVE_GAMES: usize = 10;
    pub const LEN: usize = 8 + 32 + 4 + 4 + 4 + (32 * Self::MAX_ACTIVE_GAMES);
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GameStatus {
    NotStarted,
    Live,
    Completed,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum MapSize {
    Small,
    Large,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
pub struct PlayerInfo {
    pub pubkey: Pubkey,
    pub is_bot: bool,
}

#[account]
pub struct Game {
    pub game_id: u32,
    pub creator: Pubkey,
    pub players: [Option<PlayerInfo>; 4],
    pub status: GameStatus,
    pub max_players: u8,
    pub is_multiplayer: bool,
    pub map_size: MapSize,
    pub tiles: Vec<Tile>,
}

impl Game {
    pub const MAX_PLAYERS: usize = 4;
    pub const MAX_TILES: usize = 57;

    // ~ 2119 bytes
    pub const LEN: usize =
        8 + 4 + 32 + (Self::MAX_PLAYERS * 33) + 1 + 1 + 1 + 1 + (4 + (Self::MAX_TILES * Tile::LEN));
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Units {
    pub infantry: u8,
    pub tank: u8,
    pub plane: u8,
}

impl Units {
    pub const LEN: usize = 1 + 1 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct Tile {
    pub owner: Pubkey,
    pub level: u8,
    pub mutants: u8,
    pub units: Units,
    pub buildings: u8,
    pub is_base: bool,
}

impl Tile {
    pub const LEN: usize = 32 + 1 + 1 + Units::LEN + 1 + 1;

    pub fn new(level: u8) -> Self {
        let mutants = Self::default_mutants(level);
        Self {
            owner: Pubkey::default(),
            level,
            mutants,
            units: Units::default(),
            buildings: 0,
            is_base: false,
        }
    }

    pub fn get_yield(&self) -> u8 {
        match self.level {
            1 => 0,
            2 => 1,
            3 => 2,
            _ => 0,
        }
    }

    pub fn get_defense_bonus(&self) -> u8 {
        // Mutants don't get any bonus
        if self.mutants > 0 {
            0
        } else {
            self.level
        }
    }

    pub fn default_mutants(level: u8) -> u8 {
        match level {
            1 => 1,
            2 => 3,
            3 => 8,
            _ => 0,
        }
    }
}
