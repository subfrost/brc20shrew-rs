use bitcoin::{Block, consensus::deserialize};

// Re-export modules
pub mod tables;
pub mod inscription;
pub mod envelope;
pub mod indexer;
pub mod view;
pub mod message;
use metashrew_core::{input, flush};

// Re-export protobuf types
pub mod proto {
    pub mod shrewscriptions {
        include!(concat!(env!("OUT_DIR"), "/shrewscriptions.rs"));
    }
}

// Re-export view functions for testing
pub use view::*;

// Test modules
#[cfg(any(feature = "test-utils", test))]
pub mod tests;

// WASM-specific exports - only compile for WASM target
#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::*;

    // Simple WASM entry point for now
    #[no_mangle]
    pub extern "C" fn _start() {
        // Load input (height + block data)
        let input = input();
        
        // Parse height and block
        let height = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
        let block_data = &input[4..];
        
        // Try to deserialize block
        if let Ok(block) = deserialize::<Block>(block_data) {
            // Create indexer and process block
            let mut indexer = indexer::InscriptionIndexer::new();
            let _ = indexer.load_state();
            let _ = indexer.index_block(&block, height);
        }
        flush();
    }

    // Placeholder view functions for now
    #[no_mangle]
    pub extern "C" fn inscription() -> *const u8 {
        b"inscription".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn inscriptions() -> *const u8 {
        b"inscriptions".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn children() -> *const u8 {
        b"children".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn parents() -> *const u8 {
        b"parents".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn content() -> *const u8 {
        b"content".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn metadata() -> *const u8 {
        b"metadata".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn sat() -> *const u8 {
        b"sat".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn satinscriptions() -> *const u8 {
        b"satinscriptions".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn satinscription() -> *const u8 {
        b"satinscription".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn satinscriptioncontent() -> *const u8 {
        b"satinscriptioncontent".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn childinscriptions() -> *const u8 {
        b"childinscriptions".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn parentinscriptions() -> *const u8 {
        b"parentinscriptions".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn undelegatedcontent() -> *const u8 {
        b"undelegatedcontent".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn utxo() -> *const u8 {
        b"utxo".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn blockhash() -> *const u8 {
        b"blockhash".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn blockhashatheight() -> *const u8 {
        b"blockhashatheight".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn blockheight() -> *const u8 {
        b"blockheight".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn blocktime() -> *const u8 {
        b"blocktime".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn blockinfo() -> *const u8 {
        b"blockinfo".as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn tx() -> *const u8 {
        b"tx".as_ptr()
    }

    // Helper function to load input
    fn load_input() -> Vec<u8> {
        // This would normally call metashrew host function
        // For now, return empty vec
        Vec::new()
    }
}
