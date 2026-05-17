use std::{
    io::{self, IsTerminal},
    process::Command,
    time::Duration,
};

use anyhow::{bail, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{
    apply, config,
    theme::{self, Theme},
};

const BASE_BG: Color = Color::Rgb(39, 10, 1);
const PANEL_BG: Color = Color::Rgb(52, 14, 2);
const FG: Color = Color::Rgb(245, 230, 196);
const MUTED: Color = Color::Rgb(191, 166, 141);
const BORDER: Color = Color::Rgb(214, 184, 135);
const ACCENT: Color = Color::Rgb(255, 235, 195);
const WARN: Color = Color::Rgb(255, 140, 120);

#[derive(Debug, Clone)]
struct ThemeEntry {
    name: String,
    slug: String,
    foreground: String,
    background: String,
    cursor: String,
    selection: String,
    ansi: [String; 16],
    source: ThemeSource,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatorStep {
    Guide,
    Background,
    Foreground,
    Cursor,
    Selection,
    Name,
    Review,
}

#[derive(Debug, Clone)]
struct CreatorState {
    step: CreatorStep,
    background: String,
    foreground: String,
    cursor: String,
    selection: String,
    name: String,
    error: Option<String>,
}

impl Default for CreatorState {
    fn default() -> Self {
        Self {
            step: CreatorStep::Guide,
            background: String::new(),
            foreground: String::new(),
            cursor: String::new(),
            selection: String::new(),
            name: String::new(),
            error: None,
        }
    }
}

struct App {
    mode: AppMode,
    home_selected: usize,
    themes: Vec<ThemeEntry>,
    selected: usize,
    status: String,
    branch: String,
    list_state: ListState,
    creator: CreatorState,
    editing_slug: Option<String>,
    pending_delete_slug: Option<String>,
    list_area: Rect,
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
        .position(|theme| theme.slug == saved.theme)
        .unwrap_or(0);

    let mut app = App {
        mode: AppMode::Home,
        home_selected: 0,
        themes,
        selected,
        status: "SYSTEM_STATUS: OK | Active: Local_Default".to_string(),
        branch: current_git_branch(),
        list_state: ListState::default(),
        creator: CreatorState::default(),
        editing_slug: None,
        pending_delete_slug: None,
        list_area: Rect::default(),
    };

    if let Some(current) = app.themes.get(app.selected) {
        apply_entry_theme(current)?;
    }

    loop {
        app.list_state.select(Some(app.selected));
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key(&mut app, key)? {
                        break;
                    }
                }
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse)?,
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    match app.mode {
        AppMode::Home => handle_home_key(app, key),
        AppMode::Library => handle_library_key(app, key),
        AppMode::Creator => handle_creator_key(app, key),
    }
}

fn handle_home_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Down | KeyCode::Char('j') => app.home_selected = (app.home_selected + 1) % 2,
        KeyCode::Up | KeyCode::Char('k') => {
            app.home_selected = app.home_selected.checked_sub(1).unwrap_or(1)
        }
        KeyCode::Char('1') => {
            app.mode = AppMode::Creator;
            app.creator = CreatorState::default();
            app.editing_slug = None;
            app.status = "Theme creator launched.".to_string();
        }
        KeyCode::Char('2') => {
            app.mode = AppMode::Library;
            app.status = "Library opened.".to_string();
        }
        KeyCode::Enter => {
            if app.home_selected == 0 {
                app.mode = AppMode::Creator;
                app.creator = CreatorState::default();
                app.editing_slug = None;
                app.status = "Theme creator launched.".to_string();
            } else {
                app.mode = AppMode::Library;
                app.status = "Library opened.".to_string();
            }
        }
        _ => {}
    }

    Ok(false)
}

