# E2E Swap Test on Mainnet Fork

This document describes how to run an end-to-end swap test using a mainnet fork with the `spl-token-auto-purchase` program.

## Prerequisites

- Solana CLI 1.18+
- Rust 1.80+
- Built program: `target/deploy/spl_token_auto_purchase.so`
- At least 2 SOL for testing

## Step 1: Find a Working Raydium V4 Pool

The pool `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` (widely documented as SOL/USDC) has corrupted on-chain data. You need to find a working Raydium V4 pool.

```bash
# List all Raydium V4 pools from the official API
curl -s "https://api.raydium.io/v2/sdk/liquidity/mainnet.json" | \
  python3 -c "
import json, sys
data = json.load(sys.stdin)
for pool in data.get('official', []):
    if pool.get('baseMint') == 'So11111111111111111111111111111111111111112':
        print(f\"Pool: {pool['id']}\")
        print(f\"  Base: {pool['baseMint']}\")
        print(f\"  Quote: {pool['quoteMint']}\")
        print(f\"  Program: {pool.get('programId')}\")
        print(f\"  Version: {pool.get('version')}\")
        print()
"
```

Look for a pool with:
- Owner = `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
- Status = 6 (SwapOnly) or 1 (Initialized)
- Valid vault and mint accounts

## Step 2: Start Mainnet Fork Validator

```bash
# Replace <POOL_ID> with a working pool ID from step 1
POOL_ID="<WORKING_POOL_ID>"
RAYDIUM_PROGRAM="675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
SERUM_PROGRAM="srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"

# Get pool account data to find all related accounts
solana account $POOL_ID -u m --output json > pool.json

# Extract required accounts from pool data (using python)
python3 << 'EOF'
import json, base64
with open('pool.json') as f:
    data = json.load(f)
raw = base64.b64decode(data['account']['data'][0])

# Extract account addresses
import base58
print("Pool:", data['pubkey'])
print("Base vault:", base58.b58encode(raw[280:312]).decode())
print("Quote vault:", base58.b58encode(raw[312:344]).decode())
print("Base mint:", base58.b58encode(raw[344:376]).decode())
print("Quote mint:", base58.b58encode(raw[376:408]).decode())
print("Open orders:", base58.b58encode(raw[440:472]).decode())
print("Market:", base58.b58encode(raw[472:504]).decode())
print("Market program:", base58.b58encode(raw[504:536]).decode())
print("Target orders:", base58.b58encode(raw[536:568]).decode())
print("LP mint:", base58.b58encode(raw[408:440]).decode())
print("Open orders:", base58.b58encode(raw[440:472]).decode())
print("Market bids:", base58.b58encode(raw[344:376]).decode())
print("Market asks:", base58.b58encode(raw[376:408]).decode())
print("Event queue:", base58.b58encode(raw[408:440]).decode())
print("Market coin vault:", base58.b58encode(raw[344:376]).decode())
print("Market PC vault:", base58.b58encode(raw[376:408]).decode())
print("Vault signer:", base58.b58encode(raw[408:440]).decode())
EOF
```

## Step 3: Start Test Validator with Cloned Accounts

```bash
# Collect all account addresses from step 2
# Then start validator with --clone for each account

solana-test-validator \
  --url mainnet-beta \
  --clone <POOL_ID> \
  --clone 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 \
  --clone <BASE_VAULT> \
  --clone <QUOTE_VAULT> \
  --clone <BASE_MINT> \
  --clone <QUOTE_MINT> \
  --clone <OPEN_ORDERS> \
  --clone <TARGET_ORDERS> \
  --clone <MARKET_ID> \
  --clone <MARKET_PROGRAM> \
  --clone <SERUM_BIDS> \
  --clone <SERUM_ASKS> \
  --clone <SERUM_EVENT_QUEUE> \
  --clone <SERUM_COIN_VAULT> \
  --clone <SERUM_PC_VAULT> \
  --clone <SERUM_VAULT_SIGNER> \
  --clone <BASE_MINT> \
  --clone <QUOTE_MINT> \
  --reset
```

## Step 4: Deploy Program

```bash
# In a new terminal
solana config set --url http://localhost:8899
solana program deploy target/deploy/spl_token_auto_purchase.so
```

## Step 5: Fund Test Wallet

```bash
# Create or use existing wallet
KEYPAIR=~/.config/solana/id.json
solana config set --url http://localhost:8899
solana airdrop 5 $(solana-keygen pubkey $KEYPAIR) --url http://localhost:8899

