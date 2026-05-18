use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::colors::{
    self, hex_to_rgb, is_valid_hex, lighten, mix, normalize_hex, parse_color, parse_color_input,
    rgb_to_hex, scale, ChromeColors,
};
use super::widgets;
use super::{App, AppMode, CreatorFocus, CreatorState, ThemeEntry, ThemeSource};
use crate::{config, contrast, theme};

impl CreatorState {
    pub fn from_theme(theme: &ThemeEntry) -> Self {
        Self {
            name: format!("{} fork", theme.name),
            mood: theme.mood.clone(),
            background: theme.background.clone(),
            foreground: theme.foreground.clone(),
            cursor: theme.cursor.clone(),
            selection: theme.selection.clone(),
            accent: theme.accent.clone().unwrap_or_else(|| theme.ansi[3].clone()),
            success: theme.success_color().to_string(),
            warning: theme.warning_color().to_string(),
            error: theme.error_color().to_string(),
            ansi_auto: true,
            ansi: theme.ansi.clone(),
            error_msg: None,
        }
    }

    fn active_field_value(&self, focus: &CreatorFocus) -> &str {
        match focus {
            CreatorFocus::Name => &self.name,
            CreatorFocus::Background => &self.background,
            CreatorFocus::Foreground => &self.foreground,
            CreatorFocus::Cursor => &self.cursor,
            CreatorFocus::Selection => &self.selection,
            CreatorFocus::Accent => &self.accent,
            CreatorFocus::Success => &self.success,
            CreatorFocus::Warning => &self.warning,
            CreatorFocus::Error => &self.error,
            _ => "",
        }
    }

    fn active_field_mut(&mut self, focus: &CreatorFocus) -> Option<&mut String> {
        match focus {
            CreatorFocus::Name => Some(&mut self.name),
            CreatorFocus::Background => Some(&mut self.background),
            CreatorFocus::Foreground => Some(&mut self.foreground),
            CreatorFocus::Cursor => Some(&mut self.cursor),
            CreatorFocus::Selection => Some(&mut self.selection),
            CreatorFocus::Accent => Some(&mut self.accent),
            CreatorFocus::Success => Some(&mut self.success),
            CreatorFocus::Warning => Some(&mut self.warning),
            CreatorFocus::Error => Some(&mut self.error),
            _ => None,
        }
    }

    pub fn to_theme_entry(&self) -> ThemeEntry {
        let ansi = if self.ansi_auto {
            derive_ansi(&self.background, &self.foreground, &self.cursor, &self.selection)
        } else {
            self.ansi.clone()
        };
        ThemeEntry {
            name: self.name.clone(),
            slug: slugify(&self.name),
            foreground: self.foreground.clone(),
            background: self.background.clone(),
            cursor: self.cursor.clone(),
            selection: self.selection.clone(),
            ansi,
            source: ThemeSource::Custom,
            description: String::new(),
            author: "you".to_string(),
            kind: "custom".to_string(),
            mood: self.mood.clone(),
            accent: Some(self.accent.clone()),
            success: Some(self.success.clone()),
            warning: Some(self.warning.clone()),
            error: Some(self.error.clone()),
        }
    }
}

fn derive_ansi(bg: &str, fg: &str, cursor: &str, selection: &str) -> [String; 16] {
    let bg = hex_to_rgb(bg);
    let fg = hex_to_rgb(fg);
    let cs = hex_to_rgb(cursor);
    let sel = hex_to_rgb(selection);
    [
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
    ]
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

    let right_text = if app.editing_slug.is_some() {
        format!("● unsaved · editing {}", app.creator.name)
    } else {
        "● unsaved".to_string()
    };
    widgets::top_bar(frame, root[0], &["schemer", "new theme"], &right_text, chrome);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(44), Constraint::Min(40)])
        .split(root[1]);

    draw_form(frame, body[0], app, chrome);
    draw_live_preview(frame, body[1], app, chrome);

    widgets::keybinds_bar(
        frame,
        root[2],
        &[
            ("Tab", "next"),
            ("s", "save"),
            ("p", "try"),
            ("r", "random"),
            ("Esc", "back"),
        ],
        &format!("editing: {}", app.creator.name),
        chrome,
    );
}

