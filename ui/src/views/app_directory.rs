#![allow(non_snake_case)]

use std::collections::HashMap;

use dioxus::prelude::*;

use super::app_card::AppCard;
use search_common::types::CatalogState;

use crate::state::{
    AppEntry, ContractType, DiscoveryPhase, APP_CATALOG, CATALOG_STATE, CONTRIBUTION_ENABLED,
    CONTRACT_TYPES, DISCOVERY_PHASE, NODE_CONNECTED, SEARCH_QUERY,
};

#[component]
pub fn AppDirectory() -> Element {
    let catalog = APP_CATALOG.read();
    let types = CONTRACT_TYPES.read();
    let query = SEARCH_QUERY.read().clone().to_lowercase();
    let connected = *NODE_CONNECTED.read();
    let phase = DISCOVERY_PHASE.read().clone();
    let contribution_enabled = *CONTRIBUTION_ENABLED.read();
    let mut show_cta = use_signal(|| true);

    // Only collect WebApp contracts
    let mut entries: Vec<(String, Option<AppEntry>)> = types
        .iter()
        .filter(|(_, ct)| matches!(ct, ContractType::WebApp))
        .map(|(key, _)| {
            let app_entry = catalog.get(key).cloned();
            (key.clone(), app_entry)
        })
        .collect();

    // Also add cached entries not yet in types (loaded from localStorage)
    for (key, entry) in catalog.iter() {
        if !types.contains_key(key) {
            entries.push((key.clone(), Some(entry.clone())));
        }
    }

    // Deduplicate: group by title, keep the best entry per title
    let catalog_state = CATALOG_STATE.read();
    let entries = deduplicate_by_title(entries, &catalog_state);
    drop(catalog_state);

    // Apply search
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|(key, app)| {
            if query.is_empty() {
                return true;
            }
            let key_match = key.to_lowercase().contains(&query);
            let title_match = app
                .as_ref()
                .and_then(|a| a.title.as_ref())
                .map(|t| t.to_lowercase().contains(&query))
                .unwrap_or(false);
            let desc_match = app
                .as_ref()
                .and_then(|a| a.description.as_ref())
                .map(|d| d.to_lowercase().contains(&query))
                .unwrap_or(false);
            key_match || title_match || desc_match
        })
        .collect();

    // Sort: apps with titles first, then by title alphabetically
    let mut sorted = entries;
    sorted.sort_by(|a, b| {
        let a_title = a.1.as_ref().and_then(|e| e.title.as_ref());
        let b_title = b.1.as_ref().and_then(|e| e.title.as_ref());
        match (a_title, b_title) {
            (Some(at), Some(bt)) => at.to_lowercase().cmp(&bt.to_lowercase()),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(&b.0),
        }
    });

    rsx! {
        div { class: "app-directory",
            if sorted.is_empty() {
                div { class: "directory-empty",
                    if !connected {
                        p { "Not connected to Freenet node." }
                        p { class: "text-secondary", "Check that your Freenet node is running at localhost:7509" }
                    } else if matches!(phase, DiscoveryPhase::Idle | DiscoveryPhase::FetchingContracts | DiscoveryPhase::DetectingTypes) {
                        div { class: "scanning-pulse",
                            p { "Scanning for web apps..." }
                            p { class: "text-secondary", "This may take a moment." }
                        }
                    } else if !query.is_empty() {
                        p { "No apps match your search." }
                        p { class: "text-secondary", "Try different keywords." }
                    } else {
                        p { "No web apps found." }
                        p { class: "text-secondary", "Apps will appear here as your node discovers them on the network." }
                    }
                }
            } else {
                if !contribution_enabled && *show_cta.read() {
                    div { class: "cta-banner",
                        span { class: "cta-text",
                            "Help build the decentralized index \u{2014} enable contributions to share your discoveries with the network."
                        }
                        button {
                            class: "cta-btn btn-primary",
                            onclick: move |_| { *CONTRIBUTION_ENABLED.write() = true; },
                            "Enable"
                        }
                        button {
                            class: "cta-dismiss",
                            onclick: move |_| { show_cta.set(false); },
                            "\u{00d7}"
                        }
                    }
                }

                div { class: "app-grid",
                    for (key, app_entry) in sorted.iter() {
                        {
                            let entry = app_entry.clone();
                            let now = js_sys::Date::now() as u64 / 1000;

                            // Merge verification data from catalog contract state
                            let catalog_state = CATALOG_STATE.read();
                            let (cat_desc, status_str, att_count) = if let Some(ref cs) = *catalog_state {
                                if let Some(cat_entry) = cs.entries.get(key) {
                                    let best = cat_entry.hash_variants.values().max_by_key(|v| v.total_weight);
                                    let desc = best.map(|v| v.description.clone());
                                    let status = format!("{:?}", cat_entry.status);
                                    let atts: u32 = cat_entry.hash_variants.values()
                                        .map(|v| v.attestations.len() as u32).sum();
                                    (desc, Some(status), atts)
                                } else {
                                    (None, None, 0)
                                }
                            } else {
                                (None, None, 0)
                            };
                            drop(catalog_state);

                            // Prefer catalog description over discovery description
                            // Filter empty strings — older contributions lack descriptions
                            let description = cat_desc
                                .filter(|d| !d.is_empty())
                                .or_else(|| entry.as_ref().and_then(|e| e.description.clone()))
                                .filter(|d| !d.is_empty());

                            rsx! {
                                AppCard {
                                    key: "{key}",
                                    contract_key: key.clone(),
                                    title: entry.as_ref().and_then(|e| e.title.clone()),
                                    description: description,
                                    first_seen: entry.as_ref().map(|e| e.first_seen).unwrap_or(now),
                                    size_bytes: entry.as_ref().and_then(|e| e.size_bytes),
                                    version: entry.as_ref().and_then(|e| e.version),
                                    subscribers: entry.as_ref().map(|e| e.subscribers).unwrap_or(0),
                                    status: status_str,
                                    attestation_count: att_count,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Group entries by title and keep only the best entry per app.
///
/// Ranking: has_content > catalog attestations. No further tiebreaker —
/// version/size are unreliable for distinguishing current vs old deploys.
/// Subscribers are NOT used — they only reflect direct peers, not the network.
fn deduplicate_by_title(
    entries: Vec<(String, Option<AppEntry>)>,
    catalog_state: &Option<CatalogState>,
) -> Vec<(String, Option<AppEntry>)> {
    let mut by_title: HashMap<String, (String, AppEntry)> = HashMap::new();
    let mut no_title: Vec<(String, Option<AppEntry>)> = Vec::new();

    for (key, entry) in entries {
        let Some(e) = entry else {
            no_title.push((key, None));
            continue;
        };
        let Some(title) = e.title.as_ref() else {
            no_title.push((key, Some(e)));
            continue;
        };
        let title_lower = title.to_lowercase();

        if let Some(existing) = by_title.get(&title_lower) {
            let new_content = has_content(&e);
            let old_content = has_content(&existing.1);
            let new_atts = attestation_count(catalog_state, &key);
            let old_atts = attestation_count(catalog_state, &existing.0);
            let same_publisher = match (&e.publisher_key, &existing.1.publisher_key) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            };
            let new_ver = e.version.unwrap_or(0);
            let old_ver = existing.1.version.unwrap_or(0);

            let better =
                // 1. Content wins (non-blank page beats blank)
                (new_content && !old_content)
                // 2. Same publisher → higher version = more recent deploy
                || (new_content == old_content && same_publisher && new_ver > old_ver)
                // 3. Different publishers → more attestations wins
                || (new_content == old_content && !same_publisher && new_atts > old_atts);
            if better {
                by_title.insert(title_lower, (key, e));
            }
        } else {
            by_title.insert(title_lower, (key, e));
        }
    }

    let mut result: Vec<(String, Option<AppEntry>)> =
        by_title.into_values().map(|(k, e)| (k, Some(e))).collect();
    result.extend(no_title);
    result
}

/// Whether an entry has actual page content (non-blank).
/// A blank page (e.g. just `<div id="main"></div>`) yields no description.
fn has_content(entry: &AppEntry) -> bool {
    entry
        .description
        .as_ref()
        .map(|d| !d.is_empty())
        .unwrap_or(false)
}

/// Total attestation count for a contract key across all hash variants.
fn attestation_count(catalog_state: &Option<CatalogState>, key: &str) -> u32 {
    catalog_state
        .as_ref()
        .and_then(|cs| cs.entries.get(key))
        .map(|e| {
            e.hash_variants
                .values()
                .map(|v| v.attestations.len() as u32)
                .sum()
        })
        .unwrap_or(0)
}
