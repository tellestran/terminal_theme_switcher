mod colors;
mod creator;
mod home;
mod library;
mod widgets;

use std::{
    io::{self, IsTerminal},
    process::Command,
    time::Duration,
};

use anyhow::{bail, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
    Frame, Terminal,
};

use crate::{apply, config};
use colors::derive_chrome;

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    name: String,
    slug: String,
    foreground: String,
    background: String,
    cursor: String,
    selection: String,
    ansi: [String; 16],
    source: ThemeSource,
    description: String,
    author: String,
    kind: String,
    mood: Vec<String>,
    accent: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
}

impl ThemeEntry {
    fn from_resolved(theme: config::ResolvedTheme) -> Self {
        let source = match theme.source {
            config::ResolvedThemeSource::BuiltIn => ThemeSource::BuiltIn,
            config::ResolvedThemeSource::BuiltInOverride => ThemeSource::BuiltInOverride,
            config::ResolvedThemeSource::Custom => ThemeSource::Custom,
        };
        Self {
            name: theme.name,
            slug: theme.slug,
            foreground: theme.foreground,
            background: theme.background,
            cursor: theme.cursor,
            selection: theme.selection,
            ansi: theme.ansi,
            source,
            description: theme.description,
            author: theme.author,
            kind: theme.kind,
            mood: theme.mood,
            accent: theme.accent,
            success: theme.success,
            warning: theme.warning,
            error: theme.error,
        }
    }

    fn to_custom_theme(&self) -> config::CustomTheme {
        config::CustomTheme {
            name: self.name.clone(),
            slug: self.slug.clone(),
            foreground: self.foreground.clone(),
            background: self.background.clone(),
            cursor: self.cursor.clone(),
            selection: self.selection.clone(),
            ansi: self.ansi.clone(),
            description: self.description.clone(),
            author: self.author.clone(),
            kind: self.kind.clone(),
            mood: self.mood.clone(),
            accent: self.accent.clone(),
            success: self.success.clone(),
            warning: self.warning.clone(),
            error: self.error.clone(),
        }
    }

