use crate::balance_sheet::{BalanceSheet, RuneId};
use crate::rune_indexer::{RuneEntry, RuneEvent};
use crate::tables::*;
use bitcoin::{OutPoint, Transaction, TxIn, TxOut, ScriptBuf, Sequence, Witness};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use ordinals::{Runestone, Etching, Rune, Terms, Edict, RuneId as OrdRuneId};
use shrew_test_helpers::state::{clear, get_test_address};
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs, create_block_with_coinbase_tx};
use shrew_test_helpers::indexing::index_runes_block;

/// Helper: create a runestone tx with a unique txid based on nonce
fn make_runestone_tx_with_nonce(runestone: &Runestone, nonce: u32) -> Transaction {
    let address = get_test_address(0);
    let script_pubkey = runestone.encipher();
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::from_consensus(nonce),
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut { value: 10000, script_pubkey: address.script_pubkey() },
            TxOut { value: 0, script_pubkey },
        ],
    }
}

/// Helper: create a runestone tx (nonce=0)
fn make_runestone_tx(runestone: &Runestone) -> Transaction {
    make_runestone_tx_with_nonce(runestone, 0)
}

/// Helper: read balance sheet from an outpoint
fn read_balance(outpoint: &OutPoint) -> BalanceSheet {
    let outpoint_bytes: Vec<u8> = outpoint.txid.as_byte_array().iter()
        .chain(outpoint.vout.to_le_bytes().iter()).copied().collect();
    let data = RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).get();
    if data.is_empty() {
        BalanceSheet::new()
    } else {
        BalanceSheet::from_bytes(&data).unwrap_or_default()
    }
}

// ---- Activation tests ----

#[test]
fn test_no_runes_before_activation() {
    clear();
    let block = create_block_with_coinbase_tx(100);
    index_runes_block(&block, 100);

    let events_data = HEIGHT_TO_RUNE_EVENTS.select(&100u32.to_le_bytes().to_vec()).get();
    assert!(events_data.is_empty(), "No rune events should exist before activation height");
}

// ---- Etching tests ----

#[test]
fn test_etching_creates_rune() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("TESTRUNE".parse::<Rune>().unwrap()),
            divisibility: Some(8),
            symbol: Some('$'),
            premine: Some(1000),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    assert!(!entry_bytes.is_empty(), "RuneEntry should be stored after etching");

    let entry: RuneEntry = bincode::deserialize(&entry_bytes).expect("should deserialize RuneEntry");
    assert_eq!(entry.id, rune_id);
    assert_eq!(entry.name, "TESTRUNE");
    assert_eq!(entry.divisibility, 8);
    assert_eq!(entry.symbol, Some('$'));
    assert_eq!(entry.premine, 1000);
    assert_eq!(entry.supply, 1000);
    assert_eq!(entry.mints, 0);
    assert_eq!(entry.etching_height, height);
}

#[test]
fn test_etching_with_premine() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("PREMINED".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(5000),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etching_tx = &block.txdata[1];
    let outpoint = OutPoint { txid: etching_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 5000, "Premine should be credited to first output");
}

#[test]
fn test_etching_with_terms() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("MINTABLE".parse::<Rune>().unwrap()),
            divisibility: Some(2),
            symbol: Some('M'),
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(1000),
                height: (Some(840000), Some(850000)),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).expect("should deserialize");
    assert!(entry.terms.is_some(), "Terms should be stored");
    let stored_terms = entry.terms.unwrap();
    assert_eq!(stored_terms.amount, Some(100));
    assert_eq!(stored_terms.cap, Some(1000));
    assert_eq!(stored_terms.height_start, Some(840000));
    assert_eq!(stored_terms.height_end, Some(850000));

    // Cap should be stored
    let cap_bytes = RUNE_CAP.select(&rune_id.to_bytes()).get();
    assert!(!cap_bytes.is_empty(), "RUNE_CAP should be set");
    let cap = u128::from_le_bytes(cap_bytes[..16].try_into().unwrap());
    assert_eq!(cap, 1000);

    // Mints remaining should equal cap
    let remaining_bytes = RUNE_MINTS_REMAINING.select(&rune_id.to_bytes()).get();
    let remaining = u128::from_le_bytes(remaining_bytes[..16].try_into().unwrap());
    assert_eq!(remaining, 1000);
}

#[test]
fn test_etching_name_stored() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("NAMETEST".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let stored_id_bytes = RUNE_NAME_TO_ID.select(&"NAMETEST".as_bytes().to_vec()).get();
    assert!(!stored_id_bytes.is_empty(), "RUNE_NAME_TO_ID should be populated");
    let stored_id = RuneId::from_bytes(&stored_id_bytes).expect("should parse RuneId");
    assert_eq!(stored_id, rune_id);
}

#[test]
fn test_etching_divisibility_and_symbol() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some(Rune(12345678)),
            divisibility: Some(18),
            symbol: Some('%'),
            premine: Some(0),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).expect("should deserialize");
    assert_eq!(entry.divisibility, 18);
    assert_eq!(entry.symbol, Some('%'));
    assert_eq!(entry.name, Rune(12345678).to_string());
}

