//! The Keys screen: the OpenPGP keys that decrypt `pass`, and the age identity
//! that decrypts `passage`. Importing a key and generating/backing up an
//! identity live here; the passphrase for a locked pass key is entered here and
//! held for five minutes.

use egui::{Color32, RichText};

use crate::app::App;
use crate::i18n;
use crate::theme;

pub fn screen(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    egui::ScrollArea::vertical()
        .id_salt("keys-screen")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            super::screen_header(ui, s.keys_title, s.keys_sub);
            ui.add_space(theme::space::LG);
            pass_card(ui, app);
            ui.add_space(theme::space::MD);
            passage_card(ui, app);

            if !app.status.is_empty() {
                ui.add_space(theme::space::MD);
                ui.label(RichText::new(&app.status).color(theme::ORANGE_DEEP));
            }
        });
}

fn pass_card(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    theme::card(ui, |ui| {
        theme::eyebrow(ui, s.pass_keys_section);
        if app.key_files.is_empty() {
            ui.weak(s.no_pass_keys);
        } else {
            for (i, name) in app.key_files.iter().enumerate() {
                if i > 0 {
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).monospace());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        badge(ui, s.imported, true);
                    });
                });
            }
        }

        ui.add_space(theme::space::MD);
        ui.horizontal(|ui| {
            let import =
                egui::Button::new(RichText::new(s.import_pgp).color(Color32::WHITE).strong())
                    .fill(theme::ORANGE_CORE)
                    .corner_radius(theme::radius::SM);
            if ui.add(import).clicked() {
                app.import_pgp_dialog();
            }
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.passphrase)
                    .password(true)
                    .hint_text(s.passphrase_placeholder)
                    .desired_width(240.0),
            );
            if resp.changed() {
                app.note_passphrase_changed();
            }
        });
        ui.add_space(theme::space::SM);
        ui.weak(s.passphrase_note);
    });
}

fn passage_card(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    theme::card(ui, |ui| {
        theme::eyebrow(ui, s.passage_section);
        let present = crate::keys::has_age_identity();
        ui.horizontal(|ui| {
            if present {
                ui.label(RichText::new(s.age_identity_present).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    badge(ui, s.this_machine, true);
                });
            } else {
                ui.weak(s.no_age_identity);
            }
        });

        ui.add_space(theme::space::MD);
        ui.horizontal(|ui| {
            if ui.button(s.generate_age).clicked() {
                app.generate_age_identity();
            }
            if ui.button(s.back_up_export).clicked() {
                app.export_identity();
            }
        });
        ui.add_space(theme::space::SM);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(s.sync_not_keys).color(theme::DANGER).strong());
            ui.weak(s.keep_key_safe);
        });
    });
}

/// A small pill badge; `ok` tints it with the success colour.
fn badge(ui: &mut egui::Ui, text: &str, ok: bool) {
    let (fg, bg) = if ok {
        (theme::GOOD, theme::GOOD.gamma_multiply(0.16))
    } else {
        (ui.visuals().weak_text_color(), ui.visuals().faint_bg_color)
    };
    egui::Frame::default()
        .fill(bg)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(fg).size(11.0).strong());
        });
}
