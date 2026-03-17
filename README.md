# brc20shrew-rs

A comprehensive Bitcoin metaprotocol indexing stack built for the [Metashrew](https://github.com/sandshrewmetaprotocols/metashrew) WASM runtime. Indexes inscriptions, BRC-20 tokens, programmable BRC-20 (with an embedded EVM), Runes, and more — compiled to a single `.wasm` module that processes blocks deterministically.

## What This Replaces

This project consolidates the functionality of several canonical Bitcoin metaprotocol tools into one unified indexer:

| Layer | Canonical Tool | brc20shrew-rs Equivalent | Crate |
|---|---|---|---|
| Ordinals / Inscriptions | [ord](https://github.com/ordinals/ord) | Inscription envelope parsing, sat tracking, cursed/blessed numbering, jubilee logic, parent-child, delegation, pointers | `shrew-ord` |
| BRC-20 Tokens | [OPI](https://github.com/bestinslot-xyz/OPI) (`brc20_index`) | Deploy, mint, transfer with full OPI-conformant validation (u128 amounts, case normalization, ticker rules, partial mint) | `shrew-brc20` |
| BRC-20 Self-Mint | OPI self-mint module | 5-byte tickers at block 837,090, parent inscription validation, max_supply/lim=0 defaults to MAX_AMOUNT | `shrew-brc20` |
| 6-Byte Predeploy | OPI extended tickers | 6-byte tickers at block 912,690, alphanumeric/dash validation | `shrew-brc20` |
| Programmable BRC-20 | [brc20-prog](https://github.com/bestinslot-xyz/brc20-programmable-module) | EVM smart contracts on BRC-20 via `deploy`/`call` inscriptions, controller contract, view calls | `shrew-brc20-prog` |
| EVM Execution | brc20-prog EVM module | `revm`-based EVM with Bitcoin-native precompiles (BIP-322, tx details, sat location, locked pkscript, OP_RETURN txid) | `shrew-evm` |
| Runes | [ord](https://github.com/ordinals/ord) (runes module) | Rune etching, minting, transfers, balance tracking | `shrew-runes` |
| Bitmap | Various bitmap indexers | Bitmap NFT collection indexing | `shrew-bitmap` |
| SNS | Sats Names indexers | Domain name registration and resolution | `shrew-sns` |
| PoW20 | PoW20 reference | Proof-of-work token mining and balance tracking | `shrew-pow20` |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   rockshrew-mono                        │
│              (Metashrew WASM Runtime)                   │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │            shrew_brc20_prog.wasm                  │  │
│  │                                                   │  │
│  │  ┌─────────┐  ┌───────────┐  ┌────────────────┐  │  │
│  │  │shrew-ord│→ │ shrew-brc20│→ │shrew-brc20-prog│  │  │
│  │  │         │  │           │  │                │  │  │
│  │  │inscript-│  │ deploy    │  │ EVM deploy/   │  │  │
│  │  │ions,    │  │ mint      │  │ call, view    │  │  │
│  │  │satpoints│  │ transfer  │  │ controller    │  │  │
│  │  │envelopes│  │ balances  │  │               │  │  │
│  │  └─────────┘  └───────────┘  └───────┬────────┘  │  │
│  │                                      │            │  │
│  │                              ┌───────▼────────┐   │  │
│  │                              │   shrew-evm    │   │  │
│  │                              │                │   │  │
│  │                              │ revm engine    │   │  │
│  │                              │ MetashrewDB    │   │  │
│  │                              │ precompiles:   │   │  │
│  │                              │  0xFA OP_RETURN│   │  │
│  │                              │  0xFB locked   │   │  │
│  │                              │       pkscript │   │  │
│  │                              │  0xFC sat loc  │   │  │
│  │                              │  0xFD tx detail│   │  │
│  │                              │  0xFE BIP-322  │   │  │
│  │                              └────────────────┘   │  │
│  │                                                   │  │
│  │  ┌──────────┐ ┌────────┐ ┌───────┐ ┌──────────┐  │  │
│  │  │shrew-    │ │shrew-  │ │shrew- │ │shrew-    │  │  │
│  │  │runes     │ │bitmap  │ │sns    │ │pow20     │  │  │
│  │  └──────────┘ └────────┘ └───────┘ └──────────┘  │  │
│  │                                                   │  │
│  │  ┌──────────────────┐  ┌────────────────────────┐ │  │
│  │  │  shrew-support   │  │  shrew-test-helpers    │ │  │
│  │  │  (shared types)  │  │  (test utilities)      │ │  │
│  │  └──────────────────┘  └────────────────────────┘ │  │
│  └───────────────────────────────────────────────────┘  │
│                        │                                │
│                   bitcoind RPC                          │
└─────────────────────────────────────────────────────┘
```

The primary data flow is **bitcoind → rockshrew-mono → WASM indexer**. Each block is processed sequentially: `shrew-ord` extracts inscriptions, then downstream crates (`shrew-brc20`, `shrew-brc20-prog`, `shrew-runes`, etc.) process their respective metaprotocol operations against the indexed inscription data.

## Crates

### `shrew-ord` — Ordinals & Inscriptions

Reimplements the core indexing logic from [ord](https://github.com/ordinals/ord):

- Parses inscription envelopes from witness data (content body, content type, metadata, metaprotocol tags)
- Tracks inscription numbering with cursed/blessed logic and jubilee height (block 824,544)
- Maintains sat points — the specific satoshi each inscription is bound to
- Handles parent-child relationships, delegation, and pointer inscriptions
- Provides protobuf RPC for querying inscriptions by ID, number, address, block, or content

### `shrew-brc20` — BRC-20 Token Standard

Reimplements the BRC-20 indexing logic from [OPI](https://github.com/bestinslot-xyz/OPI) (the canonical open-source BRC-20 indexer):

- **Deploy**: Create new BRC-20 tickers (4-byte standard, 5-byte self-mint, 6-byte predeploy)
- **Mint**: Mint tokens up to the per-mint limit and max supply, with partial mint support for the final mint
- **Transfer**: Two-phase transfer — inscribe a transfer, then send the inscription to the recipient
- **Validation**: Case-insensitive ticker matching, u128 precision (18 decimal places), zero-amount rejection, MAX_AMOUNT cap (2^64 - 1), first-is-first deploy rule
- **Self-mint** (block 837,090+): 5-byte tickers with `"self_mint": "true"`, mint requires parent inscription matching the deploy inscription
- **6-byte predeploy** (block 912,690+): Extended tickers, alphanumeric and dash characters only

### `shrew-brc20-prog` — Programmable BRC-20

Reimplements the [brc20-programmable-module](https://github.com/bestinslot-xyz/brc20-programmable-module) which adds EVM smart contract capabilities to BRC-20:

- **Deploy**: `brc20-prog` inscriptions with `"op": "deploy"` deploy EVM bytecode as a smart contract
- **Call**: `"op": "call"` inscriptions execute contract functions, passing BRC-20 tokens in/out of EVM state
- **View**: Read-only calls to query contract state without an inscription
- **Controller**: The BRC20_Controller Solidity contract (deployed at `0xc54dd458...`) manages mint/burn/balanceOf operations, bridging EVM storage with BRC-20 balances
- **Function selectors**: `mint(bytes,address,uint256)` = `0x1fcfe19c`, `burn(bytes,address,uint256)` = `0xdc9ae17d`, `balanceOf(bytes,address)` = `0xfc124ebd`

### `shrew-evm` — EVM Engine & Precompiles

Provides the EVM execution layer using [revm](https://github.com/bluealloy/revm):

- **MetashrewDB**: Adapter that maps EVM account/storage reads and writes to the Metashrew key-value store
- **Precompiles** — native Rust functions callable from Solidity at special addresses:

| Address | Name | Description |
|---|---|---|
| `0x...FA` | OP_RETURN TXID | Returns the Bitcoin txid of the current OP_RETURN transaction |
| `0x...FB` | Locked Pkscript | Constructs a P2TR Taproot output with CSV timelock (full TaprootBuilder implementation) |
| `0x...FC` | Last Sat Location | Returns the last known location of a specific satoshi |
| `0x...FD` | BTC TX Details | Fetches Bitcoin transaction details by txid and vout |
| `0x...FE` | BIP-322 Verify | Verifies BIP-322 signed messages |

### `shrew-runes` — Runes Protocol

Indexes the Runes fungible token protocol (activated at block 840,000):

- Rune etching (creation), minting, and transfer tracking
- Balance accounting per address
- Rune metadata: name, symbol, divisibility, spacers, premine

### `shrew-bitmap`, `shrew-sns`, `shrew-pow20`

Additional metaprotocol indexers for Bitmap NFT collections, Sats Names (SNS) domain registration, and proof-of-work (PoW20) token mining.

### `shrew-support` — Shared Types & Utilities

Foundation crate exporting common types used across all indexers: `InscriptionId`, `SatPoint`, `InscriptionEntry`, `Charm`, `Rarity`, address derivation utilities, and protocol constants (activation heights, MAX_AMOUNT, etc.).

### `shrew-test-helpers` — Test Infrastructure

Provides test utilities for building mock Bitcoin blocks, transactions, inscriptions, and assertion helpers. Used across all crate test suites.

## Building

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- Protocol Buffers compiler (`protoc`)
- `clang` / `libclang-dev` (for native dependencies)

### Build the WASM Indexer

```bash
rustup target add wasm32-unknown-unknown
cargo build --release -p shrew-brc20-prog --target wasm32-unknown-unknown
```

The output WASM binary is at `target/wasm32-unknown-unknown/release/shrew_brc20_prog.wasm`.

### Run Tests

```bash
# Run all workspace tests (301 tests across 10 crates)
cargo test --target x86_64-unknown-linux-gnu -- --test-threads=1

# Run a specific crate's tests
cargo test -p shrew-brc20 --no-default-features --target x86_64-unknown-linux-gnu -- --test-threads=1
```

Tests must run single-threaded (`--test-threads=1`) because they use shared global state that is cleared between tests.

### Docker (with Metashrew runtime)

```bash
docker build -t metashrew-brc20-prog:latest -f docker/metashrew-brc20-prog/Dockerfile .
```

This multi-stage build compiles the WASM indexer and packages it with [rockshrew-mono](https://github.com/sandshrewmetaprotocols/metashrew) (the Metashrew runtime), producing a single container that connects to a bitcoind RPC and indexes from a configurable start block.

## Deployment

The indexer runs as a single container alongside a Bitcoin full node:

```
bitcoind (full node) → rockshrew-mono (runtime) → shrew_brc20_prog.wasm (indexer)
                                                          ↓
                                                    RocksDB (state)
                                                          ↓
                                                   JSON-RPC :8080
```

Environment variables:
- `HOST` / `PORT` — RPC listen address (default `0.0.0.0:8080`)
- `DB_PATH` — RocksDB data directory (default `/data`)
- `INDEXER_PATH` — Path to the WASM module (default `/metashrew/indexer.wasm`)
- `RUST_LOG` — Log level (default `none`)

## How It Compares to Running ord + OPI Separately

| Concern | ord + OPI | brc20shrew-rs |
|---|---|---|
| Runtime | ord (Rust binary) + OPI (Python) + PostgreSQL | Single WASM module in Metashrew runtime + RocksDB |
| Inscription indexing | ord handles all inscription logic | `shrew-ord` reimplements ord's indexing in pure Rust/WASM |
| BRC-20 indexing | OPI reads from ord's database, processes BRC-20 in Python | `shrew-brc20` processes BRC-20 in the same pass as inscriptions |
| Programmable BRC-20 | Separate brc20-prog Python module with its own EVM | `shrew-brc20-prog` + `shrew-evm` (revm) in the same WASM |
| Database | ord uses redb; OPI uses PostgreSQL | All state in Metashrew's RocksDB key-value store |
| Consistency | Two separate processes may drift | Single-pass: inscriptions → BRC-20 → EVM all in one block |
| Additional protocols | Separate tools for Runes, Bitmap, SNS | All indexed in the same WASM module |

## Reference Implementations

This project is built to be conformant with:

- **[ord](https://github.com/ordinals/ord)** — Inscription indexing logic, sat arithmetic, cursed numbering
- **[OPI](https://github.com/bestinslot-xyz/OPI)** — BRC-20 protocol rules, validation edge cases, self-mint, extended tickers
- **[brc20-programmable-module](https://github.com/bestinslot-xyz/brc20-programmable-module)** — Programmable BRC-20 EVM architecture, controller contract, precompile interfaces
- **[metashrew](https://github.com/sandshrewmetaprotocols/metashrew)** — WASM runtime, host-guest interface, RocksDB storage model
- **[revm](https://github.com/bluealloy/revm)** — EVM execution engine

## License

MIT
