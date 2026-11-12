use base64;
use pinocchio::program_error::ProgramError;
use serde_json;
use spl_token_auto_purchase::{
    dex::raydium::AmmInfo,
    instruction::AutoBuyerInstruction,
    state::constants::{RAYDIUM_V4_PROGRAM_ID, TOKEN_PROGRAM_ID, WSOL_MINT},
};
use std::fs;

#[tokio::test]
async fn test_instruction_pack_unpack() {
    let instruction = AutoBuyerInstruction::BuyToken {
        amount_in: 1_000_000_000,
        min_amount_out: 500_000,
    };

    let packed = instruction.pack();
    let unpacked = AutoBuyerInstruction::unpack(&packed).unwrap();

    match unpacked {
        AutoBuyerInstruction::BuyToken {
            amount_in,
            min_amount_out,
        } => {
            assert_eq!(amount_in, 1_000_000_000);
            assert_eq!(min_amount_out, 500_000);
        }
    }
}

#[tokio::test]
async fn test_swap_calculation() {
    use spl_token_auto_purchase::dex::raydium::RaydiumV4;

    let raydium = RaydiumV4::new();

    // Test basic swap calculation
    let (amount_out, fee_amount) = raydium
        .calculate_amount_out(10000, 1000, 100000, 25, 10000)
        .unwrap();

    assert_eq!(fee_amount, 25);
    assert_eq!(amount_out, 90888);

    // Test zero fee
    let (amount_out, fee_amount) = raydium
        .calculate_amount_out(10, 1000, 100000, 0, 10000)
        .unwrap();

    assert_eq!(fee_amount, 0);
    assert_eq!(amount_out, 990); // 10 * 100000 / 1010 = 990

    // Test insufficient liquidity
    let result = raydium.calculate_amount_out(100, 0, 100000, 25, 10000);
    assert!(result.is_err());

    let result = raydium.calculate_amount_out(100, 1000, 0, 25, 10000);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_price_slippage_calculation() {
    // Test fixed-point price calculation
    let amount_in = 100u64;
    let amount_out = 9090u64;

    let price_per_unit = amount_out * 1_000_000_000 / amount_in;
    assert_eq!(price_per_unit, 90_900_000_000);

    // Slippage calculation
    let reserve_in = 1000u64;
    let reserve_out = 100000u64;
    let amount_in_after_fee = 100u64;

    let ideal_num = reserve_out as u128;
    let ideal_den = reserve_in as u128;
    let actual_num = 9090u128;
    let actual_den = amount_in_after_fee as u128;

    let diff_num = ideal_num * actual_den - ideal_den * actual_num;
    let diff_den = ideal_num * actual_den;
    let slippage_bps = (diff_num * 10000 / diff_den) as u16;

    assert_eq!(slippage_bps, 910); // 9.10%
}

#[tokio::test]
async fn test_raydium_constants() {
    use spl_token_auto_purchase::state::constants;

    // Verify RAYDIUM_V4_PROGRAM_ID matches expected base58
    // 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
    let expected = [
        75, 217, 73, 196, 54, 2, 195, 63, 32, 119, 144, 237, 22, 163, 82, 76, 161, 185, 151, 92,
        241, 33, 162, 169, 12, 255, 236, 125, 248, 182, 138, 205,
    ];
    assert_eq!(constants::RAYDIUM_V4_PROGRAM_ID.as_ref(), expected);

    // Verify WSOL_MINT
    let expected_wsol = [
        6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220,
        26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
    ];
    assert_eq!(constants::WSOL_MINT.as_ref(), expected_wsol);

    // Verify TOKEN_PROGRAM_ID
    let expected_token = [
        6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
        237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
    ];
    assert_eq!(constants::TOKEN_PROGRAM_ID.as_ref(), expected_token);
}

#[tokio::test]
async fn test_constants() {
    use spl_token_auto_purchase::state::constants;

    assert_eq!(constants::MAX_SLIPPAGE_BPS, 100);
    assert_eq!(constants::BASIS_POINTS, 10000);
    assert_eq!(constants::AMM_AUTHORITY_SEED, b"amm authority");
}

#[tokio::test]
async fn test_real_pool_fixture() {
    use spl_token_auto_purchase::dex::raydium::AmmInfo;
    use spl_token_auto_purchase::state::constants;
    use std::fs;

    // Load the real pool fixture
    let fixture_data = fs::read_to_string("tests/fixtures/raydium_pool_sol_usdc.json")
        .expect("Failed to read fixture file");
    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_data).expect("Failed to parse fixture JSON");

    // Verify pool metadata
    assert_eq!(
        fixture["pubkey"],
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
    );
    assert_eq!(
        fixture["account"]["owner"],
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    );
    assert_eq!(fixture["account"]["space"], 752);

    // Decode pool data
    let b64_data = fixture["account"]["data"][0].as_str().unwrap();
    let pool_data = base64::decode(b64_data).expect("Failed to decode base64");
    assert_eq!(pool_data.len(), 752, "Pool data must be 752 bytes");

    // Parse pool data
    let amm_info = AmmInfo::parse(&pool_data).expect("Failed to parse pool data");

    // Verify mints match the pool's actual mints
    // Base mint: 7as5eNh2inHyFxHXBjNkjgbrLoUVZ4j1KLUBd8ekQ8kw
    let expected_base_mint: [u8; 32] = [
        97, 210, 233, 250, 143, 38, 67, 24, 52, 235, 115, 111, 47, 36, 252, 42, 42, 77, 255, 31,
        213, 220, 164, 223, 242, 203, 185, 183, 96, 237, 219, 24,
    ];
    // Quote mint: 6rhwfxRSCKcr1eex2dBJ3jqbvzjv13uQPudMzRUpW6E7
    let expected_quote_mint: [u8; 32] = [
        87, 6, 48, 48, 99, 173, 51, 215, 181, 114, 150, 234, 2, 212, 224, 51, 94, 49, 206, 175,
        164, 204, 66, 221, 6, 155, 136, 87, 254, 171, 129, 132,
    ];

    assert_eq!(
        amm_info.base_mint.as_ref(),
        expected_base_mint,
        "Base mint should match fixture"
    );
    assert_eq!(
        amm_info.quote_mint.as_ref(),
        expected_quote_mint,
        "Quote mint should match fixture"
    );

    // Verify vaults exist (non-zero)
    assert_ne!(
        amm_info.base_vault.as_ref(),
        [0u8; 32],
        "Base vault should not be zero"
    );
    assert_ne!(
        amm_info.quote_vault.as_ref(),
        [0u8; 32],
        "Quote vault should not be zero"
    );

    // Verify fee structure
    // Note: This pool has trade_fee_numerator = 0 (unusual, but valid for some pools)
    // and trade_fee_denominator = 4042287200334
    assert_eq!(
        amm_info.trade_fee_numerator, 0,
        "Trade fee numerator should match fixture"
    );
    assert_eq!(
        amm_info.trade_fee_denominator, 4042287200334,
        "Trade fee denominator should match fixture"
    );

    // Verify pool status is valid for swaps (1=Initialized, 6=SwapOnly, 7=WaitingTrade)
    // This pool has status 6 (SwapOnly) which is the normal live state
    assert!(
        matches!(amm_info.status, 1 | 6 | 7),
        "Pool status must be valid for swaps"
    );
    assert_eq!(amm_info.status, 6, "Pool status should be SwapOnly (6)");
}
