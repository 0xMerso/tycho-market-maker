# Frequently Asked Questions

## General Questions

### What is Tycho Market Maker?

Tycho Market Maker is an automated trading bot that provides liquidity and captures arbitrage opportunities on decentralized exchanges. It monitors price discrepancies between DEX pools and reference prices, executing profitable trades automatically.

### Which networks are supported?

Currently supported networks:
- **Ethereum Mainnet** (Chain ID: 1)
- **Base** (Chain ID: 8453)
- **Unichain** (Chain ID: 130) - Beta

### What are the minimum requirements to run the market maker?

**Technical:**
- 2+ CPU cores, 4GB RAM
- PostgreSQL 14+, Redis
- Stable internet connection

**Financial:**
- Wallet with private key access
- Minimum 0.1 ETH (or network native token) for gas
- Trading capital (varies by strategy)
- Tycho API key

### Is this profitable?

Profitability depends on:
- Market conditions (volatility)
- Configuration parameters
- Gas costs vs spread capture
- Competition from other bots
- Capital deployed

Always test thoroughly before deploying real capital.

## Configuration Questions

### How do I get a Tycho API key?

1. Visit [tycho-protocol.com](https://tycho-protocol.com)
2. Request API access
3. Wait for approval
4. Add key to your `.env` file

### What's the difference between `min_watch_spread_bps` and `min_executable_spread_bps`?

- **`min_watch_spread_bps`**: Minimum spread to start monitoring a pool (e.g., 5 bps = 0.05%)
- **`min_executable_spread_bps`**: Minimum spread required to execute a trade (e.g., 10 bps = 0.10%)

The executable spread should be higher to ensure profitability after gas costs.

### Can `min_executable_spread_bps` be negative?

Yes, negative values mean the bot will accept small losses to provide liquidity or maintain inventory balance. Use with caution.

### What is `max_inventory_ratio`?

The maximum portion of your wallet balance that can be used in a single trade. For example, 0.3 means maximum 30% of your balance per trade. This prevents overexposure.

### Should I use `infinite_approval`?

**Pros:**
- Saves gas on repeated trades
- Faster execution

**Cons:**
- Security risk if contract is compromised
- One-time large approval transaction

Recommended: Yes for trusted contracts, No for new/untested protocols.

## Trading Questions

### Why am I not seeing any trades?

Common reasons:
1. **Spreads too tight**: Market is efficient, no opportunities
2. **Parameters too conservative**: Lower `min_executable_spread_bps`
3. **Insufficient balance**: Check wallet has enough tokens
4. **Gas costs too high**: Profits eaten by transaction fees
5. **Testing mode enabled**: Check `TESTING=false` for real trades

### How does the bot calculate profitability?

```
Profit = (Spread - Gas Costs - Slippage) × Trade Amount

Where:
- Spread = Price difference between pool and reference
- Gas Costs = Transaction fee in USD
- Slippage = Price impact of the trade
```

### What happens during high gas prices?

The bot automatically:
1. Calculates gas costs in the output token
2. Subtracts from expected profit
3. Only trades if still profitable

You can adjust `min_executable_spread_bps` higher during high gas periods.

### How does MEV protection work?

On Ethereum mainnet:
- Uses Flashbots RPC for private transactions
- Sets `inclusion_block_delay` to avoid frontrunning
- Can enable `skip_simulation` for Flashbots bundles

## Technical Questions

### What's the difference between `maker` and `monitor`?

- **`maker`**: Main trading bot that executes trades
- **`monitor`**: Optional service that logs trades to database

You can run `maker` alone, but `monitor` provides persistence and analytics.

### Can I run multiple instances?

Yes! You can run multiple instances with:
- Different trading pairs
- Different networks
- Different strategies
- Different wallets

Ensure each has unique configuration and doesn't conflict.

### How do I update the bot?

```bash
# Pull latest changes
git pull origin main

# Rebuild
cargo build --release

# Restart services
sudo systemctl restart tycho-maker
```

### What does "Testing mode" do?

When `TESTING=true`:
- Simulates all trades without broadcasting
- Shortens restart delays
- Adds debug logging
- Perfect for development and testing strategies

## Error Messages

### "Token not found: Base token not found"

The token address isn't recognized by Tycho. Check:
- Token address is correct for the network
- Tycho has indexed this token
- Token has sufficient liquidity

### "Failed to get allowance"

The wallet hasn't approved tokens to Permit2. The bot will:
1. Automatically approve on first run if `infinite_approval=true`
2. Approve per-trade if `infinite_approval=false`

### "Simulation failed"

Transaction would fail on-chain. Common causes:
- Insufficient balance
- Slippage too high
- Pool state changed
- Gas limit too low

### "Price deviation too high"

Pool price is >5% different from reference price. This pool is filtered out to avoid stale/manipulated prices.

### "Optimization panicked"

Usually occurs with extreme token ratios (like BTC/USDC). The Tycho simulation library has an overflow. Report to Tycho team.

## Performance Questions

### How can I reduce RPC costs?

1. Increase `poll_interval_ms` (e.g., 10000 = 10 seconds)
2. Use a dedicated node instead of public RPC
3. Batch operations where possible
4. Cache data that doesn't change frequently

### How can I speed up reaction time?

1. Decrease `poll_interval_ms` (minimum ~500ms)
2. Use WebSocket connections instead of HTTP
3. Colocate with RPC node
4. Optimize network latency

### Why is my bot using so much memory?

The bot caches:
- Pool states (can be thousands)
- Protocol simulations
- Historical prices

Normal usage: 500MB-2GB depending on pools monitored.

## Security Questions

### How are private keys stored?

Private keys are:
- Stored in environment variables
- Never logged or transmitted
- Only loaded at startup
- Used only for signing transactions

**Never commit private keys to git!**

### Is it safe to run on a VPS?

Yes, with precautions:
1. Use dedicated user account
2. Enable firewall
3. Disable password SSH (keys only)
4. Encrypt sensitive files
5. Regular security updates

### What if the bot is compromised?

Immediate steps:
1. Transfer funds to new wallet
2. Revoke token approvals
3. Review logs for unauthorized transactions
4. Rotate all API keys
5. Investigate breach source

## Troubleshooting

### Bot keeps restarting

Check logs:
```bash
journalctl -u tycho-maker -n 100
```

Common causes:
- Configuration errors
- Network connectivity issues
- Database connection problems
- Panic in trading logic

### High failure rate

Monitor your success rate:
```sql
SELECT 
    COUNT(*) as total,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as successful
FROM trades
WHERE created_at > NOW() - INTERVAL '1 day';
```

If low, check:
- Slippage settings
- Gas limits
- Network congestion

### Database growing too large

Implement retention policy:
```sql
DELETE FROM trades WHERE created_at < NOW() - INTERVAL '30 days';
DELETE FROM prices WHERE created_at < NOW() - INTERVAL '7 days';
VACUUM ANALYZE;
```

## Best Practices

### Starting Out

1. **Test on testnet first**
2. **Start with small amounts** (`max_inventory_ratio = 0.1`)
3. **Use conservative spreads** (`min_executable_spread_bps = 20+`)
4. **Monitor closely** for first 48 hours
5. **Gradually increase exposure** as confidence grows

### Production Operations

1. **Set up monitoring** (Better Stack, Grafana)
2. **Implement alerting** for failures
3. **Regular backups** of database
4. **Log rotation** to prevent disk fill
5. **Security updates** monthly

### Risk Management

1. **Never risk more than you can afford to lose**
2. **Diversify across multiple pairs/networks**
3. **Set appropriate `max_inventory_ratio`**
4. **Monitor gas costs vs profits**
5. **Have an emergency stop procedure**

## Getting Help

### Where can I get support?

1. **Documentation**: Read through all guides
2. **GitHub Issues**: Report bugs and request features
3. **Discord**: Community support and discussions
4. **Logs**: Always include logs when asking for help

### How do I report a bug?

Create a GitHub issue with:
- Description of the problem
- Configuration (sanitized)
- Relevant logs
- Steps to reproduce
- Expected vs actual behavior

### Can I contribute?

Yes! We welcome contributions:
- Bug fixes
- New features
- Documentation improvements
- Strategy implementations

See [Contributing Guide](contributing.md) for details.