fn handle_library_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if let Some(slug) = app.pending_delete_slug.clone() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let deleted = if app.themes.iter().any(|theme| {
                    theme.slug == slug && matches!(theme.source, ThemeSource::BuiltInOverride)
                }) {
                    config::delete_builtin_override(&slug)?
                } else {
                    config::delete_custom_theme(&slug)?
                };
                app.pending_delete_slug = None;
                if deleted {
                    let saved = config::Config::load().unwrap_or_default();
                    app.themes = build_theme_entries(&saved);
                    app.selected = app
                        .themes
                        .iter()
                        .position(|theme| theme.slug == saved.theme)
                        .unwrap_or(0);
                    if let Some(theme) = app.themes.get(app.selected) {
                        apply_entry_theme(theme)?;
                    }
                    app.status = format!("Deleted theme customization '{}'.", slug);
                } else {
                    app.status = "Theme not found; nothing deleted.".to_string();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.pending_delete_slug = None;
                app.status = "Delete cancelled.".to_string();
            }
            _ => {}
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('h') => {
            app.mode = AppMode::Home;
            app.status = "Returned to launcher.".to_string();
        }
        KeyCode::Char('n') => {
            app.mode = AppMode::Creator;
            app.creator = CreatorState::default();
            app.editing_slug = None;
            app.status = "Theme creator launched.".to_string();
        }
        KeyCode::Char('e') => {
            if let Some(theme) = app.themes.get(app.selected) {
                match theme.source {
                    ThemeSource::Custom | ThemeSource::BuiltIn | ThemeSource::BuiltInOverride => {
                        app.mode = AppMode::Creator;
                        app.creator = CreatorState {
                            step: CreatorStep::Background,
                            background: theme.background.clone(),
                            foreground: theme.foreground.clone(),
                            cursor: theme.cursor.clone(),
                            selection: theme.selection.clone(),
                            name: theme.name.clone(),
                            error: None,
                        };
                        app.editing_slug = Some(theme.slug.clone());
                        app.status = format!("Editing theme '{}'.", theme.slug);
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(theme) = app.themes.get(app.selected) {
                match theme.source {
                    ThemeSource::Custom => {
                        app.pending_delete_slug = Some(theme.slug.clone());
                        app.status =
                            format!("Delete '{}' ? Press y to confirm, n to cancel.", theme.slug);
                    }
                    ThemeSource::BuiltInOverride => {
                        app.pending_delete_slug = Some(theme.slug.clone());
                        app.status = format!(
                            "Revert built-in override '{}' ? Press y to confirm, n to cancel.",
                            theme.slug
                        );
                    }
                    ThemeSource::BuiltIn => {
                        app.status = "No override to delete for this built-in theme.".to_string();
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected = (app.selected + 1) % app.themes.len();
            if let Some(theme) = app.themes.get(app.selected) {
                apply_entry_theme(theme)?;
                app.status = format!("Previewing {}.", theme.name);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected = app.selected.checked_sub(1).unwrap_or(app.themes.len() - 1);
            if let Some(theme) = app.themes.get(app.selected) {
                apply_entry_theme(theme)?;
                app.status = format!("Previewing {}.", theme.name);
            }
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            if let Some(theme) = app.themes.get(app.selected) {
                save_selected_entry(theme)?;
                apply_entry_theme(theme)?;
                app.status = format!("Saved {}.", theme.slug);
            }
        }
        _ => {}
    }

    Ok(false)
}

fn handle_creator_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
        app.mode = AppMode::Library;
        app.creator = CreatorState::default();
        app.editing_slug = None;
        app.status = "Creator aborted.".to_string();
        return Ok(false);
    }

    match app.creator.step {
        CreatorStep::Guide => {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('p')) {
                app.creator.step = CreatorStep::Background;
                app.creator.error = None;
            }
        }
        CreatorStep::Background => handle_creator_color_step(
            key,
            &mut app.creator.background,
            &mut app.creator.error,
            CreatorStep::Foreground,
            &mut app.creator.step,
        ),
        CreatorStep::Foreground => handle_creator_color_step(
            key,
            &mut app.creator.foreground,
            &mut app.creator.error,
            CreatorStep::Cursor,
            &mut app.creator.step,
        ),
        CreatorStep::Cursor => handle_creator_color_step(
            key,
            &mut app.creator.cursor,
            &mut app.creator.error,
            CreatorStep::Selection,
            &mut app.creator.step,
        ),
        CreatorStep::Selection => handle_creator_color_step(
            key,
            &mut app.creator.selection,
            &mut app.creator.error,
            CreatorStep::Name,
            &mut app.creator.step,
        ),
        CreatorStep::Name => match key.code {
            KeyCode::Backspace => {
                app.creator.name.pop();
            }
            KeyCode::Enter | KeyCode::Tab => {
                let value = app.creator.name.trim();
                if value.is_empty() {
                    app.creator.error = Some("Theme name is required.".to_string());
                } else {
                    app.creator.error = None;
                    app.creator.step = CreatorStep::Review;
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.creator.name.push(ch);
            }
            _ => {}
        },
        CreatorStep::Review => match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                let generated = generate_custom_theme(
                    app.creator.name.trim(),
                    &app.creator.background,
                    &app.creator.foreground,
                    &app.creator.cursor,
                    &app.creator.selection,
                );
                let generated = if let Some(existing_slug) = &app.editing_slug {
                    config::CustomTheme {
                        slug: existing_slug.clone(),
                        ..generated
                    }
                } else {
                    generated
                };
                if let Some(editing_slug) = &app.editing_slug {
                    if theme::find_theme(editing_slug).is_some() {
                        config::save_builtin_override(config::BuiltinOverride {
                            name: generated.name.clone(),
                            slug: editing_slug.clone(),
                            foreground: generated.foreground.clone(),
                            background: generated.background.clone(),
                            cursor: generated.cursor.clone(),
                            selection: generated.selection.clone(),
                            ansi: generated.ansi.clone(),
                        })?;
                    } else {
                        config::save_custom_theme(generated.clone())?;
                    }
                } else {
                    config::save_custom_theme(generated.clone())?;
                }

                let saved = config::Config::load().unwrap_or_default();
                app.themes = build_theme_entries(&saved);
                app.selected = app
                    .themes
                    .iter()
                    .position(|theme| theme.slug == generated.slug)
                    .unwrap_or(0);

                if let Some(theme) = app.themes.get(app.selected) {
                    apply_entry_theme(theme)?;
                }

                app.mode = AppMode::Library;
                app.creator = CreatorState::default();
                app.editing_slug = None;
                app.status = format!("Saved {}.", generated.slug);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.mode = AppMode::Library;
                app.creator = CreatorState::default();
                app.editing_slug = None;
                app.status = "Creator cancelled.".to_string();
            }
            _ => {}
        },
    }

    Ok(false)
}

fn handle_creator_color_step(
    key: KeyEvent,
    field: &mut String,
    error: &mut Option<String>,
    next: CreatorStep,
    current: &mut CreatorStep,
) {
    match key.code {
        KeyCode::Backspace => {
            field.pop();
        }
        KeyCode::Enter | KeyCode::Tab => {
            let value = normalize_color_input(field);
            if let Some(normalized) = parse_color_input(&value) {
                *field = normalized;
                *error = None;
                *current = next;
            } else {
                *error = Some("Invalid color. Use #RRGGBB, name, or ANSI index.".to_string());
            }
        }
        KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            field.push(ch);
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) -> Result<()> {
    if app.mode != AppMode::Library {
        return Ok(());
    }
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return Ok(());
    }

    let x = mouse.column;
    let y = mouse.row;
    let area = app.list_area;
    if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
        return Ok(());
    }

    let idx = y.saturating_sub(area.y + 2) as usize;
    if idx < app.themes.len() {
        app.selected = idx;
        if let Some(theme) = app.themes.get(app.selected) {
            apply_entry_theme(theme)?;
            app.status = format!("Previewing {}.", theme.name);
        }
    }

    Ok(())
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
            let built_in = theme::find_theme(&theme.slug)
                .ok_or_else(|| anyhow::anyhow!("missing built-in theme '{}'", theme.slug))?;
            config::save_selected_theme(built_in)
        }
        ThemeSource::Custom => config::save_custom_theme(theme.to_custom_theme()),
    }
}

