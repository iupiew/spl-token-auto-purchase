#![allow(unexpected_cfgs)]

use borsh::BorshDeserialize;
use pinocchio::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

mod dex;
mod error;
pub mod instruction;
pub mod processor;
pub mod state;

use processor::Processor;

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
