//! PassPony Desktop.
//!
//! A library so the GUI binary and the console binary share one codebase. On
//! macOS and Linux one binary does both jobs; on Windows a GUI-subsystem process
//! has no stdout, so `passpony-cli` exists as a second, console-subsystem binary
//! over exactly this code.
//!
//! The crypto backends and CLI verbs are always built. The egui GUI is behind
//! the default `gui` feature, so the core logic compiles and tests without a
//! display stack.

#![forbid(unsafe_code)]

pub mod cli;
pub mod crypto;
pub mod edit;
pub mod entry_view;
pub mod i18n;
pub mod keys;
pub mod pwgen;
pub mod sync;

#[cfg(feature = "gui")]
pub mod app;
#[cfg(feature = "gui")]
pub mod appicon;
#[cfg(feature = "gui")]
pub mod panels;
#[cfg(feature = "gui")]
pub mod tasks;
#[cfg(feature = "gui")]
pub mod theme;

/// Arguments, minus the program name.
#[must_use]
pub fn args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

/// Open the window.
///
/// # Errors
///
/// Whatever `eframe` returns if the window cannot be created.
#[cfg(feature = "gui")]
pub fn run_gui() -> eframe::Result {
    // The dock/taskbar/Alt-Tab icon: the shipped PassPony artwork (horse + keys
    // on the orange gradient), the same master the phones use.
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1040.0, 720.0])
        .with_min_inner_size([820.0, 540.0])
        .with_title("PassPony");
    if let Some(icon) = appicon::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "PassPony",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