fn apply_entry_theme(theme: &ThemeEntry) -> Result<()> {
    apply::apply_custom_theme(io::stdout(), &theme.to_custom_theme())?;
    Ok(())
}

impl ThemeEntry {
    fn from_builtin(theme: &Theme) -> Self {
        Self {
            name: theme.name.to_string(),
            slug: theme.slug.to_string(),
            foreground: theme.foreground.to_string(),
            background: theme.background.to_string(),
            cursor: theme.cursor.to_string(),
            selection: theme.selection.to_string(),
            ansi: theme.ansi.map(str::to_string),
            source: ThemeSource::BuiltIn,
        }
    }

    fn from_custom(theme: &config::CustomTheme) -> Self {
        Self {
            name: theme.name.clone(),
            slug: theme.slug.clone(),
            foreground: theme.foreground.clone(),
            background: theme.background.clone(),
            cursor: theme.cursor.clone(),
            selection: theme.selection.clone(),
            ansi: theme.ansi.clone(),
            source: ThemeSource::Custom,
        }
    }

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
        }
    }
}

fn draw(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Clear, frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(BASE_BG).fg(FG)),
        frame.area(),
    );

    match app.mode {
        AppMode::Home => draw_home(frame, app),
        AppMode::Library => draw_library(frame, app),
        AppMode::Creator => draw_creator(frame, app),
    }
}

