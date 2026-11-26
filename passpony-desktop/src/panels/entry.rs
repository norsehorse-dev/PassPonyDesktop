//! The opened-entry detail: a card with the password (reveal/hide and copy),
//! the `key: value` fields, and — if the entry has one — a second card with the
//! live TOTP code and its countdown ring. Copies go to the auto-clearing
//! clipboard. Plaintext lives in the `EntryView`'s zeroizing buffer and is
//! cleared when the selection changes or the store closes.

use std::time::{SystemTime, UNIX_EPOCH};

use egui::{RichText, Vec2};

use crate::app::{App, KeyPrompt};
use crate::entry_view::group_code;
use crate::i18n;
use crate::theme;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Render the detail cards for the selected entry.
pub fn detail(ui: &mut egui::Ui, app: &mut App) {
    let t = i18n::s();
    let Some(sel) = app.selected.clone() else {
        theme::card(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(theme::space::SECTION);
                ui.label(RichText::new(t.select_entry).color(ui.visuals().weak_text_color()));
                ui.add_space(theme::space::SECTION);
            });
        });
        return;
    };

    let mut do_edit = false;
    let mut do_delete = false;
    let mut reveal = app.reveal;
    let mut copy_request: Option<String> = None;

    theme::card(ui, |ui| {
        // Detail head: the name and the Edit / Delete actions.
        ui.horizontal(|ui| {
            ui.label(RichText::new(&sel).monospace().strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.confirm_delete {
                    if theme::danger_button(ui, t.confirm_delete).clicked() {
                        do_delete = true;
                    }
                    if ui.button(t.cancel).clicked() {
                        app.confirm_delete = false;
                    }
                } else {
                    if theme::danger_button(ui, t.delete).clicked() {
                        app.confirm_delete = true;
                    }
                    if ui.button(t.edit).clicked() {
                        do_edit = true;
                    }
                }
            });
        });

        if let Some(err) = app.entry_error.clone() {
            ui.add_space(theme::space::SM);
            ui.colored_label(theme::DANGER, err);
            ui.weak(t.decrypt_hint);
            return;
        }
        if app.entry.is_none() {
            ui.add_space(theme::space::SM);
            ui.weak(t.opening);
            // A YubiKey decrypt: show what the plugin is waiting for.
            if let Some(msg) = &app.key_message {
                ui.label(msg);
            }
            return;
        }

        let view = app.entry.as_ref().expect("checked above");
        ui.add_space(theme::space::SM);

        // Password row.
        field_row(ui, t.password, |ui| {
            if reveal {
                ui.label(RichText::new(view.password.as_str()).monospace());
            } else {
                ui.label(RichText::new("•••••••••••••").monospace());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t.copy).clicked() {
                    copy_request = Some(view.password.to_string());
                }
                if ui.button(if reveal { t.hide } else { t.reveal }).clicked() {
                    reveal = !reveal;
                }
            });
        });

        for (k, v) in &view.fields {
            field_row(ui, k, |ui| {
                ui.label(RichText::new(v).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(t.copy).clicked() {
                        copy_request = Some(v.clone());
                    }
                });
            });
        }
    });

    // TOTP card (only when the entry carries an otpauth line).
    if let Some(view) = app.entry.as_ref() {
        if let Some(otp) = &view.otp {
            let now = now_unix();
            let code = otp.code_at(now);
            let remaining = otp.seconds_remaining(now);
            let frac = remaining as f32 / otp.period.max(1) as f32;
            theme::card(ui, |ui| {
                theme::eyebrow(ui, t.one_time_code);
                ui.horizontal(|ui| {
                    totp_ring(ui, frac, remaining);
                    ui.add_space(theme::space::SM);
                    ui.label(RichText::new(group_code(&code)).monospace().size(26.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t.copy_code).clicked() {
                            copy_request = Some(code.clone());
                        }
                    });
                });
            });
        }
    }

    // Clipboard countdown, if a copy is pending.
    if let Some(secs) = app.clipboard_seconds_left() {
        ui.add_space(theme::space::SM);
        ui.weak(t.clipboard_clears.replace("{}", &secs.to_string()));
    }

    // Apply deferred mutations after the borrows end.
    app.reveal = reveal;
    if do_edit {
        app.start_edit_selected();
    } else if do_delete {
        app.delete_selected();
    } else if let Some(text) = copy_request {
        app.copy_secret(&text);
    }
}

