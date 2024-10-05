use crate::errors::GameError;
use crate::states::{Building, BuildingType, Game, UnitType, Units};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct RecruitUnits<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    pub player: Signer<'info>,
}

pub fn recruit_units(
    ctx: Context<RecruitUnits>,
    unit_type: UnitType,
    quantity: u16,
    row: u8,
    col: u8,
) -> Result<()> {
    let game = &mut ctx.accounts.game;

    let row_index = row as usize;
    let col_index = col as usize;

    if row_index >= game.tiles.len() || col_index >= game.tiles[row_index].len() {
        return err!(GameError::InvalidCoordinates);
    }

    let tile = game.tiles[row_index][col_index]
        .as_ref()
        .ok_or(GameError::InvalidTile)?;

    let player_pubkey = ctx.accounts.player.key();
    if tile.owner != player_pubkey {
        return err!(GameError::TileNotOwned);
    }

    if let Some(units) = &tile.units {
        if units.unit_type != unit_type {
            return err!(GameError::DifferentUnitTypeOnTile);
        }
    }

    // Check building requirements for the unit type
    match unit_type {
        UnitType::Infantry => {
            // No specific building required
        }
        UnitType::Tank => {
            if !matches!(
                tile.building,
                Some(Building {
                    building_type: BuildingType::TankFactory,
                    ..
                })
            ) {
                return err!(GameError::RequiresTankFactory);
            }
        }
        UnitType::Plane => {
            if !matches!(
                tile.building,
                Some(Building {
                    building_type: BuildingType::PlaneFactory,
                    ..
                })
            ) {
                return err!(GameError::RequiresPlaneFactory);
            }
        }
        UnitType::Mutants => {
            return err!(GameError::InvalidUnitType);
        }
    }

    let player_info = game
        .players
        .iter_mut()
        .find(|p| {
            p.as_ref()
                .map_or(false, |info| info.pubkey == player_pubkey)
        })
        .and_then(|p| p.as_mut())
        .ok_or(GameError::InvalidPlayer)?;

    let unit_cost = unit_type.cost() as u32;
    let total_cost = unit_cost
        .checked_mul(quantity as u32)
        .ok_or(GameError::TooManyUnits)?;

    if player_info.balance < total_cost {
        return err!(GameError::InsufficientFunds);
    }

    player_info.balance = player_info.balance.saturating_sub(total_cost);

    let tile = game.tiles[row_index][col_index]
        .as_mut()
        .ok_or(GameError::InvalidTile)?;

    if let Some(units) = &mut tile.units {
        units.quantity = units
            .quantity
            .checked_add(quantity)
            .ok_or(GameError::TooManyUnits)?;
    } else {
        tile.units = Some(Units {
            unit_type,
            quantity,
            stamina: unit_type.max_stamina(),
        });
    }

    Ok(())
}
