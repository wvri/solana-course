//! Неделя 13: позиция vault в CLMM-пуле Raydium.
//!
//! open_position  → CPI open_position_with_token22_nft (NFT позиции у vault_authority)
//! collect_fees   → CPI decrease_liquidity_v2 с liquidity = 0 (забирает только комиссии)
//! decrease_liquidity → CPI decrease_liquidity_v2 (вывод ликвидности + комиссии)
//! close_position → CPI close_position (позиция пустая, rent возвращается vault_authority)
//! check_range    → проверка, вышла ли цена из диапазона (для перестановки позиции)

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::memo::Memo;
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Token2022, TokenAccount};
use raydium_clmm_cpi::{
    cpi,
    program::RaydiumClmm,
    states::{PoolState, POSITION_SEED},
};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::*;
use crate::math::tick_in_range;
use crate::state::{Strategy, VaultConfig};

// ═════════════════════════════ open_position ═════════════════════════════

#[derive(Accounts)]
pub struct OpenVaultPosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin,
        has_one = vault_asset
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// Платит rent за NFT, personal position и tick arrays → `mut`.
    #[account(mut, seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump = strategy.bump,
        has_one = pool_state,
        has_one = vault_pair,
        constraint = !strategy.has_position @ VaultError::PositionAlreadyOpen
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    #[account(mut)]
    pub vault_asset: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_pair: Box<InterfaceAccount<'info, TokenAccount>>,

    pub clmm_program: Program<'info, RaydiumClmm>,

    /// Новый keypair, генерируется клиентом и подписывает транзакцию.
    #[account(mut)]
    pub position_nft_mint: Signer<'info>,
    /// CHECK: ATA(vault_authority, position_nft_mint, Token-2022), создаёт Raydium
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    /// CHECK: устаревший аккаунт, оставлен Raydium для совместимости
    pub protocol_position: UncheckedAccount<'info>,
    /// CHECK: PDA tick array, seeds проверяет Raydium
    #[account(mut)]
    pub tick_array_lower: UncheckedAccount<'info>,
    /// CHECK: PDA tick array, seeds проверяет Raydium
    #[account(mut)]
    pub tick_array_upper: UncheckedAccount<'info>,
    /// CHECK: PDA personal position, создаёт Raydium
    #[account(mut)]
    pub personal_position: UncheckedAccount<'info>,
    /// CHECK: token vault пула для token_0, проверяет Raydium
    #[account(mut)]
    pub token_vault_0: UncheckedAccount<'info>,
    /// CHECK: token vault пула для token_1, проверяет Raydium
    #[account(mut)]
    pub token_vault_1: UncheckedAccount<'info>,
    /// CHECK: mint token_0, проверяет Raydium
    pub vault_0_mint: UncheckedAccount<'info>,
    /// CHECK: mint token_1, проверяет Raydium
    pub vault_1_mint: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program_2022: Program<'info, Token2022>,
}

/// remaining_accounts: [tick_array_bitmap_extension] — если диапазон вне основного bitmap.
#[allow(clippy::too_many_arguments)]
pub fn open_position_handler<'info>(
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
    let spacing = ctx.accounts.strategy.tick_spacing as i32;
    require!(tick_lower < tick_upper, VaultError::InvalidTickRange);
    require!(
        tick_lower % spacing == 0 && tick_upper % spacing == 0,
        VaultError::InvalidTickRange
    );

    let accs = &ctx.accounts;
    let asset_before = accs.vault_asset.amount;
    let pair_before = accs.vault_pair.amount;

    // token_0/token_1 пула ↔ token accounts vault
    let (acc_0, acc_1) = if accs.strategy.asset_is_token_0 {
        (&accs.vault_asset, &accs.vault_pair)
    } else {
        (&accs.vault_pair, &accs.vault_asset)
    };

    let config_key = accs.config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[accs.config.authority_bump],
    ];
    let signer: &[&[&[u8]]] = &[authority_seeds];

    let cpi_accounts = cpi::accounts::OpenPositionWithToken22Nft {
        payer: accs.vault_authority.to_account_info(),
        position_nft_owner: accs.vault_authority.to_account_info(),
        position_nft_mint: accs.position_nft_mint.to_account_info(),
        position_nft_account: accs.position_nft_account.to_account_info(),
        pool_state: accs.pool_state.to_account_info(),
        protocol_position: accs.protocol_position.to_account_info(),
        tick_array_lower: accs.tick_array_lower.to_account_info(),
        tick_array_upper: accs.tick_array_upper.to_account_info(),
        personal_position: accs.personal_position.to_account_info(),
        token_account_0: acc_0.to_account_info(),
        token_account_1: acc_1.to_account_info(),
        token_vault_0: accs.token_vault_0.to_account_info(),
        token_vault_1: accs.token_vault_1.to_account_info(),
        rent: accs.rent.to_account_info(),
        system_program: accs.system_program.to_account_info(),
        token_program: accs.token_program.to_account_info(),
        associated_token_program: accs.associated_token_program.to_account_info(),
        token_program_2022: accs.token_program_2022.to_account_info(),
        vault_0_mint: accs.vault_0_mint.to_account_info(),
        vault_1_mint: accs.vault_1_mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(accs.clmm_program.key(), cpi_accounts, signer)
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
    cpi::open_position_with_token22_nft(
        cpi_ctx,
        tick_lower,
        tick_upper,
        tick_array_lower_start_index,
        tick_array_upper_start_index,
        liquidity,
        amount_0_max,
        amount_1_max,
        false, // with_metadata
        base_flag,
    )?;

    ctx.accounts.vault_asset.reload()?;
    ctx.accounts.vault_pair.reload()?;
    let asset_spent = asset_before.saturating_sub(ctx.accounts.vault_asset.amount);
    let pair_spent = pair_before.saturating_sub(ctx.accounts.vault_pair.amount);

    let nft_mint = ctx.accounts.position_nft_mint.key();
    let strategy = &mut ctx.accounts.strategy;
    strategy.has_position = true;
    strategy.position_nft_mint = nft_mint;
    strategy.tick_lower = tick_lower;
    strategy.tick_upper = tick_upper;

    emit!(PositionOpened {
        config: config_key,
        position_nft_mint: nft_mint,
        tick_lower,
        tick_upper,
        asset_spent,
        pair_spent,
    });
    Ok(())
}

