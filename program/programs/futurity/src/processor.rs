use crate::error::TimeLockError;
use crate::instruction::FuturityInstruction;
use crate::state::{TimeLockDeposit, TIME_LOCK_DEPOSIT_LEN};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program::sysvar::{clock::Clock, rent::Rent, Sysvar};
use solana_program::msg;

pub struct Processor;

impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = FuturityInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        match instruction {
            FuturityInstruction::CreateTimeLockDeposit { amount, unlock_time } => {
                Self::process_create(program_id, accounts, amount, unlock_time)
            }
            FuturityInstruction::WithdrawDeposit => Self::process_withdraw(program_id, accounts),
            FuturityInstruction::CloseExpiredDeposit => Self::process_close(program_id, accounts),
        }
    }

    fn process_create(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64, unlock_time: i64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?; // signer
        let recipient = next_account_info(account_info_iter)?; // any
        let deposit = next_account_info(account_info_iter)?; // PDA
        let system_program = next_account_info(account_info_iter)?;

        if !sender.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        if unlock_time <= now {
            return Err(TimeLockError::InvalidUnlockTime.into());
        }

        const MIN_USER_DEPOSIT_LAMPORTS: u64 = 1_000_000;
        const MIN_LOCK_DURATION: i64 = 60;
        const MAX_LOCK_DURATION: i64 = 100 * 365 * 24 * 60 * 60;

        if unlock_time - now < MIN_LOCK_DURATION {
            return Err(TimeLockError::LockDurationTooShort.into());
        }
        let max_unlock = now.checked_add(MAX_LOCK_DURATION).ok_or(TimeLockError::ExcessiveUnlockTime)?;
        if unlock_time > max_unlock {
            return Err(TimeLockError::ExcessiveUnlockTime.into());
        }

        let rent = Rent::get()?;
        let rent_exempt = rent.minimum_balance(TIME_LOCK_DEPOSIT_LEN);
        let min_required = rent_exempt.max(MIN_USER_DEPOSIT_LAMPORTS);
        if amount < min_required {
            return Err(TimeLockError::InsufficientAmount.into());
        }
        if **sender.lamports.borrow() < amount + rent_exempt {
            return Err(TimeLockError::InsufficientBalance.into());
        }

        let seeds: [&[u8]; 5] = [
            b"futurity_escrow",
            sender.key.as_ref(),
            recipient.key.as_ref(),
            &amount.to_le_bytes(),
            &unlock_time.to_le_bytes(),
        ];
        let (expected_deposit, bump) = Pubkey::find_program_address(&seeds, program_id);
        if expected_deposit != *deposit.key {
            return Err(ProgramError::InvalidArgument);
        }

        // create account and fund with amount + rent
        invoke_signed(
            &system_instruction::create_account(
                sender.key,
                deposit.key,
                rent_exempt + amount,
                TIME_LOCK_DEPOSIT_LEN as u64,
                program_id,
            ),
            &[sender.clone(), deposit.clone(), system_program.clone()],
            &[&[b"futurity_escrow", sender.key.as_ref(), recipient.key.as_ref(), &amount.to_le_bytes(), &unlock_time.to_le_bytes(), &[bump]]],
        )?;

        let mut deposit_data = TimeLockDeposit {
            sender: *sender.key,
            recipient: *recipient.key,
            amount,
            unlock_time,
            created_at: now,
            is_withdrawn: false,
            withdrawn_at: None,
            withdrawn_by: None,
            bump,
        };
        deposit_data.serialize(&mut &mut deposit.data.borrow_mut()[..])?;
        msg!("Escrow created");
        Ok(())
    }

    fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let withdrawer = next_account_info(account_info_iter)?; // signer
        let deposit = next_account_info(account_info_iter)?; // PDA

        if !withdrawer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut deposit_data = TimeLockDeposit::try_from_slice(&deposit.data.borrow())?;

        let clock = Clock::get()?;
        if clock.unix_timestamp < deposit_data.unlock_time {
            return Err(TimeLockError::StillLocked.into());
        }

        if deposit_data.is_withdrawn {
            return Err(TimeLockError::AlreadyWithdrawn.into());
        }

        if *withdrawer.key != deposit_data.sender && *withdrawer.key != deposit_data.recipient {
            return Err(TimeLockError::UnauthorizedWithdrawal.into());
        }

        let rent = Rent::get()?;
        let rent_exempt = rent.minimum_balance(TIME_LOCK_DEPOSIT_LEN);
        let escrow_balance = **deposit.lamports.borrow();
        let withdrawable_raw = escrow_balance.checked_sub(rent_exempt).ok_or(TimeLockError::InsufficientFunds)?;
        let withdraw_amount = withdrawable_raw.min(deposit_data.amount);
        if withdraw_amount == 0 {
            return Err(TimeLockError::InsufficientFunds.into());
        }

        **deposit.try_borrow_mut_lamports()? -= withdraw_amount;
        **withdrawer.try_borrow_mut_lamports()? += withdraw_amount;

        deposit_data.is_withdrawn = true;
        deposit_data.withdrawn_at = Some(clock.unix_timestamp);
        deposit_data.withdrawn_by = Some(*withdrawer.key);
        deposit_data.serialize(&mut &mut deposit.data.borrow_mut()[..])?;

        msg!("Escrow withdrawn");
        Ok(())
    }

    fn process_close(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let closer = next_account_info(account_info_iter)?; // signer
        let rent_receiver = next_account_info(account_info_iter)?;
        let deposit = next_account_info(account_info_iter)?;

        if !closer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let mut deposit_data = TimeLockDeposit::try_from_slice(&deposit.data.borrow())?;

        if !deposit_data.is_withdrawn {
            return Err(TimeLockError::CannotCloseActiveDeposit.into());
        }

        const CLOSE_GRACE_PERIOD: i64 = 365 * 24 * 60 * 60;
        let clock = Clock::get()?;
        if clock.unix_timestamp < deposit_data.unlock_time + CLOSE_GRACE_PERIOD {
            return Err(TimeLockError::TooEarlyToClose.into());
        }

        **rent_receiver.try_borrow_mut_lamports()? += **deposit.lamports.borrow();
        **deposit.try_borrow_mut_lamports()? = 0;
        deposit_data.serialize(&mut &mut deposit.data.borrow_mut()[..])?;

        Ok(())
    }
}
