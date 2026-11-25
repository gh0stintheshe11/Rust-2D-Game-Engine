//! What scripts draw over the game for a frame: text, rectangles (panels,
//! bars), images (icons) and buttons, laid out in the camera frame's pixels
//! (see [`CameraFrame::size`](super::CameraFrame::size)) so they scale with
//! the game.

use super::text::{HudText, paint_text};
use std::path::{Path, PathBuf};

/// One thing drawn over the game, in the order scripts drew them: later
/// ones cover earlier ones.
#[derive(Clone, Debug, PartialEq)]
pub enum HudItem {
    Text(HudText),
    Rect(HudRect),
    Image(HudImage),
    Button(HudButton),
}

/// A filled rectangle, optionally rounded and outlined.
#[derive(Clone, Debug, PartialEq)]
pub struct HudRect {
    /// Top-left corner; (0, 0) is the camera frame's top-left corner.
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: egui::Color32,
    /// Corner radius.
    pub radius: f32,
    /// Outline color and width (0 for none), drawn inside the rectangle.
    pub outline: (egui::Color32, f32),
}

/// An image file, drawn whole.
#[derive(Clone, Debug, PartialEq)]
pub struct HudImage {
    pub path: PathBuf,
    /// Top-left corner.
    pub position: (f32, f32),
    /// Width and height; a missing one follows from the other and the
    /// image's proportions, and both missing make the image's own size
    /// (one image pixel per camera frame pixel).
    pub size: (Option<f32>, Option<f32>),
    /// Color the image is multiplied by (premultiplied); white shows it as
    /// is.
    pub tint: egui::Color32,
}

/// A button as drawn this frame: a rectangle with a label centered in it,
/// colored for its state (normal, hovered, pressed) by whoever made it.
#[derive(Clone, Debug, PartialEq)]
pub struct HudButton {
    pub label: String,
    /// Top-left corner.
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// Font height of the label.
    pub text_size: f32,
    pub text_color: egui::Color32,
    pub background: egui::Color32,
    pub radius: f32,
    /// Outline color and width (0 for none).
    pub outline: (egui::Color32, f32),
}

impl HudButton {
    /// Whether a point (in camera frame pixels) is on the button.
    pub fn contains(&self, point: (f32, f32)) -> bool {
        let (x, y) = self.position;
        let (width, height) = self.size;
        point.0 >= x && point.0 <= x + width && point.1 >= y && point.1 <= y + height
    }
}

/// Paint what scripts drew; `origin` is the screen position of the camera
/// frame's top-left corner and `scale` the screen pixels per camera frame
/// pixel. `image` gives the GPU texture of an image file and its size in
/// pixels (`None` skips the image).
pub fn paint_hud(
    painter: &egui::Painter,
    items: &[HudItem],
    origin: egui::Pos2,
    scale: f32,
    mut image: impl FnMut(&Path) -> Option<(egui::TextureId, (u32, u32))>,
) {
    let rect = |position: (f32, f32), size: (f32, f32)| {
        egui::Rect::from_min_size(
            origin + egui::vec2(position.0 * scale, position.1 * scale),
            egui::vec2(size.0 * scale, size.1 * scale),
        )
    };
    for item in items {
        match item {
            HudItem::Text(text) => paint_text(painter, text, origin, scale),
            HudItem::Rect(shape) => {
                painter.rect(
                    rect(shape.position, shape.size),
                    shape.radius * scale,
                    shape.color,
                    egui::Stroke::new(shape.outline.1 * scale, shape.outline.0),
                    egui::StrokeKind::Inside,
                );
            }
            HudItem::Image(picture) => {
                let Some((texture, (width, height))) = image(&picture.path) else {
                    continue;
                };
                let (width, height) = (width.max(1) as f32, height.max(1) as f32);
                let size = match picture.size {
                    (Some(w), Some(h)) => (w, h),
                    (Some(w), None) => (w, w * height / width),
                    (None, Some(h)) => (h * width / height, h),
                    (None, None) => (width, height),
                };
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                painter.image(texture, rect(picture.position, size), uv, picture.tint);
            }
            HudItem::Button(button) => {
                let area = rect(button.position, button.size);
                painter.rect(
                    area,
                    button.radius * scale,
                    button.background,
                    egui::Stroke::new(button.outline.1 * scale, button.outline.0),
                    egui::StrokeKind::Inside,
                );
                let size = button.text_size * scale;
                if size >= 1.0 {
                    painter.text(
                        area.center(),
                        egui::Align2::CENTER_CENTER,
                        &button.label,
                        egui::FontId::proportional(size),
                        button.text_color,
                    );
                }
            }
        }
    }
}
