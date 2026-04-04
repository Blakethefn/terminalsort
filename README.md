# terminalsort

Tile GNOME Terminal windows into even grid layouts on X11 with automatic font scaling.

Built for multi-monitor setups where you need to quickly arrange terminal windows — split 2 side-by-side, stack 3 vertically, or fill a monitor with an 8-window grid.

## Features

- **Layouts:** horizontal splits (`h2`, `h3`), vertical splits (`v2`, `v3`), auto grid (`grid`)
- **Click-to-pick:** select specific windows by clicking on them, or use `*` for all terminals
- **Font scaling:** automatically adjusts GNOME Terminal font size so text stays readable at higher split counts
- **Monitor targeting:** tile onto any connected monitor by index
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

```bash
# List monitors and open terminal windows
terminalsort list

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

## Layouts

| Layout | Description |
|--------|-------------|
| `h2`   | 2 windows side-by-side |
| `v2`   | 2 windows stacked |
| `h3`   | 3 windows side-by-side |
| `v3`   | 3 windows stacked |
| `grid` | Auto grid — 2x2 for 4, 2x3 for 6, 2x4 for 8, 3x3 for 9 |

## Requirements

- Linux with X11 (not Wayland)
- GNOME Terminal (for font scaling — tiling works with any terminal, font scaling is GNOME-specific)
- `gsettings` CLI (included with GNOME)

## License

MIT
