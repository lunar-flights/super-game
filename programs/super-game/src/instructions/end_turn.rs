use crate::errors::GameError;
use crate::states::*;
use anchor_lang::prelude::*;

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
        const MAX_PLAYERS: usize = Game::MAX_PLAYERS;
        let mut player_pubkeys = [Pubkey::default(); MAX_PLAYERS];
        let mut incomes = [0u32; MAX_PLAYERS];

        for (player_index, player_option) in game.players.iter().enumerate() {
            if let Some(player_info) = player_option {
                player_pubkeys[player_index] = player_info.pubkey;
                incomes[player_index] = 0;
            } else {
                player_pubkeys[player_index] = Pubkey::default();
                incomes[player_index] = 0;
            }
        }

        let num_players = game.players.len();
        for row in &mut game.tiles {
            for tile_option in row.iter_mut().flatten() {
                let tile = tile_option;

                // Restore stamina for all units
                if let Some(units) = &mut tile.units {
                    units.stamina = units.unit_type.max_stamina();
                }

                // Accumulate income from tiles based on ownership
                for player_index in 0..num_players {
                    if tile.owner == player_pubkeys[player_index] {
                        let tile_yield = tile.get_yield() as u32;
                        incomes[player_index] = incomes[player_index].saturating_add(tile_yield);
                        break;
                    }
                }
            }
        }

        for (player_index, player_option) in game.players.iter_mut().enumerate() {
            if let Some(player_info) = player_option {
                let income = incomes[player_index];
                player_info.balance = player_info.balance.saturating_add(income);
            }
        }

        game.round += 1;
    }

    if game.is_multiplayer {
        // Find the next human player
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
    }

    game.turn_timestamp = current_timestamp;

    Ok(())
}
