use anyhow::Result;
use bitcoin::Block;
use metashrew_core::{flush, input};
use std::io::Cursor;
use bitcoin::consensus::Decodable;

pub mod indexer;
pub mod tables;
pub mod inscription;
pub mod envelope;
pub mod view;
pub mod message;

#[cfg(feature = "test-utils")]
pub mod test_utils;

use indexer::InscriptionIndexer;
use message::InscriptionMessageContext;

/// Main WASM export function for indexing a block
#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn _start() {
    let data = input();
    let height = u32::from_le_bytes((&data[0..4]).try_into().unwrap());
    let block = Block::consensus_decode(&mut Cursor::new(&data[4..])).unwrap();
    
    match InscriptionIndexer::index_block::<InscriptionMessageContext>(block, height as u64) {
        Ok(_) => {
            flush();
        }
        Err(e) => {
            eprintln!("Error indexing block {}: {:?}", height, e);
        }
    }
}

/// View function exports - lowercase concatenated names without underscores
#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn inscription() {
    view::handle_inscription();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn inscriptions() {
    view::handle_inscriptions();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn children() {
    view::handle_children();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn parents() {
    view::handle_parents();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn content() {
    view::handle_content();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn metadata() {
    view::handle_metadata();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn sat() {
    view::handle_sat();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn satinscriptions() {
    view::handle_sat_inscriptions();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn satinscription() {
    view::handle_sat_inscription();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn satinscriptioncontent() {
    view::handle_sat_inscription_content();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn childinscriptions() {
    view::handle_child_inscriptions();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn parentinscriptions() {
    view::handle_parent_inscriptions();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn undelegatedcontent() {
    view::handle_undelegated_content();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn utxo() {
    view::handle_utxo();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn blockhash() {
    view::handle_blockhash();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn blockhashatheight() {
    view::handle_blockhash_at_height();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn blockheight() {
    view::handle_blockheight();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn blocktime() {
    view::handle_blocktime();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn blockinfo() {
    view::handle_blockinfo();
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn tx() {
    view::handle_tx();
}