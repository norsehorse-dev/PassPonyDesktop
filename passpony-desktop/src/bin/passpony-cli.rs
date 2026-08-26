//! `passpony-cli` — the console-subsystem twin, for Windows.
//!
//! A GUI-subsystem process on Windows has no stdout, so the verbs on the main
//! `passpony` binary would run and print into nothing. This binary omits the
//! windows-subsystem attribute so it can write to a terminal. On macOS and
//! Linux there is no such split and the main binary does both jobs; this one is
//! built everywhere anyway so it cannot rot.

#![forbid(unsafe_code)]

fn main() {
    let code = passpony_desktop::cli::run(&passpony_desktop::args()).unwrap_or_else(|| {
        println!("{}", passpony_desktop::cli::version_line());
        println!("(this is the console tool; run `passpony` for the app, or `passpony-cli help`)");
        0
    });
    std::process::exit(code);
}
