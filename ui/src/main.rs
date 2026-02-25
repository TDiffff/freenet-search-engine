#![allow(non_snake_case)]

use dioxus::prelude::*;

mod api;
mod discovery;
mod search;
mod state;
mod views;

use state::{
    CurrentPage, DiscoveryPhase, CURRENT_PAGE, DISCOVERY_PHASE, NODE_CONNECTED, SEARCH_QUERY,
    SEARCH_RESULTS, SHARDS_AVAILABLE,
};
use views::app_directory::AppDirectory;
use views::docs_page::DocsPage;
use views::footer::AppFooter;
use views::index_view::IndexView;
use views::onboarding::OnboardingModal;
use views::search_bar::SearchBar;
use views::search_results::SearchResults;
use views::settings::SettingsPanel;

fn main() {
    dioxus::logger::initialize_default();
    launch(App);
}

#[component]
fn App() -> Element {
    use_effect(|| {
        api::init();
    });

    // Reactive search: re-runs when query or shard data changes
    use_effect(move || {
        let query = SEARCH_QUERY.read().clone();
        let has_shards = *SHARDS_AVAILABLE.read() > 0;

        if query.is_empty() || !has_shards {
            SEARCH_RESULTS.write().clear();
            return;
        }

        let parsed = search::query::parse_query(&query);
        *SEARCH_RESULTS.write() = search::query::execute_search(&parsed);
    });

    let connected = *NODE_CONNECTED.read();
    let phase = DISCOVERY_PHASE.read().clone();
    let query = SEARCH_QUERY.read().clone();
    let has_shards = *SHARDS_AVAILABLE.read() > 0;
    let has_results = !SEARCH_RESULTS.read().is_empty();
    let show_fulltext = !query.is_empty() && has_shards && has_results;
    let current_page = CURRENT_PAGE.read().clone();

    let status_class = if connected {
        "status-indicator connected"
    } else {
        "status-indicator disconnected"
    };
    let status_text = if connected {
        "Connected"
    } else {
        "Disconnected"
    };

    let phase_text = match phase {
        DiscoveryPhase::Idle => None,
        DiscoveryPhase::FetchingContracts => Some("Discovering contracts..."),
        DiscoveryPhase::DetectingTypes => Some("Detecting types..."),
        DiscoveryPhase::Complete => Some("Scan complete"),
    };

    let mut show_settings = use_signal(|| false);
    let mut show_onboarding = use_signal(|| {
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item("onboarding_seen").ok())
            .flatten()
            .map(|v| v != "true")
            .unwrap_or(true)
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Title { "Freenet Search" }

        // Onboarding modal (overlay, shown once)
        if *show_onboarding.read() {
            OnboardingModal {
                on_close: move |_| {
                    show_onboarding.set(false);
                }
            }
        }

        div { class: "app-shell",
            // Header
            header { class: "app-header",
                h1 {
                    class: "app-title",
                    onclick: move |_| {
                        *CURRENT_PAGE.write() = CurrentPage::Home;
                    },
                    style: "cursor: pointer;",
                    "Freenet Search"
                }

                // Navigation tabs
                nav { class: "nav-tabs",
                    button {
                        class: if matches!(current_page, CurrentPage::Home) { "nav-tab active" } else { "nav-tab" },
                        onclick: move |_| {
                            *CURRENT_PAGE.write() = CurrentPage::Home;
                        },
                        "Home"
                    }
                    button {
                        class: if matches!(current_page, CurrentPage::Docs) { "nav-tab active" } else { "nav-tab" },
                        onclick: move |_| {
                            *CURRENT_PAGE.write() = CurrentPage::Docs;
                        },
                        "Docs"
                    }
                    button {
                        class: if matches!(current_page, CurrentPage::Index) { "nav-tab active" } else { "nav-tab" },
                        onclick: move |_| {
                            *CURRENT_PAGE.write() = CurrentPage::Index;
                        },
                        "Index / Catalog Explorer"
                    }
                }

                div { class: "header-controls",
                    if let Some(text) = phase_text {
                        span { class: "discovery-status", "{text}" }
                    }

                    button {
                        class: "btn btn-ghost",
                        title: "Clear cached app data and rescan",
                        onclick: move |_| {
                            discovery::cache::clear_cache();
                        },
                        "Clear cache"
                    }

                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            show_settings.toggle();
                        },
                        if *show_settings.read() { "Close settings" } else { "Settings" }
                    }

                    div { class: "{status_class}",
                        span { class: "status-dot" }
                        span { class: "status-text", "{status_text}" }
                    }
                }
            }

            // Settings panel (toggled)
            if *show_settings.read() {
                SettingsPanel {}
            }

            // Page content
            match current_page {
                CurrentPage::Home => rsx! {
                    SearchBar {}
                    if show_fulltext {
                        SearchResults {}
                    } else {
                        AppDirectory {}
                    }
                },
                CurrentPage::Docs => rsx! {
                    DocsPage {}
                },
                CurrentPage::Index => rsx! {
                    IndexView {}
                },
            }

            // Footer (always visible)
            AppFooter {}
        }
    }
}
