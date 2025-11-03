///   =============================================================================
/// Binary Search Optimization Module
///   =============================================================================
///
/// @description: Implements binary search (bisection) algorithm to find optimal
/// swap quantity that maximizes profit
///   =============================================================================
use num_bigint::BigUint;
use tycho_common::models::token::Token;
use tycho_common::simulation::protocol_sim::ProtocolSim; // ProtocolSim trait for protocol simulation

use crate::utils::constants::{BASIS_POINT_DENO, OPTI_MAX_ITERATIONS, OPTI_TOLERANCE};

///   =============================================================================
/// @struct: OptimizationResult
/// @description: Contains optimal swap amount and metrics
///   =============================================================================
#[derive(Default, Debug, Clone)]
pub struct OptimizationResult {
    pub optimal_qty: f64,             // Optimal quantity to swap (normalized)
    pub optimal_qty_powered: BigUint, // Optimal quantity (in token decimals)
    pub simulation_count: usize,      // Number of simulations performed
    pub execution_price: f64,         // Expected execution price after swap
    pub price_impact_bps: f64,        // Price impact vs reference in basis points
}

///   =============================================================================
/// @function: find_optimal_swap_amount
/// @description: Uses binary search to find swap amount that stabilizes pool price
///               to match the reference price after the swap
/// @param protosim: Protocol simulator for the pool
/// @param selling_token: Token being sold
/// @param buying_token: Token being bought  
/// @param reference_price: Target price to stabilize the pool to (base/quote)
/// @param base_is_token0: Whether base token is token0 in the pool
/// @param max_amount: Maximum amount available to swap (normalized)
/// @return Result<OptimizationResult, String>: Optimization result or error
///   =============================================================================
pub fn find_optimal_swap_amount(
    protosim: &dyn ProtocolSim, selling_token: &Token, buying_token: &Token, reference_price: f64, base_is_token0: bool, max_amount: f64,
) -> Result<OptimizationResult, String> {
    let selling_pow = 10f64.powi(selling_token.decimals as i32);
    let buying_pow = 10f64.powi(buying_token.decimals as i32);

    let mut low = 0.0;
    let mut high = max_amount;
    let mut simulation_count = 0;

    // Get initial spot price to understand the direction we need to move
    let initial_spot_price = protosim
        .spot_price(if base_is_token0 { selling_token } else { buying_token }, if base_is_token0 { buying_token } else { selling_token })
        .map_err(|e| format!("Failed to get initial spot price: {:?}", e))?;

    // First check if max amount can reach the target
    let max_post_swap_price = calculate_post_swap_price(protosim, selling_token, buying_token, max_amount, selling_pow, buying_pow, base_is_token0)?;
    simulation_count += 1;

    let (_, max_execution_price) = calculate_swap_output(protosim, selling_token, buying_token, max_amount, selling_pow, buying_pow, base_is_token0)?;
    simulation_count += 1;

    let max_diff = (max_post_swap_price - reference_price).abs();

    // Check if max amount overshoots the target
    let overshoots = if initial_spot_price < reference_price {
        // Trying to push price up
        if max_post_swap_price > reference_price {
            tracing::info!(
                "Max amount overshoots target: Pool {:.2} → {:.2} (target: {:.2}). Binary search will find exact amount.",
                initial_spot_price,
                max_post_swap_price,
                reference_price
            );
            true
        } else {
            false
        }
    } else {
        // Trying to push price down
        if max_post_swap_price < reference_price {
            tracing::info!(
                "Max amount overshoots target: Pool {:.2} → {:.2} (target: {:.2}). Binary search will find exact amount.",
                initial_spot_price,
                max_post_swap_price,
                reference_price
            );
            true
        } else {
            false
        }
    };

    // If max amount doesn't reach target, use it as best effort
    if !overshoots && max_diff > 0.0001 {
        // tracing::info!(
        //     "Max amount insufficient to reach target. Using max as best effort. Pool: {:.2} → {:.2}, Target: {:.2}",
        //     initial_spot_price,
        //     max_post_swap_price,
        //     reference_price
        // );
        // Return max amount as the best we can do
        let optimal_qty_powered = BigUint::from((max_amount * selling_pow).floor() as u128);
        let price_impact_bps = max_diff / reference_price * BASIS_POINT_DENO;

        return Ok(OptimizationResult {
            optimal_qty: max_amount,
            optimal_qty_powered,
            simulation_count,
            execution_price: max_execution_price,
            price_impact_bps,
        });
    }

    let mut best_qty = max_amount;
    let mut best_price_diff = max_diff;
    let mut best_execution_price = max_execution_price;
    let mut best_post_swap_price = max_post_swap_price;

    // Use binary search to find amount that makes post-swap price = reference price
    for _iteration in 0..OPTI_MAX_ITERATIONS {
        let mid = (low + high) / 2.0;

        // Skip if amount is too small
        if mid < f64::EPSILON {
            low = mid;
            continue;
        }

        // Calculate the price after the swap
        let post_swap_price = calculate_post_swap_price(protosim, selling_token, buying_token, mid, selling_pow, buying_pow, base_is_token0)?;
        simulation_count += 1;

        // Also get execution price for reporting
        let (_, execution_price) = calculate_swap_output(protosim, selling_token, buying_token, mid, selling_pow, buying_pow, base_is_token0)?;
        simulation_count += 1;

        // Calculate how close the post-swap price is to reference
        let price_diff = (post_swap_price - reference_price).abs();

        // tracing::debug!(
        //     "Iteration {}: qty={:.4}, post_swap_price={:.4}, ref_price={:.4}, diff={:.6}, exec_price={:.4}",
        //     _iteration, mid, post_swap_price, reference_price, price_diff, execution_price
        // );

        // Track best result (minimum difference from reference)
        if price_diff < best_price_diff {
            best_price_diff = price_diff;
            best_qty = mid;
            best_execution_price = execution_price;
            best_post_swap_price = post_swap_price;
        }

        // Check convergence
        if (high - low) < OPTI_TOLERANCE || price_diff < 0.0001 {
            break;
        }

        // Binary search based on post-swap price vs reference
        if post_swap_price < reference_price {
            // Pool price too low after swap, need more aggressive swap
            // If we're selling base (pushing price up), we need more volume
            // If we're selling quote (pushing price down), we need less volume
            if base_is_token0 {
                low = mid; // Selling base pushes price up, need more
            } else {
                high = mid; // Selling quote pushes price down, need less
            }
        } else {
            // Pool price too high after swap
            if base_is_token0 {
                high = mid; // Selling base pushes price up, need less
            } else {
                low = mid; // Selling quote pushes price down, need more
            }
        }
    }

    // Ensure we found a valid quantity
    if best_qty < f64::EPSILON {
        return Err("No valid swap amount found".to_string());
    }

    let optimal_qty_powered = BigUint::from((best_qty * selling_pow).floor() as u128);
    let price_impact_bps = ((best_post_swap_price - reference_price).abs() / reference_price) * BASIS_POINT_DENO;

    Ok(OptimizationResult {
        optimal_qty: best_qty,
        optimal_qty_powered,
        simulation_count,
        execution_price: best_execution_price,
        price_impact_bps,
    })
}

