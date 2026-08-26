//! The Sync screen: the working tree's status against its remote, the sync
//! actions, and the remote configuration. All git work runs on the worker
//! thread; this reads the `app` state the worker updates and fires the `start_*`
//! methods. The per-file conflict prompt floats over any screen.

use egui::{Color32, RichText};

use crate::app::App;
use crate::i18n;
use crate::sync::ConflictChoice;
use crate::theme;

pub fn screen(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    egui::ScrollArea::vertical()
        .id_salt("sync-screen")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            super::screen_header(ui, s.sync_title, s.sync_sub);
            ui.add_space(theme::space::LG);
            status_card(ui, app);
            ui.add_space(theme::space::MD);
            remote_card(ui, app);

            if !app.status.is_empty() {
                ui.add_space(theme::space::MD);
                ui.label(RichText::new(&app.status).color(theme::ORANGE_DEEP));
            }
        });
}

fn stat(ui: &mut egui::Ui, number: &str, label: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(number)
                .font(theme::semibold(22.0))
                .size(22.0)
                .color(color),
        );
        ui.label(
            RichText::new(label)
                .color(ui.visuals().weak_text_color())
                .size(12.0),
        );
    });
}

fn status_card(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    theme::card(ui, |ui| {
        match &app.sync_status {
            Some(st) => {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::space::SECTION;
                    let ahead_col = if st.ahead > 0 {
                        theme::GOOD
                    } else {
                        ui.visuals().text_color()
                    };
                    stat(ui, &st.ahead.to_string(), s.ahead, ahead_col);
                    stat(
                        ui,
                        &st.behind.to_string(),
                        s.behind,
                        ui.visuals().text_color(),
                    );
                    let (word, col) = if st.dirty {
                        (s.dirty_tree, theme::DANGER)
                    } else {
                        (s.clean_tree, theme::GOOD)
                    };
                    stat(ui, word, s.working_tree, col);
                });
                ui.add_space(theme::space::SM);
                let remote = app.remote_display.clone().unwrap_or_else(|| "—".into());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(s.remote_label).color(ui.visuals().weak_text_color()));
                    ui.label(RichText::new(remote).monospace());
                });
            }
            None => {
                if app.store.is_some() {
                    ui.weak(s.not_a_repo);
                } else {
                    ui.weak(s.open_or_clone);
                }
            }
        }

        ui.add_space(theme::space::MD);
        let enabled = !app.busy;
        let has_store = app.store.is_some();
        ui.horizontal_wrapped(|ui| {
            let sync_btn =
                egui::Button::new(RichText::new(s.sync_title).color(Color32::WHITE).strong())
                    .fill(theme::ORANGE_CORE)
                    .corner_radius(theme::radius::SM);
            if ui.add_enabled(enabled && has_store, sync_btn).clicked() {
                app.start_sync();
            }
            if ui
                .add_enabled(enabled && has_store, egui::Button::new(s.fetch))
                .clicked()
            {
                app.start_fetch();
            }
            if ui
                .add_enabled(enabled && has_store, egui::Button::new(s.push))
                .clicked()
            {
                app.start_push();
            }
            if ui
                .add_enabled(enabled && has_store, egui::Button::new(s.publish))
                .clicked()
            {
                app.start_publish();
            }
            if ui
                .add_enabled(enabled, egui::Button::new(s.clone))
                .clicked()
            {
                app.start_clone();
            }
            if app.busy {
                ui.spinner();
            }
        });
    });
}

fn remote_card(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    theme::card(ui, |ui| {
        theme::eyebrow(ui, s.remote_section);
        ui.label(RichText::new(s.remote_url).color(ui.visuals().weak_text_color()));
        ui.add(
            egui::TextEdit::singleline(&mut app.sync_url)
                .hint_text("https://github.com/you/store.git")
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
        ui.add_space(theme::space::SM);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(s.user).color(ui.visuals().weak_text_color()));
                ui.add(egui::TextEdit::singleline(&mut app.sync_user).desired_width(160.0));
            });
            ui.add_space(theme::space::MD);
            ui.vertical(|ui| {
                ui.label(RichText::new(s.token).color(ui.visuals().weak_text_color()));
                ui.add(
                    egui::TextEdit::singleline(&mut app.sync_token)
                        .password(true)
                        .desired_width(220.0),
                );
            });
        });
        ui.add_space(theme::space::SM);
        ui.weak(s.https_note);
    });
}

/// The blocking per-file conflict prompt. Shown while a sync is paused waiting
/// for a decision; the choice unblocks the worker thread.
pub fn conflict_window(ctx: &egui::Context, app: &mut App) {
    let Some(path) = app.pending_conflict.clone() else {
        return;
    };
    let t = i18n::s();
    let mut choice: Option<ConflictChoice> = None;
    egui::Window::new(t.conflict_title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(t.conflict_both.replace("{}", &path));
            ui.label(t.conflict_which);
            ui.add_space(theme::space::SM);
            ui.horizontal(|ui| {
                if ui.button(t.keep_mine).clicked() {
                    choice = Some(ConflictChoice::KeepLocal);
                }
                if ui.button(t.keep_remote).clicked() {
                    choice = Some(ConflictChoice::KeepRemote);
                }
                if ui.button(t.keep_both).clicked() {
                    choice = Some(ConflictChoice::KeepBoth);
                }
            });
            ui.weak(t.keep_both_note);
        });
    if let Some(c) = choice {
        app.resolve_conflict(c);
    }
}
