use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum TimeLockError {
    #[error("Unlock time must be in the future")] 
    InvalidUnlockTime,
    #[error("Time-lock duration too short")] 
    LockDurationTooShort,
    #[error("Deposit is still time-locked")] 
    StillLocked,
    #[error("Deposit has already been withdrawn")] 
    AlreadyWithdrawn,
    #[error("Only sender or recipient may withdraw")] 
    UnauthorizedWithdrawal,
    #[error("Amount below minimum or rent-exempt threshold")] 
    InsufficientAmount,
    #[error("Insufficient balance to create deposit")] 
    InsufficientBalance,
    #[error("Unlock time exceeds 100-year maximum")] 
    ExcessiveUnlockTime,
    #[error("No funds available to withdraw")] 
    InsufficientFunds,
    #[error("Too early to close withdrawn deposit")] 
    TooEarlyToClose,
    #[error("Cannot close an active (non-withdrawn) deposit")] 
    CannotCloseActiveDeposit,
}

impl From<TimeLockError> for ProgramError {
    fn from(e: TimeLockError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
