use solana_program::entrypoint;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::account_info::AccountInfo;

mod error;
mod instruction;
mod processor;
mod state;

pub use error::TimeLockError;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

security_txt! {
    name: "Futurity DeFi",
    project_url: "https://futurity.fi",
    contacts: "email:security@futurity.fi,github:FuturityDeFi",
    policy: "https://futurity.fi/privacy",
    preferred_languages: "en",
    source_code: "https://github.com/Futurity-DeFi/smart-contract-solana"
}

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    solana_program::msg!("Futurity contract invoked.");
    processor::Processor::process(program_id, accounts, instruction_data)
}

