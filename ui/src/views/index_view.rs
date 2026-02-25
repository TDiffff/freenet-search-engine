#![allow(non_snake_case)]

use dioxus::prelude::*;

use super::truncate_key;
use crate::state::{CATALOG_STATE, SHARD_STATES, SHARDS_AVAILABLE, SHARDS_TOTAL};

#[component]
pub fn IndexView() -> Element {
    let catalog_state = CATALOG_STATE.read();
    let shard_states = SHARD_STATES.read();
    let shards_available = *SHARDS_AVAILABLE.read();
    let shards_total = *SHARDS_TOTAL.read();

    let catalog_entries = catalog_state
        .as_ref()
        .map(|cs| cs.entries.len())
        .unwrap_or(0);
    let contributor_count = catalog_state
        .as_ref()
        .map(|cs| cs.contributors.len())
        .unwrap_or(0);
    let total_terms: usize = shard_states.values().map(|s| s.index.len()).sum();

    rsx! {
        div { class: "index-view",
            h2 { class: "docs-heading", "Search Index Explorer" }
            p { class: "docs-text", style: "margin-bottom: 1.5rem;",
                "Live view of the decentralized search index stored across Freenet contracts."
            }

            // Summary cards
            div { class: "index-summary",
                div { class: "index-stat-card",
                    span { class: "index-stat-value", "{catalog_entries}" }
                    span { class: "index-stat-label", "Catalog entries" }
                }
                div { class: "index-stat-card",
                    span { class: "index-stat-value", "{contributor_count}" }
                    span { class: "index-stat-label", "Contributors" }
                }
                div { class: "index-stat-card",
                    span { class: "index-stat-value", "{shards_available}/{shards_total}" }
                    span { class: "index-stat-label", "Shards loaded" }
                }
                div { class: "index-stat-card",
                    span { class: "index-stat-value", "{total_terms}" }
                    span { class: "index-stat-label", "Indexed terms" }
                }
            }

            // Catalog section
            div { class: "index-section",
                h3 { class: "index-section-title", "Catalog Entries" }
                if catalog_state.is_none() {
                    p { class: "text-secondary", "Catalog not loaded yet." }
                } else if catalog_entries == 0 {
                    p { class: "text-secondary", "No entries in catalog." }
                } else {
                    div { class: "index-table-wrap",
                        table { class: "index-table",
                            thead {
                                tr {
                                    th { "Contract" }
                                    th { "Title" }
                                    th { "Status" }
                                    th { "Variants" }
                                    th { "Attestations" }
                                    th { "Size" }
                                }
                            }
                            tbody {
                                for (key, entry) in catalog_state.as_ref().unwrap().entries.iter() {
                                    {
                                        let best = entry.hash_variants.values().max_by_key(|v| v.total_weight);
                                        let title = best.map(|v| v.title.as_str()).unwrap_or("—");
                                        let status = format!("{:?}", entry.status);
                                        let variants = entry.hash_variants.len();
                                        let atts: usize = entry.hash_variants.values()
                                            .map(|v| v.attestations.len()).sum();
                                        let size = format_size(entry.size_bytes);
                                        let short_key = truncate_key(key, 16);

                                        rsx! {
                                            tr {
                                                td {
                                                    span { class: "mono index-key", title: "{key}", "{short_key}" }
                                                }
                                                td { class: "index-title-cell",
                                                    if title == "—" {
                                                        span { class: "text-secondary", "{title}" }
                                                    } else {
                                                        "{title}"
                                                    }
                                                }
                                                td {
                                                    span { class: "index-status index-status-{status}", "{status}" }
                                                }
                                                td { "{variants}" }
                                                td { "{atts}" }
                                                td { class: "mono", "{size}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Contributors section
            if contributor_count > 0 {
                div { class: "index-section",
                    h3 { class: "index-section-title", "Contributors ({contributor_count})" }
                    div { class: "index-table-wrap",
                        table { class: "index-table",
                            thead {
                                tr {
                                    th { "Public Key" }
                                    th { "Trust Score" }
                                    th { "Contributions" }
                                }
                            }
                            tbody {
                                for (_pubkey, score) in catalog_state.as_ref().unwrap().contributors.iter() {
                                    {
                                        let hex: String = score.pubkey.iter().map(|b| format!("{:02x}", b)).collect();
                                        let short_hex = if hex.len() > 16 {
                                            format!("{}...{}", &hex[..8], &hex[hex.len()-8..])
                                        } else {
                                            hex
                                        };
                                        rsx! {
                                            tr {
                                                td { span { class: "mono index-key", title: "{score.pubkey:?}", "{short_hex}" } }
                                                td { "{score.trust_score}" }
                                                td { "{score.total_contributions}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Shards section
            div { class: "index-section",
                h3 { class: "index-section-title", "Full-Text Shards" }
                if shard_states.is_empty() {
                    p { class: "text-secondary", "No shards loaded yet." }
                } else {
                    for shard_id in 0u8..shards_total {
                        {
                            if let Some(shard) = shard_states.get(&shard_id) {
                                let term_count = shard.index.len();
                                let entry_count: usize = shard.index.values().map(|v| v.len()).sum();
                                rsx! {
                                    details { class: "index-shard",
                                        summary { class: "index-shard-summary",
                                            span { class: "index-shard-id", "Shard {shard_id}" }
                                            span { class: "index-shard-stats",
                                                "{term_count} terms, {entry_count} entries"
                                            }
                                        }
                                        if shard.index.is_empty() {
                                            p { class: "text-secondary", style: "padding: 0.5rem 1rem;",
                                                "Empty shard."
                                            }
                                        } else {
                                            div { class: "index-table-wrap",
                                                table { class: "index-table",
                                                    thead {
                                                        tr {
                                                            th { "Term" }
                                                            th { "Contract" }
                                                            th { "TF-IDF" }
                                                        }
                                                    }
                                                    tbody {
                                                        for (word, entries) in shard.index.iter() {
                                                            for entry in entries.iter() {
                                                                {
                                                                    let short = truncate_key(&entry.contract_key, 16);
                                                                    rsx! {
                                                                        tr {
                                                                            td { class: "index-term", "{word}" }
                                                                            td {
                                                                                span { class: "mono index-key", title: "{entry.contract_key}", "{short}" }
                                                                            }
                                                                            td { class: "mono", "{entry.tf_idf_score}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "index-shard",
                                        div { class: "index-shard-summary text-secondary",
                                            span { class: "index-shard-id", "Shard {shard_id}" }
                                            span { "Not loaded" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}
