use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("No target addresses provided")]
    NoTargets,
    #[msg("Vault is empty")]
    VaultEmpty,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Maximum 20 targets allowed")]
    MaxLimitError,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Wrong vault owner")]
    WrongVaultOwner,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Account not initialized")]
    AccountNotInitialized,
    #[msg("Account is frozen")]
    AccountFrozen,
    #[msg("Wrong owner")]
    WrongOwner,
    #[msg("Invalid vault token account")]
    InvalidVaultTokenAccount,
    #[msg("Signer is unauthorized to deposit in vault")]
    UnauthorizedDeposit,
    #[msg("Signer is unauthorized to withdraw in vault")]
    UnauthorizedWithDraw,
    #[msg("Wrong target mint")]
    WrongMint,
    #[msg("Mathematical Overflow")]
    Overflow,
    #[msg("you cant deposit 0")]
    ZeroAmount,
    #[msg("Vault is not empty")]
    VaultNotEmpty
}
