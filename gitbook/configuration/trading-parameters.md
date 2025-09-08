# Trading Parameters

## Overview

Trading parameters control the market maker's behavior, risk management, and execution logic. These settings directly impact profitability and risk exposure.

## Core Trading Parameters

### 📊 Spread Parameters

#### `min_watch_spread_bps`
- **Description**: Minimum spread to start monitoring a pool (in basis points)
- **Type**: Float
- **Range**: 0.0 - 10000.0
- **Example**: `5.0` (0.05%)
- **Purpose**: Filters out pools with spreads too small to be interesting
- **Strategy**: Lower values = more opportunities monitored, higher CPU usage

#### `min_executable_spread_bps`
- **Description**: Minimum spread required to execute a trade (in basis points)
- **Type**: Float
- **Range**: -50.0 - 10000.0
- **Example**: `10.0` (0.10%)
- **Purpose**: Ensures trades are profitable after gas costs
- **Important**: Can be negative for aggressive market making
- **Formula**: `profit = spread_bps - gas_costs_bps - slippage_bps`

**Spread Calculation Example:**
```
Pool Price: $3,850
Reference Price: $3,900
Spread: $50
Spread BPS: (50/3900) * 10000 = 128 bps (1.28%)

If min_executable_spread_bps = 100:
✅ Trade executes (128 > 100)

If min_executable_spread_bps = 150:
❌ Trade skipped (128 < 150)
```

### 💰 Risk Management

#### `max_inventory_ratio`
- **Description**: Maximum portion of wallet balance to use per trade
- **Type**: Float
- **Range**: 0.0 - 1.0
- **Example**: `0.5` (use max 50% of balance)
- **Purpose**: Prevents overexposure on single trades
- **Best Practice**: Start with 0.1-0.2 for testing

#### `max_slippage_pct`
- **Description**: Maximum acceptable slippage percentage
- **Type**: Float
- **Range**: 0.0 - 1.0
- **Example**: `0.005` (0.5%)
- **Purpose**: Protects against MEV and price manipulation
- **Impact**: Lower = safer but more failed transactions

### ⛽ Gas Management

#### `tx_gas_limit`
- **Description**: Maximum gas units per transaction
- **Type**: Integer
- **Range**: 50000 - 1000000
- **Default**: `300000`
- **Network Specific**:
  - Ethereum: 200000-400000 typical
  - Base/L2: 100000-200000 typical
  - Complex swaps: May need 500000+

#### `block_offset`
- **Description**: Blocks to wait before considering price stale
- **Type**: Integer
- **Range**: 0 - 10
- **Default**: `1`
- **Purpose**: Ensures price data freshness

#### `inclusion_block_delay`
- **Description**: Blocks to wait for transaction inclusion
- **Type**: Integer
- **Range**: 0 - 5
- **Default**: `1`
- **Purpose**: MEV protection on mainnet

### ⏱️ Timing Parameters

#### `poll_interval_ms`
- **Description**: Milliseconds between market checks
- **Type**: Integer
- **Range**: 100 - 60000
- **Examples**:
  - Fast trading: `500` (0.5 seconds)
  - Normal: `6000` (6 seconds)
  - Conservative: `30000` (30 seconds)
- **Trade-off**: Lower = more responsive, higher RPC costs

#### `min_publish_timeframe_ms`
- **Description**: Minimum time between publishing events
- **Type**: Integer
- **Minimum**: `30000` (30 seconds)
- **Default**: `30000`
- **Purpose**: Prevents event spam

## Advanced Parameters

### 🔄 Execution Modes

#### `skip_simulation`
- **Description**: Skip transaction simulation before execution
- **Type**: Boolean
- **Default**: `false`
- **When to use `true`**:
  - Mainnet with Flashbots
  - Preconfirmation RPCs
  - Time-sensitive arbitrage
- **Risk**: Higher chance of failed transactions

#### `infinite_approval`
- **Description**: Approve maximum amount once vs per-trade
- **Type**: Boolean
- **Default**: `true`
- **Benefits**: Saves gas on repeated trades
- **Risk**: Larger exposure if contract compromised

#### `publish_events`
- **Description**: Publish trade events to Redis
- **Type**: Boolean
- **Default**: `false`
- **Purpose**: Enable monitoring and analytics
- **Requirement**: Redis must be running

## Strategy Configurations

### Conservative Strategy
```toml
# Safe for beginners
min_watch_spread_bps = 20.0      # 0.20%
min_executable_spread_bps = 30.0 # 0.30%
max_inventory_ratio = 0.1         # 10% max
max_slippage_pct = 0.01          # 1%
poll_interval_ms = 10000         # 10 seconds
```

