use std::env;
use std::path::PathBuf;

fn main() {
    // Generate protobuf bindings
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(&out_dir)
        .inputs(&["proto/shrewscriptions.proto"])
        .include("proto")
        .run()
        .expect("Failed to generate protobuf bindings");
    
    // Tell cargo to rerun if proto files change
    println!("cargo:rerun-if-changed=proto/");
    
    // Set up WASM target optimizations
    if env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "wasm32" {
        println!("cargo:rustc-link-arg=--import-memory");
        println!("cargo:rustc-link-arg=--export-memory");
        println!("cargo:rustc-link-arg=--shared-memory");
        println!("cargo:rustc-link-arg=--max-memory=4294967296");
    }
}