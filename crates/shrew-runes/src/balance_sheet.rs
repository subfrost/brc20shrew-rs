use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

/// A RuneId represented as (block_height, tx_index)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuneId {
    pub block: u64,
    pub tx: u32,
}

impl RuneId {
    pub fn new(block: u64, tx: u32) -> Self {
        Self { block, tx }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self.block.to_le_bytes());
        bytes.extend_from_slice(&self.tx.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 { return None; }
        let block = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let tx = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
        Some(Self { block, tx })
    }
}

/// Per-outpoint rune balance tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub balances: BTreeMap<RuneId, u128>,
}

impl BalanceSheet {
    pub fn new() -> Self {
        Self { balances: BTreeMap::new() }
    }

    pub fn get(&self, rune_id: &RuneId) -> u128 {
        self.balances.get(rune_id).copied().unwrap_or(0)
    }

    pub fn set(&mut self, rune_id: RuneId, amount: u128) {
        if amount == 0 {
            self.balances.remove(&rune_id);
        } else {
            self.balances.insert(rune_id, amount);
        }
    }

    pub fn credit(&mut self, rune_id: RuneId, amount: u128) {
        let current = self.get(&rune_id);
        self.set(rune_id, current.saturating_add(amount));
    }

    pub fn debit(&mut self, rune_id: RuneId, amount: u128) -> bool {
        let current = self.get(&rune_id);
        if current < amount { return false; }
        self.set(rune_id, current - amount);
        true
    }

    pub fn is_empty(&self) -> bool {
        self.balances.is_empty()
    }

    /// Merge all balances from another sheet into this one
    pub fn merge(&mut self, other: &BalanceSheet) {
        for (rune_id, amount) in &other.balances {
            self.credit(*rune_id, *amount);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn test_balance_sheet_credit_debit() {
        let mut sheet = BalanceSheet::new();
        let rune = RuneId::new(840000, 0);
        sheet.credit(rune, 100);
        assert_eq!(sheet.get(&rune), 100);
        assert!(sheet.debit(rune, 50));
        assert_eq!(sheet.get(&rune), 50);
        assert!(!sheet.debit(rune, 100));
        assert_eq!(sheet.get(&rune), 50);
    }

    #[test]
    fn test_balance_sheet_merge() {
        let mut a = BalanceSheet::new();
        let mut b = BalanceSheet::new();
        let rune = RuneId::new(840000, 0);
        a.credit(rune, 50);
        b.credit(rune, 30);
        a.merge(&b);
        assert_eq!(a.get(&rune), 80);
    }
}
