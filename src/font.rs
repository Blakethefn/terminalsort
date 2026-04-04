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
