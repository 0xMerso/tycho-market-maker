use std::collections::HashMap;
use std::str::FromStr;
use tycho_client::HttpRPCClient;
use tycho_client::rpc::RPCClient;
use tycho_common::Bytes;
use tycho_common::dto::{PaginationParams, ProtocolStateRequestBody, ResponseToken, TokensRequestBody, VersionParam};
use tycho_simulation::evm::protocol::ekubo::state::EkuboState;
use tycho_simulation::evm::protocol::filters::{balancer_pool_filter, uniswap_v4_pool_with_hook_filter};
use tycho_simulation::models::Token;

use tycho_simulation::evm::protocol::uniswap_v3::state::UniswapV3State;
use tycho_simulation::evm::protocol::uniswap_v4::state::UniswapV4State;

use tycho_simulation::evm::{
    engine_db::tycho_db::PreCachedDB,
    protocol::{uniswap_v2::state::UniswapV2State, vm::state::EVMPoolState},
    stream::ProtocolStreamBuilder,
};

use alloy_chains::NamedChain;
use num_bigint::BigUint;
use tycho_simulation::protocol::models::ProtocolComponent;

use crate::types::config::MarketMakerConfig;
use crate::types::tycho::{AmmType, PsbConfig, TychoSupportedProtocol};
use crate::utils::r#static::BASIS_POINT_DENO;

/// Due to library conflicts, we need to redefine the Chain type depending the use case, hence the following aliases.
pub type ChainCommon = tycho_common::dto::Chain;
pub type ChainSimCore = tycho_simulation::tycho_core::dto::Chain;
pub type ChainSimu = tycho_simulation::evm::tycho_models::Chain;

/// Return the chains types for a given network name
pub fn chain(name: String) -> Option<(ChainCommon, ChainSimCore, ChainSimu)> {
    match name.as_str() {
        "ethereum" => Some((ChainCommon::Ethereum, ChainSimCore::Ethereum, ChainSimu::Ethereum)),
        "base" => Some((ChainCommon::Base, ChainSimCore::Base, ChainSimu::Base)),
        "unichain" => Some((ChainCommon::Unichain, ChainSimCore::Unichain, ChainSimu::Unichain)),
        _ => {
            tracing::error!("Unknown chain: {}", name);
            None
        }
    }
}

/// Get the Alloy chain based on the network name
pub fn get_alloy_chain(network: String) -> Result<NamedChain, String> {
    match network.as_str() {
        "ethereum" => Ok(NamedChain::Mainnet),
        "base" => Ok(NamedChain::Base),
        "unichain" => Ok(NamedChain::Unichain),
        "arbitrum" => Ok(NamedChain::Arbitrum),
        _ => {
            tracing::error!("Unsupported network: {}", network);
            Err("Unsupported network".to_string())
        }
    }
}

/// Converts a native fee (as a hex string) into a byte vector representing fee in basis points.
/// The conversion depends on the protocol type:
/// - uniswap_v2_pool: fee is already in basis points (e.g., "0x1e" → 30)
/// - uniswap_v3_pool or uniswap_v4_pool: fee is stored on a 1e6 scale (so 3000 → 30 bps, i.e. divide by 100)
/// - curve: fee is stored on a pow10 scale (e.g., 4000000 becomes 4 bps, so divide by 1_000_000)
/// - balancer_v2_pool: fee is stored on a pow18 scale (e.g., 1*10^15 becomes 10 bps, so divide by 1e14)
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

// Just a helper function to print the component name in a custom way
pub fn cpname(cp: ProtocolComponent) -> String {
    let fee = amm_fee_to_bps(cp.clone());
    let addr: String = cp.id.to_string().chars().take(7).collect();
    format!("[{} {:>15} {:>3}]", addr, cp.protocol_system, fee)
}

/// Filter out invalid strings from a vector of strings, that are not ASCII
fn sanitize(input: Vec<ResponseToken>) -> Vec<Token> {
    let mut tokens = vec![];
    for t in input.iter() {
        let g = t.gas.first().unwrap_or(&Some(0u64)).unwrap_or_default();
        if t.symbol.len() >= 20 {
            continue; // Symbol has been mistaken for a contract address, possibly.
        }
        if let Ok(addr) = tycho_simulation::tycho_core::Bytes::from_str(t.address.clone().to_string().as_str()) {
            tokens.push(Token {
                address: addr,
                decimals: t.decimals as usize,
                symbol: t.symbol.clone(),
                gas: BigUint::from(g),
            });
        }
    }
    tokens
        .into_iter()
        .filter(|s| {
            // Ensure the symbol has no control characters and meets any other symbol criteria
            s.symbol.chars().all(|c| c.is_ascii_graphic()) && !s.symbol.chars().any(|c| c.is_control()) &&
        // Check that the address looks valid (e.g., starts with "0x" and is the correct length)
        s.address.to_string().starts_with("0x")
        })
        .collect()
}

