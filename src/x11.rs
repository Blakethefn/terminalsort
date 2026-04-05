use crate::types::{Rect, TerminalWindow};
use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux,
    ConnectionExt as _, EventMask,
};
use x11rb::rust_connection::RustConnection;

/// Known terminal emulator WM_CLASS values.
const TERMINAL_CLASSES: &[&str] = &["gnome-terminal", "kitty"];

/// Check if a WM_CLASS string belongs to a supported terminal emulator.
pub fn is_terminal_class(class: &str) -> bool {
    let lower = class.to_lowercase();
    TERMINAL_CLASSES.iter().any(|t| lower.contains(t))
}

/// Wrapper around the X11 connection and root window.
pub struct X11 {
    pub conn: RustConnection,
    pub root: xproto::Window,
}

impl X11 {
    /// Connect to the X11 display.
    pub fn connect() -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).context("Cannot connect to X11 display. Is DISPLAY set?")?;
        let root = conn.setup().roots[screen_num].root;
        Ok(Self { conn, root })
    }

    /// List all terminal windows (GNOME Terminal, Kitty, etc.).
    pub fn list_terminal_windows(&self) -> Result<Vec<TerminalWindow>> {
        let net_client_list = self
            .conn
            .intern_atom(false, b"_NET_CLIENT_LIST")?
            .reply()?
            .atom;

        let reply = self
            .conn
            .get_property(false, self.root, net_client_list, AtomEnum::WINDOW, 0, u32::MAX)?
            .reply()?;

        let all_windows: Vec<u32> = reply.value32().map(|iter| iter.collect()).unwrap_or_default();

        let mut terminals = Vec::new();
        for wid in all_windows {
            if let Ok((_, class)) = self.get_wm_class(wid) {
                if is_terminal_class(&class) {
                    let title = self.get_window_title(wid).unwrap_or_default();
                    terminals.push(TerminalWindow { id: wid, title });
                }
            }
        }

        Ok(terminals)
    }

    /// Read WM_CLASS (instance, class) from a window.
    fn get_wm_class(&self, window: u32) -> Result<(String, String)> {
        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, u32::MAX)?
            .reply()?;

        let mut parts = reply.value.split(|&b| b == 0).filter(|s| !s.is_empty());

        let instance = parts
            .next()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let class = parts
            .next()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();

        Ok((instance, class))
    }

    /// Read window title (_NET_WM_NAME falling back to WM_NAME).
    pub fn get_window_title(&self, window: u32) -> Result<String> {
        let net_wm_name = self.conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
        let utf8_string = self.conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom;

        let reply = self
            .conn
            .get_property(false, window, net_wm_name, utf8_string, 0, u32::MAX)?
            .reply()?;

        if !reply.value.is_empty() {
            return Ok(String::from_utf8_lossy(&reply.value).into_owned());
        }

        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)?
            .reply()?;

        Ok(String::from_utf8_lossy(&reply.value).into_owned())
    }

    /// Get _NET_WORKAREA from the root window.
    /// Returns (x, y, width, height) of the first workarea entry.
    pub fn get_workarea(&self) -> Result<Rect> {
        let net_workarea = self
            .conn
            .intern_atom(false, b"_NET_WORKAREA")?
            .reply()?
            .atom;

        let reply = self
            .conn
            .get_property(false, self.root, net_workarea, AtomEnum::CARDINAL, 0, 4)?
            .reply()?;

        let values: Vec<u32> = reply.value32().map(|v| v.collect()).unwrap_or_default();

        if values.len() >= 4 {
            Ok(Rect {
                x: values[0] as i32,
                y: values[1] as i32,
                width: values[2],
                height: values[3],
            })
        } else {
            // Fallback: full screen
            let screen = &self.conn.setup().roots[0];
            Ok(Rect {
                x: 0,
                y: 0,
                width: screen.width_in_pixels as u32,
                height: screen.height_in_pixels as u32,
            })
        }
    }

    /// Get _GTK_FRAME_EXTENTS (CSD shadow/border) for a window.
    /// Returns (left, right, top, bottom) or (0,0,0,0) if not set.
    pub fn get_frame_extents(&self, window: u32) -> Result<(u32, u32, u32, u32)> {
        let gtk_frame = self
            .conn
            .intern_atom(false, b"_GTK_FRAME_EXTENTS")?
            .reply()?
            .atom;

        let reply = self
            .conn
            .get_property(false, window, gtk_frame, AtomEnum::CARDINAL, 0, 4)?
            .reply()?;

        let values: Vec<u32> = reply.value32().map(|v| v.collect()).unwrap_or_default();

        if values.len() >= 4 {
            Ok((values[0], values[1], values[2], values[3]))
        } else {
            Ok((0, 0, 0, 0))
        }
    }

    /// Move and resize a window to fill the given rectangle exactly.
    /// Accounts for CSD frame extents (shadows/borders) so the visible
    /// window content fills the target area precisely.
    pub fn move_resize(&self, window: u32, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
        // Remove maximized state first so the WM allows repositioning
        self.remove_maximized(window)?;

        // Account for CSD frame extents — the window's outer bounds include
        // invisible shadow/border areas. We need to expand the configure request
        // so the visible content fills our target rect.
        let (fl, fr, ft, fb) = self.get_frame_extents(window)?;

        let adj_x = x - fl as i32;
        let adj_y = y - ft as i32;
        let adj_w = width + fl + fr;
        let adj_h = height + ft + fb;

        let aux = ConfigureWindowAux::new()
            .x(adj_x)
            .y(adj_y)
            .width(adj_w)
            .height(adj_h);

        self.conn.configure_window(window, &aux)?;
        self.conn.flush()?;

        Ok(())
    }

    /// Get _NET_WM_PID from a window.
    pub fn get_window_pid(&self, window: u32) -> Result<u32> {
        let net_wm_pid = self.conn.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;

        let reply = self
            .conn
            .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)?
            .reply()?;

        let values: Vec<u32> = reply.value32().map(|v| v.collect()).unwrap_or_default();
        values
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No _NET_WM_PID on window {:#010x}", window))
    }

    /// Check if a window still exists on the X server.
    pub fn window_exists(&self, window: u32) -> bool {
        self.conn
            .get_window_attributes(window)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some()
    }

    /// Remove _NET_WM_STATE_MAXIMIZED_HORZ and _VERT from a window.
    fn remove_maximized(&self, window: u32) -> Result<()> {
        let net_wm_state = self.conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
        let max_h = self
            .conn
            .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ")?
            .reply()?
            .atom;
        let max_v = self
            .conn
            .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT")?
            .reply()?
            .atom;

        // Action 0 = _NET_WM_STATE_REMOVE
        let data = ClientMessageData::from([0u32, max_h, max_v, 1, 0]);
        let event = ClientMessageEvent::new(32, window, net_wm_state, data);

        self.conn.send_event(
            false,
            self.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;
        self.conn.flush()?;

        Ok(())
    }
}
