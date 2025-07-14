/// Optimization result containing the optimal quantity and resulting metrics
#[derive(Default, Debug, Clone)]
pub struct OptimizationResult {
    pub optimal_qty: f64,
    pub resulting_spot_price: f64,
    pub simulation_count: usize,
    pub profit_bps: f64,
    pub profitable: bool,
}

// https://github.com/hugoschnoering2/TychoTAPs/blob/main/optim.ipynb
