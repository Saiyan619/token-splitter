use anchor_lang::prelude::*;

#[account]
pub struct VaultInfo {
    pub owner:Pubkey,
    pub amount: u64,
    pub mint:Pubkey,
    pub vault_info_bump:u8,
    pub vault_token_bump:u8,
    pub created_at:i64
}