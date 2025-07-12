//! Manual debug of envelope structure to understand the parsing issue

use crate::tests::helpers::create_inscription_envelope;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn test_manual_envelope_inspection() {
    // Test the exact same content that's failing in delegation
    let content = b"This is the actual content that will be delegated";
    let content_type = b"text/plain";
    
    println!("=== MANUAL ENVELOPE INSPECTION ===");
    println!("Content length: {}", content.len());
    println!("Content: {:?}", content);
    
    // Create envelope using the same helper as the failing test
    let witness = create_inscription_envelope(content_type, content);
    
    // Extract the script from the witness
    let script_bytes = witness.iter().next().unwrap();
    
    println!("Script bytes length: {}", script_bytes.len());
    println!("Full script bytes: {:?}", script_bytes);
    
    // Let's manually parse the envelope structure step by step
    println!("\n=== MANUAL PARSING ===");
    let mut pos = 0;
    
    // Look for envelope pattern: 0x00 0x63 0x03 "ord"
    while pos + 5 < script_bytes.len() {
        if script_bytes[pos] == 0x00 && script_bytes[pos + 1] == 0x63 &&
           script_bytes[pos + 2] == 0x03 && &script_bytes[pos + 3..pos + 6] == b"ord" {
            println!("Found envelope at position {}", pos);
            pos += 6; // Skip past 0x00 0x63 0x03 "ord"
            
            // Find the end of the envelope (OP_ENDIF = 0x68)
            let mut end_pos = pos;
            while end_pos < script_bytes.len() && script_bytes[end_pos] != 0x68 {
                end_pos += 1;
            }
            
            println!("Envelope data from {} to {} (length: {})", pos, end_pos, end_pos - pos);
            let envelope_data = &script_bytes[pos..end_pos];
            println!("Envelope data: {:?}", envelope_data);
            
            // Now manually parse the fields
            let mut field_pos = 0;
            while field_pos < envelope_data.len() {
                let field_tag = envelope_data[field_pos];
                field_pos += 1;
                
                println!("Field tag: {} at position {}", field_tag, field_pos - 1);
                
                if field_tag == 0 {
                    println!("Found body content tag (0)");
                    // This is body content
                    if field_pos < envelope_data.len() {
                        let chunk_length = envelope_data[field_pos] as usize;
                        field_pos += 1;
                        println!("Body chunk length: {}", chunk_length);
                        
                        if field_pos + chunk_length <= envelope_data.len() {
                            let chunk_data = &envelope_data[field_pos..field_pos + chunk_length];
                            println!("Body chunk data: {:?}", chunk_data);
                            println!("Body chunk as string: {:?}", String::from_utf8_lossy(chunk_data));
                            field_pos += chunk_length;
                        } else {
                            println!("Chunk extends beyond envelope data");
                            break;
                        }
                    }
                    break; // Body is the last field
                } else {
                    // Other field
                    if field_pos < envelope_data.len() {
                        let field_length = envelope_data[field_pos] as usize;
                        field_pos += 1;
                        println!("Field {} length: {}", field_tag, field_length);
                        
                        if field_pos + field_length <= envelope_data.len() {
                            let field_data = &envelope_data[field_pos..field_pos + field_length];
                            println!("Field {} data: {:?}", field_tag, field_data);
                            if field_tag == 1 {
                                println!("Content type: {:?}", String::from_utf8_lossy(field_data));
                            }
                            field_pos += field_length;
                        } else {
                            println!("Field extends beyond envelope data");
                            break;
                        }
                    }
                }
            }
            
            break;
        }
        pos += 1;
    }
}