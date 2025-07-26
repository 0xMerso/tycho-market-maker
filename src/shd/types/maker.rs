/// =============================================================================
/// Market Maker Types and Data Structures
/// =============================================================================
///
/// @description: Core type definitions for market making operations including
/// the main market maker trait, data structures for trades, orders, and market
/// context. This module defines the fundamental interfaces and data models
/// used throughout the market making system.
/// =============================================================================
use std::collections::HashMap;

use alloy::rpc::types::TransactionRequest;
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

/// =============================================================================
/// @trait: IMarketMaker
/// @description: Core market maker interface defining all required operations
/// @methods:
/// - evaluate: Analyze pools for profitable opportunities
/// - readjust: Execute market readjustment strategies
/// - prices: Retrieve component prices
/// - fetch_inventory: Get current token balances
/// - fetch_market_context: Get market state information
/// - fetch_eth_usd: Get ETH/USD price
/// - fetch_market_price: Get current market price
/// - pre_trade_data: Create pre-trade analysis data
/// - solution: Build Tycho solution
/// - trade_tx_request: Create transaction requests
/// - prepare: Prepare transactions for execution
/// - run: Main market maker loop
/// =============================================================================
#[async_trait]
pub trait IMarketMaker: Send + Sync {
    /// =============================================================================
    /// @function: evaluate
    /// @description: Look for pools having a spread higher than the configured threshold
    /// @param psc: Vector of protocol simulation components
    /// @param sps: Vector of spot prices
    /// @param reference: Reference price for comparison
    /// @return Vec<CompReadjustment>: Vector of profitable readjustment opportunities
    /// =============================================================================
    fn evaluate(&self, psc: &Vec<ProtoSimComp>, sps: Vec<f64>, reference: f64) -> Vec<CompReadjustment>;

    /// =============================================================================
    /// @function: readjust
    /// @description: Analyze optimal way to readjust the market and compute profitability
    /// @param context: Current market context
    /// @param inventory: Current token inventory
    /// @param crs: Vector of component readjustments
    /// @param env: Environment configuration
    /// @return Vec<ExecutionOrder>: Vector of execution orders
    /// =============================================================================
    async fn readjust(&self, context: MarketContext, inventory: Inventory, crs: Vec<CompReadjustment>, env: EnvConfig) -> Vec<ExecutionOrder>;

    /// =============================================================================
    /// @function: prices
    /// @description: Retrieve prices for protocol simulation components
    /// @param psc: Vector of protocol simulation components
    /// @return Vec<ComponentPriceData>: Vector of component price data
    /// =============================================================================
    fn prices(&self, psc: &Vec<ProtoSimComp>) -> Vec<ComponentPriceData>;

    /// =============================================================================
    /// @function: fetch_inventory
    /// @description: Fetch current token inventory from blockchain
    /// @param env: Environment configuration
    /// @return Result<Inventory, String>: Current inventory or error
    /// =============================================================================
    async fn fetch_inventory(&self, env: EnvConfig) -> Result<Inventory, String>;

    /// =============================================================================
    /// @function: fetch_market_context
    /// @description: Fetch current market context and state
    /// @param components: Vector of protocol components
    /// @param protosims: HashMap of protocol simulations
    /// @param tokens: Vector of available tokens
    /// @return Option<MarketContext>: Market context if available
    /// =============================================================================
    async fn fetch_market_context(&self, components: Vec<ProtocolComponent>, protosims: &HashMap<std::string::String, Box<dyn ProtocolSim>>, tokens: Vec<Token>) -> Option<MarketContext>;

    /// =============================================================================
    /// @function: fetch_eth_usd
    /// @description: Fetch current ETH/USD price from price feed
    /// @return Result<f64, String>: ETH/USD price or error
    /// =============================================================================
    async fn fetch_eth_usd(&self) -> Result<f64, String>;

    /// =============================================================================
    /// @function: fetch_market_price
    /// @description: Fetch current market price for the trading pair
    /// @return Result<f64, String>: Market price or error
    /// =============================================================================
    async fn fetch_market_price(&self) -> Result<f64, String>;

    /// =============================================================================
    /// @function: pre_trade_data
    /// @description: Create trade data from execution order and market context
    /// @param order: Execution order to analyze
    /// @return PreTradeData: Pre-trade analysis data
    /// =============================================================================
    fn pre_trade_data(&self, order: &ExecutionOrder) -> PreTradeData;

    /// =============================================================================
    /// @function: solution
    /// @description: Build Tycho solution from execution order
    /// @param order: Execution order to encode
    /// @param env: Environment configuration
    /// @return Solution: Tycho solution
    /// =============================================================================
    fn solution(&self, order: ExecutionOrder, env: EnvConfig) -> Solution;

