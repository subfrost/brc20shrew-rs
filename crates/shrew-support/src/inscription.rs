use bitcoin::{OutPoint, Txid};
use bitcoin_hashes::Hash;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Unique identifier for an inscription
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InscriptionId {
    pub txid: Txid,
    pub index: u32,
}

impl InscriptionId {
    pub fn new(txid: Txid, index: u32) -> Self {
        Self { txid, index }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(36);
        bytes.extend_from_slice(self.txid.as_byte_array());
        bytes.extend_from_slice(&self.index.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 36 {
            return Err("Invalid InscriptionId bytes length".to_string());
        }
        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&bytes[0..32]);
        let txid = Txid::from_byte_array(txid_bytes);
        let mut index_bytes = [0u8; 4];
        index_bytes.copy_from_slice(&bytes[32..36]);
        let index = u32::from_le_bytes(index_bytes);
        Ok(Self { txid, index })
    }
}

impl FromStr for InscriptionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(i_pos) = s.find('i') {
            let txid_str = &s[..i_pos];
            let index_str = &s[i_pos + 1..];
            let txid = Txid::from_str(txid_str)
                .map_err(|e| format!("Invalid txid: {}", e))?;
            let index = index_str.parse::<u32>()
                .map_err(|e| format!("Invalid index: {}", e))?;
            Ok(Self { txid, index })
        } else {
            Err("Invalid inscription ID format, expected 'txid'i'index'".to_string())
        }
    }
}

impl fmt::Display for InscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}i{}", self.txid, self.index)
    }
}

/// Location of a satoshi within a UTXO
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SatPoint {
    pub outpoint: OutPoint,
    pub offset: u64,
}

impl SatPoint {
    pub fn new(outpoint: OutPoint, offset: u64) -> Self {
        Self { outpoint, offset }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(44);
        bytes.extend_from_slice(self.outpoint.txid.as_byte_array());
        bytes.extend_from_slice(&self.outpoint.vout.to_le_bytes());
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 44 {
            return Err("Invalid SatPoint bytes length".to_string());
        }
        let mut txid_bytes = [0u8; 32];
        txid_bytes.copy_from_slice(&bytes[0..32]);
        let txid = Txid::from_byte_array(txid_bytes);
        let mut vout_bytes = [0u8; 4];
        vout_bytes.copy_from_slice(&bytes[32..36]);
        let vout = u32::from_le_bytes(vout_bytes);
        let mut offset_bytes = [0u8; 8];
        offset_bytes.copy_from_slice(&bytes[36..44]);
        let offset = u64::from_le_bytes(offset_bytes);
        Ok(Self {
            outpoint: OutPoint { txid, vout },
            offset,
        })
    }
}

impl fmt::Display for SatPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.outpoint.txid, self.outpoint.vout, self.offset)
    }
}

/// Inscription entry stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InscriptionEntry {
    pub id: InscriptionId,
    pub number: i32,
    pub sequence: u32,
    pub sat: Option<u64>,
    pub satpoint: SatPoint,
    pub height: u32,
    pub fee: u64,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub timestamp: u32,
    pub genesis_fee: u64,
    pub genesis_height: u32,
    pub parent: Option<InscriptionId>,
    pub delegate: Option<InscriptionId>,
    pub metaprotocol: Option<String>,
    pub pointer: Option<u64>,
    pub charms: u16,
}

impl InscriptionEntry {
    pub fn new(
        id: InscriptionId,
        number: i32,
        sequence: u32,
        satpoint: SatPoint,
        height: u32,
        fee: u64,
        timestamp: u32,
    ) -> Self {
        Self {
            id,
            number,
            sequence,
            sat: None,
            satpoint,
            height,
            fee,
            content_type: None,
            content_length: None,
            timestamp,
            genesis_fee: fee,
            genesis_height: height,
            parent: None,
            delegate: None,
            metaprotocol: None,
            pointer: None,
            charms: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| e.to_string())
    }

