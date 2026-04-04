use crate::types::Monitor;
use crate::x11::X11;
use anyhow::{bail, Result};
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::ConnectionExt as _;

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
