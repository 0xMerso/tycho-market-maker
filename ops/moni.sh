#!/bin/bash

RED='\033[0;31m'
NC='\033[0m'

# --- Usage ---
# Requires Rust and Cargo to be installed.
# You need to provide the TOML Market Maker configuration file.
# This script launch the monitoring program for multiple instance of the Market Maker program
# It takes as argument an array of configuration files path, same format as the individual mk2 config file
# It also launch the redis server
# It expects the market maker instance to be running in another terminal.

function start() {
    trap '' SIGINT
    # ------------- Redis -------------
    rm -rf dump.rdb
    ps -ef | grep redis-server | grep -v grep | awk '{print $2}' | xargs kill 2>/dev/null
    redis-server --port 42044 --bind 127.0.0.1 2>&1 >/dev/null &
    # redis-server src/shared/config/redis.conf --bind 127.0.0.1 2>&1 >/dev/null &
    echo "Redis ready #$(ps -ef | grep redis-server | grep -v grep | awk '{print $2}')"
    sleep 1
    # ------------- Execute -------------
    echo "Building Moni program (might take a few minutes the first time) ..."
    cargo build --bin moni -q 2>/dev/null
    echo "Build successful. Executing..."
    (
        trap - SIGINT
        export RUST_LOG="off,mk2=trace,shd=trace,moni=trace"
        cargo run --bin moni -q # 2>/dev/null
    )
    echo "Program has finished or was interrupted. Continuing with the rest of the shell script ..."
    status+=($?)
    if [ $status -ne 0 ]; then
        echo "Error: $status on program ${RED}${program}${NC}"
        exit 1
    fi
    ps -ef | grep redis-server | grep -v grep | awk '{print $2}' | xargs kill 2>/dev/null
    rm -rf dump.rdb
}

# export CONFIG_PATH="config/mmc.mainnet.eth-usdc.toml"
# export CONFIG_PATH="config/mmc.mainnet.eth-wbtc.toml"
# export CONFIG_PATH="config/mmc.mainnet.usdc-dai.toml"
# export CONFIG_PATH="config/mmc.unichain.eth-usdc.toml"
# export CONFIG_PATH="config/mmc.base.eth-usdc.toml"
export CONFIGS_PATHS="config/mmc.mainnet.eth-usdc.toml,config/mmc.mainnet.eth-wbtc.toml,config/mmc.mainnet.usdc-dai.toml,config/mmc.unichain.eth-usdc.toml,config/mmc.base.eth-usdc.toml"
start $1
