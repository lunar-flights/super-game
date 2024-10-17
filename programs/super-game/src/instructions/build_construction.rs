use crate::errors::{ConstructionError, GameError};
use crate::states::{Building, BuildingType, Game};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct BuildConstruction<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player: Signer<'info>,
}

pub fn build_construction(
    ctx: Context<BuildConstruction>,
    row: usize,
    col: usize,
    building_type: BuildingType,
) -> Result<()> {
    let game = &mut ctx.accounts.game;
    let player_pubkey = ctx.accounts.player.key();

    if row >= game.tiles.len() || col >= game.tiles[0].len() {
        return err!(GameError::OutOfBounds);
    }

    let player_info_index = game
        .players
        .iter()
        .position(|player_option| {
            player_option
                .as_ref()
                .map_or(false, |player_info| player_info.pubkey == player_pubkey)
        })
        .ok_or(GameError::InvalidPlayer)?;

    let player_info = game.players[player_info_index]
        .as_ref()
        .ok_or(GameError::InvalidPlayer)?;

    let mut player_balance = player_info.balance;

    let tile = game.tiles[row][col]
        .as_mut()
        .ok_or(GameError::InvalidTile)?;

    if tile.owner != player_pubkey {
        return err!(ConstructionError::NotYourTile);
    }

    let cost;
    if let Some(existing_building) = &mut tile.building {
        if existing_building.building_type != building_type {
            return err!(ConstructionError::BuildingTypeMismatch);
        }

        if existing_building.level >= existing_building.max_level() {
            return err!(ConstructionError::MaxLevelReached);
        }

        cost = existing_building.get_upgrade_cost() as u32;

        if player_balance < cost {
            return err!(ConstructionError::NotEnoughFunds);
        }

        player_balance -= cost;
        existing_building.level += 1;
    } else {
        if building_type == BuildingType::Base {
            return err!(ConstructionError::CannotBuildBase);
        }

        cost = building_type.get_construction_cost() as u32;

        if player_balance < cost {
            return err!(ConstructionError::NotEnoughFunds);
        }

        player_balance -= cost;

        let new_building = Building {
            building_type,
            level: 1,
        };

        tile.building = Some(new_building);
    }

    {
        let player_info = game.players[player_info_index]
            .as_mut()
            .ok_or(GameError::InvalidPlayer)?;

        player_info.balance = player_balance;
    }

    Ok(())
}
