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

    // Determine total players and number of bots
    let total_players = game.max_players as usize;
    let humans = if game.is_multiplayer { 2 } else { 1 };
    let num_bots = total_players - humans;

    game.players = [None; Game::MAX_PLAYERS];

    let mut player_infos = Vec::with_capacity(total_players);

    // Game creator is always the first player
    game.players[0] = Some(PlayerInfo {
        pubkey: ctx.accounts.player.key(),
        is_bot: false,
    });
    player_infos.push(PlayerInfo {
        pubkey: ctx.accounts.player.key(),
        is_bot: false,
    });

    // Add bots if any
    for i in 1..=num_bots {
        let bot_info = PlayerInfo {
            pubkey: Pubkey::default(),
            is_bot: true,
        };
        game.players[i] = Some(bot_info);
        player_infos.push(bot_info);
    }

    let num_tiles = match game.map_size {
        MapSize::Small => 37,
        MapSize::Large => 57,
    };

    game.tiles = initialize_tiles(&game.key(), num_tiles, &player_infos, &game.map_size)?;

    Ok(())
}

fn initialize_tiles(
    game_pubkey: &Pubkey,
    num_tiles: usize,
    player_infos: &[PlayerInfo],
    map_size: &MapSize,
) -> Result<Vec<Tile>> {
    let mut tiles = Vec::with_capacity(num_tiles);
    let clock = Clock::get().unwrap();
    let slot = clock.slot;

    let base_positions = get_base_positions(map_size);

    // Create a mapping from tile index to player info
    let mut base_tile_to_player = std::collections::HashMap::new();
    for (player_info, &tile_index) in player_infos.iter().zip(base_positions.iter()) {
        base_tile_to_player.insert(tile_index, player_info);
    }

    for tile_index in 0..num_tiles {
        let level = get_random_tile_level(game_pubkey, tile_index, slot);
        let mut tile = Tile::new(level);

        if let Some(player_info) = base_tile_to_player.get(&tile_index) {
            tile.owner = player_info.pubkey;
            tile.level = 1;
            tile.mutants = 0;
            tile.units.infantry = 5;
            tile.is_base = true;
        }

        tiles.push(tile);
    }

    Ok(tiles)
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

fn get_base_positions(map_size: &MapSize) -> Vec<usize> {
    match map_size {
        MapSize::Large => vec![1, 24, 32, 55],
        MapSize::Small => vec![1, 35], // vec![1, 15, 21, 35],
    }
}
