mod apply;
mod cli;
mod config;
mod contrast;
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
            if let Some(theme) = config.resolved_theme_by_query(&config.theme) {
                apply::apply_custom_theme(
                    std::io::stdout(),
                    &config::CustomTheme {
                        name: theme.name,
                        slug: theme.slug,
                        foreground: theme.foreground,
                        background: theme.background,
                        cursor: theme.cursor,
                        selection: theme.selection,
                        ansi: theme.ansi,
                        description: theme.description,
                        author: theme.author,
                        kind: theme.kind,
                        mood: theme.mood,
                        accent: theme.accent,
                        success: theme.success,
                        warning: theme.warning,
                        error: theme.error,
                    },
                )?;
            } else {
                anyhow::bail!("unknown saved theme '{}'", config.theme);
            }
        }
        Some(Command::Set { theme: query }) => {
            let mut cfg = config::Config::load().context("failed to load switch-theme config")?;
            if let Some(found) = cfg.resolved_theme_by_query(&query) {
                cfg.theme = found.slug;
                cfg.save()?;
            } else {
                anyhow::bail!("unknown theme '{}'", query);
            }
        }
        Some(Command::List) => {
            let config = config::Config::load().context("failed to load switch-theme config")?;
            for theme in config.resolved_themes() {
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
