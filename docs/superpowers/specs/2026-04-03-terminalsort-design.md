# terminalsort Design Spec

## Overview

Pure Rust CLI tool for tiling GNOME Terminal windows into even grid layouts on X11 with automatic font scaling. Targets a multi-monitor setup where the user manually selects which windows to tile and which monitor to tile them on.

## CLI Interface

```
terminalsort tile --pick <N|*> --layout <LAYOUT> --monitor <N>
terminalsort list
terminalsort reset
```

### Commands

- **`tile`** — Pick windows, calculate layout, move/resize them, adjust font sizes.
- **`list`** — Show open GNOME Terminal windows (with IDs and titles) and available monitors (with indices, resolutions, positions).
- **`reset`** — Restore original font sizes from saved state.

### Tile Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `--pick <N\|*>` | Yes | Number of windows to click-select, or `*` for all terminals |
| `--layout <LAYOUT>` | Yes | Layout name (see below) |
| `--monitor <N>` | Yes | Monitor index (0-based, matches xrandr order) |

## Layouts

| Name | Windows | Description |
|------|---------|-------------|
| `h2` | 2 | Two windows side-by-side horizontally (left/right) |
| `v2` | 2 | Two windows stacked vertically (top/bottom) |
| `h3` | 3 | Three windows side-by-side horizontally |
| `v3` | 3 | Three windows stacked vertically |
| `grid` | N | Auto grid — rows/cols calculated to best fill the monitor. 4→2x2, 6→2x3, 8→2x4, 9→3x3. |

### Grid Calculation

For `grid` layout with N windows:
1. `cols = ceil(sqrt(N))`
2. `rows = ceil(N / cols)`
3. Windows fill left-to-right, top-to-bottom. Last row may have fewer windows — those windows stretch to fill remaining horizontal space.

### Layout Validation

- `h2`/`v2` require exactly 2 windows.
- `h3`/`v3` require exactly 3 windows.
- `grid` requires 2+ windows.
- Mismatch produces a clear error with valid alternatives.

## Architecture

### Modules

| File | Responsibility |
|------|---------------|
| `main.rs` | Entry point, clap CLI definition, command dispatch |
| `x11.rs` | X11 connection via x11rb. List windows by WM_CLASS, move/resize windows, read window properties, remove/re-add window manager decorations if needed |
| `monitor.rs` | Query monitors via X11 RANDR extension. Return list of Monitor structs (index, name, x, y, width, height) |
| `picker.rs` | Click-to-select flow. Grab X pointer, wait for button press events, resolve clicked window IDs. Supports both N-pick (fixed count) and cancel via Escape |
| `layout.rs` | Pure geometry. Takes layout name + monitor rect + window count → Vec of (x, y, w, h) rects |
| `font.rs` | Font scaling via gsettings CLI. Read/write GNOME Terminal profile font settings, calculate scaled sizes |
| `state.rs` | Persist and restore original font sizes. Read/write `~/.local/state/terminalsort/state.json` |

### Data Flow

```
CLI parse (clap)
  → monitor::list_monitors() to validate --monitor
  → picker::pick_windows(n) or x11::list_terminal_windows() for --pick *
  → layout::calculate(layout_name, monitor_rect, window_count) → Vec<Rect>
  → font::read_current_size() → save to state → font::set_size(scaled)
  → x11::move_resize(window_id, rect) for each window
```

### Key Types

```rust
struct Monitor {
    index: usize,
    name: String,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

struct SavedState {
    // profile_name → original_font_size_string (e.g. "Monospace 12")
    profiles: HashMap<String, String>,
}
```

## Font Scaling

### Mechanism

GNOME Terminal stores font settings in dconf/gsettings under `org.gnome.Terminal.ProfilesList` and per-profile schemas. We shell out to the `gsettings` CLI to read and write the font setting (pure D-Bus access to dconf requires GVariant binary serialization, which is impractical).

Specifically:
- Read the default profile UUID from `org.gnome.Terminal.ProfilesList` → `default` key
- Read/write font from `org.gnome.Terminal.Legacy.Profile:/org/gnome/terminal/legacy/profiles:/:UUID/` → `font` key (string like `"Monospace 12"`)
- The `use-system-font` key must be set to `false` for the custom font to take effect.

### Scaling Formula

```
scaled_size = base_size * (2.0 / sqrt(window_count))
scaled_size = max(scaled_size, 6.0)  // floor at 6pt
scaled_size = round(scaled_size)
```

Examples:
- 2 windows: 12 * 2/√2 = ~17pt (slight increase, keeps readable)
- 4 windows: 12 * 2/√4 = 12pt (unchanged)
- 8 windows: 12 * 2/√8 = ~8pt
- 16 windows: 12 * 2/√16 = 6pt (hits floor)

### State Persistence

Before changing fonts, save the original font string to `~/.local/state/terminalsort/state.json`. The `reset` command reads this file and restores the original value.

Format:
```json
{
  "profiles": {
    "b1dcc9dd-5262-4d8d-a863-c897e6d979b9": "Monospace 12"
  }
}
```

## Window Picking

### Auto-pick (`--pick *`)

Query all X11 windows, filter by `WM_CLASS` containing `gnome-terminal-server`. Return all matching window IDs.

### Manual pick (`--pick N`)

1. Print instruction: "Click on N terminal windows to select them (Escape to cancel)"
2. Grab the X pointer (active grab)
3. For each click:
   - Read the ButtonPress event
   - Resolve the window ID under the cursor
   - Walk up the window tree to find the top-level frame window
   - Print confirmation: "Selected window N: <title>"
4. After N clicks, ungrab pointer and return window IDs
5. Escape key press cancels and exits cleanly

## Monitor Selection

Monitors are indexed 0-based in the order RANDR reports them (same as xrandr output order). The `list` command shows available monitors so the user can identify indices.

When tiling, windows are moved to absolute coordinates within the selected monitor's geometry. The monitor's (x, y) offset is added to each window rect.

## Error Handling

| Condition | Behavior |
|-----------|----------|
| No X11 DISPLAY set | Error: "Cannot connect to X11 display. Is DISPLAY set?" |
| No terminal windows found | Error: "No GNOME Terminal windows found. Are any open?" |
| Window count doesn't match layout | Error: "Layout 'h2' requires exactly 2 windows, but N were selected. Try 'grid' for N windows." |
| gsettings command fails | Warning: "Could not adjust font size. Tiling without font adjustment." Proceed with tile only. |
| Invalid monitor index | Error: "Monitor N not found. Available monitors: ..." |
| State file missing on reset | Warning: "No saved state found. Nothing to reset." |

## Dependencies

```toml
[dependencies]
x11rb = { version = "0.13", features = ["randr"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
anyhow = "1"
```

Pure Rust for X11 operations. Font scaling shells out to `gsettings` CLI (always present on GNOME desktops).

## Testing Strategy

- `layout.rs`: Unit tests for all grid calculations (known inputs → expected rects)
- `state.rs`: Unit tests for serialization/deserialization roundtrips
- `font.rs`: Integration test (requires running dbus session)
- `x11.rs` / `picker.rs`: Manual testing (requires running X server)
- Overall CLI: Manual smoke tests on the target machine

## Out of Scope

- Wayland support
- Non-GNOME-Terminal support (kitty, alacritty, etc.)
- Persistent layout profiles / config file
- Auto-tiling on window open
- Window decoration removal/restoration
