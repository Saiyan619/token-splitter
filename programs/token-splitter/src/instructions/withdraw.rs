use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::error::CustomError;
use crate::state::VaultInfo;

pub fn _withdraw(ctx: Context<WithDraw>)->Result<()>{
    let vault_info = &mut ctx.accounts.vault_info;
        let vault_token_acc=&mut ctx.accounts.vault_token_acc;
        let user_token_acc=&mut ctx.accounts.user_token_acc;
        require!(vault_info.owner == ctx.accounts.signer.key(), CustomError::UnauthorizedWithDraw);
        require!(vault_info.mint == ctx.accounts.mint.key(), CustomError::WrongMint);
        require!(vault_token_acc.amount > 0,CustomError::VaultEmpty);
    
        let signer = &ctx.accounts.signer.key();
        let mint = &ctx.accounts.mint.key();
        let seeds = &[
            b"vault_info",
            signer.as_ref(),
            mint.as_ref(),
            &[vault_info.vault_info_bump]
        ];
        let signer_seeds=&[&seeds[..]];

        let cpi_accounts = Transfer{
            from:vault_token_acc.to_account_info(),
            to:user_token_acc.to_account_info(),
            authority:vault_info.to_account_info()
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, vault_token_acc.amount)?;
         vault_info.amount = vault_info.amount.checked_sub(vault_token_acc.amount)
        .ok_or(CustomError::Overflow)?;
    Ok(())
}

#[derive(Accounts)]
pub struct WithDraw <'info>{
    #[account(
        mut,
        seeds=[b"vault_info", signer.key().as_ref(), mint.key().as_ref()],
        bump=vault_info.vault_info_bump
    )]
    pub vault_info:Account<'info, VaultInfo>,
    #[account(
        mut,
        seeds=[b"token_vault", signer.key().as_ref(), mint.key().as_ref()],
          // ADDED: Validate mint matches
        constraint = vault_token_acc.mint == mint.key() @ CustomError::InvalidMint,
        // ADDED: Validate token account is owned by vault_info PDA
        constraint = vault_token_acc.owner == vault_info.key() @ CustomError::InvalidVaultTokenAccount,
        bump=vault_info.vault_token_bump
    )]
    pub vault_token_acc:Account<'info, TokenAccount>,
      #[account(mut)]
    pub signer:Signer<'info>,
    #[account(mut,
       // ADDED: Validate user token account belongs to signer
        constraint = user_token_acc.owner == signer.key() @ CustomError::WrongOwner,
        // ADDED: Validate mint matches
        constraint = user_token_acc.mint == mint.key() @ CustomError::InvalidMint,)]
    pub user_token_acc:Account<'info, TokenAccount>,
    pub mint:Account<'info, Mint>,
    pub token_program:Program<'info, Token>,
    pub system_program:Program<'info, System>
}