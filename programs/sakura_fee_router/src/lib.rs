use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

declare_id!("FNoE2JUhn981hBDyBMvWJYkw9DThhtYwWoPbw6wgz1rg");

pub const SAKURA_MINT: Pubkey = pubkey!("EWiVNxCqNatzV2paBHyfKUwGLnk7WKs9uZTA5jkTpump");

// TODO: Replace this with the actual mainnet Percolator Insurance Vault for the corresponding slab
pub const PERCOLATOR_INSURANCE_VAULT: Pubkey = pubkey!("63juJmvm1XHCHveWv9WdanxqJX6tD6DLFTZD7dvH12dc");

pub const INSURANCE_BPS: u64 = 5000;
pub const BURN_BPS: u64 = 5000;

// Roughly 400ms per slot = ~2.5 slots per second = ~216,000 slots per day
pub const DAILY_SLOTS: u64 = 216_000; 
// 30 days subscription
pub const SUBSCRIPTION_SLOTS: u64 = 30 * DAILY_SLOTS;

#[program]
pub mod sakura_fee_router {
    use super::*;

    pub fn process_payment(ctx: Context<ProcessPayment>, amount: u64) -> Result<()> {
        // Enforce safe math constraints
        require!(INSURANCE_BPS + BURN_BPS == 10_000, ErrorCode::InvalidSplit);
        require!(amount > 0, ErrorCode::InvalidAmount);
        
        // 1. Calculate splits (immutable BPS)
        let insurance_amount = amount.checked_mul(INSURANCE_BPS).unwrap().checked_div(10_000).unwrap();
        let burn_amount = amount.checked_sub(insurance_amount).unwrap();

        // 2. Route funds to the percolator insurance vault
        let transfer_cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.insurance_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_cpi_accounts,
        );
        token::transfer(transfer_ctx, insurance_amount)?;

        // 3. Burn the remaining tokens out of existence permanently
        let burn_cpi_accounts = Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            burn_cpi_accounts,
        );
        token::burn(burn_ctx, burn_amount)?;

        // 4. Update the on-chain Option B Subscription PDA
        let clock = Clock::get()?;
        let current_slot = clock.slot;

        let subscription = &mut ctx.accounts.subscription;
        if subscription.expires_at_slot < current_slot {
            subscription.expires_at_slot = current_slot.checked_add(SUBSCRIPTION_SLOTS).unwrap();
        } else {
            subscription.expires_at_slot = subscription.expires_at_slot.checked_add(SUBSCRIPTION_SLOTS).unwrap();
        }
        subscription.user = ctx.accounts.user.key();

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ProcessPayment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ ErrorCode::InvalidOwner,
        constraint = user_token_account.mint == SAKURA_MINT @ ErrorCode::InvalidMint
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = PERCOLATOR_INSURANCE_VAULT @ ErrorCode::InvalidVault
    )]
    pub insurance_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        address = SAKURA_MINT @ ErrorCode::InvalidMint
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 8, // discriminator + pubkey + u64
        seeds = [b"subscription", user.key().as_ref()],
        bump
    )]
    pub subscription: Account<'info, Subscription>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Subscription {
    pub user: Pubkey,
    pub expires_at_slot: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid split percentages, must sum to 10000 BPS")]
    InvalidSplit,
    #[msg("Payment amount must be greater than zero")]
    InvalidAmount,
    #[msg("Invalid user token account owner")]
    InvalidOwner,
    #[msg("Invalid token mint, must be the official SAKURA_MINT")]
    InvalidMint,
    #[msg("Invalid insurance vault, must match the designated Percolator vault")]
    InvalidVault,
}
