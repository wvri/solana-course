use anchor_lang::prelude::*;

use crate::constants::*;
use crate::events::UserRegistered;
use crate::state::{UserAccount, VaultConfig};

#[derive(Accounts)]
pub struct InitUser<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(seeds = [CONFIG_SEED, config.asset_mint.as_ref()], bump = config.bump)]
    pub config: Account<'info, VaultConfig>,

    /// Один PDA на пару (vault, пользователь).
    #[account(
        init,
        payer = owner,
        space = 8 + UserAccount::INIT_SPACE,
        seeds = [USER_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    pub system_program: Program<'info, System>,
}

pub fn init_user_handler(ctx: Context<InitUser>) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.owner = ctx.accounts.owner.key();
    user.vault = ctx.accounts.config.key();
    user.total_deposited = 0;
    user.total_withdrawn = 0;
    user.shares_minted = 0;
    user.shares_burned = 0;
    user.last_action_ts = Clock::get()?.unix_timestamp;
    user.bump = ctx.bumps.user_account;

    emit!(UserRegistered {
        config: user.vault,
        owner: user.owner,
        user_account: user.key(),
    });
    Ok(())
}
