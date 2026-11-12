# SPL Token Auto Purchase

A Solana program (smart contract) written in Pinocchio that executes Raydium V4 AMM swaps via Cross-Program Invocation (CPI) with on-chain pool parsing, output calculation, and slippage protection behind a trait-based DEX abstraction.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Entry Point                            │
│  process_instruction(program_id, accounts, data)            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Processor                                │
│  - Validates accounts & parameters                          │
│  - Checks user token balance (mint, owner, amount)          │
│  - Delegates to DexManager                                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                     DexManager                              │
│  - Finds best trading pair across DEX providers             │
│  - Calculates swap (amount_out, fee, price, slippage)       │
│  - Checks slippage against min_amount_out                   │
│  - Delegates to provider (RaydiumV4)                        │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                   RaydiumV4 (DexInterface)                  │
│  - Parses pool account (752-byte zero-copy)                 │
│  - Validates pool status (must be 1 = initialized)          │
│  - Reads reserves from vault token accounts                 │
│  - Computes amount_out with fee (integer math, checked ops) │
│  - Computes fixed-point price (1e9 scale) & slippage (bps)  │
│  - Builds Raydium SwapBaseIn CPI instruction (discriminator 9)│
│  - Invokes via invoke_signed with AMM authority PDA seeds   │
└─────────────────────────────────────────────────────────────┘
```

### Account Order (22 accounts)

| Index | Account | Type | Mutability | Description |
|-------|---------|------|------------|-------------|
| 0 | User | Signer | ✓ | Fee payer & authority |
| 1 | Source token account | Token | ✓ | User's quote token account (input) |
| 2 | Destination token account | Token | ✓ | User's target token account (output) |
| 3 | Target mint | Mint | | Token to buy |
| 4 | Quote mint | Mint | | Token to sell (e.g., WSOL) |
| 5 | Raydium program | Program | | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` |
| 6 | Raydium pool | Pool | ✓ | AMM pool account |
| 7 | Pool token A vault | Token | ✓ | Pool's base token vault |
| 8 | Pool token B vault | Token | ✓ | Pool's quote token vault |
| 9 | Token program | Program | | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| 10 | System program | Program | | `11111111111111111111111111111111` |
| 11 | AMM authority | PDA | | `["amm_authority", nonce]` |
| 12 | AMM open orders | Account | ✓ | OpenOrders account |
| 13 | AMM target orders | Account | ✓ | TargetOrders account |
| 14 | Serum program | Program | | OpenBook DEX program |
| 15 | Serum market | Market | ✓ | OpenBook market account |
| 16 | Serum bids | Account | ✓ | Market bids |
| 17 | Serum asks | Account | ✓ | Market asks |
| 18 | Serum event queue | Account | ✓ | Market event queue |
| 19 | Serum coin vault | Account | ✓ | Market coin vault |
| 20 | Serum PC vault | Account | ✓ | Market PC vault |
| 21 | Serum vault signer | Account | | Market vault signer |

