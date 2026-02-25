#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};

use dioxus::prelude::*;
use search_common::types::{CatalogState, ShardState, Status};
use serde::{Deserialize, Serialize};

// --- Data types ---

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ContractType {
    #[default]
    Unknown,
    WebApp,
    Data,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppEntry {
    pub title: Option<String>,
    pub description: Option<String>,
    pub first_seen: u64,
    pub last_seen: u64,
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub subscribers: u32,
    #[serde(default)]
    pub version: Option<u64>,
    #[serde(default)]
    pub publisher_key: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DiscoveryPhase {
    #[default]
    Idle,
    FetchingContracts,
    DetectingTypes,
    Complete,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub contract_key: String,
    pub title: String,
    pub description: String,
    pub highlighted_snippet: String,
    pub combined_score: u32,
    pub status: Status,
    pub attestation_count: u32,
}

#[derive(Clone, Debug)]
pub struct ContributionRecord {
    pub contract_key: String,
    pub timestamp: u64,
    pub status: ContributionStatus,
}

#[derive(Clone, Debug)]
pub enum ContributionStatus {
    Submitted,
    Confirmed,
    Failed(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CurrentPage {
    #[default]
    Home,
    Docs,
    Index,
}

// --- Global signals ---

/// Current page for navigation
pub static CURRENT_PAGE: GlobalSignal<CurrentPage> = Global::new(CurrentPage::default);

/// All discovered contract keys -> type mapping
pub static CONTRACT_TYPES: GlobalSignal<HashMap<String, ContractType>> = Global::new(HashMap::new);

/// Queue of (contract_key, instance_id_bytes) waiting for type detection via GET
pub static TYPE_CHECK_QUEUE: GlobalSignal<VecDeque<(String, Vec<u8>)>> = Global::new(VecDeque::new);

/// Discovered web app catalog: contract_key -> AppEntry
pub static APP_CATALOG: GlobalSignal<HashMap<String, AppEntry>> = Global::new(HashMap::new);

/// Current search query text
pub static SEARCH_QUERY: GlobalSignal<String> = Global::new(String::new);

/// Whether the node WS is connected
pub static NODE_CONNECTED: GlobalSignal<bool> = Global::new(|| false);

/// Discovery pipeline phase
pub static DISCOVERY_PHASE: GlobalSignal<DiscoveryPhase> = Global::new(DiscoveryPhase::default);

/// Total contracts found so far
pub static TOTAL_CONTRACTS: GlobalSignal<usize> = Global::new(|| 0);

/// Number of type checks completed
pub static TYPES_CHECKED: GlobalSignal<usize> = Global::new(|| 0);

/// HTTP base URL for the node
pub static NODE_HTTP_BASE: GlobalSignal<String> =
    Global::new(|| "http://127.0.0.1:7509".to_string());

/// Catalog contract state from the network
pub static CATALOG_STATE: GlobalSignal<Option<CatalogState>> = Global::new(|| None);

/// Shard contract states from the network (shard_id -> ShardState)
pub static SHARD_STATES: GlobalSignal<HashMap<u8, ShardState>> = Global::new(HashMap::new);

/// Number of shard states loaded
pub static SHARDS_AVAILABLE: GlobalSignal<u8> = Global::new(|| 0);

/// Total number of shards
pub static SHARDS_TOTAL: GlobalSignal<u8> = Global::new(|| 16);

/// Full-text search results
pub static SEARCH_RESULTS: GlobalSignal<Vec<SearchResult>> = Global::new(Vec::new);

/// Whether the contribution pipeline is enabled
pub static CONTRIBUTION_ENABLED: GlobalSignal<bool> = Global::new(|| false);

/// History of contribution attempts
pub static CONTRIBUTION_HISTORY: GlobalSignal<Vec<ContributionRecord>> = Global::new(Vec::new);

/// Contributor's public key (if generated)
pub static CONTRIBUTOR_PUBKEY: GlobalSignal<Option<[u8; 32]>> = Global::new(|| None);
