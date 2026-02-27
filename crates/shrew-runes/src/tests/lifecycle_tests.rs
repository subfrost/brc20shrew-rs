use crate::balance_sheet::{BalanceSheet, RuneId};
use crate::rune_indexer::RuneEntry;
use crate::tables::*;
use bitcoin::{OutPoint, Transaction, TxIn, TxOut, ScriptBuf, Sequence, Witness};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use ordinals::{Runestone, Etching, Rune, Terms, Edict, RuneId as OrdRuneId};
use shrew_test_helpers::state::{clear, get_test_address};
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs, create_block_with_coinbase_tx};
use shrew_test_helpers::indexing::index_runes_block;

/// Helper: create a runestone tx
fn make_runestone_tx(runestone: &Runestone) -> Transaction {
    let address = get_test_address(0);
    let script_pubkey = runestone.encipher();
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
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

/// Helper: create a mint tx for a given rune_id
fn make_mint_tx(rune_id: &RuneId) -> Transaction {
    let runestone = Runestone {
        mint: Some(OrdRuneId { block: rune_id.block, tx: rune_id.tx }),
        ..Default::default()
    };
    make_runestone_tx(&runestone)
}

/// Helper: create a mint block
fn make_mint_block(rune_id: &RuneId, height: u32) -> bitcoin::Block {
    let tx = make_mint_tx(rune_id);
    create_block_with_txs(vec![create_coinbase_transaction(height), tx])
}

#[test]
fn test_etch_mint_transfer_lifecycle() {
    clear();
    let height = 840000u32;

    // Step 1: Etch a rune with terms allowing minting
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("LIFECYCLE".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: Some('L'),
            premine: Some(500),
            terms: Some(Terms {
                amount: Some(100),
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

    // Verify premine
    let etch_tx = &etch_block.txdata[1];
    let premine_outpoint = OutPoint { txid: etch_tx.txid(), vout: 0 };
    let pre_balance = read_balance(&premine_outpoint);
    assert_eq!(pre_balance.get(&rune_id), 500, "Premine should be 500");

    // Step 2: Mint at next block
    let mint_height = height + 1;
    let mint_block = make_mint_block(&rune_id, mint_height);
    index_runes_block(&mint_block, mint_height);

    let mint_tx = &mint_block.txdata[1];
    let mint_outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let mint_balance = read_balance(&mint_outpoint);
    assert_eq!(mint_balance.get(&rune_id), 100, "Mint should yield 100");

    // Step 3: Transfer via edict (spend the mint outpoint)
    let transfer_height = height + 2;
    let address = get_test_address(0);
    let edict_runestone = Runestone {
        edicts: vec![Edict {
            id: OrdRuneId { block: rune_id.block, tx: rune_id.tx },
            amount: 60,
            output: 0,
        }],
        ..Default::default()
    };
    let script_pubkey = edict_runestone.encipher();
    let transfer_tx = Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: mint_outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut { value: 10000, script_pubkey: address.script_pubkey() },
            TxOut { value: 0, script_pubkey },
        ],
    };
    let transfer_block = create_block_with_txs(vec![create_coinbase_transaction(transfer_height), transfer_tx.clone()]);
    index_runes_block(&transfer_block, transfer_height);

    // Output 0 gets 60 from edict + 40 remaining (unallocated -> default output 0)
    let out0 = OutPoint { txid: transfer_tx.txid(), vout: 0 };
    let final_balance = read_balance(&out0);
    assert_eq!(final_balance.get(&rune_id), 100, "Output 0 should have 60 + 40 remaining = 100");

    // Verify RuneEntry supply
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).unwrap();
    assert_eq!(entry.supply, 600, "Supply should be premine(500) + mint(100) = 600");
    assert_eq!(entry.mints, 1, "Should have 1 mint recorded");
}

#[test]
fn test_multi_rune_block() {
    clear();
    let height = 840000u32;

    // Two etchings in the same block via two different transactions
    let runestone_a = Runestone {
        etching: Some(Etching {
            rune: Some(Rune(1000)),
            divisibility: Some(0),
            symbol: Some('A'),
            premine: Some(100),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx_a = make_runestone_tx(&runestone_a);

    let runestone_b = Runestone {
        etching: Some(Etching {
            rune: Some(Rune(2000)),
            divisibility: Some(2),
            symbol: Some('B'),
            premine: Some(200),
            terms: None,
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx_b = make_runestone_tx(&runestone_b);

    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        tx_a.clone(),
        tx_b.clone(),
    ]);
    index_runes_block(&block, height);

    let rune_id_a = RuneId::new(height as u64, 1);
    let rune_id_b = RuneId::new(height as u64, 2);

    // Both runes should exist
    let entry_a_bytes = RUNE_ID_TO_ENTRY.select(&rune_id_a.to_bytes()).get();
    assert!(!entry_a_bytes.is_empty(), "Rune A should be etched");
    let entry_a: RuneEntry = bincode::deserialize(&entry_a_bytes).unwrap();
    assert_eq!(entry_a.symbol, Some('A'));
    assert_eq!(entry_a.premine, 100);

    let entry_b_bytes = RUNE_ID_TO_ENTRY.select(&rune_id_b.to_bytes()).get();
    assert!(!entry_b_bytes.is_empty(), "Rune B should be etched");
    let entry_b: RuneEntry = bincode::deserialize(&entry_b_bytes).unwrap();
    assert_eq!(entry_b.symbol, Some('B'));
    assert_eq!(entry_b.premine, 200);

    // Check balances on respective outputs
    let out_a = OutPoint { txid: tx_a.txid(), vout: 0 };
    let bal_a = read_balance(&out_a);
    assert_eq!(bal_a.get(&rune_id_a), 100);

    let out_b = OutPoint { txid: tx_b.txid(), vout: 0 };
    let bal_b = read_balance(&out_b);
    assert_eq!(bal_b.get(&rune_id_b), 200);
}

#[test]
fn test_rune_across_blocks() {
    clear();
    let etch_height = 840000u32;
    let mint_height = 840001u32;

    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("CROSSBLK".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: None,
            premine: Some(0),
            terms: Some(Terms {
                amount: Some(50),
                cap: Some(100),
                height: (None, None),
                offset: (None, None),
            }),
            spacers: None,
            turbo: false,
        }),
        ..Default::default()
    };
    let tx = make_runestone_tx(&runestone);
    let etch_block = create_block_with_txs(vec![create_coinbase_transaction(etch_height), tx]);
    index_runes_block(&etch_block, etch_height);

    let rune_id = RuneId::new(etch_height as u64, 1);

    // Mint in the next block
    let mint_block = make_mint_block(&rune_id, mint_height);
    index_runes_block(&mint_block, mint_height);

    let mint_tx = &mint_block.txdata[1];
    let outpoint = OutPoint { txid: mint_tx.txid(), vout: 0 };
    let balance = read_balance(&outpoint);
    assert_eq!(balance.get(&rune_id), 50, "Mint across blocks should work");

    // Verify entry
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).unwrap();
    assert_eq!(entry.mints, 1);
    assert_eq!(entry.supply, 50);
    assert_eq!(entry.etching_height, etch_height);
}

#[test]
fn test_rune_with_premine_and_mints() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("PREMINT".parse::<Rune>().unwrap()),
            divisibility: Some(0),
            symbol: Some('P'),
            premine: Some(1000),
            terms: Some(Terms {
                amount: Some(250),
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

    // 2 mints
    let mint_block_1 = make_mint_block(&rune_id, height + 1);
    index_runes_block(&mint_block_1, height + 1);

    let mint_block_2 = make_mint_block(&rune_id, height + 2);
    index_runes_block(&mint_block_2, height + 2);

    // Verify total supply = premine + 2 * mint_amount = 1000 + 500 = 1500
    let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    let entry: RuneEntry = bincode::deserialize(&entry_bytes).unwrap();
    assert_eq!(entry.premine, 1000, "Premine should be 1000");
    assert_eq!(entry.mints, 2, "Should have 2 mints");
    assert_eq!(entry.supply, 1500, "Supply should be premine(1000) + 2*250 = 1500");

    // Verify mints remaining decreased
    let remaining_bytes = RUNE_MINTS_REMAINING.select(&rune_id.to_bytes()).get();
    let remaining = u128::from_le_bytes(remaining_bytes[..16].try_into().unwrap());
    assert_eq!(remaining, 8, "Mints remaining should be cap(10) - 2 = 8");
}

#[test]
fn test_multiple_edicts_same_tx() {
    clear();
    let height = 840000u32;
    let runestone = Runestone {
        etching: Some(Etching {
            rune: Some("MULTIEDT".parse::<Rune>().unwrap()),
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

    // Build tx with 3 edicts: send 100, 200, 300 to output 0
    let address = get_test_address(0);
    let ord_id = OrdRuneId { block: rune_id.block, tx: rune_id.tx };
    let edict_runestone = Runestone {
        edicts: vec![
            Edict { id: ord_id, amount: 100, output: 0 },
            Edict { id: ord_id, amount: 200, output: 0 },
            Edict { id: ord_id, amount: 300, output: 0 },
        ],
        ..Default::default()
    };
    let script_pubkey = edict_runestone.encipher();
    let edict_tx = Transaction {
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

    let transfer_height = height + 1;
    let transfer_block = create_block_with_txs(vec![create_coinbase_transaction(transfer_height), edict_tx.clone()]);
    index_runes_block(&transfer_block, transfer_height);

    // Output 0 should get 100+200+300=600 from edicts + 400 remaining = 1000
    let out0 = OutPoint { txid: edict_tx.txid(), vout: 0 };
    let balance = read_balance(&out0);
    assert_eq!(balance.get(&rune_id), 1000, "3 edicts + remainder should total 1000 at output 0");
}

#[test]
fn test_empty_blocks_at_activation() {
    clear();
    // Index several empty blocks at and after activation height
    for h in 840000..840005u32 {
        let block = create_block_with_coinbase_tx(h);
        index_runes_block(&block, h);
    }

    // No events at any height (coinbase-only blocks have no runestones)
    for h in 840000..840005u32 {
        let events_data = HEIGHT_TO_RUNE_EVENTS.select(&h.to_le_bytes().to_vec()).get();
        assert!(events_data.is_empty(), "Empty blocks should produce no rune events at height {}", h);
    }
}
