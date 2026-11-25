//! Seeds для всех PDA программы.
//!
//! config          = ["config", asset_mint]
//! vault_authority = ["authority", config]      — system-owned PDA, подписывает CPI
//! share_mint      = ["shares", config]         — mint share-токенов
//! vault_asset     = ["vault_asset", config]    — token account с активом
//! user_account    = ["user", config, owner]    — PDA пользователя
//! strategy        = ["strategy", config]       — привязка к пулу Raydium CLMM (нед. 11)
//! vault_pair      = ["vault_pair", config]     — token account второго токена пары

pub const CONFIG_SEED: &[u8] = b"config";
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const SHARES_SEED: &[u8] = b"shares";
pub const VAULT_ASSET_SEED: &[u8] = b"vault_asset";
pub const USER_SEED: &[u8] = b"user";
pub const STRATEGY_SEED: &[u8] = b"strategy";
pub const VAULT_PAIR_SEED: &[u8] = b"vault_pair";
