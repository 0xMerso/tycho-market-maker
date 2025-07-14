//! Tycho Market Maker
//!
//! A market making bot for decentralized exchanges using the Tycho protocol.
//!
//! ## Architecture
//!
//! The project is organized into several key modules:
//!
//! - `shd`: Core library containing all business logic
//!   - `data`: Data access layer (Redis, PostgreSQL)
//!   - `entity`: Database entities and models
//!   - `maker`: Market making logic and strategies
//!   - `types`: Type definitions and configurations
//!   - `utils`: Utility functions and helpers
//!   - `opti`: Optimization algorithms
//!
//! ## Usage
//!
//! ```rust
//! use tycho_market_maker::shd::types::config::MarketMakerConfig;
//! ```

pub mod shd;
