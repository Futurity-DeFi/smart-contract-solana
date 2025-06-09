use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
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

pub const TIME_LOCK_DEPOSIT_LEN: usize = 32 + 32 + 8 + 8 + 8 + 1 + 9 + 33 + 1;
