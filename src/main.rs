mod apply;
mod cli;
mod config;
mod shell;
mod theme;
mod tui;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Apply) => {
            let config = config::Config::load().context("failed to load switch-theme config")?;
            if let Some(theme) = theme::find_theme(&config.theme) {
                apply::apply_theme(std::io::stdout(), theme)?;
            } else if let Some(theme) = config.custom_theme_by_slug(&config.theme) {
                apply::apply_custom_theme(std::io::stdout(), theme)?;
            } else {
                anyhow::bail!("unknown saved theme '{}'", config.theme);
            }
        }
        Some(Command::Set { theme: query }) => {
            if let Some(found) = theme::find_theme(&query) {
                config::save_selected_theme(found)?;
            } else {
                let mut cfg =
                    config::Config::load().context("failed to load switch-theme config")?;
                let normalized = normalize(&query);
                if cfg.custom_theme_by_slug(&normalized).is_some() {
                    cfg.theme = normalized;
                    cfg.save()?;
                } else {
                    anyhow::bail!("unknown theme '{}'", query);
                }
            }
        }
        Some(Command::List) => {
            for theme in theme::themes() {
                println!("{}", theme.name);
            }
            let config = config::Config::load().context("failed to load switch-theme config")?;
            for theme in config.custom_themes {
                println!("{}", theme.name);
            }
        }
        Some(Command::Current) => {
            let config = config::Config::load().context("failed to load switch-theme config")?;
            println!("{}", config.theme);
        }
        Some(Command::Init) => {
            println!("{}", shell::init_snippet());
        }
        Some(Command::Reset) => {
            apply::reset_theme(std::io::stdout())?;
        }
        None => {
            tui::run()?;
        }
    }

    Ok(())
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
