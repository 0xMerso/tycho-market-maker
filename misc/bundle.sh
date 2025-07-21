#!/usr/bin/env bash

# Usage:
#   export FLASHBOTS_RPC="https://relay.flashbots.net"
#   export BUNDLE_HASH="0xYOURBUNDLEHASH"
#   export BLOCK_DEC=12345678
#   ./get_bundle_stats.sh


FLASHBOTS_RPC="https://relay.flashbots.net"
BUNDLE_HASH="0xf8aa7f70275be1222e04810103b5bc30d7f3f8699d8152da42306883e576e8fc"
BLOCK_DEC=22953244

# Fail fast
set -euo pipefail

: "${FLASHBOTS_RPC:?Need to set FLASHBOTS_RPC}"
: "${BUNDLE_HASH:?Need to set BUNDLE_HASH}"
: "${BLOCK_DEC:?Need to set BLOCK_DEC (decimal block number)}"

# Convert decimal block number to hex prefixed with 0x
BLOCK_HEX=$(printf '0x%x' "$BLOCK_DEC")

# Query bundle stats
curl -s "$FLASHBOTS_RPC" \
  -H 'Content-Type: application/json' \
  --data '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"flashbots_getBundleStats",
    "params":[
      {
        "bundleHash":"'"${BUNDLE_HASH}"'",
        "blockNumber":"'"${BLOCK_HEX}"'"
      }
    ]
  }'

# Example

# 2025-07-19T12:28:05.310414Z DEBUG shd::maker::r#impl:  - [0xb4e16      uniswap_v2  30] : Building Tycho solution: Buying WETH with USDC | Amount in: 114233731 | Amount out: 32058113524639744 | Amount out min: 0.032042084467877416 WETH
# 2025-07-19T12:28:05.596990Z  INFO shd::maker::r#impl: Prepared first trade only (🧪 skipping 0 other opportunities for now)
# 2025-07-19T12:28:05.597289Z  INFO shd::maker::exec::chain::mainnet: [MainnetExec] Executing 1 transactions on mainnet
# 2025-07-19T12:28:05.597306Z  INFO shd::maker::exec::chain::mainnet: 🚀 Skipping simulation - direct execution enabled
# 2025-07-19T12:28:05.597370Z  INFO shd::maker::exec::chain::mainnet: 🌐 [MainnetExec] Broadcasting 1 transactions on Mainnet with Flashbots
# 2025-07-19T12:28:05.852139Z  INFO shd::maker::exec::chain::mainnet: 🌐 [MainnetExec] Current block: 22953243, target inclusion block: 22953244 (delay: 1)
# 2025-07-19T12:28:06.496944Z  INFO shd::maker::exec::chain::mainnet: Bundle sent, with hash 0xf8aa7f70275be1222e04810103b5bc30d7f3f8699d8152da42306883e576e8fc

