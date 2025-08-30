# Tycho Protocol Documentation

> Comprehensive documentation from https://docs.propellerheads.xyz/tycho

## Overview

Tycho is a comprehensive platform designed to help developers perform on-chain token swaps and protocol interactions across different blockchain networks with advanced simulation and execution capabilities.

## Core Purpose

Tycho provides infrastructure for:
- Real-time protocol data indexing and streaming
- Token swap simulation across multiple protocols
- Optimized swap execution with MEV protection
- Multi-chain support (Ethereum, Base, Unichain)

## Key Features

### 1. Real-time Protocol Data Indexing
- Streams live updates from multiple blockchain protocols
- Supports protocols like Uniswap V2, Uniswap V3, Balancer V2
- Allows filtering protocols by Total Value Locked (TVL)
- Reorg-aware data streaming
- Dynamic protocol element detection

### 2. Token Swap Simulation
- Calculate spot prices and output amounts across different liquidity pools
- Compare swap options across multiple protocols
- Provide gas cost estimates and protocol state transitions
- Performance: ~1-5 microseconds per native simulation, ~100-1000 microseconds for VM simulation

### 3. Swap Execution
- Encode swap transactions for various router contracts
- Support different transfer types (Permit2, ERC20 approvals, direct transfers)
- Provide safeguards against MEV and slippage risks
- Solidity contracts for on-chain trade execution

## Architecture Components

### Tycho Indexer
The data streaming service providing real-time, low-latency onchain liquidity data.

**Two Indexing Approaches:**

1. **Native Simulation**
   - Provides structured data mirroring on-chain states
   - Allows simulating protocol logic outside the VM
   - Useful for analytical trading curve solutions
   - Fastest performance (~1-5 microseconds per simulation)

2. **Virtual Machine (VM) Compatibility**
   - Tracks protocol contract states
   - Enables local simulations with minimal network overhead
   - Slightly slower (~100-1000 microseconds per simulation)

**Key Capabilities:**
- Reorg-aware data streaming
- Complete protocol system tracking
- Detailed component data tracking
- Leverages Substreams indexing framework
- Detects new protocol elements dynamically
- Records both static and dynamic state changes

### Tycho Simulation
A Rust crate for powerful protocol interaction and token swap simulations.

**Main Interface Methods:**
- `spot_price()`: Returns the pool's current marginal price
- `get_amount_out()`: Simulates token swaps
- `fee()`: Returns protocol fee ratio
- `get_limits()`: Returns maximum tradable amounts between tokens

**Installation:**
```toml
tycho-simulation = { 
    git = "https://github.com/propeller-heads/tycho-simulation.git",
    package = "tycho-simulation",
    tag = "x.y.z", 
    features = ["evm"]
}
```

**Streaming Protocol States:**
- Uses Tycho Indexer to maintain up-to-date protocol states
- Supports multiple protocols in one stream
- First message contains all initial pool states
- Subsequent messages only include updated components

### Tycho Execution
Tools for encoding and executing swaps against Tycho routers and protocol executors.

**Components:**
1. **Encoding**: Rust crate for generating swap calldata
2. **Executing**: Solidity contracts for on-chain trade execution

**Token Transfer Methods:**
- Permit2
- Standard ERC20 Approvals
- Direct Transfers (⚠️ Caution: Tycho Router not designed to securely hold funds)

**Source Code:** Available at https://github.com/propeller-heads/tycho-execution

## Supported Networks

- **Ethereum Mainnet**: Full protocol support with Flashbots integration
- **Base**: L2 optimized operations
- **Unichain**: Custom network configurations

## Supported Protocols

### Currently Supported:
- Uniswap V2
- Uniswap V3
- Balancer V2
- Curve (various implementations)
- Additional protocols through Request for Quote (RFQ) systems

## Developer Workflow

1. **Connect to Tycho Indexer**
   - Setup client (Binary/CLI, Rust, or Python)
   - Configure protocol filters (TVL, specific protocols)

