use bitcoin::{Block, Transaction, OutPoint, Txid};
use crate::blocks::{create_coinbase_transaction, create_block_with_txs};
use crate::transactions::{
    create_inscription_transaction, create_inscription_transaction_to_address,
    create_transfer_transaction_to_address,
};
use bitcoin::Address;
use bitcoin::address::NetworkChecked;

/// Create a BRC20 JSON payload
pub fn create_brc20_json(op: &str, ticker: &str, fields: &[(&str, &str)]) -> Vec<u8> {
    let mut json = format!(r#"{{ "p": "brc-20", "op": "{}", "tick": "{}""#, op, ticker);
    for (key, value) in fields {
        json.push_str(&format!(r#", "{}": "{}""#, key, value));
    }
    json.push_str(" }");
    json.into_bytes()
}

/// Create a block with a BRC20 deploy inscription
pub fn create_brc20_deploy_block(ticker: &str, max: &str, lim: &str) -> (Block, Transaction) {
    let content = create_brc20_json("deploy", ticker, &[("max", max), ("lim", lim)]);
    let tx = create_inscription_transaction(&content, "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(0), tx.clone()]);
    (block, tx)
}

/// Create a block with a BRC20 mint inscription
pub fn create_brc20_mint_block(
    ticker: &str,
    amount: &str,
    to_address: &Address<NetworkChecked>,
    commit_txid: &Txid,
) -> (Block, Transaction) {
    let content = create_brc20_json("mint", ticker, &[("amt", amount)]);
    let tx = create_inscription_transaction_to_address(
        &content, "text/plain",
        Some(OutPoint::new(*commit_txid, 0)),
        to_address,
    );
    let block = create_block_with_txs(vec![create_coinbase_transaction(1), tx.clone()]);
    (block, tx)
}

/// Create a block with a BRC20 transfer inscribe operation
pub fn create_brc20_transfer_inscribe_block(
    ticker: &str,
    amount: &str,
    from_address: &Address<NetworkChecked>,
    commit_txid: &Txid,
) -> (Block, Transaction) {
    let content = create_brc20_json("transfer", ticker, &[("amt", amount)]);
    let tx = create_inscription_transaction_to_address(
        &content, "text/plain",
        Some(OutPoint::new(*commit_txid, 1)),
        from_address,
    );
    let block = create_block_with_txs(vec![create_coinbase_transaction(2), tx.clone()]);
    (block, tx)
}

/// Create a block with a BRC20 transfer claim (spending the transfer inscription)
pub fn create_brc20_transfer_claim_block(
    inscribe_tx: &Transaction,
    to_address: &Address<NetworkChecked>,
) -> (Block, Transaction) {
    let prev_out = OutPoint::new(inscribe_tx.compute_txid(), 0);
    let tx = create_transfer_transaction_to_address(prev_out, to_address);
    let block = create_block_with_txs(vec![create_coinbase_transaction(3), tx.clone()]);
    (block, tx)
}
