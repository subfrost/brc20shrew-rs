//! Message context for protobuf handling in the metashrew environment

use metashrew_support::compat::{to_arraybuffer_layout, to_ptr};

/// Message context for handling protobuf serialization/deserialization
/// in the metashrew WASM environment
pub struct InscriptionMessageContext;

impl InscriptionMessageContext {
    /// Create a new message context
    pub fn new() -> Self {
        Self
    }

    /// Serialize a protobuf message to bytes
    pub fn serialize<T: protobuf::Message>(message: &T) -> Result<Vec<u8>, protobuf::Error> {
        message.write_to_bytes()
    }

    /// Deserialize bytes to a protobuf message
    pub fn deserialize<T: protobuf::Message + Default>(bytes: &[u8]) -> Result<T, protobuf::Error> {
        protobuf::Message::parse_from_bytes(bytes)
    }

    /// Convert bytes to WASM-compatible pointer for output
    pub fn to_output_ptr(data: Vec<u8>) -> *const u8 {
        let mut buffer = to_arraybuffer_layout(&data);
        to_ptr(&mut buffer) as *const u8
    }

    /// Load input data from WASM environment
    pub fn load_input() -> Vec<u8> {
        // This would normally call metashrew host function
        // For now, return empty vec as placeholder
        Vec::new()
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