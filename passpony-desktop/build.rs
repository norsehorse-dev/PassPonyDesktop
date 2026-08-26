//! Records the compiler and target triple into the binary so `passpony version`
//! can report exactly what built it. CI asserts on the artifact, so runtime
//! drift is visible rather than assumed (the same reason rust-toolchain.toml is
//! checked in).

use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown rustc".into());
    println!("cargo:rustc-env=PASSPONY_RUSTC={version}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown-target".into());
    println!("cargo:rustc-env=PASSPONY_TARGET={target}");
}
