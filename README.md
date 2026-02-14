# Freenet Search Engine

A decentralized search engine for discovering web applications on the [Freenet](https://freenet.org) network. The search index is a shared Freenet contract contributed to by all participants — no central server, no crawlers.

<img width="1202" height="663" alt="image" src="https://github.com/user-attachments/assets/4b3b8d84-663d-45f5-85a0-54f63b31648a" />

## Try it

If you have a freenet node running, browse [This web app contract](http://127.0.0.1:7509/v1/contract/web/FinderTZGHAY8DWSeQPRMN9K3p884WsLTUCjjH4QSZqH/)
## Architecture

The project is a self-contained Rust workspace:

| Crate | Description |
|-------|-------------|
| `search-common` | Shared types, CBOR serialization, tokenization, scoring, bloom filters, web container extraction |
| `contract-catalog` | Freenet contract storing the app catalog (URL metadata, contributor reputation, anti-Sybil) |
| `contract-fulltext-shard` | Freenet contract storing inverted index shards for full-text search |
| `web-container-contract` | Freenet contract serving the compiled UI as a signed web container |
| `delegate-identity` | Freenet delegate managing ed25519 keypairs for contributor identity |
| `ui` | Dioxus 0.7 WASM app — browsing, search, and contribution UI |
| `deploy-helper` | CLI tool generating CBOR artifacts and contract IDs for deployment |
| `web-container-tool` | CLI tool for Ed25519 key generation and webapp signing |

### How it works

1. **Discovery** — the UI connects to the local Freenet node via WebSocket, polls diagnostics for all contracts, and type-detects web apps by fetching their state
2. **Metadata extraction** — for each web app, the UI decompresses the web container (xz tar), finds `index.html`, and extracts title and description from `<meta>` tags (falls back to visible body text when no meta tags exist)
3. **Catalog contract** stores metadata (title, description, snippet) for every indexed web app, with contributor attestations and reputation scores
4. **Fulltext shard contracts** (16 shards) store an inverted index partitioned by keyword hash, enabling search across all indexed apps
5. **Contribution pipeline** — when enabled, the UI automatically contributes discovered app metadata to the catalog and shard contracts with proof-of-work antiflood tokens
6. **Deduplication** — when multiple contracts share the same title (e.g. different deployments of the same app), the UI picks the best one by catalog attestation count (network-wide signal), then state size, then version

### Key design decisions

- **CBOR serialization** for all cross-node types (no JSON) — self-describing, schema-evolution friendly
- **Integer-only scoring** with x10000 scaling (no floating-point in any contract state) — deterministic across all WASM runtimes
- **CRDT merge** for contract state — grow-only maps, max-wins for scores, dedup by pubkey, deterministic finalization. All `update_state` implementations are commutative
- **Bloom filter sync** for `summarize_state` / `get_state_delta` — compact, efficient for grow-only CRDTs (k=7, SHA-256)
- **Anti-Sybil** — antiflood tokens (proof-of-work) + ed25519 signatures + temporal staking (triple cost per attack)
- **Deterministic extraction** — single pipeline in `search-common` so all contributors produce identical metadata hashes
- **Attestation-based ranking** — deduplication uses catalog attestation count (network-wide) rather than subscriber count (local peers only)

## Development

Prerequisites: Rust with `wasm32-unknown-unknown` target, [Dioxus CLI](https://dioxuslabs.com) (`dx`), a running [Freenet](https://github.com/freenet/freenet-core) node.

```bash
# Run all tests (149 tests across 21 test files)
cargo test --workspace

# Clippy (zero warnings policy)
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all --check

# WASM build check
cargo check -p freenet-search-engine --target wasm32-unknown-unknown

# Dev server (connects to Freenet node at ws://127.0.0.1:7509)
cd ui && dx serve

# Release build
cd ui && dx build --release
```

Smoke tests are available in `tests/`:
- `tests/check-wasm-build.sh` — verifies WASM targets compile
- `tests/check-clippy.sh` — runs clippy with `-D warnings`

## Deployment

### Live deployment

```bash
scripts/deploy-live.sh              # Incremental deploy (skips unchanged contracts)
scripts/deploy-live.sh --force      # Rebuild and republish everything
scripts/deploy-live.sh --dry-run    # Show what would be deployed without publishing
```

The deploy script handles the full pipeline:
- Builds contract WASMs and generates CBOR artifacts via `deploy-helper`
- Computes contract IDs and compares against a SHA256 manifest (`target/deploy/.manifest`)
- Queries the node to check which contracts are already stored
- Only publishes contracts that changed or are missing from the node
- Builds the Dioxus UI with the correct `base_path` for the webapp contract
- Packages, signs, and publishes the webapp with an auto-incrementing version

On a second run with no changes, all 18 contracts are skipped.

### Local test network

```bash
scripts/network-start.sh        # Start a local Freenet network
scripts/deploy-network.sh       # Deploy contracts and web app
scripts/redeploy-webapp.sh      # Redeploy just the web app after UI changes
scripts/network-stop.sh         # Stop the network
```

### Adding app descriptions

Web apps on the network get their descriptions extracted automatically. To ensure your app has a good description, add meta tags to your `index.html`:

```html
<meta name="description" content="Your app description here.">
```

If no meta description tag is present, the search engine falls back to extracting visible text from the HTML body.

## Design Document

Full specification: [docs/plans/2026-02-11-contract-based-architecture-design.md](docs/plans/2026-02-11-contract-based-architecture-design.md)

## License

LGPL-2.1 — see [LICENSE](LICENSE)
