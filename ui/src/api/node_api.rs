use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use dioxus::prelude::*;
use freenet_stdlib::client_api::{
    ClientRequest, ContractRequest, ContractResponse, HostResponse,
};
use freenet_stdlib::prelude::ContractInstanceId;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use crate::state::{
    ContractType, ContributionStatus, DiscoveryPhase, APP_CATALOG, CONTRIBUTION_HISTORY,
    CONTRACT_TYPES, DISCOVERY_PHASE, NODE_CONNECTED, TYPES_CHECKED, TYPE_CHECK_QUEUE,
};

use super::types::NodeConfig;

/// Prevent duplicate polling intervals across reconnections.
static POLLING_STARTED: AtomicBool = AtomicBool::new(false);

/// Interval between catalog/shard re-fetch (milliseconds).
/// Compensates for subscription timeouts — ensures state stays fresh.
const INDEX_REFETCH_INTERVAL_MS: i32 = 30_000;

/// Interval between type-check GET request batches (milliseconds).
const TYPE_CHECK_INTERVAL_MS: i32 = 300;

/// Maximum number of GET requests to send per type-check tick.
const TYPE_CHECK_BATCH_SIZE: usize = 10;

// Shared WebSocket handle — replaced on each reconnection so polling closures
// always use the current connection.
thread_local! {
    static CURRENT_WS: RefCell<Option<Rc<RefCell<WebSocket>>>> = const { RefCell::new(None) };
}

fn set_current_ws(ws: Rc<RefCell<WebSocket>>) {
    CURRENT_WS.with(|cell| *cell.borrow_mut() = Some(ws));
}

pub fn with_current_ws<F: FnOnce(&WebSocket)>(f: F) {
    CURRENT_WS.with(|cell| {
        if let Some(ws) = cell.borrow().as_ref() {
            let ws = ws.borrow();
            if ws.ready_state() == WebSocket::OPEN {
                f(&ws);
            }
        }
    });
}

/// Connects to the Freenet node client API and polls diagnostics periodically.
pub fn connect_node_api(config: &NodeConfig) {
    let url = config.api_url.clone();

    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(e) => {
            tracing::error!("Failed to create WebSocket: {:?}", e);
            return;
        }
    };

    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let ws_rc = Rc::new(RefCell::new(ws.clone()));

    let ws_for_open = ws_rc.clone();
    let onopen = Closure::<dyn FnMut()>::new(move || {
        tracing::info!("Node API WebSocket connected");
        *NODE_CONNECTED.write() = true;

        // Update shared handle so existing intervals use the new connection
        set_current_ws(ws_for_open.clone());

        // Subscribe to search index contracts (catalog + 16 shards)
        super::contracts::subscribe_catalog(&ws_for_open.borrow());
        for shard_id in 0u8..16 {
            super::contracts::subscribe_shard(&ws_for_open.borrow(), shard_id);
        }

        // Only start intervals once (they persist across reconnects)
        if !POLLING_STARTED.swap(true, Ordering::SeqCst) {
            start_polling_intervals();
        }
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        let data = e.data();
        if let Ok(abuf) = data.dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&abuf).to_vec();
            handle_host_response(&bytes);
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onerror = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        tracing::error!("Node API WebSocket error");
        *NODE_CONNECTED.write() = false;
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    let url_for_reconnect = config.api_url.clone();
    let onclose = Closure::<dyn FnMut()>::new(move || {
        tracing::warn!("Node API WebSocket closed, will reconnect in 5s");
        *NODE_CONNECTED.write() = false;
        schedule_reconnect(url_for_reconnect.clone());
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
}

pub fn send_request(ws: &WebSocket, request: &ClientRequest) {
    match bincode::serialize(request) {
        Ok(bytes) => {
            if let Err(e) = ws.send_with_u8_array(&bytes) {
                tracing::error!("Failed to send request: {:?}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to serialize request: {}", e);
        }
    }
}

/// Parse a bincode-encoded HostResponse and update global signals.
fn handle_host_response(bytes: &[u8]) {
    use freenet_stdlib::client_api::ClientError;

    let result: Result<HostResponse, ClientError> = match bincode::deserialize(bytes) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to deserialize HostResponse: {}", e);
            return;
        }
    };

    let response = match result {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Node returned error: {:?}", e);
            return;
        }
    };

    match response {
        HostResponse::ContractResponse(ContractResponse::GetResponse {
            key,
            state,
            contract,
        }) => {
            // Route search index contract responses to their handlers
            if super::contracts::is_catalog_key(&key) {
                super::contracts::handle_catalog_response(state.as_ref());
                return;
            }
            if super::contracts::matching_shard_id(&key).is_some() {
                super::contracts::handle_shard_response(state.as_ref());
                return;
            }

            let key_str = format!("{}", key);
            let contract_type =
                crate::discovery::detector::detect_contract_type(state.as_ref());

            // If it's a WebApp, extract metadata and update catalog
            if contract_type == ContractType::WebApp {
                let size = state.as_ref().len() as u64;
                let version = crate::discovery::title::extract_version_from_state(state.as_ref());

                // Extract publisher key (first 32 bytes of contract params = Ed25519 pubkey)
                let publisher_key: Option<Vec<u8>> = contract.as_ref().map(|c| {
                    let params = c.params();
                    let bytes = params.as_ref();
                    bytes[..32.min(bytes.len())].to_vec()
                });

                let (has_title, has_description, cached_version) = {
                    let catalog = APP_CATALOG.read();
                    match catalog.get(&key_str) {
                        Some(entry) => (
                            entry.title.is_some(),
                            entry.description.is_some(),
                            entry.version,
                        ),
                        None => (false, false, None),
                    }
                };

                // Re-extract when title or description is missing, or version changed
                let version_changed = match (version, cached_version) {
                    (Some(v), Some(cv)) => v != cv,
                    (Some(_), None) => true,
                    _ => false,
                };
                let needs_extraction = !has_title || !has_description || version_changed;

                if needs_extraction {
                    if !has_title {
                        crate::discovery::http_fallback::try_fetch_title(
                            key_str.clone(),
                            version,
                            Some(size),
                        );
                    }

                    let (title, description) =
                        crate::discovery::title::extract_title_from_state(state.as_ref());

                    crate::discovery::title::update_catalog_entry(
                        &key_str,
                        title.as_deref(),
                        description.as_deref(),
                        Some(size),
                        version,
                        true, // fresh extraction — clear stale cache if blank
                        publisher_key.as_deref(),
                    );
                } else {
                    // Fully cached and unchanged — just update size
                    crate::discovery::title::update_catalog_entry(
                        &key_str,
                        None,
                        None,
                        Some(size),
                        version,
                        false, // no extraction — preserve cached title/desc
                        publisher_key.as_deref(),
                    );
                }
                crate::discovery::cache::save_cache();
            }

            // Trigger contribution pipeline if enabled and this is a WebApp
            if contract_type == ContractType::WebApp {
                super::contribution::contribute_entry(key_str.clone(), state.as_ref().to_vec());
            }

            CONTRACT_TYPES.write().insert(key_str, contract_type);

            *TYPES_CHECKED.write() += 1;

            // Update discovery phase based on queue state
            if TYPE_CHECK_QUEUE.read().is_empty() {
                *DISCOVERY_PHASE.write() = DiscoveryPhase::Complete;
            }
        }
        HostResponse::ContractResponse(ContractResponse::UpdateResponse { key, .. }) => {
            // Catalog/shard update accepted — mark contribution as confirmed
            if super::contracts::is_catalog_key(&key) {
                let mut history = CONTRIBUTION_HISTORY.write();
                if let Some(record) = history
                    .iter_mut()
                    .rev()
                    .find(|r| matches!(r.status, ContributionStatus::Submitted))
                {
                    record.status = ContributionStatus::Confirmed;
                }
            }
        }
        HostResponse::ContractResponse(ContractResponse::UpdateNotification { key, .. }) => {
            // A subscribed contract was updated — re-fetch full state
            if super::contracts::is_catalog_key(&key) {
                tracing::info!("Catalog contract updated, re-fetching...");
                with_current_ws(super::contracts::subscribe_catalog);
            } else if let Some(shard_id) = super::contracts::matching_shard_id(&key) {
                tracing::debug!("Shard {} updated, re-fetching...", shard_id);
                with_current_ws(|ws| super::contracts::subscribe_shard(ws, shard_id));
            }
        }
        HostResponse::Ok => {}
        _ => {
            tracing::debug!("Received unhandled response type");
        }
    }
}

