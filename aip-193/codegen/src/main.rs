//! Regenerates `aip-193/src/proto/generated/` from `aip-193/proto/`.
//!
//! Run from the repository root:
//!
//! ```sh
//! cargo run --manifest-path aip-193/codegen/Cargo.toml
//! ```
//!
//! Requires `protoc` on `PATH`, or the `PROTOC` environment variable.

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let proto_dir = crate_dir.join("proto");
    let out_dir = crate_dir.join("src/proto/generated");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir)?;
    }
    connectrpc_build::Config::new()
        .files(&[
            proto_dir.join("google/rpc/code.proto"),
            proto_dir.join("google/rpc/status.proto"),
            proto_dir.join("google/rpc/error_details.proto"),
        ])
        .includes(&[proto_dir])
        .out_dir(&out_dir)
        .include_file("mod.rs")
        .emit_rerun_directives(false)
        .compile()?;
    println!("wrote {}", out_dir.display());
    Ok(())
}
