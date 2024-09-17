use crate::states::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreatePlayerProfile<'info> {
    #[account(init, payer = player, space = PlayerProfile::LEN, seeds = [b"PROFILE", player.key().as_ref()], bump)]
    pub profile: Account<'info, PlayerProfile>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_player_profile(ctx: Context<CreatePlayerProfile>) -> Result<()> {
    let profile = &mut ctx.accounts.profile;

    profile.player = ctx.accounts.player.key();
    profile.experience = 0;
    profile.completed_games = 0;
    profile.active_games = Vec::new();

    Ok(())
}
