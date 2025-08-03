// lib.rs - Fixed imports and entry point
use pinocchio::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

mod dex;
mod error;
mod instruction;
mod processor;
mod state;

use processor::Processor;

// Entry point - fixed for pinocchio
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = instruction::AutoBuyerInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    Processor::process(program_id, accounts, instruction)
}
