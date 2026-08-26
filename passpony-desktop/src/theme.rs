//! The PassPony design system, ported from AgePony Desktop's `theme.rs` so the
//! family reads as one set of apps: the same spacing and radius scales, the same
//! Inter / JetBrains Mono faces, the same styled surface ladder. The one
//! deliberate difference is the brand colour — PassPony's orange, sampled from
//! the iOS icon gradient (`#FF7A1E`), where AgePony is teal.
//!
//! Appearance is also the theme *choice* (Auto / Light / Dark, remembered):
//! `install_fonts` runs once at startup, and `ThemeChoice::apply` sets the
//! light/dark base and then paints the brand style on top for that base.

use crate::i18n;
use egui::{Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Vec2};
use std::sync::Arc;

// ---------------------------------------------------------------- palette ---

/// Light orange — the top of the icon gradient.
pub const ORANGE_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xB3, 0x47);
/// The brand orange, sampled from the iOS icon (`#FF7A1E`).
pub const ORANGE_CORE: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x1E);
/// Deep orange — pressed states, the bottom of the gradient.
pub const ORANGE_DEEP: Color32 = Color32::from_rgb(0xC7, 0x5A, 0x0E);
/// Near-brown orange — branded headlines on light surfaces.
pub const ORANGE_INK: Color32 = Color32::from_rgb(0x8A, 0x3D, 0x08);

/// Primary action colour.
pub const ACCENT: Color32 = ORANGE_CORE;

/// Destructive and error affordances. Magenta, not red — the family convention
/// (borrowed from PGPony): "dangerous" should read as a different mode, and a
/// warm red would blur into the orange brand range everywhere else.
pub const DANGER: Color32 = Color32::from_rgb(0xC0, 0x46, 0x8C);

/// Positive / success affordance (a healthy sync, a present key). Green, well
/// away from both the orange brand and the magenta danger.
pub const GOOD: Color32 = Color32::from_rgb(0x2E, 0x9E, 0x6B);

// -------------------------------------------------------------------- rail ---

/// The navigation rail's fixed width, matching the mockup and AgePony.
pub const RAIL_WIDTH: f32 = 112.0;

/// The rail is a dark "chrome" in both light and dark themes, like a title bar,
/// so the four destinations read the same whatever the content theme.
pub const RAIL_BG: Color32 = Color32::from_rgb(0x17, 0x13, 0x0E);
/// Rail text/icon at rest-bright (hover, mark name).
pub const RAIL_INK: Color32 = Color32::from_rgb(0xEB, 0xE2, 0xD6);
/// Rail text/icon at rest-dim (an unselected destination).
pub const RAIL_MUTED: Color32 = Color32::from_rgb(0x9C, 0x8E, 0x7C);

// ------------------------------------------------------------------ scale ---

/// The spacing scale. Every gap is one of these, matching the family rhythm.
pub mod space {
    pub const TIGHT: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const SECTION: f32 = 24.0;
    pub const SCREEN: f32 = 32.0;
}

/// The corner-radius scale, chosen by how big the thing is.
pub mod radius {
    pub const SM: u8 = 6;
    pub const MD: u8 = 12;
    pub const LG: u8 = 18;
}

// ------------------------------------------------------------------ fonts ---

pub const UI_FONT: &str = "Inter";
pub const UI_FONT_SEMIBOLD: &str = "InterSemiBold";
pub const MONO_FONT: &str = "JetBrainsMono";
/// The Lucide icon font (subset), the same glyph set the phones and AgePony use,
/// so the desktop's icons are the real Lucide shapes rather than hand-drawn
/// approximations. See `icon` for the code points in the subset.
pub const ICON_FONT: &str = "Lucide";
/// Noto Sans SC (subset) — the Simplified Chinese face. Inter's subset carries
/// no Han, and egui's built-in fallback (Ubuntu-Light) covers Latin and Cyrillic
/// but not CJK, so without this every Chinese glyph renders as a tofu box. The
/// subset holds exactly the characters the `zh-Hans` catalog draws; the test at
/// the foot of this file fails if a Chinese string uses a character it lacks.
pub const CJK_FONT: &str = "NotoSansSC";