fn draw_form(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });

    let fields_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // heading identity
            Constraint::Length(1),  // name
            Constraint::Length(1),  // mood
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading core
            Constraint::Length(1),  // bg
            Constraint::Length(1),  // fg
            Constraint::Length(1),  // cursor
            Constraint::Length(1),  // selection
            Constraint::Length(1),  // accent
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading semantic
            Constraint::Length(1),  // success
            Constraint::Length(1),  // warning
            Constraint::Length(1),  // error
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // heading ansi
            Constraint::Length(1),  // ansi toggle
            Constraint::Length(2),  // ansi palette
            Constraint::Min(0),    // rest
        ])
        .split(inner);

    widgets::heading(frame, fields_layout[0], "identity", chrome);
    draw_text_field(frame, fields_layout[1], "name", &app.creator.name, app.creator_focus == CreatorFocus::Name, chrome);
    draw_mood_field(frame, fields_layout[2], &app.creator.mood, app.creator_focus == CreatorFocus::Mood, chrome);

    widgets::heading(frame, fields_layout[4], "core colors", chrome);
    draw_color_field(frame, fields_layout[5], "bg", &app.creator.background, app.creator_focus == CreatorFocus::Background, chrome);
    draw_color_field(frame, fields_layout[6], "fg", &app.creator.foreground, app.creator_focus == CreatorFocus::Foreground, chrome);
    draw_color_field(frame, fields_layout[7], "cursor", &app.creator.cursor, app.creator_focus == CreatorFocus::Cursor, chrome);
    draw_color_field(frame, fields_layout[8], "select", &app.creator.selection, app.creator_focus == CreatorFocus::Selection, chrome);
    draw_color_field(frame, fields_layout[9], "accent", &app.creator.accent, app.creator_focus == CreatorFocus::Accent, chrome);

    widgets::heading(frame, fields_layout[11], "semantic", chrome);
    draw_color_field(frame, fields_layout[12], "success", &app.creator.success, app.creator_focus == CreatorFocus::Success, chrome);
    draw_color_field(frame, fields_layout[13], "warning", &app.creator.warning, app.creator_focus == CreatorFocus::Warning, chrome);
    draw_color_field(frame, fields_layout[14], "error", &app.creator.error, app.creator_focus == CreatorFocus::Error, chrome);

    widgets::heading(frame, fields_layout[16], "ansi 16", chrome);

    // ANSI toggle
    let auto_style = if app.creator.ansi_auto {
        Style::default().fg(chrome.bg).bg(chrome.fg)
    } else {
        Style::default().fg(chrome.fg)
    };
    let manual_style = if !app.creator.ansi_auto {
        Style::default().fg(chrome.bg).bg(chrome.fg)
    } else {
        Style::default().fg(chrome.fg)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" auto-derive ", auto_style),
            Span::raw(" "),
            Span::styled(" manual ", manual_style),
        ]))
        .style(Style::default().bg(chrome.bg)),
        fields_layout[17],
    );

    // ANSI palette
    let live = app.creator.to_theme_entry();
    widgets::palette_row(frame, fields_layout[18], &live.ansi);

    // Error message
    if let Some(err) = &app.creator.error_msg {
        if fields_layout.len() > 19 && fields_layout[19].height > 0 {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    err,
                    Style::default()
                        .fg(parse_color(app.active_theme().error_color()))
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(chrome.bg)),
                fields_layout[19],
            );
        }
    }
}

fn draw_text_field(frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool, chrome: &ChromeColors) {
    let bg = if active { chrome.fg } else { chrome.bg };
    let fg = if active { chrome.bg } else { chrome.fg };
    let label_fg = if active { chrome.bg } else { chrome.muted };
    let cursor = if active { "▌" } else { "" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {:<10}", label), Style::default().fg(label_fg)),
            Span::styled(value, Style::default().fg(fg)),
            Span::styled(cursor, Style::default().fg(fg)),
        ]))
        .style(Style::default().bg(bg)),
        area,
    );
}

