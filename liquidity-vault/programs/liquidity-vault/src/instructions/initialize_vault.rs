use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::VaultInitialized;
use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Депозитный актив vault.
    #[account(mint::token_program = token_program)]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [CONFIG_SEED, asset_mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, VaultConfig>,

    /// System-owned PDA без данных. Владеет share mint, token accounts
    /// и (позже) позицией в Raydium; подписывает CPI через seeds.
    #[account(
        mut,
        seeds = [AUTHORITY_SEED, config.key().as_ref()],
        bump
    )]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [SHARES_SEED, config.key().as_ref()],
        bump,
        mint::decimals = asset_mint.decimals,
        mint::authority = vault_authority,
        mint::token_program = token_program
    )]
    pub share_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [VAULT_ASSET_SEED, config.key().as_ref()],
        bump,
        token::mint = asset_mint,
        token::authority = vault_authority,
        token::token_program = token_program
    )]
    pub vault_asset: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_vault_handler(
    ctx: Context<InitializeVault>,
    deposit_cap: u64,
    authority_lamports: u64,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.asset_mint = ctx.accounts.asset_mint.key();
    config.share_mint = ctx.accounts.share_mint.key();
    config.vault_asset = ctx.accounts.vault_asset.key();
    config.total_assets = 0;
    config.total_shares = 0;
    config.deposit_cap = deposit_cap;
    config.paused = false;
    config.bump = ctx.bumps.config;
    config.authority_bump = ctx.bumps.vault_authority;
    config.share_mint_bump = ctx.bumps.share_mint;
    config.vault_asset_bump = ctx.bumps.vault_asset;

    // Пополняем vault_authority SOL-ом: позже он оплачивает rent
    // за позицию в Raydium CLMM (personal position + NFT).
    if authority_lamports > 0 {
        let min = Rent::get()?.minimum_balance(0);
        require!(
            authority_lamports >= min,
            VaultError::AuthorityNotRentExempt
        );
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                Transfer {
                    from: ctx.accounts.admin.to_account_info(),
                    to: ctx.accounts.vault_authority.to_account_info(),
                },
            ),
            authority_lamports,
        )?;
    }

    emit!(VaultInitialized {
        config: config.key(),
        admin: config.admin,
        asset_mint: config.asset_mint,
        share_mint: config.share_mint,
        deposit_cap,
    });
    Ok(())
}
