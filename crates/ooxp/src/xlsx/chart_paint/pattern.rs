//! DrawingML `a:pattFill` / `a:blipFill` foundations for chart frames.
//!
//! Pattern sampling is a general RULE, not a per-fixture fudge:
//! an 8×8 cell in 96-dpi pixel space, scaled by the request DPI, with
//! pixel-center sampling. Official Chart.Export of `ltHorz` at ~106 dpi
//! is a palette PNG whose period alternates 4/5 px (≈4.42 = 4×106/96).
//! Coverage/AA was not uniquely lockable against that quantization; the
//! binary hatch is the locked general rule. Phase residuals at non-96 dpi
//! stay on the review page; do not invent a per-fixture origin.
//!
//! Presets other than `ltHorz` use close-enough 8×8 bitmaps derived from
//! the preset name (density, alternating rows/columns, diagonals, checks).
//! Those bitmaps are not uniquely locked from an official matrix; residuals
//! stay on the review page and must not be presented as a unique lock.
//!
//! Stretch `a:blipFill` with empty/zero `srcRect`/`fillRect` and a no-op
//! effect list reuses the existing GDI+ HighQualityBicubic scaler
//! (`chart_gdi::scale_device_rgba`). Tile, crop, non-zero fillRect, a
//! non-zero `dpi`, and remaining blip effects use close-enough geometry so
//! Office-exportable picture fills produce an image instead of rejecting.
//! Residuals stay on the review page; do not claim zero-pixel identity.

/// ST_PresetPatternVal names we can render as an 8×8 cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PatternPreset {
    LtHorz,
    Horz,
    DkHorz,
    NarHorz,
    DashHorz,
    LtVert,
    Vert,
    DkVert,
    NarVert,
    DashVert,
    Pct(u8),
    Cross,
    DnDiag,
    UpDiag,
    LtDnDiag,
    LtUpDiag,
    DkDnDiag,
    DkUpDiag,
    DashDnDiag,
    DashUpDiag,
    WdDnDiag,
    WdUpDiag,
    SmCheck,
    LgCheck,
    SmGrid,
    LgGrid,
    DotGrid,
    SmConfetti,
    LgConfetti,
    HorzBrick,
    DiagBrick,
    SolidDmnd,
    OpenDmnd,
    DotDmnd,
    Plaid,
    Sphere,
    Weave,
    Divot,
    Shingle,
    Wave,
    Trellis,
    ZigZag,
}

