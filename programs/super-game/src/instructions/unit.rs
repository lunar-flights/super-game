use crate::errors::{GameError, UnitError};
use crate::states::{BuildingType, Game, PlayerInfo, Tile, Units};
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

    validate_positions(game, from_row, from_col, to_row, to_col)?;
    let (mut from_tile, mut to_tile) = get_tiles(game, from_row, from_col, to_row, to_col)?;

    if from_tile.owner != player_pubkey {
        return err!(UnitError::NotYourTile);
    }

    let from_units = match from_tile.units {
        Some(units) => units,
        None => return err!(UnitError::NoUnitsToMove),
    };

    deduct_attack_points(game, player_pubkey, &to_tile)?;

    let move_cost = calculate_move_cost(from_row, from_col, to_row, to_col)?;

    if from_units.stamina < move_cost {
        return err!(UnitError::NotEnoughStamina);
    }

    if to_tile.owner == player_pubkey {
        handle_move(from_units, &mut from_tile, &mut to_tile, move_cost)?;
    } else {
        let player_index = game
            .players
            .iter()
            .position(|p| {
                if let Some(player_info) = p {
                    player_info.pubkey == player_pubkey
                } else {
                    false
                }
            })
            .ok_or(GameError::InvalidPlayer)?;

        if player_index != game.current_player_index as usize {
            return err!(GameError::NotYourTurn);
        }

        handle_attack(
            game,
            from_units,
            &mut from_tile,
            &mut to_tile,
            player_pubkey,
            move_cost,
        )?;
    }

    game.tiles[from_row][from_col] = Some(from_tile);
    game.tiles[to_row][to_col] = Some(to_tile);

    Ok(())
}

fn validate_positions(
    game: &Game,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
) -> Result<()> {
    if from_row >= game.tiles.len()
        || from_col >= game.tiles[0].len()
        || to_row >= game.tiles.len()
        || to_col >= game.tiles[0].len()
    {
        return err!(GameError::OutOfBounds);
    }
    Ok(())
}

fn get_tiles(
    game: &Game,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
) -> Result<(Tile, Tile)> {
    let from_tile = game.tiles[from_row][from_col].ok_or(UnitError::InvalidTile)?;
    let to_tile = game.tiles[to_row][to_col].ok_or(UnitError::InvalidTile)?;
    Ok((from_tile, to_tile))
}

fn deduct_attack_points(game: &mut Game, player_pubkey: Pubkey, to_tile: &Tile) -> Result<()> {
    if to_tile.owner != player_pubkey {
        let player_info = get_player_info_mut(game, player_pubkey)?;
        if player_info.attack_points < 1 {
            return err!(UnitError::NotEnoughAttackPoints);
        }
        player_info.attack_points -= 1;
    }
    Ok(())
}

fn get_player_info_mut(game: &mut Game, player_pubkey: Pubkey) -> Result<&mut PlayerInfo> {
    let player_index = game
        .players
        .iter()
        .position(|player_option| {
            player_option
                .as_ref()
                .map_or(false, |player_info| player_info.pubkey == player_pubkey)
        })
        .ok_or(GameError::InvalidPlayer)?;

    let player = game.players[player_index]
        .as_mut()
        .ok_or(GameError::InvalidPlayer)?;

    Ok(player)
}

fn calculate_move_cost(
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
) -> Result<u8> {
    if !is_valid_move(from_row, from_col, to_row, to_col) {
        return err!(UnitError::InvalidMovement);
    }

    let row_diff = (from_row as isize - to_row as isize).unsigned_abs();
    let col_diff = (from_col as isize - to_col as isize).unsigned_abs();
    let diagonal_move = row_diff == 1 && col_diff == 1;
    Ok(if diagonal_move { 2 } else { 1 })
}

fn is_valid_move(from_row: usize, from_col: usize, to_row: usize, to_col: usize) -> bool {
    let row_diff = (from_row as isize - to_row as isize).unsigned_abs();
    let col_diff = (from_col as isize - to_col as isize).unsigned_abs();
    // Move is valid if the target tile is adjacent, including diagonals
    row_diff <= 1 && col_diff <= 1
}

