mod x11;
mod monitor;
mod picker;
mod layout;
mod font;
mod pts;
mod state;
mod types;

use anyhow::{bail, Context, Result};
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
    /// Set window titles (pins by default so they stay set)
    Rename {
        /// Number of windows to click-select (use --id instead to skip clicking)
        #[arg(long, conflicts_with = "id", required_unless_present = "id")]
        pick: Option<usize>,

        /// Window ID(s) from 'list' output (e.g. 0x04ef2087)
        #[arg(long, conflicts_with = "pick", required_unless_present = "pick")]
        id: Vec<String>,

        /// Title(s) to assign, one per window (in order)
        #[arg(long, required = true)]
        title: Vec<String>,

        /// Set title once without pinning (title may be overwritten by other programs)
        #[arg(long)]
        no_pin: bool,

        /// Pin re-apply interval in milliseconds (default: 500)
        #[arg(long, default_value = "500")]
        interval: u64,
    },
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
        Commands::Rename { pick, id, title, no_pin, interval } => {
            cmd_rename(pick, &id, &title, !no_pin, interval)?;
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

    // Clip monitor geometry to workarea (accounts for panels/taskbars)
    let workarea = x11.get_workarea()?;
    let mon_x = mon.x as i32;
    let mon_y = mon.y as i32;
    let mon_w = mon.width as u32;
    let mon_h = mon.height as u32;

    // Intersect monitor rect with workarea
    let eff_x = mon_x.max(workarea.x);
    let eff_y = mon_y.max(workarea.y);
    let eff_right = (mon_x + mon_w as i32).min(workarea.x + workarea.width as i32);
    let eff_bottom = (mon_y + mon_h as i32).min(workarea.y + workarea.height as i32);
    let eff_w = (eff_right - eff_x).max(0) as u32;
    let eff_h = (eff_bottom - eff_y).max(0) as u32;

    // Calculate layout using effective (panel-adjusted) geometry
    let rects = layout::calculate_layout(
        layout_name,
        eff_x,
        eff_y,
        eff_w,
        eff_h,
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

fn cmd_rename(
    pick: Option<usize>,
    ids: &[String],
    titles: &[String],
    pin: bool,
    interval_ms: u64,
) -> Result<()> {
    let x11 = x11::X11::connect()?;

    let windows = if let Some(count) = pick {
        if titles.len() != count {
            bail!(
                "Expected {} title(s) for {} window(s), got {}",
                count,
                count,
                titles.len()
            );
        }
        picker::pick_windows(&x11, count)?
    } else {
        if titles.len() != ids.len() {
            bail!(
                "Expected {} title(s) for {} window ID(s), got {}",
                ids.len(),
                ids.len(),
                titles.len()
            );
        }
        let mut wins = Vec::new();
        for id_str in ids {
            let id_str = id_str.trim_start_matches("0x").trim_start_matches("0X");
            let wid = u32::from_str_radix(id_str, 16)
                .map_err(|_| anyhow::anyhow!("Invalid window ID: {}", ids[wins.len()]))?;
            let title = x11.get_window_title(wid).unwrap_or_default();
            wins.push(types::TerminalWindow { id: wid, title });
        }
        wins
    };

    // Resolve PTS devices via probe (single batch for all windows)
    let parent_pid = x11.get_window_pid(windows[0].id)?;
    let window_ids: Vec<u32> = windows.iter().map(|w| w.id).collect();
    let pts_map = pts::find_pts_batch(&x11, parent_pid, &window_ids)
        .context("Failed to probe PTS devices")?;

    let mut targets: Vec<(types::TerminalWindow, std::path::PathBuf, String)> = Vec::new();
    for (win, title) in windows.into_iter().zip(titles.iter()) {
        let pts_path = pts_map
            .get(&win.id)
            .ok_or_else(|| anyhow::anyhow!("No PTS found for window {:#010x}", win.id))?
            .clone();
        eprintln!(
            "  {} → {} ({})",
            if win.title.is_empty() { "(untitled)" } else { &win.title },
            title,
            pts_path.display()
        );
        targets.push((win, pts_path, title.clone()));
    }

    // Set titles via OSC escape sequence
    for (_, pts_path, title) in &targets {
        pts::set_title(pts_path, title)?;
    }

    if pin {
        eprintln!("Pinning {} title(s) (Ctrl+C to stop)...", targets.len());
        let interval = std::time::Duration::from_millis(interval_ms);
        loop {
            std::thread::sleep(interval);
            let mut any_alive = false;
            for (win, pts_path, title) in &targets {
                if x11.window_exists(win.id) {
                    let _ = pts::set_title(pts_path, title);
                    any_alive = true;
                }
            }
            if !any_alive {
                eprintln!("All pinned windows closed. Exiting.");
                break;
            }
        }
    } else {
        eprintln!("Done! {} window(s) renamed.", targets.len());
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
