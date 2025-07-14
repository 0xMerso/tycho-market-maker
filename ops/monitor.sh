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

# Function to cleanup on exit
cleanup() {
    echo -e "\n${RED}Shutting down...${NC}"
    pkill -f "redis-server.*42044" 2>/dev/null || true
    rm -f dump.rdb
    exit 0
}

function start() {
    echo "Starting Redis …"

    # Set up signal handlers
    trap cleanup SIGINT SIGTERM

    # ------------- Redis -------------
    rm -f dump.rdb
    pkill -f "redis-server.*42044" 2>/dev/null || true
    redis-server --port 42044 --bind 127.0.0.1 >/dev/null 2>&1 &
    REDIS_PID=$!
    echo "Redis ready (PID: $REDIS_PID)"
    sleep 1

    # ------------- Build -------------
    echo "Building monitor (first run may be slow) …"
    cargo build --bin monitor
    if [ $? -ne 0 ]; then
        echo -e "${RED}Build failed${NC}"
        cleanup
    fi
    echo "Build successful. Launching monitor…"

    # ------------- Execute monitor -------------
    export RUST_LOG="off,maker=trace,shd=trace,monitor=trace"
    cargo run --bin monitor

    echo "Monitor finished. Cleaning up…"
    cleanup
}

start
