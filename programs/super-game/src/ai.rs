use crate::errors::GameError;
use crate::states::*;
use anchor_lang::prelude::*;

struct MoveAction {
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
}

const DESIRED_UNIT_QUANTITY: u16 = 30;

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

    let mut bot_tiles = Vec::new();

    for (row_index, row) in game.tiles.iter().enumerate() {
        for (col_index, tile_option) in row.iter().enumerate() {
            if let Some(tile) = tile_option {
                if tile.owner == bot_pubkey {
                    bot_tiles.push((row_index, col_index));
                }
            }
        }
    }

    let (total_units, base_level) = get_stats(game, &bot_tiles);

    if (base_level == 1 && total_units >= 5) || (base_level == 2 && total_units >= 20) {
        upgrade_base(game, bot_index, &bot_tiles)?;
    } else if base_level >= 2
        && total_units > 10
        && !has_building(game, &bot_tiles, BuildingType::GasPlant)
    {
        build_constructions(game, bot_index, &bot_tiles)?;
    } else {
        recruit_units(game, bot_index, &bot_tiles)?;
    }

    attack_adjacent_tiles(game, bot_pubkey, &bot_tiles)?;

    Ok(())
}

fn attack_adjacent_tiles(
    game: &mut Game,
    bot_pubkey: Pubkey,
    bot_tiles: &[(usize, usize)],
) -> Result<()> {
    let mut pending_moves = Vec::new();

    for &(row_index, col_index) in bot_tiles {
        let tile = &game.tiles[row_index][col_index];
        if let Some(tile) = tile {
            if let Some(units) = &tile.units {
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
                                    units.quantity as u32 * units.unit_type.strength() as u32;

                                let defense_bonus = adj_tile.get_defense_bonus() as u32;

                                let adjusted_bot_strength =
                                    bot_strength.saturating_sub(defense_bonus);

                                if adjusted_bot_strength == 0 {
                                    continue;
                                }

                                let opponent_unit_strength =
                                    if let Some(opponent_units) = &adj_tile.units {
                                        opponent_units.quantity as u32
                                            * opponent_units.unit_type.strength() as u32
                                    } else {
                                        0
                                    };

                                let opponent_building_strength =
                                    if let Some(building) = &adj_tile.building {
                                        building.get_strength() as u32
                                    } else {
                                        0
                                    };

                                let opponent_strength =
                                    opponent_unit_strength + opponent_building_strength;

                                if adjusted_bot_strength > opponent_strength {
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

    let mut players_to_update: Vec<Pubkey> = Vec::new();

    for action in pending_moves {
        if action.from_row == action.to_row && action.from_col == action.to_col {
            continue;
        }

        let (from_tile_option, to_tile_option) = get_tile_options(game, action)?;

        let from_tile = from_tile_option.as_mut().ok_or(GameError::InvalidTile)?;
        let to_tile = to_tile_option.as_mut().ok_or(GameError::InvalidTile)?;

        if let Some(destroyed_player_pubkey) = handle_attack(from_tile, to_tile, bot_pubkey)? {
            players_to_update.push(destroyed_player_pubkey);
        }
    }

    for player_pubkey in players_to_update {
        update_player_status(game, player_pubkey, false);
    }

    Ok(())
}

fn handle_attack(
    from_tile: &mut Tile,
    to_tile: &mut Tile,
    bot_pubkey: Pubkey,
) -> Result<Option<Pubkey>> {
    let from_units = from_tile.units.as_ref().ok_or(GameError::InvalidTile)?;

    let from_unit_type = from_units.unit_type;
    let from_unit_quantity = from_units.quantity;
    let from_unit_strength = from_units.unit_type.strength() as u32;
    let from_unit_stamina = from_units.stamina;

    let move_cost = 1;

    from_tile.units = None;

    let attacker_strength = from_unit_quantity as u32 * from_unit_strength;

    let defense_bonus = to_tile.get_defense_bonus() as u32;
    let adjusted_attacker_strength = attacker_strength.saturating_sub(defense_bonus);

    if adjusted_attacker_strength == 0 {
        return Ok(None);
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

    let mut base_destroyed_player: Option<Pubkey> = None;

    match adjusted_attacker_strength.cmp(&defender_strength) {
        std::cmp::Ordering::Equal => {
            // Both units die
            to_tile.units = None;

            if let Some(building) = &to_tile.building {
                if let BuildingType::Base = building.building_type {
                    base_destroyed_player = Some(to_tile.owner);
                    to_tile.building = None;
                }
            }
        }
        std::cmp::Ordering::Less => {
            // Attacker loses

            let remaining_defender_strength =
                defender_unit_strength.saturating_sub(adjusted_attacker_strength);
            if defender_unit_strength > 0 {
                let unit_strength = to_tile.units.as_ref().unwrap().unit_type.strength() as u32;
                let remaining_defender_units =
                    (remaining_defender_strength + unit_strength - 1) / unit_strength;
                let original_quantity = to_tile.units.as_ref().unwrap().quantity;
                to_tile.units.as_mut().unwrap().quantity =
                    remaining_defender_units.min(original_quantity as u32) as u16;
            } else {
                to_tile.units = None;
            }
        }
        std::cmp::Ordering::Greater => {
            // Attacker wins

            let remaining_attacker_strength = adjusted_attacker_strength - defender_strength;
            let remaining_attacker_units =
                (remaining_attacker_strength + from_unit_strength - 1) / from_unit_strength;
            let remaining_stamina = from_unit_stamina.saturating_sub(move_cost);

            to_tile.units = Some(Units {
                unit_type: from_unit_type,
                quantity: remaining_attacker_units as u16,
                stamina: remaining_stamina,
            });

            if let Some(building) = &to_tile.building {
                if let BuildingType::Base = building.building_type {
                    base_destroyed_player = Some(to_tile.owner);
                    to_tile.building = None;
                }
            }
            to_tile.owner = bot_pubkey;
        }
    }

    Ok(base_destroyed_player)
}

fn recruit_units(game: &mut Game, bot_index: usize, bot_tiles: &[(usize, usize)]) -> Result<()> {
    let bot = game.players[bot_index]
        .as_mut()
        .ok_or(GameError::InvalidPlayer)?;

    let infantry_cost = UnitType::Infantry.cost() as u32;

    for &(row_index, col_index) in bot_tiles {
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

            let affordable_units = (bot.balance / infantry_cost).min(units_needed as u32) as u16;

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
    Ok(())
}

fn upgrade_base(game: &mut Game, bot_index: usize, tiles: &[(usize, usize)]) -> Result<()> {
    let bot = game.players[bot_index]
        .as_mut()
        .ok_or(GameError::InvalidPlayer)?;

    let base_upgrade_costs = [0, 12, 22];
    let max_base_level = 3;

    for &(row_index, col_index) in tiles {
        let tile = game.tiles[row_index][col_index]
            .as_mut()
            .ok_or(GameError::InvalidTile)?;

        if let Some(building) = &mut tile.building {
            if let BuildingType::Base = building.building_type {
                if building.level < max_base_level {
                    let upgrade_cost = base_upgrade_costs[building.level as usize];
                    if bot.balance >= upgrade_cost {
                        bot.balance -= upgrade_cost;
                        building.level += 1;
                    }
                }
            }
        }
    }

    Ok(())
}

fn build_constructions(
    game: &mut Game,
    bot_index: usize,
    bot_tiles: &[(usize, usize)],
) -> Result<()> {
    let bot = game.players[bot_index]
        .as_mut()
        .ok_or(GameError::InvalidPlayer)?;

    let cost = 12;

    if bot.balance >= cost {
        let mut has_gas_plant = false;
        for &(row_index, col_index) in bot_tiles {
            let tile = game.tiles[row_index][col_index]
                .as_mut()
                .ok_or(GameError::InvalidTile)?;
            if let Some(building) = &tile.building {
                if let BuildingType::GasPlant = building.building_type {
                    has_gas_plant = true;
                    break;
                }
            }
        }

        if !has_gas_plant {
            for &(row_index, col_index) in bot_tiles {
                let tile = game.tiles[row_index][col_index]
                    .as_mut()
                    .ok_or(GameError::InvalidTile)?;
                if tile.building.is_none() {
                    tile.building = Some(Building {
                        building_type: BuildingType::GasPlant,
                        level: 1,
                    });
                    bot.balance -= cost;
                    break;
                }
            }
        }
    }

    Ok(())
}

fn get_stats(game: &Game, bot_tiles: &[(usize, usize)]) -> (u16, u8) {
    let mut total_units = 0;
    let mut base_level = 0;

    for &(row_index, col_index) in bot_tiles {
        if let Some(tile) = &game.tiles[row_index][col_index] {
            if let Some(units) = &tile.units {
                total_units += units.quantity;
            }
            if let Some(building) = &tile.building {
                if let BuildingType::Base = building.building_type {
                    base_level = building.level;
                }
            }
        }
    }

    (total_units, base_level)
}

fn has_building(
    game: &Game,
    bot_tile_positions: &[(usize, usize)],
    building_type: BuildingType,
) -> bool {
    for &(row_index, col_index) in bot_tile_positions {
        if let Some(tile) = &game.tiles[row_index][col_index] {
            if let Some(building) = &tile.building {
                if building.building_type == building_type {
                    return true;
                }
            }
        }
    }
    false
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

fn get_tile_options(
    game: &mut Game,
    action: MoveAction,
) -> Result<(&mut Option<Tile>, &mut Option<Tile>)> {
    let from_tile_option;
    let to_tile_option;

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
            return err!(GameError::InvalidTile);
        }
    }

    Ok((from_tile_option, to_tile_option))
}
