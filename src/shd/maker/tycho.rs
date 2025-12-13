//! Tycho Protocol Integration Module
//!
//! Integration layer for Tycho protocol providing market data streaming,
//! protocol state management, and token pair discovery. Handles communication with
//! Tycho RPC endpoints and manages protocol component streams.
use std::collections::HashMap;
use std::str::FromStr;
use tycho_client::rpc::RPCClient;
use tycho_client::HttpRPCClient;
use tycho_common::dto::{PaginationParams, ProtocolStateRequestBody, ResponseToken, TokensRequestBody, VersionParam};
use tycho_common::models::token::Token;
use tycho_common::Bytes;
use tycho_simulation::evm::engine_db::tycho_db::PreCachedDB;
use tycho_simulation::evm::protocol::ekubo::state::EkuboState;
use tycho_simulation::evm::protocol::filters::{balancer_v2_pool_filter, curve_pool_filter};
use tycho_simulation::evm::protocol::uniswap_v3::state::UniswapV3State;
use tycho_simulation::evm::protocol::uniswap_v4::state::UniswapV4State;
use tycho_simulation::evm::protocol::vm::state::EVMPoolState;

use tycho_simulation::evm::{protocol::uniswap_v2::state::UniswapV2State, stream::ProtocolStreamBuilder};

use alloy_chains::NamedChain;
use tycho_simulation::protocol::models::ProtocolComponent;

use crate::types::config::MarketMakerConfig;
use crate::types::tycho::{AmmType, PsbConfig, TychoSupportedProtocol};
use crate::utils::constants::BASIS_POINT_DENO;

/// Chain type aliases to resolve library conflicts between different Tycho modules.
pub type ChainCommon = tycho_common::dto::Chain;
pub type ChainSimu = tycho_simulation::evm::tycho_models::Chain;

/// Maps network name to corresponding chain type tuples from different libraries.
/// Returns None if network is unsupported.
pub fn chain(name: String) -> Option<(ChainCommon, ChainSimu)> {
    match name.as_str() {
        "ethereum" => Some((ChainCommon::Ethereum, ChainSimu::Ethereum)),
        "base" => Some((ChainCommon::Base, ChainSimu::Base)),
        "unichain" => Some((ChainCommon::Unichain, ChainSimu::Unichain)),
        _ => {
            tracing::error!("Unknown chain: {}", name);
            None
        }
    }
}

/// Converts network name to Alloy's NamedChain enum.
/// Returns error for unsupported networks.
pub fn get_alloy_chain(network: String) -> Result<NamedChain, String> {
    match network.as_str() {
        "ethereum" => Ok(NamedChain::Mainnet),
        "base" => Ok(NamedChain::Base),
        "unichain" => Ok(NamedChain::Unichain),
        _ => {
            tracing::error!("Unsupported network: {}", network);
            Err("Unsupported network".to_string())
        }
    }
}

/// Converts AMM protocol fees to basis points based on protocol type.
/// Extracts fee from static_attributes and converts using protocol-specific scaling.
pub fn amm_fee_to_bps(cp: ProtocolComponent) -> u128 {
    let value = cp
        .static_attributes
        .iter()
        .find(|(k, _)| *k == "key_lp_fee" || *k == "fee")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    let fee = value.trim_start_matches("0x");
    let fee = u128::from_str_radix(fee, 16).unwrap_or(0);

    match AmmType::from(cp.protocol_type_name.as_str()) {
        AmmType::PancakeswapV2 | AmmType::Sushiswap | AmmType::UniswapV2 => fee, // Already in bps
        AmmType::PancakeswapV3 | AmmType::UniswapV3 | AmmType::UniswapV4 => fee * (BASIS_POINT_DENO as u128) / 1_000_000,
        AmmType::Curve => 4,   // Not implemented, assuming 4 bps by default
        AmmType::EkuboV2 => 0, // Not implemented, assuming 0 bps by default
        AmmType::Balancer => (fee * (BASIS_POINT_DENO as u128)) / 1e18 as u128,
    }
}

/// Formats protocol component information for readable display.
/// Returns formatted string with truncated ID, protocol system, and fee in bps.
pub fn cpname(cp: ProtocolComponent) -> String {
    let fee = amm_fee_to_bps(cp.clone());
    let addr: String = cp.id.to_string().chars().take(7).collect();
    format!("[{} {:>15} {:>3}]", addr, cp.protocol_system, fee)
}