impl PatternPreset {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "ltHorz" => Self::LtHorz,
            "horz" => Self::Horz,
            "dkHorz" => Self::DkHorz,
            "narHorz" => Self::NarHorz,
            "dashHorz" => Self::DashHorz,
            "ltVert" => Self::LtVert,
            "vert" => Self::Vert,
            "dkVert" => Self::DkVert,
            "narVert" => Self::NarVert,
            "dashVert" => Self::DashVert,
            "pct5" => Self::Pct(5),
            "pct10" => Self::Pct(10),
            "pct20" => Self::Pct(20),
            "pct25" => Self::Pct(25),
            "pct30" => Self::Pct(30),
            "pct40" => Self::Pct(40),
            "pct50" => Self::Pct(50),
            "pct60" => Self::Pct(60),
            "pct70" => Self::Pct(70),
            "pct75" => Self::Pct(75),
            "pct80" => Self::Pct(80),
            "pct90" => Self::Pct(90),
            "cross" => Self::Cross,
            "dnDiag" => Self::DnDiag,
            "upDiag" => Self::UpDiag,
            "ltDnDiag" => Self::LtDnDiag,
            "ltUpDiag" => Self::LtUpDiag,
            "dkDnDiag" => Self::DkDnDiag,
            "dkUpDiag" => Self::DkUpDiag,
            "dashDnDiag" => Self::DashDnDiag,
            "dashUpDiag" => Self::DashUpDiag,
            "wdDnDiag" => Self::WdDnDiag,
            "wdUpDiag" => Self::WdUpDiag,
            "smCheck" => Self::SmCheck,
            "lgCheck" => Self::LgCheck,
            "smGrid" => Self::SmGrid,
            "lgGrid" => Self::LgGrid,
            "dotGrid" => Self::DotGrid,
            "smConfetti" => Self::SmConfetti,
            "lgConfetti" => Self::LgConfetti,
            "horzBrick" => Self::HorzBrick,
            "diagBrick" => Self::DiagBrick,
            "solidDmnd" => Self::SolidDmnd,
            "openDmnd" => Self::OpenDmnd,
            "dotDmnd" => Self::DotDmnd,
            "plaid" => Self::Plaid,
            "sphere" => Self::Sphere,
            "weave" => Self::Weave,
            "divot" => Self::Divot,
            "shingle" => Self::Shingle,
            "wave" => Self::Wave,
            "trellis" => Self::Trellis,
            "zigZag" => Self::ZigZag,
            _ => return None,
        })
    }

    /// Close-enough 8×8 bits, MSB = x=0. `ltHorz` matches the locked foundation
    /// (foreground on pattern rows 0 and 4). Other presets are named-geometry
    /// approximations, not uniquely locked official bitmaps.
    fn bits(self) -> [u8; 8] {
        match self {
            Self::LtHorz => from_pred(|_x, y| y == 0 || y == 4),
            Self::Horz => from_pred(|_x, y| y % 4 < 2),
            Self::DkHorz => from_pred(|_x, y| y % 4 < 3),
            Self::NarHorz => from_pred(|_x, y| y % 2 == 0),
            Self::DashHorz => from_pred(|x, y| (y == 0 || y == 4) && (x % 8) < 4),
            Self::LtVert => from_pred(|x, _y| x == 0 || x == 4),
            Self::Vert => from_pred(|x, _y| x % 4 < 2),
            Self::DkVert => from_pred(|x, _y| x % 4 < 3),
            Self::NarVert => from_pred(|x, _y| x % 2 == 0),
            Self::DashVert => from_pred(|x, y| (x == 0 || x == 4) && (y % 8) < 4),
            Self::Pct(n) => bayer_pct(n),
            Self::Cross => from_pred(|x, y| y % 4 < 2 || x % 4 < 2),
            Self::LtDnDiag => from_pred(|x, y| (x + y) % 8 == 0),
            Self::DnDiag => from_pred(|x, y| (x + y) % 4 == 0),
            Self::DkDnDiag => from_pred(|x, y| (x + y) % 4 <= 1),
            Self::WdDnDiag => from_pred(|x, y| (x + y) % 8 <= 2),
            Self::LtUpDiag => from_pred(|x, y| (x - y).rem_euclid(8) == 0),
            Self::UpDiag => from_pred(|x, y| (x - y).rem_euclid(4) == 0),
            Self::DkUpDiag => from_pred(|x, y| (x - y).rem_euclid(4) <= 1),
            Self::WdUpDiag => from_pred(|x, y| (x - y).rem_euclid(8) <= 2),
            Self::DashDnDiag => from_pred(|x, y| (x + y) % 8 == 0 && (x % 8) < 4),
            Self::DashUpDiag => from_pred(|x, y| (x - y).rem_euclid(8) == 0 && (x % 8) < 4),
            Self::SmCheck => from_pred(|x, y| ((x / 2) + (y / 2)) % 2 == 0),
            Self::LgCheck => from_pred(|x, y| ((x / 4) + (y / 4)) % 2 == 0),
            Self::SmGrid => from_pred(|x, y| x % 4 == 0 || y % 4 == 0),
            Self::LgGrid => from_pred(|x, y| x == 0 || y == 0),
            Self::DotGrid => from_pred(|x, y| (x % 4 == 0 && y % 4 == 0) || (x % 4 == 2 && y % 4 == 2)),
            Self::SmConfetti => bayer_pct(20),
            Self::LgConfetti => from_pred(|x, y| ((x * 3 + y * 5) % 8) == 0 || ((x * 5 + y * 3) % 8) == 3),
            Self::HorzBrick => from_pred(|x, y| {
                if y % 4 == 0 {
                    true
                } else if y % 8 < 4 {
                    x % 8 == 0
                } else {
                    x % 8 == 4
                }
            }),
            Self::DiagBrick => from_pred(|x, y| (x + y) % 4 == 0 || (x + y) % 8 == 3),
            Self::SolidDmnd => from_pred(|x, y| {
                let t = (x - 4).abs() + (y - 4).abs();
                t <= 4
            }),
            Self::OpenDmnd => from_pred(|x, y| {
                let t = (x - 3).abs() + (y - 3).abs();
                t == 3 || t == 4
            }),
            Self::DotDmnd => from_pred(|x, y| (x + y) % 4 == 0 && (x % 2 == 0)),
            Self::Plaid => from_pred(|x, y| y % 4 < 2 || x % 4 < 2 || ((x / 2) + (y / 2)) % 2 == 0),
            Self::Sphere => from_pred(|x, y| {
                let dx = x as i32 - 3;
                let dy = y as i32 - 3;
                dx * dx + dy * dy <= 10
            }),
            Self::Weave => from_pred(|x, y| {
                let a = (x + y) % 8 == 0 || (x - y).rem_euclid(8) == 0;
                let b = (x % 4 == 0 && y % 2 == 0) || (y % 4 == 0 && x % 2 == 0);
                a || b
            }),
            Self::Divot => from_pred(|x, y| (x + 2 * y) % 8 == 0 || (2 * x + y) % 8 == 4),
            Self::Shingle => from_pred(|x, y| y % 4 == 0 || (x + y / 4) % 8 == 0),
            Self::Wave => from_pred(|x, y| {
                let w = match x % 8 {
                    0 | 4 => 0,
                    1 | 3 => 1,
                    2 => 2,
                    5 | 7 => -1,
                    _ => -2,
                };
                (y - w).rem_euclid(8) == 0
            }),
            Self::Trellis => from_pred(|x, y| (x + y) % 2 == 0 || (x % 2 == 0 && y % 2 == 0)),
            Self::ZigZag => from_pred(|x, y| {
                let row = y % 4;
                if row == 0 {
                    true
                } else if row == 1 {
                    x % 8 == 7 || x % 8 == 0
                } else if row == 2 {
                    true
                } else {
                    x % 8 == 3 || x % 8 == 4
                }
            }),
        }
    }

    pub(super) fn is_foreground(self, x: i32, y: i32, dpi: f64) -> bool {
        let scale = (dpi / 96.0).max(1e-9);
        let col = ((x as f64 + 0.5) / scale).floor() as i32;
        let row = ((y as f64 + 0.5) / scale).floor() as i32;
        let bits = self.bits();
        let cell_x = col.rem_euclid(8) as usize;
        let cell_y = row.rem_euclid(8) as usize;
        (bits[cell_y] & (0x80 >> cell_x)) != 0
    }
}

