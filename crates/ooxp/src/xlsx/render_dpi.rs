//! 库导出可选栅格化 DPI。省略时为 96，与现有出图像素一致。
//! 只影响本次 PNG，不改源包，也不改系统 DPI。
//! 坐标、字体、描边和间距按该 DPI 原生计算，不是先画 96 再拉伸位图。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

/// 与历史出图合同一致的缺省分辨率。
pub const DEFAULT_DPI: f64 = 96.0;
/// 允许的最小整数 DPI（含）。
pub const MIN_DPI: i64 = 24;
/// 允许的最大整数 DPI（含）。再高也仍受 1..=4096 像素边长与面积上限约束。
pub const MAX_DPI: i64 = 600;
const EMU_PER_INCH: f64 = 914_400.0;

/// 已校验的导出 DPI。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderDpi {
    value: f64,
}

impl Default for RenderDpi {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl RenderDpi {
    pub const DEFAULT: Self = Self { value: DEFAULT_DPI };

    pub const fn get(self) -> f64 {
        self.value
    }

    pub const fn is_default(self) -> bool {
        self.value == DEFAULT_DPI
    }

    /// 从导出请求读取可选 `dpi`。省略视为 96。
    pub fn from_request(obj: &serde_json::Map<String, Value>) -> Result<Self, Error> {
        match obj.get("dpi") {
            None => Ok(Self::DEFAULT),
            Some(value) => parse_dpi(value),
        }
    }

    pub fn scale_px(self, px96: i32) -> i32 {
        if self.is_default() {
            px96
        } else {
            (f64::from(px96) * self.value / DEFAULT_DPI).round() as i32
        }
    }

    pub fn scale_f(self, px96: f64) -> f64 {
        if self.is_default() {
            px96
        } else {
            px96 * self.value / DEFAULT_DPI
        }
    }

    pub fn emu_to_px_round(self, emu: i64) -> i64 {
        (emu as f64 * self.value / EMU_PER_INCH).round() as i64
    }

    pub fn emu_to_px_f(self, emu: f64) -> f64 {
        emu * self.value / EMU_PER_INCH
    }

    pub fn pt_to_px_font(self, pt: f64) -> f64 {
        pt * self.value / 72.0
    }

    /// Excel 自定义行高：`floor(round(pt*12700) * dpi / 914400)`。96 dpi 时即 `floor(emu/9525)`。
    pub fn excel_row_px(self, pt: f64) -> i32 {
        let emu = (pt * 12_700.0).round();
        (emu * self.value / EMU_PER_INCH).floor() as i32
    }

    /// 迷你图宿主行高：与 `layout::row_height_px` 相同的四舍五入。
    pub fn layout_emu_to_px(self, emu: i64) -> u32 {
        let px = (emu as f64 * self.value / EMU_PER_INCH).round();
        if !px.is_finite() || px < 1.0 {
            0
        } else if px > f64::from(u32::MAX) {
            u32::MAX
        } else {
            px as u32
        }
    }
}

fn parse_dpi(value: &Value) -> Result<RenderDpi, Error> {
    let Some(number) = value.as_f64() else {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dpi".into(),
                expected: TypeName::Integer,
            },
            "dpi",
        ));
    };
    if !number.is_finite() || (number - number.round()).abs() > 0.0 {
        return Err(Error::with_path(
            ErrorKind::WrongType {
                field: "dpi".into(),
                expected: TypeName::Integer,
            },
            "dpi",
        ));
    }
    let dpi = number as i64;
    if !(MIN_DPI..=MAX_DPI).contains(&dpi) {
        return Err(Error::with_path(
            ErrorKind::OutOfRange {
                field: "dpi".into(),
                min: MIN_DPI,
                max: MAX_DPI,
            },
            "dpi",
        ));
    }
    Ok(RenderDpi { value: dpi as f64 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    fn from_json(value: Value) -> Result<RenderDpi, Error> {
        let mut obj = Map::new();
        obj.insert("dpi".into(), value);
        RenderDpi::from_request(&obj)
    }

    #[test]
    fn omitted_is_96() {
        assert_eq!(
            RenderDpi::from_request(&Map::new()).unwrap(),
            RenderDpi::DEFAULT
        );
        assert_eq!(RenderDpi::DEFAULT.get(), 96.0);
        assert_eq!(RenderDpi::DEFAULT.scale_px(64), 64);
        assert_eq!(RenderDpi::DEFAULT.emu_to_px_round(5_486_400), 576);
        assert_eq!(RenderDpi::DEFAULT.excel_row_px(15.0), 20);
        assert_eq!(RenderDpi::DEFAULT.excel_row_px(14.25), 19);
    }

    #[test]
    fn integer_106_scales_emu_and_px() {
        let dpi = from_json(json!(106)).unwrap();
        assert_eq!(dpi.get(), 106.0);
        assert_eq!(dpi.emu_to_px_round(5_486_400), 636);
        assert_eq!(dpi.emu_to_px_round(3_200_400), 371);
        assert_eq!(dpi.scale_px(64), 71);
        assert_eq!(dpi.excel_row_px(15.0), 22);
        assert_eq!(from_json(json!(96.0)).unwrap(), RenderDpi::DEFAULT);
    }

    #[test]
    fn illegal_values_are_rejected() {
        for value in [
            json!(0),
            json!(-1),
            json!(23),
            json!(601),
            json!(24.5),
            json!("96"),
            json!(true),
            json!(null),
            json!([]),
            json!({}),
        ] {
            assert!(from_json(value.clone()).is_err(), "{value}");
        }
        assert_eq!(from_json(json!(24)).unwrap().get(), 24.0);
        assert_eq!(from_json(json!(600)).unwrap().get(), 600.0);
    }
}
