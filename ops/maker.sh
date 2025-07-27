#!/bin/bash

# Color codes for terminal output
RED='\033[0;31m'
NC='\033[0m'

# =============================================================================
# Tycho Market Maker Startup Script
# =============================================================================
# 
# @description: Main entry point for starting the Tycho Market Maker bot
# @usage: 
#   sh ops/maker.sh          # Start market maker in normal mode
#   sh ops/maker.sh test     # Run tests with verbose output
# 
# @requirements:
#   - Rust and Cargo must be installed
#   - Valid TOML configuration file must be specified in CONFIG_PATH
#   - Environment variables must be properly configured
# 
# @config_files:
#   - config/base.eth-usdc.toml      # Base L2 ETH/USDC pair
#   - config/mainnet.eth-usdc.toml   # Mainnet ETH/USDC pair  
#   - config/mainnet.usdc-dai.toml   # Mainnet USDC/DAI pair
#   - config/mainnet.eth-wbtc.toml   # Mainnet ETH/WBTC pair
#   - config/unichain.eth-usdc.toml  # Unichain ETH/USDC pair
# =============================================================================

# =============================================================================
# @function: cleanup
# @description: Signal handler for graceful shutdown
# @param: None
# @return: None
# =============================================================================
cleanup() {
    echo -e "\n${RED}Shutting down market maker...${NC}"
    exit 0
}

# =============================================================================
# @function: start
# @description: Main startup function that builds and runs the market maker
# @param: $1 - Optional "test" argument to run tests instead of the main program
# @return: None
# =============================================================================
function start() {
    # Set up signal handlers for graceful shutdown
    trap cleanup SIGINT SIGTERM

    if [ "$1" = "test" ]; then
        # Test mode: Run cargo tests with verbose output
        export RUST_LOG="off,maker=trace,shd=trace,test=trace"
        cargo test -- --nocapture
    else
        # Production mode: Build and run the market maker
        echo "Building MarketMaker program (might take a few minutes the first time) ..."
        cargo build --bin maker
        if [ $? -ne 0 ]; then
            echo -e "${RED}Build failed${NC}"
            exit 1
        fi
        echo "Build successful. Executing..."

        # Set logging level for production run
        export RUST_LOG="off,maker=trace,shd=trace"
        cargo run --bin maker

        echo "Program has finished or was interrupted."
    fi
}

# =============================================================================
# Configuration Selection
# =============================================================================
# Uncomment the desired configuration file for your target network/pair:

# Mainnet configurations (production)
# export CONFIG_PATH="config/mainnet.eth-usdc.toml"
# export CONFIG_PATH="config/mainnet.usdc-dai.toml"
# export CONFIG_PATH="config/mainnet.eth-wbtc.toml"

# L2 configurations (testing/development)
# export CONFIG_PATH="config/base.eth-usdc.toml"
export CONFIG_PATH="config/unichain.eth-usdc.toml"

# =============================================================================
# Script Execution
# =============================================================================
start $1
