use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, CloseAccount};

declare_id!("HgWKLR2a2TreZCaEJZhzC7pwuQdQaU5bTb7eYPJC7s9B");

#[program]
pub mod token_splitter {
     use super::*;
    pub fn initialize_vault(ctx: Context<Initialize>) -> Result<()> {
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
    pub fn deposit_vault(ctx: Context<Deposit>, amount:u64)->Result<()>{
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

   pub fn share_funds<'info>(ctx: Context<'_, '_, '_, 'info, ShareFunds<'info>>) -> Result<()> {
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
        
        let mut data_slice: &[u8] = &target.try_borrow_data()?;
        let target_token_account = TokenAccount::try_deserialize(&mut data_slice)
            .map_err(|_| CustomError::InvalidTokenAccount)?;
        
        require!(target_token_account.mint == vault_info.mint, CustomError::InvalidMint);
        
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

pub fn withdraw(ctx: Context<WithDraw>)->Result<()>{
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

pub fn close_vault(ctx: Context<CloseVault>)->Result<()>{
    let vault_info = &mut ctx.accounts.vault_info;
        let vault_token_acc=&mut ctx.accounts.vault_token_acc;
        let signer = &ctx.accounts.signer.key();
        let mint = &ctx.accounts.mint.key();
        require!(vault_info.owner == signer.key(), CustomError::Unauthorized);
        require!(signer.key()==vault_token_acc.owner, CustomError::Unauthorized);
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
#[account]
pub struct VaultInfo {
    pub owner:Pubkey,
    pub amount: u64,
    pub mint:Pubkey,
    pub vault_info_bump:u8,
    pub vault_token_bump:u8,
    pub created_at:i64
}

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
