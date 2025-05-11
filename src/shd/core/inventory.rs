use std::{env, sync::Arc};

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::Http,
};
use reqwest::Client;

use crate::{
    types::{
        config::{EnvConfig, MarketMakerConfig},
        sol::IERC20,
    },
    utils::evm::erc20b,
};

/// Initialize the wallet by checking the balances of the tokens, nonce, etc.
pub async fn wallet(config: MarketMakerConfig, env: EnvConfig) {
    let provider = ProviderBuilder::new().on_http(config.rpc.clone().parse().expect("Failed to parse RPC_URL"));
    let tokens = vec![config.addr0.clone(), config.addr1.clone()];
    match erc20b(&provider, env.sender.clone(), tokens.clone()).await {
        Ok(balances) => {
            tracing::debug!("Balances of sender {}: {:?}", env.sender.clone(), balances);
        }
        Err(e) => {
            tracing::error!("Failed to get balances of sender: {:?}", e);
        }
    };
    // let header = provider.get_block_by_number(alloy::eips::BlockNumberOrTag::Latest, false).await.unwrap().unwrap();
    let nonce = provider.get_transaction_count(env.sender.to_string().parse().unwrap()).await.unwrap();
    tracing::debug!("Nonce of sender {}: {}", env.sender.clone(), nonce);
}
