use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    
    // Generate protobuf code
    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(&out_dir)
        .inputs(&["proto/shrewscriptions.proto"])
        .include("proto")
        .run()
        .expect("Failed to generate protobuf bindings");
    
    // Post-process the generated file to remove inner attributes
    let generated_file = Path::new(&out_dir).join("shrewscriptions.rs");
    if generated_file.exists() {
        let content = fs::read_to_string(&generated_file)
            .expect("Failed to read generated protobuf file");
        
        // Replace inner attributes with outer attributes and fix doc comments
        let fixed_content = content
            .replace("#![allow(unknown_lints)]", "#[allow(unknown_lints)]")
            .replace("#![allow(clippy::all)]", "#[allow(clippy::all)]")
            .replace("#![allow(unused_attributes)]", "#[allow(unused_attributes)]")
            .replace("#![cfg_attr(rustfmt, rustfmt::skip)]", "#[cfg_attr(rustfmt, rustfmt::skip)]")
            .replace("#![allow(dead_code)]", "#[allow(dead_code)]")
            .replace("#![allow(missing_docs)]", "#[allow(missing_docs)]")
            .replace("#![allow(non_camel_case_types)]", "#[allow(non_camel_case_types)]")
            .replace("#![allow(non_snake_case)]", "#[allow(non_snake_case)]")
            .replace("#![allow(non_upper_case_globals)]", "#[allow(non_upper_case_globals)]")
            .replace("#![allow(trivial_casts)]", "#[allow(trivial_casts)]")
            .replace("#![allow(unused_results)]", "#[allow(unused_results)]")
            .replace("#![allow(unused_mut)]", "#[allow(unused_mut)]")
            .replace("//! Generated file from", "// Generated file from");
        
        fs::write(&generated_file, fixed_content)
            .expect("Failed to write fixed protobuf file");
    }
    
    println!("cargo:rerun-if-changed=proto/shrewscriptions.proto");
}