fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile(&["api/services/images/v1/images.proto"], &["api"])
        .unwrap();
    Ok(())
}