    pub fn is_cursed(&self) -> bool {
        self.number < 0
    }

    pub fn is_blessed(&self) -> bool {
        self.number >= 0
    }

    pub fn has_charm(&self, charm: Charm) -> bool {
        (self.charms & (1 << charm as u16)) != 0
    }

    pub fn set_charm(&mut self, charm: Charm) {
        self.charms |= 1 << charm as u16;
    }

    pub fn unset_charm(&mut self, charm: Charm) {
        self.charms &= !(1 << charm as u16);
    }
}

/// Inscription charms (special properties)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum Charm {
    Coin = 0,
    Cursed = 1,
    Epic = 2,
    Legendary = 3,
    Lost = 4,
    Nineball = 5,
    Rare = 6,
    Reinscription = 7,
    Unbound = 8,
    Uncommon = 9,
    Vindicated = 10,
}

impl Charm {
    pub fn all() -> &'static [Charm] {
        &[
            Charm::Coin, Charm::Cursed, Charm::Epic, Charm::Legendary,
            Charm::Lost, Charm::Nineball, Charm::Rare, Charm::Reinscription,
            Charm::Unbound, Charm::Uncommon, Charm::Vindicated,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Charm::Coin => "coin",
            Charm::Cursed => "cursed",
            Charm::Epic => "epic",
            Charm::Legendary => "legendary",
            Charm::Lost => "lost",
            Charm::Nineball => "nineball",
            Charm::Rare => "rare",
            Charm::Reinscription => "reinscription",
            Charm::Unbound => "unbound",
            Charm::Uncommon => "uncommon",
            Charm::Vindicated => "vindicated",
        }
    }
}

impl fmt::Display for Charm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Rarity of a satoshi
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl Rarity {
    pub fn from_sat(sat: u64) -> Self {
        if sat == 0 {
            return Rarity::Mythic;
        }
        if sat % (210_000 * 32 * 50_000_000) == 0 {
            return Rarity::Legendary;
        }
        if sat % (210_000 * 50_000_000) == 0 {
            return Rarity::Epic;
        }
        if sat % (2016 * 50_000_000) == 0 {
            return Rarity::Rare;
        }
        if sat % 50_000_000 == 0 {
            return Rarity::Uncommon;
        }
        Rarity::Common
    }

    pub fn name(&self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Uncommon => "uncommon",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
            Rarity::Legendary => "legendary",
            Rarity::Mythic => "mythic",
        }
    }
}

impl fmt::Display for Rarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Media type for inscription content
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Media {
    Audio, Code, Font, Iframe, Image, Markdown, Model, Pdf, Text, Unknown, Video,
}

impl Media {
    pub fn from_content_type(content_type: &str) -> Self {
        match content_type {
            ct if ct.starts_with("audio/") => Media::Audio,
            ct if ct.starts_with("font/") => Media::Font,
            ct if ct.starts_with("image/") => Media::Image,
            ct if ct.starts_with("model/") => Media::Model,
            ct if ct.starts_with("text/") => match ct {
                "text/html" => Media::Iframe,
                "text/markdown" => Media::Markdown,
                "text/plain" => Media::Text,
                _ if ct.contains("javascript") || ct.contains("json") => Media::Code,
                _ => Media::Text,
            },
            ct if ct.starts_with("video/") => Media::Video,
            "application/pdf" => Media::Pdf,
            ct if ct.contains("json") || ct.contains("javascript") => Media::Code,
            _ => Media::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Media::Audio => "audio", Media::Code => "code", Media::Font => "font",
            Media::Iframe => "iframe", Media::Image => "image", Media::Markdown => "markdown",
            Media::Model => "model", Media::Pdf => "pdf", Media::Text => "text",
            Media::Unknown => "unknown", Media::Video => "video",
        }
    }
}

impl fmt::Display for Media {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test as test;
    use bitcoin::Txid;
    use std::str::FromStr;

