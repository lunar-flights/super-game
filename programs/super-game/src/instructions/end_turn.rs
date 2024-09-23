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

    // Restore stamina for all units if game with bots only
    if !game.is_multiplayer {
        for row in &mut game.tiles {
            for tile in row.iter_mut().flatten() {
                // if tile.owner == player_pubkey {
                if let Some(units) = &mut tile.units {
                    units.stamina = units.unit_type.max_stamina();
                }
                //}
            }
        }
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
    } else {
        game.round += 1;
    }

    game.turn_timestamp = current_timestamp;

    Ok(())
}
