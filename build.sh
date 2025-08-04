#!/bin/bash
# build.sh - Build script for SPL Token Auto Purchase program

set -e # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROGRAM_NAME="spl-token-auto-purchase"
TARGET_DIR="target/deploy"
SO_FILE="${TARGET_DIR}/spl_token_auto_purchase.so"

echo -e "${BLUE}🔨 Building Solana Program: ${PROGRAM_NAME}${NC}"
echo "=================================="

# # Check if Rust is installed
# if ! command -v rustc &>/dev/null; then
# 	echo -e "${RED}❌ Rust is not installed. Please install Rust first.${NC}"
# 	echo "Visit: https://rustup.rs/"
# 	exit 1
# fi
#
# Check if Solana CLI is installed
# if ! command -v solana &>/dev/null; then
# 	echo -e "${RED}❌ Solana CLI is not installed. Please install Solana CLI first.${NC}"
# 	echo "Visit: https://docs.solana.com/cli/install-solana-cli-tools"
# 	exit 1
# fi
#
# # Check if we're in the right directory
# if [ ! -f "Cargo.toml" ]; then
# 	echo -e "${RED}❌ Cargo.toml not found. Please run this script from the project root.${NC}"
# 	exit 1
# fi

echo -e "${YELLOW}📋 Checking environment...${NC}"
echo "Rust version: $(rustc --version)"
echo "Solana version: $(solana --version)"
echo "Current network: $(solana config get | grep 'RPC URL' | awk '{print $3}')"

# Clean previous builds
echo -e "${YELLOW}🧹 Cleaning previous builds...${NC}"
cargo clean

# Build the program
echo -e "${YELLOW}⚙️  Building program in release mode...${NC}"
cargo build-sbf

# Check if build was successful
if [ ! -f "$SO_FILE" ]; then
	echo -e "${RED}❌ Build failed! Expected file not found: $SO_FILE${NC}"
	exit 1
fi

# Display build info
echo -e "${GREEN}✅ Build successful!${NC}"
echo "📁 Built file: $SO_FILE"
echo "📏 File size: $(du -h "$SO_FILE" | cut -f1)"

# Optional: Run tests
if [ "$1" = "--test" ] || [ "$1" = "-t" ]; then
	echo -e "${YELLOW}🧪 Running tests...${NC}"
	cargo test
fi

echo -e "${GREEN}🎉 Build completed successfully!${NC}"
echo "Ready to deploy with: ./deploy.sh"

---

#!/bin/bash
# deploy.sh - Deploy script for SPL Token Auto Purchase program

set -e # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
PROGRAM_NAME="spl-token-auto-purchase"
TARGET_DIR="target/deploy"
SO_FILE="${TARGET_DIR}/spl_token_auto_purchase.so"
KEYPAIR_FILE="target/deploy/spl_token_auto_purchase-keypair.json"

# Default network (can be overridden)
NETWORK=${1:-"devnet"}

echo -e "${BLUE}🚀 Deploying Solana Program: ${PROGRAM_NAME}${NC}"
echo "============================================="

# Validate network parameter
case $NETWORK in
"localnet" | "local")
	RPC_URL="http://localhost:8899"
	;;
"devnet" | "dev")
	RPC_URL="https://api.devnet.solana.com"
	;;
"testnet" | "test")
	RPC_URL="https://api.testnet.solana.com"
	;;
"mainnet" | "main")
	RPC_URL="https://api.mainnet-beta.solana.com"
	echo -e "${RED}⚠️  WARNING: You are deploying to MAINNET!${NC}"
	read -p "Are you sure? (yes/no): " confirm
	if [ "$confirm" != "yes" ]; then
		echo "Deployment cancelled."
		exit 0
	fi
	;;
*)
	echo -e "${RED}❌ Invalid network: $NETWORK${NC}"
	echo "Valid options: localnet, devnet, testnet, mainnet"
	exit 1
	;;
esac

# Check if build exists
if [ ! -f "$SO_FILE" ]; then
	echo -e "${RED}❌ Program binary not found: $SO_FILE${NC}"
	echo "Please run ./build.sh first"
	exit 1
fi

# Set Solana config
echo -e "${YELLOW}⚙️  Configuring Solana CLI...${NC}"
solana config set --url "$RPC_URL"

# Check wallet balance
echo -e "${YELLOW}💰 Checking wallet balance...${NC}"
BALANCE=$(solana balance --lamports)
BALANCE_SOL=$(echo "scale=9; $BALANCE / 1000000000" | bc -l)

echo "Wallet: $(solana address)"
echo "Balance: $BALANCE_SOL SOL ($BALANCE lamports)"

# Minimum balance check (approximately 2 SOL for deployment)
MIN_BALANCE=2000000000
if [ "$BALANCE" -lt "$MIN_BALANCE" ]; then
	echo -e "${RED}❌ Insufficient balance for deployment${NC}"
	echo "Required: ~2 SOL, Available: $BALANCE_SOL SOL"

	if [ "$NETWORK" = "devnet" ] || [ "$NETWORK" = "testnet" ]; then
		echo -e "${YELLOW}💡 You can request SOL from faucet:${NC}"
		echo "solana airdrop 2"
		read -p "Request airdrop now? (y/n): " airdrop
		if [ "$airdrop" = "y" ] || [ "$airdrop" = "Y" ]; then
			solana airdrop 2
			sleep 5 # Wait for airdrop
		fi
	fi
fi

# Generate program keypair if it doesn't exist
if [ ! -f "$KEYPAIR_FILE" ]; then
	echo -e "${YELLOW}🔑 Generating program keypair...${NC}"
	solana-keygen new --outfile "$KEYPAIR_FILE" --no-bip39-passphrase
fi

PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_FILE")
echo -e "${PURPLE}📋 Program ID: $PROGRAM_ID${NC}"

# Check if program is already deployed
if solana account "$PROGRAM_ID" &>/dev/null; then
	echo -e "${YELLOW}⚠️  Program already exists on $NETWORK${NC}"
	read -p "Upgrade existing program? (y/n): " upgrade
	if [ "$upgrade" = "y" ] || [ "$upgrade" = "Y" ]; then
		echo -e "${YELLOW}📤 Upgrading program...${NC}"
		solana program deploy "$SO_FILE" --program-id "$KEYPAIR_FILE"
	else
		echo "Deployment cancelled."
		exit 0
	fi
else
	echo -e "${YELLOW}📤 Deploying new program...${NC}"
	solana program deploy "$SO_FILE" --program-id "$KEYPAIR_FILE"
fi

# Verify deployment
echo -e "${YELLOW}✅ Verifying deployment...${NC}"
if solana account "$PROGRAM_ID" &>/dev/null; then
	echo -e "${GREEN}🎉 Deployment successful!${NC}"
	echo "================================"
	echo -e "${PURPLE}Program ID: $PROGRAM_ID${NC}"
	echo -e "${BLUE}Network: $NETWORK ($RPC_URL)${NC}"
	echo -e "${GREEN}Status: Deployed ✅${NC}"

	# Save program ID to file for easy reference
	echo "$PROGRAM_ID" >"program-id.txt"
	echo "Program ID saved to: program-id.txt"

	# Show account info
	echo ""
	echo -e "${YELLOW}📊 Program Account Info:${NC}"
	solana account "$PROGRAM_ID"

else
	echo -e "${RED}❌ Deployment verification failed${NC}"
	exit 1
fi

#!/bin/bash
# verify.sh - Verification script for deployed program

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

PROGRAM_ID_FILE="program-id.txt"

echo -e "${BLUE}🔍 Verifying Deployed Program${NC}"
echo "============================="

# Check if program ID file exists
if [ ! -f "$PROGRAM_ID_FILE" ]; then
	echo -e "${RED}❌ Program ID file not found: $PROGRAM_ID_FILE${NC}"
	echo "Please deploy the program first with ./deploy.sh"
	exit 1
fi

PROGRAM_ID=$(cat "$PROGRAM_ID_FILE")
echo -e "${YELLOW}Program ID: $PROGRAM_ID${NC}"

# Get current network
CURRENT_RPC=$(solana config get | grep 'RPC URL' | awk '{print $3}')
echo -e "${YELLOW}Network: $CURRENT_RPC${NC}"

# Check if program exists
echo -e "${YELLOW}🔍 Checking program account...${NC}"
if solana account "$PROGRAM_ID" &>/dev/null; then
	echo -e "${GREEN}✅ Program found on network${NC}"

	echo ""
	echo -e "${YELLOW}📊 Program Details:${NC}"
	solana account "$PROGRAM_ID"

	echo ""
	echo -e "${YELLOW}📋 Program Info:${NC}"
	solana program show "$PROGRAM_ID"

else
	echo -e "${RED}❌ Program not found on current network${NC}"
	exit 1
fi

---

#!/bin/bash
# test-deploy.sh - Test deployment script

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🧪 Testing Program Deployment${NC}"
echo "=============================="

# Build first
echo -e "${YELLOW}Step 1: Building program...${NC}"
./build.sh

# Deploy to devnet
echo -e "${YELLOW}Step 2: Deploying to devnet...${NC}"
./deploy.sh devnet

# Verify deployment
echo -e "${YELLOW}Step 3: Verifying deployment...${NC}"
./verify.sh

echo -e "${GREEN}🎉 Test deployment completed successfully!${NC}"

---

#!/bin/bash
# clean.sh - Clean build artifacts and deployment files

set -e

# Colors
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${YELLOW}🧹 Cleaning build artifacts...${NC}"

# Clean Cargo build artifacts
if [ -d "target" ]; then
	echo "Removing target directory..."
	rm -rf target
fi

# Clean program ID file
if [ -f "program-id.txt" ]; then
	echo "Removing program-id.txt..."
	rm -f program-id.txt
fi

# Clean any log files
if [ -f "deploy.log" ]; then
	echo "Removing deploy.log..."
	rm -f deploy.log
fi

echo -e "${GREEN}✅ Cleanup completed!${NC}"

---

# Makefile for easy command execution

.PHONY: build deploy clean test verify help

help:
@echo "Available commands:"
@echo "  make build    - Build the Solana program"
@echo "  make deploy   - Deploy to devnet"
@echo "  make test     - Build and run tests"
@echo "  make verify   - Verify deployed program"
@echo "  make clean    - Clean build artifacts"
@echo ""
@echo "Deploy to specific network:"
@echo "  make deploy-local    - Deploy to localnet"
@echo "  make deploy-devnet   - Deploy to devnet"
@echo "  make deploy-testnet  - Deploy to testnet"
@echo "  make deploy-mainnet  - Deploy to mainnet"

build:
@chmod +x build.sh && ./build.sh

deploy:
@chmod +x deploy.sh && ./deploy.sh devnet

deploy-local:
@chmod +x deploy.sh && ./deploy.sh localnet

deploy-devnet:
@chmod +x deploy.sh && ./deploy.sh devnet

deploy-testnet:
@chmod +x deploy.sh && ./deploy.sh testnet

deploy-mainnet:
@chmod +x deploy.sh && ./deploy.sh mainnet

test:
@chmod +x build.sh && ./build.sh --test

verify:
@chmod +x verify.sh && ./verify.sh

clean:
@chmod +x clean.sh && ./clean.sh

test-deploy:
@chmod +x test-deploy.sh && ./test-deploy.sh
