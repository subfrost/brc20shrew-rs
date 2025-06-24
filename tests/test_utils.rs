use bitcoin::{Block, Transaction, TxIn, TxOut, OutPoint, Script, Witness, BlockHeader, Txid};
use bitcoin::opcodes::all::*;
use shrewscriptions_rs::inscription::InscriptionId;

/// Test utilities for creating Bitcoin transactions and blocks with inscriptions
pub struct TestUtils;

impl TestUtils {
    /// Create a simple inscription script with content type and body
    pub fn create_inscription_script(content_type: &[u8], body: &[u8]) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1); // Push 1 byte
        script.push(1); // Content-type tag
        script.push(content_type.len() as u8);
        script.extend_from_slice(content_type);
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        if !body.is_empty() {
            script.push(body.len() as u8);
            script.extend_from_slice(body);
        }
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create an inscription script with metadata
    pub fn create_inscription_with_metadata(
        content_type: &[u8],
        body: &[u8],
        metadata: &[u8],
    ) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(content_type.len() as u8);
        script.extend_from_slice(content_type);
        
        // Metadata field
        script.push(1);
        script.push(5); // Metadata tag
        script.push(metadata.len() as u8);
        script.extend_from_slice(metadata);
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        if !body.is_empty() {
            script.push(body.len() as u8);
            script.extend_from_slice(body);
        }
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create an inscription script with parent reference
    pub fn create_child_inscription_script(
        content_type: &[u8],
        body: &[u8],
        parent_id: &InscriptionId,
    ) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(content_type.len() as u8);
        script.extend_from_slice(content_type);
        
        // Parent field
        script.push(1);
        script.push(3); // Parent tag
        script.push(36); // Parent ID length (32 + 4 bytes)
        script.extend_from_slice(&parent_id.to_bytes());
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        if !body.is_empty() {
            script.push(body.len() as u8);
            script.extend_from_slice(body);
        }
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create an inscription script with delegation
    pub fn create_delegated_inscription_script(
        content_type: &[u8],
        delegate_id: &InscriptionId,
    ) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(content_type.len() as u8);
        script.extend_from_slice(content_type);
        
        // Delegate field
        script.push(1);
        script.push(11); // Delegate tag
        script.push(36); // Delegate ID length
        script.extend_from_slice(&delegate_id.to_bytes());
        
        // No body for delegated inscriptions
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create a cursed inscription script (with duplicate fields)
    pub fn create_cursed_inscription_script() -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // First content-type field
        script.push(1);
        script.push(1);
        script.push(10);
        script.extend_from_slice(b"text/plain");
        
        // Duplicate content-type field (makes it cursed)
        script.push(1);
        script.push(1);
        script.push(9);
        script.extend_from_slice(b"text/html");
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        script.push(13);
        script.extend_from_slice(b"Cursed content");
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create an inscription script with unrecognized even field (cursed)
    pub fn create_unrecognized_even_field_script() -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(10);
        script.extend_from_slice(b"text/plain");
        
        // Unrecognized even field (makes it cursed)
        script.push(1);
        script.push(100); // Even tag that's not recognized
        script.push(4);
        script.extend_from_slice(b"test");
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        script.push(13);
        script.extend_from_slice(b"Cursed content");
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create an inscription script with pointer
    pub fn create_inscription_with_pointer(
        content_type: &[u8],
        body: &[u8],
        pointer: u64,
    ) -> Vec<u8> {
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(content_type.len() as u8);
        script.extend_from_slice(content_type);
        
        // Pointer field
        script.push(1);
        script.push(2); // Pointer tag
        script.push(8); // 8 bytes for u64
        script.extend_from_slice(&pointer.to_le_bytes());
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        if !body.is_empty() {
            script.push(body.len() as u8);
            script.extend_from_slice(body);
        }
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    /// Create a transaction with inscription in witness
    pub fn create_inscription_transaction(script_bytes: Vec<u8>) -> Transaction {
        let witness = Witness::from_slice(&[script_bytes]);
        
        Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: Script::new().into(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness,
            }],
            output: vec![TxOut {
                value: bitcoin::Amount::from_sat(546),
                script_pubkey: Script::new().into(),
            }],
        }
    }

    /// Create a coinbase transaction
    pub fn create_coinbase_transaction() -> Transaction {
        Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: Script::new().into(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: bitcoin::Amount::from_sat(5000000000),
                script_pubkey: Script::new().into(),
            }],
        }
    }

    /// Create a block with given transactions
    pub fn create_block(txs: Vec<Transaction>, time: u32) -> Block {
        Block {
            header: BlockHeader {
                version: bitcoin::block::Version::ONE,
                prev_blockhash: bitcoin::BlockHash::all_zeros(),
                merkle_root: bitcoin::TxMerkleNode::all_zeros(),
                time,
                bits: bitcoin::CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txs,
        }
    }

    /// Create a test block with a simple inscription
    pub fn create_test_block_with_inscription() -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_inscription_script(b"text/plain", b"Hello, world!");
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Create a test block with multiple inscriptions
    pub fn create_test_block_with_multiple_inscriptions() -> Block {
        let coinbase = Self::create_coinbase_transaction();
        
        let script1 = Self::create_inscription_script(b"text/plain", b"First inscription");
        let tx1 = Self::create_inscription_transaction(script1);
        
        let script2 = Self::create_inscription_script(b"image/png", b"PNG data here");
        let tx2 = Self::create_inscription_transaction(script2);
        
        let script3 = Self::create_inscription_script(b"application/json", b"{\"test\": true}");
        let tx3 = Self::create_inscription_transaction(script3);
        
        Self::create_block(vec![coinbase, tx1, tx2, tx3], 1640995200)
    }

    /// Create a test block with cursed inscription
    pub fn create_test_block_with_cursed_inscription() -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_cursed_inscription_script();
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Create a test block with parent-child inscriptions
    pub fn create_test_block_with_child_inscription(parent_id: &InscriptionId) -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_child_inscription_script(
            b"text/plain",
            b"Child content",
            parent_id,
        );
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Create a test block with delegated inscription
    pub fn create_test_block_with_delegated_inscription(delegate_id: &InscriptionId) -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_delegated_inscription_script(b"text/plain", delegate_id);
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Create a test block with inscription containing metadata
    pub fn create_test_block_with_metadata() -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_inscription_with_metadata(
            b"text/plain",
            b"Content with metadata",
            b"{\"name\": \"Test NFT\", \"description\": \"A test inscription\"}",
        );
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Create a test block with inscription containing pointer
    pub fn create_test_block_with_pointer() -> Block {
        let coinbase = Self::create_coinbase_transaction();
        let script_bytes = Self::create_inscription_with_pointer(
            b"text/plain",
            b"Content with pointer",
            12345,
        );
        let inscription_tx = Self::create_inscription_transaction(script_bytes);
        
        Self::create_block(vec![coinbase, inscription_tx], 1640995200)
    }

    /// Generate a random txid for testing
    pub fn random_txid() -> Txid {
        use bitcoin::hashes::{Hash, sha256d};
        let random_bytes: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
        ];
        Txid::from_byte_array(random_bytes)
    }

    /// Create a test inscription ID
    pub fn test_inscription_id() -> InscriptionId {
        InscriptionId::new(Self::random_txid(), 0)
    }

    /// Create multiple test inscription IDs
    pub fn test_inscription_ids(count: usize) -> Vec<InscriptionId> {
        (0..count)
            .map(|i| InscriptionId::new(Self::random_txid(), i as u32))
            .collect()
    }
}

