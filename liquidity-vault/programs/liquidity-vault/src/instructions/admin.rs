use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::{AssetsReported, VaultSettingsUpdated};
use crate::math::within_bps;
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

/// Переоценка стоимости vault (нед. 13). Часть активов лежит в позиции CLMM
/// и во втором токене пары, поэтому admin пересчитывает NAV off-chain
/// (scripts/09-report-assets.ts) и записывает его. Защита: не более ±20% за раз.
pub fn report_assets_handler(ctx: Context<AdminOnly>, new_total_assets: u64) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let old = config.total_assets;
    if config.total_shares > 0 {
        require!(
            within_bps(old, new_total_assets, MAX_REPORT_CHANGE_BPS),
            VaultError::ReportOutOfBounds
        );
    }
    config.total_assets = new_total_assets;

    emit!(AssetsReported {
        config: config.key(),
        old_total_assets: old,
        new_total_assets,
    });
    Ok(())
}
