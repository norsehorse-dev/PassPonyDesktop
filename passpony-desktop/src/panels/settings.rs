//! The Settings screen, mirroring AgePony Desktop's sections and links,
//! adapted for PassPony: Appearance (theme + language), Storage, Re-encrypt,
//! Back up, About, Help, Security, Licenses, Panic wipe, and the family list.
//!
//! Short labels come from `i18n::s()`; the long-form Help FAQ, Security notes and
//! license blurbs come from `i18n::long()` — all localized into the seven
//! languages (machine-assisted, native review still recommended). Only product
//! names, file extensions and the family taglines stay English by design.

use crate::app::App;
use crate::i18n::{self, Lang};
use crate::theme::{self, ThemeChoice};

/// One sibling app: name, what it does, where it runs, where it lives, and the
/// accent its own site uses. Mirrors AgePony Desktop's family list.
struct PonyApp {
    name: &'static str,
    tagline: &'static str,
    platforms: &'static str,
    url: &'static str,
    accent: egui::Color32,
}

/// The rest of the family (PassPony excluded, AgePony included), ordered by how
/// close each sits to what someone in a password-store app is already doing.
/// Names and platform strings are product and OS names, left as they are
/// everywhere else; the accents are each app's own, matching AgePony Desktop's
/// list so the two agree.
const FAMILY: &[PonyApp] = &[
    PonyApp {
        name: "PGPony",
        tagline: "OpenPGP encryption for your messages and files.",
        platforms: "iPhone · Android · macOS · Windows · Linux",
        url: "https://pgpony.app",
        accent: egui::Color32::from_rgb(0x5F, 0xFF, 0xAF),
    },
    PonyApp {
        name: "AgePony",
        tagline: "Encrypt and decrypt files with age, including post-quantum recipients.",
        platforms: "iPhone · Android · macOS · Windows · Linux",
        url: "https://agepony.com",
        accent: egui::Color32::from_rgb(0x2E, 0xB8, 0xA6),
    },
    PonyApp {
        name: "VaultPony",
        tagline: "VeraCrypt-compatible encrypted vaults, entirely on your device.",
        platforms: "Android",
        url: "https://vaultpony.app",
        accent: egui::Color32::from_rgb(0xD9, 0x53, 0x4F),
    },
    PonyApp {
        name: "QuorumPony",
        tagline: "Split a secret into cards. Any few rebuild it. One alone reveals nothing.",
        platforms: "iPhone",
        url: "https://quorumpony.com",
        accent: egui::Color32::from_rgb(0xC8, 0x97, 0x3A),
    },
    PonyApp {
        name: "ScrubPony",
        tagline: "Strips identifying metadata out of JPEGs without touching a pixel.",
        platforms: "Android · macOS · Linux",
        url: "https://scrubpony.app",
        accent: egui::Color32::from_rgb(0x9D, 0x7C, 0xF5),
    },
    PonyApp {
        name: "RelayPony",
        tagline: "Encrypted file transfer, phone to phone.",
        platforms: "iPhone · Android · macOS · Windows · Linux",
        url: "https://relaypony.app",
        accent: egui::Color32::from_rgb(0x1F, 0x9C, 0xF0),
    },
    PonyApp {
        name: "CarrierPony",
        tagline: "Private messaging and file transfer, sealed end to end.",
        platforms: "iPhone · Android",
        url: "https://carrierpony.com",
        accent: egui::Color32::from_rgb(0xF1, 0x66, 0x7B),
    },
    PonyApp {
        name: "BurnPony",
        tagline: "Send a secret. Encrypted on your phone, burned after reading.",
        platforms: "iPhone · Android",
        url: "https://burnpony.app",
        accent: egui::Color32::from_rgb(0xF6, 0x75, 0x29),
    },
];

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    crate::theme::card(ui, |ui| add(ui));
    ui.add_space(crate::theme::space::MD);
}

fn section(ui: &mut egui::Ui, title: &str) {
    crate::theme::eyebrow(ui, title);
}

pub fn screen(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    egui::ScrollArea::vertical()
        .id_salt("settings")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            super::screen_header(ui, s.settings, "");
            ui.add_space(crate::theme::space::LG);
            appearance(ui, app);
            storage(ui, app);
            reencrypt(ui, app);
            backup(ui, app);
            about(ui);
            help(ui);
            security(ui);
            licenses(ui);
            panic(ui, app);
            family(ui);
        });
}

fn appearance(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.appearance);
        ui.horizontal(|ui| {
            let mut chosen = app.theme;
            for c in ThemeChoice::ALL {
                ui.selectable_value(&mut chosen, c, c.label());
            }
            if chosen != app.theme {
                app.set_theme(chosen, ui.ctx());
            }
        });
        ui.weak(s.theme_note);
        ui.add_space(8.0);
        section(ui, s.language);
        let mut chosen = app.lang;
        egui::ComboBox::from_id_salt("lang")
            .selected_text(app.lang.native_name())
            .show_ui(ui, |ui| {
                for l in Lang::ALL {
                    ui.selectable_value(&mut chosen, l, l.native_name());
                }
            });
        if chosen != app.lang {
            app.set_language(chosen);
        }
        ui.weak(s.language_note);
    });
}