/// Embed Inter, JetBrains Mono, the Lucide icon font, and the Noto Sans SC subset
/// (all subset). egui's own fonts stay behind as the fallback chain, so glyphs
/// outside the subsets still render. Call once.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (
            UI_FONT,
            &include_bytes!("../assets/fonts/Inter-Regular-subset.ttf")[..],
        ),
        (
            UI_FONT_SEMIBOLD,
            &include_bytes!("../assets/fonts/Inter-SemiBold-subset.ttf")[..],
        ),
        (
            MONO_FONT,
            &include_bytes!("../assets/fonts/JetBrainsMono-Regular-subset.ttf")[..],
        ),
        (
            ICON_FONT,
            &include_bytes!("../assets/fonts/Lucide-subset.ttf")[..],
        ),
        (
            CJK_FONT,
            &include_bytes!("../assets/fonts/NotoSansSC-subset.ttf")[..],
        ),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    if let Some(p) = fonts.families.get_mut(&FontFamily::Proportional) {
        p.insert(0, UI_FONT.to_owned());
        // CJK last: Latin comes from Inter, Han falls through to Noto.
        p.push(CJK_FONT.to_owned());
    }
    if let Some(m) = fonts.families.get_mut(&FontFamily::Monospace) {
        m.insert(0, MONO_FONT.to_owned());
        m.push(CJK_FONT.to_owned());
    }
    fonts.families.insert(
        FontFamily::Name(UI_FONT_SEMIBOLD.into()),
        // Chinese headings have no bold cut in the subset; the regular Noto face
        // is the graceful fallback, and reads far better than a box.
        vec![
            UI_FONT_SEMIBOLD.to_owned(),
            UI_FONT.to_owned(),
            CJK_FONT.to_owned(),
        ],
    );
    // The icon font is its own family so nothing else falls into it, and so a
    // missing glyph shows as tofu (a bug we can see) rather than a wrong letter.
    fonts.families.insert(
        FontFamily::Name(ICON_FONT.into()),
        vec![ICON_FONT.to_owned()],
    );
    ctx.set_fonts(fonts);
}

/// A `FontId` in the semibold face at `size`.
#[must_use]
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(UI_FONT_SEMIBOLD.into()))
}

/// A `FontId` in the Lucide icon face at `size`.
#[must_use]
pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICON_FONT.into()))
}

/// The Lucide code points in the bundled subset, by role. Only glyphs actually
/// present in `Lucide-subset.ttf` are listed; anything else would render as
/// tofu. The sync/refresh mark is painted (see [`paint_refresh`]) because the
/// subset carries no refresh glyph.
pub mod icon {
    pub const LOCK: char = '\u{e10b}';
    pub const KEY: char = '\u{e4a3}'; // key-round
    pub const GEAR: char = '\u{e154}'; // settings
    pub const COPY: char = '\u{e09e}';
    pub const EDIT: char = '\u{e1f9}'; // pencil
    pub const TRASH: char = '\u{e18e}';
    pub const PLUS: char = '\u{e13d}';
    pub const CHECK: char = '\u{e06c}';
    pub const DOWNLOAD: char = '\u{e0b2}';
    pub const UPLOAD: char = '\u{e19e}';
    pub const FILE_LOCK: char = '\u{e31e}';
}

