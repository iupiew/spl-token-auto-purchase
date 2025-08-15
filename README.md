# SPL Token Auto Purchase

## Project Overview

The SPL Token Auto Purchase program is a Solana smart-contract build that allows users to automatically purchase SPL tokens on the Raydium V4 DEX. The program is designed to be simple, efficient, and secure, and it can be easily integrated into other Solana projects.

## Getting Started

### Prerequisites

*   cargo 1.88.0
*   solana-cli 2.2.21

### Installation

1.  Clone the repository:

```bash
git clone https://github.com/iupiew/spl-token-auto-purchase.git
```

2.  Change into the project directory:

```bash
cd spl-token-auto-purchase
```

## Building

To build the project, run the following command:

```bash
cargo build-sbf
```

## Testing

To test the project, run the following command:

```bash
cargo test --test integration_test -- --test-threads=1 --nocapture
```

## Deployment

### Devnet

To deploy the project to devnet, run the following command:

```bash
solana config set --url devnet && solana program deploy target/deploy/spl_token_auto_purchase.so
```

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.
