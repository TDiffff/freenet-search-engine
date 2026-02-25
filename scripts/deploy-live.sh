#!/bin/bash
set -euo pipefail

# Deploy Freenet Search Engine to the LIVE Freenet network.
#
# Incremental deployment: tracks SHA256 hashes of build artifacts and queries
# the node to skip contracts that haven't changed and are already stored.
#
# Prerequisites:
#   - freenet node running in network mode: systemctl --user start freenet
#   - fdev, dx, cargo, tar, xz, curl in PATH
#   - web-container-tool must be built (for webapp signing)
#
# Usage:
#   ./scripts/deploy-live.sh [--force] [--dry-run]

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB_CONTAINER_TOOL="$PROJECT_ROOT/target/release/web-container-tool"
WEB_CONTAINER_WASM="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/web_container_contract.wasm"
DEPLOY_DIR="$PROJECT_ROOT/target/deploy"
WEBAPP_DIR="$PROJECT_ROOT/target/webapp"
DIOXUS_TOML="$PROJECT_ROOT/Dioxus.toml"
MANIFEST="$DEPLOY_DIR/.manifest"
NODE_PORT=7509
PUBLISH_TIMEOUT=5

FORCE=false
DRY_RUN=false
for arg in "$@"; do
    case "$arg" in
        --force)   FORCE=true ;;
        --dry-run) DRY_RUN=true ;;
        *) echo "Unknown flag: $arg"; echo "Usage: $0 [--force] [--dry-run]"; exit 1 ;;
    esac
done

cd "$PROJECT_ROOT"

echo "=== Freenet Search Engine — LIVE Network Deployment ==="
echo "  Node: ws://127.0.0.1:$NODE_PORT"
$FORCE && echo "  Mode: FORCE (skip change detection)" || true
$DRY_RUN && echo "  Mode: DRY RUN (no publishing)" || true
echo ""

# --- Helpers ---

sha256_file() { sha256sum "$1" | cut -d' ' -f1; }

manifest_get() {
    local key="$1"
    if [ -f "$MANIFEST" ]; then
        grep "^${key}=" "$MANIFEST" 2>/dev/null | cut -d'=' -f2 || true
    fi
}

# --- Phase 1: Prerequisites ---
echo "[1/7] Checking prerequisites..."

for cmd in fdev dx cargo tar xz curl; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "ERROR: $cmd not found in PATH"; exit 1
    fi
done

for f in "$WEB_CONTAINER_WASM" "$WEB_CONTAINER_TOOL"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: not found: $f"; exit 1
    fi
done

node_status=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$NODE_PORT/" 2>/dev/null || echo "000")
if [ "$node_status" = "000" ]; then
    echo "ERROR: Freenet node not responding on port $NODE_PORT"
    echo "Start it: systemctl --user start freenet"
    exit 1
fi
echo "  OK. Node responding (HTTP $node_status)."

# Check network connectivity — need at least 1 peer to publish
echo "  Checking network peers..."
PEER_OUTPUT=$(fdev network query 2>&1)
PEER_COUNT=$(echo "$PEER_OUTPUT" | grep -c '|.*|.*:.*|' || true)
if [ "$PEER_COUNT" -lt 1 ]; then
    echo "ERROR: No peers connected — cannot publish to network."
    echo "  Wait for the node to discover peers, or check your network."
    echo "  Peer query output:"
    echo "$PEER_OUTPUT"
    exit 1
fi
echo "  OK. $PEER_COUNT peer(s) connected."

# --- Phase 2: Build contract WASMs ---
echo ""
echo "[2/7] Building contract WASMs..."
cargo build --release -p contract-catalog --target wasm32-unknown-unknown
cargo build --release -p contract-fulltext-shard --target wasm32-unknown-unknown

CATALOG_CODE="target/wasm32-unknown-unknown/release/contract_catalog.wasm"
SHARD_CODE="target/wasm32-unknown-unknown/release/contract_fulltext_shard.wasm"
echo "  catalog: $(du -h "$CATALOG_CODE" | cut -f1)  shard: $(du -h "$SHARD_CODE" | cut -f1)"

