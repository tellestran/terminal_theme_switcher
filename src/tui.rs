use std::{
    io::{self, IsTerminal},
    process::Command,
    time::Duration,
};

use anyhow::{Result, bail};
use crossterm::{
    event::{self, Event, KeyCode},
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
    apply, config, shell,
    theme::{self, Theme},
};

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
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let themes = theme::themes();
    let saved = config::Config::load().unwrap_or_default();
    let mut selected = themes
        .iter()
        .position(|theme| theme.slug == saved.theme)
        .unwrap_or(0);
    let mut state = ListState::default();
    let mut status = "Use arrows or j/k to preview, Enter to save, q/Esc to quit.".to_string();
    let branch = current_git_branch();

    apply::apply_theme(io::stdout(), &themes[selected])?;

    loop {
        state.select(Some(selected));
        terminal.draw(|frame| draw(frame, themes, selected, &mut state, &status, &branch))?;

        if event::poll(Duration::from_millis(160))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % themes.len();
                        apply::apply_theme(io::stdout(), &themes[selected])?;
                        status = format!("Previewing {}.", themes[selected].name);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.checked_sub(1).unwrap_or(themes.len() - 1);
                        apply::apply_theme(io::stdout(), &themes[selected])?;
                        status = format!("Previewing {}.", themes[selected].name);
                    }
                    KeyCode::Enter => {
                        config::save_selected_theme(&themes[selected])?;
                        shell::install_zsh_hook()?;
                        apply::apply_theme(io::stdout(), &themes[selected])?;
                        status = format!(
                            "Saved {} and installed the zsh startup hook.",
                            themes[selected].name
                        );
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(
    frame: &mut Frame,
    themes: &[Theme],
    selected: usize,
    state: &mut ListState,
    status: &str,
    branch: &str,
) {
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

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "switch-theme",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  terminal palette picker"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, outer[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(40)])
        .split(outer[1]);

    draw_theme_list(frame, columns[0], themes, state);
    draw_preview(frame, columns[1], &themes[selected], branch);

    let footer = Paragraph::new(status.to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, outer[2]);
}

fn draw_theme_list(frame: &mut Frame, area: Rect, themes: &[Theme], state: &mut ListState) {
    let items = themes.iter().map(|theme| {
        ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default().bg(parse_color(theme.ansi[4]))),
            Span::raw(" "),
            Span::raw(theme.name),
        ]))
    });

    let list = List::new(items)
        .block(Block::default().title(" Themes ").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">");

    frame.render_stateful_widget(list, area, state);
}

fn draw_preview(frame: &mut Frame, area: Rect, theme: &Theme, branch: &str) {
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
            theme.name,
            Style::default()
                .fg(parse_color(theme.ansi[12]))
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

fn demo_lines(theme: &Theme, area: Rect, branch: &str) -> Vec<Line<'static>> {
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

fn compact_demo_lines(theme: &Theme, branch: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("$ ", Style::default().fg(parse_color(theme.ansi[10]))),
            Span::styled(
                "git status --short",
                Style::default().fg(parse_color(theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                branch.to_string(),
                Style::default().fg(parse_color(theme.ansi[12])),
            ),
            Span::raw(" "),
            Span::styled(
                "+ src/theme.rs",
                Style::default().fg(parse_color(theme.ansi[10])),
            ),
            Span::raw(" "),
            Span::styled(
                "~ src/tui.rs",
                Style::default().fg(parse_color(theme.ansi[11])),
            ),
            Span::raw(" "),
            Span::styled(
                "- old-preview.rs",
                Style::default().fg(parse_color(theme.ansi[9])),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "ok",
                Style::default()
                    .fg(parse_color(theme.ansi[10]))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" tests passed  "),
            Span::styled(
                "warn",
                Style::default()
                    .fg(parse_color(theme.ansi[11]))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" restart shell  "),
            Span::styled(
                "err",
                Style::default()
                    .fg(parse_color(theme.ansi[9]))
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

fn status_demo_lines(theme: &Theme, wide: bool) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            badge("normal", theme.ansi[7], theme.ansi[0]),
            Span::raw(" "),
            badge("red", theme.ansi[1], theme.ansi[15]),
            Span::raw(" "),
            badge("green", theme.ansi[2], theme.ansi[0]),
            Span::raw(" "),
            badge("yellow", theme.ansi[3], theme.ansi[0]),
        ]),
        Line::from(vec![
            badge("blue", theme.ansi[4], theme.ansi[15]),
            Span::raw(" "),
            badge("magenta", theme.ansi[5], theme.ansi[15]),
            Span::raw(" "),
            badge("cyan", theme.ansi[6], theme.ansi[0]),
            Span::raw(" "),
            badge("bright", theme.ansi[15], theme.ansi[0]),
        ]),
    ];

    if wide {
        lines.push(Line::from(vec![
            Span::styled(
                "selection",
                Style::default().fg(parse_color(theme.selection)),
            ),
            Span::raw("  "),
            Span::styled(
                "cursor",
                Style::default()
                    .fg(parse_color(theme.cursor))
                    .add_modifier(Modifier::REVERSED),
            ),
            Span::raw("  "),
            Span::styled(
                "foreground",
                Style::default().fg(parse_color(theme.foreground)),
            ),
            Span::raw(" on "),
            Span::styled(
                "background",
                Style::default()
                    .fg(parse_color(theme.foreground))
                    .bg(parse_color(theme.background)),
            ),
        ]));
    }

    lines
}

fn code_demo_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("fn ", Style::default().fg(parse_color(theme.ansi[5]))),
            Span::styled(
                "apply_theme",
                Style::default().fg(parse_color(theme.ansi[12])),
            ),
            Span::raw("("),
            Span::styled("palette", Style::default().fg(parse_color(theme.ansi[14]))),
            Span::raw(": "),
            Span::styled("&Theme", Style::default().fg(parse_color(theme.ansi[11]))),
            Span::raw(") -> "),
            Span::styled(
                "Result<()>",
                Style::default().fg(parse_color(theme.ansi[10])),
            ),
            Span::raw(" {"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("emit", Style::default().fg(parse_color(theme.ansi[12]))),
            Span::raw("("),
            Span::styled(
                "\"OSC 4;10;#...\"",
                Style::default().fg(parse_color(theme.ansi[10])),
            ),
            Span::raw(");"),
            Span::raw(" "),
            Span::styled(
                "// preview first",
                Style::default().fg(parse_color(theme.ansi[8])),
            ),
        ]),
    ]
}

fn log_demo_lines(theme: &Theme, wide: bool) -> Vec<Line<'static>> {
    let timing = if wide { "  42ms" } else { "" };
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("INFO ", Style::default().fg(parse_color(theme.ansi[6]))),
            Span::raw("loaded 8 themes"),
            Span::styled(timing, Style::default().fg(parse_color(theme.ansi[8]))),
        ]),
        Line::from(vec![
            Span::styled("PASS ", Style::default().fg(parse_color(theme.ansi[2]))),
            Span::raw("palette contrast sample"),
            Span::styled(timing, Style::default().fg(parse_color(theme.ansi[8]))),
        ]),
        Line::from(vec![
            Span::styled("NEXT ", Style::default().fg(parse_color(theme.ansi[4]))),
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

fn draw_swatches(frame: &mut Frame, area: Rect, theme: &Theme) {
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
                .style(Style::default().bg(parse_color(theme.ansi[index])));
            frame.render_widget(block, cols[col]);
        }
    }
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
