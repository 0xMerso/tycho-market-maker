///   =============================================================================
/// Mainnet Execution Strategy
///   =============================================================================
///
/// @description: Mainnet execution strategy optimized for Ethereum mainnet with
/// Flashbots support. This strategy provides MEV protection and bundle submission
/// capabilities for secure and efficient transaction execution on Ethereum mainnet.
///
/// @changelog:
/// - 2025-01: Upgraded from Alloy 0.5.4 → 1.0.30 + alloy-mev 0.5 → 1.0
/// - Changed: No more BundleSigner wrapper - pass PrivateKeySigner directly
/// - Changed: Use provider.bundle_builder() instead of manual EthSendBundle construction
/// - Changed: .add_transaction_request() instead of .encode_request()
///   =============================================================================
use async_trait::async_trait;
use std::str::FromStr;

use alloy::{
    network::{EthereumWallet, TransactionBuilder},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use alloy_mev::EthMevProviderExt; // Provides bundle_builder() and send_eth_bundle()
use alloy_primitives::B256;

use crate::{
    maker::{exec::ExecStrategyName, tycho::get_alloy_chain},
    types::{
        config::{EnvConfig, MarketMakerConfig},
        maker::{BroadcastData, Trade},
    },
};

use super::super::ExecStrategy;

///   =============================================================================
/// @struct: MainnetExec
/// @description: Mainnet execution strategy implementation
/// @behavior: Optimized for Ethereum mainnet with Flashbots MEV protection
///   =============================================================================
pub struct MainnetExec;

impl Default for MainnetExec {
    fn default() -> Self {
        Self::new()
    }
}

impl MainnetExec {
    pub fn new() -> Self {
        Self
    }
}

///   =============================================================================
/// TRAIT IMPLEMENTATION: ExecStrategy
///   =============================================================================
/// OVERRIDDEN FUNCTIONS:
/// - name(): Returns "Mainnet_Strategy"
/// - broadcast(): Custom implementation using Flashbots bundles for MEV protection
///
/// INHERITED FUNCTIONS (using default implementation):
/// - pre_hook(): Default logging
/// - post_hook(): Default event publishing
/// - execute(): Default orchestration flow
/// - simulate(): Default EVM simulation
///   =============================================================================
#[async_trait]
impl ExecStrategy for MainnetExec {
    /// OVERRIDDEN: Custom strategy name
    fn name(&self) -> String {
        ExecStrategyName::MainnetStrategy.as_str().to_string()
    }

    /// =============================================================================
    /// OVERRIDDEN: Custom broadcast implementation - Flashbots Bundle Submission
    /// @description: Replaces default mempool broadcast with Flashbots bundle submission
    /// @param prepared: Vector of trades to broadcast (each can have approval + swap)
    /// @param mmc: Market maker configuration
    /// @param env: Environment configuration
    /// @return `Result<Vec<BroadcastData>, String>`: Broadcast results or error
    ///
    /// @behavior:
    /// - Submits transactions as bundles to multiple builders (Flashbots, Beaverbuild, Titan, Rsync)
    /// - Handles approval transactions if infinite_approval is disabled
    /// - Targets inclusion at current_block + inclusion_block_delay
    /// - Provides MEV protection via private mempool
    ///
    /// @differs_from_default: Uses private mempool via Flashbots instead of public mempool
    /// =============================================================================
    async fn broadcast(&self, prepared: Vec<Trade>, mmc: MarketMakerConfig, env: EnvConfig) -> Result<Vec<BroadcastData>, String> {
        tracing::info!("{}: broadcasting {} transactions on Mainnet via Flashbots bundle", self.name(), prepared.len());

        // Setup provider with wallet
        let _ac = get_alloy_chain(mmc.network_name.as_str().to_string()).expect("Failed to get alloy chain");
        let rpc = mmc.rpc_url.parse::<url::Url>().unwrap();
        let pk = env.wallet_private_key.clone();
        let wallet = PrivateKeySigner::from_bytes(&B256::from_str(&pk).expect("Failed to convert wallet pk to B256")).expect("Failed to create private key signer");
        let signer = EthereumWallet::from(wallet.clone());

        let provider = ProviderBuilder::new().with_chain_id(mmc.chain_id).wallet(signer.clone()).connect_http(rpc);

        // Flashbots bundle signer for MEV protection and block builder authentication
        // Note: Using a random key (no persistent reputation) for simplicity
        // This is NOT a security risk - the bundle signer authenticates bundle submissions,
        // it does NOT control any funds (the wallet private key above handles actual transactions)
        // Production users may configure a persistent key to maintain builder reputation across restarts
        // TODO: Add optional persistent bundle signer config
        let bundle_signer = PrivateKeySigner::random();

        // Build endpoints for multiple builders (Flashbots + alternatives)
        // NEW API: No more BundleSigner::flashbots() wrapper - pass PrivateKeySigner directly
        let endpoints = provider
            .endpoints_builder()
            .beaverbuild()
            .titan(bundle_signer.clone()) // Pass signer directly
            .flashbots(bundle_signer.clone()) // Pass signer directly
            .rsync()
            .build();

        let mut results = Vec::new();

        // Skip actual broadcast in testing mode
        if env.testing {
            tracing::info!("🧪 Testing mode: Skipping bundle broadcast");
            return Ok(results);
        }

        // Process each trade (each may contain approval + swap)
        for trade in prepared.iter() {
            // Get current block and calculate target inclusion block
            let bnum = provider.get_block_number().await.map_err(|e| format!("Failed to get block number: {:?}", e))?;
            let target_block = bnum + mmc.inclusion_block_delay;

            tracing::info!("{}: Current block: {}, target inclusion: {} (delay: {})", self.name(), bnum, target_block, mmc.inclusion_block_delay);

            let mut bd = BroadcastData::default();
            let time = std::time::SystemTime::now();

            // Record broadcast timestamp
            let now = std::time::SystemTime::now();
            let broadcasted_at_ms = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            bd.broadcasted_at_ms = broadcasted_at_ms;

            // Build and get expected transaction hash (for tracking)
            match trade.swap.clone().build(&signer).await {
                Ok(tx) => {
                    let hash = tx.tx_hash().to_string();
                    tracing::info!("{}: Expected swap tx hash: {}", self.name(), hash);
                    bd.hash = hash;
                }
                Err(e) => {
                    tracing::error!("{}: Failed to build swap transaction: {:?}", self.name(), e);
                }
            }

            // Build bundle using the new bundle_builder() API
            let mut bundle_builder = provider.bundle_builder().on_block(target_block);

            // Add approval transaction if needed (when infinite_approval is false)
            if let Some(approval) = &trade.approve {
                bundle_builder = bundle_builder
                    .add_transaction_request(approval.clone())
                    .await
                    .map_err(|e| format!("Failed to add approval to bundle: {:?}", e))?;
                tracing::info!("{}: Added approval tx to bundle", self.name());
            }

            // Add the swap transaction
            bundle_builder = bundle_builder
                .add_transaction_request(trade.swap.clone())
                .await
                .map_err(|e| format!("Failed to add swap to bundle: {:?}", e))?;

            // Finalize the bundle
            let bundle = bundle_builder.build();

            tracing::info!("{}: Sending bundle to builders (targeting block {})...", self.name(), target_block);

            // Send bundle to multiple builders using alloy-mev
            let responses = provider.send_eth_bundle(bundle, &endpoints).await;

            let took = time.elapsed().unwrap_or_default().as_millis();
            bd.broadcasted_took_ms = took;

            tracing::info!("{}: Bundle submission complete. Got {} responses in {}ms", self.name(), responses.len(), took);

            // Process responses from each builder
            let mut successful_builders = 0;
            let mut failed_builders = 0;

            for response in responses.iter() {
                match response {
                    Ok(response) => {
                        successful_builders += 1;
                        tracing::info!("    ✅ Builder accepted bundle: {}", response.bundle_hash);
                    }
                    Err(e) => {
                        failed_builders += 1;
                        tracing::warn!("    ❌ Builder rejected bundle: {:?}", e);

                        // Store first error (if not already set)
                        if bd.broadcast_error.is_none() {
                            bd.broadcast_error = Some(format!("Bundle submission failed: {:?}", e));
                        }
                    }
                }
            }

            tracing::info!("{}: Bundle results: {}/{} builders accepted", self.name(), successful_builders, successful_builders + failed_builders);

            // Consider broadcast successful if at least one builder accepted
            if successful_builders == 0 {
                tracing::error!("{}: All builders rejected the bundle!", self.name());
                return Err(bd.broadcast_error.unwrap_or_else(|| "All builders rejected bundle".to_string()));
            }

            results.push(bd);
        }

        Ok(results)
    }
}

/* =============================================================================
 * OLD IMPLEMENTATION (alloy-mev 0.5) - KEPT FOR REFERENCE
 * =============================================================================
 *
 * Key API changes in alloy-mev 1.0:
 *
 * 1. NO BundleSigner wrapper:
 *    OLD: .flashbots(BundleSigner::flashbots(bundle_signer))
 *    NEW: .flashbots(bundle_signer.clone())
 *
 * 2. Bundle construction via builder pattern:
 *    OLD: Manual EthSendBundle { txs, block_number, ... }
 *    NEW: provider.bundle_builder().on_block(...).add_transaction_request(...).build()
 *
 * 3. Adding transactions:
 *    OLD: provider.encode_request(tx) → manual Vec<Bytes> txs
 *    NEW: bundle_builder.add_transaction_request(tx).await
 *
 * 4. Import changes:
 *    OLD: use alloy::rpc::types::mev::EthSendBundle;
 *    NEW: No manual import needed - use bundle_builder() API
 *
 * OLD CODE:
 *
 * use alloy_mev::{BundleSigner, EthMevProviderExt};
 * use alloy::rpc::types::mev::EthSendBundle;
 *
 * let bundle_signer = PrivateKeySigner::random();
 *
 * let endpoints = provider
 *     .endpoints_builder()
 *     .beaverbuild()
 *     .titan(BundleSigner::flashbots(bundle_signer.clone()))  // ← OLD: wrapper needed
 *     .flashbots(BundleSigner::flashbots(bundle_signer.clone()))
 *     .rsync()
 *     .build();
 *
 * let mut txs = vec![];
 * if let Some(approval) = &trade.approve {
 *     match provider.encode_request(approval.clone()).await {  // ← OLD: manual encoding
 *         Ok(encoded) => txs.push(encoded),
 *         Err(e) => tracing::error!("Failed to encode approval: {:?}", e),
 *     }
 * }
 * match provider.encode_request(trade.swap.clone()).await {
 *     Ok(encoded) => txs.push(encoded),
 *     Err(e) => return Err(format!("Failed to encode swap: {:?}", e)),
 * }
 *
 * let bundle = EthSendBundle {  // ← OLD: manual construction
 *     txs,
 *     block_number: target_block,
 *     min_timestamp: None,
 *     max_timestamp: None,
 *     reverting_tx_hashes: vec![],
 *     replacement_uuid: None,
 *     dropping_tx_hashes: vec![],
 *     refund_percent: None,
 *     refund_recipient: None,
 *     refund_tx_hashes: vec![],
 *     extra_fields: Default::default(),
 * };
 *
 * let responses = provider.send_eth_bundle(bundle, &endpoints).await;
 *
 * =============================================================================
 */