# --- Phase 3: Generate CBOR artifacts ---
echo ""
echo "[3/7] Generating CBOR state/parameter files..."

# Preserve webapp signing keys + vanity nonce + manifest across deploy dir clean
WEBAPP_KEYS="$DEPLOY_DIR/webapp-keys.toml"
VANITY_NONCE="$DEPLOY_DIR/webapp-vanity-nonce.bin"
SAVED_KEYS="" ; SAVED_MANIFEST="" ; SAVED_NONCE=""
if [ -f "$WEBAPP_KEYS" ];  then SAVED_KEYS=$(mktemp); cp "$WEBAPP_KEYS" "$SAVED_KEYS"; fi
if [ -f "$MANIFEST" ];     then SAVED_MANIFEST=$(mktemp); cp "$MANIFEST" "$SAVED_MANIFEST"; fi
if [ -f "$VANITY_NONCE" ]; then SAVED_NONCE=$(mktemp); cp "$VANITY_NONCE" "$SAVED_NONCE"; fi
rm -rf "$DEPLOY_DIR" && mkdir -p "$DEPLOY_DIR"
if [ -n "$SAVED_KEYS" ];     then mv "$SAVED_KEYS" "$WEBAPP_KEYS"; fi
if [ -n "$SAVED_MANIFEST" ]; then mv "$SAVED_MANIFEST" "$MANIFEST"; fi
if [ -n "$SAVED_NONCE" ];    then mv "$SAVED_NONCE" "$VANITY_NONCE"; fi

cargo run -p deploy-helper -- "$DEPLOY_DIR"

# --- Phase 4: Compute contract IDs, detect changes, query node ---
echo ""
echo "[4/7] Computing contract IDs and checking node state..."

CATALOG_ID=$(fdev get-contract-id --code "$CATALOG_CODE" --parameters "$DEPLOY_DIR/catalog-params.cbor")
echo "  Catalog: $CATALOG_ID"

SHARD_IDS=()
for i in $(seq 0 15); do
    SHARD_IDS+=($(fdev get-contract-id --code "$SHARD_CODE" --parameters "$DEPLOY_DIR/shard-${i}-params.cbor"))
done
echo "  Shards:  16 IDs computed"

# Webapp signing keys + parameters
mkdir -p "$WEBAPP_DIR"
if [ ! -f "$WEBAPP_KEYS" ]; then
    "$WEB_CONTAINER_TOOL" generate --output "$WEBAPP_KEYS"
    echo "  Generated new webapp signing keys"
fi
if [ ! -f "$WEBAPP_DIR/webapp.parameters" ]; then
    echo "bootstrap" > /tmp/webapp-bootstrap.tar.xz
    "$WEB_CONTAINER_TOOL" sign \
        --input /tmp/webapp-bootstrap.tar.xz \
        --output /tmp/webapp-bootstrap.metadata \
        --parameters "$WEBAPP_DIR/webapp.parameters" \
        --key-file "$WEBAPP_KEYS" \
        --version 1
    rm -f /tmp/webapp-bootstrap.tar.xz /tmp/webapp-bootstrap.metadata
fi

if [ -f "$VANITY_NONCE" ]; then
    # Compute ID with nonce-extended parameters
    VANITY_PARAMS=$(mktemp)
    head -c 32 "$WEBAPP_DIR/webapp.parameters" > "$VANITY_PARAMS"
    cat "$VANITY_NONCE" >> "$VANITY_PARAMS"
    WEBAPP_ID=$(fdev get-contract-id --code "$WEB_CONTAINER_WASM" --parameters "$VANITY_PARAMS")
    rm "$VANITY_PARAMS"
    echo "  Webapp:  $WEBAPP_ID (vanity)"
