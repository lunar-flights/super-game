use crate::errors::UnitError;
use crate::states::{Game, MapSize, Units};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct MoveUnit<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player: Signer<'info>,
}

pub fn move_unit(ctx: Context<MoveUnit>, from_tile_index: u8, to_tile_index: u8) -> Result<()> {
    let game = &mut ctx.accounts.game;
    let player_pubkey = ctx.accounts.player.key();

    let mut from_tile = *game
        .tiles
        .get(from_tile_index as usize)
        .ok_or(ProgramError::InvalidArgument)?;
    let mut to_tile = *game
        .tiles
        .get(to_tile_index as usize)
        .ok_or(ProgramError::InvalidArgument)?;

    if from_tile.owner != player_pubkey {
        // return err!(UnitError::NotYourTile);
    }

    if from_tile.units.infantry == 0 && from_tile.units.tank == 0 && from_tile.units.plane == 0 {
        return err!(UnitError::NoUnitsToMove);
    }

    if !is_adjacent(game.map_size.clone(), from_tile_index, to_tile_index) {
        return err!(UnitError::InvalidMovement);
    }

    to_tile.units.infantry += from_tile.units.infantry;
    to_tile.units.tank += from_tile.units.tank;
    to_tile.units.plane += from_tile.units.plane;

    from_tile.units = Units::default();

    game.tiles[from_tile_index as usize] = from_tile;
    game.tiles[to_tile_index as usize] = to_tile;

    Ok(())
}

fn is_adjacent(map_size: MapSize, from_tile: u8, to_tile: u8) -> bool {
    let small_map: &[u8] = &[3, 5, 7, 7, 7, 5, 3];
    let large_map: &[u8] = &[3, 5, 7, 9, 9, 9, 7, 5, 3];
    let tile_counts_per_row = match map_size {
        MapSize::Small => small_map,
        MapSize::Large => large_map,
    };

    let (from_row, from_col) = get_tile_coordinates(tile_counts_per_row, from_tile);
    let (to_row, to_col) = get_tile_coordinates(tile_counts_per_row, to_tile);

    let row_diff = (from_row as i8 - to_row as i8).abs();
    let col_diff = (from_col as i8 - to_col as i8).abs();

    row_diff <= 1 && col_diff <= 1
}

fn get_tile_coordinates(tile_counts_per_row: &[u8], tile_index: u8) -> (usize, usize) {
    let mut index = 0;
    for (row, &tiles_in_row) in tile_counts_per_row.iter().enumerate() {
        let max_tiles_in_row = *tile_counts_per_row.iter().max().unwrap();

        // add offset due to diamond-shaped map
        let offset = (max_tiles_in_row - tiles_in_row) / 2;

        if tile_index >= index && tile_index < index + tiles_in_row {
            let col = tile_index - index;
            return (row, (col + offset) as usize);
        }
        index += tiles_in_row;
    }
    // out of bounds
    (0, 0)
}
