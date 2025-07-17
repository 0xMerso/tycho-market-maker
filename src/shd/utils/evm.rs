use std::sync::Arc;

use alloy::{
    providers::{utils::Eip1559Estimation, Provider, ProviderBuilder, RootProvider},
    transports::http::Http,
};
use reqwest::Client;

use crate::types::sol::IERC20;

/// =============================================================================
/// EVM Blockchain Utilities
/// =============================================================================
///
/// @description: Collection of utility functions for interacting with EVM-compatible
/// blockchains including Ethereum, Base, and other L2 networks
/// =============================================================================

/// =============================================================================
/// @function: latest
/// @description: Retrieve the latest block number from the specified RPC endpoint
/// @param provider: RPC endpoint URL as string
/// @return u64: Latest block number (returns 0 if failed)
/// =============================================================================
pub async fn latest(provider: String) -> u64 {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());
    provider.get_block_number().await.unwrap_or_default()
}

/// =============================================================================
/// @function: gas_price
/// @description: Retrieve the current gas price from the specified RPC endpoint
/// @param provider: RPC endpoint URL as string
/// @return u128: Current gas price in wei (returns 0 if failed)
/// =============================================================================
pub async fn gas_price(provider: String) -> u128 {
    let provider = ProviderBuilder::new().on_http(provider.parse().unwrap());
    provider.get_gas_price().await.unwrap_or_default()
}

/// =============================================================================
/// @function: eip1559_fees
/// @description: Estimate EIP-1559 gas fees (max fee and priority fee) for the network
/// @param provider: RPC endpoint URL as string
/// @return Result<Eip1559Estimation, String>: EIP-1559 fee estimation or error
/// =============================================================================
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

/// =============================================================================
/// @function: balances
/// @description: Get token balances for a specific owner address across multiple tokens
/// @param provider: Alloy provider instance
/// @param owner: Owner address as string
/// @param tokens: Vector of token contract addresses
/// @return Result<Vec<u128>, String>: Vector of token balances in wei or error
/// =============================================================================
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

/// =============================================================================
/// @function: allowance
/// @description: Get the allowance amount for a specific token between owner and spender
/// @param provider: Alloy provider instance
/// @param owner: Token owner address
/// @param spender: Spender address
/// @param token: Token contract address
/// @return Result<u128, String>: Allowance amount in wei or error
/// =============================================================================
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

/// =============================================================================
/// @function: wallet
/// @description: Initialize the wallet by checking token balances, nonce, and wallet state
/// @param config: Market maker configuration containing RPC URL and token addresses
/// @param _env: Environment configuration (unused but kept for future use)
/// @return: None
///
/// @behavior:
/// - Fetches balances for base and quote tokens
/// - Gets current nonce for transaction ordering
/// - Logs wallet state for debugging
/// =============================================================================
pub async fn wallet(config: MarketMakerConfig, _env: EnvConfig) {
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
