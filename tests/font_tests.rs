use terminalsort::font::{parse_font_size, scaled_font_size, with_font_size};

#[test]
fn parse_font_size_standard() {
    assert_eq!(parse_font_size("Monospace 12"), Some(12.0));
}

#[test]
fn parse_font_size_with_style() {
    assert_eq!(parse_font_size("DejaVu Sans Mono 14"), Some(14.0));
}

#[test]
fn parse_font_size_none_for_garbage() {
    assert_eq!(parse_font_size("nosize"), None);
}

#[test]
fn with_font_size_replaces() {
    assert_eq!(with_font_size("Monospace 12", 8.0), "Monospace 8");
}

#[test]
fn with_font_size_multi_word_family() {
    assert_eq!(
        with_font_size("DejaVu Sans Mono 14", 10.0),
        "DejaVu Sans Mono 10"
    );
}

#[test]
fn scaled_size_4_windows() {
    // 12 * 2/sqrt(4) = 12 * 1.0 = 12
    assert_eq!(scaled_font_size(12.0, 4), 12.0);
}

#[test]
fn scaled_size_8_windows() {
    // 12 * 2/sqrt(8) = 12 * 0.707 = 8.485 → rounds to 8
    assert_eq!(scaled_font_size(12.0, 8), 8.0);
}

#[test]
fn scaled_size_floors_at_6() {
    // 12 * 2/sqrt(100) = 12 * 0.2 = 2.4 → clamped to 6
    assert_eq!(scaled_font_size(12.0, 100), 6.0);
}

#[test]
fn scaled_size_2_windows() {
    // 12 * 2/sqrt(2) = 12 * 1.414 = 16.97 → rounds to 17
    assert_eq!(scaled_font_size(12.0, 2), 17.0);
}
