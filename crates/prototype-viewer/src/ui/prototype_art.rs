//! PG Viewer's brand artwork and the alternate login backgrounds.
//! They decode once in the background on first use.

use egui::{Context, TextureHandle, TextureOptions};
use std::sync::OnceLock;

const BG_JPG: &[u8] = include_bytes!("../../../../assets/branding/prototype-login-bg.jpg");
const WAVE_JPG: &[u8] = include_bytes!("../../../../assets/branding/prototype-login-bg-wave.jpg");

/// The login background chosen in Preferences › Appearance (the setting
/// `prototype_login_bg`), read by `background`.
static CHOICE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_background_choice(choice: u8) {
    CHOICE.store(choice, std::sync::atomic::Ordering::Relaxed);
}

/// The user's own login picture, in the config folder.
pub fn custom_background_path() -> Option<std::path::PathBuf> {
    let dir = crate::settings::config_dir();
    ["login-background.png", "login-background.jpg", "login-background.jpeg"]
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// The chosen login background's texture (the bear until the others are
/// decoded, or when the user's own picture is missing).
pub fn background(ctx: &Context) -> Option<TextureHandle> {
    let art = match CHOICE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Art::Wave,
        2 => Art::Custom,
        3 => Art::Background,
        _ => Art::Moon,
    };
    match art {
        // PG Viewer: the skin's wallpaper (skin/background)
        Art::Moon => super::dragon_art::texture(ctx, "background"),
        _ => texture(ctx, art).or_else(|| super::dragon_art::texture(ctx, "background")),
    }
}

#[derive(Clone, Copy)]
pub enum Art {
    Moon,
    Background,
    Wordmark,
    Bear,
    Frame,
    Connect,
    PreferencesFrame,
    Wave,
    Custom,
}

impl Art {
    fn bytes(self) -> std::borrow::Cow<'static, [u8]> {
        match self {
            Art::Moon => Vec::new().into(),
            Art::Background => BG_JPG.into(),
            Art::Wordmark => Vec::new().into(),
            Art::Bear => Vec::new().into(),
            Art::Frame => Vec::new().into(),
            Art::Connect => Vec::new().into(),
            Art::PreferencesFrame => Vec::new().into(),
            Art::Wave => WAVE_JPG.into(),
            Art::Custom => custom_background_path()
                .and_then(|path| std::fs::read(path).map_err(|e| log::warn!("login background: {e}")).ok())
                .unwrap_or_default()
                .into(),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Art::Moon => "pg-viewer-background",
            Art::Background => "prototype-bg",
            Art::Wordmark => "pg-viewer-wordmark",
            Art::Bear => "pg-viewer-crest",
            Art::Frame => "pg-viewer-login-frame",
            Art::Connect => "pg-viewer-connect",
            Art::PreferencesFrame => "pg-viewer-preferences-frame",
            Art::Wave => "prototype-wave",
            Art::Custom => "prototype-custom-bg",
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}

type Decoded = [OnceLock<Option<egui::ColorImage>>; 9];
static DECODED: OnceLock<Decoded> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();

fn decoded() -> &'static Decoded {
    DECODED.get_or_init(|| std::array::from_fn(|_| OnceLock::new()))
}

/// Starts the decoding (once) and asks egui to look again shortly.
fn start(ctx: &Context) {
    if STARTED.set(()).is_ok() {
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            for art in [
                Art::Moon,
                Art::Wordmark,
                Art::Bear,
                Art::Frame,
                Art::Connect,
                Art::PreferencesFrame,
                Art::Background,
                Art::Wave,
                Art::Custom,
            ] {
                let bytes = art.bytes();
                if bytes.is_empty() {
                    let _ = decoded()[art.index()].set(None);
                    continue;
                }
                let image = match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        Some(egui::ColorImage::from_rgba_unmultiplied(
                            [rgba.width() as usize, rgba.height() as usize],
                            rgba.as_raw(),
                        ))
                    }
                    Err(e) => {
                        log::warn!("{} decode failed: {e}", art.name());
                        None
                    }
                };
                let _ = decoded()[art.index()].set(image);
                ctx.request_repaint();
            }
        });
    }
}

/// The texture of `art`, once decoded (None the first frames).
pub fn texture(ctx: &Context, art: Art) -> Option<TextureHandle> {
    let id = egui::Id::unique(("moon_brand_art", art.index()));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(t);
    }
    start(ctx);
    let image = decoded()[art.index()].get()?.clone()?;
    let t = ctx.load_texture(art.name(), image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

/// Where `paint_cover` anchors its crop (0 left/top, 1 right/bottom).
const ART_ANCHOR: (f32, f32) = (0.5, 0.0);

/// How the login wallpaper lands on `screen`: its scale, and where its
/// top-left corner lands, so the login pieces can sit where the mockup has
/// them on the art. `design` is the art's size when there is no wallpaper.
pub fn art_map(ctx: &egui::Context, screen: egui::Rect, design: egui::Vec2) -> (f32, egui::Pos2) {
    let size = super::dragon_art::texture(ctx, "background").map_or(design, |t| {
        let [w, h] = t.size();
        egui::vec2(w as f32, h as f32)
    });
    let k = (screen.width() / size.x).max(screen.height() / size.y);
    let over = egui::vec2(size.x * k - screen.width(), size.y * k - screen.height());
    (k * size.x / design.x, screen.min - egui::vec2(over.x * ART_ANCHOR.0, over.y * ART_ANCHOR.1))
}

/// Paints `t` to cover `rect` (cropped, never stretched), like CSS
/// `background-size: cover`.
pub fn paint_cover(painter: &egui::Painter, t: &TextureHandle, rect: egui::Rect, tint: egui::Color32) {
    let [w, h] = t.size();
    let (iw, ih) = (w as f32, h as f32);
    let scale = (rect.width() / iw).max(rect.height() / ih);
    let (sw, sh) = (rect.width() / (iw * scale), rect.height() / (ih * scale));
    // PG Viewer: the characters stand at both sides, their heads at the
    // top: keep the middle across and crop only from the bottom
    let u0 = (1.0 - sw).max(0.0) * ART_ANCHOR.0;
    let v0 = (1.0 - sh).max(0.0) * ART_ANCHOR.1;
    let uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u0 + sw, v0 + sh));
    painter.image(t.id(), rect, uv, tint);
}

/// The size `t` takes at `width` points wide.
pub fn size_at_width(t: &TextureHandle, width: f32) -> egui::Vec2 {
    let [w, h] = t.size();
    egui::vec2(width, width * h as f32 / w.max(1) as f32)
}
