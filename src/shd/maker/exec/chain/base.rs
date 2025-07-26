/// =============================================================================
/// Base L2 Execution Strategy
/// =============================================================================
///
/// @description: Base L2 execution strategy optimized for Base network. The flashblock
/// concept was developed by the Flashbots team. Flashblocks is one of two extensions
/// provided in the launch of Rollup-Boost. Rollup-Boost is a platform built for
/// Optimism-based (layer 2) rollup chains that allows chain operators to upgrade
/// the sequencer with additional features.
/// =============================================================================
///
/// The new approach has some subtleties:
/// - Each flashblock represents the transaction ordering for a **portion** of a coming block
/// - Each flashblock has a built-in gas limit based on its index in the sequence
/// - Once a flashblock is broadcast, its transaction ordering will be reflected in the final block
/// - The sequence of flashblocks is **fixed**, a flashblock cannot preempt another one
/// =============================================================================
use async_trait::async_trait;

use crate::maker::exec::ExecStrategyName;

use super::super::ExecStrategy;

/// =============================================================================
/// @struct: BaseExec
/// @description: Base L2 execution strategy implementation
/// @behavior: Optimized for Base network with flashblock support
/// =============================================================================
pub struct BaseExec;

/// =============================================================================
/// @function: new
/// @description: Create a new Base execution strategy instance
/// @return Self: New BaseExec instance
/// =============================================================================
impl BaseExec {
    pub fn new() -> Self {
        Self
    }
}

/// =============================================================================
/// @function: name
/// @description: Get the strategy name for logging purposes
/// @return String: Strategy name as string
/// =============================================================================
#[async_trait]
impl ExecStrategy for BaseExec {
    fn name(&self) -> String {
        ExecStrategyName::BaseStrategy.as_str().to_string()
    }
}
