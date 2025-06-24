# shrewscriptions-rs

A WASM-based inscriptions indexer for the metashrew environment, implementing comprehensive Bitcoin inscriptions indexing with support for all inscription features including parent-child relationships, delegation, cursed inscriptions, and sat tracking.

## Overview

This project implements an inscriptions indexer similar to how `alkanes-rs` indexes alkanes within the metashrew environment. It processes Bitcoin blocks one at a time, extracts inscription data from transaction witnesses, and provides a comprehensive set of view functions for querying inscription data.

## Features

- **Complete Inscription Support**: Handles all inscription envelope formats including content, metadata, parent-child relationships, delegation, and pointers
- **Cursed Inscription Logic**: Implements proper cursed vs blessed inscription numbering with jubilee height support
- **Comprehensive Database Schema**: Stores inscriptions with full indexing for efficient queries
- **WASM Integration**: Built for the metashrew WASM environment with proper host-guest interface
- **Protobuf API**: Complete protobuf schema for all view functions with request/response patterns
- **Extensive Testing**: Unit tests, integration tests, and test utilities for comprehensive coverage

## Architecture

### Core Components

- **`src/lib.rs`**: Main WASM entry point with `_start()` function and all view function exports
- **`src/indexer.rs`**: Core indexing logic for processing blocks and transactions
- **`src/envelope.rs`**: Inscription envelope parsing from Bitcoin script witnesses
- **`src/inscription.rs`**: Core data structures for inscriptions, sat points, and charms
- **`src/tables.rs`**: Database schema using IndexPointer for hierarchical storage
- **`src/view.rs`**: Implementation of all view functions for querying inscription data
- **`src/message.rs`**: Message context for handling protobuf requests and responses

### Database Schema

The indexer uses a comprehensive database schema with the following key tables:

- **Core Mappings**: ID to sequence, sequence to entry, number to sequence
- **Location Tracking**: Sequence to satpoint, sat to sequence, outpoint to inscriptions
- **Relationships**: Parent-child mappings, delegation tracking
- **Content Storage**: Inscription content and metadata
- **Indexing**: Height, content type, metaprotocol, transaction, and address indexes

### View Functions

All view functions use lowercase concatenated naming as required:

- `inscription` - Get inscription by ID or number
- `inscriptions` - List inscriptions with pagination and filtering
- `children` - Get child inscription IDs
- `parents` - Get parent inscription IDs
- `content` - Get inscription content and content type
- `metadata` - Get inscription metadata
- `sat` - Get sat information and rarity
- `satinscriptions` - Get inscriptions on a sat
- `satinscription` - Get inscription on a sat
- `satinscriptioncontent` - Get content of inscription on a sat
- `childinscriptions` - Get child inscriptions with full info
- `parentinscriptions` - Get parent inscriptions with full info
- `undelegatedcontent` - Get undelegated content
- `utxo` - Get UTXO information
- `blockhash` - Get block hash by height
- `blockhashatheight` - Get block hash at height
- `blockheight` - Get block height by hash
- `blocktime` - Get block timestamp
- `blockinfo` - Get block information
- `tx` - Get transaction information

## Building

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- Protocol Buffers compiler (`protoc`)
- Required Rust crates (see `Cargo.toml`)

### Build Commands

```bash
# Build the WASM module
./scripts/build.sh

# Or manually:
cargo build --release --target wasm32-unknown-unknown
```

### Testing

```bash
# Run all tests
./scripts/test.sh

# Or manually:
cargo test --lib                    # Unit tests
cargo test --test integration_tests # Integration tests
cargo test --test view_tests        # View function tests
```

## Usage

### WASM Entry Point

The main entry point is the `_start()` function which processes one block at a time:

```rust
#[no_mangle]
pub extern "C" fn _start() {
    // Load input (height + block data)
    let input = metashrew_core::host::__load_input();
    
    // Parse height and block
    let height = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let block_data = &input[4..];
    let block: Block = bitcoin::consensus::deserialize(block_data).unwrap();
    
    // Index the block
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state().unwrap();
    indexer.index_block(&block, height).unwrap();
}
```

### View Function Usage

View functions are called via the metashrew message protocol:

```rust
// Example: Get inscription by ID
let request = InscriptionRequest {
    id: "abc123...i0".to_string(),
    ..Default::default()
};
let request_bytes = request.write_to_bytes().unwrap();
let response_bytes = call_view_function("inscription", &request_bytes);
let response = InscriptionResponse::parse_from_bytes(&response_bytes).unwrap();
```

## Inscription Processing

### Envelope Parsing

The indexer parses inscription envelopes from Bitcoin transaction witnesses:

```
OP_FALSE
OP_IF
  <content-type-tag> <content-type>
  <metadata-tag> <metadata>
  <parent-tag> <parent-id>
  OP_0
  <content-data>
OP_ENDIF
```

### Cursed vs Blessed Logic

- **Blessed Inscriptions**: Properly formatted, positive numbers starting from 0
- **Cursed Inscriptions**: Malformed or duplicate fields, negative numbers starting from -1
- **Jubilee Height**: After block 824544, cursed inscriptions become blessed

### Parent-Child Relationships

Inscriptions can reference parent inscriptions using the parent field (tag 3), creating hierarchical relationships that are tracked in the database.

### Delegation

Inscriptions can delegate their content to other inscriptions using the delegate field (tag 11), allowing for content reuse and references.

## Testing

The project includes comprehensive testing:

### Unit Tests
- Inscription data structure serialization
- Envelope parsing logic
- Database operations
- View function implementations

### Integration Tests
- Full block indexing workflows
- Parent-child relationship tracking
- Cursed inscription handling
- State persistence

### Test Utilities
- Helper functions for creating test data
- Bitcoin transaction and block builders
- Assertion utilities for common test patterns

## Development

### Project Structure

```
shrewscriptions-rs/
├── src/
│   ├── lib.rs              # Main WASM entry point
│   ├── indexer.rs          # Core indexing logic
│   ├── envelope.rs         # Envelope parsing
│   ├── inscription.rs      # Data structures
│   ├── tables.rs           # Database schema
│   ├── view.rs             # View functions
│   └── message.rs          # Message handling
├── proto/
│   └── shrewscriptions.proto # Protobuf schema
├── tests/
│   ├── integration_tests.rs # Integration tests
│   ├── view_tests.rs       # View function tests
│   └── test_utils.rs       # Test utilities
├── scripts/
│   ├── build.sh            # Build script
│   └── test.sh             # Test script
├── memory-bank/            # Reference documentation
└── Cargo.toml              # Project configuration
```

### Adding New Features

1. **New Inscription Fields**: Add parsing logic in `envelope.rs` and storage in `indexer.rs`
2. **New View Functions**: Add protobuf messages, implement in `view.rs`, register in `message.rs`, export in `lib.rs`
3. **New Database Tables**: Add to `tables.rs` and update indexing logic
4. **New Tests**: Add test cases and update test utilities as needed

## Reference Implementation

This implementation is based on analysis of:

- **alkanes-rs**: For metashrew environment patterns and WASM structure
- **metashrew-core**: For storage abstractions and host-guest interface
- **ord**: For inscription indexing logic and database schema

All reference documentation is available in the `memory-bank/` directory.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Acknowledgments

- The ord project for inscription indexing patterns
- The metashrew project for the WASM indexing framework
- The alkanes-rs project for implementation reference