use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::colors::{parse_color, ChromeColors};
use super::widgets;
use super::{App, AppMode, ThemeEntry, ThemeSource};
use crate::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibFilter {
    All,
    Dark,
    Light,
    Warm,
    Cool,
    Custom,
}

impl LibFilter {
    pub const ALL: [LibFilter; 6] = [
        LibFilter::All,
        LibFilter::Dark,
        LibFilter::Light,
        LibFilter::Warm,
        LibFilter::Cool,
        LibFilter::Custom,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            LibFilter::All => "all",
            LibFilter::Dark => "dark",
            LibFilter::Light => "light",
            LibFilter::Warm => "warm",
            LibFilter::Cool => "cool",
            LibFilter::Custom => "custom",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            LibFilter::All => LibFilter::Dark,
            LibFilter::Dark => LibFilter::Light,
            LibFilter::Light => LibFilter::Warm,
            LibFilter::Warm => LibFilter::Cool,
            LibFilter::Cool => LibFilter::Custom,
            LibFilter::Custom => LibFilter::All,
        }
    }
}

fn matches_filter(theme: &ThemeEntry, filter: &LibFilter) -> bool {
    match filter {
        LibFilter::All => true,
        LibFilter::Dark => theme.mood.iter().any(|m| m == "dark"),
        LibFilter::Light => theme.mood.iter().any(|m| m == "light"),
        LibFilter::Warm => theme.mood.iter().any(|m| m == "warm"),
        LibFilter::Cool => theme.mood.iter().any(|m| m == "cool"),
        LibFilter::Custom => matches!(theme.source, ThemeSource::Custom),
    }
}

fn matches_search(theme: &ThemeEntry, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_ascii_lowercase();
    theme.name.to_ascii_lowercase().contains(&q)
        || theme.description.to_ascii_lowercase().contains(&q)
}

pub fn recompute_filtered(app: &mut App) {
    app.lib_filtered_indices = app
        .themes
        .iter()
        .enumerate()
        .filter(|(_, t)| matches_filter(t, &app.lib_filter))
        .filter(|(_, t)| matches_search(t, &app.lib_search_query))
        .map(|(i, _)| i)
        .collect();
}

pub fn draw(frame: &mut Frame, app: &mut App, chrome: &ChromeColors) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top bar
            Constraint::Length(1),  // search + filters
            Constraint::Min(10),   // body
            Constraint::Length(1),  // keybinds
        ])
        .split(area);

    let focused_theme = app.focused_library_theme();
    let count_text = format!(
        "{} of {} themes",
        app.lib_filtered_indices.len(),
        app.themes.len()
    );
    widgets::top_bar(frame, root[0], &["schemer", "library"], &count_text, chrome);

    draw_search_bar(frame, root[1], app, chrome);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(40)])
        .split(root[2]);

    app.list_area = body[0];
    draw_theme_list(frame, body[0], app, chrome);

    if let Some(theme) = focused_theme {
        draw_preview(frame, body[1], &theme, app, chrome);
    }

    let focused_name = app
        .focused_library_theme()
        .map(|t| t.name.clone())
        .unwrap_or_default();
    widgets::keybinds_bar(
        frame,
        root[3],
        &[
            ("↑↓", "browse"),
            ("/", "search"),
            ("p", "preview"),
            ("a", "apply"),
            ("e", "edit"),
            ("h", "home"),
        ],
        &format!("focused: {}", focused_name),
        chrome,
    );
}

fn draw_search_bar(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(20)])
        .split(area);

    // Search input
    let _border_color = if app.lib_search_active { chrome.accent } else { chrome.border };
    let search_text = if app.lib_search_query.is_empty() && !app.lib_search_active {
        "/ search themes…".to_string()
    } else {
        format!("/ {}", app.lib_search_query)
    };
    let search_style = if app.lib_search_query.is_empty() && !app.lib_search_active {
        Style::default().fg(chrome.muted)
    } else {
        Style::default().fg(chrome.fg)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(search_text, search_style))
            .style(Style::default().bg(chrome.bg)),
        cols[0],
    );

    // Filter chips
    let mut spans: Vec<Span> = Vec::new();
    for f in &LibFilter::ALL {
        let active = *f == app.lib_filter;
        let style = if active {
            Style::default().fg(chrome.bg).bg(chrome.fg)
        } else {
            Style::default().fg(chrome.fg).bg(chrome.bg)
        };
        spans.push(Span::styled(format!(" {} ", f.label()), style));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(chrome.bg)),
        cols[1],
    );
}

