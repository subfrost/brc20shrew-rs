//! Message context for protobuf handling in the metashrew environment

use prost::{Message, DecodeError};

/// Message context for handling protobuf serialization/deserialization
/// in the metashrew WASM environment
pub struct InscriptionMessageContext;

impl InscriptionMessageContext {
    /// Create a new message context
    pub fn new() -> Self {
        Self
    }

    /// Serialize a protobuf message to bytes
    pub fn serialize<T: Message>(message: &T) -> Vec<u8> {
        message.encode_to_vec()
    }

    /// Deserialize bytes to a protobuf message
    pub fn deserialize<T: Message + Default>(bytes: &[u8]) -> Result<T, DecodeError> {
        Message::decode(bytes)
    }

}

impl Default for InscriptionMessageContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_context_creation() {
        let _context = InscriptionMessageContext::new();
        let _default_context = InscriptionMessageContext::default();
    }
}