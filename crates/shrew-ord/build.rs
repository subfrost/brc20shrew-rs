fn main() -> std::io::Result<()> {
    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["proto/ord.proto"], &["proto/"])?;
    Ok(())
}
