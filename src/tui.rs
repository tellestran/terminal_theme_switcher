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
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Picker,
    Wizard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    Guide,
    Background,
    Text,
    Accent1,
    Accent2,
    Name,
    Review,
}

#[derive(Debug, Clone)]
struct WizardState {
    step: WizardStep,
    background: String,
    text: String,
    accent1: String,
    accent2: String,
    name: String,
    error: Option<String>,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            step: WizardStep::Guide,
            background: String::new(),
            text: String::new(),
            accent1: String::new(),
            accent2: String::new(),
            name: String::new(),
            error: None,
        }
    }
}

struct App {
    mode: Mode,
    themes: Vec<ThemeEntry>,
    selected: usize,
    status: String,
    branch: String,
    state: ListState,
    wizard: WizardState,
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
        mode: Mode::Picker,
        themes,
        selected,
        status: "Arrows/jk move • Enter save • c create • q quit".to_string(),
        branch: current_git_branch(),
        state: ListState::default(),
        wizard: WizardState::default(),
        list_area: Rect::default(),
    };

    if let Some(current) = app.themes.get(app.selected) {
        apply_entry_theme(current)?;
    }

    loop {
        app.state.select(Some(app.selected));
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(160))? {
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

fn handle_mouse(app: &mut App, mouse: MouseEvent) -> Result<()> {
    if app.mode != Mode::Picker {
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

    let inner_top = area.y + 1;
    let inner_bottom = area.y + area.height.saturating_sub(1);
    if y < inner_top || y >= inner_bottom {
        return Ok(());
    }
    let idx = (y - inner_top) as usize;
    if idx < app.themes.len() {
        app.selected = idx;
        if let Some(theme) = app.themes.get(app.selected) {
            apply_entry_theme(theme)?;
            app.status = format!("Previewing {}.", theme.name);
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match app.mode {
        Mode::Picker => handle_picker_key(app, key),
        Mode::Wizard => handle_wizard_key(app, key),
    }
}

fn handle_picker_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
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
        KeyCode::Enter => {
            if let Some(theme) = app.themes.get(app.selected) {
                save_selected_entry(theme)?;
                apply_entry_theme(theme)?;
                app.status = format!("Saved {}.", theme.name);
            }
        }
        KeyCode::Char('c') => {
            app.mode = Mode::Wizard;
            app.wizard = WizardState::default();
            app.status = "Mode: Create Theme".to_string();
        }
        _ => {}
    }

    Ok(false)
}

fn handle_wizard_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.code == KeyCode::Esc
        || key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.mode = Mode::Picker;
        app.wizard = WizardState::default();
        app.status = "Creation cancelled. Back to picker.".to_string();
        return Ok(false);
    }

    match app.wizard.step {
        WizardStep::Guide => {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('n')) {
                app.wizard.step = WizardStep::Background;
                app.wizard.error = None;
            }
        }
        WizardStep::Background => handle_wizard_input_step(
            key,
            &mut app.wizard.background,
            &mut app.wizard.error,
            WizardStep::Text,
            &mut app.wizard.step,
        ),
        WizardStep::Text => handle_wizard_input_step(
            key,
            &mut app.wizard.text,
            &mut app.wizard.error,
            WizardStep::Accent1,
            &mut app.wizard.step,
        ),
        WizardStep::Accent1 => handle_wizard_input_step(
            key,
            &mut app.wizard.accent1,
            &mut app.wizard.error,
            WizardStep::Accent2,
            &mut app.wizard.step,
        ),
        WizardStep::Accent2 => handle_wizard_input_step(
            key,
            &mut app.wizard.accent2,
            &mut app.wizard.error,
            WizardStep::Name,
            &mut app.wizard.step,
        ),
        WizardStep::Name => match key.code {
            KeyCode::Backspace => {
                app.wizard.name.pop();
            }
            KeyCode::Enter => {
                let value = app.wizard.name.trim();
                if value.is_empty() {
                    app.wizard.error = Some("Theme name cannot be empty.".to_string());
                } else {
                    app.wizard.error = None;
                    app.wizard.step = WizardStep::Review;
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.wizard.name.push(ch);
            }
            _ => {}
        },
        WizardStep::Review => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let generated = generate_custom_theme(
                    app.wizard.name.trim(),
                    &app.wizard.background,
                    &app.wizard.text,
                    &app.wizard.accent1,
                    &app.wizard.accent2,
                );

                config::save_custom_theme(generated.clone())?;

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

                app.mode = Mode::Picker;
                app.wizard = WizardState::default();
                app.status = format!(
                    "Created '{}' and added it to your theme list.",
                    generated.name
                );
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.mode = Mode::Picker;
                app.wizard = WizardState::default();
                app.status = "Creation cancelled. Back to picker.".to_string();
            }
            _ => {}
        },
    }

    Ok(false)
}

