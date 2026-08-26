//! PassPony Desktop — the GUI binary.
//!
//! Run with no arguments it opens the window. Run with a verb it behaves like
//! `passpony-cli`, which is what lets CI verify a packaged build on macOS and
//! Linux without shipping a second file there. On Windows this binary is
//! GUI-subsystem (no console), so `passpony-cli.exe` is the one that can print;
//! see src/bin/passpony-cli.rs.

#![forbid(unsafe_code)]
#![cfg_attr(
    all(not(debug_assertions), feature = "gui"),
    windows_subsystem = "windows"
)]

fn main() -> anyhow::Result<()> {
    if let Some(code) = passpony_desktop::cli::run(&passpony_desktop::args()) {
        std::process::exit(code);
    }
    #[cfg(feature = "gui")]
    {
        passpony_desktop::run_gui().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }
    #[cfg(not(feature = "gui"))]
    {
        eprintln!("passpony: built without the gui feature; try `passpony help`");
    }
    Ok(())
}