fn draw_theme_list(frame: &mut Frame, area: Rect, app: &mut App, chrome: &ChromeColors) {
    let items: Vec<ListItem> = app
        .lib_filtered_indices
        .iter()
        .map(|&idx| {
            let theme = &app.themes[idx];
            let selected = idx == app.selected;
            let is_active = theme.slug == app.themes.get(app.active_idx()).map(|t| t.slug.as_str()).unwrap_or("");

            let mut spans = vec![
                Span::styled("  ", Style::default().bg(parse_color(&theme.background))),
                Span::styled("  ", Style::default().bg(parse_color(&theme.foreground))),
                Span::raw(" "),
            ];

            let name_width = area.width.saturating_sub(14) as usize;
            let name: String = theme.name.chars().take(name_width).collect();
            spans.push(Span::styled(
                format!("{:<width$}", name, width = name_width),
                Style::default().fg(if selected { chrome.bg } else { chrome.fg }),
            ));

            if is_active {
                spans.push(Span::styled(
                    " ON",
                    Style::default()
                        .fg(if selected { chrome.bg } else { chrome.accent })
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                let tag = match theme.source {
                    ThemeSource::BuiltIn | ThemeSource::BuiltInOverride => "BLT",
                    ThemeSource::Custom => "YOU",
                };
                spans.push(Span::styled(
                    format!(" {}", tag),
                    Style::default().fg(if selected { chrome.bg } else { chrome.muted }),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(chrome.border))
                .style(Style::default().bg(chrome.bg)),
        )
        .highlight_style(
            Style::default()
                .bg(chrome.fg)
                .fg(chrome.bg)
                .add_modifier(Modifier::BOLD),
        );

    let filtered_pos = app
        .lib_filtered_indices
        .iter()
        .position(|&i| i == app.selected);
    app.list_state.select(filtered_pos);
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_preview(frame: &mut Frame, area: Rect, theme: &ThemeEntry, app: &App, chrome: &ChromeColors) {
    let inner = area.inner(ratatui::layout::Margin { vertical: 0, horizontal: 1 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // name + description
            Constraint::Length(1),  // spacer
            Constraint::Length(4),  // metadata
            Constraint::Length(1),  // heading
            Constraint::Length(2),  // ansi palette
            Constraint::Length(1),  // spacer
            Constraint::Length(7),  // code + shell side by side
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading
            Constraint::Min(3),    // contrast report
        ])
        .split(inner);

    // Name + active badge
    let is_active = theme.slug == app.themes.get(app.active_idx()).map(|t| t.slug.as_str()).unwrap_or("");
    let mut name_spans = vec![
        Span::styled(
            &theme.name,
            Style::default().fg(chrome.fg).add_modifier(Modifier::BOLD),
        ),
    ];
    if is_active {
        name_spans.push(Span::raw("  "));
        name_spans.push(Span::styled(
            " ACTIVE ",
            Style::default().fg(chrome.bg).bg(chrome.accent).add_modifier(Modifier::BOLD),
        ));
    }
    let desc = Line::from(Span::styled(&theme.description, Style::default().fg(chrome.muted)));
    frame.render_widget(
        Paragraph::new(vec![Line::from(name_spans), desc]).style(Style::default().bg(chrome.bg)),
        chunks[0],
    );

    // Metadata grid
    let meta_lines = vec![
        Line::from(vec![
            Span::styled("author   ", Style::default().fg(chrome.muted)),
            Span::styled(&theme.author, Style::default().fg(chrome.fg)),
            Span::raw("    "),
            Span::styled("kind   ", Style::default().fg(chrome.muted)),
            Span::styled(&theme.kind, Style::default().fg(chrome.fg)),
        ]),
        Line::from(vec![
            Span::styled("mood     ", Style::default().fg(chrome.muted)),
            Span::styled(theme.mood.join(", "), Style::default().fg(chrome.fg)),
        ]),
        Line::from(vec![
            Span::styled("fg / bg  ", Style::default().fg(chrome.muted)),
            Span::styled(
                format!("{} / {}", theme.foreground.to_uppercase(), theme.background.to_uppercase()),
                Style::default().fg(chrome.fg),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(meta_lines).style(Style::default().bg(chrome.bg)),
        chunks[2],
    );

    // ANSI palette
    widgets::heading(frame, chunks[3], "ansi 16", chrome);
    widgets::palette_row(frame, chunks[4], &theme.ansi);

    // Code + shell side by side
    let demo_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[6]);

    let code_lines = widgets::code_sample_lines(theme);
    frame.render_widget(
        Paragraph::new(code_lines)
            .style(Style::default().fg(parse_color(&theme.foreground)).bg(parse_color(&theme.background)))
            .block(
                Block::default()
                    .title(Span::styled(" syntax.js ", Style::default().fg(chrome.muted)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(chrome.border)),
            ),
        demo_cols[0],
    );

    let shell_lines = widgets::shell_log_lines(theme);
    frame.render_widget(
        Paragraph::new(shell_lines)
            .style(Style::default().fg(parse_color(&theme.foreground)).bg(parse_color(&theme.background)))
            .block(
                Block::default()
                    .title(Span::styled(" shell.log ", Style::default().fg(chrome.muted)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(chrome.border)),
            ),
        demo_cols[1],
    );

    // Accessibility
    widgets::heading(frame, chunks[8], "accessibility", chrome);
    let report_lines = widgets::contrast_report_lines(theme, chrome);
    frame.render_widget(
        Paragraph::new(report_lines).style(Style::default().bg(chrome.bg)),
        chunks[9],
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Handle delete confirmation
    if let Some(slug) = app.pending_delete_slug.clone() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let deleted = if app.themes.iter().any(|t| {
                    t.slug == slug && matches!(t.source, ThemeSource::BuiltInOverride)
                }) {
                    config::delete_builtin_override(&slug)?
                } else {
                    config::delete_custom_theme(&slug)?
                };
                app.pending_delete_slug = None;
                if deleted {
                    app.reload_themes()?;
                    app.status = format!("Deleted '{}'.", slug);
                }
            }
            _ => {
                app.pending_delete_slug = None;
                app.status = "Delete cancelled.".to_string();
            }
        }
        return Ok(false);
    }

    // Search mode
    if app.lib_search_active {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.lib_search_active = false;
            }
            KeyCode::Backspace => {
                app.lib_search_query.pop();
                recompute_filtered(app);
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.lib_search_query.push(ch);
                recompute_filtered(app);
            }
            _ => {}
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('h') => {
            app.mode = AppMode::Home;
            app.status = "Returned home.".to_string();
        }
        KeyCode::Char('/') => {
            app.lib_search_active = true;
        }
        KeyCode::Char('f') | KeyCode::Tab => {
            app.lib_filter = app.lib_filter.next();
            recompute_filtered(app);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            navigate_filtered(app, 1)?;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            navigate_filtered(app, -1)?;
        }
        KeyCode::Enter | KeyCode::Char('a') => {
            if let Some(theme) = app.themes.get(app.selected) {
                super::save_selected_entry(theme)?;
                super::apply_entry_theme(theme)?;
                app.status = format!("Applied {}.", theme.name);
            }
        }
        KeyCode::Char('p') => {
            if let Some(theme) = app.themes.get(app.selected) {
                super::apply_entry_theme(theme)?;
                app.status = format!("Previewing {}.", theme.name);
            }
        }
        KeyCode::Char('e') => {
            if let Some(theme) = app.themes.get(app.selected).cloned() {
                app.mode = AppMode::Creator;
                app.creator = super::CreatorState::from_theme(&theme);
                app.editing_slug = Some(theme.slug);
                app.status = "Editing theme.".to_string();
            }
        }
        KeyCode::Char('n') => {
            app.mode = AppMode::Creator;
            app.creator = super::CreatorState::default();
            app.editing_slug = None;
            app.status = "Theme creator launched.".to_string();
        }
        KeyCode::Char('d') => {
            if let Some(theme) = app.themes.get(app.selected) {
                match theme.source {
                    ThemeSource::Custom | ThemeSource::BuiltInOverride => {
                        app.pending_delete_slug = Some(theme.slug.clone());
                        app.status = format!("Delete '{}'? y/n", theme.slug);
                    }
                    ThemeSource::BuiltIn => {
                        app.status = "Cannot delete built-in theme.".to_string();
                    }
                }
            }
        }
        _ => {}
    }

    Ok(false)
}

fn navigate_filtered(app: &mut App, direction: i32) -> Result<()> {
    if app.lib_filtered_indices.is_empty() {
        return Ok(());
    }
    let current_pos = app
        .lib_filtered_indices
        .iter()
        .position(|&i| i == app.selected)
        .unwrap_or(0);
    let new_pos = if direction > 0 {
        (current_pos + 1) % app.lib_filtered_indices.len()
    } else {
        if current_pos == 0 {
            app.lib_filtered_indices.len() - 1
        } else {
            current_pos - 1
        }
    };
    app.selected = app.lib_filtered_indices[new_pos];
    if let Some(theme) = app.themes.get(app.selected) {
        super::apply_entry_theme(theme)?;
        app.status = format!("Previewing {}.", theme.name);
    }
    Ok(())
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) -> Result<()> {
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return Ok(());
    }
    let x = mouse.column;
    let y = mouse.row;
    let area = app.list_area;
    if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
        return Ok(());
    }
    let idx = y.saturating_sub(area.y) as usize;
    if idx < app.lib_filtered_indices.len() {
        app.selected = app.lib_filtered_indices[idx];
        if let Some(theme) = app.themes.get(app.selected) {
            super::apply_entry_theme(theme)?;
            app.status = format!("Previewing {}.", theme.name);
        }
    }
    Ok(())
}
