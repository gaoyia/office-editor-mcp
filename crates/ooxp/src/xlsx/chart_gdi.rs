//! Windows 上用 GDI 画图表文字。图里没写字体时用「等线」，和 Excel 16.0 的缺省一致。

#[cfg(windows)]
mod win {

    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct GlyphMetrics {
        black_box_x: u32,
        black_box_y: u32,
        glyph_origin_x: i32,
        glyph_origin_y: i32,
        cell_inc_x: i16,
        cell_inc_y: i16,
    }

    #[repr(C)]
    struct Fixed {
        fract: u16,
        value: i16,
    }

    #[repr(C)]
    struct TextMetric {
        height: i32,
        ascent: i32,
        descent: i32,
        internal_leading: i32,
        external_leading: i32,
        ave_char_width: i32,
        max_char_width: i32,
        weight: i32,
        overhang: i32,
        digitized_aspect_x: i32,
        digitized_aspect_y: i32,
        first_char: u16,
        last_char: u16,
        default_char: u16,
        break_char: u16,
        italic: u8,
        underlined: u8,
        struck_out: u8,
        pitch_and_family: u8,
        charset: u8,
        _pad: [u8; 3],
    }

    #[repr(C)]
    struct Mat2 {
        e_m11: Fixed,
        e_m12: Fixed,
        e_m21: Fixed,
        e_m22: Fixed,
    }

