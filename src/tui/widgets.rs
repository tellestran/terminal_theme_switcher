use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::colors::{parse_color, ChromeColors};
use super::ThemeEntry;
use crate::contrast;

pub fn top_bar(frame: &mut Frame, area: Rect, crumbs: &[&str], right_text: &str, chrome: &ChromeColors) {
    let mut spans = Vec::new();
    for (i, c) in crumbs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" / ", Style::default().fg(chrome.muted)));
        }
        let style = if i == crumbs.len() - 1 {
            Style::default().fg(chrome.fg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(chrome.muted)
        };
        spans.push(Span::styled(*c, style));
    }

    let left = Line::from(spans);
    let right = Line::from(Span::styled(right_text, Style::default().fg(chrome.muted)));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(right_text.len() as u16 + 2)])
        .split(area);

    frame.render_widget(
        Paragraph::new(left).style(Style::default().bg(chrome.bg)),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(right)
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().bg(chrome.bg)),
        cols[1],
    );
}

pub fn keybinds_bar(
    frame: &mut Frame,
    area: Rect,
    bindings: &[(&str, &str)],
    status: &str,
    chrome: &ChromeColors,
) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default().fg(chrome.bg).bg(chrome.fg).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label}"), Style::default().fg(chrome.fg)));
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(status.len() as u16 + 2)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(chrome.bg)),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(status, Style::default().fg(chrome.muted))))
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().bg(chrome.bg)),
        cols[1],
    );
}

pub fn heading(frame: &mut Frame, area: Rect, text: &str, chrome: &ChromeColors) {
    let label = format!(" \u{25b8} {} ", text.to_ascii_uppercase());
    let dashes_len = area.width.saturating_sub(label.len() as u16 + 1) as usize;
    let dashes: String = "\u{2500}".repeat(dashes_len);
    let line = Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .fg(chrome.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(dashes, Style::default().fg(chrome.border)),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(chrome.bg)),
        area,
    );
}

pub fn palette_row(frame: &mut Frame, area: Rect, colors: &[String]) {
    let half = colors.len() / 2;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    for (row_idx, row_area) in rows.iter().enumerate() {
        let start = row_idx * half;
        let end = (start + half).min(colors.len());
        let row_colors = &colors[start..end];
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(3); row_colors.len()])
            .split(*row_area);
        for (i, col_area) in cols.iter().enumerate() {
            let color = parse_color(&row_colors[i]);
            frame.render_widget(
                Paragraph::new("  ").style(Style::default().bg(color)),
                *col_area,
            );
        }
    }
}

pub fn code_sample_lines(theme: &ThemeEntry) -> Vec<Line<'static>> {
    let a = &theme.ansi;
    vec![
        Line::from(vec![
            Span::styled("// ", Style::default().fg(parse_color(&a[8]))),
            Span::styled("load theme on boot", Style::default().fg(parse_color(&a[8]))),
        ]),
        Line::from(vec![
            Span::styled("function", Style::default().fg(parse_color(&a[4]))),
            Span::styled(" ", Style::default().fg(parse_color(&a[7]))),
            Span::styled("initTerminal", Style::default().fg(parse_color(&a[5]))),
            Span::styled("() {", Style::default().fg(parse_color(&a[7]))),
        ]),
        Line::from(vec![
            Span::styled("  const", Style::default().fg(parse_color(&a[4]))),
            Span::styled(" cfg = ", Style::default().fg(parse_color(&a[7]))),
            Span::styled("load", Style::default().fg(parse_color(&a[5]))),
            Span::styled("(", Style::default().fg(parse_color(&a[7]))),
            Span::styled("\"~/cfg.toml\"", Style::default().fg(parse_color(&a[2]))),
            Span::styled(");", Style::default().fg(parse_color(&a[7]))),
        ]),
        Line::from(vec![
            Span::styled("  if", Style::default().fg(parse_color(&a[4]))),
            Span::styled(" (!cfg) ", Style::default().fg(parse_color(&a[7]))),
            Span::styled("throw", Style::default().fg(parse_color(&a[1]))),
            Span::styled(" ", Style::default().fg(parse_color(&a[7]))),
            Span::styled("new", Style::default().fg(parse_color(&a[4]))),
            Span::styled(" Error", Style::default().fg(parse_color(&a[3]))),
            Span::styled(";", Style::default().fg(parse_color(&a[7]))),
        ]),
        Line::from(vec![
            Span::styled("  apply", Style::default().fg(parse_color(&a[5]))),
            Span::styled("(cfg.theme);", Style::default().fg(parse_color(&a[7]))),
        ]),
        Line::from(vec![
            Span::styled("}", Style::default().fg(parse_color(&a[7]))),
        ]),
    ]
}

pub fn shell_log_lines(theme: &ThemeEntry) -> Vec<Line<'static>> {
    let a = &theme.ansi;
    vec![
        Line::from(vec![
            Span::styled("user@host", Style::default().fg(parse_color(&a[2]))),
            Span::styled(":", Style::default().fg(parse_color(&a[8]))),
            Span::styled("~/proj", Style::default().fg(parse_color(&a[4]))),
            Span::styled("$ ", Style::default().fg(parse_color(&a[8]))),
            Span::styled("./run_tests.sh", Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(vec![
            Span::styled("[INFO]", Style::default().fg(parse_color(&a[6]))),
            Span::styled(" loading config… ", Style::default().fg(parse_color(&theme.foreground))),
            Span::styled("OK", Style::default().fg(parse_color(&a[2]))),
        ]),
        Line::from(vec![
            Span::styled("[WARN]", Style::default().fg(parse_color(&a[3]))),
            Span::styled(" deprecated --fast", Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(vec![
            Span::styled("[PASS]", Style::default().fg(parse_color(&a[2]))),
            Span::styled(" test suite 1", Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(vec![
            Span::styled("[FAIL]", Style::default().fg(parse_color(&a[1]))),
            Span::styled(" test suite 2 — timeout", Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(vec![
            Span::styled("summary", Style::default().fg(parse_color(&a[5]))),
            Span::styled(" 47 passed, 1 failed", Style::default().fg(parse_color(&theme.foreground))),
        ]),
    ]
}

pub fn contrast_report_lines(theme: &ThemeEntry, chrome: &ChromeColors) -> Vec<Line<'static>> {
    let accent = theme.accent.as_deref().unwrap_or(&theme.ansi[3]).to_string();
    let dim = theme.ansi.get(8).unwrap_or(&theme.foreground).to_string();
    let success = theme.success_color().to_string();
    let error_c = theme.error_color().to_string();
    let pairs: Vec<(&str, &String, &String)> = vec![
        ("fg on bg", &theme.foreground, &theme.background),
        ("cursor/bg", &theme.cursor, &theme.background),
        ("accent/bg", &accent, &theme.background),
        ("dim on bg", &dim, &theme.background),
        ("success/bg", &success, &theme.background),
        ("error/bg", &error_c, &theme.background),
    ];

    pairs
        .iter()
        .map(|(label, a, b)| {
            let ratio = contrast::contrast_ratio(a, b);
            let grade = contrast::contrast_grade(ratio);
            let color = if ratio >= 4.5 {
                parse_color(theme.success_color())
            } else if ratio >= 3.0 {
                parse_color(theme.warning_color())
            } else {
                parse_color(theme.error_color())
            };
            Line::from(vec![
                Span::styled(format!("{:<12}", label), Style::default().fg(chrome.muted)),
                Span::styled(
                    format!("{:>5.1}:1 {}", ratio, grade),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect()
}