/// Filters and converts ResponseToken array to valid Token array.
///
/// Removes tokens with invalid symbols, addresses, or control characters.
fn sanitize(input: Vec<ResponseToken>, chain: ChainCommon) -> Vec<Token> {
    let mut tokens = vec![];
    for t in input.iter() {
        let g = t.gas.first().unwrap_or(&Some(0u64)).unwrap_or_default();
        if g == 0 {
            tracing::debug!("Skipping token with 0 gas: {} ({})", t.symbol, t.address);
            continue;
        }
        if let Ok(addr) = tycho_simulation::tycho_core::Bytes::from_str(t.address.clone().to_string().as_str()) {
            tokens.push(Token {
                address: addr,
                decimals: t.decimals, // Now u32, not usize
                symbol: t.symbol.clone(),
                // CONSERVATIVE: Token.gas changed from BigUint to Vec<Option<u64>> in tycho-common 0.96.1
                gas: vec![Some(g)], // Wrap in Vec for new API
                // FIXED: Extract chain from network instead of hardcoding Ethereum
                chain: chain.into(), // Use actual network chain
                quality: 100,        // High quality since we filter by quality
                tax: 0,              // Assume no tax by default
            });
        }
    }
    tokens
        .into_iter()
        .filter(|s| {
            let addr = s.address.to_string();
            // Ensure the symbol has no control characters
            let valid_symbol = s.symbol.chars().all(|c| c.is_ascii_graphic()) && !s.symbol.chars().any(|c| c.is_control());
            // Check that the address is valid: starts with "0x" and is exactly 42 chars (0x + 40 hex)
            let valid_address = addr.starts_with("0x");

            if !valid_symbol {
                tracing::debug!("Excluding token with invalid symbol: {} ({})", s.symbol, addr);
            }
            if !valid_address {
                tracing::debug!("Excluding token with invalid address: {} (len={})", addr, addr.len());
            }

            valid_symbol && valid_address
        })
        .collect()
}

/// Fetches only the base and quote tokens configured for the market maker.
/// Retrieves all tokens and filters to only base and quote tokens.
pub async fn scope(config: MarketMakerConfig, key: Option<&str>) -> Vec<Token> {
    let Some(atks) = tokens(config.clone(), key).await else {
        tracing::error!("Failed to get tokens");
        return vec![];
    };

    let targets = [config.base_token_address.clone().to_lowercase(), config.quote_token_address.clone().to_lowercase()];

    atks.iter()
        .filter(|t| {
            let addr = t.address.to_string().to_lowercase();
            targets.iter().any(|target| target == &addr)
        })
        .cloned()
        .collect::<Vec<Token>>()
}

/// Fetches specific tokens by their addresses from Tycho API.
/// Queries Tycho API for specific tokens with quality filter of 100.
pub async fn specific(mmc: MarketMakerConfig, key: Option<&str>, addresses: Vec<String>) -> Option<Vec<Token>> {
    tracing::info!("Getting specific tokens for network {}", mmc.network_name.as_str().to_string());

    let Ok(client) = HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), key) else {
        tracing::error!("Failed to create client");
        return None;
    };

    let addresses = addresses.iter().map(|a| Bytes::from_str(a.to_lowercase().as_str()).unwrap()).collect::<Vec<Bytes>>();
    let (chain, _) = chain(mmc.network_name.as_str().to_string()).expect("Invalid chain");
    let req = TokensRequestBody {
        token_addresses: Some(addresses.clone()),
        min_quality: Some(100),
        traded_n_days_ago: None,
        chain,
        pagination: PaginationParams { page: 0, page_size: 500_i64 },
    };

    match client.get_tokens(&req.clone()).await {
        Ok(result) => {
            let tokens = sanitize(result.tokens, chain); // Pass chain to sanitize
            Some(tokens)
        }
        Err(e) => {
            tracing::error!("Failed to get tokens on network {}: {:?}", mmc.network_name.as_str().to_string(), e.to_string());
            None
        }
    }
}

/// Fetches all available tokens from Tycho API for a network.
/// Retrieves all tokens with quality >= 100, traded in last 7 days, max 3000 tokens.
pub async fn tokens(mmc: MarketMakerConfig, key: Option<&str>) -> Option<Vec<Token>> {
    tracing::info!("Getting tokens for network {}", mmc.network_name.as_str());

    let Ok(client) = HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), key) else {
        tracing::error!("Failed to create client");
        return None;
    };

    let start_time = std::time::SystemTime::now();
    let (chain, _) = chain(mmc.network_name.as_str().to_string()).expect("Invalid chain");

    match client.get_all_tokens(chain, Some(100), Some(7), 3000).await {
        Ok(result) => {
            let tokens = sanitize(result, chain); // Pass chain to sanitize
            let elapsed = start_time.elapsed().unwrap_or_default().as_millis();
            tracing::info!("Got {} tokens in {} ms", tokens.len(), elapsed);
            Some(tokens)
        }
        Err(e) => {
            tracing::error!("Failed to get tokens on network {}: {:?}", mmc.network_name.as_str().to_string(), e.to_string());
            None
        }
    }
}