else
    WEBAPP_ID=$(fdev get-contract-id --code "$WEB_CONTAINER_WASM" --parameters "$WEBAPP_DIR/webapp.parameters")
    echo "  Webapp:  $WEBAPP_ID"
fi

# Current artifact hashes
cur_catalog_wasm_sha=$(sha256_file "$CATALOG_CODE")
cur_catalog_params_sha=$(sha256_file "$DEPLOY_DIR/catalog-params.cbor")
cur_shard_wasm_sha=$(sha256_file "$SHARD_CODE")
cur_webapp_wasm_sha=$(sha256_file "$WEB_CONTAINER_WASM")
cur_webapp_params_sha=$(sha256_file "$WEBAPP_DIR/webapp.parameters")
declare -A cur_shard_params_sha
for i in $(seq 0 15); do
    cur_shard_params_sha[$i]=$(sha256_file "$DEPLOY_DIR/shard-${i}-params.cbor")
done

# Detect changes vs manifest
catalog_changed=false
shard_wasm_changed=false
webapp_changed=false
declare -A shard_changed

if $FORCE; then
    catalog_changed=true; shard_wasm_changed=true; webapp_changed=true
    for i in $(seq 0 15); do shard_changed[$i]=true; done
else
    [ "$cur_catalog_wasm_sha" != "$(manifest_get catalog_wasm_sha256)" ] || \
    [ "$cur_catalog_params_sha" != "$(manifest_get catalog_params_sha256)" ] && catalog_changed=true || true

    [ "$cur_shard_wasm_sha" != "$(manifest_get shard_wasm_sha256)" ] && shard_wasm_changed=true || true

    for i in $(seq 0 15); do
        if $shard_wasm_changed || [ "${cur_shard_params_sha[$i]}" != "$(manifest_get "shard_${i}_params_sha256")" ]; then
            shard_changed[$i]=true
        else
            shard_changed[$i]=false
        fi
    done

    [ "$cur_webapp_wasm_sha" != "$(manifest_get webapp_wasm_sha256)" ] || \
    [ "$cur_webapp_params_sha" != "$(manifest_get webapp_params_sha256)" ] && webapp_changed=true || true
fi

# Batch query: which contracts are already on the node?
ALL_IDS=("$CATALOG_ID")
for sid in "${SHARD_IDS[@]}"; do ALL_IDS+=("$sid"); done
ALL_IDS+=("$WEBAPP_ID")

DIAG_ARGS=""
for id in "${ALL_IDS[@]}"; do DIAG_ARGS+=" --contract $id"; done
DIAG_OUTPUT=$(fdev diagnostics $DIAG_ARGS 2>&1) || true

declare -A ON_NODE
for id in "${ALL_IDS[@]}"; do
    if echo "$DIAG_OUTPUT" | grep -q "$id"; then ON_NODE[$id]=true; else ON_NODE[$id]=false; fi
done

# Build publish list
TO_PUBLISH=()
SKIP_COUNT=0

if $catalog_changed; then
    TO_PUBLISH+=("catalog:0"); echo "  Catalog: CHANGED → publish"
elif [ "${ON_NODE[$CATALOG_ID]}" = "true" ]; then
    SKIP_COUNT=$((SKIP_COUNT + 1)); echo "  Catalog: unchanged + on node → skip"
else
    TO_PUBLISH+=("catalog:0"); echo "  Catalog: MISSING from node → publish"
fi

shards_publish=0; shards_skip=0
for i in $(seq 0 15); do
    if [ "${shard_changed[$i]}" = "true" ]; then
        TO_PUBLISH+=("shard:$i"); shards_publish=$((shards_publish + 1))
    elif [ "${ON_NODE[${SHARD_IDS[$i]}]}" = "true" ]; then
        SKIP_COUNT=$((SKIP_COUNT + 1)); shards_skip=$((shards_skip + 1))
    else
        TO_PUBLISH+=("shard:$i"); shards_publish=$((shards_publish + 1))
    fi
done
echo "  Shards:  $shards_publish to publish, $shards_skip skipped"

