use crate::ai::process_bot_turn;
use crate::errors::GameError;
use crate::states::*;
use anchor_lang::prelude::*;

const MAX_PLAYERS: usize = Game::MAX_PLAYERS;
const MAX_ATTACK_POINTS: u8 = Game::MAX_ATTACK_POINTS;

#[derive(Accounts)]
pub struct EndTurn<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player: Signer<'info>,
}

pub fn end_turn(ctx: Context<EndTurn>) -> Result<()> {
    let game = &mut ctx.accounts.game;
    let current_timestamp = Clock::get()?.unix_timestamp as u64;

    let player_pubkey = ctx.accounts.player.key();
    let current_player_info = game.players[game.current_player_index as usize]
        .as_ref()
        .ok_or(GameError::InvalidPlayer)?;

    if current_player_info.pubkey != player_pubkey {
        return err!(GameError::NotYourTurn);
    }

    if !game.is_multiplayer {
        process_single_player_turn(game)?;
    } else {
        process_multiplayer_turn(game)?;
    }

    game.turn_timestamp = current_timestamp;

    Ok(())
}

fn process_single_player_turn(game: &mut Game) -> Result<()> {
    process_bot_turns(game)?;

    let (player_pubkeys, mut incomes) = init_incomes(game);
    calculate_incomes(game, &mut incomes, &player_pubkeys)?;

    apply_incomes(game, &incomes, MAX_ATTACK_POINTS)?;

    remove_defeated_players(game)?;

    game.round += 1;

    Ok(())
}

fn init_incomes(game: &Game) -> ([Pubkey; MAX_PLAYERS], [u32; MAX_PLAYERS]) {
    let mut player_pubkeys = [Pubkey::default(); MAX_PLAYERS];
    let incomes = [0u32; MAX_PLAYERS];

    for (player_index, player_option) in game.players.iter().enumerate() {
        if let Some(player_info) = player_option {
            player_pubkeys[player_index] = player_info.pubkey;
        }
    }

    (player_pubkeys, incomes)
}

fn process_bot_turns(game: &mut Game) -> Result<()> {
    let num_players = game.players.len();
    for bot_index in 1..num_players {
        if let Some(bot_player) = &game.players[bot_index] {
            if bot_player.is_bot && bot_player.is_alive {
                process_bot_turn(game, bot_index)?;
            }
        }
    }
    Ok(())
}

fn calculate_incomes(
    game: &mut Game,
    incomes: &mut [u32],
    player_pubkeys: &[Pubkey],
) -> Result<()> {
    let num_players = game.players.len();

    for row in &mut game.tiles {
        for tile_option in row.iter_mut().flatten() {
            let tile = tile_option;

            // Restore stamina for units
            if let Some(units) = &mut tile.units {
                units.stamina = units.unit_type.max_stamina();
            }

            // Accumulate income from tiles and buildings
            for player_index in 0..num_players {
                if tile.owner == player_pubkeys[player_index] {
                    let tile_yield = tile.get_yield() as u32;
                    incomes[player_index] = incomes[player_index].saturating_add(tile_yield);
                    break;
                }
            }
        }
    }

    Ok(())
}

fn apply_incomes(game: &mut Game, incomes: &[u32], max_attack_points: u8) -> Result<()> {
    for (player_index, player_option) in game.players.iter_mut().enumerate() {
        if let Some(player_info) = player_option {
            let income = incomes[player_index];
            player_info.balance = player_info.balance.saturating_add(income);
            player_info.attack_points = (player_info.attack_points + 1).min(max_attack_points);
        }
    }

    Ok(())
}

fn process_multiplayer_turn(game: &mut Game) -> Result<()> {
    let num_players = game.players.len();

    for i in 1..=num_players {
        let next_index = (game.current_player_index as usize + i) % num_players;
        if let Some(player_info) = &game.players[next_index] {
            if !player_info.is_bot {
                game.current_player_index = next_index as u8;
                break;
            }
        }
    }

    if game.current_player_index == 0 {
        game.round += 1;
    }

    let (player_pubkeys, mut incomes) = init_incomes(game);
    calculate_incomes(game, &mut incomes, &player_pubkeys)?;

    apply_incomes(game, &incomes, MAX_ATTACK_POINTS)?;

    remove_defeated_players(game)?;

    Ok(())
}

use std::collections::HashMap;

fn remove_defeated_players(game: &mut Game) -> Result<()> {
    let mut player_alive_status: HashMap<Pubkey, bool> = HashMap::new();

    for player_info in game.players.iter().flatten() {
        player_alive_status.insert(player_info.pubkey, false);
    }

    for row in &game.tiles {
        for tile_option in row.iter().flatten() {
            let tile = tile_option;
            if let Some(building) = &tile.building {
                if building.building_type == BuildingType::Base {
                    // Player is alive if owns a base
                    if let Some(is_alive) = player_alive_status.get_mut(&tile.owner) {
                        *is_alive = true;
                    }
                }
            }
        }
    }

    for player_info in game.players.iter_mut().flatten() {
        if let Some(&is_alive) = player_alive_status.get(&player_info.pubkey) {
            player_info.is_alive = is_alive;
        }
    }

    // Completely remove all tiles of defeated players from the grid
    // TODO: looks fun, but in some edge cases it's not possible to continue without planes
    for row in &mut game.tiles {
        for tile_option in row.iter_mut() {
            if let Some(tile) = tile_option {
                if let Some(&is_alive) = player_alive_status.get(&tile.owner) {
                    if !is_alive {
                        *tile_option = None;
                    }
                }
            }
        }
    }

    let alive_players: Vec<&PlayerInfo> = game
        .players
        .iter()
        .filter_map(|player_option| player_option.as_ref())
        .filter(|player| player.is_alive)
        .collect();

    let alive_count = alive_players.len();

    if alive_count <= 1 {
        game.status = GameStatus::Completed;
    }

    if alive_count == 1 {
        game.winner = Some(alive_players[0].pubkey);
    }

    Ok(())
}
