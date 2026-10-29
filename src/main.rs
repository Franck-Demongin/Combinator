use anyhow::Result;
use combinator::{A4_HEIGHT_MM, A4_WIDTH_MM, PAIRS_PER_BOARD, PRINT_MARGIN_MM};

fn main() -> Result<()> {
    println!(
        "Combinator {} - A4 {}x{} mm, {} source pairs/board, {} mm print margin",
        env!("CARGO_PKG_VERSION"),
        A4_WIDTH_MM,
        A4_HEIGHT_MM,
        PAIRS_PER_BOARD,
        PRINT_MARGIN_MM
    );
    Ok(())
}
