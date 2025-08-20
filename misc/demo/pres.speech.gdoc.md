# TAP-5: Tycho Market Maker Presentation Speech
*Duration: 5-7 minutes*
*Audience: Expert market makers, arbitrageurs, AMM specialists*

---

## Opening (30 seconds)

Hello everyone. I'm excited to show you TAP-5.

TAP-5 is a price stabilization and market making system. It trades against liquidity pools with Tycho to maintain prices within defined spreads. Built in Rust. Fully open source.

From an arbitrage perspective, it's statistical arbitrage with the on-chain leg only, so single-sided execution.

You can run it for profit, for price stability, or both. Ready for mainnet, Base, and Unichain.

Let me show you what makes it special.

---

## Core Architecture (1 minute)

Three main components work together.

The market maker service monitors pools and executes trades. The monitoring service captures events via Redis and persists to PostgreSQL. The web UI provides real-time visibility.

Data flows from Tycho API to our system,wWe compare pool prices with Binance or Chainlink or any reference price configured.

It swaps when spreads exceed thresholds, with optimal amounts via bundles (for revert protection). Publish results to Redis for real-time updates.

Everything is modular. Price feeds especially. It's the same code, but it has different execution behavior per chain.

---

## Technical Deep Dive (2 minutes)

*[After demo, while screen is still shared or back to slides]*

### Tycho Integration

Tycho provides the pool state infrastructure. Here's the flow.

We query Tycho API for pool snapshots. Every 500ms by default. Configurable via `poll_interval_ms`.

Tycho returns current reserves, fees, and pool parameters. We calculate marginal price from this data.

For execution, we use Tycho's router. Smart routing across multiple DEXs.

The simulation engine validates trades before execution. Uses Tycho's state to predict outcomes accurately.

### Configuration Variables Impact

Let me explain the key parameters.

**Spread Configuration:**
- `min_watch_spread_bps`: Monitoring threshold. Set to 5 means alert at 5 basis points deviation
- `min_executable_spread_bps`: Execution trigger. Zero means execute immediately when watch threshold hit
- These work together. Watch at 5, execute at 0 creates aggressive strategy

**Risk Parameters:**
- `max_slippage_pct`: Transaction revert protection. 0.0005 means 0.05% max price impact
- `max_inventory_ratio`: Position limits. 0.5 means never hold more than 50% in one asset
- `tx_gas_limit`: Cap transaction costs. 300000 typical for simple swaps

**Performance Tuning:**
- `poll_interval_ms`: API query frequency. Lower means faster reaction, higher API load
- `inclusion_block_delay`: Wait blocks before trading. Zero for immediate, higher for safety
- `skip_simulation`: Bypass pre-flight checks. Only for trusted setups with preconf RPCs

### Chain-Specific Execution

The execution layer adapts per chain.

Mainnet uses Flashbots bundle API. MEV protection guaranteed. Higher gas but safer.

Base leverages preconfirmation service. Faster inclusion. Lower latency. Perfect for high-frequency.

Unichain has custom gas oracle. Different fee structure. Optimized for their consensus.

Each chain module inherits base execution traits. Override specific methods. Clean abstraction.

### Price Feed Architecture

Factory pattern for price sources.

Binance feed polls REST API. Fast updates. Global price discovery. Set `reverse=false` for ETH/USDC.

Chainlink reads on-chain oracles. Decentralized. Higher latency. Specify oracle address in config.

Custom feeds plug in easily. Implement price feed trait. Add to factory. Configure in TOML.

---

## Quick Use Cases (30 seconds)

Before I show you the live system, let me quickly mention who benefits.

Market makers use this to extend CEX strategies on-chain. Protocol teams deploy it for price stability. Researchers quantify arbitrage opportunities.

Everyone can fork, modify, extend. It's your foundation.

Let me now share my screen to show you the live system.

*[SHARE SCREEN NOW - https://tap-5.vercel.app/strategies/3ee48b99-9676-4512-a241-f87a77d0b5f7]*
*[Drop link in chat while sharing]*

---

## Live Demo - With Screen Shared (1.5 minutes)

This is one instance running on Unichain. ETH-USDC pair.

Look at the metrics. 4000 trades in two weeks. Net PnL slightly positive. Fully automated.

Watch the spread chart. See how it maintains the bounds?

Every trade logged here. Full transparency. Real-time updates through Redis.

The bot considers gas costs. Respects inventory limits. Executes only when profitable after fees.

---

---

## Production Ready (30 seconds)

Written in Rust for performance. Handles thousands of pools. Redis pub-sub for real-time events. PostgreSQL persistence.

Comprehensive error handling. Retry logic. Structured logging. Docker deployment ready.

Integration tests with real networks. CI/CD pipelines. Battle tested in production.

---

## Next Steps & Resources (30 seconds)

Check out the early preview. Repository link in the chat.

We want your feedback. Reach out if you want to test. Help us improve before public release.

Coming soon: Pyth feeds. Multi-pool strategies. Cross-chain execution.

All the links are available:
- Repository on GitHub
- Live GUI at tap-5.vercel.app  
- Documentation coming
- Team handles on Telegram and Twitter

This is your foundation for on-chain market making. Fork it. Modify it. Deploy it.

Thank you. I'll hand it back to Tanay now.
