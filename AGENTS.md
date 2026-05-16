# AGENTS.md

## Purpose

`switch-theme` is a Rust CLI/TUI app to preview, save, and apply terminal color themes.
It supports:

- Built-in themes
- User-created custom themes
- Headless automation commands (`set`, `apply`, `list`, `current`, `init`, `reset`)

## Stack

- Language: Rust (edition 2021)
- CLI parsing: `clap`
- TUI: `ratatui` + `crossterm`
- Config format: TOML via `serde` + `toml`

## Important Commands

- Format: `cargo fmt`
- Test: `cargo test`
- Run interactive TUI: `cargo run`
- List themes: `cargo run -- list`
- Set active theme (headless-safe): `cargo run -- set <theme>`
- Apply active theme: `cargo run -- apply`
- Print init snippet: `cargo run -- init`

## Code Map

- `src/main.rs`: command routing and headless/interactive entry behavior
- `src/cli.rs`: CLI command definitions
- `src/tui.rs`: interactive UX flow (picker + create-theme wizard)
- `src/theme.rs`: built-in themes
- `src/config.rs`: persisted config and custom theme storage
- `src/apply.rs`: ANSI escape sequence emission
- `src/shell.rs`: init snippet and hook helpers

## Behavioral Rules

1. Keep interactive logic inside `src/tui.rs`; avoid mixing `println!/read_line` flows with TUI mode handling.
2. Do not require TTY for explicit subcommands (`set`, `apply`, `list`, etc.).
3. Do not implicitly modify shell profiles on normal save/create actions.
4. Preserve compatibility for both built-in and custom themes in save/apply/list/current flows.
5. Keep mobile-friendly controls (`q`, `Ctrl+C`, mouse/tap where supported).

## Validation Checklist for Changes

Before finishing changes:

1. `cargo fmt`
2. `cargo test`
3. Manual sanity checks:
   - `cargo run` → picker works
   - press `c` → create wizard works
   - created theme appears immediately in picker
   - `cargo run -- set <theme>` works in non-interactive use

