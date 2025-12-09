//! Tycho Market Maker Library
//!
//! A market making bot for decentralized exchanges using the Tycho protocol.
//! Provides core functionality for automated market making on EVM-compatible
//! blockchains with support for multiple networks and execution strategies.
//!
//! # Architecture
//!
//! - `shd`: Core library containing all business logic
//!   - `data`: Data access layer (Redis, PostgreSQL)
//!   - `entity`: Database entities and models
//!   - `maker`: Market making logic and strategies
//!   - `types`: Type definitions and configurations
//!   - `utils`: Utility functions and helpers
//!   - `opti`: Optimization algorithms
pub mod shd;
