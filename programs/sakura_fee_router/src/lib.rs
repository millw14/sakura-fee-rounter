use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

declare_id!("FNoE2JUhn981hBDyBMvWJYkw9DThhtYwWoPbw6wgz1rg");

pub const SAKURA_MINT: Pubkey = pubkey!("EWiVNxCqNatzV2paBHyfKUwGLnk7WKs9uZTA5jkTpump");

// TODO: Replace this with the actual mainnet Percolator Insurance Vault for the corresponding slab
pub const PERCOLATOR_INSURANCE_VAULT: Pubkey =
    pubkey!("63juJmvm1XHCHveWv9WdanxqJX6tD6DLFTZD7dvH12dc");

pub const PERCOLATOR_VAULT_AUTHORITY: Pubkey = pubkey!("11111111111111111111111111111111");

pub const INSURANCE_BPS: u64 = 5000;
pub const BURN_BPS: u64 = 5000;

// 30 days subscription in seconds
pub const SUBSCRIPTION_TIME: i64 = 30 * 24 * 60 * 60;

#[program]
pub mod sakura_fee_router {
    use super::*;

    pub fn process_payment(ctx: Context<ProcessPayment>, amount: u64) -> Result<()> {
        // Enforce safe math constraints
        require!(INSURANCE_BPS + BURN_BPS == 10_000, ErrorCode::InvalidSplit);
        require!(amount > 0, ErrorCode::InvalidAmount);

        // 1. Calculate splits (immutable BPS)
        let insurance_amount = amount
            .checked_mul(INSURANCE_BPS)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
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

        // 4. Update the on-chain Option B Subscription PDA using unix_timestamp
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        let subscription = &mut ctx.accounts.subscription;

        let base_time = std::cmp::max(current_time, subscription.expires_at);
        subscription.expires_at = base_time.checked_add(SUBSCRIPTION_TIME).unwrap();

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
        address = PERCOLATOR_INSURANCE_VAULT @ ErrorCode::InvalidVault,
        constraint = insurance_vault.mint == SAKURA_MINT @ ErrorCode::InvalidVaultMint,
        // The TokenAccount.owner field represents the SPL token authority over the vault
        constraint = insurance_vault.owner == PERCOLATOR_VAULT_AUTHORITY @ ErrorCode::InvalidVaultAuthority,
        // The token program natively owns the token accounts
        owner = token::ID @ ErrorCode::InvalidVaultOwner
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
        space = 8 + 32 + 8, // discriminator + pubkey + i64
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
    pub expires_at: i64,
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
    #[msg("Invalid insurance vault mint")]
    InvalidVaultMint,
    #[msg("Invalid insurance vault owner")]
    InvalidVaultOwner,
    #[msg("Invalid insurance vault authority")]
    InvalidVaultAuthority,
}
