use super::{DisplaySize, truncate_end, truncate_middle};

#[test]
fn truncate_middle_char_boundary() {
    use unicode_width::UnicodeWidthStr;
    let name = "굿걸 - 누가 방송국을 털었나 E06.mp4";
    // 37 columns (in 47 bytes): fits in 44.
    assert_eq!(truncate_middle(name, 44), name);
    let cut = truncate_middle(name, 20);
    assert!(cut.contains("[...]") && cut.width() <= 20, "{cut:?}");
}

#[test]
fn display_size_formats_kilobytes() {
    assert_eq!(format!("{}", DisplaySize(2048.0)), "2.0K");
}

/// Cut by columns, never inside a character: `é` is two bytes and `漢` two
/// columns.
#[test]
fn truncate_end_counts_columns() {
    assert_eq!(truncate_end("ééééé", 5), "ééééé");
    assert_eq!(truncate_end("éééééé", 5), "éé...");
    assert_eq!(truncate_end("漢字漢字", 7), "漢字...");
    assert_eq!(truncate_end("abcdef", 2), "ab");
    assert_eq!(truncate_end("abcdef", 0), "");
}

/// A name that fits in columns is kept whole, however many bytes it has.
#[test]
fn truncate_middle_keeps_names_that_fit() {
    let name = "ж".repeat(30);
    assert_eq!(truncate_middle(&name, 40), name);
}
