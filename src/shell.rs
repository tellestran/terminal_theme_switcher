use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

const START_MARKER: &str = "# switch-theme start";
const END_MARKER: &str = "# switch-theme end";
const HOOK_COMMAND: &str =
    "command -v switch-theme >/dev/null 2>&1 && switch-theme apply >/dev/null 2>&1";

pub fn init_snippet() -> String {
    HOOK_COMMAND.to_string()
}

#[allow(dead_code)]
pub fn suggested_profile_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let profile = if shell.contains("zsh") {
        ".zshrc"
    } else if shell.contains("bash") {
        ".bashrc"
    } else {
        ".profile"
    };
    Some(home.join(profile))
}

#[allow(dead_code)]
pub fn install_zsh_hook() -> Result<()> {
    let home = dirs::home_dir().context("could not find home directory")?;
    install_zsh_hook_at(home.join(".zshrc"))
}

pub fn install_zsh_hook_at(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let original = fs::read_to_string(path).unwrap_or_default();
    let updated = upsert_hook(&original);

    if updated != original {
        fs::write(path, updated).with_context(|| format!("failed to update {}", path.display()))?;
    }

    Ok(())
}

pub fn upsert_hook(contents: &str) -> String {
    let hook = hook_block();

    if let Some(start) = contents.find(START_MARKER) {
        if let Some(end_offset) = contents[start..].find(END_MARKER) {
            let end = start + end_offset + END_MARKER.len();
            let mut updated = String::new();
            updated.push_str(contents[..start].trim_end());
            if !updated.is_empty() {
                updated.push_str("\n\n");
            }
            updated.push_str(&hook);
            let tail = contents[end..].trim_start_matches(['\r', '\n']);
            if !tail.is_empty() {
                updated.push_str("\n\n");
                updated.push_str(tail);
            } else {
                updated.push('\n');
            }
            return updated;
        }
    }

    let mut updated = contents.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&hook);
    updated.push('\n');
    updated
}

fn hook_block() -> String {
    format!("{START_MARKER}\n{HOOK_COMMAND}\n{END_MARKER}")
}

#[allow(dead_code)]
fn _zshrc_path(home: PathBuf) -> PathBuf {
    home.join(".zshrc")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn appends_hook_to_empty_file() {
        let updated = upsert_hook("");

        assert_eq!(
            updated,
            "# switch-theme start\ncommand -v switch-theme >/dev/null 2>&1 && switch-theme apply >/dev/null 2>&1\n# switch-theme end\n"
        );
    }

    #[test]
    fn preserves_existing_shell_config() {
        let updated = upsert_hook("export PATH=\"$HOME/bin:$PATH\"\n");

        assert!(updated.starts_with("export PATH=\"$HOME/bin:$PATH\"\n\n# switch-theme start"));
    }

    #[test]
    fn replaces_existing_managed_hook() {
        let updated = upsert_hook(
            "before\n\n# switch-theme start\nold command\n# switch-theme end\n\nafter\n",
        );

        assert_eq!(
            updated,
            "before\n\n# switch-theme start\ncommand -v switch-theme >/dev/null 2>&1 && switch-theme apply >/dev/null 2>&1\n# switch-theme end\n\nafter\n"
        );
    }

    #[test]
    fn writes_hook_to_path() {
        let dir = tempdir().unwrap();
        let zshrc = dir.path().join(".zshrc");

        install_zsh_hook_at(&zshrc).unwrap();
        let contents = fs::read_to_string(zshrc).unwrap();

        assert!(contents.contains("# switch-theme start"));
        assert!(contents.contains("switch-theme apply"));
    }
}