// ═════════════════════ decrease_liquidity / collect_fees ═════════════════════

#[derive(Accounts)]
pub struct ManagePosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin,
        has_one = vault_asset
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump = strategy.bump,
        has_one = pool_state,
        has_one = vault_pair,
        constraint = strategy.has_position @ VaultError::NoPosition
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    #[account(mut)]
    pub vault_asset: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_pair: Box<InterfaceAccount<'info, TokenAccount>>,

    pub clmm_program: Program<'info, RaydiumClmm>,

    /// CHECK: ATA с NFT позиции; Raydium проверяет mint и владельца
    pub nft_account: UncheckedAccount<'info>,
    /// CHECK: PDA ["position", nft_mint] программы Raydium — именно позиция vault
    #[account(
        mut,
        seeds = [POSITION_SEED.as_bytes(), strategy.position_nft_mint.as_ref()],
        seeds::program = clmm_program.key(),
        bump
    )]
    pub personal_position: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    /// CHECK: устаревший аккаунт Raydium
    pub protocol_position: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    #[account(mut)]
    pub token_vault_0: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    #[account(mut)]
    pub token_vault_1: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    #[account(mut)]
    pub tick_array_lower: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    #[account(mut)]
    pub tick_array_upper: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    pub vault_0_mint: UncheckedAccount<'info>,
    /// CHECK: проверяет Raydium
    pub vault_1_mint: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub memo_program: Program<'info, Memo>,
}

/// CPI decrease_liquidity_v2. Возвращает (получено asset, получено pair).
fn decrease_cpi<'info>(
    ctx: Context<'info, ManagePosition<'info>>,
    liquidity: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<(u64, u64)> {
    let accs = &ctx.accounts;
    let asset_before = accs.vault_asset.amount;
    let pair_before = accs.vault_pair.amount;

    let (recipient_0, recipient_1) = if accs.strategy.asset_is_token_0 {
        (&accs.vault_asset, &accs.vault_pair)
    } else {
        (&accs.vault_pair, &accs.vault_asset)
    };

    let config_key = accs.config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[accs.config.authority_bump],
    ];
    let signer: &[&[&[u8]]] = &[authority_seeds];

    let cpi_accounts = cpi::accounts::DecreaseLiquidityV2 {
        nft_owner: accs.vault_authority.to_account_info(),
        nft_account: accs.nft_account.to_account_info(),
        personal_position: accs.personal_position.to_account_info(),
        pool_state: accs.pool_state.to_account_info(),
        protocol_position: accs.protocol_position.to_account_info(),
        token_vault_0: accs.token_vault_0.to_account_info(),
        token_vault_1: accs.token_vault_1.to_account_info(),
        tick_array_lower: accs.tick_array_lower.to_account_info(),
        tick_array_upper: accs.tick_array_upper.to_account_info(),
        recipient_token_account_0: recipient_0.to_account_info(),
        recipient_token_account_1: recipient_1.to_account_info(),
        token_program: accs.token_program.to_account_info(),
        token_program_2022: accs.token_program_2022.to_account_info(),
        memo_program: accs.memo_program.to_account_info(),
        vault_0_mint: accs.vault_0_mint.to_account_info(),
        vault_1_mint: accs.vault_1_mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(accs.clmm_program.key(), cpi_accounts, signer)
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
    cpi::decrease_liquidity_v2(cpi_ctx, liquidity, amount_0_min, amount_1_min)?;

    ctx.accounts.vault_asset.reload()?;
    ctx.accounts.vault_pair.reload()?;
    Ok((
        ctx.accounts.vault_asset.amount.saturating_sub(asset_before),
        ctx.accounts.vault_pair.amount.saturating_sub(pair_before),
    ))
}

