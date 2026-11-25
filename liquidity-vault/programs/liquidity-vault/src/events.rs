use anchor_lang::prelude::*;

#[event]
pub struct VaultInitialized {
    pub config: Pubkey,
    pub admin: Pubkey,
    pub asset_mint: Pubkey,
    pub share_mint: Pubkey,
    pub deposit_cap: u64,
}

#[event]
pub struct UserRegistered {
    pub config: Pubkey,
    pub owner: Pubkey,
    pub user_account: Pubkey,
}

#[event]
pub struct VaultSettingsUpdated {
    pub config: Pubkey,
    pub paused: bool,
    pub deposit_cap: u64,
}

#[event]
pub struct Deposited {
    pub config: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub shares: u64,
    pub total_assets: u64,
    pub total_shares: u64,
}

#[event]
pub struct Withdrawn {
    pub config: Pubkey,
    pub user: Pubkey,
    pub shares: u64,
    pub amount: u64,
    pub total_assets: u64,
    pub total_shares: u64,
}

#[event]
pub struct StrategySet {
    pub config: Pubkey,
    pub pool_state: Pubkey,
    pub pair_mint: Pubkey,
    pub asset_is_token_0: bool,
    pub tick_spacing: u16,
}

#[event]
pub struct Swapped {
    pub config: Pubkey,
    pub pool_state: Pubkey,
    pub asset_to_pair: bool,
    pub amount_in: u64,
    pub amount_out: u64,
    pub min_amount_out: u64,
}
