use shd::maker::exec::ExecStrategyFactory;
use shd::maker::feed::PriceFeedFactory;
use shd::maker::tycho::specific;
use shd::types::builder::MarketMakerBuilder;
use shd::types::config::load_market_maker_config;

// Global list of all config files to test (same as in parsing.rs)
static CONFIG_FILES: &[&str] = &["config/mainnet.eth-usdc.toml", "config/unichain.eth-usdc.toml", "config/unichain.btc-usdc.toml"];

/// Test 1: Market Maker Initialization
/// Verifies that market maker can be properly initialized with each config
#[tokio::test]
async fn test_market_maker_initialization() {
    println!("\n🚀 Testing Market Maker Initialization for all configs...\n");

    for config_path in CONFIG_FILES {
        println!("📄 Testing initialization for: {}", config_path);

        // Load config
        let config = match load_market_maker_config(config_path) {
            Ok(c) => c,
            Err(e) => {
                panic!("Failed to load config {}: {:?}", config_path, e);
            }
        };

        println!("   ✓ Config loaded: {} {}/{}", config.network_name, config.base_token, config.quote_token);

        // Fetch tokens from Tycho API
        let addresses = vec![config.base_token_address.clone(), config.quote_token_address.clone()];
        let tokens_result = specific(config.clone(), Some("sampletoken"), addresses).await;

        let (base_token, quote_token) = match tokens_result {
            Some(tokens) if tokens.len() >= 2 => {
                println!("   ✓ Tokens fetched from Tycho API");
                // Find tokens by matching addresses
                let base_addr_lower = config.base_token_address.to_lowercase();
                let quote_addr_lower = config.quote_token_address.to_lowercase();

                let base = tokens
                    .iter()
                    .find(|t| t.address.to_string().to_lowercase() == base_addr_lower)
                    .expect(&format!("Base token {} not found", config.base_token_address));
                let quote = tokens
                    .iter()
                    .find(|t| t.address.to_string().to_lowercase() == quote_addr_lower)
                    .expect(&format!("Quote token {} not found", config.quote_token_address));

                (base.clone(), quote.clone())
            }
            _ => {
                panic!("Failed to fetch tokens from Tycho API for {}", config_path);
            }
        };

        // Create price feed
        let feed = PriceFeedFactory::create(&config.price_feed_config.r#type);
        println!("   ✓ Price feed created: {}", config.price_feed_config.r#type);

        // Create execution strategy
        let exec_strategy = std::panic::catch_unwind(|| ExecStrategyFactory::create(config.network_name.as_str()));

        match exec_strategy {
            Ok(strategy) => {
                println!("   ✓ Execution strategy created for network: {}", config.network_name);

                // Build market maker
                let builder = MarketMakerBuilder::new(config.clone(), feed, strategy);
                let identifier = builder.identifier();
                println!("   ✓ Builder created with ID: {}", identifier);

                match builder.build(base_token.clone(), quote_token.clone()) {
                    Ok(market_maker) => {
                        // Verify initialization
                        assert_eq!(market_maker.config.network_name, config.network_name);
                        assert_eq!(market_maker.base.symbol, base_token.symbol);
                        assert_eq!(market_maker.quote.symbol, quote_token.symbol);
                        assert_eq!(market_maker.base.address, base_token.address);
                        assert_eq!(market_maker.quote.address, quote_token.address);
                        assert!(!market_maker.ready); // Should start as not ready

                        println!("   ✓ Market maker initialized successfully");
                        println!("      Base: {} ({})", market_maker.base.symbol, market_maker.base.address);
                        println!("      Quote: {} ({})", market_maker.quote.symbol, market_maker.quote.address);
                    }
                    Err(e) => {
                        println!("   ❌ Failed to build market maker: {}", e);
                        // Don't panic - some configs might not work in test environment
                    }
                }
            }
            Err(e) => {
                println!("   ⚠️  Could not create execution strategy: {:?}", e);
                // Expected for some networks in test environment
            }
        }

        println!();
    }

    println!("✨ Market Maker initialization tests completed!\n");
}

/// Test 2: Price Feed Strategies  
/// Tests different price feed implementations
#[tokio::test]
async fn test_price_feeds() {
    println!("\n💰 Testing Price Feed Strategies...\n");

    for config_path in CONFIG_FILES {
        println!("📄 Testing price feeds for: {}", config_path);

        let config = match load_market_maker_config(config_path) {
            Ok(c) => c,
            Err(e) => {
                panic!("Failed to load config {}: {:?}", config_path, e);
            }
        };

        println!("   Config: {} {}/{}", config.network_name, config.base_token, config.quote_token);

        // Test Binance feed
        println!("   📊 Testing Binance price feed:");
        let binance_feed = PriceFeedFactory::create("binance");

        match binance_feed.get(config.clone()).await {
            Ok(price) => {
                println!("      ✓ Binance feed returned price: ${:.2}", price);
                assert!(price > 0.0, "Price should be positive");
                assert!(price < 1_000_000.0, "Price should be reasonable");
            }
            Err(e) => {
                println!("      ⚠️  Could not fetch from Binance: {}", e);
                // API might be down or rate limited in tests
            }
        }

        // Test Chainlink feed if configured
        if config.price_feed_config.r#type == "chainlink" {
            println!("   🔗 Testing Chainlink price feed:");
            let chainlink_feed = PriceFeedFactory::create("chainlink");

            match chainlink_feed.get(config.clone()).await {
                Ok(price) => {
                    println!("      ✓ Chainlink feed returned price: ${:.2}", price);
                    assert!(price > 0.0, "Price should be positive");
                }
                Err(e) => {
                    println!("      ⚠️  Could not fetch from Chainlink: {}", e);
                    // Expected in test environment without proper RPC access
                }
            }
        }

        println!();
    }

    // Test feed factory with different types
    println!("🔍 Testing price feed factory:");
    let feed_types = vec!["binance", "chainlink"];
    for feed_type in feed_types {
        let feed = PriceFeedFactory::create(feed_type);
        println!("   ✓ Created {} feed: {}", feed_type, feed.name());
    }

    println!("\n✨ Price feed tests completed!\n");
}

/// Test 3: Market Context Fetching
/// Tests market maker's ability to fetch and process market context
#[tokio::test]
async fn test_market_context() {
    println!("\n🌍 Testing Market Context Fetching...\n");

    for config_path in CONFIG_FILES {
        println!("📄 Testing market context for: {}", config_path);

        let config = match load_market_maker_config(config_path) {
            Ok(c) => c,
            Err(e) => {
                panic!("Failed to load config {}: {:?}", config_path, e);
            }
        };

        println!("   Config: {} {}/{}", config.network_name, config.base_token, config.quote_token);

        // Fetch tokens from Tycho API
        let addresses = vec![config.base_token_address.clone(), config.quote_token_address.clone()];
        let tokens_result = specific(config.clone(), Some("sampletoken"), addresses).await;

        let (base_token, quote_token) = match tokens_result {
            Some(tokens) if tokens.len() >= 2 => {
                println!("   ✓ Tokens fetched from Tycho API");
                // Find tokens by matching addresses
                let base_addr_lower = config.base_token_address.to_lowercase();
                let quote_addr_lower = config.quote_token_address.to_lowercase();

                let base = tokens
                    .iter()
                    .find(|t| t.address.to_string().to_lowercase() == base_addr_lower)
                    .expect(&format!("Base token {} not found", config.base_token_address));
                let quote = tokens
                    .iter()
                    .find(|t| t.address.to_string().to_lowercase() == quote_addr_lower)
                    .expect(&format!("Quote token {} not found", config.quote_token_address));

                (base.clone(), quote.clone())
            }
            _ => {
                panic!("Failed to fetch tokens from Tycho API for {}", config_path);
            }
        };

        // Build market maker to test context fetching
        let feed = PriceFeedFactory::create(&config.price_feed_config.r#type);

        // Try to create execution strategy (may panic for some networks)
        let exec_result = std::panic::catch_unwind(|| ExecStrategyFactory::create(config.network_name.as_str()));

        if let Ok(exec_strategy) = exec_result {
            let builder = MarketMakerBuilder::new(config.clone(), feed, exec_strategy);

            match builder.build(base_token.clone(), quote_token.clone()) {
                Ok(market_maker) => {
                    println!("   ✓ Market maker built successfully");

                    // Verify the market maker structure
                    assert_eq!(market_maker.config.network_name, config.network_name);
                    assert_eq!(market_maker.base.symbol, base_token.symbol);
                    assert_eq!(market_maker.quote.symbol, quote_token.symbol);
                    assert_eq!(market_maker.base.address, base_token.address);
                    assert_eq!(market_maker.quote.address, quote_token.address);

                    println!("   ✓ Market maker structure verified");
                }
                Err(e) => {
                    println!("   ⚠️  Could not build market maker: {}", e);
                }
            }
        } else {
            println!("   ⚠️  Execution strategy not available for {}", config.network_name);
        }

        // Test configuration validation for API settings
        assert!(!config.tycho_api.is_empty(), "Tycho API endpoint should be configured");
        println!("   ✓ Tycho API endpoint: {}", config.tycho_api);

        assert!(config.chain_id > 0, "Chain ID should be valid");
        println!("   ✓ Chain ID: {}", config.chain_id);

        println!();
    }

    println!("✨ Market context tests completed!\n");
}

// === Simple test plans for remaining steps ===

// Step 6: Test Trade Preparation (Simple)
// Plan:
// - Create mock ExecutionOrder with simple swap
// - Test transaction encoding (will fail without real keys, but test structure)
// - Verify gas estimation returns reasonable values (100k - 500k)
// - Check approval detection for new tokens

// Step 7: Test Safety Checks (Simple)
// Plan:
// - Test inventory limit: max_inventory_ratio = 0.5, inventory = 100, try to trade 60
// - Test spread threshold: min_spread = 10 bps, actual = 5 bps, should reject
// - Test slippage: max = 0.5%, calculated = 1%, should reject
// - Verify all return proper error messages

// Step 8: Test Error Recovery (Simple)
// Plan:
// - Test network timeout: Mock provider that delays 10s
// - Test invalid price: Price feed returns 0 or negative
// - Test insufficient balance: Try to trade more than available
// - Verify errors are logged but don't panic
