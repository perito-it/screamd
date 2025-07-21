use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // This is typically <project_root>/target/<profile>/build/<crate-name>-<hash>/out
    // We want to get to <project_root>/target/<profile>/
    let dest_dir = out_dir.ancestors().nth(3).unwrap();

    // Define the source and destination paths
    let source_path = PathBuf::from("config/config.toml");
    let dest_path = dest_dir.join("config.toml");

    // Copy the file
    fs::copy(&source_path, &dest_path).unwrap_or_else(|e| {
        panic!("Failed to copy config.toml from {source_path:?} to {dest_path:?}: {e}")
    });

    // Re-run the build script if config.toml changes
    println!("cargo:rerun-if-changed=config/config.toml");
}
