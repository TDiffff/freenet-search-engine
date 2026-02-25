#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::state::{CurrentPage, CURRENT_PAGE};

#[component]
pub fn DocsPage() -> Element {
    rsx! {
        div { class: "docs-page",

            // --- For Users ---

            div { class: "docs-section",
                h2 { class: "docs-heading", "For Users" }

                div { class: "docs-grid",
                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "What is Freenet Search?" }
                        p { class: "docs-text",
                            "Freenet Search is a fully decentralized search engine for discovering web applications running on the Freenet network."
                        }
                        ul { class: "docs-list",
                            li { "No central server \u{2014} everything runs in your browser connected to your local Freenet node" }
                            li { "Apps are verified through community attestation \u{2014} multiple independent users must agree on an app's metadata" }
                            li { "The search index is a shared Freenet contract that all participants contribute to and benefit from" }
                            li { "No tracking, no censorship, no single point of failure" }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "How to Search" }
                        p { class: "docs-text",
                            "Type keywords in the search bar to find apps. The search engine supports full-text search across titles, descriptions, and page content."
                        }
                        ul { class: "docs-list",
                            li { "Results are ranked by a combination of text relevance (TF-IDF) and community trust signals" }
                            li { "Apps with more attestations and confirmed status rank higher" }
                            li { "The search index is split across 16 shards \u{2014} partial results are shown if some shards haven't loaded yet" }
                            li {
                                "You can explore the raw index data in the "
                                button {
                                    class: "docs-inline-link",
                                    onclick: move |_| { *CURRENT_PAGE.write() = CurrentPage::Index; },
                                    "Index Explorer"
                                }
                                " tab"
                            }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Understanding Trust Badges" }
                        p { class: "docs-text",
                            "Each app in the index has a trust status based on how many independent users have verified its metadata:"
                        }
                        ul { class: "docs-list",
                            li {
                                span { class: "verification-status confirmed", "Confirmed" }
                                " \u{2014} Multiple independent users have verified this app's metadata matches what they see. This is the strongest signal of authenticity."
                            }
                            li {
                                span { class: "verification-status pending", "Pending" }
                                " \u{2014} Recently discovered, awaiting more validations. The app is real but hasn't been independently verified yet."
                            }
                            li {
                                span { class: "verification-status disputed", "Disputed" }
                                " \u{2014} Different users report conflicting metadata for this app. This could indicate the app changed between observations, or potential tampering. Use caution."
                            }
                            li {
                                span { class: "verification-status unverified", "Unverified" }
                                " \u{2014} Found on the network but not yet checked by the community contribution system."
                            }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Contributing to the Index" }
                        p { class: "docs-text",
                            "The search index is built by its users. When you enable contributions, your browser helps map the Freenet network:"
                        }
                        ul { class: "docs-list",
                            li { "Enable contributions in Settings \u{2014} it's completely opt-in" }
                            li { "Your browser automatically discovers web apps on the network and extracts their metadata (title, description, content)" }
                            li { "Each contribution is cryptographically signed with your ed25519 identity and requires an antiflood proof-of-work token" }
                            li { "Your reputation (trust score) builds over time as your contributions are validated by other users" }
                            li { "Higher trust means your future attestations carry more weight, accelerating confirmation of new entries" }
                        }
                    }
                }
            }

            // --- For Developers ---

            div { class: "docs-section",
                h2 { class: "docs-heading", "For Developers" }

                div { class: "docs-grid",
                    div { class: "docs-card docs-card-wide",
                        h3 { class: "docs-subheading", "Architecture Overview" }
                        p { class: "docs-text",
                            "The search engine is composed of 18 Freenet contracts that form a fully decentralized search infrastructure:"
                        }
                        ul { class: "docs-list",
                            li {
                                strong { "SearchCatalog" }
                                " (1 contract) \u{2014} The app directory. Stores metadata for every indexed web app: title, description, snippet, trust status, and attestations. Uses a CRDT grow-only map for commutative merges \u{2014} entries from different nodes converge automatically regardless of update order."
                            }
                            li {
                                strong { "FullTextShard" }
                                " (16 contracts) \u{2014} The inverted index, partitioned by keyword hash ("
                                span { class: "docs-code", "shard_id = hash(word) % 16" }
                                "). Each shard maps terms to posting lists with integer TF-IDF scores. Also CRDT-based: duplicate entries are resolved by keeping the highest score."
                            }
                            li {
                                strong { "SearchEngine WebApp" }
                                " (1 contract) \u{2014} This UI itself, served as a Freenet web container. The contract ID uses a vanity nonce for a human-readable prefix ("
                                span { class: "docs-code", "FinderTZGH..." }
                                ")."
                            }
                        }
                        p { class: "docs-text", style: "margin-top: 0.75rem;",
                            "All contracts implement Freenet's "
                            span { class: "docs-code", "ContractInterface" }
                            " with four methods: "
                            span { class: "docs-code", "validate_state" }
                            ", "
                            span { class: "docs-code", "update_state" }
                            ", "
                            span { class: "docs-code", "summarize_state" }
                            " (bloom filter), and "
                            span { class: "docs-code", "get_state_delta" }
                            ". P2P sync happens automatically via Freenet's protocol \u{2014} no central coordination needed."
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Discovery & Indexing Pipeline" }
                        p { class: "docs-text",
                            "The contribution pipeline runs entirely in the browser (WASM):"
                        }
                        ul { class: "docs-list",
                            li {
                                strong { "Discovery: " }
                                "The node's diagnostics API is polled every 10s to find new contracts. Each contract is fetched via "
                                span { class: "docs-code", "ContractRequest::Get" }
                                " and classified as WebApp or Data."
                            }
                            li {
                                strong { "Detection: " }
                                "WebApp format is identified by the web container structure: "
                                span { class: "docs-code", "[metadata_size: u64 BE][CBOR metadata][web_size: u64 BE][tar.xz]" }
                            }
                            li {
                                strong { "Extraction: " }
                                "The tar.xz archive is decompressed in WASM (via lzma-rs), index.html is located, and title/description/snippet are parsed. An HTTP fallback fetches the page directly if decompression fails."
                            }
                            li {
                                strong { "Contribution: " }
                                "An antiflood PoW token is generated, the metadata is hashed (SHA-256 of canonical title + description + snippet), signed with the user's ed25519 key, and submitted as an "
                                span { class: "docs-code", "UpdateData" }
                                " delta to the catalog and relevant shard contracts."
                            }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Anti-Sybil & Trust Model" }
                        p { class: "docs-text",
                            "Four layers of defense prevent index poisoning:"
                        }
                        ul { class: "docs-list",
                            li {
                                strong { "Antiflood tokens: " }
                                "Each submission requires a proof-of-work token (CPU cost per contribution)."
                            }
                            li {
                                strong { "Cryptographic signatures: " }
                                "Attestations are signed with ed25519. One attestation per pubkey per (contract, metadata_hash) \u{2014} an attacker needs a new keypair per fake attestation."
                            }
                            li {
                                strong { "Temporal staking: " }
                                "Tokens must be pre-generated with a minimum age, preventing burst attacks."
                            }
                            li {
                                strong { "Contributor reputation: " }
                                "New contributors start with trust_score=0 (weight=1). Trust increases when contributed entries reach Confirmed status. Higher trust = heavier attestation weight."
                            }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Caching & Sync Strategy" }
                        ul { class: "docs-list",
                            li {
                                strong { "L1 \u{2014} localStorage: " }
                                "Instant page loads. Cache includes a version number; schema changes auto-clear stale data."
                            }
                            li {
                                strong { "L2 \u{2014} WASM in-memory: " }
                                "Reactive Dioxus signals updated by WebSocket subscriptions."
                            }
                            li {
                                strong { "L3 \u{2014} Freenet node: " }
                                "Persistent contract state cache on the local node."
                            }
                            li {
                                strong { "L4 \u{2014} Network fetch: " }
                                "P2P retrieval when the node doesn't have a contract."
                            }
                        }
                        p { class: "docs-text", style: "margin-top: 0.5rem;",
                            "Catalog and shard contracts are subscribed via WebSocket. A periodic re-fetch every 30s compensates for subscription timeouts."
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Content-Aware Deduplication" }
                        p { class: "docs-text",
                            "Multiple contract keys may correspond to different versions of the same app (e.g., after redeployment with a new vanity nonce). The UI deduplicates by title, keeping the best version:"
                        }
                        ul { class: "docs-list",
                            li {
                                strong { "Working content " }
                                "wins first \u{2014} an app with actual HTML content beats a blank page"
                            }
                            li {
                                strong { "Attestation count " }
                                "\u{2014} network-wide trust signal"
                            }
                            li {
                                strong { "State size " }
                                "\u{2014} larger = more complete"
                            }
                            li {
                                strong { "Version " }
                                "\u{2014} tiebreaker from CBOR metadata"
                            }
                        }
                    }

                    div { class: "docs-card",
                        h3 { class: "docs-subheading", "Contributing Code" }
                        p { class: "docs-text",
                            "The project is open source. Contributions are welcome!"
                        }
                        ul { class: "docs-list",
                            li {
                                "Source: "
                                a {
                                    href: "https://github.com/TDiffff/freenet-search-engine",
                                    target: "_blank",
                                    class: "docs-inline-link",
                                    "github.com/TDiffff/freenet-search-engine"
                                }
                            }
                            li { "Built with: Rust, Dioxus 0.7 (WASM), Freenet stdlib, CBOR serialization" }
                            li {
                                "Key crates: "
                                span { class: "docs-code", "search-common" }
                                " (deterministic extraction pipeline), "
                                span { class: "docs-code", "contract-catalog" }
                                " (SearchCatalog WASM), "
                                span { class: "docs-code", "contract-fulltext-shard" }
                                " (FullTextShard WASM), "
                                span { class: "docs-code", "delegate-identity" }
                                " (ed25519 key management)"
                            }
                            li {
                                "Dev environment: "
                                span { class: "docs-code", "freenet local" }
                                " for isolated testing, "
                                span { class: "docs-code", "dx serve" }
                                " for hot-reload UI development"
                            }
                        }
                    }
                }
            }
        }
    }
}
