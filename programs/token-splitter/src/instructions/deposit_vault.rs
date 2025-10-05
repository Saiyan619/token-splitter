use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::error::CustomError;
use crate::state::VaultInfo;

 pub fn _deposit_vault(ctx: Context<Deposit>, amount:u64)->Result<()>{
        let vault_info = &mut ctx.accounts.vault_info;
        let vault_token_acc=&mut ctx.accounts.vault_token_acc;
        let user_token_acc=&mut ctx.accounts.user_token_acc;
        require!(amount > 0, CustomError::ZeroAmount);
        require!(vault_info.owner == ctx.accounts.signer.key(), CustomError::UnauthorizedDeposit);
        require!(vault_info.mint == ctx.accounts.mint.key(), CustomError::WrongMint);
        let signer = &ctx.accounts.signer;
        // let mint = &ctx.accounts.mint.key();

        let cpi_accounts=Transfer{
            from:user_token_acc.to_account_info(),
            to:vault_token_acc.to_account_info(),
            authority:signer.to_account_info()
        };
        let cpi_program=ctx.accounts.token_program.to_account_info();
        let cpi_ctx=CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
        vault_info.amount = vault_info.amount.checked_add(amount).ok_or(CustomError::Overflow)?;
        Ok(())
    }

#[derive(Accounts)]
pub struct Deposit<'info>{
    #[account(
        mut,
        seeds=[b"vault_info", signer.key().as_ref(), mint.key().as_ref()],
        bump=vault_info.vault_info_bump
    )]
    pub vault_info:Account<'info, VaultInfo>,
    #[account(
        mut,
        seeds=[b"token_vault", signer.key().as_ref(), mint.key().as_ref()],
        bump=vault_info.vault_token_bump
    )]
    pub vault_token_acc:Account<'info, TokenAccount>,
    #[account(mut,
    constraint = user_token_acc.mint == mint.key() @ CustomError::InvalidMint
)]
user_token_acc: Account<'info, TokenAccount>,
    #[account(mut)]
    pub signer:Signer<'info>,
    pub mint:Account<'info, Mint>,
    pub token_program:Program<'info, Token>,
    pub system_program:Program<'info, System>
}
