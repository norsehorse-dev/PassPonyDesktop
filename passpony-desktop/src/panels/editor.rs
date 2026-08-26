//! The entry editor: a name, a raw content box whose first line is the
//! password, and a generator. Mirrors `pass edit` (you edit the whole file),
//! which keeps arbitrary fields and `otpauth://` lines working for free. Saving
//! encrypts and commits through [`crate::edit`]. Rendered as a card inside the
//! Passwords screen's detail column.

use egui::{Color32, RichText};

use crate::app::App;
use crate::i18n;
use crate::pwgen::{self, PwOptions};
use crate::theme;

/// Replace (or set) the first line of `text` with `pw`, keeping the rest.
fn set_password_line(text: &mut String, pw: &str) {
    let rest = match text.split_once('\n') {
        Some((_, r)) => format!("\n{r}"),
        None => String::new(),
    };
    *text = format!("{pw}{rest}");
}

pub fn editor(ui: &mut egui::Ui, app: &mut App) {
    let t = i18n::s();
    let mut do_save = false;
    let mut do_cancel = false;

    theme::card(ui, |ui| {
        if let Some(edit) = app.editing.as_mut() {
            theme::eyebrow(
                ui,
                if edit.is_new {
                    t.new_entry_title
                } else {
                    t.edit_entry_title
                },
            );

            ui.horizontal(|ui| {
                ui.label(RichText::new(t.name).color(ui.visuals().weak_text_color()));
                // Renaming an existing entry is a move (a later phase); the name
                // is fixed once the entry exists.
                ui.add_enabled(
                    edit.is_new,
                    egui::TextEdit::singleline(&mut edit.name)
                        .hint_text("web/example.com")
                        .desired_width(f32::INFINITY),
                );
            });

            ui.add_space(theme::space::SM);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.generate).color(ui.visuals().weak_text_color()));
                ui.add(egui::DragValue::new(&mut edit.gen_len).range(6..=128));
                if ui.button(t.new_password).clicked() {
                    let pw = pwgen::generate(&PwOptions {
                        len: edit.gen_len,
                        ..Default::default()
                    });
                    set_password_line(&mut edit.text, &pw);
                }
            });

            ui.add_space(theme::space::SM);
            ui.label(RichText::new(t.content_label).color(ui.visuals().weak_text_color()));
            egui::ScrollArea::vertical()
                .id_salt("editor-body")
                .max_height(280.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut edit.text)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("password\nlogin: you@example.com\notpauth://totp/…"),
                    );
                });

            ui.add_space(theme::space::MD);
            ui.horizontal(|ui| {
                let save = egui::Button::new(RichText::new(t.save).color(Color32::WHITE).strong())
                    .fill(theme::ORANGE_CORE)
                    .corner_radius(theme::radius::SM);
                if ui.add(save).clicked() {
                    do_save = true;
                }
                if ui.button(t.cancel).clicked() {
                    do_cancel = true;
                }
            });
        }
    });

    if do_save {
        app.save_edit();
    } else if do_cancel {
        app.cancel_edit();
    }
}
