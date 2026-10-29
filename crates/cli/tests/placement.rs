use combinator_core::{PRINT_MARGIN_MM, SOURCES_PER_BOARD, source_slot_dimensions};

#[test]
fn source_slot_is_usable() {
    let slot = source_slot_dimensions();
    assert_eq!(SOURCES_PER_BOARD, 2);
    assert!(slot.width_mm > 0.0);
    assert!(slot.height_mm > 0.0);
    let _ = PRINT_MARGIN_MM;
}
