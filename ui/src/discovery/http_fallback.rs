use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use crate::state::NODE_HTTP_BASE;

use super::title::{extract_description_from_html, extract_title_from_html, update_catalog_entry};

/// Attempt to fetch the web app's index.html from the node's HTTP endpoint
/// and extract the title. This works when same-origin (production) and may
/// work during development if the node sends CORS headers.
///
/// This is a fallback for when xz decompression fails in WASM.
pub fn try_fetch_title(key: String, version: Option<u64>, size: Option<u64>) {
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_and_extract(&key).await {
            Ok((title, description)) => {
                if title.is_some() {
                    let short = &key[..key.len().min(12)];
                    tracing::info!("HTTP fallback for {}: title={:?}, desc={:?}", short, title, description);
                    update_catalog_entry(
                        &key,
                        title.as_deref(),
                        description.as_deref(),
                        size,
                        version,
                        true, // fresh extraction
                        None, // no contract container available via HTTP
                    );
                    crate::discovery::cache::save_cache();
                }
            }
            Err(e) => {
                let short = &key[..key.len().min(12)];
                tracing::debug!("HTTP fallback failed for {}: {:?}", short, e);
            }
        }
    });
}

async fn fetch_and_extract(key: &str) -> Result<(Option<String>, Option<String>), JsValue> {
    let base = NODE_HTTP_BASE.read().clone();
    let url = format!("{}/v1/contract/web/{}/", base, key);

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Accept", "text/html")?;

    let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", resp.status())));
    }

    let text = JsFuture::from(resp.text()?).await?;
    let html = text
        .as_string()
        .ok_or(JsValue::from_str("response not string"))?;

    // Only look at the first 10KB for title/description
    let mut limit = html.len().min(10240);
    while limit > 0 && !html.is_char_boundary(limit) {
        limit -= 1;
    }
    let snippet = &html[..limit];

    let title = extract_title_from_html(snippet);
    let description = extract_description_from_html(snippet);

    Ok((title, description))
}
