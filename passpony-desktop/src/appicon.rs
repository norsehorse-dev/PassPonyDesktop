//! The real PassPony app icon — the horse-and-keys mark on the orange gradient,
//! the same 1024px master the iOS and Android apps ship — decoded from an
//! embedded PNG. Used for the window/dock/Alt-Tab icon and for the rail head.
//!
//! This replaces the earlier vector shield mesh: the shield was AgePony's shape,
//! and PassPony's identity is the horse with keys, so the window icon now comes
//! straight from the shipped artwork rather than from re-triangulated geometry.

/// The 256px icon for the OS window/dock. Small enough to embed, large enough
/// that the compositor never has to upscale a tiny source.
const ICON_256: &[u8] = include_bytes!("../assets/icon/passpony-256.png");
/// The 128px icon painted at the top of the rail (drawn at ~42px).
const ICON_128: &[u8] = include_bytes!("../assets/icon/passpony-128.png");

/// Decode a PNG into `(rgba, width, height)`.
fn decode(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

/// The window/dock icon, ready for `ViewportBuilder::with_icon`.
#[must_use]
pub fn window_icon() -> Option<egui::IconData> {
    let (rgba, width, height) = decode(ICON_256)?;
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

/// Upload the rail-head artwork as an egui texture, once, and hand back the
/// handle. Nearest-to-linear filtering keeps the rounded corners crisp.
#[must_use]
pub fn rail_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let (rgba, width, height) = decode(ICON_128)?;
    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
    Some(ctx.load_texture("passpony-mark", image, egui::TextureOptions::LINEAR))
}
