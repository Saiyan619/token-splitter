pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use error::*;
pub use instructions::*;
pub use state::*;

declare_id!("HgWKLR2a2TreZCaEJZhzC7pwuQdQaU5bTb7eYPJC7s9B");

#[program]
pub mod token_splitter {
     use super::*;
    pub fn initialize_vault(ctx: Context<Initialize>) -> Result<()> {
       _initialize_vault(ctx)
    }

    pub fn deposit_vault(ctx: Context<Deposit>, amount:u64)->Result<()>{
       _deposit_vault(ctx, amount)
    }

   pub fn share_funds<'info>(ctx: Context<'_, '_,'info,'info, ShareFunds<'info>>) -> Result<()> {
_share_funds(ctx)
}

pub fn withdraw(ctx: Context<WithDraw>)->Result<()>{
   _withdraw(ctx)
}

pub fn close_vault(ctx: Context<CloseVault>)->Result<()>{
    _close_vault(ctx)
}

}