/// Забирает накопленные комиссии, не трогая ликвидность.
pub fn collect_fees_handler<'info>(ctx: Context<'info, ManagePosition<'info>>) -> Result<()> {
    let config = ctx.accounts.config.key();
    let nft_mint = ctx.accounts.strategy.position_nft_mint;
    let (asset_fees, pair_fees) = decrease_cpi(ctx, 0, 0, 0)?;
    emit!(FeesCollected {
        config,
        position_nft_mint: nft_mint,
        asset_fees,
        pair_fees,
    });
    Ok(())
}

/// Выводит `liquidity` из позиции (вместе с комиссиями) обратно в vault.
/// `amount_0_min` / `amount_1_min` — защита от проскальзывания.
pub fn decrease_liquidity_handler<'info>(
    ctx: Context<'info, ManagePosition<'info>>,
    liquidity: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<()> {
    require!(liquidity > 0, VaultError::ZeroAmount);
    let config = ctx.accounts.config.key();
    let nft_mint = ctx.accounts.strategy.position_nft_mint;
    let (asset_received, pair_received) = decrease_cpi(ctx, liquidity, amount_0_min, amount_1_min)?;
    emit!(LiquidityDecreased {
        config,
        position_nft_mint: nft_mint,
        liquidity,
        asset_received,
        pair_received,
    });
    Ok(())
}

// ═════════════════════════════ close_position ═════════════════════════════

#[derive(Accounts)]
pub struct CloseVaultPosition<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// Получает обратно rent за позицию.
    #[account(mut, seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump = strategy.bump,
        has_one = pool_state,
        has_one = position_nft_mint,
        constraint = strategy.has_position @ VaultError::NoPosition
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    pub clmm_program: Program<'info, RaydiumClmm>,

    /// CHECK: NFT позиции = strategy.position_nft_mint (has_one)
    #[account(mut)]
    pub position_nft_mint: UncheckedAccount<'info>,
    /// CHECK: ATA с NFT, проверяет Raydium
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    /// CHECK: PDA personal position программы Raydium
    #[account(
        mut,
        seeds = [POSITION_SEED.as_bytes(), position_nft_mint.key().as_ref()],
        seeds::program = clmm_program.key(),
        bump
    )]
    pub personal_position: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,

    pub system_program: Program<'info, System>,
    /// NFT позиции выпущен в Token-2022.
    pub token_program_2022: Program<'info, Token2022>,
}

/// Закрывает пустую позицию (liquidity = 0, комиссии собраны): сжигает NFT,
/// закрывает personal position, rent возвращается vault_authority.
pub fn close_position_handler(ctx: Context<CloseVaultPosition>) -> Result<()> {
    let accs = &ctx.accounts;
    let config_key = accs.config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[accs.config.authority_bump],
    ];
    let signer: &[&[&[u8]]] = &[authority_seeds];

    let cpi_accounts = cpi::accounts::ClosePosition {
        nft_owner: accs.vault_authority.to_account_info(),
        position_nft_mint: accs.position_nft_mint.to_account_info(),
        position_nft_account: accs.position_nft_account.to_account_info(),
        personal_position: accs.personal_position.to_account_info(),
        system_program: accs.system_program.to_account_info(),
        token_program: accs.token_program_2022.to_account_info(),
    };
    // pool_state передаётся как remaining account (так делает и Raydium SDK)
    let cpi_ctx = CpiContext::new_with_signer(accs.clmm_program.key(), cpi_accounts, signer)
        .with_remaining_accounts(vec![accs.pool_state.to_account_info()]);
    cpi::close_position(cpi_ctx)?;

    let nft_mint = accs.position_nft_mint.key();
    let strategy = &mut ctx.accounts.strategy;
    strategy.has_position = false;
    strategy.position_nft_mint = Pubkey::default();
    strategy.tick_lower = 0;
    strategy.tick_upper = 0;

    emit!(PositionClosed {
        config: config_key,
        position_nft_mint: nft_mint,
    });
    Ok(())
}

// ═════════════════════════════ check_range ═════════════════════════════

#[derive(Accounts)]
pub struct CheckRange<'info> {
    #[account(seeds = [CONFIG_SEED, config.asset_mint.as_ref()], bump = config.bump)]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump = strategy.bump,
        has_one = pool_state,
        constraint = strategy.has_position @ VaultError::NoPosition
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    pub pool_state: AccountLoader<'info, PoolState>,
}

/// Сравнивает текущий тик пула с диапазоном позиции.
/// С `require_out_of_range = true` — ставится первой инструкцией в транзакцию
/// перестановки: если цена всё ещё в диапазоне, вся транзакция откатится.
pub fn check_range_handler(ctx: Context<CheckRange>, require_out_of_range: bool) -> Result<()> {
    let tick_current = ctx.accounts.pool_state.load()?.tick_current;
    let s = &ctx.accounts.strategy;
    let in_range = tick_in_range(tick_current, s.tick_lower, s.tick_upper);

    emit!(RangeChecked {
        config: ctx.accounts.config.key(),
        tick_current,
        tick_lower: s.tick_lower,
        tick_upper: s.tick_upper,
        in_range,
    });
    if require_out_of_range {
        require!(!in_range, VaultError::PositionInRange);
    }
    Ok(())
}
