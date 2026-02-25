#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::state::{NODE_HTTP_BASE, SEARCH_QUERY, SEARCH_RESULTS, SHARDS_AVAILABLE, SHARDS_TOTAL};

#[component]
pub fn SearchResults() -> Element {
    let results = SEARCH_RESULTS.read();
    let shards_available = *SHARDS_AVAILABLE.read();
    let shards_total = *SHARDS_TOTAL.read();
    let node_base = NODE_HTTP_BASE.read();

    let query = SEARCH_QUERY.read().clone();

    rsx! {
        div { class: "search-results",
            if shards_available < shards_total && shards_available > 0 {
                div { class: "partial-results-banner",
                    "Results from {shards_available}/{shards_total} shards"
                }
            }

            if results.is_empty() {
                div { class: "directory-empty",
                    if query.is_empty() {
                        p { "No results found." }
                        p { class: "text-secondary", "Try a different search term." }
                    } else {
                        p { "No results found for '{query}'." }
                        p { class: "text-secondary", "Try different keywords or a broader search." }
                    }
                }
            } else {
                div { class: "search-results-count",
                    "{results.len()} result{plural(results.len())}"
                }

                for result in results.iter() {
                    {
                        let url = format!("{}/v1/contract/web/{}/", node_base, result.contract_key);
                        let short_key = truncate_key(&result.contract_key, 20);
                        let status_class = match result.status {
                            search_common::types::Status::Confirmed => "verification-status confirmed",
                            search_common::types::Status::Pending => "verification-status pending",
                            search_common::types::Status::Disputed => "verification-status disputed",
                            search_common::types::Status::Expired => "verification-status unverified",
                        };
                        let status_text = match result.status {
                            search_common::types::Status::Confirmed => "Confirmed",
                            search_common::types::Status::Pending => "Pending",
                            search_common::types::Status::Disputed => "Disputed",
                            search_common::types::Status::Expired => "Expired",
                        };

                        rsx! {
                            div { class: "search-result",
                                a {
                                    class: "search-result-title",
                                    href: "{url}",
                                    target: "_blank",
                                    span { class: "search-result-arrow", "\u{2192} " }
                                    "{result.title}"
                                }

                                if !result.description.is_empty() {
                                    p { class: "search-result-description", "{result.description}" }
                                }

                                div {
                                    class: "search-result-snippet",
                                    dangerous_inner_html: "{result.highlighted_snippet}",
                                }

                                div { class: "search-result-meta",
                                    span {
                                        class: "mono",
                                        title: "{result.contract_key}",
                                        "{short_key}"
                                    }
                                    span { class: "{status_class}", "{status_text}" }
                                    span { class: "search-result-score",
                                        "{result.attestation_count} validations"
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

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

use super::truncate_key;
