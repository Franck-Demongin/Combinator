//! A4 composition planning without PDF generation or file reading.

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceDimensions {
    pub width_mm: f64,
    pub height_mm: f64,
}

impl SourceDimensions {
    pub fn new(width_mm: f64, height_mm: f64) -> Option<Self> {
        if !width_mm.is_finite() || !height_mm.is_finite() || width_mm <= 0.0 || height_mm <= 0.0 {
            None
        } else {
            Some(Self {
                width_mm,
                height_mm,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Rotation {
    None,
    Degrees90,
}

impl Rotation {
    pub fn degrees(&self) -> u16 {
        match self {
            Self::None => 0,
            Self::Degrees90 => 90,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FitStatus {
    Fits,
    ScaledUp,
    ScaledDown,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Placement {
    pub source_index: usize,
    pub slot_index: usize,
    pub slot_x_mm: f64,
    pub slot_y_mm: f64,
    pub slot_width_mm: f64,
    pub slot_height_mm: f64,
    pub source_width_mm: f64,
    pub source_height_mm: f64,
    pub rotation: Rotation,
    pub rotation_degrees: u16,
    pub scale_factor: f64,
    pub placed_width_mm: f64,
    pub placed_height_mm: f64,
    pub fit_status: FitStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Board {
    pub index: usize,
    pub width_mm: f64,
    pub height_mm: f64,
    pub placements: Vec<Placement>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceIssue {
    pub source_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompositionReport {
    pub board_width_mm: f64,
    pub board_height_mm: f64,
    pub print_margin_mm: f64,
    pub slot_width_mm: f64,
    pub slot_height_mm: f64,
    pub slot_gap_mm: f64,
    pub source_count: usize,
    pub board_count: usize,
    pub boards: Vec<Board>,
    pub issues: Vec<SourceIssue>,
    pub warnings: Vec<String>,
}

fn candidate_scale(source: SourceDimensions, rotation: Rotation, slot: (f64, f64)) -> f64 {
    let (width, height) = match rotation {
        Rotation::None => (source.width_mm, source.height_mm),
        Rotation::Degrees90 => (source.height_mm, source.width_mm),
    };
    (slot.0 / width).min(slot.1 / height)
}

fn fit_status(scale_factor: f64) -> FitStatus {
    let tolerance = 0.0001;
    if (scale_factor - 1.0).abs() <= tolerance {
        FitStatus::Fits
    } else if scale_factor > 1.0 {
        FitStatus::ScaledUp
    } else {
        FitStatus::ScaledDown
    }
}

/// Composes sources into A4 boards, two sources per board.
///
/// Each source is compared in both orientations and rotated when that
/// maximises its scale inside its fixed A4 slot.
pub fn compose(sources: &[SourceDimensions]) -> CompositionReport {
    let slot = crate::source_slot_dimensions();
    let slot_width = slot.width_mm;
    let slot_height = slot.height_mm;
    let left_origin = crate::PRINT_MARGIN_MM;
    let right_origin = crate::PRINT_MARGIN_MM + slot_width + crate::SLOT_GAP_MM;
    let vertical_origin = crate::PRINT_MARGIN_MM;

    let mut boards = Vec::new();
    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    let mut valid_sources: Vec<(usize, SourceDimensions)> = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        if !source.width_mm.is_finite()
            || !source.height_mm.is_finite()
            || source.width_mm <= 0.0
            || source.height_mm <= 0.0
        {
            issues.push(SourceIssue {
                source_index: index,
                reason: "invalid dimensions".to_owned(),
            });
            warnings.push(format!(
                "Source {index} was skipped because its dimensions are invalid."
            ));
        } else {
            valid_sources.push((index, *source));
        }
    }

    for (board_index, pair) in valid_sources.chunks(2).enumerate() {
        let mut placements = Vec::new();
        for (slot_index, (source_index, source)) in pair.iter().enumerate() {
            let direct = Rotation::None;
            let rotated = Rotation::Degrees90;
            let direct_scale = candidate_scale(*source, direct, (slot_width, slot_height));
            let rotated_scale = candidate_scale(*source, rotated, (slot_width, slot_height));
            let (rotation, scale_factor) = if rotated_scale > direct_scale {
                (rotated, rotated_scale)
            } else {
                (direct, direct_scale)
            };

            let (placed_width, placed_height) = match rotation {
                Rotation::None => (
                    source.width_mm * scale_factor,
                    source.height_mm * scale_factor,
                ),
                Rotation::Degrees90 => (
                    source.height_mm * scale_factor,
                    source.width_mm * scale_factor,
                ),
            };

            let origin_x = if slot_index == 0 {
                left_origin
            } else {
                right_origin
            };

            placements.push(Placement {
                source_index: *source_index,
                slot_index,
                slot_x_mm: origin_x,
                slot_y_mm: vertical_origin,
                slot_width_mm: slot_width,
                slot_height_mm: slot_height,
                source_width_mm: source.width_mm,
                source_height_mm: source.height_mm,
                rotation,
                rotation_degrees: rotation.degrees(),
                scale_factor,
                placed_width_mm: placed_width,
                placed_height_mm: placed_height,
                fit_status: fit_status(scale_factor),
            });
        }

        if pair.len() == 1 {
            warnings.push(format!(
                "Source {} has no pair and occupies only the left half of board {}.",
                pair[0].0, board_index
            ));
        }

        boards.push(Board {
            index: board_index,
            width_mm: crate::A4_WIDTH_MM,
            height_mm: crate::A4_HEIGHT_MM,
            placements,
        });
    }

    if sources.is_empty() {
        warnings.push("No sources were provided.".to_owned());
    }

    CompositionReport {
        board_width_mm: crate::A4_WIDTH_MM,
        board_height_mm: crate::A4_HEIGHT_MM,
        print_margin_mm: crate::PRINT_MARGIN_MM,
        slot_width_mm: slot_width,
        slot_height_mm: slot_height,
        slot_gap_mm: crate::SLOT_GAP_MM,
        source_count: sources.len(),
        board_count: boards.len(),
        boards,
        issues,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_pairs() {
        let report = compose(&[
            SourceDimensions::new(90.0, 260.0).unwrap(),
            SourceDimensions::new(90.0, 260.0).unwrap(),
            SourceDimensions::new(90.0, 260.0).unwrap(),
            SourceDimensions::new(90.0, 260.0).unwrap(),
        ]);
        assert_eq!(report.source_count, 4);
        assert_eq!(report.board_count, 2);
        assert_eq!(report.boards[0].placements.len(), 2);
    }

    #[test]
    fn rotates_to_improve_fit() {
        let report = compose(&[SourceDimensions::new(300.0, 100.0).unwrap()]);
        let placement = &report.boards[0].placements[0];
        assert_eq!(placement.rotation, Rotation::Degrees90);
        assert!(placement.scale_factor > 0.9);
    }

    #[test]
    fn keeps_small_source_no_upscale_beyond_slot() {
        let report = compose(&[SourceDimensions::new(20.0, 20.0).unwrap()]);
        let placement = &report.boards[0].placements[0];
        assert!(placement.scale_factor > 1.0);
        assert!(placement.placed_width_mm <= placement.slot_width_mm + 0.001);
        assert!(placement.placed_height_mm <= placement.slot_height_mm + 0.001);
    }

    #[test]
    fn skips_invalid_dimensions() {
        let invalid = SourceDimensions {
            width_mm: 0.0,
            height_mm: 10.0,
        };
        let report = compose(&[invalid]);
        assert_eq!(report.boards.len(), 0);
        assert_eq!(report.issues.len(), 1);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn handles_unpaired_source() {
        let report = compose(&[
            SourceDimensions::new(50.0, 50.0).unwrap(),
            SourceDimensions::new(50.0, 50.0).unwrap(),
            SourceDimensions::new(50.0, 50.0).unwrap(),
        ]);
        assert_eq!(report.board_count, 2);
        let warnings = report.warnings.join(" ");
        assert!(warnings.contains("no pair"));
    }
}