/// Start polling and type-checking intervals (called exactly once).
fn start_polling_intervals() {
    let window = web_sys::window().expect("no global window");

    // Periodic re-fetch of catalog + shard states (compensates for subscription timeouts)
    let refetch_callback = Closure::<dyn FnMut()>::new(move || {
        with_current_ws(|ws| {
            super::contracts::subscribe_catalog(ws);
            for shard_id in 0u8..16 {
                super::contracts::subscribe_shard(ws, shard_id);
            }
        });
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        refetch_callback.as_ref().unchecked_ref(),
        INDEX_REFETCH_INTERVAL_MS,
    );
    refetch_callback.forget();

    // Type-check polling — send up to TYPE_CHECK_BATCH_SIZE requests per tick
    let type_callback = Closure::<dyn FnMut()>::new(move || {
        for _ in 0..TYPE_CHECK_BATCH_SIZE {
            let next = TYPE_CHECK_QUEUE.write().pop_front();
            let Some((key, id_bytes)) = next else { break };
            if CONTRACT_TYPES.read().contains_key(&key) {
                continue;
            }
            if let Ok(id_arr) = <[u8; 32]>::try_from(id_bytes.as_slice()) {
                let request = ClientRequest::ContractOp(ContractRequest::Get {
                    key: ContractInstanceId::new(id_arr),
                    return_contract_code: true,
                    subscribe: false,
                    blocking_subscribe: false,
                });
                with_current_ws(|ws| send_request(ws, &request));
            }
        }
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        type_callback.as_ref().unchecked_ref(),
        TYPE_CHECK_INTERVAL_MS,
    );
    type_callback.forget();
}

fn schedule_reconnect(url: String) {
    let callback = Closure::<dyn FnMut()>::new(move || {
        let config = NodeConfig {
            api_url: url.clone(),
        };
        connect_node_api(&config);
    });

    let window = web_sys::window().expect("no global window");
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        5_000,
    );
    callback.forget();
}
