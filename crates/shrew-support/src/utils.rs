use bitcoin::{Address, Network, TxOut};

/// Extract address from a transaction output
pub fn get_address_from_txout(tx_out: &TxOut, network: Network) -> Option<Address> {
    Address::from_script(&tx_out.script_pubkey, network).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test as test;
    use bitcoin::{TxOut, ScriptBuf, Network};

    #[test]
    fn test_get_address_from_txout_empty_script() {
        let txout = TxOut { value: 0, script_pubkey: ScriptBuf::new() };
        assert!(get_address_from_txout(&txout, Network::Regtest).is_none());
    }

    #[test]
    fn test_get_address_from_txout_op_return() {
        let script = ScriptBuf::new_op_return(&[0u8; 20]);
        let txout = TxOut { value: 0, script_pubkey: script };
        assert!(get_address_from_txout(&txout, Network::Regtest).is_none());
    }

    #[test]
    fn test_get_address_from_txout_p2wpkh() {
        use bitcoin::key::Secp256k1;
        use bitcoin::secp256k1::SecretKey;
        use bitcoin::PrivateKey;
        use bitcoin::PublicKey;

        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let private_key = PrivateKey::new(secret_key, Network::Regtest);
        let public_key = PublicKey::from_private_key(&secp, &private_key);
        let address = Address::p2wpkh(&public_key, Network::Regtest).unwrap();
        let txout = TxOut { value: 10000, script_pubkey: address.script_pubkey() };
        let result = get_address_from_txout(&txout, Network::Regtest);
        assert!(result.is_some());
    }
}
