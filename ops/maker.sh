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
#   sh ops/maker.sh <config_name>              # Start with production config
#   sh ops/maker.sh --testing <config_name>    # Start with testing config
#   Example: sh ops/maker.sh base.eth-usdc
#            sh ops/maker.sh --testing mainnet.eth-usdc
# 
# @requirements:
#   - Rust and Cargo must be installed
#   - Valid TOML configuration file must be specified as argument
#   - Environment variables must be properly configured
# 
# @config_files:
#   Production (config/):
#   - config/base.eth-usdc.toml      # Base L2 ETH/USDC pair
#   - config/mainnet.eth-usdc.toml   # Mainnet ETH/USDC pair  
#   - config/mainnet.usdc-dai.toml   # Mainnet USDC/DAI pair
#   - config/mainnet.eth-wbtc.toml   # Mainnet ETH/WBTC pair
#   - config/unichain.eth-usdc.toml  # Unichain ETH/USDC pair
#   
#   Testing (config/testing/):
#   - config/testing/mainnet.eth-usdc.toml   # Testing config for mainnet
#   - config/testing/unichain.btc-usdc.toml  # Testing config for unichain BTC
#   - config/testing/unichain.eth-usdc.toml  # Testing config for unichain ETH
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
# @param: $1 - Config name (e.g., "base.eth-usdc") or "--testing" flag
# @param: $2 - Config name when using --testing flag
# @return: None
# =============================================================================
function start() {
    # Check for testing flag
    TESTING_MODE=false
    CONFIG_NAME=""
    
    if [ "$1" = "--testing" ]; then
        TESTING_MODE=true
        CONFIG_NAME="$2"
        if [ -z "$CONFIG_NAME" ]; then
            echo -e "${RED}Error: Config name required after --testing flag${NC}"
            echo "Usage: sh ops/maker.sh --testing <config_name>"
            echo "Example: sh ops/maker.sh --testing mainnet.eth-usdc"
            exit 1
        fi
    else
        CONFIG_NAME="$1"
        if [ -z "$CONFIG_NAME" ]; then
            echo -e "${RED}Error: Config argument required${NC}"
            echo "Usage: sh ops/maker.sh <config_name>"
            echo "       sh ops/maker.sh --testing <config_name>"
            echo "Example: sh ops/maker.sh base.eth-usdc"
            echo "         sh ops/maker.sh --testing mainnet.eth-usdc"
            exit 1
        fi
    fi

    # Set config path based on testing mode
    if [ "$TESTING_MODE" = true ]; then
        CONFIG_PATH="config/testing/$CONFIG_NAME.toml"
        SECRET_PATH="config/testing/secrets/.env.testing.$CONFIG_NAME.toml"
        echo "🧪 Running in TESTING mode with config: $CONFIG_NAME"
    else
        CONFIG_PATH="config/$CONFIG_NAME.toml"
        SECRET_PATH="config/secrets/.env.$CONFIG_NAME"
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
start $1 $2
