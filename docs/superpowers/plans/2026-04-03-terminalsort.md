# terminalsort Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a pure Rust CLI tool that tiles GNOME Terminal windows into even grid layouts on X11 with automatic font scaling via gsettings.

**Architecture:** Six focused modules — layout math (pure, testable), X11 window ops (x11rb), monitor detection (RANDR), click-to-pick window selection, font scaling (gsettings CLI), and state persistence (JSON). The CLI dispatches to these modules via clap commands.

**Tech Stack:** Rust, x11rb 0.13 (pure Rust X11), clap 4 (CLI), serde/serde_json (state), dirs 6 (XDG paths), anyhow (errors), gsettings CLI (font control)

**Key research finding:** GNOME Terminal font settings live in dconf. Pure D-Bus access requires GVariant binary serialization — impractical. We shell out to `gsettings` CLI instead (always present on GNOME). This is the only non-Rust dependency and it's a system utility, not a linked library.

---

### Task 1: Project setup and shared types

**Files:**
- Modify: `Cargo.toml`
- Create: `src/types.rs`
- Create: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Update Cargo.toml with dependencies**

```toml
[package]
name = "terminalsort"
version = "0.1.0"
edition = "2021"
description = "CLI tool for tiling and resizing terminal windows with font scaling on X11"

[dependencies]
x11rb = { version = "0.13", features = ["randr"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
anyhow = "1"
```

- [ ] **Step 2: Create src/types.rs with shared types**

```rust
/// A monitor's geometry as reported by RANDR.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub index: usize,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

/// A window rectangle in absolute screen coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A terminal window discovered via X11.
#[derive(Debug, Clone)]
pub struct TerminalWindow {
    pub id: u32,
    pub title: String,
}
```

- [ ] **Step 3: Create src/lib.rs to expose modules**

```rust
pub mod types;
pub mod layout;
pub mod font;
pub mod state;
```

Note: `x11`, `monitor`, and `picker` modules will be added in later tasks. We only add modules to lib.rs when they exist.

- [ ] **Step 4: Update src/main.rs with minimal clap skeleton**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terminalsort", about = "Tile terminal windows with font scaling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tile selected terminal windows on a monitor
    Tile {
        /// Number of windows to pick, or '*' for all
        #[arg(long)]
        pick: String,

        /// Layout: h2, v2, h3, v3, grid
        #[arg(long)]
        layout: String,

        /// Monitor index (0-based)
        #[arg(long)]
        monitor: usize,
    },
    /// List terminal windows and monitors
    List,
    /// Restore original font sizes
    Reset,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tile { pick, layout, monitor } => {
            println!("tile: pick={pick}, layout={layout}, monitor={monitor}");
        }
        Commands::List => {
            println!("list");
        }
        Commands::Reset => {
            println!("reset");
        }
    }

    Ok(())
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build 2>&1`
Expected: Compiles successfully (downloads deps on first run)

- [ ] **Step 6: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: project setup with deps, shared types, and CLI skeleton"
```

---

### Task 2: Layout calculation with tests

**Files:**
- Create: `src/layout.rs`
- Create: `tests/layout_tests.rs`

- [ ] **Step 1: Write failing tests for layout calculations**

Create `tests/layout_tests.rs`:

```rust
use terminalsort::layout::{calculate_layout, LayoutError};
use terminalsort::types::Rect;

#[test]
fn h2_splits_horizontally() {
    let rects = calculate_layout("h2", 0, 0, 1920, 1080, 2).unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 960, height: 1080 });
    assert_eq!(rects[1], Rect { x: 960, y: 0, width: 960, height: 1080 });
}

#[test]
fn v2_splits_vertically() {
    let rects = calculate_layout("v2", 0, 0, 1920, 1080, 2).unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 1920, height: 540 });
    assert_eq!(rects[1], Rect { x: 0, y: 540, width: 1920, height: 540 });
}

#[test]
fn h3_splits_three_horizontal() {
    let rects = calculate_layout("h3", 0, 0, 1920, 1080, 3).unwrap();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 640, height: 1080 });
    assert_eq!(rects[1], Rect { x: 640, y: 0, width: 640, height: 1080 });
    assert_eq!(rects[2], Rect { x: 1280, y: 0, width: 640, height: 1080 });
}

#[test]
fn v3_splits_three_vertical() {
    let rects = calculate_layout("v3", 0, 0, 1920, 1080, 3).unwrap();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 1920, height: 360 });
    assert_eq!(rects[1], Rect { x: 0, y: 360, width: 1920, height: 360 });
    assert_eq!(rects[2], Rect { x: 0, y: 720, width: 1920, height: 360 });
}

#[test]
fn grid_4_makes_2x2() {
    let rects = calculate_layout("grid", 0, 0, 1920, 1080, 4).unwrap();
    assert_eq!(rects.len(), 4);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 960, height: 540 });
    assert_eq!(rects[1], Rect { x: 960, y: 0, width: 960, height: 540 });
    assert_eq!(rects[2], Rect { x: 0, y: 540, width: 960, height: 540 });
    assert_eq!(rects[3], Rect { x: 960, y: 540, width: 960, height: 540 });
}

#[test]
fn grid_6_makes_2x3() {
    let rects = calculate_layout("grid", 0, 0, 1800, 1000, 6).unwrap();
    assert_eq!(rects.len(), 6);
    // 3 cols, 2 rows -> each cell 600x500
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 600, height: 500 });
    assert_eq!(rects[5], Rect { x: 1200, y: 500, width: 600, height: 500 });
}

#[test]
fn grid_with_monitor_offset() {
    let rects = calculate_layout("grid", 1920, 0, 1920, 1080, 4).unwrap();
    assert_eq!(rects[0], Rect { x: 1920, y: 0, width: 960, height: 540 });
    assert_eq!(rects[3], Rect { x: 2880, y: 540, width: 960, height: 540 });
}

#[test]
fn h2_wrong_count_errors() {
    let err = calculate_layout("h2", 0, 0, 1920, 1080, 3).unwrap_err();
    assert!(matches!(err, LayoutError::WindowCountMismatch { .. }));
}

#[test]
fn grid_needs_at_least_2() {
    let err = calculate_layout("grid", 0, 0, 1920, 1080, 1).unwrap_err();
    assert!(matches!(err, LayoutError::WindowCountMismatch { .. }));
}

#[test]
fn unknown_layout_errors() {
    let err = calculate_layout("potato", 0, 0, 1920, 1080, 2).unwrap_err();
    assert!(matches!(err, LayoutError::UnknownLayout(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: Compilation error — `layout` module doesn't exist yet

- [ ] **Step 3: Implement src/layout.rs**

```rust
use crate::types::Rect;

