use shrew_ord::tables::{GLOBAL_SEQUENCE_COUNTER, BLESSED_INSCRIPTION_COUNTER, CURSED_INSCRIPTION_COUNTER};
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin::{Address, Network};
use std::sync::Arc;

/// Clear metashrew state and initialize for testing.
/// MUST be called at the start of every test to ensure clean state.
pub fn clear() {
    metashrew_core::clear();
    let mut counter = GLOBAL_SEQUENCE_COUNTER.select(&vec![]);
    counter.set(Arc::new(vec![]));
    let mut blessed = BLESSED_INSCRIPTION_COUNTER.select(&vec![]);
    blessed.set(Arc::new(vec![]));
    let mut cursed = CURSED_INSCRIPTION_COUNTER.select(&vec![]);
    cursed.set(Arc::new(vec![]));
    configure_network();
}

/// Configure network parameters for testing (regtest)
pub fn configure_network() {
    // regtest network — address helpers use Network::Regtest directly
}

/// Get a deterministic test address for the regtest network
pub fn get_test_address(index: u8) -> Address<bitcoin::address::NetworkChecked> {
    use bitcoin::key::Secp256k1;
    use bitcoin::secp256k1::SecretKey;
    use bitcoin::PrivateKey;
    use bitcoin::PublicKey;

    let secp = Secp256k1::new();
    let mut key_data = [1u8; 32];
    key_data[0] = index;
    let secret_key = SecretKey::from_slice(&key_data).unwrap();
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);

    Address::p2wpkh(&public_key, Network::Regtest).unwrap()
}