### Balanced Strategy
```toml
# Good risk/reward
min_watch_spread_bps = 10.0      # 0.10%
min_executable_spread_bps = 15.0 # 0.15%
max_inventory_ratio = 0.3        # 30% max
max_slippage_pct = 0.005        # 0.5%
poll_interval_ms = 5000         # 5 seconds
```

### Aggressive Strategy
```toml
# High risk/reward
min_watch_spread_bps = 5.0       # 0.05%
min_executable_spread_bps = 7.0  # 0.07%
max_inventory_ratio = 0.5        # 50% max
max_slippage_pct = 0.003        # 0.3%
poll_interval_ms = 1000         # 1 second
```

### Market Making Strategy
```toml
# Provide liquidity
min_watch_spread_bps = 2.0         # 0.02%
min_executable_spread_bps = -10.0  # -0.10% (willing to take small loss)
max_inventory_ratio = 0.7          # 70% max
max_slippage_pct = 0.002          # 0.2%
poll_interval_ms = 500            # 0.5 seconds
```

## Network-Specific Settings

### Ethereum Mainnet
```toml
# High gas costs, MEV protection needed
min_executable_spread_bps = 20.0  # Higher due to gas
tx_gas_limit = 400000
inclusion_block_delay = 1          # MEV protection
skip_simulation = true             # Using Flashbots
```

### Base L2
```toml
# Low gas costs, fast blocks
min_executable_spread_bps = 5.0   # Lower viable spread
tx_gas_limit = 200000
inclusion_block_delay = 0
skip_simulation = false
poll_interval_ms = 2000           # Faster polling
```

### Unichain
```toml
# Beta network, be conservative
min_executable_spread_bps = 15.0
tx_gas_limit = 300000
skip_simulation = false           # Always simulate
poll_interval_ms = 5000
```

## Parameter Optimization

### 1. Finding Optimal Spreads

Start wide and narrow down:

```toml
# Week 1: Wide spreads
min_executable_spread_bps = 50.0

# Week 2: If profitable, reduce
min_executable_spread_bps = 30.0

# Week 3: Find minimum profitable
min_executable_spread_bps = 20.0
```

### 2. Inventory Management

Monitor utilization:

```toml
# If never using full allocation:
max_inventory_ratio = 0.5  # Increase

# If frequently maxed out:
max_inventory_ratio = 0.2  # Decrease
```

### 3. Gas Optimization

Track failed transactions:

```toml
# Many "out of gas" errors:
tx_gas_limit = 500000  # Increase

# Consistently using < 200k:
tx_gas_limit = 250000  # Decrease to save
```

## Monitoring Parameters

### Key Metrics to Track

1. **Spread Capture Rate**
   - Actual spread vs min_executable_spread
   - Adjust if consistently higher

2. **Slippage Statistics**
   - Average slippage vs max_slippage_pct
   - Tighten if consistently lower

3. **Inventory Turnover**
   - How often max_inventory_ratio is hit
   - Increase if underutilized

4. **Gas Efficiency**
   - Gas used vs tx_gas_limit
   - Optimize based on actual usage

## Risk Considerations

### Parameter Risks

| Parameter | Too Low | Too High |
|-----------|---------|----------|
| min_executable_spread_bps | Unprofitable trades | Missed opportunities |
| max_inventory_ratio | Limited profit potential | Overexposure |
| max_slippage_pct | Failed transactions | Loss to MEV |
| poll_interval_ms | High RPC costs | Slow reaction |

### Safety Limits

Always maintain:
- `min_executable_spread_bps >= estimated_gas_cost_bps`
- `max_inventory_ratio <= 0.5` for beginners
- `max_slippage_pct >= 0.001` (0.1%) minimum
- `tx_gas_limit <= 1000000` (hard limit)

## Dynamic Adjustment

### Market Conditions

**High Volatility:**
```toml
min_executable_spread_bps = 30.0  # Increase
max_slippage_pct = 0.01          # Increase
poll_interval_ms = 1000          # Decrease
```

**Low Volatility:**
```toml
min_executable_spread_bps = 10.0  # Decrease
max_slippage_pct = 0.003         # Decrease
poll_interval_ms = 10000         # Increase
```

**High Gas Prices:**
```toml
min_executable_spread_bps = 50.0  # Increase significantly
max_inventory_ratio = 0.7        # Use larger trades
```

## Testing Parameters

### Development Settings
```toml
# Safe for testing
TESTING = true  # No real transactions
min_executable_spread_bps = 5.0
max_inventory_ratio = 0.01
poll_interval_ms = 30000
```

### Paper Trading
```toml
# Realistic but safe
TESTING = true
min_executable_spread_bps = 15.0
max_inventory_ratio = 0.2
poll_interval_ms = 5000
```

## Next Steps

- Review [Network Settings](network-settings.md)
- Configure [Price Feeds](price-feeds.md)
- Set up [Monitoring](../deployment/monitoring.md)
- Learn about [Risk Management](../advanced/risk-management.md)