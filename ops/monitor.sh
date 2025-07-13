#!/bin/bash

# --- Usage ---
# Requires Rust and Cargo to be installed.
# You need to provide the TOML Market Maker configuration file.
# This script launch the monitortoring program for multiple instance of the Market Maker program
# It takes as argument an array of configuration files path, same format as the individual maker config file
# It also launch the redis server
# It expects the market maker instance to be running in another terminal.

RED='\033[0;31m'
NC='\033[0m'

function start() {
    trap '' SIGINT
    # ------------- Redis -------------
    rm -f dump.rdb
    pkill -f "redis-server.*42044" 2>/dev/null || true
    redis-server --port 42044 --bind 127.0.0.1 >/dev/null 2>&1 &
    echo "Redis ready #$!"
    sleep 1

    # ------------- Build -------------
    echo "Building monitor (first run may be slow)…"
    cargo build --bin monitor -q 2>/dev/null
    echo "Build successful. Launching monitor…"

    # ------------- Execute once with all configs -------------
    (
        trap - SIGINT
        export RUST_LOG="off,maker=trace,shd=trace,monitor=trace"
        cargo run --bin monitor -q
    )

    echo "Monitor finished. Cleaning up…"
    pkill -f "redis-server.*42044"
    rm -f dump.rdb
}

# Comma-separated list of config files (your program reads this env var as an array)
export CONFIGS_PATHS="config/mmc.mainnet.eth-usdc.toml,config/mmc.mainnet.eth-wbtc.toml,config/mmc.mainnet.usdc-dai.toml,config/mmc.unichain.eth-usdc.toml,config/mmc.base.eth-usdc.toml"

start
