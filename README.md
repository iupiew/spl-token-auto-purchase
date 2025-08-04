# SPL Token Auto Purchase

## Project Overview

The SPL Token Auto Purchase program is a Solana smart contract that allows users to automatically purchase SPL tokens on the Raydium V4 DEX. The program is designed to be simple, efficient, and secure, and it can be easily integrated into other Solana projects.

## Features

*   **Automatic token purchases:** The program can be used to automatically purchase SPL tokens on Raydium V4.
*   **Slippage protection:** The program includes slippage protection to prevent users from losing money due to price fluctuations.
*   **Support for any SPL token:** The program can be used to purchase any SPL token that is listed on Raydium V4.
*   **Easy to use:** The program is easy to use and can be integrated into other Solana projects with just a few lines of code.

## Getting Started

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Solana CLI](https://docs.solana.com/cli/install)
*   [Anchor](https://project-serum.github.io/anchor/getting-started/installation.html)

### Installation

1.  Clone the repository:

```bash
git clone https://github.com/your-username/solana-auto-token-buyer.git
```

2.  Change into the project directory:

```bash
cd solana-auto-token-buyer
```

3.  Install the dependencies:

```bash
npm install
```

## Building

To build the project, run the following command:

```bash
build.sh
```

## Testing

To test the project, run the following command:

```bash
cargo test
```

## Deployment

### Devnet

To deploy the project to devnet, run the following command:

```bash
solana program deploy --url https://api.devnet.solana.com target/deploy/spl_token_auto_purchase.so
```

### Mainnet

To deploy the project to mainnet, run the following command:

```bash
solana program deploy target/deploy/spl_token_auto_purchase.so
```

## Usage

To use the program, you will need to create a new Solana account and fund it with SOL. You will also need to create a new token account for the token that you want to purchase.

Once you have created the necessary accounts, you can use the following code to purchase a token:

```rust
use solana_sdk::{
    pubkey::Pubkey,
    system_instruction,
    transaction::Transaction,
};
use spl_token_auto_purchase::{
    instruction::AutoBuyerInstruction,
    state::{
        constants::{RAYDIUM_V4_PROGRAM_ID, TOKEN_PROGRAM_ID},
        PoolConfig,
    },
};

fn main() {
    // Set up the necessary accounts
    let user_account = Pubkey::new_unique();
    let source_token_account = Pubkey::new_unique();
    let destination_token_account = Pubkey::new_unique();
    let target_mint = Pubkey::new_unique();
    let quote_mint = Pubkey::new_unique();
    let raydium_program = RAYDIUM_V4_PROGRAM_ID;
    let raydium_pool = Pubkey::new_unique();
    let raydium_pool_token_a_account = Pubkey::new_unique();
    let raydium_pool_token_b_account = Pubkey::new_unique();
    let token_program = TOKEN_PROGRAM_ID;
    let system_program = Pubkey::new_unique();

    // Create the instruction
    let instruction = AutoBuyerInstruction::BuyToken {
        amount_in: 1000000,
        min_amount_out: 900000,
    };

    // Create the transaction
    let mut transaction = Transaction::new_with_payer(
        &[instruction],
        Some(&user_account),
    );

    // Sign the transaction
    transaction.sign(&[&user_account], recent_blockhash);

    // Send the transaction
    let result = rpc_client.send_and_confirm_transaction(&transaction);

    // Check the result
    assert!(result.is_ok());
}
```

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.
