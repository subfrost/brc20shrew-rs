// Chadson v69.0.0: This file contains utility functions for the shrewscriptions indexer.

use bitcoin::{Address, Network, TxOut};

/// Extract address from a transaction output
pub fn get_address_from_txout(tx_out: &TxOut, network: Network) -> Option<Address> {
    Address::from_script(&tx_out.script_pubkey, network).ok()
}