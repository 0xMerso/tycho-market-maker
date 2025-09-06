#!/bin/bash

# Test runner for Tycho Market Maker
# Usage: 
#   sh tests/runner.sh                    # Run all tests
#   sh tests/runner.sh parsing            # Run tests in parsing.rs
#   sh tests/runner.sh parsing test_name  # Run specific test

export RUST_LOG=${RUST_LOG:-debug}

if [ -z "$1" ]; then
    cargo test -- --nocapture
elif [ -z "$2" ]; then
    cargo test --test $1 -- --nocapture
else
    cargo test --test $1 $2 -- --nocapture --exact
fi

# cargo test test_parse_all_configs -- --nocapture