fn draw_color_field(frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool, chrome: &ChromeColors) {
    let bg = if active { chrome.fg } else { chrome.bg };
    let fg = if active { chrome.bg } else { chrome.fg };
    let label_fg = if active { chrome.bg } else { chrome.muted };
    let swatch_color = if is_valid_hex(&normalize_hex(value)) {
        parse_color(value)
    } else {
        chrome.border
    };
    let cursor = if active { "▌" } else { "" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {:<10}", label), Style::default().fg(label_fg)),
            Span::styled("  ", Style::default().bg(swatch_color)),
            Span::raw(" "),
            Span::styled(value, Style::default().fg(fg)),
            Span::styled(cursor, Style::default().fg(fg)),
        ]))
        .style(Style::default().bg(bg)),
        area,
    );
}

fn draw_mood_field(frame: &mut Frame, area: Rect, mood: &[String], active: bool, chrome: &ChromeColors) {
    let bg = if active { chrome.fg } else { chrome.bg };
    let label_fg = if active { chrome.bg } else { chrome.muted };
    let mut spans = vec![
        Span::styled(" mood      ", Style::default().fg(label_fg)),
    ];
    for tag in &["dark", "light", "warm", "cool", "neutral"] {
        let on = mood.iter().any(|m| m == tag);
        let style = if on {
            Style::default().fg(chrome.bg).bg(chrome.fg)
        } else {
            Style::default().fg(if active { chrome.bg } else { chrome.fg })
        };
        spans.push(Span::styled(format!(" {} ", tag), style));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)),
        area,
    );
}

fn draw_live_preview(frame: &mut Frame, area: Rect, app: &App, chrome: &ChromeColors) {
    let live = app.creator.to_theme_entry();
    let inner = area.inner(ratatui::layout::Margin { vertical: 0, horizontal: 1 });

    let c_fg = contrast::contrast_ratio(&live.background, &live.foreground);
    let c_acc = contrast::contrast_ratio(&live.background, &live.accent.as_deref().unwrap_or(&live.ansi[3]));
    let low_contrast = c_fg < 4.5;

    let constraints = if low_contrast {
        vec![
            Constraint::Length(1),  // warning
            Constraint::Length(1),  // heading
            Constraint::Min(8),    // preview box
            Constraint::Length(1),  // spacer
            Constraint::Length(3),  // metrics
        ]
    } else {
        vec![
            Constraint::Length(0),  // no warning
            Constraint::Length(1),  // heading
            Constraint::Min(8),    // preview box
            Constraint::Length(1),  // spacer
            Constraint::Length(3),  // metrics
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Low contrast warning
    if low_contrast {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(" ⚠ fg/bg contrast is {:.1}:1 — text will be hard to read.", c_fg),
                Style::default().fg(parse_color(&live.background)).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(parse_color(live.error_color()))),
            chunks[0],
        );
    }

    widgets::heading(frame, chunks[1], "live preview", chrome);

    // Preview box
    let preview_area = chunks[2];
    let preview_bg = parse_color(&live.background);
    let preview_fg = parse_color(&live.foreground);

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(chrome.border))
        .style(Style::default().bg(preview_bg).fg(preview_fg));
    let preview_inner = preview_block.inner(preview_area);
    frame.render_widget(preview_block, preview_area);

    let preview_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // path bar
            Constraint::Length(5),  // code + shell
            Constraint::Length(1),  // selection demo
            Constraint::Length(1),  // semantic dots
            Constraint::Min(1),    // ansi strip
        ])
        .split(preview_inner);

    // Path bar
    let cursor_color = parse_color(&live.cursor);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" ~/projects/{} ", slugify(&live.name)),
                Style::default().fg(parse_color(&live.ansi.get(8).unwrap_or(&live.foreground))),
            ),
            Span::styled("█", Style::default().fg(cursor_color)),
        ]))
        .style(Style::default().bg(preview_bg)),
        preview_rows[0],
    );

    // Code + shell
    let demo_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(preview_rows[1]);

    let code = widgets::code_sample_lines(&live);
    frame.render_widget(
        Paragraph::new(code).style(Style::default().bg(preview_bg).fg(preview_fg)),
        demo_cols[0],
    );
    let shell = widgets::shell_log_lines(&live);
    frame.render_widget(
        Paragraph::new(shell).style(Style::default().bg(preview_bg).fg(preview_fg)),
        demo_cols[1],
    );

    // Selection demo
    let sel_bg = parse_color(&live.selection);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" normal text · ", Style::default().fg(preview_fg)),
            Span::styled(" selected text ", Style::default().fg(preview_fg).bg(sel_bg)),
            Span::styled(" · normal", Style::default().fg(preview_fg)),
        ]))
        .style(Style::default().bg(preview_bg)),
        preview_rows[2],
    );

    // Semantic dots
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ● success", Style::default().fg(parse_color(&live.success.as_deref().unwrap_or(&live.ansi[2])))),
            Span::raw("  "),
            Span::styled("● warning", Style::default().fg(parse_color(&live.warning.as_deref().unwrap_or(&live.ansi[3])))),
            Span::raw("  "),
            Span::styled("● error", Style::default().fg(parse_color(&live.error.as_deref().unwrap_or(&live.ansi[1])))),
            Span::raw("  "),
            Span::styled("● accent", Style::default().fg(parse_color(&live.accent.as_deref().unwrap_or(&live.ansi[3])))),
        ]))
        .style(Style::default().bg(preview_bg)),
        preview_rows[3],
    );

    // ANSI strip
    if preview_rows[4].height > 0 {
        widgets::palette_row(frame, preview_rows[4], &live.ansi);
    }

    // Metrics
    let metric_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[4]);

    draw_metric(frame, metric_cols[0], "fg / bg", c_fg, 4.5, chrome);
    draw_metric(frame, metric_cols[1], "accent / bg", c_acc, 3.0, chrome);

    let pass_count = [c_fg >= 4.5, c_acc >= 3.0].iter().filter(|&&x| x).count();
    let overall = format!("{}/2 pass", pass_count);
    let overall_ok = pass_count == 2;
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("OVERALL", Style::default().fg(chrome.muted))),
            Line::from(Span::styled(
                overall,
                Style::default()
                    .fg(if overall_ok { parse_color("#A8C060") } else { parse_color("#E8B060") })
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(chrome.border)),
        )
        .style(Style::default().bg(chrome.bg)),
        metric_cols[2],
    );
}

