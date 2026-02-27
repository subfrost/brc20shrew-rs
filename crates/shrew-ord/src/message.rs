use prost::{Message, DecodeError};

pub struct InscriptionMessageContext;

impl InscriptionMessageContext {
    pub fn new() -> Self { Self }
    pub fn serialize<T: Message>(message: &T) -> Vec<u8> { message.encode_to_vec() }
    pub fn deserialize<T: Message + Default>(bytes: &[u8]) -> Result<T, DecodeError> { Message::decode(bytes) }
}

impl Default for InscriptionMessageContext {
    fn default() -> Self { Self::new() }
}