WEBAPP_NEEDS_PUBLISH=false
if $webapp_changed || $FORCE; then
    WEBAPP_NEEDS_PUBLISH=true; echo "  Webapp:  CHANGED → rebuild + publish"
elif [ "${ON_NODE[$WEBAPP_ID]}" != "true" ]; then
    WEBAPP_NEEDS_PUBLISH=true; echo "  Webapp:  MISSING from node → rebuild + publish"
else
    echo "  Webapp:  unchanged + on node → skip"
fi

# --- Phase 5: Build + package webapp (only if needed) ---
echo ""
if $WEBAPP_NEEDS_PUBLISH; then
    echo "[5/7] Building UI + packaging webapp..."

    DIOXUS_BACKUP=$(mktemp)
    cp "$DIOXUS_TOML" "$DIOXUS_BACKUP"
    sed -i '/^base_path/d' "$DIOXUS_TOML"
    sed -i "/\[web\.app\]/a base_path = \"v1/contract/web/$WEBAPP_ID\"" "$DIOXUS_TOML"

    (cd ui && dx build --release 2>&1)
    mv "$DIOXUS_BACKUP" "$DIOXUS_TOML"
    echo "  UI built."

    # Find dx output
    DX_OUTPUT=""
    for candidate in \
        "target/dx/freenet-search-engine/release/web/public" \
        "target/dx/freenet_search_engine/release/web/public" \
        "ui/target/dx/freenet-search-engine/release/web/public" \
        "ui/target/dx/freenet_search_engine/release/web/public"; do
        if [ -d "$candidate" ]; then DX_OUTPUT="$candidate"; break; fi
    done
    if [ -z "$DX_OUTPUT" ]; then echo "ERROR: dx build output not found"; exit 1; fi

    # Package + sign
    (cd "$DX_OUTPUT" && tar -cJf "$WEBAPP_DIR/webapp.tar.xz" *)
    version=$(( $(date +%s) / 60 ))
    "$WEB_CONTAINER_TOOL" sign \
        --input "$WEBAPP_DIR/webapp.tar.xz" \
        --output "$WEBAPP_DIR/webapp.metadata" \
        --parameters "$WEBAPP_DIR/webapp.parameters" \
        --key-file "$WEBAPP_KEYS" \
        --version "$version"
    echo "  Signed (version=$version)."

    # Append vanity nonce if present (sign regenerates parameters to 32 bytes)
    VANITY_NONCE="$DEPLOY_DIR/webapp-vanity-nonce.bin"
    if [ -f "$VANITY_NONCE" ]; then
        cat "$VANITY_NONCE" >> "$WEBAPP_DIR/webapp.parameters"
        echo "  Appended vanity nonce ($(wc -c < "$WEBAPP_DIR/webapp.parameters") byte params)."
    fi

    cur_dx_output_sha=$(find "$DX_OUTPUT" -type f -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)
    TO_PUBLISH+=("webapp:0")
else
    echo "[5/7] Skipping UI build + webapp packaging (unchanged + on node)."
    cur_dx_output_sha=$(manifest_get "dx_output_sha256")
fi

# --- Phase 6: Publish ---
echo ""
publish_count=${#TO_PUBLISH[@]}
total_contracts=18
echo "[6/7] Publishing $publish_count/$total_contracts contracts ($SKIP_COUNT skipped)..."

if $DRY_RUN; then
    if [ "$publish_count" -gt 0 ]; then
        echo "  DRY RUN — would publish:"
        for entry in "${TO_PUBLISH[@]}"; do
            type="${entry%%:*}"; idx="${entry##*:}"
            case "$type" in
                catalog) echo "    Catalog ($CATALOG_ID)" ;;
                shard)   echo "    Shard $idx (${SHARD_IDS[$idx]})" ;;
                webapp)  echo "    Webapp ($WEBAPP_ID)" ;;
            esac
        done
    fi
    echo "  DRY RUN complete. Nothing published."
