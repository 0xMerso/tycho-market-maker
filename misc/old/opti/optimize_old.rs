use crate::types::config::MarketMakerConfig;
use crate::types::maker::{CompReadjustment, ExecutionOrder, SwapCalculation};
use crate::types::tycho::ProtoSimComp;
use crate::utils::r#static::BASIS_POINT_DENO;
use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
/// Optimize the bot's MarketMaker trades
use std::collections::HashMap;
use tracing;

// --- Get limits ---
// let selling_address: Address = adjustment.selling.address.to_string().parse().unwrap();
// let buying_address: Address = adjustment.buying.address.to_string().parse().unwrap();
// match &adjustment.psc.protosim.get_limits(selling_address.clone(), buying_address.clone()) {
//     Ok(limits) => {
//         let limits = limits.clone();
//         let amount_in = limits.0; // The maximum input amount
//         let amount_out = limits.1; // The maximum output amount
//         tracing::debug!("Amount in: {:?} | Amount out: {:?}", amount_in, amount_out);
//         let result = adjustment.psc.protosim.get_amount_out(amount_in, &adjustment.selling, &adjustment.buying);
//         tracing::debug!("Result: {:?}", result);
//     }
//     _ => {}
// }

/// Represents an AMM (Automated Market Maker) for a token pair
#[derive(Debug, Clone)]
pub struct AMM {
    pub token1: String,
    pub qty1: f64,
    pub token2: String,
    pub qty2: f64,
    pub fee: f64,
}

impl AMM {
    /// Creates a new AMM instance
    pub fn new(token1: String, qty1: f64, token2: String, qty2: f64, fee: f64) -> Self {
        Self { token1, qty1, token2, qty2, fee }
    }

    /// Returns the spot price of token2 in terms of token1
    /// (i.e., how many token1 per token2)
    pub fn spot_price(&self) -> Result<f64, String> {
        if self.qty2 == 0.0 {
            return Err("Quantity of token2 is zero; spot price is undefined.".into());
        }
        Ok(self.qty1 / self.qty2)
    }

    /// Returns the amount of output tokens received from inputting `qty_in` of `token_in`
    pub fn get_amount_out(&self, token_in: &str, qty_in: f64) -> Result<f64, String> {
        let qty_in_with_fee = qty_in * (1.0 - self.fee);

        if token_in == self.token1 {
            let numerator = qty_in_with_fee * self.qty2;
            let denominator = self.qty1 + qty_in_with_fee;
            Ok(numerator / denominator)
        } else if token_in == self.token2 {
            let numerator = qty_in_with_fee * self.qty1;
            let denominator = self.qty2 + qty_in_with_fee;
            Ok(numerator / denominator)
        } else {
            Err("Invalid token specified for input.".into())
        }
    }

    /// Applies a state delta by simulating a swap of `qty_in` of `token_in`
    /// Returns a new AMM instance representing the new state
    pub fn delta_transition(&self, token_in: &str, qty_in: f64) -> Result<Self, String> {
        let (new_qty1, new_qty2) = if token_in == self.token1 {
            let qty_in_with_fee = qty_in * (1.0 - self.fee);
            let new_qty1 = self.qty1 + qty_in;
            let amount_out = (qty_in_with_fee * self.qty2) / (self.qty1 + qty_in_with_fee);
            let new_qty2 = self.qty2 - amount_out;
            (new_qty1, new_qty2)
        } else if token_in == self.token2 {
            let qty_in_with_fee = qty_in * (1.0 - self.fee);
            let new_qty2 = self.qty2 + qty_in;
            let amount_out = (qty_in_with_fee * self.qty1) / (self.qty2 + qty_in_with_fee);
            let new_qty1 = self.qty1 - amount_out;
            (new_qty1, new_qty2)
        } else {
            return Err("Invalid token specified for input.".into());
        };

        Ok(AMM {
            token1: self.token1.clone(),
            qty1: new_qty1,
            token2: self.token2.clone(),
            qty2: new_qty2,
            fee: self.fee,
        })
    }

    /// Simulates a swap and returns a tuple: (amount_out, new_AMM_instance)
    pub fn simulate(&self, token_in: &str, qty_in: f64) -> Result<(f64, Self), String> {
        let amount_out = self.get_amount_out(token_in, qty_in)?;
        let new_amm = self.delta_transition(token_in, qty_in)?;
        Ok((amount_out, new_amm))
    }
}

/// Optimization result containing the optimal quantity and resulting metrics
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub optimal_qty: f64,
    pub resulting_spot_price: f64,
    pub simulation_count: usize,
    pub profit_bps: f64,
    pub profitable: bool,
}

