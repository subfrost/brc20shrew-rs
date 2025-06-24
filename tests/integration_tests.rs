use bitcoin::{Block, Transaction, Txid, OutPoint, TxIn, TxOut, Script, Witness};
use bitcoin::consensus::deserialize;
use bitcoin::hex::FromHex;
use shrewscriptions_rs::{
    indexer::{InscriptionIndexer, IndexError},
    inscription::{InscriptionId, InscriptionEntry, SatPoint},
    envelope::{parse_inscriptions_from_transaction, Envelope},
    tables::TABLES,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_initialization() {
        let mut indexer = InscriptionIndexer::new();
        assert_eq!(indexer.sequence_counter, 0);
        assert_eq!(indexer.blessed_counter, 0);
        assert_eq!(indexer.cursed_counter, -1);
        assert_eq!(indexer.jubilee_height, 824544);
    }

    #[test]
    fn test_indexer_state_persistence() {
        let mut indexer = InscriptionIndexer::new();
        
        // Modify state
        indexer.sequence_counter = 100;
        indexer.blessed_counter = 50;
        indexer.cursed_counter = -25;
        
        // Save state
        indexer.save_state().unwrap();
        
        // Create new indexer and load state
        let mut new_indexer = InscriptionIndexer::new();
        new_indexer.load_state().unwrap();
        
        assert_eq!(new_indexer.sequence_counter, 100);
        assert_eq!(new_indexer.blessed_counter, 50);
        assert_eq!(new_indexer.cursed_counter, -25);
    }

    #[test]
    fn test_inscription_id_serialization() {
        let txid = Txid::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        let id = InscriptionId::new(txid, 42);
        
        let bytes = id.to_bytes();
        assert_eq!(bytes.len(), 36);
        
        let restored = InscriptionId::from_bytes(&bytes).unwrap();
        assert_eq!(restored.txid, txid);
        assert_eq!(restored.index, 42);
    }

    #[test]
    fn test_satpoint_serialization() {
        let txid = Txid::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        let outpoint = OutPoint { txid, vout: 1 };
        let satpoint = SatPoint::new(outpoint, 12345);
        
        let bytes = satpoint.to_bytes();
        assert_eq!(bytes.len(), 44);
        
        let restored = SatPoint::from_bytes(&bytes).unwrap();
        assert_eq!(restored.outpoint.txid, txid);
        assert_eq!(restored.outpoint.vout, 1);
        assert_eq!(restored.offset, 12345);
    }

    #[test]
    fn test_inscription_entry_serialization() {
        let txid = Txid::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        let id = InscriptionId::new(txid, 0);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        let mut entry = InscriptionEntry::new(id, 1, 1, satpoint, 800000, 1000, 1640995200);
        entry.content_type = Some("text/plain".to_string());
        entry.content_length = Some(13);
        
        let bytes = entry.to_bytes();
        assert!(!bytes.is_empty());
        
        let restored = InscriptionEntry::from_bytes(&bytes).unwrap();
        assert_eq!(restored.id.txid, txid);
        assert_eq!(restored.number, 1);
        assert_eq!(restored.sequence, 1);
        assert_eq!(restored.height, 800000);
        assert_eq!(restored.content_type, Some("text/plain".to_string()));
        assert_eq!(restored.content_length, Some(13));
    }

    #[test]
    fn test_envelope_parsing_simple() {
        // Create a simple inscription transaction
        let script_bytes = create_inscription_script(b"text/plain", b"Hello, world!");
        let witness = Witness::from_slice(&[script_bytes]);
        
        let tx = Transaction {
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
        };

        let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
        assert_eq!(envelopes.len(), 1);
        
        let envelope = &envelopes[0];
        assert_eq!(envelope.input, 0);
        assert_eq!(envelope.payload.content_type(), Some("text/plain".to_string()));
        assert_eq!(envelope.payload.body, Some(b"Hello, world!".to_vec()));
        assert!(!envelope.payload.is_cursed());
    }

    #[test]
    fn test_envelope_parsing_cursed() {
        // Create an inscription with duplicate fields (cursed)
        let script_bytes = create_cursed_inscription_script();
        let witness = Witness::from_slice(&[script_bytes]);
        
        let tx = Transaction {
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
        };

        let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
        assert_eq!(envelopes.len(), 1);
        
        let envelope = &envelopes[0];
        assert!(envelope.payload.duplicate_field);
        assert!(envelope.payload.is_cursed());
    }

    #[test]
    fn test_block_indexing() {
        let mut indexer = InscriptionIndexer::new();
        
        // Create a test block with inscription
        let block = create_test_block_with_inscription();
        let height = 800000;
        
        let result = indexer.index_block(&block, height).unwrap();
        
        assert_eq!(result.height, height);
        assert_eq!(result.block_hash, block.block_hash());
        assert_eq!(result.transactions_processed, block.txs.len());
        
        // Verify block metadata was stored
        let height_bytes = height.to_le_bytes();
        let stored_hash = TABLES.HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get().unwrap();
        assert_eq!(stored_hash, block.block_hash().to_byte_array());
        
        let hash_bytes = block.block_hash().to_byte_array();
        let stored_height = TABLES.BLOCK_HASH_TO_HEIGHT.select(&hash_bytes).get().unwrap();
        assert_eq!(stored_height, height_bytes);
    }

    #[test]
    fn test_inscription_numbering() {
        let mut indexer = InscriptionIndexer::new();
        
        // Create blocks with inscriptions
        let block1 = create_test_block_with_inscription();
        let block2 = create_test_block_with_cursed_inscription();
        
        // Index first block (blessed inscription)
        let result1 = indexer.index_block(&block1, 800000).unwrap();
        assert_eq!(indexer.blessed_counter, 1);
        assert_eq!(indexer.cursed_counter, -1);
        
        // Index second block (cursed inscription)
        let result2 = indexer.index_block(&block2, 800001).unwrap();
        assert_eq!(indexer.blessed_counter, 1);
        assert_eq!(indexer.cursed_counter, -2);
    }

    #[test]
    fn test_jubilee_height_behavior() {
        let mut indexer = InscriptionIndexer::new();
        indexer.jubilee_height = 800000; // Set jubilee height for testing
        
        // Create cursed inscription before jubilee
        let block1 = create_test_block_with_cursed_inscription();
        indexer.index_block(&block1, 799999).unwrap();
        assert_eq!(indexer.cursed_counter, -2); // Should be cursed
        
        // Create cursed inscription after jubilee
        let block2 = create_test_block_with_cursed_inscription();
        indexer.index_block(&block2, 800000).unwrap();
        assert_eq!(indexer.blessed_counter, 1); // Should be blessed now
    }

    #[test]
    fn test_parent_child_relationships() {
        let mut indexer = InscriptionIndexer::new();
        
        // Create parent inscription
        let parent_block = create_test_block_with_inscription();
        indexer.index_block(&parent_block, 800000).unwrap();
        
        // Create child inscription referencing parent
        let parent_id = InscriptionId::new(parent_block.txs[1].txid(), 0);
        let child_block = create_test_block_with_child_inscription(&parent_id);
        indexer.index_block(&child_block, 800001).unwrap();
        
        // Verify parent-child relationship was stored
        let parent_id_bytes = parent_id.to_bytes();
        let parent_seq_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id_bytes).get().unwrap();
        
        let child_id = InscriptionId::new(child_block.txs[1].txid(), 0);
        let child_id_bytes = child_id.to_bytes();
        let child_seq_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&child_id_bytes).get().unwrap();
        
        // Check parent has child
        let children_list = TABLES.SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).get_list().unwrap();
        assert!(children_list.contains(&child_seq_bytes));
        
        // Check child has parent
        let parents_list = TABLES.SEQUENCE_TO_PARENTS.select(&child_seq_bytes).get_list().unwrap();
        assert!(parents_list.contains(&parent_seq_bytes));
    }

    #[test]
    fn test_content_storage() {
        let mut indexer = InscriptionIndexer::new();
        
        let block = create_test_block_with_inscription();
        indexer.index_block(&block, 800000).unwrap();
        
        // Verify content was stored
        let inscription_id = InscriptionId::new(block.txs[1].txid(), 0);
        let id_bytes = inscription_id.to_bytes();
        let sequence_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).get().unwrap();
        
        let stored_content = TABLES.INSCRIPTION_CONTENT.select(&sequence_bytes).get().unwrap();
        assert_eq!(stored_content, b"Hello, world!");
    }

    // Helper functions for creating test data

    fn create_inscription_script(content_type: &[u8], body: &[u8]) -> Vec<u8> {
        use bitcoin::opcodes::all::*;
        
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
        script.push(body.len() as u8);
        script.extend_from_slice(body);
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    fn create_cursed_inscription_script() -> Vec<u8> {
        use bitcoin::opcodes::all::*;
        
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Duplicate content-type fields (makes it cursed)
        script.push(1); // Push 1 byte
        script.push(1); // Content-type tag
        script.push(10);
        script.extend_from_slice(b"text/plain");
        
        script.push(1); // Push 1 byte
        script.push(1); // Content-type tag (duplicate!)
        script.push(9);
        script.extend_from_slice(b"text/html");
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        script.push(13);
        script.extend_from_slice(b"Hello, world!");
        
        script.push(OP_ENDIF.to_u8());
        script
    }

    fn create_test_block_with_inscription() -> Block {
        let script_bytes = create_inscription_script(b"text/plain", b"Hello, world!");
        let witness = Witness::from_slice(&[script_bytes]);
        
        let coinbase = Transaction {
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
        };

        let inscription_tx = Transaction {
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
        };

        Block {
            header: bitcoin::BlockHeader {
                version: bitcoin::block::Version::ONE,
                prev_blockhash: bitcoin::BlockHash::all_zeros(),
                merkle_root: bitcoin::TxMerkleNode::all_zeros(),
                time: 1640995200,
                bits: bitcoin::CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txs: vec![coinbase, inscription_tx],
        }
    }

    fn create_test_block_with_cursed_inscription() -> Block {
        let script_bytes = create_cursed_inscription_script();
        let witness = Witness::from_slice(&[script_bytes]);
        
        let coinbase = Transaction {
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
        };

        let inscription_tx = Transaction {
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
        };

        Block {
            header: bitcoin::BlockHeader {
                version: bitcoin::block::Version::ONE,
                prev_blockhash: bitcoin::BlockHash::all_zeros(),
                merkle_root: bitcoin::TxMerkleNode::all_zeros(),
                time: 1640995200,
                bits: bitcoin::CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txs: vec![coinbase, inscription_tx],
        }
    }

    fn create_test_block_with_child_inscription(parent_id: &InscriptionId) -> Block {
        use bitcoin::opcodes::all::*;
        
        let mut script = Vec::new();
        script.push(OP_FALSE.to_u8());
        script.push(OP_IF.to_u8());
        
        // Content-type field
        script.push(1);
        script.push(1);
        script.push(10);
        script.extend_from_slice(b"text/plain");
        
        // Parent field
        script.push(1);
        script.push(3); // Parent tag
        script.push(36); // Parent ID length
        script.extend_from_slice(&parent_id.to_bytes());
        
        // Body separator
        script.push(OP_0.to_u8());
        
        // Body
        script.push(12);
        script.extend_from_slice(b"Child content");
        
        script.push(OP_ENDIF.to_u8());
        
        let witness = Witness::from_slice(&[script]);
        
        let coinbase = Transaction {
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
        };

        let inscription_tx = Transaction {
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
        };

        Block {
            header: bitcoin::BlockHeader {
                version: bitcoin::block::Version::ONE,
                prev_blockhash: bitcoin::BlockHash::all_zeros(),
                merkle_root: bitcoin::TxMerkleNode::all_zeros(),
                time: 1640995200,
                bits: bitcoin::CompactTarget::from_consensus(0x1d00ffff),
                nonce: 0,
            },
            txs: vec![coinbase, inscription_tx],
        }
    }
}