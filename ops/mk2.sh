#!/bin/bash

RED='\033[0;31m'
NC='\033[0m'

# --- Usage ---
# Requires Rust and Cargo to be installed.
# You need to provide the TOML Market Maker configuration file.
# sh ops/mm.start.sh
# sh ops/mm.start.sh test

function start() {
    trap '' SIGINT
    # ------------- Redis -------------
    # rm -rf dump.rdb
    # ps -ef | grep redis-server | grep -v grep | awk '{print $2}' | xargs kill 2>/dev/null
    # redis-server --port 42777 --bind 127.0.0.1 2>&1 >/dev/null &
    # # redis-server src/shared/config/redis.conf --bind 127.0.0.1 2>&1 >/dev/null &
    # echo "Redis ready #$(ps -ef | grep redis-server | grep -v grep | awk '{print $2}')"
    # sleep 1
    # ------------- Execute -------------
    if [ "$1" = "test" ]; then
        export RUST_LOG="off,mk2=trace,shd=trace,test=trace"
        cargo test -- --nocapture
    else
        echo "Building MarketMaker program (might take a few minutes the first time) ..."
        cargo build --bin mk2 -q 2>/dev/null
        echo "Build successful. Executing..."
        (
            trap - SIGINT
            export RUST_LOG="off,mk2=trace,shd=trace"
            cargo run --bin mk2 -q # 2>/dev/null
        )
        echo "Program has finished or was interrupted. Continuing with the rest of the shell script ..."
        status+=($?)
        if [ $status -ne 0 ]; then
            echo "Error: $status on program ${RED}${program}${NC}"
            exit 1
        fi
    fi
    # ps -ef | grep redis-server | grep -v grep | awk '{print $2}' | xargs kill 2>/dev/null
    # rm -rf dump.rdb
}

# export CONFIG_PATH="config/mmc.mainnet.eth-usdc.toml"
# export CONFIG_PATH="config/mmc.mainnet.eth-wbtc.toml"
# export CONFIG_PATH="config/mmc.unichain.eth-usdc.toml"
export CONFIG_PATH="config/mmc.base.eth-usdc.toml"
start $1
