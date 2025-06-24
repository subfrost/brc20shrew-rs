use bitcoin::{OutPoint, Txid};
use serde::{Deserialize, Serialize};
use std::fmt;

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
        bytes.extend_from_slice(&self.txid.to_byte_array());
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
        bytes.extend_from_slice(&self.outpoint.txid.to_byte_array());
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
            Charm::Coin,
            Charm::Cursed,
            Charm::Epic,
            Charm::Legendary,
            Charm::Lost,
            Charm::Nineball,
            Charm::Rare,
            Charm::Reinscription,
            Charm::Unbound,
            Charm::Uncommon,
            Charm::Vindicated,
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

    pub fn icon(&self) -> char {
        match self {
            Charm::Coin => '🪙',
            Charm::Cursed => '👹',
            Charm::Epic => '🪻',
            Charm::Legendary => '🌝',
            Charm::Lost => '🤔',
            Charm::Nineball => '9',
            Charm::Rare => '🧿',
            Charm::Reinscription => '♻',
            Charm::Unbound => '🔓',
            Charm::Uncommon => '🔥',
            Charm::Vindicated => '❤',
        }
    }
}

impl fmt::Display for Charm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Rarity of a satoshi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

        // Legendary: first sat of each cycle (every 6 halvings)
        if sat % (210_000 * 32 * 50_000_000) == 0 {
            return Rarity::Legendary;
        }

        // Epic: first sat of each halving epoch
        if sat % (210_000 * 50_000_000) == 0 {
            return Rarity::Epic;
        }

        // Rare: first sat of each difficulty adjustment period
        if sat % (2016 * 50_000_000) == 0 {
            return Rarity::Rare;
        }

        // Uncommon: first sat of each block
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

    pub fn icon(&self) -> char {
        match self {
            Rarity::Common => '⚪',
            Rarity::Uncommon => '🟢',
            Rarity::Rare => '🔵',
            Rarity::Epic => '🟣',
            Rarity::Legendary => '🟠',
            Rarity::Mythic => '🔴',
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
    Audio,
    Code,
    Font,
    Iframe,
    Image,
    Markdown,
    Model,
    Pdf,
    Text,
    Unknown,
    Video,
}

impl Media {
    pub fn from_content_type(content_type: &str) -> Self {
        match content_type {
            ct if ct.starts_with("audio/") => Media::Audio,
            ct if ct.starts_with("font/") => Media::Font,
            ct if ct.starts_with("image/") => Media::Image,
            ct if ct.starts_with("model/") => Media::Model,
            ct if ct.starts_with("text/") => {
                match ct {
                    "text/html" => Media::Iframe,
                    "text/markdown" => Media::Markdown,
                    "text/plain" => Media::Text,
                    _ if ct.contains("javascript") || ct.contains("json") => Media::Code,
                    _ => Media::Text,
                }
            }
            ct if ct.starts_with("video/") => Media::Video,
            "application/pdf" => Media::Pdf,
            ct if ct.contains("json") || ct.contains("javascript") => Media::Code,
            _ => Media::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Media::Audio => "audio",
            Media::Code => "code",
            Media::Font => "font",
            Media::Iframe => "iframe",
            Media::Image => "image",
            Media::Markdown => "markdown",
            Media::Model => "model",
            Media::Pdf => "pdf",
            Media::Text => "text",
            Media::Unknown => "unknown",
            Media::Video => "video",
        }
    }
}

impl fmt::Display for Media {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}