///   =============================================================================
/// @function: calculate_post_swap_price
/// @description: Calculates the pool's spot price after a swap is executed
/// @return Result<f64, String>: Post-swap spot price (base/quote) or error
///   =============================================================================
fn calculate_post_swap_price(
    protosim: &dyn ProtocolSim, selling_token: &Token, buying_token: &Token, amount_normalized: f64, selling_pow: f64, _buying_pow: f64, base_is_token0: bool,
) -> Result<f64, String> {
    if amount_normalized < f64::EPSILON {
        // No swap, return current spot price
        return protosim
            .spot_price(if base_is_token0 { selling_token } else { buying_token }, if base_is_token0 { buying_token } else { selling_token })
            .map_err(|e| format!("Failed to get spot price: {:?}", e));
    }

    let amount_powered = BigUint::from((amount_normalized * selling_pow).floor() as u128);

    // Get the result which includes the new state after the swap
    let result = protosim
        .get_amount_out(amount_powered, selling_token, buying_token)
        .map_err(|e| format!("Failed to simulate swap: {:?}", e))?;

    // The result.new_state contains the pool state after the swap
    // Get the spot price from this new state
    let post_swap_price = result
        .new_state
        .spot_price(if base_is_token0 { selling_token } else { buying_token }, if base_is_token0 { buying_token } else { selling_token })
        .map_err(|e| format!("Failed to get post-swap price: {:?}", e))?;

    Ok(post_swap_price)
}

///   =============================================================================
/// @function: calculate_swap_output
/// @description: Calculates the output amount and execution price for a given swap
/// @return Result<(f64, f64), String>: (output_amount, execution_price) or error
///   =============================================================================
fn calculate_swap_output(
    protosim: &dyn ProtocolSim, selling_token: &Token, buying_token: &Token, amount_normalized: f64, selling_pow: f64, buying_pow: f64, base_is_token0: bool,
) -> Result<(f64, f64), String> {
    if amount_normalized < f64::EPSILON {
        // For zero amount, return zero output and spot price
        let spot_price = protosim
            .spot_price(if base_is_token0 { selling_token } else { buying_token }, if base_is_token0 { buying_token } else { selling_token })
            .unwrap_or(0.0);
        return Ok((0.0, spot_price));
    }

    let amount_powered = BigUint::from((amount_normalized * selling_pow).floor() as u128);

    // Get amount out from AMM
    let result = protosim
        .get_amount_out(amount_powered, selling_token, buying_token)
        .map_err(|e| format!("Failed to simulate swap: {:?}", e))?;

    let amount_out = result.amount.to_string().parse::<f64>().unwrap_or(0.0) / buying_pow;

    if amount_out <= 0.0 {
        return Err("Invalid swap: zero output".to_string());
    }

    // Calculate execution price (always as base/quote)
    let execution_price = if base_is_token0 {
        // Selling base for quote: price = quote_out / base_in
        amount_out / amount_normalized
    } else {
        // Selling quote for base: price = quote_in / base_out
        amount_normalized / amount_out
    };

    Ok((amount_out, execution_price))
}
