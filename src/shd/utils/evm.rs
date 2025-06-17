use std::sync::Arc;

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider, utils::Eip1559Estimation},
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
        Ok(fees) => {
            // tracing::debug!("EIP-1559 Fees: {:?}", fees);
            Ok(fees)
        }
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
