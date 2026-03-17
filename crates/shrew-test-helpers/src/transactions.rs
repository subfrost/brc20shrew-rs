use bitcoin::{Amount, Transaction, TxIn, TxOut, OutPoint, Witness, ScriptBuf, Sequence, Txid, Address};
use bitcoin::address::NetworkChecked;
use bitcoin::transaction::Version;
use std::str::FromStr;

use crate::state::get_test_address;
use crate::inscriptions::create_inscription_envelope;

/// Create a transaction with inscription data in the witness
pub fn create_inscription_transaction(
    content: &[u8],
    content_type: &str,
    previous_output: Option<OutPoint>,
) -> Transaction {
    let witness = create_inscription_envelope(content_type.as_bytes(), content);
    let prev_out = previous_output.unwrap_or_else(|| OutPoint {
        txid: Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        vout: 0,
    });
    let address = get_test_address(0);
    Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: prev_out,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness,
        }],
        output: vec![TxOut {
            value: Amount::from_sat(100_000_000),
            script_pubkey: address.script_pubkey(),
        }],
    }
}

/// Create a transaction with inscription data, sending to a specific address
pub fn create_inscription_transaction_to_address(
    content: &[u8],
    content_type: &str,
    previous_output: Option<OutPoint>,
    to_address: &Address<NetworkChecked>,
) -> Transaction {
    let witness = create_inscription_envelope(content_type.as_bytes(), content);
    let prev_out = previous_output.unwrap_or_else(|| OutPoint {
        txid: Txid::from_str("1111111111111111111111111111111111111111111111111111111111111111").unwrap(),
        vout: 0,
    });
    Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: prev_out,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness,
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10000),
            script_pubkey: to_address.script_pubkey(),
        }],
    }
}

/// Create a transfer transaction (no inscription, just moves an outpoint)
pub fn create_transfer_transaction(prev_txid: &Txid, prev_vout: u32) -> Transaction {
    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(*prev_txid, prev_vout),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10000),
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a transfer transaction that moves an inscription to a specific address
pub fn create_transfer_transaction_to_address(
    previous_output: OutPoint,
    to_address: &Address<NetworkChecked>,
) -> Transaction {
    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10000),
            script_pubkey: to_address.script_pubkey(),
        }],
    }
}

/// Create a reveal transaction that spends from a commit transaction
pub fn create_reveal_transaction(commit_txid: &Txid, witness: Witness) -> Transaction {
    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(*commit_txid, 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10000),
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a transaction with multiple inscription inputs
pub fn create_multi_inscription_transaction(commit_txid: &Txid, witnesses: Vec<Witness>) -> Transaction {
    let inputs = witnesses.into_iter().enumerate().map(|(i, witness)| {
        TxIn {
            previous_output: OutPoint::new(*commit_txid, i as u32),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        }
    }).collect();
    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: inputs,
        output: vec![TxOut {
            value: Amount::from_sat(10000),
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a basic test transaction
pub fn create_test_transaction() -> Transaction {
    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(5_000_000_000),
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a mock outpoint for testing
pub fn create_mock_outpoint(n: u32) -> OutPoint {
    OutPoint {
        txid: Txid::from_str(&format!(
            "000000000000000000000000000000000000000000000000000000000000000{}",
            n
        )).unwrap(),
        vout: 0,
    }
}
