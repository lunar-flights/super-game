use crate::errors::GameError;
use crate::states::*;
use anchor_lang::prelude::*;

struct MoveAction {
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
}

const DESIRED_UNIT_QUANTITY: u16 = 9;

/* Bots make decisions in the following order:
    1. Attack adjacent tiles if possible
    2. Recruit units
    3. Upgrade base if possible
    4. Build new constructions
*/
pub fn process_bot_turn(game: &mut Game, bot_index: usize) -> Result<()> {
    let bot_pubkey = game.players[bot_index]
        .as_ref()
        .ok_or(GameError::InvalidPlayer)?
        .pubkey;

    let mut bot_tile_positions = Vec::new();

    for (row_index, row) in game.tiles.iter().enumerate() {
        for (col_index, tile_option) in row.iter().enumerate() {
            if let Some(tile) = tile_option {
                if tile.owner == bot_pubkey {
                    bot_tile_positions.push((row_index, col_index));
                }
            }
        }
    }

    // 1) Attack adjacent tiles if possible
    {
        let mut pending_moves = Vec::new();

        for &(row_index, col_index) in &bot_tile_positions {
            let tile = &game.tiles[row_index][col_index];
            if let Some(tile) = tile {
                if let Some(units) = &tile.units {
                    // Only units with stamina can move
                    if units.stamina > 0 {
                        let adjacent_positions = get_adjacent_tiles(row_index, col_index, game);

                        for (adj_row, adj_col) in adjacent_positions {
                            if adj_row >= game.tiles.len() || adj_col >= game.tiles[adj_row].len() {
                                continue;
                            }

                            let adj_tile_option = &game.tiles[adj_row][adj_col];
                            if let Some(adj_tile) = adj_tile_option {
                                if adj_tile.owner != bot_pubkey {
                                    // Check if it makes sense to attack
                                    let bot_strength =
                                        units.quantity * units.unit_type.strength() as u16;
                                    let opponent_strength =
                                        if let Some(opponent_units) = &adj_tile.units {
                                            opponent_units.quantity
                                                * opponent_units.unit_type.strength() as u16
                                        } else {
                                            0
                                        };

                                    if bot_strength > opponent_strength {
                                        // Schedule the attack
                                        pending_moves.push(MoveAction {
                                            from_row: row_index,
                                            from_col: col_index,
                                            to_row: adj_row,
                                            to_col: adj_col,
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for action in pending_moves {
            if action.from_row == action.to_row && action.from_col == action.to_col {
                continue;
            }

            let (from_tile_option, to_tile_option);

            if action.from_row != action.to_row {
                let (from_row_slice, to_row_slice) = if action.from_row < action.to_row {
                    let (first, second) = game.tiles.split_at_mut(action.to_row);
                    (&mut first[action.from_row], &mut second[0])
                } else {
                    let (first, second) = game.tiles.split_at_mut(action.from_row);
                    (&mut second[0], &mut first[action.to_row])
                };
                from_tile_option = &mut from_row_slice[action.from_col];
                to_tile_option = &mut to_row_slice[action.to_col];
            } else {
                let row = &mut game.tiles[action.from_row];
                if action.from_col != action.to_col {
                    let (from_tile_ref, to_tile_ref) = if action.from_col < action.to_col {
                        let (first, second) = row.split_at_mut(action.to_col);
                        (&mut first[action.from_col], &mut second[0])
                    } else {
                        let (first, second) = row.split_at_mut(action.from_col);
                        (&mut second[0], &mut first[action.to_col])
                    };
                    from_tile_option = from_tile_ref;
                    to_tile_option = to_tile_ref;
                } else {
                    continue;
                }
            }

            let mut from_tile = from_tile_option.take().ok_or(GameError::InvalidTile)?;
            let mut to_tile = to_tile_option.take().ok_or(GameError::InvalidTile)?;
            let units = from_tile.units.take().ok_or(GameError::InvalidTile)?;
            let move_cost = 1;

            // Attack Logic
            if let Some(to_units) = &mut to_tile.units {
                // TODO: add tile defense bonus
                let attacker_strength = units.quantity as u32 * units.unit_type.strength() as u32;
                let defender_strength =
                    to_units.quantity as u32 * to_units.unit_type.strength() as u32;

                match attacker_strength.cmp(&defender_strength) {
                    std::cmp::Ordering::Equal => {
                        // Both attacker and defender units are destroyed
                        to_tile.units = None;
                    }
                    std::cmp::Ordering::Less => {
                        // Attacker lost
                        let remaining_strength = defender_strength - attacker_strength;
                        let unit_strength = to_units.unit_type.strength() as u32;
                        let mut remaining_defender_units = remaining_strength / unit_strength;
                        if remaining_strength % unit_strength != 0 {
                            remaining_defender_units += 1;
                        }
                        to_units.quantity = remaining_defender_units as u16;
                    }
                    std::cmp::Ordering::Greater => {
                        // Attacker won
                        let remaining_attacker_strength = attacker_strength - defender_strength;
                        let unit_strength = units.unit_type.strength() as u32;
                        let mut remaining_attacker_units =
                            remaining_attacker_strength / unit_strength;
                        if remaining_attacker_strength % unit_strength != 0 {
                            remaining_attacker_units += 1;
                        }

                        to_tile.owner = bot_pubkey;
                        to_tile.units = Some(Units {
                            unit_type: units.unit_type,
                            quantity: remaining_attacker_units as u16,
                            stamina: units.stamina - move_cost,
                        });
                        to_tile.building = None;
                    }
                }
            } else {
                // No units on the target tile, take it without combat
                if to_tile.owner != bot_pubkey {
                    to_tile.owner = bot_pubkey;
                    to_tile.building = None;
                }
                to_tile.units = Some(Units {
                    unit_type: units.unit_type,
                    quantity: units.quantity,
                    stamina: units.stamina - move_cost,
                });
            }

            *from_tile_option = Some(from_tile);
            *to_tile_option = Some(to_tile);
        }
    }

    // 2) Recruit units if possible
    {
        let bot = game.players[bot_index]
            .as_mut()
            .ok_or(GameError::InvalidPlayer)?;

        let infantry_cost = UnitType::Infantry.cost() as u32;

        // TODO: Sort tiles based on proximity to enemy tiles

        for &(row_index, col_index) in &bot_tile_positions {
            let tile = game.tiles[row_index][col_index]
                .as_mut()
                .ok_or(GameError::InvalidTile)?;

            let current_quantity = if let Some(units) = &tile.units {
                units.quantity
            } else {
                0
            };

            if current_quantity < DESIRED_UNIT_QUANTITY {
                let units_needed = DESIRED_UNIT_QUANTITY - current_quantity;

                let affordable_units =
                    (bot.balance / infantry_cost).min(units_needed as u32) as u16;

                if affordable_units > 0 {
                    bot.balance = bot
                        .balance
                        .saturating_sub(affordable_units as u32 * infantry_cost);

                    if let Some(units) = &mut tile.units {
                        units.quantity += affordable_units;
                    } else {
                        tile.units = Some(Units {
                            unit_type: UnitType::Infantry,
                            quantity: affordable_units,
                            stamina: UnitType::Infantry.max_stamina(),
                        });
                    }
                }

                // Bot is out of funds, no need to check other tiles
                if bot.balance < infantry_cost {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn get_adjacent_tiles(row: usize, col: usize, game: &Game) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    let max_row = game.tiles.len() as isize;
    let max_col = if max_row > 0 {
        game.tiles[0].len() as isize
    } else {
        0
    };

    let directions = vec![(-1, 0), (1, 0), (0, -1), (0, 1)];

    for (dx, dy) in directions {
        let new_row = row as isize + dx;
        let new_col = col as isize + dy;
        if new_row >= 0 && new_row < max_row && new_col >= 0 && new_col < max_col {
            positions.push((new_row as usize, new_col as usize));
        }
    }

    positions
}
