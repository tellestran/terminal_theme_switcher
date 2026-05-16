use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "switch-theme")]
#[command(about = "Preview, select, and persist terminal color themes")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Apply the saved theme to the current terminal session.
    Apply,
    /// Set the active theme by slug or name (works in headless/non-TTY).
    Set {
        /// Theme slug or name.
        theme: String,
    },
    /// List all bundled themes.
    List,
    /// Print the currently saved theme.
    Current,
    /// Print a shell init snippet for manual profile setup.
    Init,
    /// Ask the terminal to restore its default palette.
    Reset,
}
