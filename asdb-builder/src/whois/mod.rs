//! WHOIS API client for fetching AS, Organisation, and Person data.
//!
//! This module provides access to Regional Internet Registry (RIR) databases
//! via REST APIs. Currently supports RIPE NCC (Europe, Middle East, Central Asia).
//!
//! # Example
//!
//! ```ignore
//! use asdb_builder::whois::RipeClient;
//!
//! let client = RipeClient::new();
//!
//! // Get AS information
//! let aut_num = client.get_aut_num(5550).await?;
//! println!("AS name: {:?}", aut_num.as_name);
//!
//! // Get complete WHOIS data including org and contacts
//! let data = client.get_as_whois_data(5550).await?;
//! ```

pub mod error;
pub mod models;
pub mod ripe;

pub use error::{Error, Result};
pub use models::{AsWhoisData, AutNum, Organisation, Person};
pub use ripe::RipeClient;
