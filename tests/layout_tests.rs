use terminalsort::layout::{calculate_layout, LayoutError};
use terminalsort::types::{FrameExtents, Rect};

const NO_FRAME: FrameExtents = FrameExtents { left: 0, right: 0, top: 0, bottom: 0 };

#[test]
fn h2_splits_horizontally() {
    let rects = calculate_layout("h2", 0, 0, 1920, 1080, 2, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 960, height: 1080 });
    assert_eq!(rects[1], Rect { x: 960, y: 0, width: 960, height: 1080 });
}

#[test]
fn v2_splits_vertically() {
    let rects = calculate_layout("v2", 0, 0, 1920, 1080, 2, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 1920, height: 540 });
    assert_eq!(rects[1], Rect { x: 0, y: 540, width: 1920, height: 540 });
}

#[test]
fn h3_splits_three_horizontal() {
    let rects = calculate_layout("h3", 0, 0, 1920, 1080, 3, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 640, height: 1080 });
    assert_eq!(rects[1], Rect { x: 640, y: 0, width: 640, height: 1080 });
    assert_eq!(rects[2], Rect { x: 1280, y: 0, width: 640, height: 1080 });
}

#[test]
fn v3_splits_three_vertical() {
    let rects = calculate_layout("v3", 0, 0, 1920, 1080, 3, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 1920, height: 360 });
    assert_eq!(rects[1], Rect { x: 0, y: 360, width: 1920, height: 360 });
    assert_eq!(rects[2], Rect { x: 0, y: 720, width: 1920, height: 360 });
}

#[test]
fn grid_4_makes_2x2() {
    let rects = calculate_layout("grid", 0, 0, 1920, 1080, 4, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 4);
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 960, height: 540 });
    assert_eq!(rects[1], Rect { x: 960, y: 0, width: 960, height: 540 });
    assert_eq!(rects[2], Rect { x: 0, y: 540, width: 960, height: 540 });
    assert_eq!(rects[3], Rect { x: 960, y: 540, width: 960, height: 540 });
}

#[test]
fn grid_6_makes_2x3() {
    let rects = calculate_layout("grid", 0, 0, 1800, 1000, 6, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 6);
    // 3 cols, 2 rows -> each cell 600x500
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 600, height: 500 });
    assert_eq!(rects[5], Rect { x: 1200, y: 500, width: 600, height: 500 });
}

#[test]
fn grid_8_makes_2x4() {
    let rects = calculate_layout("grid", 0, 0, 1920, 1080, 8, &NO_FRAME).unwrap();
    assert_eq!(rects.len(), 8);
    // 2 rows, 4 cols -> each cell 480x540
    assert_eq!(rects[0], Rect { x: 0, y: 0, width: 480, height: 540 });
    assert_eq!(rects[3], Rect { x: 1440, y: 0, width: 480, height: 540 });
    assert_eq!(rects[4], Rect { x: 0, y: 540, width: 480, height: 540 });
    assert_eq!(rects[7], Rect { x: 1440, y: 540, width: 480, height: 540 });
}

#[test]
fn grid_with_monitor_offset() {
    let rects = calculate_layout("grid", 1920, 0, 1920, 1080, 4, &NO_FRAME).unwrap();
    assert_eq!(rects[0], Rect { x: 1920, y: 0, width: 960, height: 540 });
    assert_eq!(rects[3], Rect { x: 2880, y: 540, width: 960, height: 540 });
}

#[test]
fn grid_with_frame_extents() {
    // 37px title bar like kitty on a typical WM
    let frame = FrameExtents { left: 0, right: 0, top: 37, bottom: 0 };
    let rects = calculate_layout("grid", 0, 0, 1920, 1080, 4, &frame).unwrap();
    assert_eq!(rects.len(), 4);
    // 2 rows, 2 cols: cell_h = (1080 - 2*37) / 2 = 503
    assert_eq!(rects[0], Rect { x: 0, y: 37, width: 960, height: 503 });
    assert_eq!(rects[1], Rect { x: 960, y: 37, width: 960, height: 503 });
    // Row 1: y = 37 + 1*(503+37) = 577
    assert_eq!(rects[2], Rect { x: 0, y: 577, width: 960, height: 503 });
    assert_eq!(rects[3], Rect { x: 960, y: 577, width: 960, height: 503 });
}

#[test]
fn v2_with_frame_extents() {
    let frame = FrameExtents { left: 0, right: 0, top: 37, bottom: 0 };
    let rects = calculate_layout("v2", 0, 0, 1920, 1080, 2, &frame).unwrap();
    assert_eq!(rects.len(), 2);
    // cell_h = (1080 - 2*37) / 2 = 503
    assert_eq!(rects[0], Rect { x: 0, y: 37, width: 1920, height: 503 });
    assert_eq!(rects[1], Rect { x: 0, y: 577, width: 1920, height: 503 });
}

#[test]
fn h2_wrong_count_errors() {
    let err = calculate_layout("h2", 0, 0, 1920, 1080, 3, &NO_FRAME).unwrap_err();
    assert!(matches!(err, LayoutError::WindowCountMismatch { .. }));
}

#[test]
fn grid_needs_at_least_2() {
    let err = calculate_layout("grid", 0, 0, 1920, 1080, 1, &NO_FRAME).unwrap_err();
    assert!(matches!(err, LayoutError::WindowCountMismatch { .. }));
}

#[test]
fn unknown_layout_errors() {
    let err = calculate_layout("potato", 0, 0, 1920, 1080, 2, &NO_FRAME).unwrap_err();
    assert!(matches!(err, LayoutError::UnknownLayout(_)));
}
