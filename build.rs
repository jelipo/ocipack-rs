fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tonic_build::configure()
    //     .out_dir("src/adapter/containerd-adapter")
    //     .compile(&["proto/images.proto"], &["proto"])
    //     .unwrap();
    Ok(())
}

fn build_tonic() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello");
    Ok(())
}