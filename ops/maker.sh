#!/bin/bash

# Tycho Market Maker Startup Script
# Usage: sh ops/maker.sh <config_name>
#        sh ops/maker.sh --testing <config_name>
# Example: sh ops/maker.sh base.eth-usdc

# ! If you prefer, a one-liner command, instead of the script
# RUST_LOG="off,maker=trace,shd=trace" CONFIG_PATH=config/unichain.eth-usdc.toml SECRET_PATH=config/secrets/.env.unichain.eth-usdc cargo run --bin maker

RED='\033[0;31m'
NC='\033[0m'

cleanup() {
    echo -e "\n${RED}Shutting down market maker...${NC}"
    exit 0
}

start() {
    TESTING_MODE=false
    CONFIG_NAME=""

    if [ "$1" = "--testing" ]; then
        TESTING_MODE=true
        CONFIG_NAME="$2"
        if [ -z "$CONFIG_NAME" ]; then
            echo -e "${RED}Error: Config name required after --testing flag${NC}"
            echo "Usage: sh ops/maker.sh --testing <config_name>"
            exit 1
        fi
    else
        CONFIG_NAME="$1"
        if [ -z "$CONFIG_NAME" ]; then
            echo -e "${RED}Error: Config argument required${NC}"
            echo "Usage: sh ops/maker.sh <config_name>"
            echo "       sh ops/maker.sh --testing <config_name>"
            exit 1
        fi
    fi

    if [ "$TESTING_MODE" = true ]; then
        CONFIG_PATH="config/testing/$CONFIG_NAME.toml"
        SECRET_PATH="config/testing/secrets/.env.testing.$CONFIG_NAME.toml"
        echo "🧪 Running in TESTING mode with config: $CONFIG_NAME"
    else
        CONFIG_PATH="config/$CONFIG_NAME.toml"
        SECRET_PATH="config/secrets/.env.$CONFIG_NAME"
    fi

    echo "CONFIG_PATH:" $CONFIG_PATH
    echo "SECRET_PATH:" $SECRET_PATH

    trap cleanup SIGINT SIGTERM

    echo "Building MarketMaker program..."
    cargo build --bin maker
    if [ $? -ne 0 ]; then
        echo -e "${RED}Build failed${NC}"
        exit 1
    fi
    echo "Build successful. Executing with config: $CONFIG_PATH"

    export RUST_LOG="off,maker=trace,shd=trace"
    export CONFIG_PATH="$CONFIG_PATH"
    export SECRET_PATH="$SECRET_PATH"
    cargo run --bin maker

    echo "Program has finished or was interrupted."
}

start $1 $2
