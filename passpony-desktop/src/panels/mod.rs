//! The screens behind the four rail destinations, plus the first-run wizard and
//! the shared entry editor. Each screen is one `screen(ui, app)` entry point the
//! app shell dispatches to; the editor and the entry detail are used inside the
//! Passwords screen.

pub mod editor;
pub mod entry;
pub mod keys;
pub mod onboarding;
pub mod passwords;
pub mod settings;
pub mod sync;

/// A screen header: a bold title with an optional muted one-line subtitle. Used
/// by the Keys, Sync, and Settings screens (Passwords has its own store head).
pub fn screen_header(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(
        egui::RichText::new(title)
            .font(crate::theme::semibold(20.0))
            .size(20.0),
    );
    if !sub.is_empty() {
        ui.add_space(2.0);
        ui.label(egui::RichText::new(sub).color(ui.visuals().weak_text_color()));
    }
}