    const GGO_GRAY8_BITMAP: u32 = 6;
    const GDI_ERROR: u32 = 0xFFFF_FFFF;
    const ANTIALIASED_QUALITY: u32 = 4;
    const FW_NORMAL: i32 = 400;
    const FW_BOLD: i32 = 700;
    const FACES: [&str; 5] = ["等线", "DengXian", "微软雅黑", "Microsoft YaHei", "Calibri"];

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
        fn DeleteDC(hdc: *mut c_void) -> i32;
        fn GetTextFaceW(hdc: *mut c_void, count: i32, face: *mut u16) -> i32;
        fn GetTextMetricsW(hdc: *mut c_void, metric: *mut TextMetric) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike_out: u32,
            char_set: u32,
            out_precision: u32,
            clip_precision: u32,
            quality: u32,
            pitch_and_family: u32,
            face: *const u16,
        ) -> *mut c_void;
        fn SelectObject(hdc: *mut c_void, obj: *mut c_void) -> *mut c_void;
        fn DeleteObject(obj: *mut c_void) -> i32;
        fn GetGlyphOutlineW(
            hdc: *mut c_void,
            u_char: u32,
            format: u32,
            metrics: *mut GlyphMetrics,
            buf_size: u32,
            buffer: *mut c_void,
            mat: *const Mat2,
        ) -> u32;
    }

    pub struct Face {
        dc: *mut c_void,
        font: *mut c_void,
    }

    impl Drop for Face {
        fn drop(&mut self) {
            unsafe {
                if !self.font.is_null() {
                    DeleteObject(self.font);
                }
                if !self.dc.is_null() {
                    DeleteDC(self.dc);
                }
            }
        }
    }

    impl Face {
        /// 按顺序找本机有的字体。图里没写字体时，Excel 16.0 用「等线」。
        pub fn open(px: f32, bold: bool) -> Option<Self> {
            for name in FACES {
                let Some(face) = Self::new(px, bold, name) else {
                    continue;
                };
                if face.text_face() == name || name == "Calibri" {
                    return Some(face);
                }
            }
            None
        }

        /// `px` 是 96 dpi 下的像素字号。负高度让 GDI 按字符高度而不是单元格高度来建。
        fn new(px: f32, bold: bool, name: &str) -> Option<Self> {
            let height = -(px.round() as i32);
            if height == 0 {
                return None;
            }
            let mut face: Vec<u16> = name.encode_utf16().collect();
            face.push(0);
            unsafe {
                let dc = CreateCompatibleDC(std::ptr::null_mut());
                if dc.is_null() {
                    return None;
                }
                let font = CreateFontW(
                    height,
                    0,
                    0,
                    0,
                    if bold { FW_BOLD } else { FW_NORMAL },
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    ANTIALIASED_QUALITY,
                    0,
                    face.as_ptr(),
                );
                if font.is_null() {
                    DeleteDC(dc);
                    return None;
                }
                SelectObject(dc, font);
                Some(Self { dc, font })
            }
        }

        fn text_face(&self) -> String {
            let mut buf = vec![0u16; 64];
            let n = unsafe { GetTextFaceW(self.dc, buf.len() as i32, buf.as_mut_ptr()) };
            if n <= 1 {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..n as usize - 1])
        }

        pub fn width(&self, text: &str) -> f64 {
            text.chars().map(|ch| self.advance(ch)).sum()
        }

        fn advance(&self, ch: char) -> f64 {
            let Some(metrics) = self.metrics(ch) else {
                return 0.0;
            };
            f64::from(metrics.cell_inc_x)
        }

        fn metrics(&self, ch: char) -> Option<GlyphMetrics> {
            let mut metrics = GlyphMetrics {
                black_box_x: 0,
                black_box_y: 0,
                glyph_origin_x: 0,
                glyph_origin_y: 0,
                cell_inc_x: 0,
                cell_inc_y: 0,
            };
            let mat = identity();
            let size = unsafe {
                GetGlyphOutlineW(
                    self.dc,
                    ch as u32,
                    GGO_GRAY8_BITMAP,
                    &mut metrics,
                    0,
                    std::ptr::null_mut(),
                    &mat,
                )
            };
            if size == GDI_ERROR {
                return None;
            }
            Some(metrics)
        }

        /// `top` 是字身顶部，和 fontdue 那条路径同一约定。
        pub fn draw(
            &self,
            text: &str,
            x: f64,
            top: f64,
            color: [u8; 3],
            mut blend: impl FnMut(i32, i32, [u8; 3], u8),
        ) {
            let line = self.line_height();
            let mut cursor = x;
            for ch in text.chars() {
                let Some(metrics) = self.metrics(ch) else {
                    continue;
                };
                let mat = identity();
                let size = unsafe {
                    GetGlyphOutlineW(
                        self.dc,
                        ch as u32,
                        GGO_GRAY8_BITMAP,
                        &mut GlyphMetrics {
                            black_box_x: 0,
                            black_box_y: 0,
                            glyph_origin_x: 0,
                            glyph_origin_y: 0,
                            cell_inc_x: 0,
                            cell_inc_y: 0,
                        },
                        0,
                        std::ptr::null_mut(),
                        &mat,
                    )
                };
                if size == GDI_ERROR || size == 0 {
                    cursor += f64::from(metrics.cell_inc_x);
                    continue;
                }
                let mut buf = vec![0u8; size as usize];
                let mut got = metrics;
                let wrote = unsafe {
                    GetGlyphOutlineW(
                        self.dc,
                        ch as u32,
                        GGO_GRAY8_BITMAP,
                        &mut got,
                        size,
                        buf.as_mut_ptr().cast(),
                        &mat,
                    )
                };
                if wrote == GDI_ERROR {
                    cursor += f64::from(metrics.cell_inc_x);
                    continue;
                }
                let pitch = (got.black_box_x as usize + 3) & !3;
                let baseline = top + line;
                let left = cursor + f64::from(got.glyph_origin_x);
                let glyph_top = baseline - f64::from(got.glyph_origin_y);
                for row in 0..got.black_box_y as usize {
                    for col in 0..got.black_box_x as usize {
                        let sample = buf.get(row * pitch + col).copied().unwrap_or(0);
                        if sample == 0 {
                            continue;
                        }
                        let cov = ((u16::from(sample) * 255) / 64).min(255) as u8;
                        blend(
                            left.round() as i32 + col as i32,
                            glyph_top.round() as i32 + row as i32,
                            color,
                            cov,
                        );
                    }
                }
                cursor += f64::from(got.cell_inc_x);
            }
        }

        fn text_metric(&self) -> Option<TextMetric> {
            let mut metric = TextMetric {
                height: 0,
                ascent: 0,
                descent: 0,
                internal_leading: 0,
                external_leading: 0,
                ave_char_width: 0,
                max_char_width: 0,
                weight: 0,
                overhang: 0,
                digitized_aspect_x: 0,
                digitized_aspect_y: 0,
                first_char: 0,
                last_char: 0,
                default_char: 0,
                break_char: 0,
                italic: 0,
                underlined: 0,
                struck_out: 0,
                pitch_and_family: 0,
                charset: 0,
                _pad: [0; 3],
            };
            let ok = unsafe { GetTextMetricsW(self.dc, &mut metric) };
            (ok != 0 && metric.ascent > 0).then_some(metric)
        }

        /// 基线到字顶的距离。画字时用它，避免汉字高出数字「0」的黑框。
        pub fn line_height(&self) -> f64 {
            self.text_metric()
                .map(|m| f64::from(m.ascent))
                .unwrap_or(12.0)
        }

        /// 一整行的高度，含内部行距。标题下面的绘图区从这里往下排。
        pub fn em_box(&self) -> f64 {
            self.text_metric()
                .map(|m| f64::from(m.height))
                .unwrap_or_else(|| self.line_height())
        }
    }

    fn identity() -> Mat2 {
        Mat2 {
            e_m11: Fixed { fract: 0, value: 1 },
            e_m12: Fixed { fract: 0, value: 0 },
            e_m21: Fixed { fract: 0, value: 0 },
            e_m22: Fixed { fract: 0, value: 1 },
        }
    }

    #[test]
    fn chart_text_uses_dengxian() {
        let face = Face::open(13.0, true).expect("本机应有图表字体");
        let name = face.text_face();
        assert!(
            name == "等线" || name.eq_ignore_ascii_case("DengXian"),
            "实际字体是 {name}"
        );
        assert_eq!(face.width("1400"), 28.0);
    }
}

#[cfg(windows)]
pub use win::Face;

#[cfg(not(windows))]
pub struct Face;

#[cfg(not(windows))]
impl Face {
    pub fn open(_: f32, _: bool) -> Option<Self> {
        None
    }

    pub fn line_height(&self) -> f64 {
        0.0
    }

    pub fn em_box(&self) -> f64 {
        0.0
    }

    pub fn width(&self, _: &str) -> f64 {
        0.0
    }

    pub fn draw(&self, _: &str, _: f64, _: f64, _: [u8; 3], _: impl FnMut(i32, i32, [u8; 3], u8)) {}
}
