use std::sync::Arc;

use alloy::{
    providers::{utils::Eip1559Estimation, Provider, ProviderBuilder, RootProvider},
    transports::http::Http,
};
use reqwest::Client;

use crate::types::sol::IERC20;

/// Retrieve the latest block number
pub async fn latest(provider: String) -> u64 {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());
    provider.get_block_number().await.unwrap_or_default()
}

/// Used to retrieve gas price
pub async fn gas_price(provider: String) -> u128 {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());
    provider.get_gas_price().await.unwrap_or_default()
}

/// Used to retrieve gas price
pub async fn eip1559_fees(provider: String) -> Result<Eip1559Estimation, String> {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());

    match provider.estimate_eip1559_fees(None).await {
        Ok(fees) => Ok(fees),
        Err(e) => {
            tracing::error!("Failed to estimate EIP-1559 fees: {:?}", e);
            Err(format!("Failed to call estimate_eip1559_fees: {:?}", e))
        }
    }
}

/// Get the balance of the owner for the specified tokens.
pub async fn balances(provider: &RootProvider<Http<Client>>, owner: String, tokens: Vec<String>) -> Result<Vec<u128>, String> {
    let mut balances = vec![];
    let client = Arc::new(provider);

    for token in tokens {
        let contract = IERC20::new(token.parse().unwrap(), client.clone());

        match contract.balanceOf(owner.parse().unwrap()).call().await {
            Ok(res) => {
                let balance = res.balance.to_string().parse::<u128>().unwrap_or_default();
                balances.push(balance);
            }
            Err(e) => {
                tracing::error!("Failed to get balance for {}: {:?}", token, e);
                balances.push(0);
            }
        }
    }

    Ok(balances)
}

/// Get allowances
pub async fn allowance(provider: &RootProvider<Http<Client>>, owner: String, spender: String, token: String) -> Result<u128, String> {
    let client = Arc::new(provider);
    let contract = IERC20::new(token.parse().unwrap(), client.clone());
    match contract.allowance(owner.parse().unwrap(), spender.parse().unwrap()).call().await {
        Ok(allowance) => Ok(allowance._0.to_string().parse::<u128>().unwrap_or_default()),
        Err(e) => {
            tracing::error!("Failed to get allowance for {}: {:?}", token, e);
            Err(format!("Failed to get allowance for {}: {:?}", token, e))
        }
    }
}

use crate::types::config::{EnvConfig, MarketMakerConfig};

/// Initialize the wallet by checking the balances of the tokens, nonce, etc.
pub async fn wallet(config: MarketMakerConfig, env: EnvConfig) {
    let provider = ProviderBuilder::new().on_http(config.rpc_url.clone().parse().expect("Failed to parse RPC_URL"));

    let tokens = vec![config.base_token_address.clone(), config.quote_token_address.clone()];

    if let Ok(balances) = balances(&provider, config.wallet_public_key.clone(), tokens.clone()).await {
        tracing::debug!("Balances of sender {}: {:?}", config.wallet_public_key.clone(), balances);
    } else {
        tracing::error!("Failed to get balances of sender");
    }

    let nonce = provider.get_transaction_count(config.wallet_public_key.to_string().parse().unwrap()).await.unwrap();
    tracing::debug!("Nonce of sender {}: {}", config.wallet_public_key.clone(), nonce);
}
