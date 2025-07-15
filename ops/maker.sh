#!/bin/bash

RED='\033[0;31m'
NC='\033[0m'

# --- Usage ---
# Requires Rust and Cargo to be installed.
# You need to provide the TOML Market Maker configuration file.
# sh ops/mm.start.sh
# sh ops/mm.start.sh test

# Function to cleanup on exit
cleanup() {
    echo -e "\n${RED}Shutting down market maker...${NC}"
    exit 0
}

function start() {
    # Set up signal handlers
    trap cleanup SIGINT SIGTERM

    if [ "$1" = "test" ]; then
        export RUST_LOG="off,maker=trace,shd=trace,test=trace"
        cargo test -- --nocapture
    else
        echo "Building MarketMaker program (might take a few minutes the first time) ..."
        cargo build --bin maker
        if [ $? -ne 0 ]; then
            echo -e "${RED}Build failed${NC}"
            exit 1
        fi
        echo "Build successful. Executing..."

        export RUST_LOG="off,maker=trace,shd=trace"
        cargo run --bin maker

        echo "Program has finished or was interrupted."
    fi
}

# export CONFIG_PATH="config/mainnet.eth-usdc.toml"
# export CONFIG_PATH="config/mainnet.usdc-dai.toml"
# export CONFIG_PATH="config/mainnet.eth-wbtc.toml"
# export CONFIG_PATH="config/unichain.eth-usdc.toml"
export CONFIG_PATH="config/base.eth-usdc.toml"
start $1
