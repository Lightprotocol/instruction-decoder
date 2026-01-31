use anchor_lang::prelude::*;
use light_instruction_decoder_derive::instruction_decoder;

declare_id!("Counter111111111111111111111111111111111111");

#[instruction_decoder]
#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.counter.count = 0;
        ctx.accounts.counter.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.counter.count += 1;
        Ok(())
    }

    pub fn decrement(ctx: Context<Decrement>) -> Result<()> {
        ctx.accounts.counter.count -= 1;
        Ok(())
    }

    pub fn set(ctx: Context<Set>, value: u64) -> Result<()> {
        ctx.accounts.counter.count = value;
        Ok(())
    }

    pub fn configure(
        ctx: Context<Configure>,
        new_value: u64,
        multiplier: u16,
        enabled: bool,
        label: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        if enabled {
            ctx.accounts.counter.count = new_value.saturating_mul(multiplier as u64);
        }
        // Use label and nonce to avoid unused warnings
        msg!("label: {:?}, nonce: {}", &label[..4], nonce);
        Ok(())
    }
}

#[account]
pub struct Counter {
    pub count: u64,
    pub authority: Pubkey,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 8 + 32)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Decrement<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Set<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Configure<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
    /// CHECK: Delegate authority
    pub delegate: UncheckedAccount<'info>,
    /// CHECK: Fee receiver
    #[account(mut)]
    pub fee_receiver: UncheckedAccount<'info>,
    /// CHECK: Config account
    pub config: UncheckedAccount<'info>,
    /// CHECK: Metadata account
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Oracle account
    pub oracle: UncheckedAccount<'info>,
    /// CHECK: Backup authority
    pub backup_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Rent sysvar
    pub rent: UncheckedAccount<'info>,
}
