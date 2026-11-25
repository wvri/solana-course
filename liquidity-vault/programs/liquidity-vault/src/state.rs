use anchor_lang::prelude::*;

/// Аккаунт настроек vault. Один на каждый `asset_mint`.
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    /// Администратор: меняет настройки, управляет стратегией.
    pub admin: Pubkey,
    /// Токен, который принимает vault (депозитный актив).
    pub asset_mint: Pubkey,
    /// Mint share-токенов (доля пользователя в vault).
    pub share_mint: Pubkey,
    /// Token account vault, где лежит актив.
    pub vault_asset: Pubkey,
    /// Учётная стоимость активов vault (в единицах `asset_mint`).
    pub total_assets: u64,
    /// Сколько share-токенов выпущено.
    pub total_shares: u64,
    /// Лимит на `total_assets` (0 = без лимита).
    pub deposit_cap: u64,
    /// Аварийная пауза депозитов.
    pub paused: bool,
    pub bump: u8,
    pub authority_bump: u8,
    pub share_mint_bump: u8,
    pub vault_asset_bump: u8,
}

/// PDA пользователя: статистика его операций в конкретном vault.
/// Сами доли — это share-токены на кошельке пользователя.
#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    pub owner: Pubkey,
    pub vault: Pubkey,
    /// Всего внесено актива.
    pub total_deposited: u64,
    /// Всего выведено актива.
    pub total_withdrawn: u64,
    /// Всего получено share-токенов.
    pub shares_minted: u64,
    /// Всего сожжено share-токенов.
    pub shares_burned: u64,
    pub last_action_ts: i64,
    pub bump: u8,
}

/// Стратегия vault: к какому CLMM-пулу Raydium привязан vault (нед. 11).
#[account]
#[derive(InitSpace)]
pub struct Strategy {
    pub config: Pubkey,
    /// Аккаунт пула Raydium CLMM (PoolState).
    pub pool_state: Pubkey,
    /// Второй токен пары (первый — `config.asset_mint`).
    pub pair_mint: Pubkey,
    /// Token account vault для второго токена.
    pub vault_pair: Pubkey,
    /// Порядок токенов в пуле: Raydium сортирует mint-ы, token_0 < token_1.
    pub asset_is_token_0: bool,
    pub tick_spacing: u16,
    pub bump: u8,
    pub vault_pair_bump: u8,
    // ── нед. 13: активная позиция в CLMM ──
    /// Есть ли открытая позиция.
    pub has_position: bool,
    /// NFT позиции Raydium (Token-2022), владелец — vault_authority.
    pub position_nft_mint: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
}
