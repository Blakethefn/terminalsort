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
