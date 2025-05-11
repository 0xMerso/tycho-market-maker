use std::collections::HashMap;
use std::str::FromStr;
use tycho_client::rpc::RPCClient;
use tycho_client::HttpRPCClient;
use tycho_common::dto::{PaginationParams, PaginationResponse, ResponseToken, TokensRequestBody};
use tycho_common::Bytes;
use tycho_simulation::evm::protocol::ekubo::state::EkuboState;
use tycho_simulation::evm::protocol::filters::{balancer_pool_filter, curve_pool_filter, uniswap_v4_pool_with_hook_filter};
use tycho_simulation::models::Token;

use tycho_simulation::evm::protocol::uniswap_v3::state::UniswapV3State;
use tycho_simulation::evm::protocol::uniswap_v4::state::UniswapV4State;

use tycho_simulation::evm::{
    engine_db::tycho_db::PreCachedDB,
    protocol::{uniswap_v2::state::UniswapV2State, vm::state::EVMPoolState},
    stream::ProtocolStreamBuilder,
};

use num_bigint::BigUint;

use crate::types::config::{EnvConfig, MarketMakerConfig};
use crate::types::tycho::{PsbConfig, TychoSupportedProtocol};

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

/// Get the default protocol stream builder
/// But any other configuration of ProtocolStreamBuilder can be used
pub async fn psb(mmc: MarketMakerConfig, key: String, psbc: PsbConfig, tokens: Vec<Token>) -> ProtocolStreamBuilder {
    let (_, _, chain) = crate::types::tycho::chain(mmc.network.clone()).expect("Invalid chain");
    let u4 = uniswap_v4_pool_with_hook_filter;
    let balancer = balancer_pool_filter;
    // let curve = curve_pool_filter;
    let filter = psbc.filter.clone();
    let mut hmt = HashMap::new();
    tokens.iter().for_each(|t| {
        hmt.insert(t.address.clone(), t.clone());
    });
    tracing::debug!("Tycho endpoint: {} and chain: {}", mmc.tycho_endpoint, chain);
    let mut psb = ProtocolStreamBuilder::new(&mmc.tycho_endpoint, chain)
        .exchange::<UniswapV2State>(TychoSupportedProtocol::UniswapV2.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV3State>(TychoSupportedProtocol::UniswapV3.to_string().as_str(), filter.clone(), None)
        .exchange::<UniswapV4State>(TychoSupportedProtocol::UniswapV4.to_string().as_str(), filter.clone(), Some(u4))
        .auth_key(Some(key.clone()))
        .skip_state_decode_failures(true)
        .set_tokens(hmt.clone()) // ALL Tokens
        .await;
    if mmc.network.as_str() == "ethereum" {
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

/// Get the tokens only for the configured strategy
pub async fn scope(config: MarketMakerConfig, env: EnvConfig) -> Vec<Token> {
    let atks = match tokens(config.clone(), env.clone()).await {
        Some(t) => t,
        None => {
            tracing::error!("Failed to get tokens");
            return vec![]
        }
    };
    let targets = vec![config.addr0.clone().to_lowercase(), config.addr1.clone().to_lowercase()];
    let tokens = atks
        .iter()
        .filter(|t| {
            let addr = t.address.to_string().to_lowercase();
            targets.iter().any(|target| target == &addr)
        })
        .cloned()
        .collect::<Vec<Token>>();
    tokens
}

/// Get the tokens from the Tycho API
/// Filters are hardcoded for now.
pub async fn specific(mmc: MarketMakerConfig, env: EnvConfig, addresses: Vec<String>) -> Option<Vec<Token>> {
    tracing::info!("Getting tokens for network {}", mmc.network);
    match HttpRPCClient::new(format!("https://{}", mmc.tycho_endpoint).as_str(), Some(&env.tycho_api_key.as_str())) {
        Ok(client) => {
            let addresses = addresses.iter().map(|a| Bytes::from_str(a.to_lowercase().as_str()).unwrap()).collect::<Vec<Bytes>>();
            let (chain, _, _) = chain(mmc.network.clone()).expect("Invalid chain");
            let req = TokensRequestBody {
                token_addresses: Some(addresses.clone()),
                min_quality: Some(100),
                traded_n_days_ago: None,
                chain,
                pagination: PaginationParams { page: 0, page_size: 500 as i64 },
            };
            match client.get_tokens(&req.clone()).await {
                Ok(result) => {
                    let tokens = sanitize(result.tokens);
                    Some(tokens)
                }
                Err(e) => {
                    tracing::error!("Failed to get tokens on network {}: {:?}", mmc.network, e.to_string());
                    return None
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create client: {:?}", e.to_string());
            None
        }
    }
}


/// Get the tokens from the Tycho API
/// Filters are hardcoded for now.
pub async fn tokens(mmc: MarketMakerConfig, env: EnvConfig) -> Option<Vec<Token>> {
    tracing::info!("Getting tokens for network {}", mmc.network);
    match HttpRPCClient::new(format!("https://{}", mmc.tycho_endpoint).as_str(), Some(&env.tycho_api_key.as_str())) {
        Ok(client) => {
            let time = std::time::SystemTime::now();
            let (chain, _, _) = chain(mmc.network.clone()).expect("Invalid chain");
            match client.get_all_tokens(chain, Some(100), Some(1), 500).await {
                Ok(result) => {
                    let tokens = sanitize(result);
                    let elasped = time.elapsed().unwrap_or_default().as_millis();
                    tracing::debug!("Took {:?} ms to get {} tokens on {}", elasped, tokens.len(), mmc.network);
                    Some(tokens)
                }
                Err(e) => {
                    tracing::error!("Failed to get tokens on network {}: {:?}", mmc.network, e.to_string());
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
    tokens.into_iter()
    .filter(|s| {
        // Ensure the symbol has no control characters and meets any other symbol criteria
        s.symbol.chars().all(|c| c.is_ascii_graphic()) && 
        !s.symbol.chars().any(|c| c.is_control()) &&
        // Check that the address looks valid (e.g., starts with "0x" and is the correct length)
        s.address.to_string().starts_with("0x")
    })
    .collect()
}

