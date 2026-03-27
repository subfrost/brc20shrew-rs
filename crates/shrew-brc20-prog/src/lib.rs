#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod prog_indexer;
pub mod controller;
pub mod trace_hash;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_brc20_prog.rs"));
}

pub use prog_indexer::ProgrammableBrc20Indexer;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        // Run inscription indexer
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        // Run BRC20 indexer
        let brc20_indexer = shrew_brc20::Brc20Indexer::new();
        brc20_indexer.process_block(&block, height);

        // Run programmable BRC20 indexer
        let mut prog_indexer = ProgrammableBrc20Indexer::new();
        prog_indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn call(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::CallRequest = from_slice(input)?;
    Ok(to_vec(&view::call(&req)?)?)
}

/// Read a storage slot from any EVM account.
/// Input: JSON { "address": "0x...", "slot": "0x..." }
/// Returns: JSON { "value": "0x..." }
#[metashrew_core::view]
pub fn storage_at(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use shrew_evm::database::MetashrewDB;
    use revm::Database;
    use revm::primitives::{Address, U256};

    let req: serde_json::Value = serde_json::from_slice(input)?;
    let addr_hex = req["address"].as_str().unwrap_or("");
    let slot_hex = req["slot"].as_str().unwrap_or("0x0");

    let addr_bytes = hex::decode(addr_hex.strip_prefix("0x").unwrap_or(addr_hex))?;
    if addr_bytes.len() != 20 {
        return Ok(serde_json::to_vec(&serde_json::json!({"error": "invalid address"}))?);
    }
    let address = Address::from_slice(&addr_bytes);

    let slot_bytes = hex::decode(slot_hex.strip_prefix("0x").unwrap_or(slot_hex))?;
    let mut slot_arr = [0u8; 32];
    let start = 32usize.saturating_sub(slot_bytes.len());
    slot_arr[start..].copy_from_slice(&slot_bytes[..slot_bytes.len().min(32)]);
    let slot = U256::from_be_bytes(slot_arr);

    let mut db = MetashrewDB;
    let value = db.storage(address, slot)?;

    let value_hex = format!("0x{}", hex::encode(value.to_be_bytes::<32>()));
    Ok(serde_json::to_vec(&serde_json::json!({"value": value_hex}))?)
}

/// Read account info (code size, nonce, balance) at an address.
/// Input: JSON { "address": "0x..." }
/// Returns: JSON { "code_size": N, "nonce": N, "has_code": bool, "code_hash": "0x..." }
#[metashrew_core::view]
pub fn code_at(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use shrew_evm::database::MetashrewDB;
    use revm::Database;
    use revm::primitives::Address;

    let req: serde_json::Value = serde_json::from_slice(input)?;
    let addr_hex = req["address"].as_str().unwrap_or("");
    let addr_bytes = hex::decode(addr_hex.strip_prefix("0x").unwrap_or(addr_hex))?;
    if addr_bytes.len() != 20 {
        return Ok(serde_json::to_vec(&serde_json::json!({"error": "invalid address"}))?);
    }
    let address = Address::from_slice(&addr_bytes);

    let mut db = MetashrewDB;
    match db.basic(address)? {
        Some(info) => {
            let code_size = info.code.as_ref().map(|c| c.len()).unwrap_or(0);
            let code_hash_hex = format!("0x{}", hex::encode(info.code_hash.as_slice()));
            Ok(serde_json::to_vec(&serde_json::json!({
                "code_size": code_size,
                "nonce": info.nonce,
                "has_code": code_size > 0,
                "code_hash": code_hash_hex,
                "code_is_none": info.code.is_none(),
            }))?)
        }
        None => {
            Ok(serde_json::to_vec(&serde_json::json!({
                "code_size": 0,
                "nonce": 0,
                "has_code": false,
                "code_hash": "none",
                "exists": false,
            }))?)
        }
    }
}

/// Debug view: returns the last processed inscription content and EVM execution result.
/// Call via metashrew_view ["debug", "0x", "latest"]
#[metashrew_core::view]
pub fn debug(_input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use metashrew_support::index_pointer::KeyValuePointer;
    let last_inscription = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_inscription").get();
    let last_result = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_result").get();
    let last_commit = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_commit").get();
    let last_deploy = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_deploy").get();
    let last_deploy_result = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_deploy_result").get();
    // Also get deploy info for the proxy size (677 bytes * 2 + 2 = 1356 chars)
    let proxy_deploy = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/deploy/1356").get();
    let response = serde_json::json!({
        "last_inscription": String::from_utf8_lossy(&last_inscription).to_string(),
        "last_result": String::from_utf8_lossy(&last_result).to_string(),
        "last_commit": String::from_utf8_lossy(&last_commit).to_string(),
        "last_deploy": String::from_utf8_lossy(&last_deploy).to_string(),
        "last_deploy_result": String::from_utf8_lossy(&last_deploy_result).to_string(),
        "proxy_deploy": String::from_utf8_lossy(&proxy_deploy).to_string(),
    });
    Ok(serde_json::to_vec(&response)?)
}

#[cfg(test)]
mod tests;
