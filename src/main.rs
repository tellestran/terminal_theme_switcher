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
            let theme = theme::find_theme(&config.theme)
                .with_context(|| format!("unknown saved theme '{}'", config.theme))?;
            apply::apply_theme(std::io::stdout(), theme)?;
        }
        Some(Command::List) => {
            for theme in theme::themes() {
                println!("{}", theme.name);
            }
        }
        Some(Command::Current) => {
            let config = config::Config::load().context("failed to load switch-theme config")?;
            println!("{}", config.theme);
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
