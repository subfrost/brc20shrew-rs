//! Debug envelope parsing to isolate content truncation issue

use {
    metashrew_core::{println, stdio::stdout},
    std::fmt::Write
};

use crate::envelope::parse_envelope_from_script;
use crate::tests::helpers::create_inscription_envelope;
use bitcoin::ScriptBuf;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn test_envelope_parsing_debug() {
    // Test the exact same content that's failing in delegation
    let content = b"This is the actual content that will be delegated";
    let content_type = b"text/plain";
    
    println!("Original content length: {}", content.len());
    println!("Original content: {:?}", content);
    
    // Create envelope using the same helper as the failing test
    let witness = create_inscription_envelope(content_type, content);
    
    // Extract the script from the witness
    let script_bytes = witness.iter().next().unwrap();
    let script = ScriptBuf::from_bytes(script_bytes.to_vec());
    
    println!("Script bytes length: {}", script_bytes.len());
    println!("Script bytes: {:?}", script_bytes);
    
    // Let's manually examine the envelope structure
    println!("Envelope structure analysis:");
    let mut pos = 0;
    while pos < script_bytes.len() {
        println!("  Position {}: 0x{:02x} ({})", pos, script_bytes[pos], script_bytes[pos]);
        pos += 1;
        if pos > 80 { // Show more of the structure
            println!("  ... (truncated)");
            break;
        }
    }
    
    // Parse the envelope
    match parse_envelope_from_script(&script, 0, 0) {
        Ok(Some(envelope)) => {
            println!("Envelope parsed successfully");
            if let Some(body) = &envelope.payload.body {
                println!("Parsed body length: {}", body.len());
                println!("Parsed body: {:?}", body);
                
                if body != content {
                    panic!("Content mismatch: expected {:?}, got {:?}", content, body);
                }
                println!("Content matches!");
            } else {
                panic!("No body found in parsed envelope");
            }
        }
        Ok(None) => {
            panic!("No envelope found in script");
        }
        Err(e) => {
            panic!("Error parsing envelope: {:?}", e);
        }
    }
}

#[wasm_bindgen_test]
fn test_envelope_parsing_simple() {
    // Test with simple short content first
    let content = b"Hello";
    let content_type = b"text/plain";
    
    println!("Simple content length: {}", content.len());
    println!("Simple content: {:?}", content);
    
    let witness = create_inscription_envelope(content_type, content);
    let script_bytes = witness.iter().next().unwrap();
    let script = ScriptBuf::from_bytes(script_bytes.to_vec());
    
    println!("Simple script bytes: {:?}", script_bytes);
    
    match parse_envelope_from_script(&script, 0, 0) {
        Ok(Some(envelope)) => {
            if let Some(body) = &envelope.payload.body {
                println!("Simple parsed body: {:?}", body);
                assert_eq!(body, content);
                println!("Simple content matches!");
            } else {
                panic!("No body in simple envelope");
            }
        }
        Ok(None) => panic!("No simple envelope found"),
        Err(e) => panic!("Error parsing simple envelope: {:?}", e),
    }
}