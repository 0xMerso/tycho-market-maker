// wip

/// Compute the optimal amount to swap
fn optimum(&self, context: MarketContext, inventory: Inventory, adjustment: CompReadjustment) -> OptimizationResult {
    let mut sim_count = 0;
    // Compute selling amount for 1$ and 1m$
    let low_usd_in_eth = 1. / context.eth_to_usd;
    let high_usd_in_eth = 1e5 / context.eth_to_usd; // 100k USD
    let is_base_to_quote = adjustment.selling == self.base;
    let (selling_low_usd, selling_high_usd) = match is_base_to_quote {
        true => (context.base_to_eth * low_usd_in_eth, context.base_to_eth * high_usd_in_eth),
        false => (1.0 / context.quote_to_eth * low_usd_in_eth, 1.0 / context.quote_to_eth * high_usd_in_eth),
    };
    let min = selling_low_usd;
    let max = selling_high_usd;
    tracing::debug!("Min: {:?} | Max: {:?} of {}", min, max, adjustment.selling.symbol);
    let mut current_low = (min * 10f64.powi(adjustment.selling.decimals as i32)).floor();
    let mut current_high = (max * 10f64.powi(adjustment.selling.decimals as i32)).floor();

    // --- Get limits ---
    let selling_address: Address = adjustment.selling.address.to_string().parse().unwrap();
    let buying_address: Address = adjustment.buying.address.to_string().parse().unwrap();
    match &adjustment.psc.protosim.get_limits(selling_address.clone(), buying_address.clone()) {
        Ok(limits) => {
            let limit_amount_in = limits.0.to_f64().unwrap_or(0.0); // The maximum input amount
            let limit_amount_in_normalized = limit_amount_in / 10f64.powi(adjustment.selling.decimals as i32);
            tracing::debug!("limit_amount_in_normalized: {:?}", limit_amount_in_normalized);
            if current_high > limit_amount_in {
                current_high = limit_amount_in;
                tracing::debug!("Overriding current_high with limit: {:?}", limit_amount_in);
            }
        }
        _ => {}
    }

    let mut optimal_quantity = 0.0;
    let mut resulting_spot_price = 0.0;
    for step in 0..OPTI_MAX_ITERATIONS {
        let mid_powered = (current_low + current_high) / 2.0;
        // tracing::debug!("Step {} | Mid: {:?}", step, mid_powered);
        let mid_powered_bg = BigUint::from(mid_powered.floor() as u128);
        match adjustment.psc.protosim.get_amount_out(mid_powered_bg.clone(), &adjustment.selling, &adjustment.buying) {
            Ok(result) => {
                sim_count += 1;
                match result.new_state.spot_price(&adjustment.selling, &adjustment.buying) {
                    Ok(new_spot_price) => {
                        let new_spot_price_normalized = 1. / new_spot_price / 10f64.powi(adjustment.selling.decimals as i32);
                        tracing::debug!(
                            "Step {} | Quantity: {:?} | new_spot_price_normalized: {:?} vs reference: {:?}",
                            step,
                            mid_powered,
                            new_spot_price_normalized,
                            adjustment.reference
                        );
                        // --- Assign new quantity ---
                        if (new_spot_price_normalized < adjustment.reference && current_low < current_high) || (new_spot_price_normalized > adjustment.reference && current_low > current_high) {
                            current_low = mid_powered;
                        } else {
                            current_high = mid_powered;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get spot price: {:?}", e.to_string());
                        continue;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to simulate get amount out: {:?}", e.to_string());
                continue;
            }
        }
    }

    let mut result = OptimizationResult::default();

    result
}
