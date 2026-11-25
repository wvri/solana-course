use anchor_lang::prelude::*;
use anchor_spl::memo::Memo;
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, Token2022, TokenAccount};
use raydium_clmm_cpi::{cpi, program::RaydiumClmm, states::PoolState};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::Swapped;
use crate::state::{Strategy, VaultConfig};

/// Обмен токенов vault через CLMM-пул Raydium (CPI `swap_v2`).
///
/// remaining_accounts (в этом порядке, все writable):
///   [tick_array_bitmap_extension — если нужен], tick_array_0, tick_array_1, ...
/// Клиент получает их из симуляции swap в Raydium SDK.
#[derive(Accounts)]
pub struct VaultSwap<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin,
        has_one = asset_mint,
        has_one = vault_asset
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump = strategy.bump,
        has_one = pool_state,
        has_one = pair_mint,
        has_one = vault_pair
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    #[account(mut)]
    pub vault_asset: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_pair: Box<InterfaceAccount<'info, TokenAccount>>,
    pub asset_mint: Box<InterfaceAccount<'info, Mint>>,
    pub pair_mint: Box<InterfaceAccount<'info, Mint>>,

    pub clmm_program: Program<'info, RaydiumClmm>,

    /// CHECK: проверяется Raydium (address = pool_state.amm_config)
    pub amm_config: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    /// CHECK: token vault пула для входного токена, проверяется Raydium
    #[account(mut)]
    pub input_pool_vault: UncheckedAccount<'info>,
    /// CHECK: token vault пула для выходного токена, проверяется Raydium
    #[account(mut)]
    pub output_pool_vault: UncheckedAccount<'info>,
    /// CHECK: observation пула, проверяется Raydium (address = pool_state.observation_key)
    #[account(mut)]
    pub observation_state: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub memo_program: Program<'info, Memo>,
}

/// * `amount_in`            — сколько отдать (exact input)
/// * `min_amount_out`       — минимум, который vault согласен получить (slippage)
/// * `sqrt_price_limit_x64` — предельная цена (0 = без ограничения)
/// * `asset_to_pair`        — направление: true = asset → pair, false = pair → asset
pub fn swap_handler<'info>(
    ctx: Context<'info, VaultSwap<'info>>,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit_x64: u128,
    asset_to_pair: bool,
) -> Result<()> {
    require!(amount_in > 0, VaultError::ZeroAmount);
    // Обмен без защиты от проскальзывания запрещён.
    require!(min_amount_out > 0, VaultError::SlippageNotSet);

    let accs = &ctx.accounts;
    let (input_acc, output_acc, input_mint, output_mint) = if asset_to_pair {
        (
            &accs.vault_asset,
            &accs.vault_pair,
            &accs.asset_mint,
            &accs.pair_mint,
        )
    } else {
        (
            &accs.vault_pair,
            &accs.vault_asset,
            &accs.pair_mint,
            &accs.asset_mint,
        )
    };
    require!(
        input_acc.amount >= amount_in,
        VaultError::InsufficientIdleLiquidity
    );
    let input_before = input_acc.amount;
    let output_before = output_acc.amount;

    let config_key = accs.config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[accs.config.authority_bump],
    ];
    let signer: &[&[&[u8]]] = &[authority_seeds];

    let cpi_accounts = cpi::accounts::SwapSingleV2 {
        payer: accs.vault_authority.to_account_info(),
        amm_config: accs.amm_config.to_account_info(),
        pool_state: accs.pool_state.to_account_info(),
        input_token_account: input_acc.to_account_info(),
        output_token_account: output_acc.to_account_info(),
        input_vault: accs.input_pool_vault.to_account_info(),
        output_vault: accs.output_pool_vault.to_account_info(),
        observation_state: accs.observation_state.to_account_info(),
        token_program: accs.token_program.to_account_info(),
        token_program_2022: accs.token_program_2022.to_account_info(),
        memo_program: accs.memo_program.to_account_info(),
        input_vault_mint: input_mint.to_account_info(),
        output_vault_mint: output_mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(accs.clmm_program.key(), cpi_accounts, signer)
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());

    // other_amount_threshold = min_amount_out: Raydium сам откатит swap,
    // если выход меньше порога (is_base_input = true).
    cpi::swap_v2(
        cpi_ctx,
        amount_in,
        min_amount_out,
        sqrt_price_limit_x64,
        true,
    )?;

    // Повторная проверка на своей стороне по фактическим балансам.
    ctx.accounts.vault_asset.reload()?;
    ctx.accounts.vault_pair.reload()?;
    let (input_after, output_after) = if asset_to_pair {
        (
            ctx.accounts.vault_asset.amount,
            ctx.accounts.vault_pair.amount,
        )
    } else {
        (
            ctx.accounts.vault_pair.amount,
            ctx.accounts.vault_asset.amount,
        )
    };
    let spent = input_before
        .checked_sub(input_after)
        .ok_or(VaultError::MathOverflow)?;
    let received = output_after
        .checked_sub(output_before)
        .ok_or(VaultError::MathOverflow)?;
    require!(spent <= amount_in, VaultError::SlippageExceeded);
    require!(received >= min_amount_out, VaultError::SlippageExceeded);

    emit!(Swapped {
        config: config_key,
        pool_state: ctx.accounts.pool_state.key(),
        asset_to_pair,
        amount_in: spent,
        amount_out: received,
        min_amount_out,
    });
    Ok(())
}
