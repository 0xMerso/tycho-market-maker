/// =============================================================================
/// Unichain Execution Strategy
/// =============================================================================
///
/// @description: Unichain execution strategy optimized for Unichain network.
/// Unichain provides advanced transaction features and optimizations for
/// high-frequency trading and market making operations.
/// @reference: https://docs.unichain.org/docs/technical-information/advanced-txn
/// =============================================================================
use async_trait::async_trait;

use crate::maker::exec::ExecStrategyName;

use super::super::ExecStrategy;

/// =============================================================================
/// @struct: UnichainExec
/// @description: Unichain execution strategy implementation
/// @behavior: Optimized for Unichain network with advanced transaction features
/// =============================================================================
pub struct UnichainExec;

/// =============================================================================
/// @function: new
/// @description: Create a new Unichain execution strategy instance
/// @return Self: New UnichainExec instance
/// =============================================================================
impl Default for UnichainExec {
    fn default() -> Self {
        Self::new()
    }
}

impl UnichainExec {
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
impl ExecStrategy for UnichainExec {
    fn name(&self) -> String {
        ExecStrategyName::UnichainStrategy.as_str().to_string()
    }
}
