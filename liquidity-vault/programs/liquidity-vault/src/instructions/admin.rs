use anchor_lang::prelude::*;

use crate::constants::*;
use crate::events::VaultSettingsUpdated;
use crate::state::VaultConfig;

/// Общий контекст для админских инструкций: подпись `admin` + `has_one`.
#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin
    )]
    pub config: Account<'info, VaultConfig>,
}

pub fn update_settings_handler(
    ctx: Context<AdminOnly>,
    paused: bool,
    deposit_cap: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.paused = paused;
    config.deposit_cap = deposit_cap;

    emit!(VaultSettingsUpdated {
        config: config.key(),
        paused,
        deposit_cap,
    });
    Ok(())
}
