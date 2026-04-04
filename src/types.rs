/// A monitor's geometry as reported by RANDR.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub index: usize,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

/// A window rectangle in absolute screen coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A terminal window discovered via X11.
#[derive(Debug, Clone)]
pub struct TerminalWindow {
    pub id: u32,
    pub title: String,
}
