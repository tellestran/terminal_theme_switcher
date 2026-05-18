use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::colors::{parse_color, ChromeColors};
use super::widgets;
use super::App;
use crate::contrast;

pub fn draw(frame: &mut Frame, app: &App, chrome: &ChromeColors) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    widgets::top_bar(
        frame,
        root[0],
        &["schemer", "home"],
        &format!("v1.0.0 · {} themes", app.themes.len()),
        chrome,
    );

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .margin(1)
        .split(root[1]);

    draw_left_column(frame, content[0], app, chrome);
    draw_right_column(frame, content[1], app, chrome);

    widgets::keybinds_bar(
        frame,
        root[2],
        &[("↑↓", "move"), ("↵", "select"), ("?", "help"), ("q", "quit")],
        &format!("active: {}", app.active_theme().name),
        chrome,
    );
}

fn draw_left_column(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let theme = app.active_theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // heading
            Constraint::Length(2),  // name + metadata
            Constraint::Length(1),  // description
            Constraint::Length(1),  // spacer
            Constraint::Length(2),  // palette
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // color swatches
            Constraint::Length(1),  // contrast
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading
            Constraint::Min(4),    // menu
        ])
        .split(area);

    widgets::heading(frame, chunks[0], "active theme", chrome);

    // Name + metadata
    let name_line = Line::from(vec![
        Span::styled(
            &theme.name,
            Style::default().fg(chrome.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} · {}", theme.kind, theme.mood.join(" · ")),
            Style::default().fg(chrome.muted),
        ),
    ]);
    let desc_line = Line::from(Span::styled(&theme.description, Style::default().fg(chrome.muted)));
    frame.render_widget(
        Paragraph::new(vec![name_line, Line::from("")]).style(Style::default().bg(chrome.bg)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(desc_line).style(Style::default().bg(chrome.bg)),
        chunks[2],
    );

    // ANSI palette
    widgets::palette_row(frame, chunks[4], &theme.ansi);

    // Core color swatches
    let acc = theme.accent.as_deref().unwrap_or(&theme.ansi[3]).to_string();
    let swatch_spans: Vec<Span> = [("bg", &theme.background), ("fg", &theme.foreground), ("cur", &theme.cursor), ("acc", &acc)]
        .iter()
        .flat_map(|(label, hex)| {
            vec![
                Span::styled("  ", Style::default().bg(parse_color(hex))),
                Span::styled(format!(" {} {} ", label, hex.to_uppercase()), Style::default().fg(chrome.muted)),
            ]
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(swatch_spans)).style(Style::default().bg(chrome.bg)),
        chunks[6],
    );

    // Contrast
    let c_fg = contrast::contrast_ratio(&theme.background, &theme.foreground);
    let c_acc = contrast::contrast_ratio(&theme.background, theme.accent.as_deref().unwrap_or(&theme.ansi[3]));
    let fg_color = if c_fg >= 4.5 { parse_color(theme.success_color()) } else { parse_color(theme.warning_color()) };
    let acc_color = if c_acc >= 3.0 { parse_color(theme.success_color()) } else { parse_color(theme.warning_color()) };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("fg/bg ", Style::default().fg(chrome.muted)),
            Span::styled(
                format!("{:.1}:1 {}", c_fg, contrast::contrast_grade(c_fg)),
                Style::default().fg(fg_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("accent/bg ", Style::default().fg(chrome.muted)),
            Span::styled(
                format!("{:.1}:1", c_acc),
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
        ]))
        .style(Style::default().bg(chrome.bg)),
        chunks[7],
    );

    // What now menu
    widgets::heading(frame, chunks[9], "what now?", chrome);
    draw_menu(frame, chunks[10], app, chrome);
}

fn draw_menu(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let items = menu_items(app);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); items.len()])
        .split(area);

    for (i, (key, label, hint)) in items.iter().enumerate() {
        let selected = i == app.home_selected;
        let bg = if selected { chrome.fg } else { chrome.bg };
        let fg = if selected { chrome.bg } else { chrome.fg };
        let key_style = if selected {
            Style::default().fg(chrome.bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(chrome.accent).add_modifier(Modifier::BOLD)
        };
        let line = Line::from(vec![
            Span::styled(format!(" {key} "), key_style),
            Span::styled(format!(" {:<18}", label), Style::default().fg(fg)),
            Span::styled(*hint, Style::default().fg(if selected { fg } else { chrome.muted })),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(bg)),
            rows[i],
        );
    }
}

