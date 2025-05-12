use std::sync::Arc;

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::Http,
};
use reqwest::Client;

use crate::types::sol::IERC20;

/// Retrieve the latest block number
pub async fn latest(provider: String) -> u64 {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());
    provider.get_block_number().await.unwrap_or_default()
}

/// Get the balance of the owner for the specified tokens.
pub async fn balances(provider: &RootProvider<Http<Client>>, owner: String, tokens: Vec<String>) -> Result<Vec<u128>, String> {
    let mut balances = vec![];
    let client = Arc::new(provider);
    for t in tokens.iter() {
        let contract = IERC20::new(t.parse().unwrap(), client.clone());
        match contract.balanceOf(owner.parse().unwrap()).call().await {
            Ok(res) => {
                let balance = res.balance.to_string().parse::<u128>().unwrap_or_default();
                balances.push(balance);
            }
            Err(e) => {
                tracing::error!("Failed to get balance for {}: {:?}", t, e);
                balances.push(0);
            }
        }
    }
    Ok(balances)
}
