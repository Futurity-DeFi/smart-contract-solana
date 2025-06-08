use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("G5vZC5LFoB3yFTyFtZjhW7mf8p9yuioVgFJ8WEGP2g1a");

/// -------------------------------------------------------------------------
/// Security.txt for Futurity Protocol
/// -------------------------------------------------------------------------
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Futurity DeFi",
    project_url: "https://futurity.fi",
    contacts: "email:security@futurity.fi,twitter:@FuturityDeFi",
    policy: "https://futurity.fi/privacy",
    preferred_languages: "en",
    source_code: "https://github.com/futurity-defi/smart-contract-solana"
}

/// -------------------------------------------------------------------------
/// FUTURITY PROTOCOL – Secure Time-Locked Escrow
/// -------------------------------------------------------------------------

#[program]
pub mod futurity {
    use super::*;

    // --------------------------  Constants  --------------------------------
    /// Minimum user-intended deposit (ex rent). Actual min is max(this, rent).
    const MIN_USER_DEPOSIT_LAMPORTS: u64 = 1_000_000;                      // 0.001 SOL
    const MIN_LOCK_DURATION:        i64 = 60;                              // 60 s
    const MAX_LOCK_DURATION:        i64 = 100 * 365 * 24 * 60 * 60;        // 100 y
    const CLOSE_GRACE_PERIOD:       i64 = 365 * 24 * 60 * 60;              // 1 y

    // -----------------------  create_time_lock_deposit  --------------------
    pub fn create_time_lock_deposit(
        ctx: Context<CreateTimeLockDeposit>,
        amount: u64,
        unlock_time: i64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let now   = clock.unix_timestamp;

        // ------------------------ validation --------------------------------
        let rent_exempt = Rent::get()?.minimum_balance(TimeLockDeposit::INIT_SPACE + 8);
        let min_required = rent_exempt.max(MIN_USER_DEPOSIT_LAMPORTS);

        // user must deposit at least max(user_min, rent)
        require!(amount >= min_required,               TimeLockError::InsufficientAmount);
        require!(unlock_time > now,                    TimeLockError::InvalidUnlockTime);
        require!(
            unlock_time - now >= MIN_LOCK_DURATION,
            TimeLockError::LockDurationTooShort
        );
        let max_unlock = now
            .checked_add(MAX_LOCK_DURATION)
            .ok_or(TimeLockError::ExcessiveUnlockTime)?;
        require!(unlock_time <= max_unlock,            TimeLockError::ExcessiveUnlockTime);

        // sender must have funds for both deposit amount *and* PDA rent
        let sender_balance = ctx.accounts.sender.to_account_info().lamports();
        require!(
            sender_balance >= amount + rent_exempt,
            TimeLockError::InsufficientBalance
        );
        // --------------------------------------------------------------------

        // Transfer SOL into the PDA escrow
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.sender.to_account_info(),
                    to:   ctx.accounts.deposit.to_account_info(),
                },
            ),
            amount,
        )?;

        // Populate escrow state
        let deposit = &mut ctx.accounts.deposit;
        deposit.sender       = ctx.accounts.sender.key();
        deposit.recipient    = ctx.accounts.recipient.key();
        deposit.amount       = amount;
        deposit.unlock_time  = unlock_time;
        deposit.created_at   = now;
        deposit.is_withdrawn = false;
        deposit.bump         = ctx.bumps.deposit;

        emit!(EscrowCreatedEvent {
            deposit_address: deposit.key(),
            sender:          deposit.sender,
            recipient:       deposit.recipient,
            amount,
            unlock_time,
            created_at:      now,
        });

        Ok(())
    }

    // ---------------------------  withdraw_deposit  ------------------------
    pub fn withdraw_deposit(ctx: Context<WithdrawDeposit>) -> Result<()> {
        let clock      = Clock::get()?;
        let deposit    = &mut ctx.accounts.deposit;
        let withdrawer = ctx.accounts.withdrawer.key();

        require!(clock.unix_timestamp >= deposit.unlock_time, TimeLockError::StillLocked);
        require!(
            withdrawer == deposit.sender || withdrawer == deposit.recipient,
            TimeLockError::UnauthorizedWithdrawal
        );

        let rent_exempt      = Rent::get()?.minimum_balance(deposit.to_account_info().data_len());
        let escrow_balance   = deposit.to_account_info().lamports();
        let withdrawable_raw = escrow_balance
            .checked_sub(rent_exempt)
            .ok_or(TimeLockError::InsufficientFunds)?;

        // Cap to originally recorded amount to neutralise forced lamport transfers.
        let withdraw_amount = withdrawable_raw.min(deposit.amount);
        require!(withdraw_amount > 0, TimeLockError::InsufficientFunds);

        **deposit.to_account_info().try_borrow_mut_lamports()? -= withdraw_amount;
        **ctx.accounts.withdrawer.to_account_info().try_borrow_mut_lamports()? += withdraw_amount;

        deposit.is_withdrawn  = true;
        deposit.withdrawn_at  = Some(clock.unix_timestamp);
        deposit.withdrawn_by  = Some(withdrawer);

        emit!(EscrowWithdrawnEvent {
            deposit_address:    deposit.key(),
            withdrawer,
            amount:             withdraw_amount,
            withdrawn_at:       clock.unix_timestamp,
            original_sender:    deposit.sender,
            original_recipient: deposit.recipient,
        });

        Ok(())
    }

    // ------------------------  close_expired_deposit  ----------------------
    /// Anyone may close after the grace period.  
    /// Remaining rent is sent to the mutable `rent_receiver` account, defaulting
    /// to the original sender if they choose to sign.
    pub fn close_expired_deposit(ctx: Context<CloseExpiredDeposit>) -> Result<()> {
        let clock   = Clock::get()?;
        let deposit = &ctx.accounts.deposit;

        require!(deposit.is_withdrawn, TimeLockError::CannotCloseActiveDeposit);
        require!(
            clock.unix_timestamp >= deposit.unlock_time + CLOSE_GRACE_PERIOD,
            TimeLockError::TooEarlyToClose
        );

        // No additional logic required: Anchor `close = rent_receiver` handles
        // lamport refund. Emit event for transparency.
        emit!(EscrowClosedEvent {
            deposit_address: deposit.key(),
            rent_receiver:   ctx.accounts.rent_receiver.key(),
            closed_at:       clock.unix_timestamp,
        });

        Ok(())
    }
}

