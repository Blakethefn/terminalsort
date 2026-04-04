use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedState {
    pub profiles: HashMap<String, String>,
}

impl SavedState {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }
}

/// Load saved state from a JSON file. Returns empty state if file doesn't exist.
pub fn load_state(path: &Path) -> Result<SavedState> {
    if !path.exists() {
        return Ok(SavedState::new());
    }
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read state file: {}", path.display()))?;
    let state: SavedState = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse state file: {}", path.display()))?;
    Ok(state)
}

/// Save state to a JSON file. Creates parent directories if needed.
pub fn save_state(path: &Path, state: &SavedState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
    }
    let contents = serde_json::to_string_pretty(state)?;
    std::fs::write(path, contents)
        .with_context(|| format!("Failed to write state file: {}", path.display()))?;
    Ok(())
}

/// Get the default state file path: ~/.local/state/terminalsort/state.json
pub fn default_state_path() -> Result<std::path::PathBuf> {
    let state_dir = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))
        .context("Cannot determine state directory")?;
    Ok(state_dir.join("terminalsort").join("state.json"))
}
