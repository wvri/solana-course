use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Vault is paused")]
    VaultPaused,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Not enough lamports to make the vault authority rent-exempt")]
    AuthorityNotRentExempt,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Deposit would exceed the vault cap")]
    DepositCapExceeded,
    #[msg("Deposit is too small to mint at least one share")]
    ZeroShares,
    #[msg("Slippage limit exceeded")]
    SlippageExceeded,
    #[msg("Signer is not allowed to use this account")]
    Unauthorized,
    #[msg("Not enough share tokens")]
    InsufficientShares,
    #[msg("Not enough idle liquidity in the vault, assets are deployed to the pool")]
    InsufficientIdleLiquidity,
}
