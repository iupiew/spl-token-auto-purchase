use crate::error::AutoBuyerError;
use pinocchio::program_error::ProgramError;

/// Program instruction discriminators
const INSTRUCTION_BUY_TOKEN: u8 = 0;

/// Instructions for automatic token buyer
#[derive(Debug, Clone, Copy)]
pub enum AutoBuyerInstruction {
    /// Buy token automatically
    ///
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[writable]` User quote token account (source)
    /// 2. `[writable]` User target token account (destination)
    /// 3. `[]` Target token mint
    /// 4. `[]` Quote token mint
    /// 5. `[]` Raydium program
    /// 6. `[writable]` Raydium pool
    /// 7. `[writable]` Raydium pool token A vault
    /// 8. `[writable]` Raydium pool token B vault
    /// 9. `[]` Token program
    /// 10. `[]` System program
    /// 11. `[]` AMM authority (PDA)
    /// 12. `[writable]` AMM open orders
    /// 13. `[writable]` AMM target orders
    /// 14. `[]` Serum program
    /// 15. `[writable]` Serum market
    /// 16. `[writable]` Serum bids
    /// 17. `[writable]` Serum asks
    /// 18. `[writable]` Serum event queue
    /// 19. `[writable]` Serum coin vault
    /// 20. `[writable]` Serum pc vault
    /// 21. `[]` Serum vault signer
    BuyToken {
        /// Amount in quote token to swap
        amount_in: u64,
        /// Minimum acceptable amount of output token
        min_amount_out: u64,
    },
}

impl AutoBuyerInstruction {
    /// Pack instruction into bytes (1 byte discriminator + 2 u64 LE = 17 bytes)
    pub fn pack(&self) -> [u8; 17] {
        match self {
            Self::BuyToken { amount_in, min_amount_out } => {
                let mut data = [0u8; 17];
                data[0] = INSTRUCTION_BUY_TOKEN;
                data[1..9].copy_from_slice(&amount_in.to_le_bytes());
                data[9..17].copy_from_slice(&min_amount_out.to_le_bytes());
                data
            }
        }
    }

    /// Unpack instruction from bytes
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() != 17 {
            return Err(AutoBuyerError::InvalidInstruction.into());
        }
        let discriminator = input[0];
        if discriminator != INSTRUCTION_BUY_TOKEN {
            return Err(AutoBuyerError::InvalidInstruction.into());
        }
        let amount_in = u64::from_le_bytes(input[1..9].try_into().unwrap());
        let min_amount_out = u64::from_le_bytes(input[9..17].try_into().unwrap());
        Ok(Self::BuyToken { amount_in, min_amount_out })
    }
}

/// Buy result data (for off-chain use only)
#[derive(Debug, Clone)]
pub struct BuyResult {
    /// Transaction success
    pub success: bool,
    /// Actual output token amount
    pub amount_out: u64,
    /// Fee paid
    pub fee_paid: u64,
    /// Execution timestamp
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AutoBuyerError;
    use pinocchio::program_error::ProgramError;

    #[test]
    fn test_instruction_pack_unpack_roundtrip() {
        let instruction = AutoBuyerInstruction::BuyToken {
            amount_in: 1_000_000_000,
            min_amount_out: 500_000,
        };
        let packed = instruction.pack();
        let unpacked = AutoBuyerInstruction::unpack(&packed).unwrap();
        
        match unpacked {
            AutoBuyerInstruction::BuyToken { amount_in, min_amount_out } => {
                assert_eq!(amount_in, 1_000_000_000);
                assert_eq!(min_amount_out, 500_000);
            }
        }
    }

    #[test]
    fn test_instruction_unpack_truncated_input() {
        let short_input = [0u8; 16];
        let result = AutoBuyerInstruction::unpack(&short_input);
        assert!(matches!(result, Err(ProgramError::Custom(code)) if code == AutoBuyerError::InvalidInstruction as u32));
    }

    #[test]
    fn test_instruction_unpack_wrong_discriminator() {
        let mut input = [0u8; 17];
        input[0] = 255; // Invalid discriminator
        let result = AutoBuyerInstruction::unpack(&input);
        assert!(matches!(result, Err(ProgramError::Custom(code)) if code == AutoBuyerError::InvalidInstruction as u32));
    }

    #[test]
    fn test_instruction_unpack_zero_amounts() {
        let input = [0u8; 17]; // discriminator 0, amount_in 0, min_amount_out 0
        let result = AutoBuyerInstruction::unpack(&input);
        assert!(result.is_ok());
        match result.unwrap() {
            AutoBuyerInstruction::BuyToken { amount_in, min_amount_out } => {
                assert_eq!(amount_in, 0);
                assert_eq!(min_amount_out, 0);
            }
        }
    }

    #[test]
    fn test_instruction_pack_format() {
        let instruction = AutoBuyerInstruction::BuyToken {
            amount_in: 0x123456789ABCDEF0,
            min_amount_out: 0xFEDCBA9876543210,
        };
        let packed = instruction.pack();
        
        // Check discriminator
        assert_eq!(packed[0], 0);
        
        // Check amount_in (little-endian)
        assert_eq!(u64::from_le_bytes(packed[1..9].try_into().unwrap()), 0x123456789ABCDEF0);
        
        // Check min_amount_out (little-endian)
        assert_eq!(u64::from_le_bytes(packed[9..17].try_into().unwrap()), 0xFEDCBA9876543210);
    }
}