// ---- Mint tests ----

/// Helper: create a mint tx for a given rune_id with unique nonce
fn make_mint_tx(rune_id: &RuneId, nonce: u32) -> Transaction {
    let runestone = Runestone {
        mint: Some(OrdRuneId { block: rune_id.block, tx: rune_id.tx }),
        ..Default::default()
    };
    make_runestone_tx_with_nonce(&runestone, nonce)
}

/// Helper: create a mint block (uses height as nonce for uniqueness)
fn make_mint_block(rune_id: &RuneId, height: u32) -> bitcoin::Block {
    let tx = make_mint_tx(rune_id, height);
    create_block_with_txs(vec![create_coinbase_transaction(height), tx])
}

#[test]
fn test_mint_basic() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("MINTRUNE".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(500),
                cap: Some(10),
                height: (None, None),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let mint_height = height + 1;
    let mint_block = make_mint_block(&rune_id, mint_height);
    index_runes_block(&mint_block, mint_height);

    let mint_tx = &mint_block.txdata[1];
    let outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 500, "Mint should credit the amount to output 0");
}

#[test]
fn test_mint_increments_count() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("COUNTME".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(5),
                height: (None, None),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);

    for i in 1..=3u32 {
        let mint_block = make_mint_block(&rune_id, height + i);
        index_runes_block(&mint_block, height + i);
    }

    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).expect("should deserialize");
    assert_eq!(entry.mints, 3, "Mints counter should be 3 after 3 mints");
    assert_eq!(entry.supply, 300, "Supply should be 3 * 100 = 300");
}

#[test]
fn test_mint_before_start_height_rejected() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("EARLYMNT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(10),
                height: (Some(840010), None),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let mint_block = make_mint_block(&rune_id, 840005);
    index_runes_block(&mint_block, 840005);

    let mint_tx = &mint_block.txdata[1];
    let outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 0, "Mint before start_height should be rejected");
}

#[test]
fn test_mint_after_end_height_rejected() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("LATEMINT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(10),
                height: (None, Some(840005)),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let mint_block = make_mint_block(&rune_id, 840005);
    index_runes_block(&mint_block, 840005);

    let mint_tx = &mint_block.txdata[1];
    let outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 0, "Mint at or after end_height should be rejected");
}

#[test]
fn test_mint_cap_exhausted() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("CAPPED".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(100),
                cap: Some(1),
                height: (None, None),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);

    // First mint succeeds
    let mint_block_1 = make_mint_block(&rune_id, height + 1);
    index_runes_block(&mint_block_1, height + 1);
    let mint_tx_1 = &mint_block_1.txdata[1];
    let outpoint_1 = OutPoint { txid: mint_tx_1.txid(), vout: 0 };
    let balance_1 = read_balance(&outpoint_1);
    assert_eq!(balance_1.get(&rune_id), 100, "First mint should succeed");

    // Second mint should fail (cap exhausted)
    let mint_block_2 = make_mint_block(&rune_id, height + 2);
    index_runes_block(&mint_block_2, height + 2);
    let mint_tx_2 = &mint_block_2.txdata[1];
    let outpoint_2 = OutPoint { txid: mint_tx_2.txid(), vout: 0 };
    let balance_2 = read_balance(&outpoint_2);
    assert_eq!(balance_2.get(&rune_id), 0, "Second mint should fail after cap exhausted");
}

#[test]
fn test_mint_nonexistent_rune_ignored() {
    clear();
    let height = 840000u32;
    let fake_rune_id = RuneId::new(840000, 99);
    let mint_block = make_mint_block(&fake_rune_id, height);
    index_runes_block(&mint_block, height);

    let mint_tx = &mint_block.txdata[1];
    let outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert!(balance.is_empty(), "Minting a nonexistent rune should produce no balance");
}

// ---- Edict (transfer) tests ----

/// Helper: create a runestone tx that spends a specific outpoint
fn make_edict_tx(prev_outpoint: OutPoint, edicts: Vec<Edict>) -> Transaction {
    let address = get_test_address(0);
    let edict_runestone = Runestone {
        edicts,
        ..Default::default()
    };
    let script_pubkey = edict_runestone.encipher();
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: prev_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut { value: 10000, script_pubkey: address.script_pubkey() },
            TxOut { value: 0, script_pubkey },
        ],
    }
}

#[test]
fn test_edict_transfer() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("TRANSFER".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(1000),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &etch_block.txdata[1];
    let premine_outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };

    let edict_tx = make_edict_tx(premine_outpoint, vec![Edict {
        id: OrdRuneId { block: rune_id.block, tx: rune_id.tx },
        amount: 400,
        output: 0,
    }]);

    let transfer_height = height + 1;
    let transfer_block = create_block_with_txs(vec![create_coinbase_transaction(transfer_height), edict_tx.clone()]);
    index_runes_block(&transfer_block, transfer_height);

    // Output 0 should have 400 from edict + 600 remaining (default pointer=0)
    let out0 = OutPoint { txid: edict_tx.txid(), vout: 0 };
    let balance = read_balance(&out0);
    assert_eq!(balance.get(&rune_id), 1000, "Output 0 should get 400 from edict + 600 remaining");
}