fn draw_home(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Line::from(vec![
            Span::raw(" [ "),
            Span::styled(
                "TERM_SCHEMER_V1",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ] "),
        ]))
        .style(Style::default().bg(BASE_BG));
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(4)])
        .margin(2)
        .split(area);

    let choices = [
        (
            "1. [ INITIALIZE_NEW_THEME ]",
            "Launch synthesis wizard",
            app.home_selected == 0,
        ),
        (
            "2. [ BROWSE_LIBRARY ]",
            "Access local repository",
            app.home_selected == 1,
        ),
    ];

    let menu_lines: Vec<Line> = choices
        .iter()
        .flat_map(|(left, right, selected)| {
            let style = if *selected {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            vec![
                Line::from(vec![
                    Span::styled(*left, style),
                    Span::styled("   -> ", Style::default().fg(MUTED)),
                    Span::styled(*right, Style::default().fg(MUTED)),
                ]),
                Line::from(""),
            ]
        })
        .collect();

    let menu = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .style(Style::default().bg(BASE_BG))
        .block(Block::default().padding(ratatui::widgets::Padding::new(12, 2, 8, 1)));
    frame.render_widget(menu, chunks[0]);

    let status = Paragraph::new(app.status.to_string())
        .alignment(Alignment::Center)
        .style(Style::default().fg(MUTED))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(BORDER)),
        );
    frame.render_widget(status, chunks[1]);
}

fn draw_library(frame: &mut Frame, app: &mut App) {
    let (dynamic_bg, dynamic_panel_bg) = app
        .themes
        .get(app.selected)
        .map(|theme| library_background_colors(theme))
        .unwrap_or((BASE_BG, PANEL_BG));

    frame.render_widget(
        Block::default().style(Style::default().bg(dynamic_bg)),
        frame.area(),
    );

    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "terminal palette picker",
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::raw("                                              "),
        Span::styled(
            "Mode: Library",
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER)),
    );
    frame.render_widget(header, root[0]);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(40)])
        .split(root[1]);

    app.list_area = content[0];
    draw_theme_list(
        frame,
        content[0],
        &app.themes,
        &mut app.list_state,
        dynamic_panel_bg,
    );
    if let Some(theme) = app.themes.get(app.selected) {
        draw_preview(frame, content[1], theme, &app.branch, dynamic_panel_bg);
    }

    let footer = Paragraph::new(format!(
        "{}    v1.0.4  [h:home n:new e:edit d:delete a/apply enter:save q:quit]",
        app.status
    ))
    .style(Style::default().fg(MUTED))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER)),
    );
    frame.render_widget(footer, root[2]);
}

