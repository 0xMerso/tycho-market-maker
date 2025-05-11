use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tycho_simulation::tycho_core::Bytes;
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
    tycho_client::feed::component_tracker::ComponentFilter,
};

#[derive(Clone)]
pub struct PsbConfig {
    pub filter: ComponentFilter,
}

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

use std::{collections::HashMap, str::FromStr, sync::Arc};
use strum::VariantNames;
use strum_macros::{Display, EnumString};

#[derive(Display, VariantNames, EnumString)]
pub enum TychoSupportedProtocol {
    #[strum(serialize = "pancakeswap_v2")]
    PancakeswapV2,
    #[strum(serialize = "pancakeswap_v3")]
    PancakeswapV3,
    #[strum(serialize = "sushiswap_v2")]
    Sushiswap,
    #[strum(serialize = "uniswap_v2")]
    UniswapV2,
    #[strum(serialize = "uniswap_v3")]
    UniswapV3,
    #[strum(serialize = "uniswap_v4")]
    UniswapV4,
    #[strum(serialize = "ekubo_v2")]
    EkuboV2,
    #[strum(serialize = "vm:balancer_v2")]
    BalancerV2,
    #[strum(serialize = "vm:curve")]
    Curve,
}

impl TychoSupportedProtocol {
    pub fn vectorize() -> Vec<String> {
        TychoSupportedProtocol::VARIANTS
            .iter()
            .filter_map(|&variant| TychoSupportedProtocol::from_str(variant).ok())
            .map(|v| v.to_string())
            .collect()
    }
}

#[derive(Display, EnumString)]
pub enum AmmType {
    #[strum(serialize = "pancakeswap_v2_pool")]
    PancakeswapV2,
    #[strum(serialize = "pancakeswap_v3_pool")]
    PancakeswapV3,
    #[strum(serialize = "sushiswap_v2_pool")]
    Sushiswap,
    #[strum(serialize = "uniswap_v2_pool")]
    UniswapV2,
    #[strum(serialize = "uniswap_v3_pool")]
    UniswapV3,
    #[strum(serialize = "uniswap_v4_pool")]
    UniswapV4,
    #[strum(serialize = "ekubo_v2_pool")]
    EkuboV2,
    #[strum(serialize = "balancer_v2_pool")]
    Balancer,
    #[strum(serialize = "curve_pool")]
    Curve,
}

pub type SharedTychoStreamState = Arc<RwLock<TychoStreamState>>;

/// Tycho Stream Data, stored in a Mutex/Arc for shared access between the SDK stream and the client or API.
pub struct TychoStreamState {
    // ProtocolSim instances, indexed by their unique identifier. Impossible to store elsewhere than memory
    pub protosims: HashMap<String, Box<dyn ProtocolSim>>,
    // Components instances, indexed by their unique identifier. Serialised and stored in Redis
    pub components: HashMap<String, ProtocolComponent>,
    // Tokens from Tycho Client
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrzToken {
    pub address: String,
    pub decimals: usize,
    pub symbol: String,
    pub gas: String,
}

impl From<Token> for SrzToken {
    fn from(token: Token) -> Self {
        SrzToken {
            address: token.address.to_string(),
            decimals: token.decimals,
            symbol: token.symbol,
            gas: token.gas.to_string(), // Convert BigUint to String
        }
    }
}

impl From<SrzToken> for Token {
    fn from(serialized: SrzToken) -> Self {
        Token {
            address: Bytes::from_str(serialized.address.to_lowercase().as_str()).unwrap(),
            decimals: serialized.decimals,
            symbol: serialized.symbol,
            gas: BigUint::parse_bytes(serialized.gas.as_bytes(), 10).expect("Failed to parse BigUint"), // Convert String back to BigUint
        }
    }
}
