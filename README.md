# switch-theme

Interactive terminal theme switcher for macOS and modern terminals.

`switch-theme` lets you preview bundled color themes, save one as default, and re-apply it quickly from the command line.

## Quick Install (curl)

```bash
curl -fsSL https://raw.githubusercontent.com/tellestran/terminal_theme_switcher/main/scripts/install.sh | bash
```

This installer:

- Installs Rust/Cargo via `rustup` if missing
- Installs `switch-theme` from this GitHub repo using `cargo install`

### What `install.sh` does

1. Checks whether `cargo` exists.
2. If missing, installs Rust using `rustup`.
3. Loads Cargo environment from `~/.cargo/env`.
4. Runs:

```bash
cargo install --locked --git https://github.com/tellestran/terminal_theme_switcher.git switch-theme
```

5. Prints a success message and suggests `switch-theme --help`.

### Optional: override installer variables

You can override defaults by exporting environment variables before running the script:

```bash
REPO_URL="https://github.com/your-org/switch-theme.git" BIN_NAME="switch-theme" \
  bash <(curl -fsSL https://raw.githubusercontent.com/tellestran/terminal_theme_switcher/main/scripts/install.sh)
```

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

This launches an interactive start menu:

This launches the interactive TUI picker. Press `c` inside the picker to create a new theme.

### Create New Theme Flow

When you press `c`, the app opens an in-TUI wizard and guides you step by step:

1. Shows color guidance and asks for 4 colors:
   - Background
   - Text
   - Accent 1
   - Accent 2
2. Prompts for a theme name.
3. Shows a review summary.
4. Asks for confirmation (`y`/`yes`) before creating.
5. Saves, applies, and sets it as current.

Custom themes are stored in the same config file and included in `list`.
Color input supports:
- `#RRGGBB`
- `RRGGBB` (auto `#`)
- Named colors like `red`, `blue`, `gray`
- ANSI 256 color indexes (`0`-`255`)

## CLI Usage

```bash
# List bundled themes
cargo run -- list

# Print saved/current theme
cargo run -- current

# Set active theme by slug or name (headless-safe)
cargo run -- set rose-fern

# Apply saved theme
cargo run -- apply

# Print shell init snippet (manual opt-in setup)
cargo run -- init

# Reset terminal palette
cargo run -- reset
```

## Interactive Controls

In the TUI:

- `↑` / `k`: previous theme
- `↓` / `j`: next theme
- `Enter`: save selected theme
- `c`: create new custom theme (guided wizard)
- `q` or `Esc`: quit
- `Ctrl+C`: quit/cancel (mobile-friendly fallback)

## Configuration

Saved theme config path:

```text
~/.config/switch-theme/config.toml
```

## Troubleshooting

- `cargo: command not found`
  - Install Rust and source Cargo env (`source "$HOME/.cargo/env"`).
- `switch-theme: command not found` after install
  - Your shell may not have Cargo bin on `PATH` yet.
  - Run `source "$HOME/.cargo/env"` and retry.
  - To persist, add `source "$HOME/.cargo/env"` to `~/.zshrc`.
- `interactive mode requires a TTY`
  - Run `cargo run` directly in a normal terminal session for interactive UI.
  - For non-interactive environments and automation, use explicit commands like:
    - `cargo run -- set <theme>`
    - `cargo run -- apply`
    - `cargo run -- list`
