use crate::types::{FrameExtents, Rect};

#[derive(Debug)]
pub enum LayoutError {
    UnknownLayout(String),
    WindowCountMismatch {
        layout: String,
        expected: String,
        got: usize,
    },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::UnknownLayout(name) => {
                write!(f, "Unknown layout '{name}'. Available: h2, v2, h3, v3, grid")
            }
            LayoutError::WindowCountMismatch { layout, expected, got } => {
                write!(
                    f,
                    "Layout '{layout}' requires {expected} windows, but {got} were selected. Try 'grid' for {got} windows."
                )
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Calculate window rectangles for the given layout.
///
/// `mon_x`, `mon_y` are the monitor's top-left offset in screen coordinates.
/// `mon_w`, `mon_h` are the monitor's dimensions.
/// `count` is the number of windows to tile.
/// `frame` provides WM decoration sizes so cell heights/widths account for title bars.
pub fn calculate_layout(
    layout: &str,
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    frame: &FrameExtents,
) -> Result<Vec<Rect>, LayoutError> {
    match layout {
        "h2" => fixed_horizontal(mon_x, mon_y, mon_w, mon_h, count, 2, frame),
        "v2" => fixed_vertical(mon_x, mon_y, mon_w, mon_h, count, 2, frame),
        "h3" => fixed_horizontal(mon_x, mon_y, mon_w, mon_h, count, 3, frame),
        "v3" => fixed_vertical(mon_x, mon_y, mon_w, mon_h, count, 3, frame),
        "grid" => grid(mon_x, mon_y, mon_w, mon_h, count, frame),
        other => Err(LayoutError::UnknownLayout(other.to_string())),
    }
}

fn fixed_horizontal(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    expected: usize,
    frame: &FrameExtents,
) -> Result<Vec<Rect>, LayoutError> {
    if count != expected {
        return Err(LayoutError::WindowCountMismatch {
            layout: format!("h{expected}"),
            expected: format!("exactly {expected}"),
            got: count,
        });
    }
    // Single row: subtract frame top/bottom once from height
    let cell_h = mon_h.saturating_sub(frame.top + frame.bottom);
    let cell_w = mon_w / expected as u32;
    let rects = (0..expected)
        .map(|i| Rect {
            x: mon_x + (i as u32 * cell_w) as i32,
            y: mon_y + frame.top as i32,
            width: cell_w,
            height: cell_h,
        })
        .collect();
    Ok(rects)
}

fn fixed_vertical(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    expected: usize,
    frame: &FrameExtents,
) -> Result<Vec<Rect>, LayoutError> {
    if count != expected {
        return Err(LayoutError::WindowCountMismatch {
            layout: format!("v{expected}"),
            expected: format!("exactly {expected}"),
            got: count,
        });
    }
    // Each row has frame_top + cell_h + frame_bottom visual height
    // rows * (frame_top + cell_h + frame_bottom) = mon_h
    let frame_v = frame.top + frame.bottom;
    let cell_h = mon_h.saturating_sub(expected as u32 * frame_v) / expected as u32;
    let stride = cell_h + frame_v;
    let rects = (0..expected)
        .map(|i| Rect {
            x: mon_x,
            y: mon_y + frame.top as i32 + (i as u32 * stride) as i32,
            width: mon_w,
            height: cell_h,
        })
        .collect();
    Ok(rects)
}

fn grid(
    mon_x: i32,
    mon_y: i32,
    mon_w: u32,
    mon_h: u32,
    count: usize,
    frame: &FrameExtents,
) -> Result<Vec<Rect>, LayoutError> {
    if count < 2 {
        return Err(LayoutError::WindowCountMismatch {
            layout: "grid".to_string(),
            expected: "at least 2".to_string(),
            got: count,
        });
    }

    // Prefer fewer rows (wider cells are better for terminals).
    // Perfect squares use square grid (4→2x2, 9→3x3, 16→4x4).
    // Otherwise minimize rows, capping at 6 columns (6→2x3, 8→2x4, 10→2x5).
    let sq_root = (count as f64).sqrt().round() as usize;
    let (rows, cols) = if sq_root * sq_root == count {
        (sq_root, sq_root)
    } else {
        let mut r = 2;
        while count.div_ceil(r) > 6 && r < count {
            r += 1;
        }
        (r, count.div_ceil(r))
    };

    // Account for WM decorations: each row costs frame_top + frame_bottom extra
    let frame_v = frame.top + frame.bottom;
    let cell_h = mon_h.saturating_sub(rows as u32 * frame_v) / rows as u32;
    let row_stride = cell_h + frame_v;

    let cell_w = mon_w / cols as u32;

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let row = i / cols;
        let col = i % cols;

        let windows_in_row = if row == rows - 1 {
            count - row * cols
        } else {
            cols
        };

        let this_cell_w = if windows_in_row < cols && col == windows_in_row - 1 {
            mon_w - (col as u32 * (mon_w / windows_in_row as u32))
        } else if windows_in_row < cols {
            mon_w / windows_in_row as u32
        } else {
            cell_w
        };

        let this_x = if windows_in_row < cols {
            mon_x + (col as u32 * (mon_w / windows_in_row as u32)) as i32
        } else {
            mon_x + (col as u32 * cell_w) as i32
        };

        rects.push(Rect {
            x: this_x,
            y: mon_y + frame.top as i32 + (row as u32 * row_stride) as i32,
            width: this_cell_w,
            height: cell_h,
        });
    }

    Ok(rects)
}