/// Paint the brand style (text scale, spacing, radius, tinted surfaces, orange
/// accent) onto both the light and dark variants; the per-variant surface
/// colours branch on `dark_mode`.
pub fn install_style(ctx: &Context) {
    use egui::{FontFamily::Proportional, TextStyle};
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, semibold(19.0)),
            (TextStyle::Body, FontId::new(14.0, Proportional)),
            (TextStyle::Button, FontId::new(14.0, Proportional)),
            (TextStyle::Small, FontId::new(12.0, Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(12.5, FontFamily::Monospace),
            ),
        ]
        .into();

        style.spacing.item_spacing = Vec2::new(8.0, 7.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.menu_margin = 8.into();

        let v = &mut style.visuals;
        if v.dark_mode {
            v.panel_fill = Color32::from_rgb(0x14, 0x11, 0x0E);
            v.window_fill = Color32::from_rgb(0x1B, 0x17, 0x12);
            v.extreme_bg_color = Color32::from_rgb(0x0E, 0x0B, 0x08);
            v.faint_bg_color = Color32::from_rgb(0x22, 0x1C, 0x15);
            v.window_stroke.color = Color32::from_rgb(0x3A, 0x32, 0x2A);
        } else {
            v.panel_fill = Color32::from_rgb(0xF6, 0xF2, 0xEC);
            v.window_fill = Color32::from_rgb(0xFB, 0xF8, 0xF3);
            v.extreme_bg_color = Color32::from_rgb(0xFF, 0xFF, 0xFF);
            v.faint_bg_color = Color32::from_rgb(0xEF, 0xE8, 0xDE);
            v.window_stroke.color = Color32::from_rgb(0xD8, 0xCD, 0xBE);
        }

        v.hyperlink_color = ORANGE_CORE;
        v.selection.bg_fill = ORANGE_CORE.linear_multiply(0.35);
        v.selection.stroke.color = ORANGE_CORE;
        v.widgets.hovered.bg_stroke.color = ORANGE_CORE;
        v.widgets.active.bg_stroke.color = ORANGE_DEEP;
        v.widgets.open.bg_stroke.color = ORANGE_DEEP;
        v.error_fg_color = DANGER;

        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = CornerRadius::same(radius::SM);
        }
        v.widgets.inactive.bg_stroke = egui::Stroke::new(
            1.5,
            if v.dark_mode {
                Color32::from_rgb(0x3A, 0x32, 0x2A)
            } else {
                Color32::from_rgb(0xD8, 0xCD, 0xBE)
            },
        );
    });
}

// --------------------------------------------------------- theme choice ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Auto,
    Light,
    Dark,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::Auto, ThemeChoice::Light, ThemeChoice::Dark];

    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            ThemeChoice::Auto => "auto",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> ThemeChoice {
        match code {
            "light" => ThemeChoice::Light,
            "dark" => ThemeChoice::Dark,
            _ => ThemeChoice::Auto,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        let s = i18n::s();
        match self {
            ThemeChoice::Auto => s.theme_auto,
            ThemeChoice::Light => s.theme_light,
            ThemeChoice::Dark => s.theme_dark,
        }
    }

    /// Set the light/dark base, then paint the brand style over it.
    pub fn apply(self, ctx: &Context) {
        match self {
            ThemeChoice::Auto => ctx.set_theme(egui::ThemePreference::System),
            ThemeChoice::Light => ctx.set_theme(egui::ThemePreference::Light),
            ThemeChoice::Dark => ctx.set_theme(egui::ThemePreference::Dark),
        }
        install_style(ctx);
    }
}

// --------------------------------------------------------------- widgets ---
//
// The shared shapes the rail shell is built from — the rail head and its four
// destinations, the surface card, the accent buttons, and the segmented pill —
// ported from AgePony Desktop and recoloured to PassPony's orange. They paint
// directly so the look is identical across platforms and does not depend on the
// host widget theme.

use egui::{Align2, Pos2, Response, Sense, Stroke, Ui};

/// Card fill for the current base (matches the mockup `--card`).
#[must_use]
pub fn card_fill(ui: &Ui) -> Color32 {
    ui.visuals().window_fill
}

/// Paint a Lucide glyph centred at `center` in `color`.
pub fn paint_glyph(painter: &egui::Painter, center: Pos2, color: Color32, glyph: char, size: f32) {
    painter.text(center, Align2::CENTER_CENTER, glyph, icon_font(size), color);
}