    /// =============================================================================
    /// @function: trade_tx_request
    /// @description: Create transaction request from Tycho solution
    /// @param solution: Tycho solution
    /// @param encoded: Encoded transaction
    /// @param context: Market context
    /// @param inventory: Current inventory
    /// @param env: Environment configuration
    /// @return Result<TradeTxRequest, String>: Transaction request or error
    /// =============================================================================
    fn trade_tx_request(&self, solution: Solution, encoded: Transaction, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Result<TradeTxRequest, String>;

    /// =============================================================================
    /// @function: prepare
    /// @description: Prepare transactions for execution
    /// @param orders: Vector of execution orders
    /// @param tdata: Vector of trade data
    /// @param context: Market context
    /// @param inventory: Current inventory
    /// @param env: Environment configuration
    /// @return Vec<Trade>: Vector of prepared trades
    /// =============================================================================
    fn prepare(&self, orders: Vec<ExecutionOrder>, tdata: Vec<TradeData>, context: MarketContext, inventory: Inventory, env: EnvConfig) -> Vec<Trade>;

    /// =============================================================================
    /// @function: run
    /// @description: Main market maker loop that monitors Tycho stream state
    /// @param mtx: Shared Tycho stream state
    /// @param env: Environment configuration
    /// @behavior: Infinite loop looking for trading opportunities
    /// =============================================================================
    async fn run(&mut self, mtx: SharedTychoStreamState, env: EnvConfig);
}

/// =============================================================================
/// @struct: MarketMaker
/// @description: Main market maker implementation struct
/// @fields:
/// - ready: Indicates if ProtocolStreamBuilder is initialized
/// - identifier: Unique instance identifier
/// - config: Market maker configuration
/// - feed: Dynamic price feed implementation
/// - initialised: Protocol stream initialization status
/// - base: Base token from Tycho client
/// - quote: Quote token from Tycho client
/// - single: Testing flag for single swap execution
/// - execution: Dynamic execution strategy
/// =============================================================================
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

/// =============================================================================
/// @struct: PriceFeedConfig
/// @description: Configuration for price feed sources
/// @fields:
/// - r#type: Price feed type ("binance" or "chainlink")
/// - source: Source URL or contract address
/// - reverse: Whether to reverse the price (1 / price)
/// =============================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceFeedConfig {
    pub r#type: String, // "binance" or "chainlink"
    pub source: String, // https if type is "binance", of 0xAddress if type is "chainlink"
    pub reverse: bool,  // true if the price is to be reversed (e.g. 1 / price), only used for chainlink
}

/// =============================================================================
/// @enum: TradeDirection
/// @description: Direction of trade execution
/// @variants:
/// - Buy: Buy order (quote to base)
/// - Sell: Sell order (base to quote)
/// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeDirection {
    Buy,
    Sell,
}

/// =============================================================================
/// @struct: ComponentPriceData
/// @description: Price data for a specific component
/// @fields:
/// - address: Component contract address
/// - r#type: Component type
/// - price: Current price
/// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPriceData {
    pub address: String,
    pub r#type: String,
    pub price: f64,
}

/// =============================================================================
/// @struct: CompReadjustment
/// @description: Component readjustment opportunity
/// @fields:
/// - psc: Protocol simulation component
/// - direction: Trade direction
/// - selling: Token being sold
/// - buying: Token being bought
/// - reference: Reference price
/// - spread: Price spread
/// - spread_bps: Spread in basis points
/// =============================================================================
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

/// =============================================================================
/// @struct: Inventory
/// @description: Current token inventory and wallet state
/// @fields:
/// - base_balance: Base token balance in wei
/// - quote_balance: Quote token balance in wei
/// - nonce: Current wallet nonce
/// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub base_balance: u128,  // Divided
    pub quote_balance: u128, // Divided
    pub nonce: u64,
}

/// =============================================================================
/// @struct: MarketContext
/// @description: Current market context and pricing information
/// @fields:
/// - base_to_eth: Base token to ETH conversion rate
/// - quote_to_eth: Quote token to ETH conversion rate
/// - eth_to_usd: ETH to USD conversion rate
/// - max_fee_per_gas: Maximum gas fee in wei
/// - max_priority_fee_per_gas: Maximum priority fee in wei
/// - native_gas_price: Current gas price in wei
/// - block: Current block number
/// =============================================================================
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

/// =============================================================================
/// @struct: ExecutionOrder
/// @description: Complete execution order with adjustment and calculation
/// @fields:
/// - adjustment: Component readjustment opportunity
/// - calculation: Swap calculation details
/// =============================================================================
#[derive(Debug, Clone)]
pub struct ExecutionOrder {
    pub adjustment: CompReadjustment,
    pub calculation: SwapCalculation,
    // pub bribing: BribeCalculation,
}

