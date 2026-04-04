use terminalsort::state::{SavedState, load_state, save_state};
use std::collections::HashMap;

#[test]
fn roundtrip_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");

    let mut profiles = HashMap::new();
    profiles.insert(
        "b1dcc9dd-5262-4d8d-a863-c897e6d979b9".to_string(),
        "Monospace 12".to_string(),
    );
    let state = SavedState { profiles };

    save_state(&path, &state).unwrap();
    let loaded = load_state(&path).unwrap();

    assert_eq!(loaded.profiles.len(), 1);
    assert_eq!(
        loaded.profiles["b1dcc9dd-5262-4d8d-a863-c897e6d979b9"],
        "Monospace 12"
    );
}

#[test]
fn load_missing_file_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");

    let state = load_state(&path).unwrap();
    assert!(state.profiles.is_empty());
}

#[test]
fn save_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("state.json");

    let state = SavedState {
        profiles: HashMap::new(),
    };
    save_state(&path, &state).unwrap();
    assert!(path.exists());
}
