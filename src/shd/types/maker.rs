use std::collections::HashMap;

use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tycho_execution::encoding::models::{Solution, Transaction};
use tycho_simulation::{
    models::Token,
    protocol::{models::ProtocolComponent, state::ProtocolSim},
};

use crate::maker::{exec::ExecStrategy, feed::PriceFeed};

use super::{
    config::{EnvConfig, MarketMakerConfig},
    tycho::{ProtoSimComp, SharedTychoStreamState},
};

#[async_trait]
pub trait IMarketMaker: Send + Sync {
    // Looks for pools having a spread higher than the configured threshold
    fn evaluate(&self, psc: &Vec<ProtoSimComp>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment>;
    // Analyzes the optimal way to readjust the market, compute if it's profitable according to a custom MM algo, and returns the execution orders
    async fn readjust(&self, context: MarketContext, inventory: Inventory, crs: Vec<CompReadjustment>, env: EnvConfig) -> Vec<ExecutionOrder>;
    // Retrieves prices, current inventory and market context. Stores some of it in cache memory, optimally reducing latency.
    fn prices(&self, psc: &Vec<ProtoSimComp>) -> Vec<ComponentPriceData>;
    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String>;
    async fn fetch_market_context(&self, components: Vec<ProtocolComponent>, protosims: &HashMap<std::string::String, Box<dyn ProtocolSim>>, tokens: Vec<Token>) -> Option<MarketContext>;
    async fn fetch_eth_usd(&self) -> Result<f64, String>;
    async fn fetch_market_price(&self) -> Result<f64, String>;
    // fn optimum(&self, context: MarketContext, inventory: Inventory, adjustment: CompReadjustment) -> OptimizationResult;

    // Create trade data from execution order and market context
    fn pre_trade_data(&self, order: &ExecutionOrder, market_context: &MarketContext, inventory: &Inventory) -> PreTradeData;
    // fn post_trade_data(&self, order: &ExecutionOrder, market_context: &MarketContext) -> PreTradeData;

    // Functions to build Tycho solution, encode, prepare, sign transactions
    fn solution(&self, order: ExecutionOrder, env: EnvConfig) -> Solution;
    // Encode the Tycho solution into a transaction
    fn encode(&self, solution: Solution, encoded: Transaction, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Result<(TransactionRequest, TransactionRequest), String>;
    // Prepare the transactions for execution (format, tycho encoder, approvals, swap)
    async fn prepare(&self, orders: Vec<ExecutionOrder>, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Vec<PreparedTrade>;
    // Infinite loop that monitors the Tycho stream state, looking for opportunities
    async fn run(&mut self, mtx: SharedTychoStreamState, env: EnvConfig);
}

/// ================== Market Maker ==================
pub struct MarketMaker {
    // Ready when the ProtocolStreamBuilder is initialised
    pub ready: bool,
    // Hash of the instance, used to uniquely identify the instance, for external programs (monitoring, etc.)
    pub identifier: String,
    // Configuration for the market maker
    pub config: MarketMakerConfig,
    // Price feed to use for market price (dynamic)
    pub feed: Box<dyn PriceFeed>,
    // Indicates whether the ProtocolStreamBuilder has been initialised (true if first stream has been received and saved)
    pub initialised: bool,
    // Base token from Tycho Client
    pub base: Token,
    // Quote token from Tycho Client
    pub quote: Token,
    // Snapshots of the market, price, etc.
    // pub snapshots: HashMap<String, MarketSnapshot>,

    // Used to limit the bot to 1 single swap exec in his entire lifetime, for testing purpose
    pub single: bool,

    // Execution strategy (dynamic)
    pub execution: Box<dyn ExecStrategy>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String, // "binance" or "chainlink"
    pub source: String, // https if type is "binance", of 0xAddress if type is "chainlink"
    pub reverse: bool,  // true if the price is to be reversed (e.g. 1 / price), only used for chainlink
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPriceData {
    pub address: String,
    pub r#type: String,
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct CompReadjustment {
    // Tycho
    pub psc: ProtoSimComp,
    // Recomputated
    pub direction: TradeDirection,
    pub selling: Token,
    pub buying: Token,
    pub spot: f64,
    pub reference: f64,
    pub spread: f64,
    pub spread_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub base_balance: u128,  // Divided
    pub quote_balance: u128, // Divided
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    pub base_to_eth: f64,
    pub quote_to_eth: f64,
    pub eth_to_usd: f64,
    pub max_fee_per_gas: u128,          // maximum base fee : gwei but why ?
    pub max_priority_fee_per_gas: u128, // base_fee_per_gas : 10^9 : gwei
    pub native_gas_price: u128,         // gwei: to be used for gas cost calculations
    // pub block: alloy::rpc::types::Block,
    pub block: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionOrder {
    pub adjustment: CompReadjustment,
    pub calculation: SwapCalculation,
    // pub bribing: BribeCalculation,
}

#[derive(Clone, Debug)]
pub struct SwapCalculation {
    pub base_to_quote: bool,
    pub selling_amount: f64,
    pub buying_amount: f64,
    pub powered_selling_amount: f64,
    pub powered_buying_amount: f64,
    // Post-swap price evaluation
    pub amount_out_normalized: f64,
    pub amount_out_powered: f64,
    pub amount_out_min_normalized: f64,
    pub amount_out_min_powered: f64,
    pub average_sell_price: f64,
    pub average_sell_price_net_gas: f64,
    // Gas
    pub gas_units: u128,
    pub gas_cost_eth: f64,
    pub gas_cost_usd: f64,
    pub gas_cost_in_output_token: f64,
    // Valuation
    pub selling_worth_usd: f64,
    pub buying_worth_usd: f64,
    // Profitability
    pub profit_delta_bps: f64,
    pub profitable: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedTrade {
    pub approve: TransactionRequest,
    pub swap: TransactionRequest,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ExecTxResult {
    pub sent: bool,
    pub status: bool,
    pub hash: String,
    pub error: Option<String>,
    pub receipt: Option<TransactionReceipt>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedPayload {
    pub approval: ExecTxResult,
    pub swap: ExecTxResult,
}

// ================== Trade Status ==================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeStatus {
    Pending,
    Simulating,
    SimulationFailed,
    ReadyToExecute,
    Broadcasting,
    BroadcastFailed,
    Confirmed,
    Failed,
}

/// Enhanced trade data structure optimized for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTrade {
    // Core trade info
    pub status: TradeStatus,
    pub created_at_ms: u64,
    // Pre-trade data
    pub pre_market_context: MarketContext,
    pub pre_trade_data: PreTradeData,
    // Execution data (all optional since steps can fail)
    // pub approval_simulation: Option<SimulatedData>,
    pub swap_simulation: Option<SimulatedData>,
    // pub approval_broadcast: Option<BroadcastData>,
    pub swap_broadcast: Option<BroadcastData>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedData {
    pub simulated_at_ms: u64,
    pub simulated_took_ms: u64,
    pub estimated_gas: u128,
    pub status: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastData {
    pub broadcasted_at_ms: u64,
    pub broadcasted_took_ms: u64,
    pub hash: String,
    pub broadcast_error: Option<String>,
    pub receipt: Option<ReceiptData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    pub status: bool,
    pub gas_used: u128,
    pub confirmed_at_ms: u64,
    pub error: Option<String>,
    pub transaction_hash: String,
    pub transaction_index: u64,
    pub block_number: u64,
    pub effective_gas_price: u128,
}

/// Simple trade data structure with essential fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreTradeData {
    // Token information
    pub base_token: String,
    pub quote_token: String,
    pub trade_direction: TradeDirection,
    // Trade amounts
    pub amount_in_normalized: f64,
    pub amount_out_expected: f64,
    // Price information
    pub spot_price: f64,
    pub reference_price: f64,
    pub eth_usd_price: f64,
    // Slippage and profitability
    pub slippage_tolerance_bps: f64,
    pub profit_delta_bps: f64,
    // Gas cost
    pub gas_cost_usd: f64,
    // Timing
    pub computed_at_time_ms: u64,
    pub computed_at_block: u64,
    // Wallet
    pub wallet_nonce: String,
    pub base_balance: u128,
    pub quote_balance: u128,
}