/// Paint a two-arrow refresh/sync mark centred at `center`. Hand-drawn because
/// the Lucide subset carries no refresh glyph; two ~300° arcs with arrowheads.
pub fn paint_refresh(painter: &egui::Painter, center: Pos2, color: Color32, size: f32) {
    let r = size * 0.42;
    let stroke = Stroke::new((size * 0.09).max(1.6), color);
    use std::f32::consts::PI;
    // Top arc sweeps clockwise; bottom arc mirrors it — together they read as
    // the familiar two-arrow refresh loop.
    for flip in [false, true] {
        let (a0, a1) = if flip {
            (PI * 0.15, PI * 1.15)
        } else {
            (PI * 1.15, PI * 2.15)
        };
        let steps = 24;
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let t = a0 + (a1 - a0) * (i as f32 / steps as f32);
                Pos2::new(center.x + r * t.cos(), center.y + r * t.sin())
            })
            .collect();
        // Arrowhead at the arc's end.
        let tip = *pts.last().expect("non-empty");
        let dir = a1;
        let head = size * 0.26;
        let left = Pos2::new(
            tip.x + head * (dir + 2.4).cos(),
            tip.y + head * (dir + 2.4).sin(),
        );
        let right = Pos2::new(
            tip.x + head * (dir - 2.4).cos(),
            tip.y + head * (dir - 2.4).sin(),
        );
        painter.add(egui::Shape::line(pts, stroke));
        painter.add(egui::Shape::line(vec![left, tip, right], stroke));
    }
}

/// The rail head: the app mark (real artwork) over the "PassPony" wordmark.
pub fn rail_head(ui: &mut Ui, texture: Option<&egui::TextureHandle>) {
    ui.add_space(space::SM);
    ui.vertical_centered(|ui| {
        if let Some(tex) = texture {
            let img = egui::Image::new(tex)
                .fit_to_exact_size(egui::vec2(42.0, 42.0))
                .corner_radius(11);
            ui.add(img);
        }
        ui.add_space(space::SM);
        ui.label(
            egui::RichText::new("PassPony")
                .color(RAIL_INK)
                .font(semibold(13.0)),
        );
    });
    ui.add_space(space::MD);
}

/// One rail destination: an icon over a small label, full rail width. Selected
/// paints the orange gradient with white content; hover lifts a dim item toward
/// the bright ink. `paint_icon` draws the icon centred at the point it is given,
/// so callers can use a Lucide glyph or a painted mark (Sync).
pub fn rail_item(
    ui: &mut Ui,
    label: &str,
    selected: bool,
    paint_icon: impl FnOnce(&egui::Painter, Pos2, Color32),
) -> Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 54.0), Sense::click());
    let painter = ui.painter();
    let hovered = resp.hovered();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(radius::MD), ORANGE_CORE);
    } else if hovered {
        painter.rect_filled(
            rect,
            CornerRadius::same(radius::MD),
            Color32::from_white_alpha(13),
        );
    }
    let color = if selected {
        Color32::WHITE
    } else if hovered {
        RAIL_INK
    } else {
        RAIL_MUTED
    };
    let icon_center = Pos2::new(rect.center().x, rect.top() + 19.0);
    paint_icon(painter, icon_center, color);
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 12.0),
        Align2::CENTER_CENTER,
        label,
        FontId::new(11.5, FontFamily::Proportional),
        color,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small round icon button for the rail foot (e.g. the theme toggle).
