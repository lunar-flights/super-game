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
    pub balance: u32,
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
    pub round: u32,
    pub current_player_index: u8,
    pub turn_timestamp: u64,
    pub turn_time_limit: u64,
    pub tiles: Vec<Vec<Option<Tile>>>,
}

impl Game {
    pub const MAX_PLAYERS: usize = 4;
    pub const MAX_TILES: usize = 81;

    // ~ 2119 bytes
    pub const LEN: usize = 5000;
    // 8 + 4 + 32 + (1 + (Self::MAX_PLAYERS * (32 + 1 + 4))) + 1 + 1 + 1 + 1 + (4 + (Self::MAX_TILES * Tile::LEN));

    pub fn get_map_layout(map_size: MapSize) -> Vec<u8> {
        match map_size {
            MapSize::Small => vec![3, 5, 7, 7, 7, 5, 3],
            MapSize::Large => vec![3, 5, 7, 9, 9, 9, 7, 5, 3],
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct Units {
    pub unit_type: UnitType,
    pub quantity: u16,
    pub stamina: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    Infantry,
    Tank,
    Plane,
    Mutants, // neutral
}

impl UnitType {
    pub fn max_stamina(&self) -> u8 {
        match self {
            UnitType::Infantry => 1,
            UnitType::Tank => 3,
            UnitType::Plane => 5,
            UnitType::Mutants => 0,
        }
    }

    pub fn strength(&self) -> u8 {
        match self {
            UnitType::Infantry => 1,
            UnitType::Tank => 3,
            UnitType::Plane => 4,
            UnitType::Mutants => 1,
        }
    }

    pub fn cost(&self) -> u8 {
        match self {
            UnitType::Infantry => 1,
            UnitType::Tank => 3,
            UnitType::Plane => 5,
            UnitType::Mutants => 0, // can't be purchased
        }
    }
}

impl Units {
    pub const LEN: usize = 1 + 2 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum BuildingType {
    Base,
    GasPlant,
    TankFactory,
    PlaneFactory,
    Fort,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct Building {
    pub building_type: BuildingType,
    pub level: u8,
}

impl Building {
    pub const LEN: usize = 1 + 1;

    pub fn get_yield(&self) -> u8 {
        match self.building_type {
            BuildingType::Base => match self.level {
                1 => 3,
                2 => 4,
                3 => 6,
                _ => 0,
            },
            BuildingType::GasPlant => 1,
            _ => 0,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct Tile {
    pub owner: Pubkey,
    pub level: u8,
    pub units: Option<Units>,
    pub building: Option<Building>,
}

impl Tile {
    pub const LEN: usize = 32 + 1 + 1 + Units::LEN + 1 + Building::LEN;

    pub fn new(level: u8) -> Self {
        let mutants = Self::default_mutants(level);
        Self {
            owner: Pubkey::default(),
            level,
            units: Some(Units {
                unit_type: UnitType::Mutants,
                quantity: mutants,
                stamina: 0,
            }),
            building: None,
        }
    }

    pub fn get_yield(&self) -> u8 {
        let tile_yield = match self.level {
            1 => 0,
            2 => 0,
            3 => 1,
            _ => 0,
        };

        let building_yield = if let Some(building) = &self.building {
            building.get_yield()
        } else {
            0
        };

        tile_yield + building_yield
    }

    pub fn is_neutral(&self) -> bool {
        self.owner == Pubkey::default()
    }

    pub fn get_defense_bonus(&self) -> u8 {
        // Mutants don't get any bonus
        if self.is_neutral() {
            0
        } else {
            let mut bonus = self.level;
            if let Some(building) = &self.building {
                if building.building_type == BuildingType::Fort {
                    bonus += building.level;
                }
            }
            bonus
        }
    }

    pub fn default_mutants(level: u8) -> u16 {
        match level {
            1 => 1,
            2 => 3,
            3 => 8,
            _ => 0,
        }
    }
}
