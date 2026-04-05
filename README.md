# terminalsort

Tile and rename terminal windows on X11 with automatic font scaling.

Built for multi-monitor setups where you need to quickly arrange terminal windows — split 2 side-by-side, stack 3 vertically, or fill a monitor with an 8-window grid. Also supports renaming window titles, with pinning to override programs that continuously set their own titles (like Claude Code).

## Features

- **Layouts:** horizontal splits (`h2`, `h3`), vertical splits (`v2`, `v3`), auto grid (`grid`)
- **Click-to-pick:** select specific windows by clicking on them, or use `*` for all terminals
- **Font scaling:** automatically adjusts GNOME Terminal font size so text stays readable at higher split counts
- **Window renaming:** set terminal window titles via OSC escape sequences, with optional pinning to keep them set
- **Monitor targeting:** tile onto any connected monitor by index
- **Multi-terminal:** supports GNOME Terminal, Kitty, and any terminal added to the config
- **Pure Rust** X11 via [x11rb](https://github.com/nickel-org/x11rb) — no xdotool/wmctrl dependencies

## Install

Requires Rust toolchain. Clone and build:

```bash
git clone https://github.com/Blakethefn/terminalsort.git
cd terminalsort
cargo install --path .
```

Or build manually:

```bash
cargo build --release
# Binary at ./target/release/terminalsort
```

## Usage

### List

```bash
# List monitors and open terminal windows
terminalsort list
```

### Tile

```bash
# Tile all terminals as a grid on monitor 1
terminalsort tile --pick '*' --layout grid --monitor 1

# Click to pick 2 windows, split side-by-side
terminalsort tile --pick 2 --layout h2 --monitor 1

# Pick 8 windows, 2x4 grid
terminalsort tile --pick 8 --layout grid --monitor 0

# Stack 3 windows vertically
terminalsort tile --pick 3 --layout v3 --monitor 0

# Restore original font size
terminalsort reset
```

### Rename

```bash
# Rename a window by ID (from 'list' output) — pins by default
terminalsort rename --id 0x04ef2087 --title "Opus 4.6"

# Rename multiple windows at once
terminalsort rename --id 0x04ef2087 --id 0x04e8a215 --title "Opus 4.6" --title "Sonnet 4.6"

# Click to pick windows and rename them
terminalsort rename --pick 2 --title "Opus 4.6" --title "GPT-5.4"

# One-shot rename without pinning (title may be overwritten by other programs)
terminalsort rename --id 0x04ef2087 --title "My Terminal" --no-pin

# Custom pin interval (default 500ms)
terminalsort rename --id 0x04ef2087 --title "Opus 4.6" --interval 250
```

Pinning re-applies the title every 500ms via OSC escape sequences, which keeps it visible even when programs like Claude Code continuously overwrite the window title. For terminals that don't fight the title (like Kitty), `--no-pin` works as a permanent one-shot rename.

## Layouts

| Layout | Description |
|--------|-------------|
| `h2`   | 2 windows side-by-side |
| `v2`   | 2 windows stacked |
| `h3`   | 3 windows side-by-side |
| `v3`   | 3 windows stacked |
| `grid` | Auto grid — 2x2 for 4, 2x3 for 6, 2x4 for 8, 3x3 for 9 |

## Supported Terminals

- GNOME Terminal
- Kitty

Adding a new terminal is a one-line change in `src/x11.rs` (`TERMINAL_CLASSES`).

Font scaling currently uses `gsettings` and only works with GNOME Terminal. Tiling and renaming work with all supported terminals.

## Requirements

- Linux with X11 (not Wayland)
- `gsettings` CLI for font scaling (included with GNOME, optional)

## License

MIT
