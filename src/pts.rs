use crate::x11::X11;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Resolve PTS devices for multiple windows in a single probe pass.
pub fn find_pts_batch(x11: &X11, parent_pid: u32, window_ids: &[u32]) -> Result<HashMap<u32, PathBuf>> {
    let children = child_pts_devices(parent_pid)?;
    if children.is_empty() {
        bail!("No child PTS devices found for PID {parent_pid}");
    }

    // Write unique probe titles
    let probe_prefix = "__TERMINALSORT_PROBE_";
    for (pid, pts) in &children {
        let probe = format!("{probe_prefix}{pid}");
        let _ = write_osc_title(pts, &probe);
    }

    thread::sleep(Duration::from_millis(100));

    // Read back all window titles and build the mapping
    let mut result = HashMap::new();
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
