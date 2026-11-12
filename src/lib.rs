#![allow(unexpected_cfgs)]

use pinocchio::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey};

pub mod dex;
mod error;
pub mod instruction;
pub mod processor;
pub mod state;

use processor::Processor;

entrypoint!(process_instruction);

// Program ID is defined in state::constants
pub const ID: Pubkey = state::constants::RAYDIUM_V4_PROGRAM_ID; // Placeholder, actual program ID will be deployed

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = instruction::AutoBuyerInstruction::unpack(instruction_data)?;
    Processor::process(program_id, accounts, instruction)
}
