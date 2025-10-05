use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::error::CustomError;

use crate::state::VaultInfo;


pub fn _share_funds<'info>(ctx: Context<'_, '_,'info,'info, ShareFunds<'info>>) -> Result<()> {
       let vault_info = &mut ctx.accounts.vault_info;
    let vault_token_account = &ctx.accounts.vault_token_acc;
    let targets = ctx.remaining_accounts;
    
    // Validations
    require!(!targets.is_empty(), CustomError::NoTargets);
    require!(targets.len() <= 20, CustomError::MaxLimitError);
    require!(vault_token_account.amount > 0, CustomError::VaultEmpty);
    require!(vault_info.owner == ctx.accounts.signer.key(), CustomError::Unauthorized);
    
    let num_targets = targets.len() as u64;
    let split_amount = vault_token_account.amount / num_targets;
    let remainder = vault_token_account.amount % num_targets;
    
    // PDA seeds for signing
    let signer_key = ctx.accounts.signer.key();
    let mint_key = ctx.accounts.mint.key();
    let seeds = &[
        b"vault_info",
        signer_key.as_ref(),
        mint_key.as_ref(),
        &[vault_info.vault_info_bump]
    ];
    let signer_seeds = &[&seeds[..]];
    
    // Transfer to each target (validate right before transfer)
    for target in targets {
        // Validation at transfer time to prevent TOCTOU
        require!(target.owner == &anchor_spl::token::ID, CustomError::InvalidTokenAccount);
        require!(!target.data_is_empty(), CustomError::AccountNotInitialized);
        
  let target_account = Account::<TokenAccount>::try_from(target)
        .map_err(|_| CustomError::InvalidTokenAccount)?;
    
    require!(target_account.mint == vault_info.mint, CustomError::InvalidMint);
        
        
        // Transfer immediately after validation
        let cpi_accounts = Transfer {
            from: vault_token_account.to_account_info(),
            to: target.to_account_info(),
            authority: vault_info.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds
        );
        token::transfer(cpi_ctx, split_amount)?;
    }
    
    // Return remainder to user if any
    if remainder > 0 {
        let cpi_accounts = Transfer {
            from: vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_acc.to_account_info(),
            authority: vault_info.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds
        );
        token::transfer(cpi_ctx, remainder)?;
    }
    
    let total_transfer = split_amount
    .checked_mul(num_targets)
    .and_then(|x| x.checked_add(remainder))
    .ok_or(CustomError::Overflow)?;
    vault_info.amount = vault_info.amount.checked_sub(total_transfer).ok_or(CustomError::InsufficientFunds)?;
    Ok(())
}

#[derive(Accounts)]
pub struct ShareFunds<'info>{
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