/// =============================================================================
/// @struct: SwapCalculation
/// @description: Detailed swap calculation with profitability analysis
/// @fields:
/// - base_to_quote: Direction flag (true if base to quote)
/// - selling_amount: Amount being sold
/// - buying_amount: Amount being bought
/// - powered_selling_amount: Selling amount with power adjustments
/// - powered_buying_amount: Buying amount with power adjustments
/// - amount_out_normalized: Expected output amount (normalized)
/// - amount_out_powered: Expected output amount (powered)
/// - amount_out_min_normalized: Minimum output amount (normalized)
/// - amount_out_min_powered: Minimum output amount (powered)
/// - average_sell_price: Average selling price
/// - average_sell_price_net_gas: Average selling price after gas costs
/// - gas_units: Estimated gas units
/// - gas_cost_eth: Gas cost in ETH
/// - gas_cost_usd: Gas cost in USD
/// - gas_cost_in_output_token: Gas cost in output token
/// - selling_worth_usd: USD value of selling amount
/// - buying_worth_usd: USD value of buying amount
/// - profit_delta_bps: Profit delta in basis points
/// - profitable: Whether the trade is profitable
/// =============================================================================
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

/// =============================================================================
/// @struct: TradeTxRequest
/// @description: Transaction request for trade execution
/// @fields:
/// - approve: Optional approval transaction request
/// - swap: Swap transaction request
/// =============================================================================
#[derive(Debug, Clone)]
pub struct TradeTxRequest {
    pub approve: Option<TransactionRequest>,
    pub swap: TransactionRequest,
}

/// =============================================================================
/// @struct: Trade
/// @description: Complete trade with transactions and metadata
/// @fields:
/// - approve: Optional approval transaction
/// - swap: Swap transaction
/// - metadata: Trade metadata and analysis data
/// =============================================================================
#[derive(Debug, Clone)]
pub struct Trade {
    pub approve: Option<TransactionRequest>,
    pub swap: TransactionRequest,
    pub metadata: TradeData,
}

/// =============================================================================
/// @enum: TradeStatus
/// @description: Status of trade execution
/// @variants:
/// - Pending: Trade is pending execution
/// - SimulationSucceeded: Simulation completed successfully
/// - SimulationFailed: Simulation failed
/// - ReadyToExecute: Trade is ready for execution
/// - BroadcastSucceeded: Transaction broadcast succeeded
/// - BroadcastFailed: Transaction broadcast failed
/// - Confirmed: Transaction confirmed on blockchain
/// - Failed: Trade execution failed
/// =============================================================================
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

/// =============================================================================
/// @struct: TradeData
/// @description: Complete trade data with all execution information
/// @fields:
/// - status: Current trade status
/// - timestamp: Trade timestamp
/// - context: Market context at trade time
/// - metadata: Pre-trade analysis data
/// - inventory: Inventory at trade time
/// - simulation: Optional simulation results
/// - broadcast: Optional broadcast results
/// =============================================================================
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

/// =============================================================================
/// @struct: SimulatedData
/// @description: Transaction simulation results
/// @fields:
/// - simulated_at_ms: Simulation timestamp in milliseconds
/// - simulated_took_ms: Simulation duration in milliseconds
/// - estimated_gas: Estimated gas usage
/// - status: Simulation success status
/// - error: Optional simulation error message
/// =============================================================================
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedData {
    pub simulated_at_ms: u128,
    pub simulated_took_ms: u128,
    pub estimated_gas: u128,
    pub status: bool,
    pub error: Option<String>,
}

/// =============================================================================
/// @struct: BroadcastData
/// @description: Transaction broadcast results
/// @fields:
/// - broadcasted_at_ms: Broadcast timestamp in milliseconds
/// - broadcasted_took_ms: Broadcast duration in milliseconds
/// - hash: Transaction hash
/// - broadcast_error: Optional broadcast error message
/// - receipt: Optional transaction receipt data
/// =============================================================================
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastData {
    pub broadcasted_at_ms: u128,
    pub broadcasted_took_ms: u128,
    pub hash: String,
    pub broadcast_error: Option<String>,
    pub receipt: Option<ReceiptData>, // Fetched in monitor program
}

/// =============================================================================
/// @struct: ReceiptData
/// @description: Transaction receipt data from blockchain
/// @fields:
/// - status: Transaction success status
/// - gas_used: Actual gas used
/// - error: Optional transaction error
/// - transaction_hash: Transaction hash
/// - transaction_index: Transaction index in block
/// - block_number: Block number
/// - effective_gas_price: Effective gas price
/// =============================================================================
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

/// =============================================================================
/// @struct: PreTradeData
/// @description: Pre-trade analysis and planning data
/// @fields:
/// - base_token: Base token address
/// - quote_token: Quote token address
/// - trade_direction: Direction of trade
/// - amount_in_normalized: Input amount (normalized)
/// - amount_out_expected: Expected output amount
/// - spot_price: Current spot price
/// - reference_price: Reference price for comparison
/// - slippage_tolerance_bps: Slippage tolerance in basis points
/// - profit_delta_bps: Expected profit in basis points
/// - gas_cost_usd: Estimated gas cost in USD
/// =============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreTradeData {
    // pub pool: String,
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
