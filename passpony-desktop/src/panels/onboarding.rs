//! The first-run tour: pick a format, set up keys (generate an age identity for
//! passage, matching the phones, or import a PGP key for pass), back up the age
//! identity, then create or clone a store. Shown in place of the main UI until
//! it completes or is skipped. Actions are deferred past the mutable borrow of
//! `app.onboarding`, the same pattern the editor uses.

use pass_core::store::StoreFormat;

use crate::app::{App, OnbStep};
use crate::i18n;

pub fn wizard(ui: &mut egui::Ui, app: &mut App) {
    let t = i18n::s();
    let mut goto: Option<OnbStep> = None;
    let (mut do_generate, mut do_import, mut do_export) = (false, false, false);
    let (mut do_create, mut do_clone, mut do_skip) = (false, false, false);

    if let Some(onb) = app.onboarding.as_mut() {
        ui.add_space(8.0);
        ui.heading(t.welcome);
        ui.separator();
        ui.add_space(6.0);

        match onb.step {
            OnbStep::Welcome => {
                ui.label(t.pick_format);
                ui.add_space(4.0);
                ui.radio_value(&mut onb.format, StoreFormat::Pass, t.fmt_pass);
                ui.radio_value(&mut onb.format, StoreFormat::Passage, t.fmt_passage);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(t.next).clicked() {
                        goto = Some(OnbStep::Keys);
                    }
                    if ui.button(t.skip_setup).clicked() {
                        do_skip = true;
                    }
                });
            }
            OnbStep::Keys => match onb.format {
                StoreFormat::Passage => {
                    ui.label(t.gen_identity_desc);
                    ui.add_space(4.0);
                    if ui.button(t.generate_my_identity).clicked() {
                        do_generate = true;
                    }
                    if let Some(r) = &onb.recipient {
                        ui.add_space(4.0);
                        ui.label(t.your_recipient);
                        ui.monospace(r);
                    } else if onb.identity_ready {
                        ui.add_space(4.0);
                        ui.weak(t.already_have_identity);
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t.back).clicked() {
                            goto = Some(OnbStep::Welcome);
                        }
                        if ui
                            .add_enabled(onb.identity_ready, egui::Button::new(t.next))
                            .clicked()
                        {
                            goto = Some(OnbStep::Backup);
                        }
                    });
                }
                StoreFormat::Pass => {
                    ui.label(t.import_key_desc);
                    ui.add_space(4.0);
                    if ui.button(t.import_pgp).clicked() {
                        do_import = true;
                    }
                    if let Some(r) = &onb.recipient {
                        ui.add_space(4.0);
                        ui.label(t.imported_key);
                        ui.monospace(r);
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t.back).clicked() {
                            goto = Some(OnbStep::Welcome);
                        }
                        let ready = onb.recipient.is_some();
                        if ui.add_enabled(ready, egui::Button::new(t.next)).clicked() {
                            goto = Some(OnbStep::Store);
                        }
                    });
                }
            },
            OnbStep::Backup => {
                ui.colored_label(egui::Color32::LIGHT_YELLOW, t.backup_warn);
                ui.label(t.backup_desc);
                ui.add_space(4.0);
                if ui.button(t.save_a_copy).clicked() {
                    do_export = true;
                }
                if onb.backed_up {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, t.backed_up);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(t.back).clicked() {
                        goto = Some(OnbStep::Keys);
                    }
                    let label = if onb.backed_up {
                        t.next
                    } else {
                        t.skip_for_now
                    };
                    if ui.button(label).clicked() {
                        goto = Some(OnbStep::Store);
                    }
                });
            }
            OnbStep::Store => {
                ui.label(t.where_store);
                ui.add(egui::TextEdit::singleline(&mut onb.store_path).desired_width(380.0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(t.create_store).clicked() {
                        do_create = true;
                    }
                    if ui.button(t.clone_instead).clicked() {
                        do_clone = true;
                    }
                });
                ui.add_space(4.0);
                if ui.button(t.back).clicked() {
                    goto = Some(if onb.format == StoreFormat::Passage {
                        OnbStep::Backup
                    } else {
                        OnbStep::Keys
                    });
                }
            }
        }
    }

    if let Some(s) = goto {
        app.onb_set_step(s);
    }
    if do_generate {
        app.onb_generate_identity();
    }
    if do_import {
        app.onb_import_key();
    }
    if do_export {
        app.onb_export_identity();
    }
    if do_create {
        app.onb_create_store();
    }
    if do_clone {
        app.onb_go_clone();
    }
    if do_skip {
        app.onb_skip();
    }

    if !app.status.is_empty() {
        ui.add_space(10.0);
        ui.separator();
        ui.weak(&app.status);
    }
}