// --------------------------------------------------------------------------
// Account Contexts
// --------------------------------------------------------------------------
#[derive(Accounts)]
#[instruction(amount: u64, unlock_time: i64)]
pub struct CreateTimeLockDeposit<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    /// CHECK: may equal `sender` (self-escrow)
    pub recipient: AccountInfo<'info>,

    #[account(
        init,
        payer  = sender,
        space  = 8 + TimeLockDeposit::INIT_SPACE,
        seeds  = [
            b"futurity_escrow",
            sender.key().as_ref(),
            recipient.key().as_ref(),
            &amount.to_le_bytes(),
            &unlock_time.to_le_bytes(),
        ],
        bump
    )]
    pub deposit: Account<'info, TimeLockDeposit>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawDeposit<'info> {
    #[account(mut)]
    pub withdrawer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"futurity_escrow",
            deposit.sender.as_ref(),
            deposit.recipient.as_ref(),
            &deposit.amount.to_le_bytes(),
            &deposit.unlock_time.to_le_bytes(),
        ],
        bump = deposit.bump,
        constraint = !deposit.is_withdrawn @ TimeLockError::AlreadyWithdrawn,
    )]
    pub deposit: Account<'info, TimeLockDeposit>,
}

#[derive(Accounts)]
pub struct CloseExpiredDeposit<'info> {
    #[account(mut)]
    pub closer: Signer<'info>,          // anyone may trigger closure

    /// Account that will receive remaining rent lamports.
    /// CHECK: rent_receiver can be any account to receive remaining rent lamports after closure. This is safe because only lamports are transferred, not data or authority.
    #[account(mut)]
    pub rent_receiver: AccountInfo<'info>,

    #[account(
        mut,
        close = rent_receiver,
        seeds = [
            b"futurity_escrow",
            deposit.sender.as_ref(),
            deposit.recipient.as_ref(),
            &deposit.amount.to_le_bytes(),
            &deposit.unlock_time.to_le_bytes(),
        ],
        bump = deposit.bump,
    )]
    pub deposit: Account<'info, TimeLockDeposit>,
}

// --------------------------------------------------------------------------
// Escrow State
// --------------------------------------------------------------------------
#[account]
#[derive(InitSpace)]
pub struct TimeLockDeposit {
    pub sender:        Pubkey,
    pub recipient:     Pubkey,
    pub amount:        u64,
    pub unlock_time:   i64,
    pub created_at:    i64,
    pub is_withdrawn:  bool,
    pub withdrawn_at:  Option<i64>,
    pub withdrawn_by:  Option<Pubkey>,
    pub bump:          u8,
}

// --------------------------------------------------------------------------
// Events
// --------------------------------------------------------------------------
#[event]
pub struct EscrowCreatedEvent {
    pub deposit_address: Pubkey,
    pub sender:          Pubkey,
    pub recipient:       Pubkey,
    pub amount:          u64,
    pub unlock_time:     i64,
    pub created_at:      i64,
}

#[event]
pub struct EscrowWithdrawnEvent {
    pub deposit_address:    Pubkey,
    pub withdrawer:         Pubkey,
    pub amount:             u64,
    pub withdrawn_at:       i64,
    pub original_sender:    Pubkey,
    pub original_recipient: Pubkey,
}

#[event]
pub struct EscrowClosedEvent {
    pub deposit_address: Pubkey,
    pub rent_receiver:   Pubkey,
    pub closed_at:       i64,
}

// --------------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------------
#[error_code]
pub enum TimeLockError {
    #[msg("Unlock time must be in the future")]
    InvalidUnlockTime,
    #[msg("Time-lock duration too short")]
    LockDurationTooShort,
    #[msg("Deposit is still time-locked")]
    StillLocked,
    #[msg("Deposit has already been withdrawn")]
    AlreadyWithdrawn,
    #[msg("Only sender or recipient may withdraw")]
    UnauthorizedWithdrawal,
    #[msg("Amount below minimum or rent-exempt threshold")]
    InsufficientAmount,
    #[msg("Insufficient balance to create deposit")]
    InsufficientBalance,
    #[msg("Unlock time exceeds 100-year maximum")]
    ExcessiveUnlockTime,
    #[msg("No funds available to withdraw")]
    InsufficientFunds,
    #[msg("Too early to close withdrawn deposit")]
    TooEarlyToClose,
    #[msg("Cannot close an active (non-withdrawn) deposit")]
    CannotCloseActiveDeposit,
}