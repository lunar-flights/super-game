use crate::errors::GameError;
use crate::states::*;
use anchor_lang::solana_program::hash::{hashv, Hash};

use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateGame<'info> {
    #[account(mut, seeds = [b"SUPER"], bump)]
    pub super_state: Account<'info, SuperState>,
    #[account(mut, seeds = [b"PROFILE", player.key().as_ref()], bump, has_one = player)]
    pub creator_profile: Account<'info, PlayerProfile>,
    #[account(init, payer = player, space = Game::LEN, seeds = [b"GAME", &super_state.game_count.to_le_bytes()], bump)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_game(
    ctx: Context<CreateGame>,
    max_players: u8,
    is_multiplayer: bool,
    map_size: MapSize,
) -> Result<()> {
    let super_state = &mut ctx.accounts.super_state;
    let game = &mut ctx.accounts.game;
    let creator_profile = &mut ctx.accounts.creator_profile;

    if creator_profile.active_games.len() >= PlayerProfile::MAX_ACTIVE_GAMES {
        return err!(GameError::TooManyActiveGames);
    }
    creator_profile.active_games.push(game.key());

    let game_id = super_state.game_count;
    super_state.game_count += 1;

    game.game_id = game_id;
    game.creator = ctx.accounts.player.key();
    game.status = if is_multiplayer {
        GameStatus::NotStarted
    } else {
        GameStatus::Live
    };
    game.max_players = max_players;
    game.is_multiplayer = is_multiplayer;
    game.map_size = map_size;
    game.round = 1;
    game.current_player_index = 0;
    game.turn_time_limit = 60;
    game.turn_timestamp = if is_multiplayer {
        Clock::get().unwrap().unix_timestamp as u64
    } else {
        0
    };

    let total_players = game.max_players as usize;
    let humans = if game.is_multiplayer { 2 } else { 1 };
    let num_bots = total_players - humans;

    game.players = [None; Game::MAX_PLAYERS];

    let mut player_infos = Vec::with_capacity(total_players);

    game.players[0] = Some(PlayerInfo {
        pubkey: ctx.accounts.player.key(),
        is_bot: false,
        balance: 0,
    });
    player_infos.push(PlayerInfo {
        pubkey: ctx.accounts.player.key(),
        is_bot: false,
        balance: 0,
    });

    // Add bots if any
    for i in 1..=num_bots {
        let bot_info = PlayerInfo {
            pubkey: Pubkey::default(),
            is_bot: true,
            balance: 0,
        };
        game.players[i] = Some(bot_info);
        player_infos.push(bot_info);
    }

    game.tiles = initialize_tiles(&game.key(), &player_infos, &game.map_size)?;

    Ok(())
}

fn initialize_tiles(
    game_pubkey: &Pubkey,
    player_infos: &[PlayerInfo],
    map_size: &MapSize,
) -> Result<Vec<Vec<Option<Tile>>>> {
    let layout = Game::get_map_layout(map_size.clone());
    let grid_size = layout.len();

    // Initialize empty grid
    let mut grid: Vec<Vec<Option<Tile>>> = vec![vec![None; grid_size]; grid_size];

    let clock = Clock::get().unwrap();
    let slot = clock.slot;

    let base_positions = get_base_positions(map_size);

    let mut base_tile_to_player = std::collections::HashMap::new();
    for (player_info, &(row, col)) in player_infos.iter().zip(base_positions.iter()) {
        base_tile_to_player.insert((row, col), player_info);
    }

    for (row_index, &tiles_in_row) in layout.iter().enumerate() {
        let tiles_in_row = tiles_in_row as usize;
        let empty_spaces = (grid_size - tiles_in_row) / 2;

        for col_index in 0..tiles_in_row {
            let adjusted_col = col_index + empty_spaces;

            let tile_index = row_index * grid_size + adjusted_col;
            let level = get_random_tile_level(game_pubkey, tile_index, slot);
            let mut tile = Tile::new(level);

            // initalize base
            if let Some(player_info) = base_tile_to_player.get(&(row_index, adjusted_col)) {
                tile.owner = player_info.pubkey;
                tile.level = 1;
                tile.units = Some(Units {
                    unit_type: UnitType::Infantry,
                    quantity: 5,
                    stamina: 1,
                });
                tile.is_base = true;
            }

            grid[row_index][adjusted_col] = Some(tile);
        }
    }

    Ok(grid)
}

// 40% chance of level 1, 40% chance of level 2, 20% chance of level 3
fn get_random_tile_level(game_pubkey: &Pubkey, tile_index: usize, slot: u64) -> u8 {
    let seed_data = &[
        game_pubkey.as_ref(),
        &tile_index.to_le_bytes(),
        &slot.to_le_bytes(),
    ];

    let hash_result: Hash = hashv(seed_data);
    let random_number = hash_result.as_ref()[0] % 100 + 1;

    match random_number {
        1..=40 => 1,
        41..=80 => 2,
        81..=100 => 3,
        _ => 1,
    }
}

fn get_base_positions(map_size: &MapSize) -> Vec<(usize, usize)> {
    match map_size {
        MapSize::Small => vec![(1, 1), (1, 5), (5, 1), (5, 5)],
        MapSize::Large => vec![(0, 4), (4, 0), (4, 8), (8, 4)],
    }
}
