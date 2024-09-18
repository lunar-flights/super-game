use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod states;

use instructions::*;
use states::MapSize;

declare_id!("GnbCZsVXcRXVegmrQj99eSXjoQWTV1K72KYM6yocoP9S");

#[program]
pub mod super_game {
    use states::MapSize;

    use super::*;

    pub fn initialize_program(ctx: Context<InitializeSuper>) -> Result<()> {
        instructions::initialize_program::initialize_super(ctx)
    }

    pub fn create_player_profile(ctx: Context<CreatePlayerProfile>) -> Result<()> {
        instructions::player_profile::create_player_profile(ctx)
    }

    pub fn create_game(
        ctx: Context<CreateGame>,
        max_players: u8,
        is_multiplayer: bool,
        map_size: MapSize,
    ) -> Result<()> {
        instructions::create_game::create_game(ctx, max_players, is_multiplayer, map_size)
    }

    pub fn move_unit(ctx: Context<MoveUnit>, from_tile_index: u8, to_tile_index: u8) -> Result<()> {
        instructions::move_unit(ctx, from_tile_index, to_tile_index)
    }
}
