//! The store browser: pick a format, open a store, list and search entries, and
//! open an entry to decrypt it with the loaded keys/identities. Browsing and
//! search decrypt nothing; only opening an entry does.

use pass_core::store::StoreFormat;

use crate::app::App;
use crate::i18n;

/// Render the central store view into the Ui eframe hands us.
pub fn central(ui: &mut egui::Ui, app: &mut App) {
    // First-run tour takes over the whole view until done or skipped.
    if app.onboarding.is_some() {
        super::onboarding::wizard(ui, app);
        return;
    }

    let s = i18n::s();
    ui.horizontal(|ui| {
        ui.heading("PassPony");
        ui.separator();
        let mut fmt = app.format;
        egui::ComboBox::from_id_salt("store-format")
            .selected_text(match fmt {
                StoreFormat::Pass => s.fmt_pass,
                StoreFormat::Passage => s.fmt_passage,
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut fmt, StoreFormat::Pass, s.fmt_pass);
                ui.selectable_value(&mut fmt, StoreFormat::Passage, s.fmt_passage);
            });
        if fmt != app.format {
            app.format = fmt;
            app.close_store();
        }
        if ui.button(s.open_store).clicked() {
            app.open_store_dialog();
        }
        if ui.button(s.default_store).clicked() {
            app.open_default_store();
        }
        if ui
            .add_enabled(app.store.is_some(), egui::Button::new(s.new_entry))
            .clicked()
        {
            app.start_new_entry();
        }
        ui.toggle_value(&mut app.show_keys, s.keys);
        ui.toggle_value(&mut app.show_sync, s.sync);
        ui.toggle_value(&mut app.show_settings, s.settings);
    });

    if app.show_settings {
        super::settings::screen(ui, app);
        return;
    }

    if app.show_keys {
        keys_section(ui, app);
    }
    if app.show_sync {
        super::sync::section(ui, app);
    }
    super::sync::conflict_window(ui.ctx(), app);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(s.search);
        ui.text_edit_singleline(&mut app.search);
    });
    ui.separator();

    if !app.status.is_empty() {
        ui.colored_label(egui::Color32::LIGHT_YELLOW, &app.status);
        ui.add_space(4.0);
    }

    // Two columns: entry list on the left; the editor if open, else the detail.
    ui.columns(2, |cols| {
        entry_list(&mut cols[0], app);
        if app.editing.is_some() {
            super::editor::editor(&mut cols[1], app);
        } else {
            super::entry::detail(&mut cols[1], app);
        }
    });
}

fn keys_section(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new(s.keys_identities).strong());
        ui.horizontal(|ui| {
            if ui.button(s.import_pgp).clicked() {
                app.import_pgp_dialog();
            }
            if ui.button(s.generate_age).clicked() {
                app.generate_age_identity();
            }
        });
        ui.horizontal(|ui| {
            ui.label(s.passphrase);
            let resp = ui.add(egui::TextEdit::singleline(&mut app.passphrase).password(true));
            if resp.changed() {
                app.note_passphrase_changed();
            }
            ui.weak(s.passphrase_hint);
        });
        if app.key_files.is_empty() {
            ui.weak(s.no_pass_keys);
        } else {
            ui.label(s.pass_keys.replace("{}", &app.key_files.join(", ")));
        }
    });
}

fn entry_list(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    match &app.entries {
        None => {
            ui.label(s.open_a_store);
        }
        Some(entries) => {
            let needle = app.search.to_lowercase();
            let shown: Vec<String> = entries
                .iter()
                .filter(|n| needle.is_empty() || n.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            ui.label(s.entries_count.replace("{}", &shown.len().to_string()));
            let mut to_open: Option<String> = None;
            egui::ScrollArea::vertical()
                .id_salt("entries")
                .show(ui, |ui| {
                    for name in &shown {
                        let selected = app.selected.as_deref() == Some(name.as_str());
                        if ui.selectable_label(selected, name).clicked() {
                            to_open = Some(name.clone());
                        }
                    }
                });
            if let Some(name) = to_open {
                app.open_entry(name);
            }
        }
    }
}