#[test]
fn test_edict_transfer_all() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("XFERALL".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(500),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &etch_block.txdata[1];
    let premine_outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };

    // Edict with amount=0 means "transfer all"
    let edict_tx = make_edict_tx(premine_outpoint, vec![Edict {
        id: OrdRuneId { block: rune_id.block, tx: rune_id.tx },
        amount: 0,
        output: 0,
    }]);

    let transfer_height = height + 1;
    let transfer_block = create_block_with_txs(vec![create_coinbase_transaction(transfer_height), edict_tx.clone()]);
    index_runes_block(&transfer_block, transfer_height);

    let out0 = OutPoint { txid: edict_tx.txid(), vout: 0 };
    let balance = read_balance(&out0);
    assert_eq!(balance.get(&rune_id), 500, "Transfer-all edict should move entire balance");
}

#[test]
fn test_edict_invalid_output_cenotaph() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("BADOUT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(1000),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &etch_block.txdata[1];
    let premine_outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };

    // Edict pointing to output index 99 creates a cenotaph (EdictOutput flaw).
    // Per the runes protocol, a cenotaph burns all input runes.
    let edict_tx = make_edict_tx(premine_outpoint, vec![Edict {
        id: OrdRuneId { block: rune_id.block, tx: rune_id.tx },
        amount: 500,
        output: 99,
    }]);

    let transfer_height = height + 1;
    let transfer_block = create_block_with_txs(vec![create_coinbase_transaction(transfer_height), edict_tx.clone()]);
    index_runes_block(&transfer_block, transfer_height);

    // Cenotaph burns everything; no balance on any output
    let out0 = OutPoint { txid: edict_tx.txid(), vout: 0 };
    let balance = read_balance(&out0);
    assert_eq!(balance.get(&rune_id), 0, "Cenotaph from invalid edict output should burn all input runes");
}

#[test]
fn test_unallocated_to_default_output() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("DEFAULT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(777),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &block.txdata[1];
    let outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 777, "Unallocated runes should go to default output 0");
}

// ---- Collect input runes test ----

#[test]
fn test_collect_input_runes() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("COLLECT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(200),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&etch_block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &etch_block.txdata[1];
    let premine_outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };

    // Create a tx that spends this outpoint with an empty runestone (no edicts)
    let address = get_test_address(0);
    let simple_runestone = Runestone { ..Default::default() };
    let script_pubkey = simple_runestone.encipher();
    let spend_tx = Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: premine_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut { value: 10000, script_pubkey: address.script_pubkey() },
            TxOut { value: 0, script_pubkey },
        ],
    };

    let spend_height = height + 1;
    let spend_block = create_block_with_txs(vec![create_coinbase_transaction(spend_height), spend_tx.clone()]);
    index_runes_block(&spend_block, spend_height);

    // The input runes (200) should flow to default output 0
    let out0 = OutPoint { txid: spend_tx.txid(), vout: 0 };
    let balance = read_balance(&out0);
    assert_eq!(balance.get(&rune_id), 200, "Input runes should be collected and assigned to output");
}

// ---- Events test ----

#[test]
fn test_events_stored_per_height() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("EVENTS".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: Some('E'),
            premine: Some(100),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let events_data = HEIGHT_TO_RUNE_EVENTS.select(&height.to_le_bytes().to_vec()).get();
    assert!(!events_data.is_empty(), "HEIGHT_TO_RUNE_EVENTS should contain data after etching");

    let events: Vec<RuneEvent> =
        bincode::deserialize(&events_data).expect("events should deserialize");
    assert!(!events.is_empty(), "Should have at least one event (premine allocation)");

    // Check for the premine allocation event (event_type=1)
    let premine_events: Vec<_> = events.iter().filter(|e| e.event_type == 1).collect();
    assert!(!premine_events.is_empty(), "Should have a premine allocation event");
    assert_eq!(premine_events[0].amount, 100);
}

// ---- Mapping test ----

#[test]
fn test_etching_to_rune_id_mapping() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("MAPPED".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_runes_block(&block, height);

    let rune_id = RuneId::new(height as u64, 1);
    let etch_tx = &block.txdata[1];
    let etching_bytes = etch_tx.txid().as_byte_array().to_vec();

    // ETCHING_TO_RUNE_ID should map the etching txid to the rune id
    let stored_id_bytes = ETCHING_TO_RUNE_ID.select(&etching_bytes).get();
    assert!(!stored_id_bytes.is_empty(), "ETCHING_TO_RUNE_ID should be populated");
    let stored_id = RuneId::from_bytes(&stored_id_bytes).expect("should parse RuneId");
    assert_eq!(stored_id, rune_id);

    // And the reverse mapping
    let reverse_bytes = RUNE_ID_TO_ETCHING.select(&rune_id.to_bytes()).get();
    assert_eq!(reverse_bytes.as_ref(), &etching_bytes, "RUNE_ID_TO_ETCHING should point back to etching txid");
}