#[derive(Debug)]
pub enum LayoutError {
    UnknownLayout(String),
    WindowCountMismatch {
        layout: String,
        expected: String,
        got: usize,
    },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::UnknownLayout(name) => {
                write!(f, "Unknown layout '{name}'. Available: h2, v2, h3, v3, grid")
            }
            LayoutError::WindowCountMismatch { layout, expected, got } => {
                write!(
                    f,
                    "Layout '{layout}' requires {expected} windows, but {got} were selected. Try 'grid' for {got} windows."
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Calculate window rectangles for the given layout.
///
/// `mon_x`, `mon_y` are the monitor's top-left offset in screen coordinates.
/// `mon_w`, `mon_h` are the monitor's dimensions.
/// `count` is the number of windows to tile.
pub fn calculate_layout(
    layout: &str,
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
) -> Result<Vec<Rect>, LayoutError> {
    match layout {
        "h2" => fixed_horizontal(mon_x, mon_y, mon_w, mon_h, count, 2),
        "v2" => fixed_vertical(mon_x, mon_y, mon_w, mon_h, count, 2),
        "h3" => fixed_horizontal(mon_x, mon_y, mon_w, mon_h, count, 3),
        "v3" => fixed_vertical(mon_x, mon_y, mon_w, mon_h, count, 3),
        "grid" => grid(mon_x, mon_y, mon_w, mon_h, count),
        other => Err(LayoutError::UnknownLayout(other.to_string())),
    }
}

fn fixed_horizontal(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    expected: usize,
) -> Result<Vec<Rect>, LayoutError> {
    if count != expected {
        return Err(LayoutError::WindowCountMismatch {
            layout: format!("h{expected}"),
            expected: format!("exactly {expected}"),
            got: count,
        });
    }
    let cell_w = mon_w / expected as u32;
    let rects = (0..expected)
        .map(|i| Rect {
            x: mon_x + (i as u32 * cell_w) as i32,
            y: mon_y,
            width: cell_w,
            height: mon_h,
        })
        .collect();
    Ok(rects)
}

fn fixed_vertical(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    expected: usize,
) -> Result<Vec<Rect>, LayoutError> {
    if count != expected {
        return Err(LayoutError::WindowCountMismatch {
            layout: format!("v{expected}"),
            expected: format!("exactly {expected}"),
            got: count,
        });
    }
    let cell_h = mon_h / expected as u32;
    let rects = (0..expected)
        .map(|i| Rect {
            x: mon_x,
            y: mon_y + (i as u32 * cell_h) as i32,
            width: mon_w,
            height: cell_h,
        })
        .collect();
    Ok(rects)
}

fn grid(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
) -> Result<Vec<Rect>, LayoutError> {
    if count < 2 {
        return Err(LayoutError::WindowCountMismatch {
            layout: "grid".to_string(),
            expected: "at least 2".to_string(),
            got: count,
        });
    }

    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = (count + cols - 1) / cols; // ceil division

    let cell_w = mon_w / cols as u32;
    let cell_h = mon_h / rows as u32;

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let row = i / cols;
        let col = i % cols;

        // How many windows are in this row
        let windows_in_row = if row == rows - 1 {
            count - row * cols
        } else {
            cols
        };

        let this_cell_w = if windows_in_row < cols && col == windows_in_row - 1 {
            // Last window in a short last row: stretch to fill remaining space
            mon_w - (col as u32 * (mon_w / windows_in_row as u32))
        } else if windows_in_row < cols {
            mon_w / windows_in_row as u32
        } else {
            cell_w
        };

        let this_x = if windows_in_row < cols {
            mon_x + (col as u32 * (mon_w / windows_in_row as u32)) as i32
        } else {
            mon_x + (col as u32 * cell_w) as i32
        };

        rects.push(Rect {
            x: this_x,
            y: mon_y + (row as u32 * cell_h) as i32,
            width: this_cell_w,
            height: cell_h,
        });
    }

    Ok(rects)
}
```

- [ ] **Step 4: Add layout module to lib.rs**

Update `src/lib.rs` — it already has `pub mod layout;` from Step 3 of Task 1.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: All 10 tests pass

- [ ] **Step 6: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: layout calculation with full test coverage"
```

---

### Task 3: State persistence with tests

**Files:**
- Create: `src/state.rs`
- Create: `tests/state_tests.rs`

- [ ] **Step 1: Write failing tests for state serialization**

Create `tests/state_tests.rs`:

```rust
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
```

- [ ] **Step 2: Add tempfile as a dev dependency**

Add to `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: Compilation error — `state` module doesn't have the expected functions yet

- [ ] **Step 4: Implement src/state.rs**

```rust
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: All tests pass (layout + state)

- [ ] **Step 6: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: state persistence with JSON serialization"
```

---

### Task 4: Font scaling via gsettings

**Files:**
- Create: `src/font.rs`

- [ ] **Step 1: Implement src/font.rs**

```rust
use anyhow::{bail, Context, Result};
use std::process::Command;

const PROFILE_LIST_SCHEMA: &str = "org.gnome.Terminal.ProfilesList";
const PROFILE_SCHEMA: &str = "org.gnome.Terminal.Legacy.Profile";

fn profile_path(uuid: &str) -> String {
    format!("/org/gnome/terminal/legacy/profiles:/:{uuid}/")
}

/// Get the default GNOME Terminal profile UUID.
pub fn get_default_profile() -> Result<String> {
    let output = Command::new("gsettings")
        .args(["get", PROFILE_LIST_SCHEMA, "default"])
        .output()
        .context("Failed to run gsettings. Is GNOME Terminal installed?")?;

    if !output.status.success() {
        bail!(
            "gsettings failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let raw = String::from_utf8(output.stdout)?;
    Ok(raw.trim().trim_matches('\'').to_string())
}

/// Read the current font string from a profile (e.g. "Monospace 12").
pub fn get_font(uuid: &str) -> Result<String> {
    let path = profile_path(uuid);
    let schema_path = format!("{PROFILE_SCHEMA}:{path}");
    let output = Command::new("gsettings")
        .args(["get", &schema_path, "font"])
        .output()
        .context("Failed to read font from gsettings")?;

    if !output.status.success() {
        bail!(
            "gsettings get font failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let raw = String::from_utf8(output.stdout)?;
    Ok(raw.trim().trim_matches('\'').to_string())
}

/// Set the font string on a profile (e.g. "Monospace 10").
pub fn set_font(uuid: &str, font: &str) -> Result<()> {
    let path = profile_path(uuid);
    let schema_path = format!("{PROFILE_SCHEMA}:{path}");

    // Ensure use-system-font is false so our custom font takes effect
    let status = Command::new("gsettings")
        .args(["set", &schema_path, "use-system-font", "false"])
        .status()
        .context("Failed to set use-system-font")?;
    if !status.success() {
        bail!("gsettings set use-system-font failed");
    }

    let status = Command::new("gsettings")
        .args(["set", &schema_path, "font", font])
        .status()
        .context("Failed to set font")?;
    if !status.success() {
        bail!("gsettings set font failed");
    }

    Ok(())
}

/// Parse the font size from a font description string like "Monospace 12".
pub fn parse_font_size(font_desc: &str) -> Option<f64> {
    font_desc
        .rsplit_once(' ')
        .and_then(|(_, size_str)| size_str.parse::<f64>().ok())
}

/// Build a font description with a new size: "Monospace 12" + 8.0 → "Monospace 8"
pub fn with_font_size(font_desc: &str, new_size: f64) -> String {
    let size_rounded = new_size.round() as u32;
    match font_desc.rsplit_once(' ') {
        Some((family, _)) => format!("{family} {size_rounded}"),
        None => format!("{font_desc} {size_rounded}"),
    }
}

/// Calculate the scaled font size for a given window count.
/// Formula: base_size * (2.0 / sqrt(count)), clamped to minimum 6pt.
pub fn scaled_font_size(base_size: f64, window_count: usize) -> f64 {
    let scaled = base_size * (2.0 / (window_count as f64).sqrt());
    scaled.round().max(6.0)
}
```

- [ ] **Step 2: Add font module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod types;
pub mod layout;
pub mod font;
pub mod state;
```

- [ ] **Step 3: Write tests for the pure functions**

Add to `tests/font_tests.rs`:

```rust
use terminalsort::font::{parse_font_size, with_font_size, scaled_font_size};

#[test]
fn parse_font_size_standard() {
    assert_eq!(parse_font_size("Monospace 12"), Some(12.0));
}

#[test]
fn parse_font_size_with_style() {
    assert_eq!(parse_font_size("DejaVu Sans Mono 14"), Some(14.0));
}

#[test]
fn parse_font_size_none_for_garbage() {
    assert_eq!(parse_font_size("nosize"), None);
}

#[test]
fn with_font_size_replaces() {
    assert_eq!(with_font_size("Monospace 12", 8.0), "Monospace 8");
}

#[test]
fn with_font_size_multi_word_family() {
    assert_eq!(
        with_font_size("DejaVu Sans Mono 14", 10.0),
        "DejaVu Sans Mono 10"
    );
}

#[test]
fn scaled_size_4_windows() {
    // 12 * 2/sqrt(4) = 12 * 1.0 = 12
    assert_eq!(scaled_font_size(12.0, 4), 12.0);
}

#[test]
fn scaled_size_8_windows() {
    // 12 * 2/sqrt(8) = 12 * 0.707 = 8.485 → rounds to 8
    assert_eq!(scaled_font_size(12.0, 8), 8.0);
}

#[test]
fn scaled_size_floors_at_6() {
    // 12 * 2/sqrt(100) = 12 * 0.2 = 2.4 → clamped to 6
    assert_eq!(scaled_font_size(12.0, 100), 6.0);
}

#[test]
fn scaled_size_2_windows() {
    // 12 * 2/sqrt(2) = 12 * 1.414 = 16.97 → rounds to 17
    assert_eq!(scaled_font_size(12.0, 2), 17.0);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: font scaling via gsettings with size calculation"
```

---

### Task 5: X11 window operations

**Files:**
- Create: `src/x11.rs`

- [ ] **Step 1: Implement src/x11.rs**

```rust
use crate::types::TerminalWindow;
use anyhow::{bail, Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux,
    ConnectionExt as _, EventMask,
};
use x11rb::rust_connection::RustConnection;

/// Wrapper around the X11 connection and root window.
pub struct X11 {
    pub conn: RustConnection,
    pub root: xproto::Window,
    pub screen_num: usize,
}

impl X11 {
    /// Connect to the X11 display.
    pub fn connect() -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).context("Cannot connect to X11 display. Is DISPLAY set?")?;
        let root = conn.setup().roots[screen_num].root;
        Ok(Self { conn, root, screen_num })
    }

    /// List all GNOME Terminal windows.
    pub fn list_terminal_windows(&self) -> Result<Vec<TerminalWindow>> {
        let net_client_list = self
            .conn
            .intern_atom(false, b"_NET_CLIENT_LIST")?
            .reply()?
            .atom;

        let reply = self
            .conn
            .get_property(false, self.root, net_client_list, AtomEnum::WINDOW, 0, u32::MAX)?
            .reply()?;

        let all_windows: Vec<u32> = reply.value32().map(|iter| iter.collect()).unwrap_or_default();

        let mut terminals = Vec::new();
        for wid in all_windows {
            if let Ok((_, class)) = self.get_wm_class(wid) {
                if class.to_lowercase().contains("gnome-terminal") {
                    let title = self.get_window_title(wid).unwrap_or_default();
                    terminals.push(TerminalWindow { id: wid, title });
                }
            }
        }

        Ok(terminals)
    }

    /// Read WM_CLASS (instance, class) from a window.
    fn get_wm_class(&self, window: u32) -> Result<(String, String)> {
        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, u32::MAX)?
            .reply()?;

        let mut parts = reply.value.split(|&b| b == 0).filter(|s| !s.is_empty());

        let instance = parts
            .next()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let class = parts
            .next()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();

        Ok((instance, class))
    }

    /// Read window title (_NET_WM_NAME falling back to WM_NAME).
    fn get_window_title(&self, window: u32) -> Result<String> {
        let net_wm_name = self.conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
        let utf8_string = self.conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom;

        let reply = self
            .conn
            .get_property(false, window, net_wm_name, utf8_string, 0, u32::MAX)?
            .reply()?;

        if !reply.value.is_empty() {
            return Ok(String::from_utf8_lossy(&reply.value).into_owned());
        }

        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)?
            .reply()?;

        Ok(String::from_utf8_lossy(&reply.value).into_owned())
    }

    /// Move and resize a window to the given position and size.
    pub fn move_resize(&self, window: u32, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
        // Remove maximized state first so the WM allows repositioning
        self.remove_maximized(window)?;

        let aux = ConfigureWindowAux::new()
            .x(x)
            .y(y)
            .width(width)
            .height(height);

        self.conn.configure_window(window, &aux)?;
        self.conn.flush()?;

        Ok(())
    }

    /// Remove _NET_WM_STATE_MAXIMIZED_HORZ and _VERT from a window.
    fn remove_maximized(&self, window: u32) -> Result<()> {
        let net_wm_state = self.conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
        let max_h = self
            .conn
            .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ")?
            .reply()?
            .atom;
        let max_v = self
            .conn
            .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT")?
            .reply()?
            .atom;

        // Action 0 = _NET_WM_STATE_REMOVE
        let data = ClientMessageData::from([0u32, max_h, max_v, 1, 0]);
        let event = ClientMessageEvent::new(32, window, net_wm_state, data);

        self.conn.send_event(
            false,
            self.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;
        self.conn.flush()?;

        Ok(())
    }
}
```

- [ ] **Step 2: Add x11 module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod types;
pub mod layout;
pub mod font;
pub mod state;
pub mod x11;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: X11 window operations via x11rb"
```

---

### Task 6: Monitor detection via RANDR

**Files:**
- Create: `src/monitor.rs`

- [ ] **Step 1: Implement src/monitor.rs**

```rust
use crate::types::Monitor;
use crate::x11::X11;
use anyhow::{bail, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;

/// List all active monitors via RANDR.
pub fn list_monitors(x11: &X11) -> Result<Vec<Monitor>> {
    let reply = x11.conn.randr_get_monitors(x11.root, true)?.reply()?;

    let mut monitors = Vec::new();
    for (i, mon) in reply.monitors.iter().enumerate() {
        let name = x11
            .conn
            .get_atom_name(mon.name)?
            .reply()
            .map(|r| String::from_utf8_lossy(&r.name).into_owned())
            .unwrap_or_else(|_| format!("Monitor {i}"));

        monitors.push(Monitor {
            index: i,
            name,
            x: mon.x,
            y: mon.y,
            width: mon.width,
            height: mon.height,
        });
    }

    Ok(monitors)
}

/// Get a specific monitor by index, or error with available monitors listed.
pub fn get_monitor(x11: &X11, index: usize) -> Result<Monitor> {
    let monitors = list_monitors(x11)?;

    if let Some(mon) = monitors.into_iter().find(|m| m.index == index) {
        return Ok(mon);
    }

    let available = list_monitors(x11)?;
    let listing: Vec<String> = available
        .iter()
        .map(|m| format!("  {}: {} ({}x{})", m.index, m.name, m.width, m.height))
        .collect();

    bail!(
        "Monitor {} not found. Available monitors:\n{}",
        index,
        listing.join("\n")
    );
}
```

- [ ] **Step 2: Add monitor module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod types;
pub mod layout;
pub mod font;
pub mod state;
pub mod x11;
pub mod monitor;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: monitor detection via RANDR"
```

---

### Task 7: Window picker (click-to-select)

**Files:**
- Create: `src/picker.rs`

- [ ] **Step 1: Implement src/picker.rs**

```rust
use crate::types::TerminalWindow;
use crate::x11::X11;
use anyhow::{bail, Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, EventMask, GrabMode, GrabStatus,
};
use x11rb::protocol::Event;

/// Pick N windows by clicking on them. Returns their window IDs and titles.
pub fn pick_windows(x11: &X11, count: usize) -> Result<Vec<TerminalWindow>> {
    eprintln!(
        "Click on {} terminal window{} to select {} (Escape to cancel)",
        count,
        if count == 1 { "" } else { "s" },
        if count == 1 { "it" } else { "them" },
    );

    // Grab pointer
    let grab = x11
        .conn
        .grab_pointer(
            false,
            x11.root,
            EventMask::BUTTON_PRESS,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            x11rb::NONE,
            x11rb::NONE,
            x11rb::CURRENT_TIME,
        )?
        .reply()?;

    if grab.status != GrabStatus::SUCCESS {
        bail!("Failed to grab pointer: {:?}", grab.status);
    }

    // Grab keyboard for Escape detection
    x11.conn
        .grab_keyboard(
            false,
            x11.root,
            x11rb::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?
        .reply()?;

    x11.conn.flush()?;

    let mut selected = Vec::new();

    let result = pick_loop(x11, count, &mut selected);

    // Always release grabs
    let _ = x11.conn.ungrab_pointer(x11rb::CURRENT_TIME);
    let _ = x11.conn.ungrab_keyboard(x11rb::CURRENT_TIME);
    let _ = x11.conn.flush();

    result?;
    Ok(selected)
}

fn pick_loop(x11: &X11, count: usize, selected: &mut Vec<TerminalWindow>) -> Result<()> {
    while selected.len() < count {
        let event = x11.conn.wait_for_event()?;

        match event {
            Event::ButtonPress(bp) => {
                let target = if bp.child != x11rb::NONE {
                    bp.child
                } else {
                    continue; // Clicked on root desktop, ignore
                };

                // Find the client window (WM may reparent into frames)
                let client = find_client_window(&x11.conn, target)?;

                // Check if it's a GNOME Terminal
                let reply = x11.conn.get_property(
                    false, client, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, u32::MAX,
                )?.reply()?;

                let class_str = String::from_utf8_lossy(&reply.value).to_lowercase();
                if !class_str.contains("gnome-terminal") {
                    eprintln!("  Not a GNOME Terminal window, try again");
                    continue;
                }

                // Check for duplicates
                if selected.iter().any(|w| w.id == client) {
                    eprintln!("  Window already selected, try again");
                    continue;
                }

                // Get title
                let title = get_title(&x11.conn, client).unwrap_or_default();

                eprintln!(
                    "  Selected window {}/{}: {}",
                    selected.len() + 1,
                    count,
                    if title.is_empty() { "(untitled)" } else { &title }
                );

                selected.push(TerminalWindow { id: client, title });
            }
            Event::KeyPress(kp) => {
                // Escape keycode is typically 9
                if kp.detail == 9 {
                    bail!("Selection cancelled");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Walk down from a frame window to find the actual client window (has WM_STATE).
fn find_client_window(
    conn: &x11rb::rust_connection::RustConnection,
    window: u32,
) -> Result<u32> {
    let wm_state = conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;

    let reply = conn
        .get_property(false, window, wm_state, AtomEnum::ANY, 0, 0)?
        .reply()?;

    if reply.type_ != x11rb::NONE {
        return Ok(window);
    }

    let tree = conn.query_tree(window)?.reply()?;
    for &child in &tree.children {
        if let Ok(w) = find_client_window(conn, child) {
            return Ok(w);
        }
    }

    // If no WM_STATE found, return the original window
    Ok(window)
}

fn get_title(conn: &x11rb::rust_connection::RustConnection, window: u32) -> Result<String> {
    let net_wm_name = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
    let utf8_string = conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom;

    let reply = conn
        .get_property(false, window, net_wm_name, utf8_string, 0, u32::MAX)?
        .reply()?;

    if !reply.value.is_empty() {
        return Ok(String::from_utf8_lossy(&reply.value).into_owned());
    }

    let reply = conn
        .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)?
        .reply()?;

    Ok(String::from_utf8_lossy(&reply.value).into_owned())
}
```

- [ ] **Step 2: Add picker module to lib.rs**

Update `src/lib.rs`:

```rust
pub mod types;
pub mod layout;
pub mod font;
pub mod state;
pub mod x11;
pub mod monitor;
pub mod picker;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: click-to-select window picker with Escape cancel"
```

---

### Task 8: Wire up main.rs with all commands

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement the full CLI dispatch**

Replace `src/main.rs` entirely:

```rust
mod x11;
mod monitor;
mod picker;
mod layout;
mod font;
mod state;
mod types;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "terminalsort", about = "Tile terminal windows with font scaling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tile selected terminal windows on a monitor
    Tile {
        /// Number of windows to pick, or '*' for all
        #[arg(long)]
        pick: String,

        /// Layout: h2, v2, h3, v3, grid
        #[arg(long)]
        layout: String,

        /// Monitor index (0-based)
        #[arg(long)]
        monitor: usize,
    },
    /// List terminal windows and monitors
    List,
    /// Restore original font sizes
    Reset,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tile { pick, layout, monitor: mon_idx } => {
            cmd_tile(&pick, &layout, mon_idx)?;
        }
        Commands::List => {
            cmd_list()?;
        }
        Commands::Reset => {
            cmd_reset()?;
        }
    }

    Ok(())
}

fn cmd_tile(pick: &str, layout_name: &str, mon_idx: usize) -> Result<()> {
    let x11 = x11::X11::connect()?;
    let mon = monitor::get_monitor(&x11, mon_idx)?;

    // Pick windows
    let windows = if pick == "*" {
        let terminals = x11.list_terminal_windows()?;
        if terminals.is_empty() {
            bail!("No GNOME Terminal windows found. Are any open?");
        }
        eprintln!("Found {} terminal window(s)", terminals.len());
        terminals
    } else {
        let count: usize = pick
            .parse()
            .map_err(|_| anyhow::anyhow!("--pick must be a number or '*', got '{pick}'"))?;
        picker::pick_windows(&x11, count)?
    };

    // Calculate layout
    let rects = layout::calculate_layout(
        layout_name,
        mon.x as i32,
        mon.y as i32,
        mon.width as u32,
        mon.height as u32,
        windows.len(),
    )?;

    // Scale fonts
    let font_result = scale_fonts(windows.len());
    if let Err(e) = &font_result {
        eprintln!("Warning: Could not adjust font size: {e}. Tiling without font adjustment.");
    }

    // Move/resize windows
    for (win, rect) in windows.iter().zip(rects.iter()) {
        x11.move_resize(win.id, rect.x, rect.y, rect.width, rect.height)?;
        eprintln!(
            "  Tiled: {} → {}x{} at ({},{})",
            if win.title.is_empty() { "(untitled)" } else { &win.title },
            rect.width,
            rect.height,
            rect.x,
            rect.y,
        );
    }

    eprintln!("Done! {} windows tiled with layout '{layout_name}'", windows.len());
    Ok(())
}

fn scale_fonts(window_count: usize) -> Result<()> {
    let state_path = state::default_state_path()?;

    let uuid = font::get_default_profile()?;
    let current_font = font::get_font(&uuid)?;
    let base_size = font::parse_font_size(&current_font)
        .ok_or_else(|| anyhow::anyhow!("Cannot parse font size from '{current_font}'"))?;

    // Save original state before modifying
    let mut saved = state::load_state(&state_path)?;
    // Only save if we haven't already (don't overwrite the original with an already-scaled value)
    if !saved.profiles.contains_key(&uuid) {
        saved.profiles.insert(uuid.clone(), current_font.clone());
        state::save_state(&state_path, &saved)?;
    }

    let new_size = font::scaled_font_size(base_size, window_count);
    let new_font = font::with_font_size(&current_font, new_size);

    if new_font != current_font {
        font::set_font(&uuid, &new_font)?;
        eprintln!("Font scaled: {current_font} → {new_font}");
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    let x11 = x11::X11::connect()?;

    // List monitors
    let monitors = monitor::list_monitors(&x11)?;
    println!("Monitors:");
    for mon in &monitors {
        println!(
            "  {}: {} ({}x{} at +{}+{})",
            mon.index, mon.name, mon.width, mon.height, mon.x, mon.y
        );
    }

    // List terminal windows
    let terminals = x11.list_terminal_windows()?;
    println!("\nGNOME Terminal windows:");
    if terminals.is_empty() {
        println!("  (none found)");
    } else {
        for win in &terminals {
            println!(
                "  {:#010x}: {}",
                win.id,
                if win.title.is_empty() { "(untitled)" } else { &win.title }
            );
        }
    }

    Ok(())
}

fn cmd_reset() -> Result<()> {
    let state_path = state::default_state_path()?;
    let saved = state::load_state(&state_path)?;

    if saved.profiles.is_empty() {
        eprintln!("No saved state found. Nothing to reset.");
        return Ok(());
    }

    for (uuid, original_font) in &saved.profiles {
        font::set_font(uuid, original_font)?;
        eprintln!("Restored font for profile {uuid}: {original_font}");
    }

    // Clear the state file
    let empty = state::SavedState::new();
    state::save_state(&state_path, &empty)?;
    eprintln!("State cleared.");

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 3: Run all tests**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "feat: wire up CLI commands — tile, list, reset"
```

---

### Task 9: Manual smoke testing

**Files:** None — this is a testing task

- [ ] **Step 1: Build release binary**

Run: `cd /media/blakethefn/500SSD1/Projects/terminalsort && cargo build --release 2>&1`
Expected: Compiles successfully

- [ ] **Step 2: Test the list command**

Run: `./target/release/terminalsort list`
Expected: Shows monitors (DP-0 2560x1440, HDMI-0 1920x1080) and any open GNOME Terminal windows

- [ ] **Step 3: Test tile with 2 windows**

Open 2 GNOME Terminal windows, then run:
`./target/release/terminalsort tile --pick 2 --layout h2 --monitor 1`
Expected: Click 2 windows, they tile side-by-side on the HDMI monitor. Font may scale.

- [ ] **Step 4: Test reset**

Run: `./target/release/terminalsort reset`
Expected: Font restored to original "Monospace 12"

- [ ] **Step 5: Test tile --pick * with grid**

Open 4 GNOME Terminal windows, then run:
`./target/release/terminalsort tile --pick '*' --layout grid --monitor 1`
Expected: All 4 terminals tile as 2x2 grid on the HDMI monitor

- [ ] **Step 6: Test error cases**

Run: `./target/release/terminalsort tile --pick 3 --layout h2 --monitor 1`
Expected: Error about layout 'h2' requiring exactly 2 windows

Run: `./target/release/terminalsort tile --pick 2 --layout h2 --monitor 5`
Expected: Error about monitor 5 not found, lists available monitors

- [ ] **Step 7: Commit any fixes from smoke testing**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "fix: address issues found during smoke testing"
```

---

### Task 10: Update spec and project hub

**Files:**
- Modify: `docs/superpowers/specs/2026-04-03-terminalsort-design.md`

- [ ] **Step 1: Update the spec to reflect gsettings approach**

In the spec, update the Dependencies section to remove `zbus` and update the font.rs description to say "gsettings CLI" instead of "dbus communication with GNOME Terminal via zbus".

Update the dependency block:

```toml
[dependencies]
x11rb = { version = "0.13", features = ["randr"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
anyhow = "1"
```

Update the font.rs row in the Modules table:
`font.rs` — Font scaling via gsettings CLI. Read/write GNOME Terminal profile font settings.

Update the "dbus connection fails" error row to:
`gsettings command fails` — Warning: "Could not adjust font size. Tiling without font adjustment." Proceed with tile only.

- [ ] **Step 2: Commit**

```bash
cd /media/blakethefn/500SSD1/Projects/terminalsort
git add -A
git commit -m "docs: update spec to reflect gsettings approach"
```