fn handle_wizard_input_step(
    key: KeyEvent,
    field: &mut String,
    error: &mut Option<String>,
    next: WizardStep,
    current: &mut WizardStep,
) {
    match key.code {
        KeyCode::Tab => {
            let value = normalize_color_input(field);
            if let Some(normalized) = parse_color_input(&value) {
                *field = normalized;
                *error = None;
                *current = next;
            } else {
                *error =
                    Some("Invalid color. Use hex, name (red), or ANSI index (0-255).".to_string());
            }
        }
        KeyCode::Backspace => {
            field.pop();
        }
        KeyCode::Enter => {
            let value = normalize_color_input(field);
            if let Some(normalized) = parse_color_input(&value) {
                *field = normalized;
                *error = None;
                *current = next;
            } else {
                *error =
                    Some("Invalid color. Use hex, name (red), or ANSI index (0-255).".to_string());
            }
        }
        KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            field.push(ch);
        }
        _ => {}
    }
}

fn build_theme_entries(saved: &config::Config) -> Vec<ThemeEntry> {
    let mut entries: Vec<ThemeEntry> = theme::themes()
        .iter()
        .map(ThemeEntry::from_builtin)
        .collect();
    entries.extend(saved.custom_themes.iter().map(ThemeEntry::from_custom));
    entries
}

fn save_selected_entry(theme: &ThemeEntry) -> Result<()> {
    match theme.source {
        ThemeSource::BuiltIn => {
            let built_in = theme::find_theme(&theme.slug)
                .ok_or_else(|| anyhow::anyhow!("missing built-in theme '{}'", theme.slug))?;
            config::save_selected_theme(built_in)
        }
        ThemeSource::Custom => config::save_custom_theme(theme.to_custom_theme()),
    }
}

fn apply_entry_theme(theme: &ThemeEntry) -> Result<()> {
    match theme.source {
        ThemeSource::BuiltIn => {
            let built_in = theme::find_theme(&theme.slug)
                .ok_or_else(|| anyhow::anyhow!("missing built-in theme '{}'", theme.slug))?;
            apply::apply_theme(io::stdout(), built_in)?;
        }
        ThemeSource::Custom => apply::apply_custom_theme(io::stdout(), &theme.to_custom_theme())?,
    }

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
    let area = frame.area();
    frame.render_widget(Clear, area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let mode_label = match app.mode {
        Mode::Picker => "Mode: Picker",
        Mode::Wizard => "Mode: Create Theme",
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "switch-theme",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  terminal palette picker  "),
        Span::styled(mode_label, Style::default().fg(Color::Yellow)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, outer[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(40)])
        .split(outer[1]);

    app.list_area = columns[0];
    draw_theme_list(frame, columns[0], &app.themes, &mut app.state);

    if let Some(theme) = app.themes.get(app.selected) {
        draw_preview(frame, columns[1], theme, &app.branch);
    }

    if app.mode == Mode::Wizard {
        draw_wizard_overlay(frame, area, &app.wizard);
    }

    let footer = Paragraph::new(app.status.to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, outer[2]);
}

fn draw_theme_list(frame: &mut Frame, area: Rect, themes: &[ThemeEntry], state: &mut ListState) {
    let items = themes.iter().map(|theme| {
        let marker = match theme.source {
            ThemeSource::BuiltIn => "B",
            ThemeSource::Custom => "C",
        };

        ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default().bg(parse_color(&theme.ansi[4]))),
            Span::raw(" "),
            Span::styled(format!("[{marker}] "), Style::default().fg(Color::DarkGray)),
            Span::raw(theme.name.clone()),
        ]))
    });

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Themes (B=Built-in, C=Custom) ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, area, state);
}

