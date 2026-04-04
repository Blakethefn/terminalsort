use crate::types::TerminalWindow;
use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux,
    ConnectionExt as _, EventMask,
};
use x11rb::rust_connection::RustConnection;

/// Wrapper around the X11 connection and root window.
pub struct X11 {
    pub conn: RustConnection,
    pub root: xproto::Window,
    pub screen_num: usize,
}

impl X11 {
    /// Connect to the X11 display.
    pub fn connect() -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).context("Cannot connect to X11 display. Is DISPLAY set?")?;
        let root = conn.setup().roots[screen_num].root;
        Ok(Self { conn, root, screen_num })
    }

    /// List all GNOME Terminal windows.
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
                if class.to_lowercase().contains("gnome-terminal") {
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
    fn get_window_title(&self, window: u32) -> Result<String> {
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

    /// Move and resize a window to the given position and size.
    pub fn move_resize(&self, window: u32, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
        // Remove maximized state first so the WM allows repositioning
        self.remove_maximized(window)?;

        let aux = ConfigureWindowAux::new()
            .x(x)
            .y(y)
            .width(width)
            .height(height);

        self.conn.configure_window(window, &aux)?;
        self.conn.flush()?;

        Ok(())
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
