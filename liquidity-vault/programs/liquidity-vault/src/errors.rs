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
    #[msg("Pool does not trade the vault asset against the given pair mint")]
    InvalidPool,
    #[msg("min_amount_out must be set (swap without slippage protection is not allowed)")]
    SlippageNotSet,
    #[msg("Vault already has an open position")]
    PositionAlreadyOpen,
    #[msg("Vault has no open position")]
    NoPosition,
    #[msg("Invalid tick range")]
    InvalidTickRange,
    #[msg("Price is still inside the position range, rebalance is not needed")]
    PositionInRange,
    #[msg("Reported total assets changed too much in one step")]
    ReportOutOfBounds,
}
