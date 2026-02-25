use std::collections::HashMap;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::{AppEntry, APP_CATALOG};

const STORAGE_KEY: &str = "freenet_search_apps";

/// Bump this when the AppEntry schema changes to auto-clear stale caches.
const CACHE_VERSION: u32 = 7;

#[derive(Serialize, Deserialize)]
struct CacheData {
    #[serde(default)]
    version: u32,
    apps: HashMap<String, AppEntry>,
}

/// Load cached app catalog from localStorage.
pub fn load_cache() {
    let storage = match get_storage() {
        Some(s) => s,
        None => return,
    };
    let json = match storage.get_item(STORAGE_KEY) {
        Ok(Some(j)) => j,
        _ => return,
    };
    let data: CacheData = match serde_json::from_str(&json) {
        Ok(d) => d,
        Err(_) => {
            let _ = storage.remove_item(STORAGE_KEY);
            return;
        }
    };
    if data.version != CACHE_VERSION {
        tracing::info!(
            "Cache version mismatch ({} != {}), clearing",
            data.version,
            CACHE_VERSION
        );
        let _ = storage.remove_item(STORAGE_KEY);
        return;
    }
    *APP_CATALOG.write() = data.apps;
    tracing::info!(
        "Loaded {} cached apps from localStorage",
        APP_CATALOG.read().len()
    );
}

/// Save current app catalog to localStorage.
pub fn save_cache() {
    let storage = match get_storage() {
        Some(s) => s,
        None => return,
    };
    let data = CacheData {
        version: CACHE_VERSION,
        apps: APP_CATALOG.read().clone(),
    };
    if let Ok(json) = serde_json::to_string(&data) {
        let _ = storage.set_item(STORAGE_KEY, &json);
    }
}

/// Clear cache and reset in-memory catalog (including contract state).
pub fn clear_cache() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY);
        let _ = storage.remove_item(TOTAL_KEY);
    }
    APP_CATALOG.write().clear();
    // Also clear catalog/shard state so fresh data is fetched from the node
    crate::api::contracts::clear_contract_cache();
    tracing::info!("Cache cleared");
}

const TOTAL_KEY: &str = "freenet_search_total";

/// Save the total contract count to localStorage.
pub fn save_total_contracts(total: usize) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(TOTAL_KEY, &total.to_string());
    }
}

/// Load the cached total contract count.
pub fn load_total_contracts() -> usize {
    get_storage()
        .and_then(|s| s.get_item(TOTAL_KEY).ok().flatten())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