fn draw_metric(frame: &mut Frame, area: Rect, label: &str, value: f64, threshold: f64, chrome: &ChromeColors) {
    let ok = value >= threshold;
    let color = if ok { parse_color("#A8C060") } else { parse_color("#E8B060") };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                label.to_ascii_uppercase(),
                Style::default().fg(chrome.muted),
            )),
            Line::from(Span::styled(
                format!("{:.1}:1", value),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(chrome.border)),
        )
        .style(Style::default().bg(chrome.bg)),
        area,
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.code == KeyCode::Esc {
        app.mode = AppMode::Library;
        app.creator = CreatorState::default();
        app.editing_slug = None;
        app.status = "Creator cancelled.".to_string();
        return Ok(false);
    }

    // Save
    if key.code == KeyCode::Char('s') && !matches!(app.creator_focus, CreatorFocus::Name) {
        return save_theme(app);
    }

    // Preview live
    if key.code == KeyCode::Char('p') && !matches!(app.creator_focus, CreatorFocus::Name) {
        let live = app.creator.to_theme_entry();
        super::apply_entry_theme(&live)?;
        app.status = "Previewing draft.".to_string();
        return Ok(false);
    }

    // Randomize
    if key.code == KeyCode::Char('r') && !matches!(app.creator_focus, CreatorFocus::Name) {
        randomize(&mut app.creator);
        app.status = "Randomized colors.".to_string();
        return Ok(false);
    }

    // Tab navigation
    if key.code == KeyCode::Tab {
        app.creator_focus = app.creator_focus.next();
        return Ok(false);
    }
    if key.code == KeyCode::BackTab {
        app.creator_focus = app.creator_focus.prev();
        return Ok(false);
    }

    // Mood toggle
    if app.creator_focus == CreatorFocus::Mood {
        if let KeyCode::Char(ch) = key.code {
            let tag = match ch {
                'd' => Some("dark"),
                'l' => Some("light"),
                'w' => Some("warm"),
                'c' => Some("cool"),
                'n' => Some("neutral"),
                _ => None,
            };
            if let Some(t) = tag {
                let t = t.to_string();
                if app.creator.mood.contains(&t) {
                    app.creator.mood.retain(|m| m != &t);
                } else {
                    app.creator.mood.push(t);
                }
            }
        }
        return Ok(false);
    }

    // ANSI toggle
    if app.creator_focus == CreatorFocus::AnsiToggle {
        if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
            app.creator.ansi_auto = !app.creator.ansi_auto;
        }
        return Ok(false);
    }

    // Text input for other fields
    if let Some(field) = app.creator.active_field_mut(&app.creator_focus) {
        match key.code {
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                field.push(ch);
                app.creator.error_msg = None;
            }
            KeyCode::Backspace => {
                field.pop();
            }
            KeyCode::Enter => {
                if app.creator_focus != CreatorFocus::Name {
                    let normalized = colors::normalize_color_input(field);
                    if let Some(hex) = parse_color_input(&normalized) {
                        *field = hex;
                        app.creator.error_msg = None;
                        app.creator_focus = app.creator_focus.next();
                    } else {
                        app.creator.error_msg =
                            Some("Invalid color. Use #RRGGBB, name, or 0-255.".to_string());
                    }
                } else {
                    app.creator_focus = app.creator_focus.next();
                }
            }
            _ => {}
        }
    }

    Ok(false)
}