/// Get the tokens only for the configured strategy
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

/// Get the tokens from the Tycho API
/// Filters are hardcoded for now.
pub async fn specific(mmc: MarketMakerConfig, key: Option<&str>, addresses: Vec<String>) -> Option<Vec<Token>> {
    tracing::info!("Getting tokens for network {}", mmc.network_name.as_str().to_string());

    let Ok(client) = HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), key) else {
        tracing::error!("Failed to create client");
        return None;
    };

    let addresses = addresses.iter().map(|a| Bytes::from_str(a.to_lowercase().as_str()).unwrap()).collect::<Vec<Bytes>>();
    let (chain, _, _) = chain(mmc.network_name.as_str().to_string()).expect("Invalid chain");
    let req = TokensRequestBody {
        token_addresses: Some(addresses.clone()),
        min_quality: Some(100),
        traded_n_days_ago: None,
        chain,
        pagination: PaginationParams { page: 0, page_size: 500_i64 },
    };

    match client.get_tokens(&req.clone()).await {
        Ok(result) => {
            let tokens = sanitize(result.tokens);
            Some(tokens)
        }
        Err(e) => {
            tracing::error!("Failed to get tokens on network {}: {:?}", mmc.network_name.as_str().to_string(), e.to_string());
            None
        }
    }
}

/// Get the tokens from the Tycho API
/// Filters are hardcoded for now.
pub async fn tokens(mmc: MarketMakerConfig, key: Option<&str>) -> Option<Vec<Token>> {
    tracing::info!("Getting tokens for network {}", mmc.network_name.as_str());

    let Ok(client) = HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), key) else {
        tracing::error!("Failed to create client");
        return None;
    };

    let start_time = std::time::SystemTime::now();
    let (chain, _, _) = chain(mmc.network_name.as_str().to_string()).expect("Invalid chain");

    match client.get_all_tokens(chain, Some(100), Some(1), 3000).await {
        Ok(result) => {
            let tokens = sanitize(result);
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

/// Get the default protocol stream builder
/// But any other configuration of ProtocolStreamBuilder can be used
pub async fn psb(mmc: MarketMakerConfig, key: String, psbc: PsbConfig, tokens: Vec<Token>) -> ProtocolStreamBuilder {
    let (_, _, chain) = crate::types::tycho::chain(mmc.network_name.clone().as_str().to_string()).expect("Invalid chain");
    let u4 = uniswap_v4_pool_with_hook_filter;
    let balancer = balancer_pool_filter;
    // let curve = curve_pool_filter;
    let filter = psbc.filter.clone();
    let mut hmt = HashMap::new();
    tokens.iter().for_each(|t| {
        hmt.insert(t.address.clone(), t.clone());
    });
    tracing::debug!("Tycho endpoint: {} and chain: {}", mmc.tycho_api, chain);
    let mut psb = ProtocolStreamBuilder::new(&mmc.tycho_api, chain)
        .exchange::<UniswapV2State>(TychoSupportedProtocol::UniswapV2.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV3State>(TychoSupportedProtocol::UniswapV3.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV4State>(TychoSupportedProtocol::UniswapV4.to_string().as_str(), filter.clone(), Some(u4))
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
            .exchange::<EVMPoolState<PreCachedDB>>(TychoSupportedProtocol::BalancerV2.to_string().as_str(), filter.clone(), Some(balancer))
        // .exchange::<EVMPoolState<PreCachedDB>>(TychoSupportedProtocol::Curve.to_string().as_str(), filter.clone(), Some(curve));
    }
    psb
}

/// Get the balances of the component in the specified protocol system.
/// Returns a HashMap of component addresses and their balances.
/// Balance is returned as a u128, with decimals.
pub async fn get_component_balances(mmc: MarketMakerConfig, cp: ProtocolComponent, key: String) -> Option<HashMap<String, u128>> {
    match HttpRPCClient::new(format!("https://{}", mmc.tycho_api).as_str(), Some(key.as_str())) {
        Ok(client) => {
            let (chain, _, _) = chain(mmc.network_name.clone().as_str().to_string()).expect("Invalid chain");
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
                    let attributes = response.states.clone().into_iter().map(|state| state.attributes.clone()).collect::<Vec<_>>();
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