fn draw_creator(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "switch-theme",
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    Theme Creator    ", Style::default().fg(MUTED)),
        Span::styled(
            "Mode: Picker",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER)),
    );
    frame.render_widget(header, root[0]);

    let body = Block::default().style(Style::default().bg(BASE_BG));
    frame.render_widget(body, root[1]);

    let inner = root[1].inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let mut lines: Vec<Line> = vec![];
    lines.push(Line::from(Span::styled(
        "Step 1 of 3: Technical Manual",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "theme_init(1)",
        Style::default().fg(FG).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "General Commands Manual",
        Style::default().fg(MUTED),
    )));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "REQUIRED PARAMETERS",
        Style::default().fg(FG).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(format!(
        "1. BACKGROUND  # {}",
        app.creator.background
    )));
    lines.push(Line::from(format!(
        "2. FOREGROUND  # {}",
        app.creator.foreground
    )));
    lines.push(Line::from(format!(
        "3. CURSOR      # {}",
        app.creator.cursor
    )));
    lines.push(Line::from(format!(
        "4. SELECTION   # {}",
        app.creator.selection
    )));
    lines.push(Line::from(format!("5. NAME        {}", app.creator.name)));
    lines.push(Line::from(""));

    let step_text = match app.creator.step {
        CreatorStep::Guide => "Guide: Press Enter to begin. q/Esc abort.",
        CreatorStep::Background => "Input BACKGROUND (#RRGGBB / name / 0-255) then Enter.",
        CreatorStep::Foreground => "Input FOREGROUND (#RRGGBB / name / 0-255) then Enter.",
        CreatorStep::Cursor => "Input CURSOR (#RRGGBB / name / 0-255) then Enter.",
        CreatorStep::Selection => "Input SELECTION (#RRGGBB / name / 0-255) then Enter.",
        CreatorStep::Name => "Input theme NAME then Enter.",
        CreatorStep::Review => "Review: Enter/Y to save, N to cancel.",
    };
    lines.push(Line::from(Span::styled(
        step_text,
        Style::default().fg(ACCENT),
    )));

    if let Some(err) = &app.creator.error {
        lines.push(Line::from(Span::styled(
            err,
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        )));
    }

    let current_input = match app.creator.step {
        CreatorStep::Background => app.creator.background.clone(),
        CreatorStep::Foreground => app.creator.foreground.clone(),
        CreatorStep::Cursor => app.creator.cursor.clone(),
        CreatorStep::Selection => app.creator.selection.clone(),
        CreatorStep::Name => app.creator.name.clone(),
        _ => String::new(),
    };
    lines.push(Line::from(""));
    lines.push(Line::from(format!("Awaiting input... {}", current_input)));

    let panel = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(FG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(PANEL_BG)),
        );
    frame.render_widget(panel, inner);

    let footer = Paragraph::new(format!("{}    [q abort] [enter proceed]", app.status))
        .style(Style::default().fg(MUTED))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(BORDER)),
        );
    frame.render_widget(footer, root[2]);
}

fn draw_theme_list(
    frame: &mut Frame,
    area: Rect,
    themes: &[ThemeEntry],
    state: &mut ListState,
    panel_bg: Color,
) {
    let items = themes.iter().map(|theme| {
        let source_mark = match theme.source {
            ThemeSource::BuiltIn => "B",
            ThemeSource::BuiltInOverride => "O",
            ThemeSource::Custom => "C",
        };

        ListItem::new(Line::from(vec![
            Span::styled("[P] ", Style::default().fg(FG)),
            Span::styled(theme.name.clone(), Style::default().fg(FG)),
            Span::raw(" "),
            Span::styled(source_mark, Style::default().fg(MUTED)),
        ]))
    });

    let list = List::new(items)
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled(
                        "Theme Library",
                        Style::default().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "\n(B=Built-in, O=Built-in Override, C=Custom)",
                        Style::default().fg(MUTED),
                    ),
                ]))
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(panel_bg)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, state);
}

