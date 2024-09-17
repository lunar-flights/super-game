use crate::states::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeSuper<'info> {
    #[account(init, payer = payer, space = SuperState::LEN, seeds = [b"SUPER"], bump)]
    pub super_state: Account<'info, SuperState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_super(ctx: Context<InitializeSuper>) -> Result<()> {
    let super_state = &mut ctx.accounts.super_state;

    super_state.game_count = 0;

    Ok(())
}