    fn test_txid() -> Txid {
        Txid::from_str("ababababababababababababababababababababababababababababababab01").unwrap()
    }

    #[test]
    fn test_inscription_id_new() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 5);
        assert_eq!(id.txid, txid);
        assert_eq!(id.index, 5);
    }

    #[test]
    fn test_inscription_id_to_string() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 3);
        let s = id.to_string();
        assert!(s.ends_with("i3"));
        assert!(s.contains(&txid.to_string()));
    }

    #[test]
    fn test_inscription_id_roundtrip() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 42);
        let bytes = id.to_bytes();
        assert_eq!(bytes.len(), 36);
        let id2 = InscriptionId::from_bytes(&bytes).unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_inscription_id_from_bytes_invalid_length() {
        assert!(InscriptionId::from_bytes(&[0u8; 10]).is_err());
        assert!(InscriptionId::from_bytes(&[0u8; 37]).is_err());
    }

    #[test]
    fn test_inscription_id_from_str() {
        let txid = test_txid();
        let id_str = format!("{}i7", txid);
        let id: InscriptionId = id_str.parse().unwrap();
        assert_eq!(id.txid, txid);
        assert_eq!(id.index, 7);
    }

    #[test]
    fn test_inscription_id_from_str_invalid() {
        assert!(InscriptionId::from_str("invalid").is_err());
        assert!(InscriptionId::from_str("abc").is_err());
    }

    #[test]
    fn test_satpoint_new() {
        let txid = test_txid();
        let outpoint = bitcoin::OutPoint { txid, vout: 2 };
        let sp = SatPoint::new(outpoint, 500);
        assert_eq!(sp.outpoint.txid, txid);
        assert_eq!(sp.outpoint.vout, 2);
        assert_eq!(sp.offset, 500);
    }

    #[test]
    fn test_satpoint_to_bytes_roundtrip() {
        let txid = test_txid();
        let outpoint = bitcoin::OutPoint { txid, vout: 1 };
        let sp = SatPoint::new(outpoint, 12345);
        let bytes = sp.to_bytes();
        assert_eq!(bytes.len(), 44);
        let sp2 = SatPoint::from_bytes(&bytes).unwrap();
        assert_eq!(sp, sp2);
    }

    #[test]
    fn test_satpoint_from_bytes_invalid() {
        assert!(SatPoint::from_bytes(&[0u8; 10]).is_err());
    }

    #[test]
    fn test_satpoint_display() {
        let txid = test_txid();
        let outpoint = bitcoin::OutPoint { txid, vout: 3 };
        let sp = SatPoint::new(outpoint, 99);
        let s = sp.to_string();
        assert!(s.contains(":3:99"));
    }

    #[test]
    fn test_inscription_entry_new() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 0);
        let outpoint = bitcoin::OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        let entry = InscriptionEntry::new(id.clone(), 1, 1, satpoint, 100, 500, 1640995200);
        assert_eq!(entry.id, id);
        assert_eq!(entry.number, 1);
        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.height, 100);
        assert_eq!(entry.fee, 500);
        assert_eq!(entry.genesis_fee, 500);
        assert_eq!(entry.genesis_height, 100);
        assert!(entry.sat.is_none());
        assert!(entry.content_type.is_none());
        assert!(entry.parent.is_none());
        assert!(entry.delegate.is_none());
        assert_eq!(entry.charms, 0);
    }

    #[test]
    fn test_inscription_entry_serialization() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 0);
        let outpoint = bitcoin::OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        let mut entry = InscriptionEntry::new(id, 1, 1, satpoint, 100, 500, 1640995200);
        entry.content_type = Some("text/plain".to_string());
        entry.content_length = Some(28);
        let bytes = entry.to_bytes();
        assert!(!bytes.is_empty());
        let entry2 = InscriptionEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry.id, entry2.id);
        assert_eq!(entry.number, entry2.number);
        assert_eq!(entry.content_type, entry2.content_type);
        assert_eq!(entry.content_length, entry2.content_length);
    }

    #[test]
    fn test_inscription_entry_cursed_blessed() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 0);
        let outpoint = bitcoin::OutPoint { txid, vout: 0 };
        let sp = SatPoint::new(outpoint, 0);

        let blessed = InscriptionEntry::new(id.clone(), 1, 1, sp.clone(), 100, 0, 0);
        assert!(blessed.is_blessed());
        assert!(!blessed.is_cursed());

        let cursed = InscriptionEntry::new(id, -1, 2, sp, 100, 0, 0);
        assert!(cursed.is_cursed());
        assert!(!cursed.is_blessed());
    }

    #[test]
    fn test_charm_set_and_check() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 0);
        let outpoint = bitcoin::OutPoint { txid, vout: 0 };
        let sp = SatPoint::new(outpoint, 0);
        let mut entry = InscriptionEntry::new(id, 0, 0, sp, 0, 0, 0);

        assert!(!entry.has_charm(Charm::Cursed));
        entry.set_charm(Charm::Cursed);
        assert!(entry.has_charm(Charm::Cursed));
        assert!(!entry.has_charm(Charm::Uncommon));
    }

    #[test]
    fn test_charm_multiple() {
        let txid = test_txid();
        let id = InscriptionId::new(txid, 0);
        let outpoint = bitcoin::OutPoint { txid, vout: 0 };
        let sp = SatPoint::new(outpoint, 0);
        let mut entry = InscriptionEntry::new(id, 0, 0, sp, 0, 0, 0);

        entry.set_charm(Charm::Cursed);
        entry.set_charm(Charm::Uncommon);
        entry.set_charm(Charm::Rare);
        assert!(entry.has_charm(Charm::Cursed));
        assert!(entry.has_charm(Charm::Uncommon));
        assert!(entry.has_charm(Charm::Rare));
        assert!(!entry.has_charm(Charm::Epic));

        entry.unset_charm(Charm::Cursed);
        assert!(!entry.has_charm(Charm::Cursed));
        assert!(entry.has_charm(Charm::Uncommon));
    }

    #[test]
    fn test_charm_all() {
        let all = Charm::all();
        assert_eq!(all.len(), 11);
    }

    #[test]
    fn test_rarity_from_sat() {
        assert_eq!(Rarity::from_sat(0), Rarity::Mythic);
        assert_eq!(Rarity::from_sat(50_000_000), Rarity::Uncommon);
        assert_eq!(Rarity::from_sat(1), Rarity::Common);
        assert_eq!(Rarity::from_sat(2016 * 50_000_000), Rarity::Rare);
        assert_eq!(Rarity::from_sat(210_000 * 50_000_000), Rarity::Epic);
    }

    #[test]
    fn test_rarity_ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
        assert!(Rarity::Legendary < Rarity::Mythic);
    }

    #[test]
    fn test_rarity_name() {
        assert_eq!(Rarity::Common.name(), "common");
        assert_eq!(Rarity::Mythic.name(), "mythic");
    }

    #[test]
    fn test_media_from_content_type() {
        assert_eq!(Media::from_content_type("text/plain"), Media::Text);
        assert_eq!(Media::from_content_type("text/html"), Media::Iframe);
        assert_eq!(Media::from_content_type("text/markdown"), Media::Markdown);
        assert_eq!(Media::from_content_type("image/png"), Media::Image);
        assert_eq!(Media::from_content_type("audio/mpeg"), Media::Audio);
        assert_eq!(Media::from_content_type("video/mp4"), Media::Video);
        assert_eq!(Media::from_content_type("application/pdf"), Media::Pdf);
        assert_eq!(Media::from_content_type("application/json"), Media::Code);
        assert_eq!(Media::from_content_type("font/woff2"), Media::Font);
        assert_eq!(Media::from_content_type("model/gltf"), Media::Model);
        assert_eq!(Media::from_content_type("application/octet-stream"), Media::Unknown);
    }
}
