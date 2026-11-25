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

    // ───────────── Неделя 13: позиция в CLMM ─────────────

    /// Открывает позицию vault в диапазоне [tick_lower, tick_upper).
    #[allow(clippy::too_many_arguments)]
    pub fn open_position<'info>(
        ctx: Context<'info, OpenVaultPosition<'info>>,
        tick_lower: i32,
        tick_upper: i32,
        tick_array_lower_start_index: i32,
        tick_array_upper_start_index: i32,
        liquidity: u128,
        amount_0_max: u64,
        amount_1_max: u64,
        base_flag: Option<bool>,
    ) -> Result<()> {
        instructions::position::open_position_handler(
            ctx,
            tick_lower,
            tick_upper,
            tick_array_lower_start_index,
            tick_array_upper_start_index,
            liquidity,
            amount_0_max,
            amount_1_max,
            base_flag,
        )
    }

    /// Собирает комиссии позиции в vault.
    pub fn collect_fees<'info>(ctx: Context<'info, ManagePosition<'info>>) -> Result<()> {
        instructions::position::collect_fees_handler(ctx)
    }

    /// Выводит ликвидность из позиции обратно в vault.
    pub fn decrease_liquidity<'info>(
        ctx: Context<'info, ManagePosition<'info>>,
        liquidity: u128,
        amount_0_min: u64,
        amount_1_min: u64,
    ) -> Result<()> {
        instructions::position::decrease_liquidity_handler(
            ctx,
            liquidity,
            amount_0_min,
            amount_1_min,
        )
    }

    /// Закрывает пустую позицию.
    pub fn close_position(ctx: Context<CloseVaultPosition>) -> Result<()> {
        instructions::position::close_position_handler(ctx)
    }

    /// Проверяет, вышла ли цена из диапазона позиции.
    pub fn check_range(ctx: Context<CheckRange>, require_out_of_range: bool) -> Result<()> {
        instructions::position::check_range_handler(ctx, require_out_of_range)
    }

    /// Переоценка total_assets (не более ±20% за вызов).
    pub fn report_assets(ctx: Context<AdminOnly>, new_total_assets: u64) -> Result<()> {
        instructions::admin::report_assets_handler(ctx, new_total_assets)
    }
}
