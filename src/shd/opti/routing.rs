use std::collections::{HashMap, HashSet, VecDeque};
use tycho_common::models::token::Token;
use tycho_simulation::protocol::models::ProtocolComponent;

use crate::types::tycho::{ProtoSimComp, ValorisationPath};

/// Finds a conversion path between two tokens using BFS graph traversal.
///
/// Builds an adjacency graph from protocol components and finds the shortest
/// path from input to target token. Returns both the token path and the
/// component IDs used for pricing.
pub fn find_path(cps: Vec<ProtocolComponent>, input: String, target: String) -> Result<ValorisationPath, String> {
    // Build adjacency graph: (destination token address, component id that provides this conversion)
    let mut graph: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for comp in cps {
        let comp_id = comp.id.clone();
        let addresses: Vec<String> = comp.tokens.iter().map(|t| t.address.to_string().to_lowercase()).collect();
        for token_in in &addresses {
            for token_out in &addresses {
                if token_in != token_out {
                    graph.entry(token_in.clone()).or_default().push((token_out.clone(), comp_id.to_string().clone()));
                }
            }
        }
    }

    // For debugging: print the graph
    // e.g., log::info!("Graph: {:?}", graph);
    let start = input.to_lowercase();
    let target = target.to_lowercase();

    // BFS queue items: (current token, token path, component id path)
    let mut queue: VecDeque<(String, Vec<String>, Vec<String>)> = VecDeque::new();
    queue.push_back((start.clone(), vec![start.clone()], vec![]));
    let mut visited: HashSet<String> = HashSet::new();

    while let Some((current, token_path, comp_path)) = queue.pop_front() {
        if current == target {
            return Ok(ValorisationPath { token_path, comp_path });
        }
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());
        if let Some(neighbors) = graph.get(&current) {
            for (next, comp_id) in neighbors {
                if token_path.contains(next) {
                    continue;
                }
                let mut new_token_path = token_path.clone();
                new_token_path.push(next.clone());
                let mut new_comp_path = comp_path.clone();
                new_comp_path.push(comp_id.clone());
                queue.push_back((next.clone(), new_token_path, new_comp_path));
            }
        }
    }
    Err(format!("No path found from {} to {}", input, target))
}

/// Quotes a token path price using protocol simulations.
///
/// Calculates the cumulative price across a path of tokens by chaining
/// spot prices from protocol components.
pub fn quote(pts: Vec<ProtoSimComp>, atks: Vec<Token>, path: Vec<String>) -> Option<f64> {
    // If ETH, return 1. Else, if the path is empty, return None.
    if path.len() == 1 {
        // tracing::debug!(" - Path is just ETH. Returning quote of 1.0");
        return Some(1.0);
    } else if path.len() < 2 {
        tracing::error!("🔺 Path is too short: {:?}", path);
        return None;
    }

    let mut cumulative_price = 1.0;

    // For each consecutive pair in the path ...
    for window in path.windows(2) {
        let token_in = window[0].to_lowercase();
        let token_out = window[1].to_lowercase();

        // Find a protocol state that can convert token_in to token_out.
        let mut found = false;
        for state in &pts {
            // Extract the component's token addresses.
            let comp_tokens: Vec<String> = state.component.tokens.iter().map(|t| t.address.to_string().to_lowercase()).collect();
            if comp_tokens.contains(&token_in) && comp_tokens.contains(&token_out) {
                // Resolve the tokens from the global list.
                let base = match atks.iter().find(|t| t.address.to_string().to_lowercase() == token_in) {
                    Some(t) => t.clone(),
                    None => {
                        tracing::warn!("Token not found in list: {}", token_in);
                        return None;
                    }
                };
                let quote = match atks.iter().find(|t| t.address.to_string().to_lowercase() == token_out) {
                    Some(t) => t.clone(),
                    None => {
                        tracing::warn!("Token not found in list: {}", token_out);
                        return None;
                    }
                };
                match state.protosim.spot_price(&base, &quote) {
                    Ok(rate) => {
                        cumulative_price *= rate;
                        found = true;
                        break;
                    }
                    Err(_e) => {}
                }
            }
        }
        if !found {
            tracing::warn!("🔺 Quote error: no conversion path found for {} -> {}", token_in, token_out);
            return None;
        }
    }
    // tracing::debug!(" - One unit of token ({:?} to {:?}) quoted to ETH = {}", path.first(), path.last(), cumulative_price);
    Some(cumulative_price)
}
