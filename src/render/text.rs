//! Text drawn over the game (scores, timers, messages).

/// Horizontal alignment of a text relative to its position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// The text starts at the position.
    #[default]
    Left,
    /// The text is centered on the position.
    Center,
    /// The text ends at the position.
    Right,
}

/// One text drawn over the game for a frame. Positions and sizes are in the
/// camera frame's pixels (see [`CameraFrame::size`](super::CameraFrame::size)),
/// so the text scales with the game.
#[derive(Clone, Debug, PartialEq)]
pub struct HudText {
    pub text: String,
    /// Top of the text; (0, 0) is the camera frame's top-left corner.
    pub position: (f32, f32),
    /// Font height.
    pub size: f32,
    pub color: egui::Color32,
    pub align: TextAlign,
}

/// Parse a color written as `#rrggbb` or `#rrggbbaa` (hexadecimal, any
/// case; `aa` is the opacity, ff = opaque).
pub fn parse_color(text: &str) -> Result<egui::Color32, String> {
    let invalid = || {
        format!(
            "invalid color \"{}\" (expected \"#rrggbb\" or \"#rrggbbaa\")",
            text
        )
    };
    let hex = text.strip_prefix('#').ok_or_else(invalid)?;
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| invalid());
    let alpha = if hex.len() == 8 { channel(6)? } else { 255 };
    Ok(egui::Color32::from_rgba_unmultiplied(
        channel(0)?,
        channel(2)?,
        channel(4)?,
        alpha,
    ))
}

/// Paint texts; `origin` is the screen position of the camera frame's
/// top-left corner and `scale` the screen pixels per camera frame pixel.
pub fn paint_texts(painter: &egui::Painter, texts: &[HudText], origin: egui::Pos2, scale: f32) {
    for text in texts {
        let size = text.size * scale;
        // Too small to read (or NaN): skip instead of laying out
        if size.is_nan() || size < 1.0 {
            continue;
        }
        let anchor = match text.align {
            TextAlign::Left => egui::Align2::LEFT_TOP,
            TextAlign::Center => egui::Align2::CENTER_TOP,
            TextAlign::Right => egui::Align2::RIGHT_TOP,
        };
        let position = origin + egui::vec2(text.position.0 * scale, text.position.1 * scale);
        painter.text(
            position,
            anchor,
            &text.text,
            egui::FontId::proportional(size),
            text.color,
        );
    }
}