fn from_pred(pred: impl Fn(i32, i32) -> bool) -> [u8; 8] {
    let mut bits = [0u8; 8];
    for y in 0..8 {
        for x in 0..8 {
            if pred(x, y) {
                bits[y as usize] |= 0x80 >> x;
            }
        }
    }
    bits
}

/// Ordered 8×8 Bayer dither for `pctN` presets (close-enough density, not a lock).
fn bayer_pct(pct: u8) -> [u8; 8] {
    const BAYER: [[u8; 8]; 8] = [
        [0, 32, 8, 40, 2, 34, 10, 42],
        [48, 16, 56, 24, 50, 18, 58, 26],
        [12, 44, 4, 36, 14, 46, 6, 38],
        [60, 28, 52, 20, 62, 30, 54, 22],
        [3, 35, 11, 43, 1, 33, 9, 41],
        [51, 19, 59, 27, 49, 17, 57, 25],
        [15, 47, 7, 39, 13, 45, 5, 37],
        [63, 31, 55, 23, 61, 29, 53, 21],
    ];
    let threshold = ((pct as u16 * 64 + 50) / 100).min(64) as u8;
    from_pred(|x, y| BAYER[y as usize][x as usize] < threshold)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PatternFill {
    pub(super) prst: PatternPreset,
    pub(super) fg: [u8; 3],
    pub(super) bg: [u8; 3],
}

impl PatternFill {
    pub(super) fn color_at(&self, x: i32, y: i32, dpi: f64) -> [u8; 3] {
        if self.prst.is_foreground(x, y, dpi) {
            self.fg
        } else {
            self.bg
        }
    }
}

/// Picture fill. `embed` is the chart-part relationship id (`r:embed`).
/// `src_rect` / `fill_rect` are l/t/r/b in 1000ths of a percent (may be
/// empty/zero). `tile` is present for a tiled fill; otherwise the blip is
/// stretched. `dpi` of 0 means source pixels map 1:1 at 100% tile scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlipFill {
    pub(super) embed: String,
    pub(super) src_rect: Option<[i64; 4]>,
    pub(super) fill_rect: Option<[i64; 4]>,
    pub(super) tile: Option<BlipTile>,
    pub(super) dpi: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlipTile {
    pub(super) tx: i64,
    pub(super) ty: i64,
    pub(super) sx: i64,
    pub(super) sy: i64,
    pub(super) flip: BlipFlip,
    pub(super) algn: BlipAlign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlipFlip {
    None,
    X,
    Y,
    Xy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlipAlign {
    Tl,
    T,
    Tr,
    L,
    Ctr,
    R,
    Bl,
    B,
    Br,
}

impl BlipFlip {
    pub(super) fn parse(name: Option<&str>) -> Self {
        match name {
            Some("x") => Self::X,
            Some("y") => Self::Y,
            Some("xy") => Self::Xy,
            _ => Self::None,
        }
    }
}

impl BlipAlign {
    pub(super) fn parse(name: Option<&str>) -> Self {
        match name {
            Some("t") => Self::T,
            Some("tr") => Self::Tr,
            Some("l") => Self::L,
            Some("ctr") => Self::Ctr,
            Some("r") => Self::R,
            Some("bl") => Self::Bl,
            Some("b") => Self::B,
            Some("br") => Self::Br,
            _ => Self::Tl,
        }
    }

    pub(super) fn origin_frac(self) -> (f64, f64) {
        match self {
            Self::Tl => (0.0, 0.0),
            Self::T => (0.5, 0.0),
            Self::Tr => (1.0, 0.0),
            Self::L => (0.0, 0.5),
            Self::Ctr => (0.5, 0.5),
            Self::R => (1.0, 0.5),
            Self::Bl => (0.0, 1.0),
            Self::B => (0.5, 1.0),
            Self::Br => (1.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatternPreset;

    #[test]
    fn lt_horz_is_foreground_on_rows_zero_and_four_at_96_dpi() {
        let prst = PatternPreset::LtHorz;
        for x in 0..16 {
            assert!(prst.is_foreground(x, 0, 96.0), "row 0 x={x}");
            assert!(!prst.is_foreground(x, 1, 96.0), "row 1 x={x}");
            assert!(!prst.is_foreground(x, 2, 96.0), "row 2 x={x}");
            assert!(!prst.is_foreground(x, 3, 96.0), "row 3 x={x}");
            assert!(prst.is_foreground(x, 4, 96.0), "row 4 x={x}");
            assert!(prst.is_foreground(x, 8, 96.0), "row 8 repeats row 0");
        }
    }

    #[test]
    fn pct10_is_sparser_than_lt_horz_at_96_dpi() {
        let mut fg = 0;
        for y in 0..8 {
            for x in 0..8 {
                if PatternPreset::Pct(10).is_foreground(x, y, 96.0) {
                    fg += 1;
                }
            }
        }
        assert!(fg > 0 && fg < 16, "pct10 should be a light dither, got {fg}/64");
    }
}
