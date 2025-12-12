//! Market Maker Types and Data Structures
//!
//! Core type definitions for market making operations including the main market
//! maker struct, data structures for trades, orders, and market context.
use alloy::rpc::types::TransactionRequest;
use serde::{Deserialize, Serialize};
use tycho_common::models::token::Token;

use crate::maker::{exec::ExecStrategy, feed::PriceFeed};

use super::{config::MarketMakerConfig, tycho::ProtoSimComp};

/// Main market maker implementation struct.
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

/// Configuration for price feed sources.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String, // "binance" or "chainlink"
    pub source: String, // https if type is "binance", of 0xAddress if type is "chainlink"
    pub reverse: bool,  // true if the price is to be reversed (e.g. 1 / price), only used for chainlink
}

/// Direction of trade execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TradeDirection {
    Buy,
    Sell,
}

/// Price data for a specific component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPriceData {
    pub address: String,
    pub r#type: String,
    pub price: f64,
}

/// Component readjustment opportunity.
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

/// Current token inventory and wallet state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub base_balance: u128,  // Divided
    pub quote_balance: u128, // Divided
    pub nonce: u64,
}

/// Current market context and pricing information.
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

/// Complete execution order with adjustment and calculation.
#[derive(Debug, Clone)]
pub struct ExecutionOrder {
    pub adjustment: CompReadjustment,
    pub calculation: SwapCalculation,
    // pub bribing: BribeCalculation,
}

/// Detailed swap calculation with profitability analysis.
#[derive(Debug, Clone)]
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

/// Transaction request for trade execution.
#[derive(Debug, Clone)]
pub struct TradeTxRequest {
    pub approve: Option<TransactionRequest>,
    pub swap: TransactionRequest,
}

/// Complete trade with transactions and metadata.
#[derive(Debug, Clone)]
pub struct Trade {
    pub approve: Option<TransactionRequest>,
    pub swap: TransactionRequest,
    pub metadata: TradeData,
}

/// Status of trade execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeStatus {
    Pending,
    SimulationInProgress,
    SimulationSucceeded,
    SimulationFailed,
    BroadcastInProgress,
    BroadcastSucceeded,
    BroadcastFailed,
}

/// Complete trade data with all execution information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    // Core trade info
    pub status: TradeStatus,
    pub timestamp: u128,
    // Pre-trade data
    pub context: MarketContext,
    pub metadata: PreTradeData,
    pub inventory: Inventory,
    // Sim/Exec
    pub simulation: Option<SimulatedData>,
    pub broadcast: Option<BroadcastData>,
}

/// Transaction simulation results.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedData {
    pub simulated_at_ms: u128,
    pub simulated_took_ms: u128,
    pub estimated_gas: u128,
    pub status: bool,
    pub error: Option<String>,
}

/// Transaction broadcast results.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastData {
    pub broadcasted_at_ms: u128,
    pub broadcasted_took_ms: u128,
    pub hash: String,
    pub broadcast_error: Option<String>,
    pub receipt: Option<ReceiptData>, // Fetched in monitor program
}

/// Transaction receipt data from blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    pub status: bool,
    pub gas_used: u128,
    pub error: Option<String>,
    pub transaction_hash: String,
    pub transaction_index: u64,
    pub block_number: u64,
    pub effective_gas_price: u128,
}

/// Pre-trade analysis and planning data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreTradeData {
    pub pool: String,
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
    // Slippage and profitability
    pub slippage_tolerance_bps: f64,
    pub profit_delta_bps: f64,
    // Gas cost
    pub gas_cost_usd: f64,
}