/// Creates and configures a ProtocolStreamBuilder for streaming AMM updates.
/// Sets up stream for UniswapV2, V3, V4 protocols with provided filters.
pub async fn psb(mmc: MarketMakerConfig, key: String, psbc: PsbConfig, tokens: Vec<Token>) -> ProtocolStreamBuilder {
    let (_, chain) = crate::types::tycho::chain(mmc.network_name.clone().as_str().to_string()).expect("Invalid chain");
    let filter = psbc.filter.clone();
    let mut hmt = HashMap::new();
    tokens.iter().for_each(|t| {
        hmt.insert(t.address.clone(), t.clone());
    });
    tracing::debug!("Tycho endpoint: {} and chain: {}", mmc.tycho_api, chain);
    let mut psb = ProtocolStreamBuilder::new(&mmc.tycho_api, chain)
        .exchange::<UniswapV2State>(TychoSupportedProtocol::UniswapV2.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV3State>(TychoSupportedProtocol::UniswapV3.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV4State>(TychoSupportedProtocol::UniswapV4.to_string().as_str(), filter.clone(), None) // Some(u4))
        .auth_key(Some(key.clone()))
        .skip_state_decode_failures(true)
        .set_tokens(hmt.clone()) // ALL Tokens
        .await;

    if mmc.network_name.as_str() == "ethereum" {
        tracing::trace!("Adding mainnet-specific exchanges");
        psb = psb
            .exchange::<UniswapV2State>(TychoSupportedProtocol::Sushiswap.to_string().as_str(), filter.clone(), None)
            .exchange::<UniswapV2State>(TychoSupportedProtocol::PancakeswapV2.to_string().as_str(), filter.clone(), None)
            .exchange::<UniswapV3State>(TychoSupportedProtocol::PancakeswapV3.to_string().as_str(), filter.clone(), None)
            .exchange::<EkuboState>(TychoSupportedProtocol::EkuboV2.to_string().as_str(), filter.clone(), None)
            .exchange::<EVMPoolState<PreCachedDB>>(TychoSupportedProtocol::BalancerV2.to_string().as_str(), filter.clone(), Some(balancer_v2_pool_filter))
            .exchange::<EVMPoolState<PreCachedDB>>(TychoSupportedProtocol::Curve.to_string().as_str(), filter.clone(), Some(curve_pool_filter));
    }

    psb
}

/// Fetches token balances for a specific protocol component (pool).
/// Queries protocol state with balances and returns HashMap of address->balance.
pub async fn get_component_balances(mmc: MarketMakerConfig, cp: ProtocolComponent, key: String) -> Option<HashMap<String, u128>> {
    match HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), Some(key.as_str())) {
        Ok(client) => {
            let (chain, _) = chain(mmc.network_name.clone().as_str().to_string()).expect("Invalid chain");
            let body = ProtocolStateRequestBody {
                protocol_ids: Some(vec![cp.id.to_string().to_lowercase().clone()]),
                protocol_system: cp.protocol_system, // Single, so cannot use protocol_ids vec of different protocols ?
                chain,
                include_balances: true,           // We want to include account balances.
                version: VersionParam::default(), // { timestamp: None, block: None },
                pagination: PaginationParams {
                    page: 0,        // Start at the first page.
                    page_size: 100, // Maximum page size supported is 100.
                },
            };

            match client.get_protocol_states(&body).await {
                Ok(response) => {
                    let _attributes = response.states.clone().into_iter().map(|state| state.attributes.clone()).collect::<Vec<_>>();
                    // for attribute in attributes.iter() {
                    //     for a in attribute.iter() {
                    //         tracing::debug!(" - Attribute key: {:?}", a.0);
                    //     }
                    // }
                    let component_balances = response.states.into_iter().map(|state| state.balances.clone()).collect::<Vec<_>>();
                    let mut result = HashMap::new();
                    for cb in component_balances.iter() {
                        for c in cb.iter() {
                            let b = u128::from_str_radix(c.1.to_string().trim_start_matches("0x"), 16);
                            if let Ok(b) = b {
                                result.insert(c.0.clone().to_string().to_lowercase(), b);
                            }
                        }
                    }
                    Some(result)
                }
                Err(e) => {
                    tracing::error!("Failed to get protocol states: {}: {:?}", cp.id.to_string().clone(), e.to_string());
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create client: {:?}", e.to_string());
            None
        }
    }
}
