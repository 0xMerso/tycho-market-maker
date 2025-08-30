use alloy::providers::Provider;
use shd::maker::feed::chainlink;
use shd::types::config::load_market_maker_config;
use shd::utils::evm::{create_provider, eip1559_fees, gas_price, latest};

#[test]
fn test_parse_all_configs() {
    // List of all config files to test
    let config_files = vec![
        "config/base.eth-usdc.toml",
        "config/mainnet.eth-usdc.toml",
        "config/mainnet.eth-wbtc.toml",
        "config/mainnet.usdc-dai.toml",
        "config/unichain.eth-usdc.toml",
    ];

    println!("\n🔍 Testing parsing of all config files...\n");

    for config_path in config_files {
        println!("📄 Testing: {}", config_path);

        // Test loading the config
        let config = load_market_maker_config(config_path);

        // Assert the config loads successfully
        assert!(config.is_ok(), "Failed to parse config {}: {:?}", config_path, config);

        let config = config.unwrap();

        // Verify essential fields are populated
        assert!(!config.base_token.is_empty(), "base_token is empty in {}", config_path);
        assert!(!config.quote_token.is_empty(), "quote_token is empty in {}", config_path);
        assert!(!config.base_token_address.is_empty(), "base_token_address is empty in {}", config_path);
        assert!(!config.quote_token_address.is_empty(), "quote_token_address is empty in {}", config_path);
        assert!(!config.network_name.is_empty(), "network_name is empty in {}", config_path);
        assert!(config.chain_id > 0, "chain_id is 0 in {}", config_path);
        assert!(!config.wallet_public_key.is_empty(), "wallet_public_key is empty in {}", config_path);
        assert!(!config.rpc_url.is_empty(), "rpc_url is empty in {}", config_path);
        assert!(!config.tycho_api.is_empty(), "tycho_api is empty in {}", config_path);
        assert!(!config.permit2_address.is_empty(), "permit2_address is empty in {}", config_path);
        assert!(!config.tycho_router_address.is_empty(), "tycho_router_address is empty in {}", config_path);

        // Verify execution parameters are reasonable
        // Note: min_watch_spread_bps and min_executable_spread_bps can be negative for certain strategies
        assert!(config.max_slippage_pct >= 0.0, "max_slippage_pct is negative in {}", config_path);
        assert!(
            config.max_inventory_ratio > 0.0 && config.max_inventory_ratio <= 1.0,
            "max_inventory_ratio out of bounds in {}",
            config_path
        );
        assert!(config.tx_gas_limit > 0, "tx_gas_limit is 0 in {}", config_path);
        assert!(config.poll_interval_ms > 0, "poll_interval_ms is 0 in {}", config_path);

        // Verify price feed config
        assert!(!config.price_feed_config.r#type.is_empty(), "price_feed_config.type is empty in {}", config_path);
        assert!(!config.price_feed_config.source.is_empty(), "price_feed_config.source is empty in {}", config_path);

        println!("  ✅ Config parsed successfully");
        println!("     Network: {} (Chain ID: {})", config.network_name, config.chain_id);
        println!("     Pair: {} {}/{}", config.pair_tag, config.base_token, config.quote_token);
        println!("     Price Feed: {}", config.price_feed_config.r#type);
        println!("     Spread: {} bps (watch), {} bps (exec)", config.min_watch_spread_bps, config.min_executable_spread_bps);
        println!();
    }

    println!("✨ All configs parsed successfully!\n");
}

#[test]
fn test_parse_current_config() {
    // Load the actual config file from the config folder
    let config_path = "config/unichain.eth-usdc.toml";

    // Test loading the config
    let config = load_market_maker_config(config_path);

    // Assert the config loads successfully
    assert!(config.is_ok(), "Failed to parse current config format: {:?}", config);

    let config = config.unwrap();

    // Test all current field names are parsed correctly
    assert_eq!(config.pair_tag, "🟣");
    assert_eq!(config.base_token, "ETH");
    assert_eq!(config.base_token_address, "0x4200000000000000000000000000000000000006");
    assert_eq!(config.quote_token, "USDC");
    assert_eq!(config.quote_token_address, "0x078D782b760474a361dDA0AF3839290b0EF57AD6");
    assert_eq!(config.network_name, "unichain");
    assert_eq!(config.chain_id, 130);
    assert_eq!(config.wallet_public_key, "0xF5029A50494714b3f80b39F05acC2c9Cea017FD6");
    assert_eq!(config.gas_token_symbol, "0x4200000000000000000000000000000000000006");
    assert_eq!(config.gas_token_chainlink_price_feed, "");
    assert_eq!(config.rpc_url, "https://unichain-mainnet.blastapi.io/713ef8fa-d6fb-4192-bf47-3c924ec3ff6b");
    assert_eq!(config.explorer_url, "https://uniscan.xyz/");
    assert_eq!(config.tycho_api, "tycho-unichain-beta.propellerheads.xyz");

    // Execution parameters
    assert_eq!(config.min_watch_spread_bps, 2.0);
    assert_eq!(config.min_executable_spread_bps, 2.0);
    assert_eq!(config.max_slippage_pct, 0.0005);
    assert_eq!(config.max_inventory_ratio, 0.5);
    assert_eq!(config.tx_gas_limit, 300000);
    assert_eq!(config.block_offset, 1);
    assert_eq!(config.inclusion_block_delay, 0);
    assert_eq!(config.permit2_address, "0x000000000022D473030F116dDEE9F6B43aC78BA3");
    assert_eq!(config.tycho_router_address, "0xFfA5ec2e444e4285108e4a17b82dA495c178427B");

    // Misc parameters
    assert_eq!(config.poll_interval_ms, 500);
    assert_eq!(config.publish_events, true);
    assert_eq!(config.skip_simulation, false);
    assert_eq!(config.infinite_approval, true);
    assert_eq!(config.min_publish_timeframe_ms, 5000);

    // Price feed config
    assert_eq!(config.price_feed_config.r#type, "binance");
    assert_eq!(config.price_feed_config.source, "https://api.binance.com/api/v3");
    assert_eq!(config.price_feed_config.reverse, false);
}

#[tokio::test]
async fn test_basic_endpoints() {
    // List of all config files to test
    let config_files = vec![
        "config/base.eth-usdc.toml",
        "config/mainnet.eth-usdc.toml",
        "config/mainnet.eth-wbtc.toml",
        "config/mainnet.usdc-dai.toml",
        "config/unichain.eth-usdc.toml",
    ];

    println!("\n🔌 Testing basic endpoints for all configs...\n");

    for config_path in config_files {
        println!("📄 Testing endpoints for: {}", config_path);

        let config = load_market_maker_config(config_path).expect("Failed to load config");
        println!("   Network: {} (Chain ID: {})", config.network_name, config.chain_id);

        // Test 1: Fetch block number
        let block_num = latest(config.rpc_url.clone()).await;
        assert!(block_num > 0, "Block number should be greater than 0 for {}, got: {}", config_path, block_num);
        println!("   ✓ Block number: {}", block_num);

        // Test 2: Fetch gas price
        let gas = gas_price(config.rpc_url.clone()).await;
        assert!(gas > 0, "Gas price should be greater than 0 for {}, got: {}", config_path, gas);
        println!("   ✓ Gas price: {} wei", gas);

        // Test 3: Fetch EIP-1559 fees
        match eip1559_fees(config.rpc_url.clone()).await {
            Ok(fees) => {
                assert!(fees.max_fee_per_gas > 0, "Max fee per gas should be greater than 0 for {}", config_path);
                assert!(fees.max_priority_fee_per_gas > 0, "Max priority fee per gas should be greater than 0 for {}", config_path);
                println!("   ✓ EIP-1559: max={}, priority={}", fees.max_fee_per_gas, fees.max_priority_fee_per_gas);
            }
            Err(_) => {
                println!("   ⚠ EIP-1559 not available");
            }
        }

        // Test 4: Test provider connection and chain ID
        let provider = create_provider(&config.rpc_url);
        let chain_id = provider.get_chain_id().await.expect("Failed to get chain ID");
        assert_eq!(chain_id, config.chain_id, "Chain ID mismatch for {}", config_path);
        println!("   ✓ Chain ID verified: {}", chain_id);

        // Test 5: Fetch Chainlink oracle price (if configured)
        if !config.gas_token_chainlink_price_feed.is_empty() {
            match chainlink(config.rpc_url.clone(), config.gas_token_chainlink_price_feed.clone()).await {
                Ok(price) => {
                    assert!(price > 0.0, "Oracle price should be greater than 0 for {}", config_path);
                    println!("   ✓ Chainlink oracle: ${:.2}", price);
                }
                Err(_) => {
                    println!("   ⚠ Chainlink oracle unavailable");
                }
            }
        } else {
            println!("   ⚠ No Chainlink oracle configured");
        }

        println!();
    }

    println!("✨ All endpoint tests completed!\n");
}
