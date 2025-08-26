fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/github.com/containerd/containerd/api/services/images/v1/images.proto"], &["proto"])?;
    Ok(())
}
