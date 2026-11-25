use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::Withdrawn;
use crate::math::assets_for_shares;
use crate::state::{UserAccount, VaultConfig};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = asset_mint,
        has_one = share_mint,
        has_one = vault_asset
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    /// PDA-владелец vault_asset: подписывает перевод из vault.
    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub asset_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub share_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub vault_asset: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Куда вернуть актив.
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_asset_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Откуда сжигаются share-токены.
    #[account(
        mut,
        token::mint = share_mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_share_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [USER_SEED, config.key().as_ref(), user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key() @ VaultError::Unauthorized
    )]
    pub user_account: Box<Account<'info, UserAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

/// Вывод не блокируется паузой: пользователь всегда может забрать свои средства.
pub fn withdraw_handler(ctx: Context<Withdraw>, shares: u64, min_assets_out: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(shares > 0, VaultError::ZeroAmount);
    require!(
        ctx.accounts.user_share_account.amount >= shares,
        VaultError::InsufficientShares
    );

    let assets = assets_for_shares(shares, config.total_assets, config.total_shares)
        .ok_or(VaultError::MathOverflow)?;
    require!(assets > 0, VaultError::ZeroAmount);
    require!(assets >= min_assets_out, VaultError::SlippageExceeded);
    // Часть средств может быть размещена в пуле (нед. 13) — выдаём только свободный остаток.
    require!(
        ctx.accounts.vault_asset.amount >= assets,
        VaultError::InsufficientIdleLiquidity
    );

    // 1. Сжигаем доли пользователя (подписывает сам пользователь).
    burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        shares,
    )?;

    // 2. CPI в Token program: vault → пользователь.
    //    Владелец vault_asset — PDA, поэтому подпись через seeds (invoke_signed).
    let config_key = config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[config.authority_bump],
    ];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.vault_asset.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                to: ctx.accounts.user_asset_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        assets,
        ctx.accounts.asset_mint.decimals,
    )?;

    // 3. Учёт.
    let config = &mut ctx.accounts.config;
    config.total_assets = config
        .total_assets
        .checked_sub(assets)
        .ok_or(VaultError::MathOverflow)?;
    config.total_shares = config
        .total_shares
        .checked_sub(shares)
        .ok_or(VaultError::MathOverflow)?;

    let user = &mut ctx.accounts.user_account;
    user.total_withdrawn = user
        .total_withdrawn
        .checked_add(assets)
        .ok_or(VaultError::MathOverflow)?;
    user.shares_burned = user
        .shares_burned
        .checked_add(shares)
        .ok_or(VaultError::MathOverflow)?;
    user.last_action_ts = Clock::get()?.unix_timestamp;

    emit!(Withdrawn {
        config: config_key,
        user: user.owner,
        shares,
        amount: assets,
        total_assets: config.total_assets,
        total_shares: config.total_shares,
    });
    Ok(())
}
