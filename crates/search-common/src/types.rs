use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Parameters set at contract deployment time, part of key derivation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogParameters {
    pub protocol_version: u16,
    pub shard_count: u8,
    pub confirmation_weight_threshold: u32,
    pub entry_ttl_days: u16,
}

/// Full state of the SearchCatalog contract.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogState {
    pub entries: BTreeMap<String, CatalogEntry>,
    #[serde(default)]
    pub contributors: BTreeMap<[u8; 32], ContributorScore>,
}

/// A single indexed contract in the catalog.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogEntry {
    pub contract_key: String,
    pub hash_variants: BTreeMap<[u8; 32], HashVariant>,
    pub size_bytes: u64,
    pub version: Option<u64>,
    pub status: Status,
    pub first_seen: u64,
    pub last_seen: u64,
}

/// A specific metadata variant for a catalog entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashVariant {
    pub title: String,
    pub description: String,
    pub mini_snippet: String,
    pub attestations: Vec<Attestation>,
    pub total_weight: u32,
}

/// An attestation from a contributor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attestation {
    pub contributor_pubkey: [u8; 32],
    pub antiflood_token: AntifloodToken,
    pub token_created_at: u64,
    pub weight: u32,
}

/// Proof-of-work antiflood token.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntifloodToken {
    pub nonce: Vec<u8>,
    pub difficulty: u8,
}

/// Lifecycle status of a catalog entry.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    #[default]
    Pending,
    Confirmed,
    Disputed,
    Expired,
}

/// Reputation score for a contributor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributorScore {
    pub pubkey: [u8; 32],
    pub trust_score: u32,
    pub total_contributions: u32,
}

/// Full state of a FullTextShard contract.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardState {
    pub shard_id: u8,
    pub index: BTreeMap<String, Vec<TermEntry>>,
}

/// A term entry in the inverted index.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermEntry {
    pub contract_key: String,
    pub snippet: String,
    pub tf_idf_score: u32,
}

/// Delta for updating the SearchCatalog.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogDelta {
    pub contract_key: String,
    pub title: String,
    pub description: String,
    pub mini_snippet: String,
    pub snippet: String,
    pub size_bytes: u64,
    pub version: Option<u64>,
    pub metadata_hash: [u8; 32],
    pub attestation: Attestation,
}

/// Delta for updating a FullTextShard.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardDelta {
    pub entries: Vec<ShardDeltaEntry>,
    pub antiflood_token: AntifloodToken,
}

/// A single entry in a shard delta.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardDeltaEntry {
    pub word: String,
    pub contract_key: String,
    pub snippet: String,
    pub tf_idf_score: u32,
}