pub fn rail_icon_button(
    ui: &mut Ui,
    paint_icon: impl FnOnce(&egui::Painter, Pos2, Color32),
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::click());
    let painter = ui.painter();
    let hovered = resp.hovered();
    painter.rect_filled(
        rect,
        CornerRadius::same(radius::SM),
        Color32::from_white_alpha(if hovered { 24 } else { 15 }),
    );
    let color = if hovered { RAIL_INK } else { RAIL_MUTED };
    paint_icon(painter, rect.center(), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A surface card: the window-fill panel with a hairline border and inner
/// padding that the screens group related controls into.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(card_fill(ui))
        .stroke(ui.visuals().window_stroke)
        .corner_radius(CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A surface card with no inner padding — for content that manages its own
/// edges, like the entry list (count header, separator, flush rows).
pub fn card_tight<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(card_fill(ui))
        .stroke(ui.visuals().window_stroke)
        .corner_radius(CornerRadius::same(radius::MD))
        .inner_margin(egui::Margin::same(0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// An uppercase, letter-spaced section eyebrow, the small label heading a card.
pub fn eyebrow(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .color(ui.visuals().weak_text_color())
            .extra_letter_spacing(0.8)
            .font(semibold(11.0)),
    );
    ui.add_space(space::SM);
}

/// The primary action button: solid orange, white label.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let label = egui::RichText::new(text).color(Color32::WHITE).strong();
    ui.add(
        egui::Button::new(label)
            .fill(ORANGE_CORE)
            .corner_radius(radius::SM),
    )
}

/// The destructive button: magenta danger, white label.
pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let label = egui::RichText::new(text).color(Color32::WHITE).strong();
    ui.add(
        egui::Button::new(label)
            .fill(DANGER)
            .corner_radius(radius::SM),
    )
}

/// A segmented pill (like the mockup's pass/passage and Auto/Light/Dark). Draws
/// each option as a selectable chip; returns the index clicked this frame.
#[must_use]
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    egui::Frame::default()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().window_stroke)
        .corner_radius(CornerRadius::same(radius::SM))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, lab) in labels.iter().enumerate() {
                    let on = i == selected;
                    let text = egui::RichText::new(*lab).strong().size(13.0).color(if on {
                        ui.visuals().strong_text_color()
                    } else {
                        ui.visuals().weak_text_color()
                    });
                    let fill = if on {
                        card_fill(ui)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let btn = egui::Button::new(text)
                        .fill(fill)
                        .stroke(Stroke::NONE)
                        .corner_radius(radius::SM)
                        .min_size(egui::vec2(0.0, 24.0));
                    if ui.add(btn).clicked() {
                        clicked = Some(i);
                    }
                }
            });
        });
    clicked
}

#[cfg(test)]
mod font_coverage {
    /// Every Simplified-Chinese character the UI catalog draws must be present in
    /// the bundled Noto Sans SC subset. A missing glyph does not fail loudly — it
    /// renders as an empty tofu box that survives all the way to a screenshot
    /// (which is exactly how the whole Chinese UI once shipped). The source of
    /// truth is the `zh-Hans` table plus the language picker's own native name;
    /// regenerate the subset with `tools/make-fonts.py` when this list grows.
    #[test]
    fn noto_subset_covers_every_chinese_character() {
        // Both the short-string table and the long-form Settings prose carry
        // Chinese; a helper pulls a named `const` block's body out of a source.
        fn block<'a>(src: &'a str, marker: &str) -> &'a str {
            let start = src.find(marker).expect("the const block exists");
            let rest = &src[start..];
            let end = rest.find("\n};").expect("the block is terminated");
            &rest[..end]
        }

        let mut chars: Vec<char> = Vec::new();
        chars.extend(
            block(
                include_str!("i18n_data.rs"),
                "pub const ZH: Strings = Strings {",
            )
            .chars(),
        );
        chars.extend(
            block(
                include_str!("i18n_help.rs"),
                "pub const ZH_LONG: LongForm = LongForm {",
            )
            .chars(),
        );
        chars.extend("简体中文".chars()); // Lang::Zh::native_name, from i18n.rs
        chars.retain(|&c| c as u32 > 0x2E7F);
        chars.sort_unstable();
        chars.dedup();

        let bytes = include_bytes!("../assets/fonts/NotoSansSC-subset.ttf");
        let face = ttf_parser::Face::parse(bytes, 0).expect("the subset is a valid font");
        let missing: Vec<char> = chars
            .iter()
            .copied()
            .filter(|&c| face.glyph_index(c).is_none())
            .collect();

        assert!(
            missing.is_empty(),
            "NotoSansSC-subset.ttf is missing {} Chinese glyph(s): {missing:?}\n\
             regenerate it with tools/make-fonts.py",
            missing.len(),
        );
    }
}
