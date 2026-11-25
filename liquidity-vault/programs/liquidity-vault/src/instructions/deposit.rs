use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::Deposited;
use crate::math::shares_for_deposit;
use crate::state::{UserAccount, VaultConfig};

#[derive(Accounts)]
pub struct Deposit<'info> {
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

    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub asset_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub share_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub vault_asset: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Откуда пользователь вносит актив.
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_asset_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Куда придут share-токены (ATA создаётся при первом депозите).
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = share_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
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
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/// `min_shares_out` — защита пользователя: если цена доли изменилась
/// между расчётом на клиенте и исполнением, транзакция откатится.
pub fn deposit_handler(ctx: Context<Deposit>, amount: u64, min_shares_out: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.paused, VaultError::VaultPaused);
    require!(amount > 0, VaultError::ZeroAmount);

    let new_total_assets = config
        .total_assets
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    require!(
        config.deposit_cap == 0 || new_total_assets <= config.deposit_cap,
        VaultError::DepositCapExceeded
    );

    let shares = shares_for_deposit(amount, config.total_assets, config.total_shares)
        .ok_or(VaultError::MathOverflow)?;
    require!(shares > 0, VaultError::ZeroShares);
    require!(shares >= min_shares_out, VaultError::SlippageExceeded);

    // 1. Актив: пользователь → vault (подписывает пользователь).
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.user_asset_account.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info(),
                to: ctx.accounts.vault_asset.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.asset_mint.decimals,
    )?;

    // 2. Share-токены: vault → пользователь (подписывает PDA vault_authority).
    let config_key = config.key();
    let authority_seeds: &[&[u8]] = &[
        AUTHORITY_SEED,
        config_key.as_ref(),
        &[config.authority_bump],
    ];
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        shares,
    )?;

    // 3. Учёт.
    let config = &mut ctx.accounts.config;
    config.total_assets = new_total_assets;
    config.total_shares = config
        .total_shares
        .checked_add(shares)
        .ok_or(VaultError::MathOverflow)?;

    let user = &mut ctx.accounts.user_account;
    user.total_deposited = user
        .total_deposited
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    user.shares_minted = user
        .shares_minted
        .checked_add(shares)
        .ok_or(VaultError::MathOverflow)?;
    user.last_action_ts = Clock::get()?.unix_timestamp;

    emit!(Deposited {
        config: config_key,
        user: user.owner,
        amount,
        shares,
        total_assets: config.total_assets,
        total_shares: config.total_shares,
    });
    Ok(())
}
