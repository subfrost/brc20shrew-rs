# BRC20 Shrew - Metashrew Indexer

A WASM-based inscriptions indexer for the metashrew environment, implementing comprehensive Bitcoin inscriptions indexing with support for BRC20, and a programmable BRC20 EVM module.

## Overview

This project implements an inscriptions indexer for the `metashrew` environment. It processes Bitcoin blocks one at a time, extracts inscription data from transaction witnesses, and provides a comprehensive set of view functions for querying inscription data.

It includes a full implementation of the BRC20 standard, and a programmable BRC20 module that embeds a `revm`-based EVM to allow for smart contract execution on top of BRC20 tokens.

## Features

-   **Complete Inscription Support**: Handles all inscription envelope formats including content, metadata, parent-child relationships, delegation, and pointers.
-   **Cursed Inscription Logic**: Implements proper cursed vs blessed inscription numbering with jubilee height support.
-   **BRC20 Indexing**: Full support for BRC20 deploy, mint, and transfer operations.
-   **Programmable BRC20 (EVM)**: An embedded EVM for smart contract functionality on BRC20 tokens, including `deploy` and `call` operations.
-   **Native Precompiles**: Efficient, native Rust precompiles for EVM contracts to interact with the BRC20 state.
-   **Comprehensive Database Schema**: Stores inscriptions, BRC20 state, and EVM state with full indexing for efficient queries.
-   **WASM Integration**: Built for the metashrew WASM environment with proper host-guest interface.
-   **Protobuf API**: Complete protobuf schema for all view functions with request/response patterns.
-   **Extensive Testing**: Unit tests, integration tests, and test utilities for comprehensive coverage.

## Architecture

### Core Components

-   **`src/lib.rs`**: Main WASM entry point with `_start()` function and all view function exports.
-   **`src/indexer.rs`**: Core indexing logic for processing blocks and transactions.
-   **`src/envelope.rs`**: Inscription envelope parsing from Bitcoin script witnesses.
-   **`src/inscription.rs`**: Core data structures for inscriptions, sat points, and charms.
-   **`src/brc20.rs`**: Defines data structures and logic for BRC20 operations (deploy, mint, transfer).
-   **`src/programmable_brc20.rs`**: Implements the programmable BRC20 module, including the EVM, database, and execution logic.
-   **`src/tables.rs`**: Database schema using IndexPointer for hierarchical storage.
-   **`src/view.rs`**: Implementation of all view functions for querying inscription data.
-   **`src/message.rs`**: Message context for handling protobuf requests and responses.

### BRC20 Module

The BRC20 module indexes `brc-20` metaprotocol inscriptions. It maintains the state of all tickers, user balances (total and available), and validates all operations according to BRC20 rules.

### Programmable BRC20 Module

This module introduces smart contract capabilities for BRC20 tokens.
-   **EVM Integration**: It uses `revm` to execute EVM bytecode.
-   **Operations**: It handles `brc20-prog` inscriptions for deploying (`deploy`) and interacting with (`call`) smart contracts.
-   **State Management**: EVM state (accounts, storage, bytecode) is stored directly in the `metashrew` key-value store for high performance.
-   **Native Precompiles**: Smart contracts can call native Rust functions to query BRC20 balances and other on-chain data, bridging the gap between the EVM and the BRC20 state.

## Building

### Prerequisites

-   Rust with `wasm32-unknown-unknown` target
-   Protocol Buffers compiler (`protoc`)
-   Required Rust crates (see `Cargo.toml`)

### Build Commands

```bash
# Build the WASM module
cargo build --release --target wasm32-unknown-unknown
```

### Testing

```bash
# Run all tests
cargo test
```

## Project Structure

```
brc20shrew-rs/
├── src/
│   ├── lib.rs              # Main WASM entry point
│   ├── indexer.rs          # Core indexing logic
│   ├── envelope.rs         # Envelope parsing
│   ├── inscription.rs      # Inscription data structures
│   ├── brc20.rs            # BRC20 indexing logic
│   ├── programmable_brc20.rs # Programmable BRC20 EVM module
│   ├── tables.rs           # Database schema
│   ├── view.rs             # View functions
│   └── message.rs          # Message handling
├── proto/
│   └── shrewscriptions.proto # Protobuf schema
├── tests/
│   ├── integration_tests.rs # Integration tests
│   └── ...                 # Other tests
└── Cargo.toml              # Project configuration
```

## Reference Implementation

This implementation is based on analysis of:

-   **alkanes-rs**: For metashrew environment patterns and WASM structure.
-   **metashrew-core**: For storage abstractions and host-guest interface.
-   **ord**: For inscription indexing logic and database schema.
-   **brc20-programmable-module**: For the programmable BRC20 concepts and EVM integration patterns.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

1.  Fork the repository
2.  Create a feature branch
3.  Add tests for new functionality
4.  Ensure all tests pass
5.  Submit a pull request

## Acknowledgments

-   The `ord` project for inscription indexing patterns.
-   The `metashrew` project for the WASM indexing framework.
-   The `revm` project for the lightweight and performant EVM implementation.