fn draw_preview(frame: &mut Frame, area: Rect, theme: &ThemeEntry, branch: &str) {
    let demo_height = if area.height >= 30 {
        15
    } else if area.height >= 24 {
        11
    } else {
        7
    };

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(demo_height),
            Constraint::Min(7),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            theme.name.clone(),
            Style::default()
                .fg(parse_color(&theme.ansi[12]))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "fg {}  bg {}  cursor {}",
            theme.foreground, theme.background, theme.cursor
        )),
        Line::from(format!("selection {}", theme.selection)),
    ])
    .block(Block::default().title(" Preview ").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    frame.render_widget(header, vertical[0]);

    let sample = Paragraph::new(demo_lines(theme, vertical[1], branch))
        .block(Block::default().title(" Demo ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(sample, vertical[1]);

    draw_swatches(frame, vertical[2], theme);
}

fn draw_wizard_overlay(frame: &mut Frame, area: Rect, wizard: &WizardState) {
    let popup = centered_rect(78, 78, area);
    frame.render_widget(Clear, popup);

    let title = match wizard.step {
        WizardStep::Guide => "Create Theme • Guide",
        WizardStep::Background => "Create Theme • Step 1/5 Background",
        WizardStep::Text => "Create Theme • Step 2/5 Text",
        WizardStep::Accent1 => "Create Theme • Step 3/5 Accent 1",
        WizardStep::Accent2 => "Create Theme • Step 4/5 Accent 2",
        WizardStep::Name => "Create Theme • Step 5/5 Name",
        WizardStep::Review => "Create Theme • Review",
    };

    let mut lines: Vec<Line> = Vec::new();
    match wizard.step {
        WizardStep::Guide => {
            lines.push(Line::from("Need 4 colors:"));
            lines.push(Line::from("- Background: base terminal background"));
            lines.push(Line::from("- Text: main readable text"));
            lines.push(Line::from("- Accent 1: primary highlight"));
            lines.push(Line::from("- Accent 2: secondary highlight"));
            lines.push(Line::from(
                "Accepted: #RRGGBB, RRGGBB, color name (red), or ANSI index (0-255)",
            ));
            lines.push(Line::from(""));
            lines.push(Line::from("Press Enter to start. q/Esc/Ctrl+C to cancel."));
        }
        WizardStep::Background => wizard_input_lines(&mut lines, "Background", &wizard.background),
        WizardStep::Text => wizard_input_lines(&mut lines, "Text", &wizard.text),
        WizardStep::Accent1 => wizard_input_lines(&mut lines, "Accent 1", &wizard.accent1),
        WizardStep::Accent2 => wizard_input_lines(&mut lines, "Accent 2", &wizard.accent2),
        WizardStep::Name => {
            lines.push(Line::from("Type theme name and press Enter:"));
            lines.push(Line::from(format!("> {}", wizard.name)));
            lines.push(Line::from("q/Esc/Ctrl+C: cancel"));
        }
        WizardStep::Review => {
            lines.push(Line::from(format!("Name      : {}", wizard.name.trim())));
            lines.push(Line::from(format!("Background: {}", wizard.background)));
            lines.push(Line::from(format!("Text      : {}", wizard.text)));
            lines.push(Line::from(format!("Accent 1  : {}", wizard.accent1)));
            lines.push(Line::from(format!("Accent 2  : {}", wizard.accent2)));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter/Y: create and apply"));
            lines.push(Line::from("N: cancel"));
            lines.push(Line::from("q/Esc/Ctrl+C: cancel"));
        }
    }

    if let Some(error) = &wizard.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    let body = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(body, popup);
}

fn wizard_input_lines(lines: &mut Vec<Line>, label: &str, value: &str) {
    lines.push(Line::from(format!("Enter {label} color (#RRGGBB):")));
    lines.push(Line::from(format!("> {value}")));
    lines.push(Line::from(
        "Enter/Tab: continue  |  Backspace: edit  |  q/Esc/Ctrl+C: cancel",
    ));
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn demo_lines(theme: &ThemeEntry, area: Rect, branch: &str) -> Vec<Line<'static>> {
    let inner_height = area.height.saturating_sub(2) as usize;
    let wide = area.width >= 76;
    let mut lines = compact_demo_lines(theme, branch);

    if inner_height >= 7 {
        lines.extend(status_demo_lines(theme, wide));
    }

    if inner_height >= 10 {
        lines.extend(code_demo_lines(theme));
    }

    if inner_height >= 13 {
        lines.extend(log_demo_lines(theme, wide));
    }

    lines.truncate(inner_height.max(1));
    lines
}

fn compact_demo_lines(theme: &ThemeEntry, branch: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("$ ", Style::default().fg(parse_color(&theme.ansi[10]))),
            Span::styled(
                "git status --short",
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                branch.to_string(),
                Style::default().fg(parse_color(&theme.ansi[12])),
            ),
            Span::raw(" "),
            Span::styled(
                "+ src/theme.rs",
                Style::default().fg(parse_color(&theme.ansi[10])),
            ),
            Span::raw(" "),
            Span::styled(
                "~ src/tui.rs",
                Style::default().fg(parse_color(&theme.ansi[11])),
            ),
            Span::raw(" "),
            Span::styled(
                "- old-preview.rs",
                Style::default().fg(parse_color(&theme.ansi[9])),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "ok",
                Style::default()
                    .fg(parse_color(&theme.ansi[10]))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" tests passed  "),
            Span::styled(
                "warn",
                Style::default()
                    .fg(parse_color(&theme.ansi[11]))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" restart shell  "),
            Span::styled(
                "err",
                Style::default()
                    .fg(parse_color(&theme.ansi[9]))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" none"),
        ]),
    ]
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