2. **Fetch Available Tokens and Protocol States**
   - Stream real-time protocol updates
   - Maintain local state representation

3. **Simulate Potential Swaps**
   - Calculate optimal routes
   - Compare prices across protocols
   - Estimate gas costs

4. **Encode Optimal Swap Solution**
   - Generate calldata for execution
   - Configure transfer methods

5. **Execute or Simulate Transaction**
   - Submit to chain or test locally
   - Monitor execution results

## Code Examples

### Basic Protocol Stream Setup (Rust)
```rust
let protocol_stream = ProtocolStreamBuilder::new(&tycho_url, Chain::Ethereum)
    .exchange::<UniswapV2State>("uniswap_v2", tvl_filter, None)
    .build()
    .await;
```

### Swap Simulation
```rust
// Calculate amount out for a swap
let result = state.get_amount_out(amount_in, &token_in, &token_out);

// Get spot price
let price = state.spot_price(&token_a, &token_b);

// Check trading limits
let limits = state.get_limits(&token_a, &token_b);
```

## Client Libraries

### Binary/CLI
Command-line interface for direct interaction with Tycho services.

### Rust Client
Native Rust implementation with full feature support.
- Repository: https://github.com/propeller-heads/tycho-simulation

### Python Client
Python bindings for integration with data science workflows.

## Documentation Structure

### For Solvers
- **Indexer**: Data streaming and protocol tracking
  - Tycho RPC
  - Client implementations (Binary, Rust, Python)
- **Simulation**: Protocol interaction and swap calculations
- **Execution**: Transaction encoding and submission
  - Contract addresses
  - Execution venues
- **Hosted Endpoints**: Available service endpoints
- **Supported Protocols**: Complete protocol list

### For DEXs
- **Protocol Integration**: Adding new protocols to Tycho
  - **Indexing**:
    - Setup and implementation
    - Common patterns (tracking components, storage, balances)
    - Custom protobuf models
  - **Simulation**: Protocol-specific simulation logic
  - **Execution**: Router integration
  - **Testing**: Validation procedures
- **Request for Quote Protocols**: RFQ system integration
- **Contributing Guidelines**: How to contribute to Tycho

## Key Differentiators

1. **Real-time Data Streaming**: Low-latency protocol state updates
2. **Multi-Protocol Support**: Unified interface across different DEXs
3. **Performance Optimization**: Microsecond-level simulation speeds
4. **MEV Protection**: Built-in safeguards against sandwich attacks
5. **Flexible Execution**: Multiple transfer methods and execution venues
6. **Extensibility**: Support for custom protocols and RFQ systems

## Important Considerations

### Security
- Direct transfers require extreme caution as Tycho Router is not designed to hold funds
- Always validate simulation results before execution
- Use appropriate slippage protection

### Performance
- Native simulation: ~1-5 microseconds
- VM simulation: ~100-1000 microseconds
- Network latency additional for on-chain execution

### Best Practices
- Filter protocols by TVL to focus on liquid pools
- Use streaming updates for real-time data
- Implement proper error handling for reorgs
- Test simulations before mainnet execution

## Resources

- **Main Documentation**: https://docs.propellerheads.xyz/tycho
- **GitHub Organization**: https://github.com/propeller-heads
- **Tycho Simulation**: https://github.com/propeller-heads/tycho-simulation
- **Tycho Execution**: https://github.com/propeller-heads/tycho-execution

## Contributing

Tycho welcomes contributions in several areas:
- Protocol integrations
- Client library improvements
- Documentation enhancements
- Bug reports and fixes

See the contributing guidelines in the documentation for detailed information on:
- Bounty programs
- Code submission process
- Testing requirements
- Community engagement

## Transparency

Tycho emphasizes transparency in:
- Protocol data accuracy
- Simulation methodology
- Execution processes
- Community governance

---

*This documentation is compiled from the official Tycho documentation at https://docs.propellerheads.xyz/tycho and will be updated as the project evolves.*