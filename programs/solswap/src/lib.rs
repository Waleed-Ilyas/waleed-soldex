use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("DdQhz1JLghP5q67tjgPZHRT4U3mPk5QxzWYK2wt8QAuD");

const FEE_DENOMINATOR: u128 = 10_000;
const POOL_FEE_BPS: u16 = 30;

#[program]
pub mod solswap {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
        require!(
            ctx.accounts.mint_a.key().to_bytes() < ctx.accounts.mint_b.key().to_bytes(),
            SwapError::MintsNotCanonical
        );
        let pool = &mut ctx.accounts.pool;
        pool.authority = ctx.accounts.authority.key();
        pool.mint_a = ctx.accounts.mint_a.key();
        pool.mint_b = ctx.accounts.mint_b.key();
        pool.reserve_a = 0;
        pool.reserve_b = 0;
        pool.total_liquidity = 0;
        pool.fee_bps = POOL_FEE_BPS;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        position_id: u64,
        amount_a: u64,
        amount_b: u64,
        minimum_shares: u64,
    ) -> Result<()> {
        require!(amount_a > 0 && amount_b > 0, SwapError::ZeroAmount);
        let pool = &ctx.accounts.pool;
        let shares = if pool.total_liquidity == 0 {
            require!(
                pool.reserve_a == 0 && pool.reserve_b == 0,
                SwapError::InvalidReserves
            );
            integer_sqrt((amount_a as u128) * (amount_b as u128))?
        } else {
            require!(
                pool.reserve_a > 0 && pool.reserve_b > 0,
                SwapError::InvalidReserves
            );
            let expected_b = checked_mul_div(amount_a, pool.reserve_b, pool.reserve_a)?;
            require!(
                amount_b.abs_diff(expected_b) <= 1,
                SwapError::ImbalancedLiquidity
            );
            let shares_a = checked_mul_div(amount_a, pool.total_liquidity, pool.reserve_a)?;
            let shares_b = checked_mul_div(amount_b, pool.total_liquidity, pool.reserve_b)?;
            shares_a.min(shares_b)
        };
        require!(
            shares > 0 && shares >= minimum_shares,
            SwapError::InsufficientShares
        );

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_a.to_account_info(),
                    to: ctx.accounts.vault_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_a,
        )?;
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_b.to_account_info(),
                    to: ctx.accounts.vault_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_b,
        )?;

        let pool = &mut ctx.accounts.pool;
        pool.reserve_a = pool
            .reserve_a
            .checked_add(amount_a)
            .ok_or(SwapError::MathOverflow)?;
        pool.reserve_b = pool
            .reserve_b
            .checked_add(amount_b)
            .ok_or(SwapError::MathOverflow)?;
        pool.total_liquidity = pool
            .total_liquidity
            .checked_add(shares)
            .ok_or(SwapError::MathOverflow)?;

        let position = &mut ctx.accounts.position;
        position.pool = pool.key();
        position.owner = ctx.accounts.user.key();
        position.position_id = position_id;
        position.shares = shares;
        position.bump = ctx.bumps.position;
        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, minimum_out: u64, a_to_b: bool) -> Result<()> {
        require!(amount_in > 0, SwapError::ZeroAmount);
        let (reserve_in, reserve_out) = if a_to_b {
            (ctx.accounts.pool.reserve_a, ctx.accounts.pool.reserve_b)
        } else {
            (ctx.accounts.pool.reserve_b, ctx.accounts.pool.reserve_a)
        };
        require!(reserve_in > 0 && reserve_out > 0, SwapError::EmptyPool);

        let amount_out = constant_product_out(
            amount_in,
            reserve_in,
            reserve_out,
            ctx.accounts.pool.fee_bps,
        )?;
        require!(
            amount_out > 0 && amount_out >= minimum_out,
            SwapError::SlippageExceeded
        );
        require!(amount_out < reserve_out, SwapError::InsufficientLiquidity);

        let (user_source, user_destination, vault_source, vault_destination) = if a_to_b {
            (
                ctx.accounts.user_a.to_account_info(),
                ctx.accounts.user_b.to_account_info(),
                ctx.accounts.vault_a.to_account_info(),
                ctx.accounts.vault_b.to_account_info(),
            )
        } else {
            (
                ctx.accounts.user_b.to_account_info(),
                ctx.accounts.user_a.to_account_info(),
                ctx.accounts.vault_b.to_account_info(),
                ctx.accounts.vault_a.to_account_info(),
            )
        };
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: user_source,
                    to: vault_source,
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_in,
        )?;

        let mint_a = ctx.accounts.pool.mint_a;
        let mint_b = ctx.accounts.pool.mint_b;
        let bump = [ctx.accounts.pool.bump];
        let signer_seeds: &[&[u8]] = &[b"pool", mint_a.as_ref(), mint_b.as_ref(), &bump];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: vault_destination,
                    to: user_destination,
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[signer_seeds],
            ),
            amount_out,
        )?;

        let pool = &mut ctx.accounts.pool;
        if a_to_b {
            pool.reserve_a = pool
                .reserve_a
                .checked_add(amount_in)
                .ok_or(SwapError::MathOverflow)?;
            pool.reserve_b = pool
                .reserve_b
                .checked_sub(amount_out)
                .ok_or(SwapError::MathOverflow)?;
        } else {
            pool.reserve_b = pool
                .reserve_b
                .checked_add(amount_in)
                .ok_or(SwapError::MathOverflow)?;
            pool.reserve_a = pool
                .reserve_a
                .checked_sub(amount_out)
                .ok_or(SwapError::MathOverflow)?;
        }
        Ok(())
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        minimum_a: u64,
        minimum_b: u64,
    ) -> Result<()> {
        let shares = ctx.accounts.position.shares;
        let pool = &ctx.accounts.pool;
        require!(
            shares > 0 && pool.total_liquidity > 0,
            SwapError::InsufficientShares
        );
        let amount_a = checked_mul_div(pool.reserve_a, shares, pool.total_liquidity)?;
        let amount_b = checked_mul_div(pool.reserve_b, shares, pool.total_liquidity)?;
        require!(
            amount_a >= minimum_a && amount_b >= minimum_b,
            SwapError::SlippageExceeded
        );

        let mint_a = pool.mint_a;
        let mint_b = pool.mint_b;
        let bump = [pool.bump];
        let signer_seeds: &[&[u8]] = &[b"pool", mint_a.as_ref(), mint_b.as_ref(), &bump];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_a.to_account_info(),
                    to: ctx.accounts.user_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[signer_seeds],
            ),
            amount_a,
        )?;
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_b.to_account_info(),
                    to: ctx.accounts.user_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[signer_seeds],
            ),
            amount_b,
        )?;

        let pool = &mut ctx.accounts.pool;
        pool.reserve_a = pool
            .reserve_a
            .checked_sub(amount_a)
            .ok_or(SwapError::MathOverflow)?;
        pool.reserve_b = pool
            .reserve_b
            .checked_sub(amount_b)
            .ok_or(SwapError::MathOverflow)?;
        pool.total_liquidity = pool
            .total_liquidity
            .checked_sub(shares)
            .ok_or(SwapError::MathOverflow)?;
        Ok(())
    }
}

