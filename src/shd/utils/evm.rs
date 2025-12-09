use crate::types::config::{EnvConfig, MarketMakerConfig};
use std::{str::FromStr, sync::Arc};

use alloy::{
    providers::{utils::Eip1559Estimation, Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::{B256, U256};
use url;

use crate::types::sol::IERC20;

/// Creates an HTTP provider instance from RPC URL.
pub fn create_provider(rpc: &str) -> impl Provider {
    ProviderBuilder::new().connect_http(rpc.parse().expect("Failed to parse RPC URL"))
}

/// Retrieves the latest block number from the specified RPC endpoint.
pub async fn latest(provider: String) -> u64 {
    let provider = create_provider(&provider);
    provider.get_block_number().await.unwrap_or_default()
}

/// Retrieves the current gas price from the specified RPC endpoint.
pub async fn gas_price(provider: String) -> u128 {
    let provider = create_provider(&provider);
    provider.get_gas_price().await.unwrap_or_default()
}

/// Estimates EIP-1559 gas fees for the network.
pub async fn eip1559_fees(provider_url: String) -> Result<Eip1559Estimation, String> {
    let provider = create_provider(&provider_url);

    match provider.estimate_eip1559_fees().await {
        Ok(fees) => Ok(fees),
        Err(e) => {
            // Fallback: use legacy gas_price when eth_feeHistory isn't supported
            tracing::warn!("EIP-1559 estimation failed, falling back to legacy gas price: {:?}", e);
            match provider.get_gas_price().await {
                Ok(gas_price) => Ok(Eip1559Estimation {
                    max_fee_per_gas: gas_price,
                    max_priority_fee_per_gas: gas_price / 10, // ~10% tip
                }),
                Err(e2) => {
                    tracing::error!("Both EIP-1559 and legacy gas estimation failed: {:?}", e2);
                    Err(format!("Both EIP-1559 and legacy gas estimation failed: {:?}", e2))
                }
            }
        }
    }
}

/// Gets token balances for a specific owner address across multiple tokens.
pub async fn balances(provider: &impl Provider, owner: String, tokens: Vec<String>) -> Result<Vec<u128>, String> {
    let mut balances = vec![];
    let client = Arc::new(provider);

    for token in tokens {
        let contract = IERC20::new(token.parse().unwrap(), client.clone());

        match contract.balanceOf(owner.parse().unwrap()).call().await {
            Ok(res) => {
                // Alloy 1.0: balanceOf returns U256 directly, not wrapped in struct
                let balance = res.to_string().parse::<u128>().unwrap_or_default();
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

/// Gets the allowance amount for a specific token between owner and spender.
pub async fn allowance(rpc: String, owner: String, spender: String, token: String) -> Result<u128, String> {
    let provider = create_provider(&rpc);
    let client = Arc::new(provider);
    let contract = IERC20::new(token.parse().unwrap(), client.clone());
    match contract.allowance(owner.parse().unwrap(), spender.parse().unwrap()).call().await {
        Ok(allowance) => {
            // Alloy 1.0: allowance returns U256 directly, not wrapped
            Ok(allowance.to_string().parse::<u128>().unwrap_or_default())
        }
        Err(e) => {
            tracing::error!("Failed to get allowance for {}: {:?}", token, e);
            Err(format!("Failed to get allowance for {}: {:?}", token, e))
        }
    }
}

/// Approves a spender to spend a specific amount of tokens.
pub async fn approve(mmc: MarketMakerConfig, env: EnvConfig, spender: String, token: String, amount: u128) -> Result<TransactionReceipt, String> {
    let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let provider = ProviderBuilder::new().with_chain_id(mmc.chain_id).wallet(signer.clone()).connect_http(rpc.clone());
    let client = Arc::new(provider);
    let contract = IERC20::new(token.parse().unwrap(), client.clone());
    // Alloy 1.0: symbol() returns String directly, not wrapped
    let symbol = contract.symbol().call().await.expect("Failed to get symbol");
    let amount = U256::from(amount);
    tracing::info!("Approval: {} at address {} for spender {} and owner {}", symbol, token, spender, wallet.address().to_string());
    let native_gas_price = crate::utils::evm::eip1559_fees(mmc.rpc_url).await.expect("Failed to get native gas price");
    let nonce = client.get_transaction_count(wallet.address()).await.expect("Failed to get nonce");
    let call = contract
        .approve(spender.parse().unwrap(), amount)
        .nonce(nonce)
        .gas(100_000)
        .max_priority_fee_per_gas(native_gas_price.max_priority_fee_per_gas)
        .max_fee_per_gas(native_gas_price.max_fee_per_gas);

    match call.send().await {
        Ok(pending) => {
            tracing::info!("Approval pending ... Explorer: {}tx/{}", mmc.explorer_url, pending.tx_hash());
            match pending.get_receipt().await {
                Ok(receipt) => {
                    tracing::info!("Approval status: {:?} at block {:?}", receipt.status(), receipt.block_number);
                    Ok(receipt)
                }
                Err(e) => {
                    tracing::error!("Failed to confirm approval: {:?}", e);
                    Err(format!("Failed to confirm approval: {:?}", e))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to approve {}: {:?}", token, e);
            Err(format!("Failed to approve {}: {:?}", token, e))
        }
    }
}

/// Fetches wallet state including token balances and nonce.
pub async fn fetch_wallet_state(config: MarketMakerConfig) {
    let provider = create_provider(&config.rpc_url);
    let tokens = vec![config.base_token_address.clone(), config.quote_token_address.clone()];
    if let Ok(balances) = balances(&provider, config.wallet_public_key.clone(), tokens.clone()).await {
        tracing::debug!("Balances of sender {}: {:?}", config.wallet_public_key.clone(), balances);
    } else {
        tracing::error!("Failed to get balances of sender");
    }
    let nonce = provider.get_transaction_count(config.wallet_public_key.to_string().parse().unwrap()).await.unwrap();
    tracing::debug!("Nonce of sender {}: {}", config.wallet_public_key.clone(), nonce);
}

/// Fetches the receipt for a specific transaction hash.
pub async fn fetch_receipt(rpc: String, hash: String) -> Result<TransactionReceipt, String> {
    // If it doesn't contain 0x, return error
    if !hash.starts_with("0x") {
        return Err(format!("Invalid transaction hash: {}", hash));
    }
    let provider = create_provider(&rpc);
    match provider.get_transaction_receipt(hash.parse().unwrap()).await {
        Ok(receipt) => match receipt {
            Some(receipt) => Ok(receipt),
            None => Err(format!("No receipt found for transaction {}", hash)),
        },
        Err(e) => {
            tracing::error!("Failed to get receipt for transaction {}: {:?}", hash, e);
            Err(format!("Failed to get receipt for transaction {}: {:?}", hash, e))
        }
    }
}
