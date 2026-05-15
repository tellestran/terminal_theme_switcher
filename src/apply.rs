use std::io::{self, Write};

use crate::theme::Theme;

const ESC: &str = "\x1b]";
const BEL: &str = "\x07";

pub fn apply_theme<W: Write>(mut writer: W, theme: &Theme) -> io::Result<()> {
    writer.write_all(theme_escape_sequence(theme).as_bytes())?;
    writer.flush()
}

pub fn reset_theme<W: Write>(mut writer: W) -> io::Result<()> {
    writer.write_all(reset_escape_sequence().as_bytes())?;
    writer.flush()
}

pub fn theme_escape_sequence(theme: &Theme) -> String {
    let mut sequence = String::new();

    for (index, color) in theme.ansi.iter().enumerate() {
        sequence.push_str(&format!("{ESC}4;{index};{color}{BEL}"));
    }

    sequence.push_str(&format!("{ESC}10;{}{BEL}", theme.foreground));
    sequence.push_str(&format!("{ESC}11;{}{BEL}", theme.background));
    sequence.push_str(&format!("{ESC}12;{}{BEL}", theme.cursor));

    sequence
}

pub fn reset_escape_sequence() -> String {
    let mut sequence = String::new();

    for index in 0..16 {
        sequence.push_str(&format!("{ESC}104;{index}{BEL}"));
    }

    sequence.push_str(&format!("{ESC}110{BEL}"));
    sequence.push_str(&format!("{ESC}111{BEL}"));
    sequence.push_str(&format!("{ESC}112{BEL}"));

    sequence
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    #[test]
    fn builds_theme_escape_sequence() {
        let sequence = theme_escape_sequence(theme::default_theme());

        assert!(sequence.contains("\x1b]4;0;#15161e\x07"));
        assert!(sequence.contains("\x1b]10;#c0caf5\x07"));
        assert!(sequence.contains("\x1b]11;#1a1b26\x07"));
        assert!(sequence.contains("\x1b]12;#c0caf5\x07"));
    }

    #[test]
    fn builds_reset_escape_sequence() {
        let sequence = reset_escape_sequence();

        assert!(sequence.contains("\x1b]104;0\x07"));
        assert!(sequence.contains("\x1b]104;15\x07"));
        assert!(sequence.contains("\x1b]110\x07"));
        assert!(sequence.contains("\x1b]111\x07"));
        assert!(sequence.contains("\x1b]112\x07"));
    }
}
