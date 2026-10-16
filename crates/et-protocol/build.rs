use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "proto/inference.proto";
    println!("cargo:rerun-if-changed={proto}");
    let fds = protox::compile([proto], ["proto"])?;
    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    tonic_prost_build::configure()
        .file_descriptor_set_path(out.join("inference_descriptor.bin"))
        .compile_fds(fds)?;
    Ok(())
}