fn handle_move(
    from_units: Units,
    from_tile: &mut Tile,
    to_tile: &mut Tile,
    move_cost: u8,
) -> Result<()> {
    if let Some(to_units) = &to_tile.units.clone() {
        if from_units.unit_type == to_units.unit_type {
            // Merge units
            let new_quantity = from_units.quantity + to_units.quantity;
            let new_stamina = (from_units.stamina - move_cost).min(to_units.stamina);
            to_tile.units = Some(Units {
                unit_type: from_units.unit_type,
                quantity: new_quantity,
                stamina: new_stamina,
            });
            from_tile.units = None;
        } else {
            // Swap units if possible
            if from_units.stamina >= move_cost && to_units.stamina >= move_cost {
                let mut from_units_moved = from_units;
                let mut to_units_moved = *to_units;

                from_units_moved.stamina -= move_cost;
                to_units_moved.stamina -= move_cost;

                from_tile.units = Some(to_units_moved);
                to_tile.units = Some(from_units_moved);
            } else {
                return err!(UnitError::TileOccupiedByOtherUnitType);
            }
        }
    } else {
        // Move units to empty friendly tile
        to_tile.units = Some(Units {
            unit_type: from_units.unit_type,
            quantity: from_units.quantity,
            stamina: from_units.stamina - move_cost,
        });
        from_tile.units = None;
    }
    Ok(())
}

fn handle_attack(
    game: &mut Game,
    from_units: Units,
    from_tile: &mut Tile,
    to_tile: &mut Tile,
    player_pubkey: Pubkey,
    move_cost: u8,
) -> Result<()> {
    let attacker_strength = from_units.quantity as u32 * from_units.unit_type.strength() as u32;

    let defense_bonus = to_tile.get_defense_bonus() as u32;
    let adjusted_attacker_strength = attacker_strength.saturating_sub(defense_bonus);

    if adjusted_attacker_strength == 0 {
        from_tile.units = None;
        return Ok(());
    }

    let defender_unit_strength = if let Some(to_units) = &to_tile.units {
        to_units.quantity as u32 * to_units.unit_type.strength() as u32
    } else {
        0
    };

    let defender_building_strength = if let Some(building) = &to_tile.building {
        building.get_strength() as u32
    } else {
        0
    };

    let defender_strength = defender_unit_strength + defender_building_strength;

    match adjusted_attacker_strength.cmp(&defender_strength) {
        std::cmp::Ordering::Equal => {
            // Both units die
            from_tile.units = None;
            to_tile.units = None;

            if let Some(building) = &to_tile.building {
                if let BuildingType::Base = building.building_type {
                    update_player_status(game, to_tile.owner, false);
                    to_tile.building = None;
                }
            }
        }
        std::cmp::Ordering::Less => {
            // Attacker loses
            from_tile.units = None;

            let remaining_defender_strength =
                defender_unit_strength.saturating_sub(adjusted_attacker_strength);
            if defender_unit_strength > 0 {
                let unit_strength = to_tile.units.as_ref().unwrap().unit_type.strength() as u32;
                let remaining_defender_units =
                    (remaining_defender_strength + unit_strength - 1) / unit_strength;
                to_tile.units = Some(Units {
                    unit_type: to_tile.units.as_ref().unwrap().unit_type,
                    quantity: remaining_defender_units as u16,
                    stamina: to_tile.units.as_ref().unwrap().stamina,
                });
            }
        }
        std::cmp::Ordering::Greater => {
            // Attacker wins
            from_tile.units = None;

            let remaining_attacker_strength = adjusted_attacker_strength - defender_strength;
            let unit_strength = from_units.unit_type.strength() as u32;
            let remaining_attacker_units =
                (remaining_attacker_strength + unit_strength - 1) / unit_strength;
            let remaining_stamina = from_units.stamina - move_cost;

            to_tile.units = Some(Units {
                unit_type: from_units.unit_type,
                quantity: remaining_attacker_units as u16,
                stamina: remaining_stamina,
            });

            if let Some(building) = &to_tile.building {
                if let BuildingType::Base = building.building_type {
                    update_player_status(game, to_tile.owner, false);
                    to_tile.building = None;
                }
            }
            to_tile.owner = player_pubkey;
        }
    }
    Ok(())
}

fn update_player_status(game: &mut Game, player_pubkey: Pubkey, is_alive: bool) {
    if let Some(player_index) = game.players.iter().position(|player_option| {
        if let Some(player_info) = player_option {
            player_info.pubkey == player_pubkey
        } else {
            false
        }
    }) {
        if let Some(player_info) = &mut game.players[player_index] {
            player_info.is_alive = is_alive;
        }
    }
}
