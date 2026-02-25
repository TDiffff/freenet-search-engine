#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::state::{CurrentPage, CURRENT_PAGE};

#[component]
pub fn AppFooter() -> Element {
    rsx! {
        footer { class: "app-footer",
            span { class: "text-muted", "Built on Freenet" }

            div { class: "footer-links",
                a {
                    class: "footer-link",
                    href: "#",
                    onclick: move |e: Event<MouseData>| {
                        e.prevent_default();
                        *CURRENT_PAGE.write() = CurrentPage::Docs;
                    },
                    "Docs"
                }
                a {
                    class: "footer-link",
                    href: "https://github.com/TDiffff/freenet-search-engine",
                    target: "_blank",
                    "GitHub"
                }
            }
        }
    }
}