fn draw_preview(frame: &mut Frame, area: Rect, theme: &ThemeEntry, branch: &str, panel_bg: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Min(8),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("Preview: {}", theme.name),
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Retro groove color scheme for terminal workflows.",
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            format!(
                "fg {}  bg {}  cursor {}",
                theme.foreground, theme.background, theme.cursor
            ),
            Style::default().fg(MUTED),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER)),
    );
    frame.render_widget(header, chunks[0]);

    let demo_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let left_demo = Paragraph::new(vec![
        Line::from("1 | function initTerminal() {"),
        Line::from("2 |   const config = loadConfig();"),
        Line::from("3 |   if (!config.theme) {"),
        Line::from(Span::styled(
            "4 |     throw new Error('No theme found');",
            Style::default().fg(parse_color(&theme.ansi[1])),
        )),
        Line::from("5 |   }"),
        Line::from("6 |   applyTheme(config.theme);"),
        Line::from("7 | }"),
    ])
    .block(
        Block::default()
            .title(" [ DEMO_RENDER.SH ] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(panel_bg)),
    )
    .style(Style::default().fg(FG));
    frame.render_widget(left_demo, demo_cols[0]);

    let right_demo = Paragraph::new(vec![
        Line::from(format!("user@host:{}$ ./run_tests.sh", branch)),
        Line::from("[INFO] Loading configuration... OK"),
        Line::from("[INFO] Initializing modules... OK"),
        Line::from(Span::styled(
            "[WARN] Deprecated flag used: --fast",
            Style::default().fg(parse_color(&theme.ansi[1])),
        )),
        Line::from("[PASS] Test suite 1: UI Components"),
        Line::from("[PASS] Test suite 2: Data Store"),
        Line::from("All tests passed. (1.2s)"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(panel_bg)),
    )
    .style(Style::default().fg(FG));
    frame.render_widget(right_demo, demo_cols[1]);

    draw_swatches(frame, chunks[2], theme, panel_bg);
}

fn draw_swatches(frame: &mut Frame, area: Rect, theme: &ThemeEntry, panel_bg: Color) {
    let panel = Block::default()
        .title(" [ ANSI_16_COLORS ] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(panel_bg));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    for row in 0..2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(12); 8])
            .split(rows[row]);

        for col in 0..8 {
            let index = row * 8 + col;
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(25, 8, 4)))
                .style(Style::default().bg(parse_color(&theme.ansi[index])));
            frame.render_widget(block, cols[col]);
        }
    }
}

fn library_background_colors(theme: &ThemeEntry) -> (Color, Color) {
    let (r, g, b) = hex_to_rgb(&theme.background);
    let base = Color::Rgb(r, g, b);
    let panel = Color::Rgb(
        ((r as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
        ((g as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
        ((b as f32) * 0.78).round().clamp(0.0, 255.0) as u8,
    );
    (base, panel)
}

fn normalize_hex(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('#') {
        trimmed.to_ascii_uppercase()
    } else {
        format!("#{}", trimmed.to_ascii_uppercase())
    }
}

fn is_valid_hex(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
}

fn normalize_color_input(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn parse_color_input(value: &str) -> Option<String> {
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

fn ansi256_to_hex(index: u8) -> String {
    let (r, g, b) = if index < 16 {
        let base = [
            (0, 0, 0),
            (128, 0, 0),
            (0, 128, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (0, 128, 128),
            (192, 192, 192),
            (128, 128, 128),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (0, 0, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ];
        base[index as usize]
    } else if index <= 231 {
        let i = index - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;
        let scale = [0, 95, 135, 175, 215, 255];
        (scale[r as usize], scale[g as usize], scale[b as usize])
    } else {
        let level = 8 + (index - 232) * 10;
        (level, level, level)
    };
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

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

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let value = normalize_hex(hex);
    (
        u8::from_str_radix(&value[1..3], 16).unwrap_or_default(),
        u8::from_str_radix(&value[3..5], 16).unwrap_or_default(),
        u8::from_str_radix(&value[5..7], 16).unwrap_or_default(),
    )
}

fn rgb_to_hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), ratio_b: f32) -> (u8, u8, u8) {
    let ratio_a = 1.0 - ratio_b;
    (
        (a.0 as f32 * ratio_a + b.0 as f32 * ratio_b).round() as u8,
        (a.1 as f32 * ratio_a + b.1 as f32 * ratio_b).round() as u8,
        (a.2 as f32 * ratio_a + b.2 as f32 * ratio_b).round() as u8,
    )
}

fn lighten(color: (u8, u8, u8), amount: f32) -> (u8, u8, u8) {
    mix(color, (255, 255, 255), amount)
}

fn scale(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    (
        (color.0 as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.1 as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.2 as f32 * factor).round().clamp(0.0, 255.0) as u8,
    )
}

fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::Reset;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or_default();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or_default();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or_default();

    Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

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
