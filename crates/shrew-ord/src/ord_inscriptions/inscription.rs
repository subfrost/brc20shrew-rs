//! Inscription type and functionality
//! 
//! Ported from ord/src/inscriptions/inscription.rs

use super::{envelope, Tag};
use bitcoin::{
    blockdata::{opcodes, constants::MAX_SCRIPT_ELEMENT_SIZE},
    script, ScriptBuf, Witness,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Eq, Default)]
pub struct Inscription {
    pub body: Option<Vec<u8>>,
    pub content_encoding: Option<Vec<u8>>,
    pub content_type: Option<Vec<u8>>,
    pub delegate: Option<Vec<u8>>,
    pub duplicate_field: bool,
    pub incomplete_field: bool,
    pub metadata: Option<Vec<u8>>,
    pub metaprotocol: Option<Vec<u8>>,
    pub parents: Vec<Vec<u8>>,
    pub pointer: Option<Vec<u8>>,
    pub properties: Option<Vec<u8>>,
    pub rune: Option<Vec<u8>>,
    pub unrecognized_even_field: bool,
}

impl Inscription {
    pub fn append_reveal_script_to_builder(&self, mut builder: script::Builder) -> script::Builder {
        builder = builder
            .push_opcode(opcodes::OP_FALSE)
            .push_opcode(opcodes::all::OP_IF)
            .push_slice(envelope::PROTOCOL_ID);

        Tag::ContentType.append(&mut builder, &self.content_type);
        Tag::ContentEncoding.append(&mut builder, &self.content_encoding);
        Tag::Metaprotocol.append(&mut builder, &self.metaprotocol);
        Tag::Parent.append_array(&mut builder, &self.parents);
        Tag::Delegate.append(&mut builder, &self.delegate);
        Tag::Pointer.append(&mut builder, &self.pointer);
        Tag::Metadata.append(&mut builder, &self.metadata);
        Tag::Rune.append(&mut builder, &self.rune);
        Tag::Properties.append(&mut builder, &self.properties);

        if let Some(body) = &self.body {
            builder = builder.push_slice(envelope::BODY_TAG);
            for chunk in body.chunks(MAX_SCRIPT_ELEMENT_SIZE) {
                builder = builder.push_slice::<&script::PushBytes>(chunk.try_into().unwrap());
            }
        }

        builder.push_opcode(opcodes::all::OP_ENDIF)
    }

    pub fn append_reveal_script(&self, builder: script::Builder) -> ScriptBuf {
        self.append_reveal_script_to_builder(builder).into_script()
    }

    pub fn to_witness(&self) -> Witness {
        let builder = script::Builder::new();

        let script = self.append_reveal_script(builder);

        let mut witness = Witness::new();

        witness.push(script);
        witness.push([]);

        witness
    }

    pub fn body(&self) -> Option<&[u8]> {
        Some(self.body.as_ref()?)
    }

    pub fn into_body(self) -> Option<Vec<u8>> {
        self.body
    }

    pub fn content_length(&self) -> Option<usize> {
        Some(self.body()?.len())
    }

    pub fn content_type(&self) -> Option<&str> {
        std::str::from_utf8(self.content_type.as_ref()?).ok()
    }
}