fn checked_mul_div(left: u64, right: u64, denominator: u64) -> Result<u64> {
    require!(denominator > 0, SwapError::InvalidReserves);
    let value = (left as u128)
        .checked_mul(right as u128)
        .ok_or(SwapError::MathOverflow)?
        / denominator as u128;
    u64::try_from(value).map_err(|_| error!(SwapError::MathOverflow))
}

fn constant_product_out(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<u64> {
    require!(amount_in > 0, SwapError::ZeroAmount);
    require!(reserve_in > 0 && reserve_out > 0, SwapError::EmptyPool);
    require!(
        (fee_bps as u128) < FEE_DENOMINATOR,
        SwapError::InvalidReserves
    );

    let amount_in_with_fee = (amount_in as u128)
        .checked_mul(FEE_DENOMINATOR - fee_bps as u128)
        .ok_or(SwapError::MathOverflow)?;
    let numerator = amount_in_with_fee
        .checked_mul(reserve_out as u128)
        .ok_or(SwapError::MathOverflow)?;
    let denominator = (reserve_in as u128)
        .checked_mul(FEE_DENOMINATOR)
        .and_then(|value| value.checked_add(amount_in_with_fee))
        .ok_or(SwapError::MathOverflow)?;
    u64::try_from(numerator / denominator).map_err(|_| error!(SwapError::MathOverflow))
}

fn integer_sqrt(value: u128) -> Result<u64> {
    require!(value > 0, SwapError::ZeroAmount);
    let mut current = value;
    let mut next = (current + 1) / 2;
    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }
    u64::try_from(current).map_err(|_| error!(SwapError::MathOverflow))
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,
    #[account(
        init,
        payer = authority,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        payer = authority,
        associated_token::mint = mint_a,
        associated_token::authority = pool
    )]
    pub vault_a: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        associated_token::mint = mint_b,
        associated_token::authority = pool
    )]
    pub vault_b: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_id: u64)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump,
        has_one = mint_a,
        has_one = mint_b
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        payer = user,
        space = 8 + LiquidityPosition::INIT_SPACE,
        seeds = [b"position", pool.key().as_ref(), user.key().as_ref(), &position_id.to_le_bytes()],
        bump
    )]
    pub position: Account<'info, LiquidityPosition>,
    #[account(mut, token::mint = mint_a, token::authority = user)]
    pub user_a: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint_b, token::authority = user)]
    pub user_b: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_a, associated_token::authority = pool)]
    pub vault_a: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_b, associated_token::authority = pool)]
    pub vault_b: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump,
        has_one = mint_a,
        has_one = mint_b
    )]
    pub pool: Account<'info, Pool>,
    #[account(mut, token::mint = mint_a, token::authority = user)]
    pub user_a: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint_b, token::authority = user)]
    pub user_b: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_a, associated_token::authority = pool)]
    pub vault_a: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_b, associated_token::authority = pool)]
    pub vault_b: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump = pool.bump,
        has_one = mint_a,
        has_one = mint_b
    )]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"position", pool.key().as_ref(), owner.key().as_ref(), &position.position_id.to_le_bytes()],
        bump = position.bump,
        has_one = pool,
        has_one = owner,
        close = owner
    )]
    pub position: Account<'info, LiquidityPosition>,
    #[account(mut, token::mint = mint_a, token::authority = owner)]
    pub user_a: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint_b, token::authority = owner)]
    pub user_b: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_a, associated_token::authority = pool)]
    pub vault_a: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = mint_b, associated_token::authority = pool)]
    pub vault_b: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub authority: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_liquidity: u64,
    pub fee_bps: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct LiquidityPosition {
    pub pool: Pubkey,
    pub owner: Pubkey,
    pub position_id: u64,
    pub shares: u64,
    pub bump: u8,
}

