//! Ord inscription types and functionality
//! 
//! This module contains inscription-related code ported from the ord crate
//! to provide proper inscription envelope creation and parsing.

pub mod envelope;
pub mod inscription;
pub mod tag;

pub use envelope::{Envelope, ParsedEnvelope, RawEnvelope, PROTOCOL_ID, BODY_TAG};
pub use inscription::Inscription;
pub use tag::Tag;