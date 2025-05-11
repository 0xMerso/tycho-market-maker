use std::{collections::HashMap, env, hash::Hash};

use alloy::providers::{Provider, ProviderBuilder};
use async_trait::async_trait;
use futures::StreamExt;
use tycho_client::feed::component_tracker::ComponentFilter;
use tycho_simulation::protocol::state::ProtocolSim;

use crate::{
    data::keys,
    types::{
        config::EnvConfig,
        maker::{IMarketMaker, MarketMaker},
        tycho::{PsbConfig, SharedTychoStreamState, SrzToken},
    },
    utils::r#static::{ADD_TVL_THRESHOLD, NULL_ADDRESS},
};

#[async_trait]
impl IMarketMaker for MarketMaker {
    async fn market_price(&self) -> Result<f64, String> {
        self.feed.get(self.config.clone()).await
    }

    async fn monitor(&mut self, mtx: SharedTychoStreamState, env: EnvConfig) {
        loop {
            tracing::debug!("Connecting ProtocolStreamBuilder for {}", self.config.network);
            let state = mtx.read().await;
            let tokens = state.tokens.clone(); // .iter().map(|t| SrzToken::from(t.clone())).collect::<Vec<_>>();
            drop(state); // Explicitly drop the read lock
            // let key = keys::tokens(self.config.network.clone());
            // =============================================== Tycho Stream ==============================================
            let psbc = PsbConfig {
                filter: ComponentFilter::with_tvl_range(ADD_TVL_THRESHOLD, ADD_TVL_THRESHOLD),
            };
            let mut components = vec![];
            let mut protosims: HashMap<String, Box<dyn ProtocolSim>> = HashMap::new();
            let psb = crate::core::helpers::psb(self.config.clone(), env.tycho_api_key.to_string(), psbc.clone(), tokens.clone()).await;
            let _stream = match psb.build().await {
                Ok(mut stream) => loop {
                    // Looping
                    match stream.next().await {
                        Some(msg) => match msg {
                            Ok(msg) => {
                                let market_price = self.market_price().await.unwrap_or_default();
                                tracing::info!(
                                    "'{}' stream: block # {} with {} states updates | Current market price: {}", // , + {} pairs, - {} pairs",
                                    self.config.network.clone(),
                                    msg.block_number,
                                    msg.states.len(),
                                    market_price // msg.new_pairs.len(),
                                                 // msg.removed_pairs.len()
                                );
                                if self.ready == false {
                                    // --- First stream ---

                                    protosims = msg.states.clone();
                                    let mut keys = vec![];
                                    for (_id, comp) in msg.new_pairs.iter() {
                                        keys.push(comp.id.to_string().to_lowercase());
                                    }
                                    for k in keys.clone() {
                                        if let Some(proto) = msg.states.get(&k.to_string()) {
                                            let comp = msg.new_pairs.get(&k.to_string()).expect("New pair not found");
                                            if !comp.id.to_string().contains(NULL_ADDRESS) {
                                                let t0to1 = proto.spot_price(&comp.tokens[0], &comp.tokens[1]);
                                                let t1to0 = proto.spot_price(&comp.tokens[1], &comp.tokens[0]);
                                                match (t0to1, t1to0) {
                                                    (Ok(t0to1), Ok(t1to0)) => {
                                                        let spread = t1to0 - market_price;
                                                        tracing::debug!(
                                                            " - Adding component of type {:<20} | SpotPrice t0to1: {:.7} and t1to0: {:.7} | Spread: {:.7} | Id: {}",
                                                            comp.protocol_type_name,
                                                            t0to1,
                                                            t1to0,
                                                            spread,
                                                            comp.id
                                                        );
                                                        components.push(comp.clone());
                                                    }
                                                    _ => {
                                                        tracing::warn!("Failed to get spot price on component {}", comp.id);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    self.ready = true;
                                    tracing::info!("✅ ProtocolStreamBuilder initialised successfully");
                                } else {
                                    // --- Update protosims ---
                                    if !msg.states.is_empty() {
                                        for x in msg.states.iter() {
                                            protosims.insert(x.0.clone().to_lowercase(), x.1.clone());
                                        }
                                    }
                                    // --- Update new pairs (add or overwrite) ---
                                    for x in msg.new_pairs.iter() {
                                        if let Some(pos) = components.iter().position(|current| current.id.to_string().to_lowercase() == x.0.to_string().to_lowercase()) {
                                            components[pos] = x.1.clone();
                                        } else {
                                            components.push(x.1.clone());
                                        }
                                    }
                                    // --- Remove old pairs ---
                                    for x in msg.removed_pairs.iter() {
                                        if let Some(pos) = components.iter().position(|current| current.id.to_string().to_lowercase() == x.0.to_string().to_lowercase()) {
                                            components.swap_remove(pos);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Stream error: {:?}", e);
                                break;
                            }
                        },
                        None => {
                            tracing::warn!("Stream closed. Retrying...");
                            break;
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to build stream on {}: {:?}. Exiting.", self.config.network, e.to_string());
                    return;
                }
            };
            // End
        }
    }
}
