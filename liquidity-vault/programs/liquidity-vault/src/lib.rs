//! Liquidity Vault — сквозной проект курса (недели 8–13).
//!
//! Пользователи вносят токены в vault и получают share-токены (свою долю).
//! Администратор размещает средства vault в CLMM-пуле Raydium (devnet program
//! `DRayAUgENGQBKVaX8owNhgzkEDyoHTGVEGHVJT1E9pfH`).

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

pub use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod liquidity_vault {
    use super::*;

    // ───────────── Неделя 8: каркас ─────────────

    /// Создаёт VaultConfig, vault_authority PDA, share mint и token account vault.
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        deposit_cap: u64,
        authority_lamports: u64,
    ) -> Result<()> {
        instructions::initialize_vault::initialize_vault_handler(
            ctx,
            deposit_cap,
            authority_lamports,
        )
    }

    /// Создаёт PDA пользователя для данного vault.
    pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
        instructions::init_user::init_user_handler(ctx)
    }

    /// Пауза и лимит депозитов (только admin).
    pub fn update_settings(ctx: Context<AdminOnly>, paused: bool, deposit_cap: u64) -> Result<()> {
        instructions::admin::update_settings_handler(ctx, paused, deposit_cap)
    }

    // ───────────── Неделя 9: депозит ─────────────

    /// Вносит `amount` актива и выпускает share-токены пользователю.
    pub fn deposit(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<()> {
        instructions::deposit::deposit_handler(ctx, amount, min_shares_out)
    }

    // ───────────── Неделя 10: вывод ─────────────

    /// Сжигает `shares` и возвращает пользователю его долю актива (CPI с подписью PDA).
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64, min_assets_out: u64) -> Result<()> {
        instructions::withdraw::withdraw_handler(ctx, shares, min_assets_out)
    }

    // ───────────── Неделя 11: Raydium CLMM ─────────────

    /// Привязывает vault к CLMM-пулу Raydium и создаёт vault_pair.
    pub fn set_strategy(ctx: Context<SetStrategy>) -> Result<()> {
        instructions::set_strategy::set_strategy_handler(ctx)
    }

    // ───────────── Неделя 12: swap через Raydium ─────────────

    /// Обменивает токены vault в CLMM-пуле Raydium с ограничением `min_amount_out`.
    pub fn swap<'info>(
        ctx: Context<'info, VaultSwap<'info>>,
        amount_in: u64,
        min_amount_out: u64,
        sqrt_price_limit_x64: u128,
        asset_to_pair: bool,
    ) -> Result<()> {
        instructions::swap::swap_handler(
            ctx,
            amount_in,
            min_amount_out,
            sqrt_price_limit_x64,
            asset_to_pair,
        )
    }
}
