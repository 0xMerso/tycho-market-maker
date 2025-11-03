use alloy::providers::Provider;
use shd::maker::feed::chainlink;
use shd::types::config::load_market_maker_config;
use shd::utils::evm::{create_provider, eip1559_fees, gas_price, latest};

// Global list of all config files to test
static CONFIG_FILES: &[&str] = &[
    "config/mainnet.eth-usdc.toml",
    "config/unichain.eth-usdc.toml",
];

#[test]
fn test_parse_all_configs() {

    println!("\n🔍 Testing parsing of all config files...\n");

    for config_path in CONFIG_FILES {
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

        println!("  - Config parsed successfully");
        println!("     Network: {} (Chain ID: {})", config.network_name, config.chain_id);
        println!("     Pair: {} {}/{}", config.pair_tag, config.base_token, config.quote_token);
        println!("     Price Feed: {}", config.price_feed_config.r#type);
        println!("     Spread: {} bps (watch), {} bps (exec)", config.min_watch_spread_bps, config.min_executable_spread_bps);
        println!();
    }

    println!("✨ All configs parsed successfully!\n");
}

#[test]
fn test_validate_configs() {
    // Load and validate the reference config
    let config_path = "config/unichain.eth-usdc.toml";

    println!("\n🔍 Testing config validation for: {}\n", config_path);

    // Test loading the config
    let config = load_market_maker_config(config_path);

    // Assert the config loads successfully
    assert!(config.is_ok(), "Failed to parse config: {:?}", config.err());

    let config = config.unwrap();

    // The validation already ran during load_market_maker_config
    // So if we get here, the config is valid
    println!("  - Config loaded and validated successfully");

    // Test some key validations are working
    println!("\n  - Validation checks passed:");

    // Check spreads
    assert!(config.min_executable_spread_bps >= -50.0, "min_executable_spread_bps below -50 bps");
    println!("    ✓ Spread limits: {} bps (watch), {} bps (exec)", config.min_watch_spread_bps, config.min_executable_spread_bps);

    // Check inventory ratio
    assert!(config.max_inventory_ratio > 0.0 && config.max_inventory_ratio <= 1.0);
    println!("    ✓ Inventory ratio: {}%", config.max_inventory_ratio * 100.0);

    // Check gas limit
    assert!(config.tx_gas_limit <= 1_000_000, "Gas limit exceeds 1M");
    println!("    ✓ Gas limit: {}", config.tx_gas_limit);

    // Check addresses are valid format
    assert!(config.base_token_address.starts_with("0x") && config.base_token_address.len() == 42);
    assert!(config.quote_token_address.starts_with("0x") && config.quote_token_address.len() == 42);
    println!("    ✓ Token addresses are valid Ethereum addresses");

    // Check tokens are different
    assert_ne!(config.base_token_address.to_lowercase(), config.quote_token_address.to_lowercase());
    println!("    ✓ Base and quote tokens are different");

    // Display config summary
    println!("\n  📊 Config Summary:");
    println!("    Network: {} (Chain ID: {})", config.network_name, config.chain_id);
    println!("    Pair: {} {}/{}", config.pair_tag, config.base_token, config.quote_token);
    println!("    Price Feed: {} from {}", config.price_feed_config.r#type, config.price_feed_config.source);
    println!("    Skip Simulation: {}", config.skip_simulation);
    println!("    Publish Events: {}", config.publish_events);

    println!("\n✨ Config validation test completed!\n");
}

#[tokio::test]
async fn test_basic_endpoints() {
    println!("\n🔌 Testing basic endpoints for all configs...\n");

    for config_path in CONFIG_FILES {
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