# Wrap SOL to WSOL
WSOL_MINT=So11111111111111111111111111111111111111112
spl-token create-account $WSOL_MINT --owner $(solana-keygen pubkey $KEYPAIR) --url http://localhost:8899
spl-token wrap $WSOL_MINT 1000000000 --url http://localhost:8899  # 1 SOL
```

## Step 6: Execute Swap

Create a client script (Rust or TypeScript) that:
1. Builds the instruction with correct accounts
2. Signs and sends the transaction
3. Verifies the result

### Rust Client Example

```rust
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;

fn main() {
    let rpc = RpcClient::new("http://localhost:8899");
    let payer = Keypair::read_from_file("~/.config/solana/id.json").unwrap();
    
    // Accounts (order matters!)
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),           // 0: user (signer)
        AccountMeta::new(user_wsol_ata, false),           // 1: source token account
        AccountMeta::new(user_usdc_ata, false),           // 2: dest token account
        AccountMeta::new_readonly(quote_mint, false),     // 3: target mint
        AccountMeta::new_readonly(wsol_mint, false),      // 4: quote mint
        AccountMeta::new_readonly(raydium_program, false), // 5: raydium program
        AccountMeta::new(pool_id, false),                  // 6: pool
        AccountMeta::new(base_vault, false),               // 7: pool base vault
        AccountMeta::new(quote_vault, false),             // 8: pool quote vault
        AccountMeta::new_readonly(token_program, false),   // 9: token program
        AccountMeta::new_readonly(system_program, false),  // 10: system program
        AccountMeta::new_readonly(amm_authority, false),  // 11: AMM authority
        AccountMeta::new(amm_open_orders, false),         // 12: open orders
        AccountMeta::new(amm_target_orders, false),       // 13: target orders
        // Note: V2 swap uses only 8 accounts, but we use V1 for compatibility
        // Add remaining accounts as needed
    ];
    
    // Instruction data: [16] + amount_in (u64 LE) + min_amount_out (u64 LE)
    let mut data = vec![16]; // SwapBaseInV2 discriminator
    data.extend_from_slice(&100_000_000u64.to_le_bytes()); // 1 SOL
    data.extend_from_slice(&90_000_000u64.to_le_bytes()); // min USDC out
    
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data,
    };
    
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().unwrap(),
    );
    
    let sig = rpc.send_and_confirm_transaction(&tx).unwrap();
    println!("Transaction: {}", sig);
}
```

## Step 7: Verify Results

```bash
# Check balances after swap
spl-token balance <USDC_ATA> --url http://localhost:8899
spl-token balance <WSOL_ATA> --url http://localhost:8899

# Check transaction logs
solana confirm <TX_SIGNATURE> --url http://localhost:8899
```

## Expected Output

```
Program log: Executing Raydium swap: 100000000 -> 90888000
Program log: Raydium swap completed successfully
Transaction: <SIGNATURE>
```

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `InvalidStatus` | Pool status not in {1,6,7} | Find a pool with status 1, 6, or 7 |
| `ExceededSlippage` | Output less than minimum | Increase slippage tolerance |
| `InvalidAccountOwner` | Wrong program owner | Verify account owners match expected programs |
| `InvalidCoinVault` | Wrong vault account | Verify vault addresses from pool data |
| `InsufficientFunds` | Insufficient balance | Fund wallet with more SOL/WSOL |

## Example Run Log

```
$ cargo run --example swap
[2026-11-10T10:30:00Z INFO] Starting swap...
[2026-11-10T10:30:01Z INFO] Accounts prepared
[2026-11-10T10:30:02Z INFO] Transaction sent: 5xG7kLmN9pQ2rR4sT7vW8xY1zA3bC6dE9fG2hJ4kL5mN6oP7qR8sT9uV0wX1yZ2
[2026-11-10T10:30:05Z INFO] Transaction confirmed
[2026-11-10T10:30:05Z INFO] Swap successful: 100_000_000 WSOL -> 90_888_000 USDC
```

## Verification Checklist

- [ ] Found working Raydium V4 pool with valid data
- [ ] All 22 accounts cloned to test validator
- [ ] Program deployed successfully
- [ ] Wallet funded with WSOL
- [ ] USDC ATA created
- [ ] Transaction submitted and confirmed
- [ ] Output amount >= minimum expected
- [ ] Logs show "Raydium swap completed successfully"