elif [ "$publish_count" -eq 0 ]; then
    echo "  Nothing to publish — all contracts up to date."
else
    PUBLISH_PIDS=()
    for entry in "${TO_PUBLISH[@]}"; do
        type="${entry%%:*}"; idx="${entry##*:}"
        case "$type" in
            catalog)
                echo "  Publishing catalog..."
                fdev network publish \
                    --code "$CATALOG_CODE" --parameters "$DEPLOY_DIR/catalog-params.cbor" \
                    contract --state "$DEPLOY_DIR/catalog-state.cbor" &
                PUBLISH_PIDS+=($!) ;;
            shard)
                echo "  Publishing shard $idx..."
                fdev network publish \
                    --code "$SHARD_CODE" --parameters "$DEPLOY_DIR/shard-${idx}-params.cbor" \
                    contract --state "$DEPLOY_DIR/shard-${idx}-state.cbor" &
                PUBLISH_PIDS+=($!) ;;
            webapp)
                echo "  Publishing webapp..."
                fdev network publish \
                    --code "$WEB_CONTAINER_WASM" --parameters "$WEBAPP_DIR/webapp.parameters" \
                    contract --webapp-archive "$WEBAPP_DIR/webapp.tar.xz" \
                    --webapp-metadata "$WEBAPP_DIR/webapp.metadata" &
                PUBLISH_PIDS+=($!) ;;
        esac
    done

    echo "  ${#PUBLISH_PIDS[@]} jobs launched. Waiting up to ${PUBLISH_TIMEOUT}s..."
    deadline=$(( $(date +%s) + PUBLISH_TIMEOUT ))
    while [ $(date +%s) -lt $deadline ]; do
        all_done=true
        for pid in "${PUBLISH_PIDS[@]}"; do
            kill -0 "$pid" 2>/dev/null && { all_done=false; break; }
        done
        if $all_done; then break; fi
        sleep 1
    done
    for pid in "${PUBLISH_PIDS[@]}"; do kill "$pid" 2>/dev/null || true; done
    wait 2>/dev/null || true
    echo "  Publish complete."
fi

# --- Phase 7: Verify + update manifest ---
echo ""
echo "[7/7] Verifying..."

DIAG_OUTPUT=$(fdev diagnostics $DIAG_ARGS 2>&1) || true
found=0; missing=()
for id in "${ALL_IDS[@]}"; do
    if echo "$DIAG_OUTPUT" | grep -q "$id"; then
        found=$((found + 1))
    else
        missing+=("$id")
    fi
done

echo "  $found/${#ALL_IDS[@]} contracts found on node."
if [ ${#missing[@]} -gt 0 ]; then
    echo "  MISSING:"; for m in "${missing[@]}"; do echo "    $m"; done
fi

if ! $DRY_RUN; then
    cat > "$MANIFEST" <<MANIFEST_EOF
catalog_wasm_sha256=$cur_catalog_wasm_sha
catalog_params_sha256=$cur_catalog_params_sha
shard_wasm_sha256=$cur_shard_wasm_sha
$(for i in $(seq 0 15); do echo "shard_${i}_params_sha256=${cur_shard_params_sha[$i]}"; done)
webapp_wasm_sha256=$cur_webapp_wasm_sha
webapp_params_sha256=$cur_webapp_params_sha
dx_output_sha256=${cur_dx_output_sha:-}
last_deploy_timestamp=$(date +%s)
MANIFEST_EOF
    echo "  Manifest updated."
fi

echo ""
echo "=========================================="
$DRY_RUN && echo "Dry run complete!" || echo "Deployment complete!"
echo ""
echo "Search engine: http://127.0.0.1:$NODE_PORT/v1/contract/web/$WEBAPP_ID/"
echo ""
echo "Contract IDs:"
echo "  Catalog: $CATALOG_ID"
for i in $(seq 0 15); do echo "  Shard $i: ${SHARD_IDS[$i]}"; done
echo "  Webapp:  $WEBAPP_ID"
echo "=========================================="
