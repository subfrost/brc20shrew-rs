use bitcoin::{Amount, Block, Transaction, TxIn, TxOut, OutPoint, Witness, ScriptBuf, Sequence};
use ordinals::{Runestone, Etching, Rune, Terms, Edict, RuneId as OrdRuneId};
use shrew_runes::balance_sheet::{BalanceSheet, RuneId};
use shrew_runes::tables::RUNE_BALANCES_BY_OUTPOINT;
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin_hashes::Hash;

use crate::blocks::{create_coinbase_transaction, create_block_with_txs};
use crate::state::get_test_address;

/// Create a transaction containing a Runestone OP_RETURN
pub fn create_runestone_tx(runestone: &Runestone) -> Transaction {
    let address = get_test_address(0);
    let script_pubkey = runestone.encipher();

    Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(10000),
                script_pubkey: address.script_pubkey(),
            },
            TxOut {
                value: Amount::ZERO,
                script_pubkey,
            },
        ],
    }
}

/// Create a block with a rune etching
pub fn create_etching_block(
    name: &str,
    divisibility: u8,
    symbol: Option<char>,
    premine: u128,
    terms: Option<Terms>,
    height: u32,
) -> (Block, RuneId) {
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some(name.parse::<Rune>().unwrap_or(Rune(0))),
            divisibility: Some(divisibility),
            symbol,
            premine: Some(premine),
            terms,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = create_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    let rune_id = RuneId::new(height as u64, 1); // tx_index=1 (after coinbase)
    (block, rune_id)
}

/// Create a block with a rune mint
pub fn create_mint_block(rune_id: RuneId, height: u32) -> Block {
    let runestone = Runestone {
        mint: Some(OrdRuneId { block: rune_id.block, tx: rune_id.tx }),
        ..Default::default()
    };
    let tx = create_runestone_tx(&runestone);
    create_block_with_txs(vec![create_coinbase_transaction(height), tx])
}

/// Create a block with rune edicts (transfers)
pub fn create_edict_block(rune_id: RuneId, edicts: Vec<(u128, u32)>, height: u32) -> Block {
    let runestone = Runestone {
        edicts: edicts.into_iter().map(|(amount, output)| Edict {
            id: OrdRuneId { block: rune_id.block, tx: rune_id.tx },
            amount,
            output,
        }).collect(),
        ..Default::default()
    };
    let tx = create_runestone_tx(&runestone);
    create_block_with_txs(vec![create_coinbase_transaction(height), tx])
}

/// Get rune balance for an outpoint
pub fn get_rune_balance(outpoint: &OutPoint) -> BalanceSheet {
    let outpoint_bytes: Vec<u8> = outpoint.txid.as_byte_array().iter()
        .chain(outpoint.vout.to_le_bytes().iter()).copied().collect();
    let data = RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).get();
    if data.is_empty() {
        BalanceSheet::new()
    } else {
        BalanceSheet::from_bytes(&data).unwrap_or_default()
    }
}
