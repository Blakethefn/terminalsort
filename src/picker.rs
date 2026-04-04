use crate::types::TerminalWindow;
use crate::x11::X11;
use anyhow::{bail, Result};
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
