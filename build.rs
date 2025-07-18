fn main() {
  println!("cargo:rerun-if-changed=proto/shrewscriptions.proto");
  tonic_build::configure()
    .build_server(true)
    .build_client(true)
    .compile(&["proto/shrewscriptions.proto"], &["proto"])
    .unwrap();
}