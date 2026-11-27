//! Chart/plot-area DrawingML `a:pattFill`.
//!
//! An 8x8 preset cell lives in 96-dpi pixel space and is scaled by the
//! request DPI (same native scale as the rest of chart paint). `ltHorz` is
//! the standard light-horizontal hatch: two foreground rows, four cells
//! apart. Other presets stay NI until official gold uniquely locks them.
//! Pixel-center sampling is the general rule; Chart.Export palette
//! quantization and residual phase at non-96 dpi stay on the review page.

pub(super) const CELL: i32 = 8;

/// Light horizontal: foreground on pattern rows 0 and 4.
const LT_HORZ: [u8; 8] = [0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PatternPreset {
    LtHorz,
}

impl PatternPreset {
    pub(super) fn parse(prst: &str) -> Option<Self> {
        match prst {
            "ltHorz" => Some(Self::LtHorz),
            _ => None,
        }
    }

    fn bits(self) -> [u8; 8] {
        match self {
            Self::LtHorz => LT_HORZ,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PatternFill {
    pub(super) prst: PatternPreset,
    pub(super) fg: [u8; 3],
    pub(super) bg: [u8; 3],
}

impl PatternFill {
    pub(super) fn color_at(&self, x: i32, y: i32, dpi: f64) -> [u8; 3] {
        if is_foreground(self.prst, x, y, dpi) {
            self.fg
        } else {
            self.bg
        }
    }
}

pub(super) fn is_foreground(prst: PatternPreset, x: i32, y: i32, dpi: f64) -> bool {
    let scale = 96.0 / dpi;
    let col = ((f64::from(x) + 0.5) * scale).floor() as i32;
    let row = ((f64::from(y) + 0.5) * scale).floor() as i32;
    let bits = prst.bits();
    let c = col.rem_euclid(CELL) as usize;
    let r = row.rem_euclid(CELL) as usize;
    bits[r] & (0x80 >> c) != 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlipFill {
    pub(super) embed: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lt_horz_is_foreground_on_rows_zero_and_four_at_96_dpi() {
        for y in 0..16 {
            let fg = is_foreground(PatternPreset::LtHorz, 0, y, 96.0);
            assert_eq!(fg, y % 8 == 0 || y % 8 == 4, "y={y}");
        }
        // Horizontal: every column in a fg row is fg.
        for x in 0..8 {
            assert!(is_foreground(PatternPreset::LtHorz, x, 0, 96.0));
            assert!(!is_foreground(PatternPreset::LtHorz, x, 1, 96.0));
        }
    }
}