/// Finds the optimal quantity using bisection method
pub fn find_optimal_qty_dichotomy(amm: &AMM, token_in: &str, target_price: f64, low: f64, high: f64, tol: f64, max_iter: usize) -> Result<OptimizationResult, String> {
    let mut sim_count = 0;
    let mut current_low = low;
    let mut current_high = high;

    // Initial simulations at bounds
    let spot_low = amm.simulate(token_in, low)?.1.spot_price()?;
    sim_count += 1;
    let spot_high = amm.simulate(token_in, high)?.1.spot_price()?;
    sim_count += 1;

    // Check if target price is reachable
    if !(spot_low.min(spot_high) <= target_price && target_price <= spot_low.max(spot_high)) {
        return Err(format!("Target price ({:.4}) is not reachable within the interval: [{:.4}, {:.4}]", target_price, spot_low, spot_high));
    }

    let mut optimal_qty = 0.0;
    let mut resulting_spot_price = 0.0;

    for step in 0..max_iter {
        let mid = (current_low + current_high) / 2.0;
        let (_, new_amm) = amm.simulate(token_in, mid)?;
        let spot = new_amm.spot_price()?;
        sim_count += 1;

        if (spot - target_price).abs() < tol {
            optimal_qty = mid;
            resulting_spot_price = spot;
            break;
        }

        // Determine which half to search
        if (spot < target_price && spot_low < spot_high) || (spot > target_price && spot_low > spot_high) {
            current_low = mid;
        } else {
            current_high = mid;
        }

        if step == max_iter - 1 {
            optimal_qty = mid;
            resulting_spot_price = spot;
        }
    }

    let profit_bps = ((target_price - amm.spot_price()?) / amm.spot_price()?) * BASIS_POINT_DENO;
    let profitable = profit_bps.abs() > amm.fee * BASIS_POINT_DENO;

    Ok(OptimizationResult {
        optimal_qty,
        resulting_spot_price,
        simulation_count: sim_count,
        profit_bps,
        profitable,
    })
}

/// Optimizes market maker trades using AMM simulation
pub fn optimize_market_maker_trades(adjustments: &[CompReadjustment], config: &MarketMakerConfig, reference_price: f64) -> Vec<ExecutionOrder> {
    let mut optimized_orders = Vec::new();

    for adjustment in adjustments {
        // Create AMM from component balances (simplified - you'll need to get actual balances)
        let amm = AMM::new(
            adjustment.selling.symbol.clone(),
            1000000.0, // Placeholder - get from actual pool balances
            adjustment.buying.symbol.clone(),
            1000000.0, // Placeholder - get from actual pool balances
            0.003,     // 0.3% fee - get from actual pool
        );

        // Find optimal quantity using bisection
        match find_optimal_qty_dichotomy(&amm, &adjustment.selling.symbol, reference_price, 0.0, 50000.0, 1e-3, 100) {
            Ok(result) => {
                if result.profitable {
                    tracing::info!(
                        "Optimal trade found: {} {} for {} {} (profit: {:.2} bps)",
                        result.optimal_qty,
                        adjustment.selling.symbol,
                        result.resulting_spot_price * result.optimal_qty,
                        adjustment.buying.symbol,
                        result.profit_bps
                    );

                    // Create execution order (simplified - you'll need to adapt to your actual types)
                    // This is a placeholder - you'll need to create the actual SwapCalculation
                    let calculation = SwapCalculation {
                        base_to_quote: adjustment.selling.symbol == config.base_token,
                        selling_amount: result.optimal_qty,
                        buying_amount: result.resulting_spot_price * result.optimal_qty,
                        powered_selling_amount: result.optimal_qty * 1e18, // Adjust based on decimals
                        powered_buying_amount: (result.resulting_spot_price * result.optimal_qty) * 1e18,
                        amount_out_normalized: result.resulting_spot_price * result.optimal_qty,
                        amount_out_powered: (result.resulting_spot_price * result.optimal_qty) * 1e18,
                        amount_out_min_normalized: (result.resulting_spot_price * result.optimal_qty) * 0.99, // 1% slippage
                        amount_out_min_powered: ((result.resulting_spot_price * result.optimal_qty) * 0.99) * 1e18,
                        gas_units: 200000, // Placeholder
                        average_sell_price: result.resulting_spot_price,
                        average_sell_price_net_gas: result.resulting_spot_price * 0.995, // Gas adjustment
                        gas_cost_eth: 0.01,                                              // Placeholder
                        gas_cost_usd: 20.0,                                              // Placeholder
                        gas_cost_in_output_token: 0.01,
                        selling_worth_usd: result.optimal_qty * reference_price,
                        buying_worth_usd: (result.resulting_spot_price * result.optimal_qty) * reference_price,
                        profit_delta_bps: result.profit_bps,
                        profitable: result.profitable,
                    };

                    let order = ExecutionOrder {
                        adjustment: adjustment.clone(),
                        calculation,
                    };

                    optimized_orders.push(order);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to optimize trade: {}", e);
            }
        }
    }

    optimized_orders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amm_spot_price() {
        let amm = AMM::new("DAI".to_string(), 200000.0, "ETH".to_string(), 100.0, 0.003);
        assert_eq!(amm.spot_price().unwrap(), 2000.0);
    }

    #[test]
    fn test_amm_get_amount_out() {
        let amm = AMM::new("DAI".to_string(), 200000.0, "ETH".to_string(), 100.0, 0.003);
        let amount_out = amm.get_amount_out("DAI", 1000.0).unwrap();
        assert!(amount_out > 0.0);
    }

    #[test]
    fn test_amm_simulate() {
        let amm = AMM::new("DAI".to_string(), 200000.0, "ETH".to_string(), 100.0, 0.003);
        let (amount_out, new_amm) = amm.simulate("DAI", 1000.0).unwrap();
        assert!(amount_out > 0.0);
        assert!(new_amm.qty1 > amm.qty1);
        assert!(new_amm.qty2 < amm.qty2);
    }

    #[test]
    fn test_find_optimal_qty_dichotomy() {
        let amm = AMM::new("DAI".to_string(), 200000.0, "ETH".to_string(), 100.0, 0.003);
        let target_price = 2100.0;
        let result = find_optimal_qty_dichotomy(&amm, "DAI", target_price, 0.0, 50000.0, 1e-3, 100).unwrap();

        assert!(result.optimal_qty > 0.0);
        assert!((result.resulting_spot_price - target_price).abs() < 1e-3);
        assert!(result.simulation_count > 0);
    }
}