fn save_theme(app: &mut App) -> Result<bool> {
    let draft = &app.creator;
    if draft.name.trim().is_empty() {
        app.creator.error_msg = Some("Theme name is required.".to_string());
        return Ok(false);
    }

    let live = draft.to_theme_entry();
    let custom = live.to_custom_theme();
    let saved_slug = app.editing_slug.clone().unwrap_or_else(|| custom.slug.clone());

    if let Some(editing_slug) = &app.editing_slug {
        if theme::find_theme(editing_slug).is_some() {
            config::save_builtin_override(config::BuiltinOverride {
                name: custom.name.clone(),
                slug: editing_slug.clone(),
                foreground: custom.foreground,
                background: custom.background,
                cursor: custom.cursor,
                selection: custom.selection,
                ansi: custom.ansi,
                description: custom.description,
                author: custom.author,
                kind: custom.kind,
                mood: custom.mood,
                accent: custom.accent,
                success: custom.success,
                warning: custom.warning,
                error: custom.error,
            })?;
        } else {
            let mut ct = custom;
            ct.slug = editing_slug.clone();
            config::save_custom_theme(ct)?;
        }
    } else {
        config::save_custom_theme(custom)?;
    }

    app.reload_themes()?;
    let slug = saved_slug;
    app.selected = app
        .themes
        .iter()
        .position(|t| t.slug == slug)
        .unwrap_or(0);
    if let Some(t) = app.themes.get(app.selected) {
        super::apply_entry_theme(t)?;
    }
    app.mode = AppMode::Library;
    app.creator = CreatorState::default();
    app.editing_slug = None;
    app.status = format!("Saved {}.", slug);
    Ok(false)
}

fn randomize(state: &mut CreatorState) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();

    let hue = |s: u32| -> (u8, u8, u8) {
        let h = (s % 360) as f32;
        let s_val = 0.5 + (((s >> 4) % 40) as f32) / 100.0;
        let l = ((s >> 8) % 30) as f32 / 100.0 + 0.1;
        hsl_to_rgb(h, s_val, l)
    };

    let bg = hue(seed);
    let fg = lighten(bg, 0.7 + ((seed >> 12) % 20) as f32 / 100.0);
    let cursor = lighten(bg, 0.6);
    let selection = mix(bg, fg, 0.25);
    let accent = hue(seed.wrapping_mul(7));

    state.background = rgb_to_hex(bg);
    state.foreground = rgb_to_hex(fg);
    state.cursor = rgb_to_hex(cursor);
    state.selection = rgb_to_hex(selection);
    state.accent = rgb_to_hex(accent);
    state.success = rgb_to_hex(hue(seed.wrapping_add(120)));
    state.warning = rgb_to_hex(hue(seed.wrapping_add(60)));
    state.error = rgb_to_hex(hue(seed.wrapping_add(0)));
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}
