use crate::errors::{GameError, UnitError};
use crate::states::{BuildingType, Game, Tile, Units};
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
        return err!(GameError::OutOfBounds);
    }

    let from_tile_option = game.tiles[from_row][from_col];
    let to_tile_option = game.tiles[to_row][to_col];

    let mut from_tile = match from_tile_option {
        Some(tile) => tile,
        None => return err!(UnitError::InvalidTile),
    };

    let mut to_tile = match to_tile_option {
        Some(tile) => tile,
        None => return err!(UnitError::InvalidTile),
    };

    if from_tile.owner != player_pubkey {
        return err!(UnitError::NotYourTile);
    }

    let from_units = match from_tile.units {
        Some(units) => units,
        None => return err!(UnitError::NoUnitsToMove),
    };

    // Check if player has enough AP when moving to a tile owned by other player
    {
        let player_index = game
            .players
            .iter()
            .position(|player_option| {
                if let Some(player_info) = player_option {
                    player_info.pubkey == player_pubkey
                } else {
                    false
                }
            })
            .ok_or(GameError::InvalidPlayer)?;

        let player_info = game.players[player_index]
            .as_mut()
            .ok_or(GameError::InvalidPlayer)?;

        if to_tile.owner != player_pubkey {
            if player_info.attack_points < 1 {
                return err!(UnitError::NotEnoughAttackPoints);
            }
            player_info.attack_points -= 1;
        }
    }

    // diagonal moves cost 2 stamina, vertical/horizontal - 1 stamina
    let row_diff = (from_row as isize - to_row as isize).unsigned_abs();
    let col_diff = (from_col as isize - to_col as isize).unsigned_abs();
    let diagonal_move = row_diff == 1 && col_diff == 1;
    let move_cost = if diagonal_move { 2 } else { 1 };

    if !is_valid_move(&game.tiles, from_row, from_col, to_row, to_col) {
        return err!(UnitError::InvalidMovement);
    }

    if from_units.stamina < move_cost {
        return err!(UnitError::NotEnoughStamina);
    }

    // Here we have 4 scenarios:
    // 1) Destination tile belongs to player and it has units of the same type - units get merged
    // 2) Destination tile belongs to player, but it has different unit type.
    //    2a) They change positions (swap units between tiles) if both have enough stamina
    //    2b) Otherwise error that tile is occupied by other unit type.
    // 3) Destination tile belongs to enemy - attack logic applied
    // 4) Destination tile is not occupied by other units - move unit normally, change tile owner if needed.
    if let Some(to_units) = &to_tile.units.clone() {
        if to_tile.owner == player_pubkey {
            // 1) Destination tile is friendly & occupied by same unit type, merge them
            if from_units.unit_type == to_units.unit_type {
                let new_quantity = from_units.quantity + to_units.quantity;
                let new_stamina = (from_units.stamina - move_cost).min(to_units.stamina);
                let merged_units = Units {
                    unit_type: from_units.unit_type,
                    quantity: new_quantity,
                    stamina: new_stamina,
                };
                to_tile.units = Some(merged_units);
                from_tile.units = None;
            } else {
                // 2a) Units of different types, try to swap
                if from_units.stamina >= move_cost && to_units.stamina >= move_cost {
                    let mut from_units_moved = from_units;
                    let mut to_units_moved = *to_units;

                    from_units_moved.stamina -= move_cost;
                    to_units_moved.stamina -= move_cost;

                    from_tile.units = Some(to_units_moved);
                    to_tile.units = Some(from_units_moved);
                } else {
                    // 2b) Units in the destination tile don't have enough of stamina to swap positions
                    return err!(UnitError::TileOccupiedByOtherUnitType);
                }
            }
        } else {
            let attacker_strength =
                from_units.quantity as u32 * from_units.unit_type.strength() as u32;

            // Tile defense bonus applied before the attack
            let defense_bonus = to_tile.get_defense_bonus() as u32;
            let adjusted_attacker_strength = attacker_strength.saturating_sub(defense_bonus);

            if adjusted_attacker_strength == 0 {
                from_tile.units = None;

                game.tiles[from_row][from_col] = Some(from_tile);
                game.tiles[to_row][to_col] = Some(to_tile);
                return Ok(());
            }

            let defender_unit_strength =
                to_units.quantity as u32 * to_units.unit_type.strength() as u32;

            let defender_building_strength = match to_tile.building {
                Some(building) => building.get_strength() as u32,
                None => 0,
            };

            let defender_strength = defender_unit_strength + defender_building_strength;

            match adjusted_attacker_strength.cmp(&defender_strength) {
                std::cmp::Ordering::Equal => {
                    // Both attacker and defender units died
                    from_tile.units = None;
                    to_tile.units = None;

                    if let Some(building) = to_tile.building {
                        if let BuildingType::Base = building.building_type {
                            update_player_status(game, to_tile.owner, false);
                            to_tile.building = None;
                        }
                    }

                    game.tiles[from_row][from_col] = Some(from_tile);
                    game.tiles[to_row][to_col] = Some(to_tile);

                    return Ok(());
                }
                std::cmp::Ordering::Less => {
                    // Attacker lost
                    from_tile.units = None;

                    let remaining_defender_strength =
                        defender_unit_strength.saturating_sub(adjusted_attacker_strength);
                    let unit_strength = to_units.unit_type.strength() as u32;
                    let mut remaining_defender_units = remaining_defender_strength / unit_strength;
                    if remaining_defender_strength % unit_strength != 0 {
                        remaining_defender_units += 1;
                    }

                    to_tile.units = Some(Units {
                        unit_type: to_units.unit_type,
                        quantity: remaining_defender_units as u16,
                        stamina: to_units.stamina,
                    });

                    game.tiles[from_row][from_col] = Some(from_tile);
                    game.tiles[to_row][to_col] = Some(to_tile);

                    return Ok(());
                }
                std::cmp::Ordering::Greater => {
                    // Attacker won
                    to_tile.units = None;

                    let remaining_attacker_strength =
                        adjusted_attacker_strength.saturating_sub(defender_strength);
                    let unit_strength = from_units.unit_type.strength() as u32;
                    let mut remaining_attacker_units = remaining_attacker_strength / unit_strength;
                    if remaining_attacker_strength % unit_strength != 0 {
                        remaining_attacker_units += 1;
                    }
                    let remaining_stamina = from_units.stamina - move_cost;

                    from_tile.units = None;

                    to_tile.units = Some(Units {
                        unit_type: from_units.unit_type,
                        quantity: remaining_attacker_units as u16,
                        stamina: remaining_stamina,
                    });

                    if let Some(building) = to_tile.building {
                        if let BuildingType::Base = building.building_type {
                            update_player_status(game, to_tile.owner, false);
                            to_tile.building = None;
                        }
                    }
                    to_tile.owner = player_pubkey;

                    game.tiles[from_row][from_col] = Some(from_tile);
                    game.tiles[to_row][to_col] = Some(to_tile);

                    return Ok(());
                }
            }
        }
    } else {
        // 4) Destination tile is empty, move units normally
        let moved_units = Units {
            unit_type: from_units.unit_type,
            quantity: from_units.quantity,
            stamina: from_units.stamina - move_cost,
        };
        to_tile.units = Some(moved_units);
        from_tile.units = None;
        if to_tile.owner != from_tile.owner {
            to_tile.owner = from_tile.owner;
            to_tile.building = None;
        }
    }

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
) -> bool {
    if to_row >= grid.len() || to_col >= grid[0].len() {
        return false;
    }

    if grid[to_row][to_col].is_none() {
        return false;
    }

    let row_diff = (from_row as isize - to_row as isize).unsigned_abs();
    let col_diff = (from_col as isize - to_col as isize).unsigned_abs();

    // Move is valid if the target tile is adjacent, including diagonals
    // stamina check is implemented directly in move_unit
    row_diff <= 1 && col_diff <= 1
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
