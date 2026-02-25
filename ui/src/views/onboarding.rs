#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::state::CONTRIBUTION_ENABLED;

#[component]
pub fn OnboardingModal(on_close: EventHandler<()>) -> Element {
    let mut step = use_signal(|| 0u8);

    // Check localStorage on mount — if already seen, don't render
    let already_seen = use_signal(|| {
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item("onboarding_seen").ok())
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false)
    });

    if *already_seen.read() {
        return rsx! {};
    }

    let dismiss = move || {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
        {
            let _ = storage.set_item("onboarding_seen", "true");
        }
        on_close.call(());
    };

    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-content",
                button {
                    class: "modal-close",
                    onclick: move |_| dismiss(),
                    "x"
                }

                match *step.read() {
                    0 => rsx! {
                        div { class: "modal-header",
                            h2 { "Discover Decentralized Web Apps" }
                        }
                        div { class: "modal-body",
                            p {
                                "Freenet Search is a fully decentralized search engine. \
                                 It discovers and indexes web applications running on the \
                                 Freenet network. No servers, no tracking, no censorship."
                            }
                        }
                        div { class: "modal-footer",
                            button {
                                class: "btn-primary",
                                onclick: move |_| step.set(1),
                                "Next"
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "modal-header",
                            h2 { "Community-Powered Index" }
                        }
                        div { class: "modal-body",
                            p {
                                "The search index is a shared Freenet contract. Each user's \
                                 browser independently discovers web apps on the network and \
                                 extracts their metadata (title, description). When you enable \
                                 contributions, your discoveries are cryptographically signed \
                                 and submitted to the shared index. Other users see your \
                                 contributions and can validate them. The more people \
                                 contribute, the more complete and trustworthy the index becomes."
                            }
                            ul { class: "docs-list",
                                li { "Your node scans the network for web apps" }
                                li { "Metadata is extracted locally in your browser" }
                                li { "Contributions are signed with your identity" }
                                li { "Trust builds over time as your contributions are validated" }
                            }
                        }
                        div { class: "modal-footer",
                            button {
                                class: "btn-primary",
                                onclick: move |_| step.set(2),
                                "Next"
                            }
                        }
                    },
                    _ => rsx! {
                        div { class: "modal-header",
                            h2 { "Enable Contributions" }
                        }
                        div { class: "modal-body",
                            p {
                                "Enabling contributions allows your browser to share discovered \
                                 app metadata with the network. It's opt-in, privacy-respecting, \
                                 and strengthens the index for everyone."
                            }
                        }
                        div { class: "modal-footer",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    *CONTRIBUTION_ENABLED.write() = true;
                                    if let Some(storage) = web_sys::window()
                                        .and_then(|w| w.local_storage().ok())
                                        .flatten()
                                    {
                                        let _ = storage.set_item("contribution_enabled", "true");
                                    }
                                    dismiss();
                                },
                                "Enable & Get Started"
                            }
                            button {
                                class: "btn-ghost",
                                onclick: move |_| dismiss(),
                                "Maybe later"
                            }
                        }
                    },
                }
            }
        }
    }
}
