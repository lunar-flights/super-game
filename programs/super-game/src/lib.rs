use anchor_lang::prelude::*;

pub mod ai;
pub mod errors;
pub mod instructions;
pub mod states;

use instructions::*;
use states::{BuildingType, MapSize, UnitType};

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

    pub fn join_game(ctx: Context<JoinGame>) -> Result<()> {
        instructions::create_game::join_game(ctx)
    }

    pub fn move_unit(
        ctx: Context<MoveUnit>,
        from_row: u8,
        from_col: u8,
        to_row: u8,
        to_col: u8,
    ) -> Result<()> {
        instructions::move_unit(
            ctx,
            from_row.into(),
            from_col.into(),
            to_row.into(),
            to_col.into(),
        )
    }

    pub fn recruit_units(
        ctx: Context<RecruitUnits>,
        unit_type: UnitType,
        quantity: u16,
        row: u8,
        col: u8,
    ) -> Result<()> {
        instructions::recruit_units(ctx, unit_type, quantity, row, col)
    }

    pub fn build_construction(
        ctx: Context<BuildConstruction>,
        row: u8,
        col: u8,
        building_type: BuildingType,
    ) -> Result<()> {
        instructions::build_construction(ctx, row.into(), col.into(), building_type)
    }

    pub fn end_turn(ctx: Context<EndTurn>) -> Result<()> {
        instructions::end_turn::end_turn(ctx)
    }
}