#[error_code]
pub enum SwapError {
    #[msg("Order the mint addresses by public key to create a canonical pool.")]
    MintsNotCanonical,
    #[msg("Amounts must be greater than zero.")]
    ZeroAmount,
    #[msg("The pool reserves are inconsistent or empty.")]
    InvalidReserves,
    #[msg("Liquidity deposits must match the existing pool ratio.")]
    ImbalancedLiquidity,
    #[msg("The requested liquidity shares are unavailable or below the minimum.")]
    InsufficientShares,
    #[msg("The pool has no available liquidity for this swap.")]
    EmptyPool,
    #[msg("The requested output is below the minimum or the trade is too large.")]
    SlippageExceeded,
    #[msg("The pool cannot satisfy the requested output.")]
    InsufficientLiquidity,
    #[msg("A checked arithmetic operation overflowed.")]
    MathOverflow,
}

#[cfg(test)]
mod tests {
    use super::{checked_mul_div, constant_product_out, integer_sqrt};

    #[test]
    fn square_root_of_liquidity_product_is_integer_and_floored() {
        assert_eq!(integer_sqrt(144).unwrap(), 12);
        assert_eq!(integer_sqrt(143).unwrap(), 11);
        assert!(integer_sqrt(0).is_err());
    }

    #[test]
    fn proportional_math_uses_checked_u128_intermediates() {
        assert_eq!(checked_mul_div(10, 20, 5).unwrap(), 40);
        assert!(checked_mul_div(u64::MAX, u64::MAX, 1).is_err());
    }

    #[test]
    fn constant_product_quote_applies_fee_and_reserve_limits() {
        assert_eq!(constant_product_out(100, 1_000, 1_000, 30).unwrap(), 90);
        assert!(constant_product_out(1, 0, 100, 30).is_err());
        assert!(constant_product_out(100, 1_000, 1_000, 10_000).is_err());
    }
}
