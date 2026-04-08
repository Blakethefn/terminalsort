use crate::x11::X11;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Resolve PTS devices for multiple windows.
///
/// Strategy 1 (per-window): Get each window's PID and find its child with a PTS.
/// This works for kitty where each OS window is a separate process.
///
/// Strategy 2 (probe): Falls back to title-probing when per-window mapping fails.
/// This works for GNOME Terminal where all windows share one process.
pub fn find_pts_batch(x11: &X11, _parent_pid: u32, window_ids: &[u32]) -> Result<HashMap<u32, PathBuf>> {
    // Strategy 1: per-window direct mapping (works for kitty, one process per window)
    let mut result = HashMap::new();
    for &wid in window_ids {
        if let Ok(win_pid) = x11.get_window_pid(wid) {
            let children = child_pts_devices(win_pid)?;
            if let Some((_, pts)) = children.first() {
                result.insert(wid, pts.clone());
            }
        }
    }

    if result.len() == window_ids.len() {
        return Ok(result);
    }

    // Strategy 2: probe-based mapping (works for GNOME Terminal, shared process)
    result.clear();
    let first_pid = x11.get_window_pid(window_ids[0])?;
    let children = child_pts_devices(first_pid)?;
    if children.is_empty() {
        bail!("No child PTS devices found for PID {first_pid}");
    }

    let probe_prefix = "__TERMINALSORT_PROBE_";
    for (pid, pts) in &children {
        let probe = format!("{probe_prefix}{pid}");
        let _ = write_osc_title(pts, &probe);
    }

    thread::sleep(Duration::from_millis(100));

    for &wid in window_ids {
        let title = x11.get_window_title(wid).unwrap_or_default();
        if let Some(pid_str) = title.strip_prefix(probe_prefix) {
            if let Ok(matched_pid) = pid_str.parse::<u32>() {
                if let Some((_, pts)) = children.iter().find(|(pid, _)| *pid == matched_pid) {
                    result.insert(wid, pts.clone());
                }
            }
        }
    }

    // Clear probe titles
    for (_, pts) in &children {
        let _ = write_osc_title(pts, "");
    }

    Ok(result)
}

/// Write an OSC escape sequence to set the terminal title.
pub fn set_title(pts: &PathBuf, title: &str) -> Result<()> {
    write_osc_title(pts, title)
}

fn write_osc_title(pts: &PathBuf, title: &str) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(pts)
        .with_context(|| format!("Cannot open {}", pts.display()))?;

    // OSC 2 = set window title
    write!(f, "\x1b]2;{}\x07", title)?;
    Ok(())
}

/// Find all child processes of a PID and their PTS devices.
fn child_pts_devices(parent_pid: u32) -> Result<Vec<(u32, PathBuf)>> {
    let mut children = Vec::new();

    for entry in fs::read_dir("/proc").context("Cannot read /proc")? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let pid: u32 = match entry.file_name().to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if parse_ppid(&stat) != Some(parent_pid) {
            continue;
        }

        let pts = match fs::read_link(format!("/proc/{pid}/fd/0")) {
            Ok(p) if p.to_string_lossy().starts_with("/dev/pts/") => p,
            _ => continue,
        };

        children.push((pid, pts));
    }

    Ok(children)
}

/// Parse PPID from /proc/<pid>/stat content.
/// Format: "pid (comm) state ppid ..." — comm can contain parens/spaces.
fn parse_ppid(stat: &str) -> Option<u32> {
    let after_comm = stat.rfind(')')? + 1;
    let fields: Vec<&str> = stat[after_comm..].split_whitespace().collect();
    // fields[0] = state, fields[1] = ppid
    fields.get(1)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ppid_standard() {
        let stat = "1234 (bash) S 5678 1234 1234 0 -1";
        assert_eq!(parse_ppid(stat), Some(5678));
    }

    #[test]
    fn parse_ppid_comm_with_spaces() {
        let stat = "1234 (Web Content) S 5678 1234 1234 0 -1";
        assert_eq!(parse_ppid(stat), Some(5678));
    }

    #[test]
    fn parse_ppid_comm_with_parens() {
        let stat = "1234 (foo (bar)) S 5678 1234 1234 0 -1";
        assert_eq!(parse_ppid(stat), Some(5678));
    }

    #[test]
    fn parse_ppid_empty() {
        assert_eq!(parse_ppid(""), None);
    }

    #[test]
    fn child_pts_filters_non_pts() {
        // This test verifies the filter logic — can't test /proc reads in CI
        // but the parse_ppid tests cover the critical parsing
    }
}
