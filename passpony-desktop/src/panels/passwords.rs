//! The Passwords screen: pick a format, see the open store, search, and browse
//! entries in a master–detail — the list on the left, the decrypted detail (or
//! the editor) on the right. Browsing and search decrypt nothing; only opening
//! an entry does.

use egui::{Color32, Layout, RichText, Sense, Vec2};
use pass_core::store::StoreFormat;

use crate::app::App;
use crate::i18n;
use crate::theme;

pub fn screen(ui: &mut egui::Ui, app: &mut App) {
    store_head(ui, app);
    ui.add_space(theme::space::LG);
    search_bar(ui, app);
    ui.add_space(theme::space::MD);

    if !app.status.is_empty() {
        ui.label(RichText::new(&app.status).color(theme::ORANGE_DEEP));
        ui.add_space(theme::space::SM);
    }

    if app.entries.is_none() {
        empty_state(ui);
        return;
    }

    let full_h = ui.available_height();
    ui.horizontal_top(|ui| {
        let list_w = 260.0;
        ui.allocate_ui_with_layout(
            Vec2::new(list_w, full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_width(list_w);
                ui.set_max_width(list_w);
                entry_list(ui, app);
            },
        );
        ui.add_space(theme::space::LG);
        let rest = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(rest, full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(rest);
                if app.editing.is_some() {
                    super::editor::editor(ui, app);
                } else {
                    super::entry::detail(ui, app);
                }
            },
        );
    });
}

fn store_head(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    ui.horizontal(|ui| {
        let sel = match app.format {
            StoreFormat::Pass => 0,
            StoreFormat::Passage => 1,
        };
        if let Some(i) = theme::segmented(ui, &[s.seg_pass, s.seg_passage], sel) {
            app.set_format(if i == 0 {
                StoreFormat::Pass
            } else {
                StoreFormat::Passage
            });
        }

        ui.add_space(theme::space::SM);
        ui.label(RichText::new(s.store_label).color(ui.visuals().weak_text_color()));
        let path = app
            .store
            .as_ref()
            .map(|st| st.root().display().to_string())
            .unwrap_or_else(|| "—".to_string());
        ui.label(RichText::new(path).monospace().strong());

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let new_btn =
                egui::Button::new(RichText::new(s.new_entry).color(Color32::WHITE).strong())
                    .fill(theme::ORANGE_CORE)
                    .corner_radius(theme::radius::SM);
            if ui.add_enabled(app.store.is_some(), new_btn).clicked() {
                app.start_new_entry();
            }
            if ui.button(s.clone).clicked() {
                app.screen = crate::app::Screen::Sync;
            }
            if ui.button(s.open_store).clicked() {
                app.open_store_dialog();
            }
        });
    });
}

fn search_bar(ui: &mut egui::Ui, app: &mut App) {
    theme::card(ui, |ui| {
        ui.horizontal(|ui| {
            // A small painted magnifier (no search glyph in the Lucide subset).
            let (rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
            let p = ui.painter();
            let c = ui.visuals().weak_text_color();
            let center = rect.center() - Vec2::new(1.0, 1.0);
            p.circle_stroke(center, 5.0, egui::Stroke::new(1.6, c));
            p.line_segment(
                [center + Vec2::new(3.6, 3.6), center + Vec2::new(7.0, 7.0)],
                egui::Stroke::new(1.6, c),
            );
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.search)
                    .hint_text(i18n::s().search_placeholder)
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            if app.focus_search {
                resp.request_focus();
                app.focus_search = false;
            }
        });
    });
}

fn entry_list(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    let needle = app.search.to_lowercase();
    let shown: Vec<String> = app
        .entries
        .as_ref()
        .map(|es| {
            es.iter()
                .filter(|n| needle.is_empty() || n.to_lowercase().contains(&needle))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    theme::card_tight(ui, |ui| {
        // Count header.
        ui.horizontal(|ui| {
            ui.add_space(theme::space::MD);
            ui.label(
                RichText::new(s.entries_count.replace("{}", &shown.len().to_string()))
                    .color(ui.visuals().weak_text_color())
                    .size(12.0),
            );
        });
        ui.separator();

        let mut to_open: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("entries")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for name in &shown {
                    let selected = app.selected.as_deref() == Some(name.as_str());
                    if entry_row(ui, name, selected).clicked() {
                        to_open = Some(name.clone());
                    }
                }
            });
        if let Some(name) = to_open {
            app.open_entry(name);
        }
    });
}

/// One entry row: the leaf name over its full path, with a left accent and tint
/// when selected.
fn entry_row(ui: &mut egui::Ui, name: &str, selected: bool) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, theme::ORANGE_CORE.gamma_multiply(0.12));
        painter.vline(
            rect.left() + 1.0,
            rect.y_range(),
            egui::Stroke::new(2.0, theme::ORANGE_CORE),
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
    }
    let leaf = name.rsplit('/').next().unwrap_or(name);
    let x = rect.left() + 14.0;
    painter.text(
        egui::Pos2::new(x, rect.top() + 12.0),
        egui::Align2::LEFT_CENTER,
        leaf,
        egui::FontId::monospace(13.0),
        ui.visuals().text_color(),
    );
    painter.text(
        egui::Pos2::new(x, rect.bottom() - 12.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::monospace(11.0),
        ui.visuals().weak_text_color(),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn empty_state(ui: &mut egui::Ui) {
    let s = i18n::s();
    theme::card(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(theme::space::SECTION);
            ui.label(RichText::new(s.open_a_store).color(ui.visuals().weak_text_color()));
            ui.add_space(theme::space::SECTION);
        });
    });
}