    pub fn success_color(&self) -> &str {
        self.success.as_deref().unwrap_or(&self.ansi[2])
    }
    pub fn warning_color(&self) -> &str {
        self.warning.as_deref().unwrap_or(&self.ansi[3])
    }
    pub fn error_color(&self) -> &str {
        self.error.as_deref().unwrap_or(&self.ansi[1])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeSource {
    BuiltIn,
    BuiltInOverride,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Home,
    Library,
    Creator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorState {
    name: String,
    mood: Vec<String>,
    background: String,
    foreground: String,
    cursor: String,
    selection: String,
    accent: String,
    success: String,
    warning: String,
    error: String,
    ansi_auto: bool,
    ansi: [String; 16],
    error_msg: Option<String>,
}

impl Default for CreatorState {
    fn default() -> Self {
        Self {
            name: "my-theme".to_string(),
            mood: vec!["dark".to_string()],
            background: "#1A1B26".to_string(),
            foreground: "#C0CAF5".to_string(),
            cursor: "#C0CAF5".to_string(),
            selection: "#33467C".to_string(),
            accent: "#E0AF68".to_string(),
            success: "#9ECE6A".to_string(),
            warning: "#E0AF68".to_string(),
            error: "#F7768E".to_string(),
            ansi_auto: true,
            ansi: std::array::from_fn(|_| "#000000".to_string()),
            error_msg: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorFocus {
    Name,
    Mood,
    Background,
    Foreground,
    Cursor,
    Selection,
    Accent,
    Success,
    Warning,
    Error,
    AnsiToggle,
}

impl CreatorFocus {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Mood,
            Self::Mood => Self::Background,
            Self::Background => Self::Foreground,
            Self::Foreground => Self::Cursor,
            Self::Cursor => Self::Selection,
            Self::Selection => Self::Accent,
            Self::Accent => Self::Success,
            Self::Success => Self::Warning,
            Self::Warning => Self::Error,
            Self::Error => Self::AnsiToggle,
            Self::AnsiToggle => Self::Name,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Name => Self::AnsiToggle,
            Self::Mood => Self::Name,
            Self::Background => Self::Mood,
            Self::Foreground => Self::Background,
            Self::Cursor => Self::Foreground,
            Self::Selection => Self::Cursor,
            Self::Accent => Self::Selection,
            Self::Success => Self::Accent,
            Self::Warning => Self::Success,
            Self::Error => Self::Warning,
            Self::AnsiToggle => Self::Error,
        }
    }
}

pub(crate) struct App {
    mode: AppMode,
    home_selected: usize,
    themes: Vec<ThemeEntry>,
    selected: usize,
    status: String,
    branch: String,
    list_state: ratatui::widgets::ListState,
    creator: CreatorState,
    creator_focus: CreatorFocus,
    editing_slug: Option<String>,
    pending_delete_slug: Option<String>,
    list_area: Rect,
    lib_search_query: String,
    lib_search_active: bool,
    lib_filter: library::LibFilter,
    lib_filtered_indices: Vec<usize>,
}

impl App {
    fn active_theme(&self) -> &ThemeEntry {
        self.themes.get(self.active_idx()).unwrap_or(&self.themes[0])
    }

    fn active_idx(&self) -> usize {
        self.selected
    }

    fn focused_library_theme(&self) -> Option<ThemeEntry> {
        self.themes.get(self.selected).cloned()
    }

    fn reload_themes(&mut self) -> Result<()> {
        let saved = config::Config::load().unwrap_or_default();
        self.themes = build_theme_entries(&saved);
        self.selected = self
            .themes
            .iter()
            .position(|t| t.slug == saved.theme)
            .unwrap_or(0);
        library::recompute_filtered(self);
        Ok(())
    }
}

pub fn run() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!(
            "interactive mode requires a TTY. Run `switch-theme list` to view themes or start `switch-theme` directly in your terminal."
        );
    }

    let mut terminal = setup_terminal()?;
    let outcome = run_app(&mut terminal);
    restore_terminal(&mut terminal)?;
    outcome
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let saved = config::Config::load().unwrap_or_default();
    let themes = build_theme_entries(&saved);
    let selected = themes
        .iter()
        .position(|t| t.slug == saved.theme)
        .unwrap_or(0);

    let mut app = App {
        mode: AppMode::Home,
        home_selected: 0,
        themes,
        selected,
        status: "Ready.".to_string(),
        branch: current_git_branch(),
        list_state: ratatui::widgets::ListState::default(),
        creator: CreatorState::default(),
        creator_focus: CreatorFocus::Name,
        editing_slug: None,
        pending_delete_slug: None,
        list_area: Rect::default(),
        lib_search_query: String::new(),
        lib_search_active: false,
        lib_filter: library::LibFilter::All,
        lib_filtered_indices: Vec::new(),
    };

    library::recompute_filtered(&mut app);

    if let Some(current) = app.themes.get(app.selected) {
        apply_entry_theme(current)?;
    }

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    let quit = match app.mode {
                        AppMode::Home => home::handle_key(&mut app, key)?,
                        AppMode::Library => library::handle_key(&mut app, key)?,
                        AppMode::Creator => creator::handle_key(&mut app, key)?,
                    };
                    if quit {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    if app.mode == AppMode::Library {
                        library::handle_mouse(&mut app, mouse)?;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.active_theme();
    let chrome = derive_chrome(&theme.background, &theme.foreground);

    frame.render_widget(Clear, frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(chrome.bg)),
        frame.area(),
    );

    match app.mode {
        AppMode::Home => home::draw(frame, app, &chrome),
        AppMode::Library => library::draw(frame, app, &chrome),
        AppMode::Creator => creator::draw(frame, app, &chrome),
    }
}

fn build_theme_entries(saved: &config::Config) -> Vec<ThemeEntry> {
    saved
        .resolved_themes()
        .into_iter()
        .map(ThemeEntry::from_resolved)
        .collect()
}

fn save_selected_entry(theme: &ThemeEntry) -> Result<()> {
    match theme.source {
        ThemeSource::BuiltIn | ThemeSource::BuiltInOverride => {
            let built_in = crate::theme::find_theme(&theme.slug)
                .ok_or_else(|| anyhow::anyhow!("missing built-in '{}'", theme.slug))?;
            config::save_selected_theme(built_in)
        }
        ThemeSource::Custom => config::save_custom_theme(theme.to_custom_theme()),
    }
}

fn apply_entry_theme(theme: &ThemeEntry) -> Result<()> {
    apply::apply_custom_theme(io::stdout(), &theme.to_custom_theme())?;
    Ok(())
}

fn current_git_branch() -> String {
    git_output(["branch", "--show-current"])
        .filter(|branch| !branch.is_empty())
        .or_else(|| {
            git_output(["rev-parse", "--short", "HEAD"]).map(|sha| format!("detached:{sha}"))
        })
        .unwrap_or_else(|| "no-git".to_string())
}

fn git_output<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::colors::*;

    fn generate_custom_theme(
        name: &str,
        background: &str,
        foreground: &str,
        cursor: &str,
        selection: &str,
    ) -> config::CustomTheme {
        let bg = hex_to_rgb(background);
        let fg = hex_to_rgb(foreground);
        let cs = hex_to_rgb(cursor);
        let sel = hex_to_rgb(selection);
        let ansi = [
            rgb_to_hex(scale(bg, 0.55)),
            rgb_to_hex(mix(cs, fg, 0.4)),
            rgb_to_hex(mix(sel, fg, 0.35)),
            rgb_to_hex(mix(fg, cs, 0.15)),
            rgb_to_hex(cs),
            rgb_to_hex(mix(cs, sel, 0.6)),
            rgb_to_hex(sel),
            rgb_to_hex(scale(fg, 0.8)),
            rgb_to_hex(scale(bg, 0.8)),
            rgb_to_hex(lighten(mix(cs, fg, 0.4), 0.15)),
            rgb_to_hex(lighten(mix(sel, fg, 0.35), 0.15)),
            rgb_to_hex(lighten(mix(fg, cs, 0.15), 0.15)),
            rgb_to_hex(lighten(cs, 0.2)),
            rgb_to_hex(lighten(mix(cs, sel, 0.6), 0.2)),
            rgb_to_hex(lighten(sel, 0.2)),
            rgb_to_hex(lighten(fg, 0.14)),
        ];
        config::CustomTheme {
            name: name.to_string(),
            slug: slugify(name),
            foreground: normalize_hex(foreground),
            background: normalize_hex(background),
            cursor: normalize_hex(cursor),
            selection: normalize_hex(selection),
            ansi,
            description: String::new(),
            author: "you".to_string(),
            kind: "custom".to_string(),
            mood: Vec::new(),
            accent: None,
            success: None,
            warning: None,
            error: None,
        }
    }

    fn slugify(value: &str) -> String {
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

    #[test]
    fn generated_theme_is_deterministic_for_same_input() {
        let a = generate_custom_theme("Forest Rose", "#237227", "#FFD786", "#FFF2CD", "#502B1A");
        let b = generate_custom_theme("Forest Rose", "#237227", "#FFD786", "#FFF2CD", "#502B1A");
        assert_eq!(a, b);
    }

    #[test]
    fn slugify_normalizes_words() {
        assert_eq!(slugify("  Forest   Rose  Theme "), "forest-rose-theme");
    }
}
