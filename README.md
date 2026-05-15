# switch-theme

Interactive terminal theme switcher for macOS and modern terminals.

`switch-theme` lets you preview bundled color themes, save one as default, and re-apply it quickly from the command line.

## Features

- Interactive TUI theme picker
- Live preview while navigating themes
- Persist selected theme to user config
- Re-apply saved theme from CLI
- Reset terminal palette to default

## Requirements

- macOS (recommended)
- Rust toolchain (`cargo`, `rustc`)
- A terminal that supports ANSI escape sequences

## Install Rust (if needed)

If `cargo` is not installed:

```bash
brew install rustup-init
rustup-init -y --no-modify-path
source "$HOME/.cargo/env"
```

Optional: add Cargo to your shell startup:

```bash
echo 'source "$HOME/.cargo/env"' >> ~/.zshrc
source ~/.zshrc
```

## Build and Run

From the repository root:

```bash
cargo run
```

This launches the interactive TUI.

## CLI Usage

```bash
# List bundled themes
cargo run -- list

# Print saved/current theme
cargo run -- current

# Apply saved theme
cargo run -- apply

# Reset terminal palette
cargo run -- reset
```

## Interactive Controls

In the TUI:

- `↑` / `k`: previous theme
- `↓` / `j`: next theme
- `Enter`: save selected theme
- `q` or `Esc`: quit

## Configuration

Saved theme config path:

```text
~/.config/switch-theme/config.toml
```

## Troubleshooting

- `cargo: command not found`
  - Install Rust and source Cargo env (`source "$HOME/.cargo/env"`).
- `interactive mode requires a TTY`
  - Run `cargo run` directly in a normal terminal session.
  - For non-interactive environments, use subcommands like `cargo run -- list`.