fn draw_right_column(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // heading
            Constraint::Length(5),  // recent
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading
            Constraint::Length(4),  // terminal info
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading
            Constraint::Min(3),    // tip
        ])
        .split(area);

    // Recent themes
    widgets::heading(frame, chunks[0], "recent", chrome);
    let recent_entries: Vec<(&str, &str)> = vec![
        ("Tokyo Night", "2h ago"),
        ("Gruvbox Dark", "yesterday"),
        ("Solarized Dark", "3d ago"),
        ("Dracula", "last week"),
        ("Nord", "last week"),
    ];
    let recent_lines: Vec<Line> = recent_entries
        .iter()
        .take(chunks[1].height as usize)
        .map(|(name, when)| {
            Line::from(vec![
                Span::styled(format!("  {:<22}", name), Style::default().fg(chrome.fg)),
                Span::styled(*when, Style::default().fg(chrome.muted)),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(recent_lines).style(Style::default().bg(chrome.bg)),
        chunks[1],
    );

    // Terminal info
    widgets::heading(frame, chunks[3], "terminal", chrome);
    let info_lines = vec![
        Line::from(vec![
            Span::styled("  shell      ", Style::default().fg(chrome.muted)),
            Span::styled("zsh", Style::default().fg(chrome.fg)),
        ]),
        Line::from(vec![
            Span::styled("  config     ", Style::default().fg(chrome.muted)),
            Span::styled("~/.config/schemer/", Style::default().fg(chrome.fg)),
        ]),
        Line::from(vec![
            Span::styled("  branch     ", Style::default().fg(chrome.muted)),
            Span::styled(&app.branch, Style::default().fg(chrome.fg)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(info_lines).style(Style::default().bg(chrome.bg)),
        chunks[4],
    );

    // Tip
    widgets::heading(frame, chunks[6], "tip", chrome);
    let tip = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("  press "),
            Span::styled(" p ", Style::default().fg(chrome.bg).bg(chrome.fg).add_modifier(Modifier::BOLD)),
            Span::raw(" on any theme to "),
            Span::styled("preview live", Style::default().fg(chrome.accent).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  for 5 seconds without committing."),
    ])
    .style(Style::default().fg(chrome.fg).bg(chrome.bg))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(chrome.border))
            .style(Style::default().bg(chrome.bg)),
    );
    frame.render_widget(tip, chunks[7]);
}

fn menu_items(_app: &App) -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("b", "browse library", &"themes installed"),
        ("n", "new theme", "create from scratch"),
        ("e", "edit active", "fork current theme"),
        ("s", "settings", "shell, keybinds, paths"),
    ]
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Down | KeyCode::Char('j') => app.home_selected = (app.home_selected + 1) % 4,
        KeyCode::Up | KeyCode::Char('k') => {
            app.home_selected = if app.home_selected == 0 { 3 } else { app.home_selected - 1 }
        }
        KeyCode::Char('b') | KeyCode::Char('2') => {
            app.mode = super::AppMode::Library;
            app.status = "Library opened.".to_string();
        }
        KeyCode::Char('n') | KeyCode::Char('1') => {
            app.mode = super::AppMode::Creator;
            app.creator = super::CreatorState::default();
            app.editing_slug = None;
            app.status = "Theme creator launched.".to_string();
        }
        KeyCode::Char('e') => {
            let slug = app.active_theme().slug.clone();
            app.mode = super::AppMode::Creator;
            app.creator = super::CreatorState::from_theme(app.active_theme());
            app.editing_slug = Some(slug);
            app.status = "Editing active theme.".to_string();
        }
        KeyCode::Enter => {
            match app.home_selected {
                0 => {
                    app.mode = super::AppMode::Library;
                    app.status = "Library opened.".to_string();
                }
                1 => {
                    app.mode = super::AppMode::Creator;
                    app.creator = super::CreatorState::default();
                    app.editing_slug = None;
                    app.status = "Theme creator launched.".to_string();
                }
                2 => {
                    let slug = app.active_theme().slug.clone();
                    app.mode = super::AppMode::Creator;
                    app.creator = super::CreatorState::from_theme(app.active_theme());
                    app.editing_slug = Some(slug);
                    app.status = "Editing active theme.".to_string();
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}
