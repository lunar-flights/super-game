use crate::errors::UnitError;
use crate::states::{Game, Tile, Units};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct MoveUnit<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player: Signer<'info>,
}

pub fn move_unit(
    ctx: Context<MoveUnit>,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
) -> Result<()> {
    let game = &mut ctx.accounts.game;
    let player_pubkey = ctx.accounts.player.key();

    // Check if positions are within bounds
    if from_row >= game.tiles.len()
        || from_col >= game.tiles[0].len()
        || to_row >= game.tiles.len()
        || to_col >= game.tiles[0].len()
    {
        return Err(ProgramError::InvalidArgument.into());
    }

    let from_tile_option = game.tiles[from_row][from_col];
    let to_tile_option = game.tiles[to_row][to_col];

    let mut from_tile = match from_tile_option {
        Some(tile) => tile,
        None => return Err(UnitError::InvalidTile.into()),
    };

    let mut to_tile = match to_tile_option {
        Some(tile) => tile,
        None => return Err(UnitError::InvalidTile.into()),
    };

    if from_tile.owner != player_pubkey {
        return Err(UnitError::NotYourTile.into());
    }

    let from_units = match from_tile.units {
        Some(units) => units,
        None => return Err(UnitError::NoUnitsToMove.into()),
    };

    if !is_valid_move(
        &game.tiles,
        from_row,
        from_col,
        to_row,
        to_col,
        from_units.stamina,
    ) {
        return Err(UnitError::InvalidMovement.into());
    }

    // Move units to destination tile
    to_tile.units = Some(Units {
        stamina: from_units.stamina - 1, // FIX ME
        ..from_units
    });
    // Remove units from the initial tile
    from_tile.units = None;

    to_tile.owner = from_tile.owner;

    game.tiles[from_row][from_col] = Some(from_tile);
    game.tiles[to_row][to_col] = Some(to_tile);

    Ok(())
}

fn is_valid_move(
    grid: &[Vec<Option<Tile>>],
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    stamina: u8,
) -> bool {
    if to_row >= grid.len() || to_col >= grid[0].len() {
        return false;
    }

    if grid[to_row][to_col].is_none() {
        return false;
    }

    let row_diff = (from_row as isize - to_row as isize).unsigned_abs();
    let col_diff = (from_col as isize - to_col as isize).unsigned_abs();

    // diagonal movement costs 2 stamina, normal movement costs 1
    let diagonal_move = row_diff == 1 && col_diff == 1;
    let move_cost = if diagonal_move { 2 } else { 1 };

    (row_diff <= 1 && col_diff <= 1) && stamina >= move_cost
}
