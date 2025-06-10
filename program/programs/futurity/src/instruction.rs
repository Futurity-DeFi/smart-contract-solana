use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum FuturityInstruction {
    CreateTimeLockDeposit { amount: u64, unlock_time: i64 },
    WithdrawDeposit,
    CloseExpiredDeposit,
}