/// A labelled row: a fixed-width key on the left, the value builder on the right,
/// separated by a hairline above.
fn field_row(ui: &mut egui::Ui, key: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(80.0, 20.0),
            egui::Label::new(
                RichText::new(key.trim_end_matches(':'))
                    .color(ui.visuals().weak_text_color())
                    .size(12.5),
            ),
        );
        add(ui);
    });
}

/// A small countdown ring: a full track with a coloured arc for the fraction of
/// the period remaining, and the seconds in the middle. Uses the brand orange.
fn totp_ring(ui: &mut egui::Ui, frac: f32, remaining: u64) {
    let size = Vec2::new(44.0, 44.0);
    let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = rect.width() / 2.0 - 4.0;

    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(4.0, ui.visuals().faint_bg_color),
    );

    let start = -std::f32::consts::FRAC_PI_2;
    let sweep = frac.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let steps = 48;
    let points: Vec<egui::Pos2> = (0..=steps)
        .map(|i| {
            let a = start + sweep * (i as f32 / steps as f32);
            egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
        })
        .collect();
    let accent = if remaining <= 5 {
        theme::DANGER
    } else {
        theme::ORANGE_CORE
    };
    painter.add(egui::Shape::line(points, egui::Stroke::new(4.0, accent)));
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        remaining.to_string(),
        egui::FontId::monospace(12.0),
        ui.visuals().text_color(),
    );
}

/// The YubiKey plugin's prompt (PIN, or "is the YubiKey plugged in?"),
/// floating over any screen while an entry opens. Answering unblocks the
/// worker; closing the window cancels.
pub fn key_prompt_window(ctx: &egui::Context, app: &mut App) {
    let t = i18n::s();
    let Some(prompt) = &app.key_prompt else {
        return;
    };
    let mut open = true;
    let mut confirm: Option<Option<bool>> = None;
    let mut secret: Option<bool> = None;
    egui::Window::new("YubiKey")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| match prompt {
            KeyPrompt::Confirm { text, yes, no } => {
                ui.label(text);
                ui.add_space(theme::space::SM);
                ui.horizontal(|ui| {
                    if ui.button(yes).clicked() {
                        confirm = Some(Some(true));
                    }
                    if let Some(no) = no {
                        if ui.button(no).clicked() {
                            confirm = Some(Some(false));
                        }
                    }
                });
            }
            KeyPrompt::Secret { description } => {
                ui.label(description);
                ui.add_space(theme::space::SM);
                let field = ui.add(
                    egui::TextEdit::singleline(&mut app.key_pin)
                        .password(true)
                        .desired_width(220.0),
                );
                // Focus the PIN once, when the prompt opens; grabbing it every
                // frame would swallow Enter and Tab.
                if std::mem::take(&mut app.key_pin_focus) {
                    field.request_focus();
                }
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button(t.unlock).clicked() || entered {
                        secret = Some(true);
                    }
                    if ui.button(t.cancel).clicked() {
                        secret = Some(false);
                    }
                });
            }
        });
    if let Some(answer) = confirm {
        app.answer_key_confirm(answer);
    } else if let Some(send) = secret {
        app.answer_key_secret(send);
    } else if !open {
        match app.key_prompt {
            Some(KeyPrompt::Confirm { .. }) => app.answer_key_confirm(None),
            Some(KeyPrompt::Secret { .. }) => app.answer_key_secret(false),
            None => {}
        }
    }
}
