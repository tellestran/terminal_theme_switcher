pub fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
    Some((r, g, b))
}

fn linearize(c: f64) -> f64 {
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn relative_luminance(hex: &str) -> f64 {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((0.0, 0.0, 0.0));
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

pub fn contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    let l1 = relative_luminance(hex1);
    let l2 = relative_luminance(hex2);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn contrast_grade(ratio: f64) -> &'static str {
    if ratio >= 7.0 {
        "AAA"
    } else if ratio >= 4.5 {
        "AA"
    } else if ratio >= 3.0 {
        "AA-lg"
    } else {
        "fail"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        let r = contrast_ratio("#000000", "#FFFFFF");
        assert!((r - 21.0).abs() < 0.01);
    }

    #[test]
    fn same_color_is_1() {
        let r = contrast_ratio("#445566", "#445566");
        assert!((r - 1.0).abs() < 0.01);
    }

    #[test]
    fn grades() {
        assert_eq!(contrast_grade(8.0), "AAA");
        assert_eq!(contrast_grade(5.0), "AA");
        assert_eq!(contrast_grade(3.5), "AA-lg");
        assert_eq!(contrast_grade(2.0), "fail");
    }

    #[test]
    fn tokyo_night_fg_bg() {
        let r = contrast_ratio("#1a1b26", "#c0caf5");
        assert!(r > 9.0);
        assert_eq!(contrast_grade(r), "AAA");
    }
}
