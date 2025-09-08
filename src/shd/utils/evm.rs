use crate::types::config::{EnvConfig, MarketMakerConfig};
use std::{str::FromStr, sync::Arc};

use alloy::{
    providers::{utils::Eip1559Estimation, Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    transports::http::Http,
};
use alloy_primitives::{B256, U256};
use reqwest::Client;
use url;

use crate::types::sol::IERC20;

/// =============================================================================
/// EVM Blockchain Utilities
/// =============================================================================
///
/// @description: Collection of utility functions for interacting with EVM-compatible
/// blockchains including Ethereum, Base, and other L2 networks
/// =============================================================================

/// =============================================================================
/// @function: create_provider
/// @description: Create an HTTP provider instance from RPC URL
/// @param rpc: RPC endpoint URL as string
/// @return RootProvider<Http<Client>>: Configured provider instance
/// =============================================================================
pub fn create_provider(rpc: &str) -> RootProvider<Http<Client>> {
    ProviderBuilder::new().on_http(rpc.parse().expect("Failed to parse RPC URL"))
}

/// =============================================================================
/// @function: latest
/// @description: Retrieve the latest block number from the specified RPC endpoint
/// @param provider: RPC endpoint URL as string
/// @return u64: Latest block number (returns 0 if failed)
/// =============================================================================
pub async fn latest(provider: String) -> u64 {
    let provider = create_provider(&provider);
    provider.get_block_number().await.unwrap_or_default()
}

/// =============================================================================
/// @function: gas_price
/// @description: Retrieve the current gas price from the specified RPC endpoint
/// @param provider: RPC endpoint URL as string
/// @return u128: Current gas price in wei (returns 0 if failed)
/// =============================================================================
pub async fn gas_price(provider: String) -> u128 {
    let provider = create_provider(&provider);
    provider.get_gas_price().await.unwrap_or_default()
}

/// =============================================================================
/// @function: eip1559_fees
/// @description: Estimate EIP-1559 gas fees (max fee and priority fee) for the network
/// @param provider: RPC endpoint URL as string
/// @return Result<Eip1559Estimation, String>: EIP-1559 fee estimation or error
/// =============================================================================
pub async fn eip1559_fees(provider: String) -> Result<Eip1559Estimation, String> {
    let provider = create_provider(&provider);

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
pub async fn allowance(rpc: String, owner: String, spender: String, token: String) -> Result<u128, String> {
    let provider = create_provider(&rpc);
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

/// =============================================================================
/// @function: approve
/// @description: Approve a spender to spend a specific amount of tokens
/// @param mmc: Market maker configuration
/// @param env: Environment configuration
/// @param spender: Spender address
/// @param token: Token contract address
pub async fn approve(mmc: MarketMakerConfig, env: EnvConfig, spender: String, token: String, amount: u128) -> Result<TransactionReceipt, String> {
    let rpc = mmc.rpc_url.parse::<url::Url>().unwrap().clone();
    let pk = env.wallet_private_key.clone();
    let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert swapper pk to B256")).expect("Failed to private key signer");
    let signer = alloy::network::EthereumWallet::from(wallet.clone());
    let provider = ProviderBuilder::new().with_chain_id(mmc.chain_id).wallet(signer.clone()).on_http(rpc.clone());
    let client = Arc::new(provider);
    let contract = IERC20::new(token.parse().unwrap(), client.clone());
    let symbol = contract.symbol().call().await.expect("Failed to get symbol")._0.to_string();
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

/// =============================================================================
/// @function: fetch_receipt
/// @description: Fetch the receipt for a specific transaction hash
/// @param rpc: RPC endpoint URL as string
/// @param hash: Transaction hash as string
/// @return Result<TransactionReceipt, String>: Receipt or error
/// =============================================================================
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

// let approve_receipt = if let Some(approve) = approval_result {
//     match approve.get_receipt().await {
//         Ok(receipt) => Some(receipt),
//         Err(e) => {
//             tracing::error!("Failed to get receipt for approval transaction: {:?}", e.to_string());
//             None
//         }
//     }
// } else {
//     None
// };

// let swap_receipt = swap.get_receipt().await;
// let total_time = time.elapsed().unwrap_or_default().as_millis();
// tracing::debug!(" - Receipt processing took {} ms", total_time);

// match (approve_receipt, swap_receipt) {
//     (Ok(approval_receipt), Ok(swap_receipt)) => {
//         if let Some(approval_receipt) = approve_receipt.unwrap() {
//             tracing::debug!("   - Approval receipt: status: {:?}", approval_receipt.status());
//         }
//         let swap_receipt = swap_receipt.clone();
//         let swap_receipt_data = ReceiptData {
//             status: swap_receipt.status().clone(),
//             gas_used: swap_receipt.gas_used,
//             effective_gas_price: swap_receipt.effective_gas_price,
//             error: None,
//             transaction_hash: swap_receipt.transaction_hash.to_string(),
//             transaction_index: swap_receipt.transaction_index.unwrap_or_default(),
//             block_number: swap_receipt.block_number.clone().unwrap_or_default(),
//         };
//         bd.receipt = Some(swap_receipt_data);
//     }
//     (_, Err(e)) => {
//         tracing::error!("Failed to get receipt for swap transaction: {:?}", e.to_string());
//         bd.broadcast_error = Some(e.to_string());
//     }
//     _ => {
//         tracing::error!("Failed to get receipts, unhandled error");
//     }
// }
