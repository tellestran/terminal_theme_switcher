#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/tellestran/terminal_theme_switcher.git}"
BIN_NAME="${BIN_NAME:-switch-theme}"

log() {
  printf '[switch-theme installer] %s\n' "$1"
}

ensure_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi

  log "Cargo not found. Installing Rust toolchain with rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
}

main() {
  ensure_cargo

  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

  log "Installing ${BIN_NAME} from ${REPO_URL}..."
  cargo install --locked --git "$REPO_URL" "$BIN_NAME"

  log "Installed successfully."
  log "Try: ${BIN_NAME} --help"
}

main "$@"
