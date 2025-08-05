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
#   sh ops/maker.sh <config_name>    # Start market maker with specified config
#   Example: sh ops/maker.sh base.eth-usdc
# 
# @requirements:
#   - Rust and Cargo must be installed
#   - Valid TOML configuration file must be specified as argument
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
# @param: $1 - Config name (e.g., "base.eth-usdc")
# @return: None
# =============================================================================
function start() {
    # Check if config argument is provided
    if [ -z "$1" ]; then
        echo -e "${RED}Error: Config argument required${NC}"
        echo "Usage: sh ops/maker.sh <config_name>"
        echo "Example: sh ops/maker.sh base.eth-usdc"
        exit 1
    fi

    # Set config path
    CONFIG_PATH="config/$1.toml"
    SECRET_PATH="config/secrets/.env.$1"
    
    # Check if config file exists
    if [ ! -f "$CONFIG_PATH" ]; then
        echo -e "${RED}Error: Config file not found: $CONFIG_PATH${NC}"
        echo "Available configs:"
        ls config/*.toml | sed 's|config/||' | sed 's|.toml||'
        exit 1
    fi

    # Set up signal handlers for graceful shutdown
    trap cleanup SIGINT SIGTERM

    # Build and run the market maker
    echo "Building MarketMaker program (might take a few minutes the first time) ..."
    cargo build --bin maker
    if [ $? -ne 0 ]; then
        echo -e "${RED}Build failed${NC}"
        exit 1
    fi
    echo "Build successful. Executing with config: $CONFIG_PATH"

    # Set logging level for production run
    export RUST_LOG="off,maker=trace,shd=trace"
    export CONFIG_PATH="$CONFIG_PATH"
    export SECRET_PATH="$SECRET_PATH"
    cargo run --bin maker

    echo "Program has finished or was interrupted."
}

# =============================================================================
# Script Execution
# =============================================================================
start $1