fn storage(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.storage);
        if let Some(dir) = crate::keys::config_dir() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(dir.display().to_string())
                        .font(egui::FontId::monospace(12.0)),
                )
                .wrap(),
            );
        }
        ui.weak(s.storage_note);
        ui.add_space(4.0);
        if ui.button(s.show_in_folder).clicked() {
            app.reveal_config_dir();
        }
    });
}

fn reencrypt(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.reencrypt_title);
        ui.weak(s.reencrypt_note);
        ui.add_space(4.0);
        let enabled = app.store.is_some() && !app.busy;
        if ui
            .add_enabled(enabled, egui::Button::new(s.reencrypt_run))
            .clicked()
        {
            app.start_reencrypt();
        }
    });
}

fn backup(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.backup_title);
        ui.weak(s.backup_settings_note);
        ui.add_space(4.0);
        if ui.button(s.export_keys).clicked() {
            app.export_identity();
        }
    });
}

fn about(ui: &mut egui::Ui) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.about);
        ui.label(
            egui::RichText::new(format!("PassPony Desktop {}", env!("CARGO_PKG_VERSION"))).strong(),
        );
        ui.weak(s.about_desc);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.hyperlink_to("passpony.app/desktop", "https://passpony.app/desktop");
            ui.label("·");
            ui.hyperlink_to(
                s.source_github,
                "https://github.com/norsehorse-dev/PassPonyDesktop",
            );
        });
        ui.weak(s.license_apache);
    });
}

fn help(ui: &mut egui::Ui) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.help);
        ui.add_space(4.0);
        for (q, a) in i18n::long().faq {
            egui::CollapsingHeader::new(*q).id_salt(*q).show(ui, |ui| {
                ui.weak(*a);
            });
        }
    });
}

fn security(ui: &mut egui::Ui) {
    let s = i18n::s();
    let long = i18n::long();
    card(ui, |ui| {
        section(ui, s.security_title);
        for (title, body) in long.security {
            ui.label(egui::RichText::new(*title).strong());
            ui.weak(*body);
            ui.add_space(6.0);
        }
        ui.label(egui::RichText::new(long.security_not_title).strong());
        for line in long.security_not {
            ui.weak(format!("•  {line}"));
        }
    });
}

fn licenses(ui: &mut egui::Ui) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.licenses_title);
        ui.weak(s.licenses_note);
        ui.add_space(4.0);
        for (name, lic) in i18n::long().crates {
            ui.weak(format!("{name} — {lic}"));
        }
    });
}

fn panic(ui: &mut egui::Ui, app: &mut App) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.panic_title);
        ui.weak(s.panic_desc);
        ui.add_space(4.0);
        if app.confirm_panic {
            ui.horizontal(|ui| {
                if ui.button(s.panic_confirm).clicked() {
                    app.run_panic_wipe();
                }
                if ui.button(s.cancel).clicked() {
                    app.confirm_panic = false;
                }
            });
        } else if ui.button(s.panic_button).clicked() {
            app.confirm_panic = true;
        }
    });
}

fn family(ui: &mut egui::Ui) {
    let s = i18n::s();
    card(ui, |ui| {
        section(ui, s.family_title);
        ui.add_space(theme::space::SM);
        for app_link in FAMILY {
            family_row(ui, app_link);
        }
        ui.add_space(theme::space::SM);
        ui.horizontal(|ui| {
            ui.weak(s.family_hub_label);
            ui.hyperlink_to("pony.norsehor.se", "https://pony.norsehor.se");
        });
        ui.add_space(theme::space::TIGHT);
        ui.weak(s.family_note);
    });
}

/// One row of the family list: the app's accent as a dot, its name as the link,
/// where it runs, and one sentence on what it does.
fn family_row(ui: &mut egui::Ui, app_link: &PonyApp) {
    ui.horizontal(|ui| {
        let (dot, _) = ui.allocate_exact_size(egui::Vec2::splat(9.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 4.0, app_link.accent);
        ui.hyperlink_to(app_link.name, app_link.url);
        ui.label(
            egui::RichText::new(app_link.platforms)
                .font(egui::FontId::new(10.5, egui::FontFamily::Monospace))
                .color(ui.visuals().weak_text_color()),
        );
    });
    ui.horizontal(|ui| {
        ui.add_space(9.0 + theme::space::SM);
        ui.weak(app_link.tagline);
    });
    ui.add_space(theme::space::SM);
}
