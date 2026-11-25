use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use raydium_clmm_cpi::{program::RaydiumClmm, states::PoolState};

use crate::constants::*;
use crate::errors::VaultError;
use crate::events::StrategySet;
use crate::state::{Strategy, VaultConfig};

/// Привязывает vault к CLMM-пулу Raydium (asset_mint / pair_mint)
/// и создаёт token account vault для второго токена пары.
#[derive(Accounts)]
pub struct SetStrategy<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED, config.asset_mint.as_ref()],
        bump = config.bump,
        has_one = admin
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [AUTHORITY_SEED, config.key().as_ref()], bump = config.authority_bump)]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Strategy::INIT_SPACE,
        seeds = [STRATEGY_SEED, config.key().as_ref()],
        bump
    )]
    pub strategy: Box<Account<'info, Strategy>>,

    /// PoolState проверяется по owner = Raydium CLMM program.
    pub pool_state: AccountLoader<'info, PoolState>,

    #[account(
        mint::token_program = token_program,
        constraint = pair_mint.key() != config.asset_mint @ VaultError::InvalidPool
    )]
    pub pair_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = admin,
        seeds = [VAULT_PAIR_SEED, config.key().as_ref()],
        bump,
        token::mint = pair_mint,
        token::authority = vault_authority,
        token::token_program = token_program
    )]
    pub vault_pair: Box<InterfaceAccount<'info, TokenAccount>>,

    pub clmm_program: Program<'info, RaydiumClmm>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn set_strategy_handler(ctx: Context<SetStrategy>) -> Result<()> {
    // PoolState — packed-структура: копируем поля в локальные переменные.
    let pool = ctx.accounts.pool_state.load()?;
    let mint_0 = pool.token_mint_0;
    let mint_1 = pool.token_mint_1;
    let tick_spacing = pool.tick_spacing;
    drop(pool);

    let asset = ctx.accounts.config.asset_mint;
    let pair = ctx.accounts.pair_mint.key();
    let asset_is_token_0 = if mint_0 == asset && mint_1 == pair {
        true
    } else if mint_0 == pair && mint_1 == asset {
        false
    } else {
        return err!(VaultError::InvalidPool);
    };

    let strategy = &mut ctx.accounts.strategy;
    strategy.config = ctx.accounts.config.key();
    strategy.pool_state = ctx.accounts.pool_state.key();
    strategy.pair_mint = pair;
    strategy.vault_pair = ctx.accounts.vault_pair.key();
    strategy.asset_is_token_0 = asset_is_token_0;
    strategy.tick_spacing = tick_spacing;
    strategy.bump = ctx.bumps.strategy;
    strategy.vault_pair_bump = ctx.bumps.vault_pair;
    strategy.has_position = false;
    strategy.position_nft_mint = Pubkey::default();
    strategy.tick_lower = 0;
    strategy.tick_upper = 0;

    emit!(StrategySet {
        config: strategy.config,
        pool_state: strategy.pool_state,
        pair_mint: pair,
        asset_is_token_0,
        tick_spacing,
    });
    Ok(())
}
