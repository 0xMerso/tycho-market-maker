use alloy::providers::{Provider, ProviderBuilder};

use crate::{
    types::config::{EnvConfig, MarketMakerConfig},
    utils::evm::balances,
};

/// Initialize the wallet by checking the balances of the tokens, nonce, etc.
pub async fn wallet(config: MarketMakerConfig, env: EnvConfig) {
    let provider = ProviderBuilder::new().on_http(config.rpc_url.clone().parse().expect("Failed to parse RPC_URL"));
    let tokens = vec![config.base_token_address.clone(), config.quote_token_address.clone()];
    match balances(&provider, env.wallet_public_key.clone(), tokens.clone()).await {
        Ok(balances) => {
            tracing::debug!("Balances of sender {}: {:?}", env.wallet_public_key.clone(), balances);
        }
        Err(e) => {
            tracing::error!("Failed to get balances of sender: {:?}", e);
        }
    };
    // let header = provider.get_block_by_number(alloy::eips::BlockNumberOrTag::Latest, false).await.unwrap().unwrap();
    let nonce = provider.get_transaction_count(env.wallet_public_key.to_string().parse().unwrap()).await.unwrap();
    tracing::debug!("Nonce of sender {}: {}", env.wallet_public_key.clone(), nonce);
}
