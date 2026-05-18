use ratatui::style::Color;

pub fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::Reset;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or_default();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or_default();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or_default();
    Color::Rgb(r, g, b)
}

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let value = normalize_hex(hex);
    (
        u8::from_str_radix(&value[1..3], 16).unwrap_or_default(),
        u8::from_str_radix(&value[3..5], 16).unwrap_or_default(),
        u8::from_str_radix(&value[5..7], 16).unwrap_or_default(),
    )
}

pub fn rgb_to_hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn normalize_hex(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('#') {
        trimmed.to_ascii_uppercase()
    } else {
        format!("#{}", trimmed.to_ascii_uppercase())
    }
}

pub fn is_valid_hex(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
}

pub fn mix(a: (u8, u8, u8), b: (u8, u8, u8), ratio_b: f32) -> (u8, u8, u8) {
    let ratio_a = 1.0 - ratio_b;
    (
        (a.0 as f32 * ratio_a + b.0 as f32 * ratio_b).round() as u8,
        (a.1 as f32 * ratio_a + b.1 as f32 * ratio_b).round() as u8,
        (a.2 as f32 * ratio_a + b.2 as f32 * ratio_b).round() as u8,
    )
}

pub fn lighten(color: (u8, u8, u8), amount: f32) -> (u8, u8, u8) {
    mix(color, (255, 255, 255), amount)
}

pub fn scale(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    (
        (color.0 as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.1 as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.2 as f32 * factor).round().clamp(0.0, 255.0) as u8,
    )
}

pub fn normalize_color_input(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn parse_color_input(value: &str) -> Option<String> {
    let named = match value {
        "black" => Some("#000000"),
        "red" => Some("#FF0000"),
        "green" => Some("#00FF00"),
        "yellow" => Some("#FFFF00"),
        "blue" => Some("#0000FF"),
        "magenta" => Some("#FF00FF"),
        "cyan" => Some("#00FFFF"),
        "white" => Some("#FFFFFF"),
        "gray" | "grey" => Some("#808080"),
        "orange" => Some("#FFA500"),
        "purple" => Some("#800080"),
        "pink" => Some("#FFC0CB"),
        _ => None,
    };
    if let Some(color) = named {
        return Some(color.to_string());
    }
    if let Ok(index) = value.parse::<u8>() {
        return Some(ansi256_to_hex(index));
    }
    let hex_candidate = normalize_hex(value);
    if is_valid_hex(&hex_candidate) {
        return Some(hex_candidate);
    }
    None
}

pub fn ansi256_to_hex(index: u8) -> String {
    let (r, g, b) = if index < 16 {
        let base = [
            (0, 0, 0), (128, 0, 0), (0, 128, 0), (128, 128, 0),
            (0, 0, 128), (128, 0, 128), (0, 128, 128), (192, 192, 192),
            (128, 128, 128), (255, 0, 0), (0, 255, 0), (255, 255, 0),
            (0, 0, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255),
        ];
        base[index as usize]
    } else if index <= 231 {
        let i = index - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;
        let s = [0u8, 95, 135, 175, 215, 255];
        (s[r as usize], s[g as usize], s[b as usize])
    } else {
        let level = 8 + (index - 232) * 10;
        (level, level, level)
    };
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

pub fn derive_chrome(bg_hex: &str, fg_hex: &str) -> ChromeColors {
    let (r, g, b) = hex_to_rgb(bg_hex);
    let bg = Color::Rgb(r, g, b);
    let panel_bg = Color::Rgb(
        ((r as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
        ((g as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
        ((b as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
    );
    let (fr, fg_val, fb) = hex_to_rgb(fg_hex);
    let fg = Color::Rgb(fr, fg_val, fb);
    let muted = Color::Rgb(
        ((fr as f32) * 0.7 + (r as f32) * 0.3).round().clamp(0.0, 255.0) as u8,
        ((fg_val as f32) * 0.7 + (g as f32) * 0.3).round().clamp(0.0, 255.0) as u8,
        ((fb as f32) * 0.7 + (b as f32) * 0.3).round().clamp(0.0, 255.0) as u8,
    );
    let border = Color::Rgb(
        ((fr as f32) * 0.4 + (r as f32) * 0.6).round().clamp(0.0, 255.0) as u8,
        ((fg_val as f32) * 0.4 + (g as f32) * 0.6).round().clamp(0.0, 255.0) as u8,
        ((fb as f32) * 0.4 + (b as f32) * 0.6).round().clamp(0.0, 255.0) as u8,
    );
    ChromeColors { bg, panel_bg, fg, muted, border, accent: fg }
}

#[derive(Clone, Copy)]
pub struct ChromeColors {
    pub bg: Color,
    pub panel_bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
}