fn status_demo_lines(theme: &ThemeEntry, wide: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            badge("normal", &theme.ansi[7], &theme.ansi[0]),
            Span::raw(" "),
            badge("red", &theme.ansi[1], &theme.ansi[15]),
            Span::raw(" "),
            badge("green", &theme.ansi[2], &theme.ansi[0]),
            Span::raw(" "),
            badge("yellow", &theme.ansi[3], &theme.ansi[0]),
        ]),
        Line::from(vec![
            badge("blue", &theme.ansi[4], &theme.ansi[15]),
            Span::raw(" "),
            badge("magenta", &theme.ansi[5], &theme.ansi[15]),
            Span::raw(" "),
            badge("cyan", &theme.ansi[6], &theme.ansi[0]),
            Span::raw(" "),
            badge("bright", &theme.ansi[15], &theme.ansi[0]),
        ]),
    ];

    if wide {
        lines.push(Line::from(vec![
            Span::styled(
                "selection",
                Style::default().fg(parse_color(&theme.selection)),
            ),
            Span::raw("  "),
            Span::styled(
                "cursor",
                Style::default()
                    .fg(parse_color(&theme.cursor))
                    .add_modifier(Modifier::REVERSED),
            ),
            Span::raw("  "),
            Span::styled(
                "foreground",
                Style::default().fg(parse_color(&theme.foreground)),
            ),
            Span::raw(" on "),
            Span::styled(
                "background",
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .bg(parse_color(&theme.background)),
            ),
        ]));
    }

    lines
}

fn code_demo_lines(theme: &ThemeEntry) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("fn ", Style::default().fg(parse_color(&theme.ansi[5]))),
            Span::styled(
                "apply_theme",
                Style::default().fg(parse_color(&theme.ansi[12])),
            ),
            Span::raw("("),
            Span::styled("palette", Style::default().fg(parse_color(&theme.ansi[14]))),
            Span::raw(": "),
            Span::styled("&Theme", Style::default().fg(parse_color(&theme.ansi[11]))),
            Span::raw(") -> "),
            Span::styled(
                "Result<()>",
                Style::default().fg(parse_color(&theme.ansi[10])),
            ),
            Span::raw(" {"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("emit", Style::default().fg(parse_color(&theme.ansi[12]))),
            Span::raw("("),
            Span::styled(
                "\"OSC 4;10;#...\"",
                Style::default().fg(parse_color(&theme.ansi[10])),
            ),
            Span::raw(");"),
            Span::raw(" "),
            Span::styled(
                "// preview first",
                Style::default().fg(parse_color(&theme.ansi[8])),
            ),
        ]),
    ]
}

