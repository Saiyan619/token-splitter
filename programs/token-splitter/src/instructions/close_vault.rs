use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, CloseAccount};
use crate::error::CustomError;
use crate::state::VaultInfo;

pub fn _close_vault(ctx: Context<CloseVault>)->Result<()>{
    let vault_info = &mut ctx.accounts.vault_info;
        let vault_token_acc=&mut ctx.accounts.vault_token_acc;
        let signer = &ctx.accounts.signer.key();
        let mint = &ctx.accounts.mint.key();
        require!(vault_info.owner == signer.key(), CustomError::Unauthorized);
        require!(mint.key() == vault_token_acc.mint , CustomError::InvalidMint);
        require!(mint.key() == vault_info.mint.key(), CustomError::InvalidMint);
        require!(vault_token_acc.amount == 0, CustomError::VaultNotEmpty);
         let seeds = &[
            b"vault_info",
            signer.as_ref(),
            mint.as_ref(),
            &[vault_info.vault_info_bump]
        ];
        let signer_seeds=&[&seeds[..]];

        let cpi_accounts = CloseAccount {
            account: ctx.accounts.vault_token_acc.to_account_info(),
            destination: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.vault_info.to_account_info(),
        };

        let cpi_program=ctx.accounts.token_program.to_account_info();
        let cpi_context=CpiContext::new_with_signer(cpi_program,cpi_accounts,signer_seeds);
        token::close_account(cpi_context)?;

    Ok(())
}


#[derive(Accounts)]
pub struct CloseVault <'info>{
    #[account(
        mut,
        close=signer,
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