/// Common test assertions
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that an inscription entry has expected basic properties
    pub fn assert_inscription_entry_basic(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        expected_number: i32,
        expected_height: u32,
    ) {
        assert_eq!(entry.number, expected_number);
        assert_eq!(entry.height, expected_height);
        assert_eq!(entry.genesis_height, expected_height);
        assert!(entry.sequence > 0);
    }

    /// Assert that an inscription is blessed (positive number)
    pub fn assert_inscription_blessed(entry: &shrewscriptions_rs::inscription::InscriptionEntry) {
        assert!(entry.is_blessed());
        assert!(!entry.is_cursed());
        assert!(entry.number >= 0);
    }

    /// Assert that an inscription is cursed (negative number)
    pub fn assert_inscription_cursed(entry: &shrewscriptions_rs::inscription::InscriptionEntry) {
        assert!(entry.is_cursed());
        assert!(!entry.is_blessed());
        assert!(entry.number < 0);
    }

    /// Assert that an inscription has specific content properties
    pub fn assert_inscription_content(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        expected_content_type: Option<&str>,
        expected_content_length: Option<u64>,
    ) {
        assert_eq!(entry.content_type.as_deref(), expected_content_type);
        assert_eq!(entry.content_length, expected_content_length);
    }

    /// Assert that an inscription has a specific parent
    pub fn assert_inscription_parent(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        expected_parent: Option<&InscriptionId>,
    ) {
        assert_eq!(entry.parent.as_ref(), expected_parent);
    }

    /// Assert that an inscription has a specific delegate
    pub fn assert_inscription_delegate(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        expected_delegate: Option<&InscriptionId>,
    ) {
        assert_eq!(entry.delegate.as_ref(), expected_delegate);
    }

    /// Assert that an inscription has specific charm
    pub fn assert_inscription_has_charm(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        charm: shrewscriptions_rs::inscription::Charm,
    ) {
        assert!(entry.has_charm(charm), "Inscription should have charm: {}", charm);
    }

    /// Assert that an inscription does not have specific charm
    pub fn assert_inscription_lacks_charm(
        entry: &shrewscriptions_rs::inscription::InscriptionEntry,
        charm: shrewscriptions_rs::inscription::Charm,
    ) {
        assert!(!entry.has_charm(charm), "Inscription should not have charm: {}", charm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_inscription_script() {
        let script = TestUtils::create_inscription_script(b"text/plain", b"Hello");
        assert!(!script.is_empty());
        assert_eq!(script[0], OP_FALSE.to_u8());
        assert_eq!(script[1], OP_IF.to_u8());
    }

    #[test]
    fn test_create_inscription_transaction() {
        let script = TestUtils::create_inscription_script(b"text/plain", b"Hello");
        let tx = TestUtils::create_inscription_transaction(script);
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 1);
        assert!(!tx.input[0].witness.is_empty());
    }

    #[test]
    fn test_create_test_block() {
        let block = TestUtils::create_test_block_with_inscription();
        assert_eq!(block.txs.len(), 2); // coinbase + inscription
        assert_eq!(block.header.time, 1640995200);
    }

    #[test]
    fn test_random_txid() {
        let txid1 = TestUtils::random_txid();
        let txid2 = TestUtils::random_txid();
        // Note: These will be the same since we use fixed bytes, but in real usage would be random
        assert_eq!(txid1, txid2);
    }

    #[test]
    fn test_test_inscription_ids() {
        let ids = TestUtils::test_inscription_ids(3);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].index, 0);
        assert_eq!(ids[1].index, 1);
        assert_eq!(ids[2].index, 2);
    }
}