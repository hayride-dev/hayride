use std::process::Command;

fn main() {
    // Run the frontend build command
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir("../hayride-ui") // Path to the frontend crate
        .status()
        .expect("Failed to build hayride-ui");

    if !status.success() {
        panic!("Failed to build hayride-ui crate!");
    }

    println!("cargo:rerun-if-changed=../hayride-ui");
    tauri_build::build()
}
