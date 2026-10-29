//! Core, GUI-independent combinator logic.

pub mod composition;
pub mod error;
pub mod sources;

pub const A4_WIDTH_MM: f64 = 297.0;
pub const A4_HEIGHT_MM: f64 = 210.0;
pub const PRINT_MARGIN_MM: f64 = 5.0;
pub const PAIRS_PER_BOARD: usize = 2;
pub const SOURCES_PER_BOARD: usize = 2;

/// Distance between the two source slots inside the printable area.
pub const SLOT_GAP_MM: f64 = PRINT_MARGIN_MM;

/// Dimensions of the region available for one source on an A4 board.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardRegion {
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Returns the shared dimensions of both A4 source slots.
pub fn source_slot_dimensions() -> BoardRegion {
    let usable_width = A4_WIDTH_MM - PRINT_MARGIN_MM * 2.0;
    let slot_width = (usable_width - SLOT_GAP_MM) / PAIRS_PER_BOARD as f64;
    let usable_height = A4_HEIGHT_MM - PRINT_MARGIN_MM * 2.0;
    BoardRegion {
        width_mm: slot_width,
        height_mm: usable_height,
    }
}

pub use composition::{
    Board, CompositionReport, Placement, SourceDimensions, SourceIssue, compose,
};
pub use error::SourceError;
pub use sources::{
    dimensions_from_path, expand_directory, is_supported_path, supported_extensions,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_has_two_source_slots() {
        let slot = source_slot_dimensions();
        assert!(slot.width_mm > 0.0);
        assert!(slot.height_mm > 0.0);
        assert_eq!(PAIRS_PER_BOARD, 2);
    }

    #[test]
    fn slot_fits_a4_with_fixed_margins() {
        let slot = source_slot_dimensions();
        let full_width = PRINT_MARGIN_MM * 2.0 + slot.width_mm + SLOT_GAP_MM + slot.width_mm;
        assert!((full_width - A4_WIDTH_MM).abs() < 0.001);
        assert!(slot.height_mm + PRINT_MARGIN_MM * 2.0 <= A4_HEIGHT_MM + 0.001);
    }
}
