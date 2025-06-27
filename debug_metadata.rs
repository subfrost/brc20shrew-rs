use crate::tests::helpers::*;
use crate::envelope::*;

fn debug_metadata_parsing() {
    // Create the exact same metadata as the test
    let json_metadata = br#"{"name": "Test NFT", "description": "A test inscription", "attributes": [{"trait_type": "Color", "value": "Blue"}]}"#;
    
    // Create envelope with metadata
    let envelope = create_inscription_envelope_with_metadata(
        b"text/plain", 
        b"Content with JSON metadata",
        Some(json_metadata)
    );
    
    // Get the raw bytes from the witness
    let witness_bytes = &envelope[0];
    
    println!("Raw witness bytes: {:?}", witness_bytes);
    println!("Raw witness bytes (hex): {}", hex::encode(witness_bytes));
    
    // Parse the envelope
    let script = bitcoin::ScriptBuf::from_bytes(witness_bytes.clone());
    let parsed_envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
    
    if let Some(env) = parsed_envelope {
        if let Some(metadata) = &env.payload.metadata {
            println!("Parsed metadata: {:?}", metadata);
            println!("Expected metadata: {:?}", json_metadata);
            println!("First 10 bytes of parsed: {:?}", &metadata[..10.min(metadata.len())]);
            println!("First 10 bytes of expected: {:?}", &json_metadata[..10]);
        }
    }
}