fn log_demo_lines(theme: &ThemeEntry, wide: bool) -> Vec<Line<'static>> {
    let timing = if wide { "  42ms" } else { "" };
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("INFO ", Style::default().fg(parse_color(&theme.ansi[6]))),
            Span::raw("loaded 8 themes"),
            Span::styled(timing, Style::default().fg(parse_color(&theme.ansi[8]))),
        ]),
        Line::from(vec![
            Span::styled("PASS ", Style::default().fg(parse_color(&theme.ansi[2]))),
            Span::raw("palette contrast sample"),
            Span::styled(timing, Style::default().fg(parse_color(&theme.ansi[8]))),
        ]),
        Line::from(vec![
            Span::styled("NEXT ", Style::default().fg(parse_color(&theme.ansi[4]))),
            Span::raw("press Enter to persist this theme"),
        ]),
    ]
}

fn badge(label: &'static str, background: &str, foreground: &str) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(parse_color(foreground))
            .bg(parse_color(background))
            .add_modifier(Modifier::BOLD),
    )
}

fn draw_swatches(frame: &mut Frame, area: Rect, theme: &ThemeEntry) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    for row in 0..2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(12); 8])
            .split(rows[row]);

        for col in 0..8 {
            let index = row * 8 + col;
            let block = Block::default()
                .title(format!(" {index} "))
                .borders(Borders::ALL)
                .style(Style::default().bg(parse_color(&theme.ansi[index])));
            frame.render_widget(block, cols[col]);
        }
    }
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
    accent1: &str,
    accent2: &str,
) -> config::CustomTheme {
    let bg = hex_to_rgb(background);
    let fg = hex_to_rgb(foreground);
    let a1 = hex_to_rgb(accent1);
    let a2 = hex_to_rgb(accent2);

    let ansi = [
        rgb_to_hex(scale(bg, 0.55)),
        rgb_to_hex(a1),
        rgb_to_hex(mix(a1, a2, 0.45)),
        rgb_to_hex(mix(a1, fg, 0.45)),
        rgb_to_hex(a2),
        rgb_to_hex(mix(a1, a2, 0.7)),
        rgb_to_hex(mix(a2, fg, 0.4)),
        rgb_to_hex(scale(fg, 0.8)),
        rgb_to_hex(scale(bg, 0.8)),
        rgb_to_hex(lighten(a1, 0.18)),
        rgb_to_hex(lighten(mix(a1, a2, 0.45), 0.18)),
        rgb_to_hex(lighten(mix(a1, fg, 0.45), 0.15)),
        rgb_to_hex(lighten(a2, 0.2)),
        rgb_to_hex(lighten(mix(a1, a2, 0.7), 0.2)),
        rgb_to_hex(lighten(mix(a2, fg, 0.4), 0.2)),
        rgb_to_hex(lighten(fg, 0.14)),
    ];

    config::CustomTheme {
        name: name.to_string(),
        slug: slugify(name),
        foreground: normalize_hex(foreground),
        background: normalize_hex(background),
        cursor: rgb_to_hex(lighten(fg, 0.2)),
        selection: rgb_to_hex(lighten(bg, 0.18)),
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
        let a = generate_custom_theme("Forest Rose", "#237227", "#FFD786", "#F26076", "#77AADD");
        let b = generate_custom_theme("Forest Rose", "#237227", "#FFD786", "#F26076", "#77AADD");

        assert_eq!(a, b);
    }

    #[test]
    fn slugify_normalizes_words() {
        assert_eq!(slugify("  Forest   Rose  Theme "), "forest-rose-theme");
    }
}
