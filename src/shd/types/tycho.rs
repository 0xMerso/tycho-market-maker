use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tycho_common::models::token::Token;
use tycho_common::simulation::protocol_sim::ProtocolSim; // ProtocolSim trait for protocol simulation
use tycho_simulation::tycho_core::Bytes;
use tycho_simulation::{
    protocol::models::ProtocolComponent,
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
use strum_macros::{Display, EnumString, VariantNames as VariantNamesMacro};

#[derive(Display, VariantNamesMacro, EnumString)]
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

#[derive(Display)]
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

impl From<&str> for AmmType {
    fn from(s: &str) -> Self {
        match s {
            "pancakeswap_v2_pool" => AmmType::PancakeswapV2,
            "pancakeswap_v3_pool" => AmmType::PancakeswapV3,
            "sushiswap_v2_pool" => AmmType::Sushiswap,
            "uniswap_v2_pool" => AmmType::UniswapV2,
            "uniswap_v3_pool" => AmmType::UniswapV3,
            "uniswap_v4_pool" => AmmType::UniswapV4,
            "balancer_v2_pool" => AmmType::Balancer,
            "curve_pool" => AmmType::Curve,      // ?
            "ekubo_v2_pool" => AmmType::EkuboV2, // ?
            _ => panic!("Unknown AMM type"),
        }
    }
}

pub type SharedTychoStreamState = Arc<RwLock<TychoStreamState>>;

/// Tycho Stream Data, stored in a Mutex/Arc for shared access between the SDK stream and the client or API.
pub struct TychoStreamState {
    // ProtocolSim instances, indexed by their unique identifier. Impossible to store elsewhere than memory
    pub protosims: HashMap<String, Box<dyn ProtocolSim>>,
    // Components instances, indexed by their unique identifier. Serialised and stored in Redis
    pub components: HashMap<String, ProtocolComponent>,
    // All tokens given Tycho, used to find path, price, etc.
    pub atks: Vec<Token>,
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
            decimals: token.decimals as usize, // Convert u32 to usize for serialization
            symbol: token.symbol,
            // CONSERVATIVE: Extract first gas value from Vec for serialization
            gas: token.gas.first().and_then(|g| *g).unwrap_or(0).to_string(),
        }
    }
}

impl From<SrzToken> for Token {
    fn from(serialized: SrzToken) -> Self {
        Token {
            address: Bytes::from_str(serialized.address.to_lowercase().as_str()).unwrap(),
            decimals: serialized.decimals as u32, // Convert usize to u32
            symbol: serialized.symbol,
            // CONSERVATIVE: Parse gas as u64 and wrap in Vec
            gas: vec![serialized.gas.parse::<u64>().ok()],
            // CONSERVATIVE DEFAULTS - New required fields (not Option):
            chain: tycho_common::dto::Chain::Ethereum.into(), // TODO: Serialize/deserialize chain
            quality: 100,    // High quality default
            tax: 0,          // Assume no tax
        }
    }
}

/// One component of the Tycho protocol, with his simulation instance
#[derive(Clone, Debug)]
pub struct ProtoSimComp {
    pub component: ProtocolComponent,
    pub protosim: Box<dyn ProtocolSim>,
}

#[derive(Clone, Debug)]
pub struct ValorisationPath {
    pub token_path: Vec<String>,
    pub comp_path: Vec<String>,
}
