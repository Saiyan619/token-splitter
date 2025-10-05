use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::state::VaultInfo;

pub fn _initialize_vault(ctx: Context<Initialize>) -> Result<()> {
        let clock=Clock::get()?;
        let vault_info=&mut ctx.accounts.vault_info;
        vault_info.amount=0;
        vault_info.owner=ctx.accounts.signer.key();
        vault_info.mint=ctx.accounts.mint.key();
        vault_info.vault_info_bump=ctx.bumps.vault_info;
        vault_info.vault_token_bump=ctx.bumps.vault_token_acc;  
        vault_info.created_at=clock.unix_timestamp;
        Ok(())
    }



#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init,
    payer = signer,
    seeds=[b"vault_info", signer.key().as_ref(), mint.key().as_ref()],
    bump, 
    space = 8 + 32 + 32 + 8 + 8 + 4 + 1 + 1)]
    pub vault_info: Account<'info, VaultInfo>,
    #[account (init,
    payer=signer,
    seeds=[b"token_vault", signer.key().as_ref(), mint.key().as_ref()],
    bump,
    token::mint=mint,
    token::authority=vault_info
    )]
        pub vault_token_acc:Account<'info, TokenAccount>,
        #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
