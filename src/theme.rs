#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub slug: &'static str,
    pub foreground: &'static str,
    pub background: &'static str,
    pub cursor: &'static str,
    pub selection: &'static str,
    pub ansi: [&'static str; 16],
    pub description: &'static str,
    pub author: &'static str,
    pub kind: &'static str,
    pub mood: &'static [&'static str],
}

impl Theme {
    pub fn accent(&self) -> &str {
        self.ansi[3]
    }
    pub fn dim(&self) -> &str {
        self.ansi[8]
    }
    pub fn border(&self) -> &str {
        self.ansi[8]
    }
    pub fn success(&self) -> &str {
        self.ansi[2]
    }
    pub fn warning(&self) -> &str {
        self.ansi[3]
    }
    pub fn error(&self) -> &str {
        self.ansi[1]
    }
}

pub fn themes() -> &'static [Theme] {
    &THEMES
}

pub fn default_theme() -> &'static Theme {
    &THEMES[0]
}

pub fn find_theme(query: &str) -> Option<&'static Theme> {
    let normalized = normalize(query);
    themes()
        .iter()
        .find(|theme| theme.slug == normalized || normalize(theme.name) == normalized)
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

const THEMES: [Theme; 13] = [
    Theme {
        name: "Tokyo Night",
        slug: "tokyo-night",
        foreground: "#c0caf5",
        background: "#1a1b26",
        cursor: "#c0caf5",
        selection: "#33467c",
        ansi: [
            "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
            "#414868", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c0caf5",
        ],
        description: "Clean modern dark with a cool blue cast.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "cool"],
    },
    Theme {
        name: "Catppuccin Mocha",
        slug: "catppuccin-mocha",
        foreground: "#cdd6f4",
        background: "#1e1e2e",
        cursor: "#f5e0dc",
        selection: "#45475a",
        ansi: [
            "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
            "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
        ],
        description: "Soothing pastel theme with warm undertones.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "warm"],
    },
    Theme {
        name: "Gruvbox Dark",
        slug: "gruvbox-dark",
        foreground: "#ebdbb2",
        background: "#282828",
        cursor: "#ebdbb2",
        selection: "#504945",
        ansi: [
            "#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984",
            "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#ebdbb2",
        ],
        description: "Retro warm earth tones \u{2014} high contrast and easy on eyes.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "warm"],
    },
    Theme {
        name: "Solarized Dark",
        slug: "solarized-dark",
        foreground: "#839496",
        background: "#002b36",
        cursor: "#93a1a1",
        selection: "#073642",
        ansi: [
            "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
        description: "Precision-tuned palette \u{2014} cyan-leaning dark base.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "cool"],
    },
    Theme {
        name: "Solarized Light",
        slug: "solarized-light",
        foreground: "#657b83",
        background: "#fdf6e3",
        cursor: "#586e75",
        selection: "#eee8d5",
        ansi: [
            "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
        description: "Light variant \u{2014} cream paper background.",
        author: "built-in",
        kind: "built-in",
        mood: &["light", "warm"],
    },
    Theme {
        name: "Dracula",
        slug: "dracula",
        foreground: "#f8f8f2",
        background: "#282a36",
        cursor: "#f8f8f2",
        selection: "#44475a",
        ansi: [
            "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2",
            "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df", "#a4ffff", "#ffffff",
        ],
        description: "Pastel-on-purple, popular vampire theme.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "cool"],
    },
    Theme {
        name: "Nord",
        slug: "nord",
        foreground: "#d8dee9",
        background: "#2e3440",
        cursor: "#d8dee9",
        selection: "#4c566a",
        ansi: [
            "#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0",
            "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#8fbcbb", "#eceff4",
        ],
        description: "Cold arctic minimalism \u{2014} desaturated blues and frost.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "cool"],
    },
    Theme {
        name: "One Dark",
        slug: "one-dark",
        foreground: "#abb2bf",
        background: "#282c34",
        cursor: "#528bff",
        selection: "#3e4451",
        ansi: [
            "#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
            "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff",
        ],
        description: "Atom-inspired dark theme with balanced colors.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "neutral"],
    },
    Theme {
        name: "Cedar Clay",
        slug: "cedar-clay",
        foreground: "#eadfb0",
        background: "#1d120c",
        cursor: "#e6d9a7",
        selection: "#6f3a22",
        ansi: [
            "#1d120c", "#8d341f", "#9f9a70", "#a96832", "#7c6f53", "#b8784a", "#b2a982", "#d8cc9b",
            "#4b2416", "#c4552f", "#c3bd8c", "#d28a45", "#a89775", "#dea36d", "#d9d1aa", "#f2e6b9",
        ],
        description: "Warm earthy dark \u{2014} clay reds and amber on deep umber.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "warm"],
    },
    Theme {
        name: "Mint Lagoon",
        slug: "mint-lagoon",
        foreground: "#1f725f",
        background: "#f1f1f1",
        cursor: "#2f9f87",
        selection: "#ccebdd",
        ansi: [
            "#e9e9e9", "#c75f67", "#6dcc91", "#b7b66a", "#4d91a8", "#8a7bb8", "#35a383", "#237663",
            "#cfcfcf", "#df7c83", "#83d9a4", "#d0cb80", "#68adc0", "#a694c9", "#57bea0", "#124f43",
        ],
        description: "Soft mint-on-light \u{2014} calm low-contrast green.",
        author: "built-in",
        kind: "built-in",
        mood: &["light", "cool"],
    },
    Theme {
        name: "Harbor Harvest",
        slug: "harbor-harvest",
        foreground: "#58627d",
        background: "#f4edc9",
        cursor: "#deaa62",
        selection: "#e8d086",
        ansi: [
            "#eee5b8", "#ad6659", "#8f9364", "#dfaf66", "#7d88a7", "#9b7fa2", "#6f9b99", "#5f6680",
            "#d8ca8e", "#c67a65", "#a8aa75", "#efc37a", "#929db8", "#b497b4", "#86b4ad", "#312a24",
        ],
        description: "Warm golden light theme \u{2014} harvest hues on parchment.",
        author: "built-in",
        kind: "built-in",
        mood: &["light", "warm"],
    },
    Theme {
        name: "Forest Depths",
        slug: "forest-depths",
        foreground: "#d8ead7",
        background: "#071f18",
        cursor: "#b7e4c7",
        selection: "#1f4d3d",
        ansi: [
            "#071f18", "#d46a5d", "#6fbe7d", "#d6b85f", "#5f9ea8", "#b07ab2", "#5fc7a3", "#c7d8c3",
            "#164236", "#ef8a7b", "#92d99b", "#ead37a", "#80bdc5", "#c996cb", "#82dfbf", "#eff7e8",
        ],
        description: "Deep forest dark \u{2014} emerald canopy with mossy undertones.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "cool"],
    },
    Theme {
        name: "Rose Fern",
        slug: "rose-fern",
        foreground: "#FFD786",
        background: "#237227",
        cursor: "#FFD6DD",
        selection: "#2F6E5A",
        ansi: [
            "#2A5D4D", "#F26076", "#7ED4A4", "#F2C66D", "#86B6FF", "#D99AF6", "#6DD5CF", "#F7B6C2",
            "#367A63", "#FF7F91", "#A0E2BA", "#F8D98F", "#A5C8FF", "#E6B5FA", "#8FE4DF", "#FFE6EB",
        ],
        description: "Muted rose on forest \u{2014} dusty botanical.",
        author: "built-in",
        kind: "built-in",
        mood: &["dark", "warm"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_theme_by_slug_or_name() {
        assert_eq!(find_theme("tokyo-night").unwrap().name, "Tokyo Night");
        assert_eq!(find_theme("Tokyo Night").unwrap().slug, "tokyo-night");
        assert_eq!(
            find_theme("catppuccin mocha").unwrap().slug,
            "catppuccin-mocha"
        );
        assert_eq!(find_theme("cedar clay").unwrap().slug, "cedar-clay");
        assert_eq!(find_theme("mint-lagoon").unwrap().name, "Mint Lagoon");
        assert_eq!(find_theme("Harbor Harvest").unwrap().slug, "harbor-harvest");
        assert_eq!(find_theme("forest depths").unwrap().slug, "forest-depths");
        assert_eq!(find_theme("rose fern").unwrap().slug, "rose-fern");
    }

    #[test]
    fn returns_none_for_unknown_theme() {
        assert!(find_theme("made-up-theme").is_none());
    }

    #[test]
    fn all_themes_have_metadata() {
        for t in themes() {
            assert!(!t.description.is_empty(), "{} missing description", t.name);
            assert!(!t.author.is_empty(), "{} missing author", t.name);
            assert!(!t.kind.is_empty(), "{} missing kind", t.name);
            assert!(!t.mood.is_empty(), "{} missing mood", t.name);
        }
    }
}
