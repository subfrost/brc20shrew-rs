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

/// Create a BRC20-Prog activation transaction.
/// Input[0] spends the reveal tx's inscription output.
/// Output[0] is OP_RETURN "BRC20PROG" (1 sat).
/// Output[1+] are additional outputs (e.g., dust to signer address).
/// Output[last] is change.
pub fn create_activation_transaction(
    reveal_txid: &Txid,
    reveal_vout: u32,
    additional_outputs: Vec<TxOut>,
) -> Transaction {
    use bitcoin::blockdata::opcodes;
    use bitcoin::blockdata::script::Builder;

    let op_return_script = Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(b"BRC20PROG")
        .into_script();

    let mut outputs = vec![TxOut {
        value: Amount::from_sat(1),
        script_pubkey: op_return_script,
    }];
    outputs.extend(additional_outputs);

    // Change output
    let change_addr = get_test_address(0);
    outputs.push(TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: change_addr.script_pubkey(),
    });

    Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![
            TxIn {
                previous_output: OutPoint::new(*reveal_txid, reveal_vout),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            },
            TxIn {
                previous_output: create_mock_outpoint(99),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            },
        ],
        output: outputs,
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
            "{:0>64x}",
            n
        )).unwrap(),
        vout: 0,
    }
}
