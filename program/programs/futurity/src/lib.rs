use solana_program::entrypoint;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::account_info::AccountInfo;

mod error;
mod instruction;
mod processor;
mod state;

pub use error::TimeLockError;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::Processor::process(program_id, accounts, instruction_data)
}
