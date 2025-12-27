fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        // optional, but often useful if you want `common` to compile cleanly for wasm without “connect()”
        .build_transport(false)
        .compile_protos(&["proto/shapes.proto"], &["proto"])?;
    Ok(())
}