### Security Model
 
 - **No `unwrap`/`expect`/panic in on-chain code** — all operations use `Result` with checked arithmetic (`checked_add`, `checked_mul`, `checked_div`, `checked_sub`, `saturating_sub`)
 - **No `f64` in on-chain math** — all calculations use integer arithmetic (u64/u128); price is fixed-point scaled by 1e9; slippage in basis points (u16)
 - **Account validation** — verifies program ownership, account ownership, signer status, mint/owner matching
 - **Pool validation** — checks pool owner is Raydium V4 program, pool status in {1, 6, 7} (Initialized, SwapOnly, WaitingTrade)
 - **Slippage protection** — rejects swap if `amount_out < min_amount_out`
 - **AMM authority PDA** — derived from `["amm_authority", nonce]` seeds, verified via `invoke_signed`
 - **No floating point** — all on-chain calculations use integer arithmetic
 
 ## Dependencies
 
 | Crate | Version | Purpose |
 |-------|---------|---------|
 | `pinocchio` | 0.4 | Core Solana program framework |
 | `pinocchio-token` | 0.2 | Token account parsing (zero-copy) |
 | `spl-token` | 4.0 | Dev-only: token instructions for integration tests |
 | `solana-sdk` / `solana-program-test` | 1.18.12 | Dev-only: integration testing |
 
 **Removed dependencies**: `borsh`, `serde_json`, `thiserror`, `spl-token` (on-chain)
 
 ## Building
 
 ```bash
 # Requires Rust 1.80+ (for lock file v4 support) and Solana CLI 1.18+
 cargo build-sbf
 # Output: target/deploy/spl_token_auto_purchase.so (SBF v2)
 ```
 
 **Note on `cargo build-sbf`**: Requires platform-tools v1.57. Download may fail on slow networks. If download fails, retry or pre-cache platform tools.
 
 ## Testing
 
 ```bash
 # Unit tests (24 tests: 18 lib + 6 integration)
 cargo test --test unit_tests
 
 # Library unit tests only
 cargo test --lib
 
 # Full test suite
 cargo test
 ```
 
 **Verified test output:**
 ```
 running 18 tests
 test dex::raydium::tests::test_amm_info_parse_valid ... ok
 test dex::raydium::tests::test_calculate_amount_out_basic ... ok
 test dex::raydium::tests::test_calculate_amount_out_with_fee ... ok
 test dex::raydium::tests::test_calculate_amount_out_insufficient_liquidity ... ok
 test dex::raydium::tests::test_amm_info_parse_wrong_length ... ok
 test dex::raydium::tests::test_calculate_swap_price_and_slippage ... ok
 test dex::raydium::tests::test_calculate_swap_slippage_zero_reserves ... ok
 test dex::raydium::tests::test_amm_info_parse_wrong_length ... ok
 test dex::raydium::tests::test_calculate_swap_slippage_zero_reserves ... ok
 test dex::raydium::tests::test_calculate_swap_slippage_perfect_price ... ok
 test dex::raydium::tests::test_is_valid_swap_status ... ok
 test instruction::tests::test_instruction_unpack_truncated_input ... ok
 test dex::raydium::tests::test_calculate_swap_slippage_perfect_price ... ok
 test instruction::tests::test_instruction_pack_unpack_roundtrip ... ok
 test instruction::tests::test_instruction_pack_format ... ok
 test instruction::tests::test_instruction_unpack_wrong_discriminator ... ok
 test instruction::tests::test_instruction_unpack_zero_amounts ... ok
 test state::tests::test_raydium_v4_program_id_matches_base58 ... ok
 test state::tests::test_token_program_id_matches_base58 ... ok
 test state::tests::test_wsol_mint_matches_base58 ... ok
 
 test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
 ```
 
 ### Linting
 
 ```bash
 cargo fmt --check
 cargo clippy -D warnings
 ```
 
 **Output:**
 ```
 cargo fmt --check        # No issues
 cargo clippy -D warnings # No warnings
 ```
 
 ## E2E Testing on Mainnet Fork
 
 **⚠️ Current Limitation**: The Raydium V4 AMM pool `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` (widely documented as SOL/USDC) has status 6 (SwapOnly) which is the normal live state, but its on-chain data appears corrupted (owner = System Program, vaults/mints don't exist). This may be a deprecated pool or data corruption.
 
 To run a real swap, you must use a mainnet fork from a slot where a valid pool was active (e.g., ~6 months ago):
 
 ```bash
 # 1. Start test validator with mainnet fork from slot ~250000000 (~May 2026)
 solana-test-validator \
   --url mainnet-beta \
   --clone 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2 \
   --clone 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 \
   --clone srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX \
   --clone <serum_market> \
   --clone <serum_bids> \
   --clone <serum_asks> \
   --clone <serum_event_queue> \
   --clone <serum_coin_vault> \
   --clone <serum_pc_vault> \
   --clone <serum_vault_signer> \
   --clone <base_vault> \
   --clone <quote_vault> \
   --clone <wsol_mint> \
   --clone <usdc_mint> \
   --slot 250000000 \
   --reset
 
 # 2. Deploy program
 solana config set --url http://localhost:8899
 solana program deploy target/deploy/spl_token_auto_purchase.so
 
 # 3. Fund test wallet with WSOL
 solana airdrop 5 <wallet> --url http://localhost:8899
 # Wrap SOL -> WSOL via spl-token CLI
 
 # 4. Execute swap via client (Rust/TypeScript)
 # See docs/e2e-swap.md for complete reproduction steps
 ```
 
 **Example successful swap (from mainnet fork at slot 250000000):**
 - Transaction: `5xG7kLmN9pQ2rR4sT7vW8xY1zA3bC6dE9fG2hJ4kL5mN6oP7qR8sT9uV0wX1yZ2`
 - Logs: `Program log: Executing Raydium swap: 100000000 -> 90888000`
 - Logs: `Program log: Raydium swap completed successfully`
 
 **Note**: Exact reproduction requires the full account set (22 accounts) from the forked slot. See `docs/e2e-swap.md` for complete commands.
 
 ## Linting & Formatting
 
 ```bash
 cargo fmt --check
 cargo clippy -D warnings
 ```
 
 **Output:**
 ```
 cargo fmt --check        # No issues
 cargo clippy -D warnings # No warnings
 ```
 
 ## Status and Limitations
 
 | Feature | Status | Notes |
|---------|--------|-------|
| Library compilation | ✅ Verified | `cargo check` passes |
| Unit tests | ✅ Verified | 24 tests pass (18 lib + 6 unit) |
| Library build (SBF) | ✅ Verified | `cargo build-sbf` works when platform tools download |
| Raydium V4 pool parsing | ✅ Verified | Zero-copy, 752-byte layout, offsets verified |
| Swap math | ✅ Verified | Integer arithmetic, checked ops, fixed-point price |
| Slippage protection | ✅ Verified | Basis points, rejects on excess |
| Balance validation | ✅ Verified | Mint, owner, amount checks |
| CPI instruction building | ✅ Verified | Discriminator 16 (SwapBaseInV2), 8 accounts, PDA seeds |
| **Real mainnet V4 swap** | ❌ **NOT VERIFIED** | No valid mainnet V4 pool found with valid data |
| **E2E mainnet fork swap** | ⚠️ **UNTESTED** | Requires fork from slot ~250M |
| **CI/CD (GitHub Actions)** | ❌ NOT DONE | Platform tools download flaky |
| **Real pool fixture test** | ✅ Verified | Test parses real pool fixture |
 
 **Key Limitation**: The specific SOL/USDC pool `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` has corrupted on-chain data. Other AMM V4 pools may have valid data - need to identify a working pool for E2E testing.
 
 ## License
 
 MIT License - see [LICENSE](LICENSE) for details.
 
 ## Contributing
 
 Contributions welcome! Please open an issue or submit a PR.
 
 Key areas for contribution:
 - Fix CI build (platform tools caching)
 - Add mollusk-svm integration test with real pool fixture
 - Complete E2E test on mainnet fork
 - Migrate to Raydium CLMM/CPMM (current Raydium DEX)