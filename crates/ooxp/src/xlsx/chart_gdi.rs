//! Windows 上用 GDI+ 的 AntiAliasGridFit 画图表文字。图里没写字体时用「等线」。

#[cfg(windows)]
mod win {

    use std::ffi::c_void;
    use std::sync::OnceLock;

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
        name: String,
        bold: bool,
        italic: bool,
        pt: f32,
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
            let pt = (f64::from(px) * 72.0 / 96.0) as f32;
            for name in FACES {
                let Some(mut face) = Self::new(px, bold, false, name, pt) else {
                    continue;
                };
                let actual = face.text_face();
                if actual == name || name == "Calibri" {
                    face.name = actual;
                    return Some(face);
                }
            }
            None
        }

        /// 按形状上写的字体名打开。找不到就退回图表那套默认字体。
        pub fn open_named(px: f32, bold: bool, name: &str) -> Option<Self> {
            let pt = (f64::from(px) * 72.0 / 96.0) as f32;
            if !name.is_empty() && !name.starts_with('+') {
                if let Some(mut face) = Self::new(px, bold, false, name, pt) {
                    face.name = face.text_face();
                    if !face.name.is_empty() {
                        return Some(face);
                    }
                }
            }
            Self::open(px, bold)
        }

        /// 单元格文字。斜体只走这一条，图表字仍不加斜。
        pub fn open_cell(px: f32, bold: bool, italic: bool, name: &str) -> Option<Self> {
            let pt = (f64::from(px) * 72.0 / 96.0) as f32;
            if !name.is_empty() && !name.starts_with('+') {
                if let Some(mut face) = Self::new(px, bold, italic, name, pt) {
                    face.name = face.text_face();
                    if !face.name.is_empty() {
                        return Some(face);
                    }
                }
            }
            Self::new(px, bold, italic, "Calibri", pt).map(|mut face| {
                face.name = face.text_face();
                face
            })
        }

        /// `px` 是 96 dpi 下的像素字号。负高度让 GDI 按字符高度而不是单元格高度来建。
        fn new(px: f32, bold: bool, italic: bool, name: &str, pt: f32) -> Option<Self> {
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
                    u32::from(italic),
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
                Some(Self {
                    dc,
                    font,
                    name: name.to_string(),
                    bold,
                    italic,
                    pt,
                })
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

        /// GDI+ 默认排版宽度。比字形前进宽度多出防裁切的边，幻灯片图例用它留空。
        pub fn layout_width(&self, text: &str) -> f64 {
            self.measure_layout(text)
                .unwrap_or_else(|| self.width(text))
        }

        fn measure_layout(&self, text: &str) -> Option<f64> {
            if text.is_empty() || gdiplus_token().is_none() {
                return None;
            }
            let chars: Vec<u16> = text.encode_utf16().collect();
            let mut name: Vec<u16> = self.name.encode_utf16().collect();
            name.push(0);
            unsafe {
                let mut pixels = [255u8; 4];
                let mut bitmap = std::ptr::null_mut();
                if GdipCreateBitmapFromScan0(
                    1,
                    1,
                    4,
                    PIXEL_FORMAT_32BPP_ARGB,
                    pixels.as_mut_ptr(),
                    &mut bitmap,
                ) != 0
                    || bitmap.is_null()
                {
                    return None;
                }
                GdipBitmapSetResolution(bitmap, 96.0, 96.0);
                let mut graphics = std::ptr::null_mut();
                if GdipGetImageGraphicsContext(bitmap, &mut graphics) != 0 || graphics.is_null() {
                    GdipDisposeImage(bitmap);
                    return None;
                }
                GdipSetPageUnit(graphics, UNIT_PIXEL);
                let mut family = std::ptr::null_mut();
                if GdipCreateFontFamilyFromName(name.as_ptr(), std::ptr::null_mut(), &mut family)
                    != 0
                    || family.is_null()
                {
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return None;
                }
                let mut font = std::ptr::null_mut();
                let style = self.gdip_style();
                if GdipCreateFont(family, self.pt, style, UNIT_POINT, &mut font) != 0
                    || font.is_null()
                {
                    GdipDeleteFontFamily(family);
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return None;
                }
                let layout = RectF {
                    x: 0.0,
                    y: 0.0,
                    w: 10_000.0,
                    h: 1_000.0,
                };
                let mut bounds = RectF {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                };
                let status = GdipMeasureString(
                    graphics,
                    chars.as_ptr(),
                    chars.len() as i32,
                    font,
                    &layout,
                    std::ptr::null_mut(),
                    &mut bounds,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                GdipDeleteFont(font);
                GdipDeleteFontFamily(family);
                GdipDeleteGraphics(graphics);
                GdipDisposeImage(bitmap);
                (status == 0 && bounds.w > 0.0).then_some(f64::from(bounds.w))
            }
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
            if text.is_empty() {
                return;
            }
            if self.draw_gdiplus(text, x, top, color, &mut blend) {
                return;
            }
            self.draw_gray8(text, x, top, color, &mut blend);
        }

        fn draw_gray8(
            &self,
            text: &str,
            x: f64,
            top: f64,
            color: [u8; 3],
            blend: &mut impl FnMut(i32, i32, [u8; 3], u8),
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

        /// 把整段字画进一块白底，再按灰度还原覆盖率。字宽仍用 GDI 的前进宽度，轴线不跟着挪。
        fn draw_gdiplus(
            &self,
            text: &str,
            x: f64,
            top: f64,
            color: [u8; 3],
            blend: &mut impl FnMut(i32, i32, [u8; 3], u8),
        ) -> bool {
            if gdiplus_token().is_none() {
                return false;
            }
            let pad = 4i32;
            let bw = (self.width(text).ceil() as i32 + pad * 2 + 8).max(1);
            let bh = (self.em_box().ceil() as i32 + pad * 2 + 8).max(1);
            let mut pixels = vec![0xFFFF_FFFFu32; (bw as usize) * (bh as usize)];
            let chars: Vec<u16> = text.encode_utf16().collect();
            let mut name: Vec<u16> = self.name.encode_utf16().collect();
            name.push(0);
            let drawn = unsafe {
                let mut bitmap = std::ptr::null_mut();
                if GdipCreateBitmapFromScan0(
                    bw,
                    bh,
                    bw * 4,
                    PIXEL_FORMAT_32BPP_ARGB,
                    pixels.as_mut_ptr().cast(),
                    &mut bitmap,
                ) != 0
                    || bitmap.is_null()
                {
                    return false;
                }
                GdipBitmapSetResolution(bitmap, 96.0, 96.0);
                let mut graphics = std::ptr::null_mut();
                if GdipGetImageGraphicsContext(bitmap, &mut graphics) != 0 || graphics.is_null() {
                    GdipDisposeImage(bitmap);
                    return false;
                }
                GdipSetPageUnit(graphics, UNIT_PIXEL);
                GdipSetTextRenderingHint(graphics, TEXT_ANTIALIAS_GRIDFIT);
                let mut family = std::ptr::null_mut();
                if GdipCreateFontFamilyFromName(name.as_ptr(), std::ptr::null_mut(), &mut family)
                    != 0
                    || family.is_null()
                {
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return false;
                }
                let mut font = std::ptr::null_mut();
                let style = self.gdip_style();
                if GdipCreateFont(family, self.pt, style, UNIT_POINT, &mut font) != 0
                    || font.is_null()
                {
                    GdipDeleteFontFamily(family);
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return false;
                }
                let mut brush = std::ptr::null_mut();
                if GdipCreateSolidFill(0xFF00_0000, &mut brush) != 0 || brush.is_null() {
                    GdipDeleteFont(font);
                    GdipDeleteFontFamily(family);
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return false;
                }
                let mut generic = std::ptr::null_mut();
                let mut format = std::ptr::null_mut();
                if GdipStringFormatGetGenericTypographic(&mut generic) != 0
                    || GdipCloneStringFormat(generic, &mut format) != 0
                    || format.is_null()
                {
                    GdipDeleteBrush(brush);
                    GdipDeleteFont(font);
                    GdipDeleteFontFamily(family);
                    GdipDeleteGraphics(graphics);
                    GdipDisposeImage(bitmap);
                    return false;
                }
                GdipSetStringFormatFlags(format, STRING_FORMAT_TYPO);
                let rect = RectF {
                    x: pad as f32,
                    y: pad as f32,
                    w: (bw - pad) as f32,
                    h: (bh - pad) as f32,
                };
                let status = GdipDrawString(
                    graphics,
                    chars.as_ptr(),
                    chars.len() as i32,
                    font,
                    &rect,
                    format,
                    brush,
                );
                GdipDeleteStringFormat(format);
                GdipDeleteBrush(brush);
                GdipDeleteFont(font);
                GdipDeleteFontFamily(family);
                GdipDeleteGraphics(graphics);
                GdipDisposeImage(bitmap);
                status == 0
            };
            if !drawn {
                return false;
            }
            let ox = x.round() as i32 - pad;
            let oy = top.round() as i32 - pad;
            for py in 0..bh {
                for px in 0..bw {
                    let row = pixels[py as usize * bw as usize + px as usize];
                    let red = ((row >> 16) & 0xff) as u8;
                    let green = ((row >> 8) & 0xff) as u8;
                    let blue = (row & 0xff) as u8;
                    if red == 255 && green == 255 && blue == 255 {
                        continue;
                    }
                    let cov = (255 - u16::from(red))
                        .max(255 - u16::from(green))
                        .max(255 - u16::from(blue))
                        .min(255) as u8;
                    if cov == 0 {
                        continue;
                    }
                    blend(ox + px, oy + py, color, cov);
                }
            }
            true
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

        fn gdip_style(&self) -> i32 {
            let mut style = 0;
            if self.bold {
                style |= FONT_STYLE_BOLD;
            }
            if self.italic {
                style |= FONT_STYLE_ITALIC;
            }
            style
        }

        /// 单元格左缘留白。Excel 取平均字宽的一半，20 磅加粗微软雅黑是 7 像素。
        pub fn cell_pad(&self) -> f64 {
            self.text_metric()
                .map(|m| f64::from(m.ave_char_width / 2).max(1.0))
                .unwrap_or(2.0)
        }

        /// 按 GDI 前进宽度逐字画。整段 GDI+ 会把有的字压窄 1 像素，标题就会越排越偏。
        /// 像素仍用 GDI+。GDI 灰阶能把汉字收进字框，但整表字形更灰，差会多出约六千像素。
        pub fn draw_cells(
            &self,
            text: &str,
            x: f64,
            top: f64,
            color: [u8; 3],
            mut blend: impl FnMut(i32, i32, [u8; 3], u8),
        ) {
            let mut cursor = x;
            for ch in text.chars() {
                let advance = self.advance(ch);
                let piece = ch.to_string();
                self.draw(&piece, cursor, top, color, &mut blend);
                cursor += advance;
            }
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

    const TEXT_ANTIALIAS_GRIDFIT: i32 = 3;
    const UNIT_PIXEL: i32 = 2;
    const UNIT_POINT: i32 = 3;
    const FONT_STYLE_BOLD: i32 = 1;
    const FONT_STYLE_ITALIC: i32 = 2;
    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
    /// GenericTypographic 再加不换行、保留行尾空格、不裁切。
    const STRING_FORMAT_TYPO: i32 = 0x7804;

    #[repr(C)]
    struct GdiplusStartupInput {
        version: u32,
        callback: *mut c_void,
        suppress_background: i32,
        suppress_codecs: i32,
    }

    #[repr(C)]
    struct RectF {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    }

    #[link(name = "gdiplus")]
    extern "system" {
        fn GdiplusStartup(
            token: *mut usize,
            input: *const GdiplusStartupInput,
            output: *mut c_void,
        ) -> i32;
        fn GdipCreateBitmapFromScan0(
            width: i32,
            height: i32,
            stride: i32,
            format: i32,
            scan0: *mut u8,
            bitmap: *mut *mut c_void,
        ) -> i32;
        fn GdipBitmapSetResolution(bitmap: *mut c_void, xdpi: f32, ydpi: f32) -> i32;
        fn GdipGetImageGraphicsContext(image: *mut c_void, graphics: *mut *mut c_void) -> i32;
        fn GdipSetPageUnit(graphics: *mut c_void, unit: i32) -> i32;
        fn GdipSetTextRenderingHint(graphics: *mut c_void, hint: i32) -> i32;
        fn GdipCreateFontFamilyFromName(
            name: *const u16,
            collection: *mut c_void,
            family: *mut *mut c_void,
        ) -> i32;
        fn GdipCreateFont(
            family: *mut c_void,
            size: f32,
            style: i32,
            unit: i32,
            font: *mut *mut c_void,
        ) -> i32;
        fn GdipCreateSolidFill(color: u32, brush: *mut *mut c_void) -> i32;
        fn GdipStringFormatGetGenericTypographic(format: *mut *mut c_void) -> i32;
        fn GdipCloneStringFormat(src: *mut c_void, clone: *mut *mut c_void) -> i32;
        fn GdipSetStringFormatFlags(format: *mut c_void, flags: i32) -> i32;
        fn GdipMeasureString(
            graphics: *mut c_void,
            text: *const u16,
            length: i32,
            font: *mut c_void,
            layout: *const RectF,
            format: *mut c_void,
            bounds: *mut RectF,
            codepoints: *mut i32,
            lines: *mut i32,
        ) -> i32;
        fn GdipDrawString(
            graphics: *mut c_void,
            text: *const u16,
            length: i32,
            font: *mut c_void,
            layout: *const RectF,
            format: *mut c_void,
            brush: *mut c_void,
        ) -> i32;
        fn GdipDeleteStringFormat(format: *mut c_void) -> i32;
        fn GdipDeleteBrush(brush: *mut c_void) -> i32;
        fn GdipDeleteFont(font: *mut c_void) -> i32;
        fn GdipDeleteFontFamily(family: *mut c_void) -> i32;
        fn GdipDeleteGraphics(graphics: *mut c_void) -> i32;
        fn GdipDisposeImage(image: *mut c_void) -> i32;
        fn GdipSetSmoothingMode(graphics: *mut c_void, mode: i32) -> i32;
        fn GdipSetPixelOffsetMode(graphics: *mut c_void, mode: i32) -> i32;
        fn GdipFillEllipse(
            graphics: *mut c_void,
            brush: *mut c_void,
            x: f32,
            y: f32,
            width: f32,
            height: f32,
        ) -> i32;
        fn GdipCreatePath(mode: i32, path: *mut *mut c_void) -> i32;
        fn GdipAddPathArc(
            path: *mut c_void,
            x: f32,
            y: f32,
            width: f32,
            height: f32,
            start_angle: f32,
            sweep: f32,
        ) -> i32;
        fn GdipClosePathFigure(path: *mut c_void) -> i32;
        fn GdipFillPath(graphics: *mut c_void, brush: *mut c_void, path: *mut c_void) -> i32;
        fn GdipDeletePath(path: *mut c_void) -> i32;
        fn GdipFillRectangle(
            graphics: *mut c_void,
            brush: *mut c_void,
            x: f32,
            y: f32,
            width: f32,
            height: f32,
        ) -> i32;
    }

    const SMOOTHING_ANTIALIAS: i32 = 4;
    const PIXEL_OFFSET_HALF: i32 = 4;

    /// 用 GDI+ 抗锯齿和半像素偏移，填充设备像素矩形里的椭圆。
    /// 回调坐标是页面像素，覆盖率从 0 到 255。
    pub fn fill_device_ellipse(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        mut blend: impl FnMut(i32, i32, u8),
    ) -> bool {
        if w <= 0 || h <= 0 || gdiplus_token().is_none() {
            return false;
        }
        let pad = 2i32;
        let bw = w + pad * 2;
        let bh = h + pad * 2;
        let mut pixels = vec![0xFFFF_FFFFu32; (bw as usize) * (bh as usize)];
        let drawn = unsafe {
            let mut bitmap = std::ptr::null_mut();
            if GdipCreateBitmapFromScan0(
                bw,
                bh,
                bw * 4,
                PIXEL_FORMAT_32BPP_ARGB,
                pixels.as_mut_ptr().cast(),
                &mut bitmap,
            ) != 0
                || bitmap.is_null()
            {
                return false;
            }
            GdipBitmapSetResolution(bitmap, 96.0, 96.0);
            let mut graphics = std::ptr::null_mut();
            if GdipGetImageGraphicsContext(bitmap, &mut graphics) != 0 || graphics.is_null() {
                GdipDisposeImage(bitmap);
                return false;
            }
            GdipSetPageUnit(graphics, UNIT_PIXEL);
            GdipSetSmoothingMode(graphics, SMOOTHING_ANTIALIAS);
            GdipSetPixelOffsetMode(graphics, PIXEL_OFFSET_HALF);
            let mut brush = std::ptr::null_mut();
            if GdipCreateSolidFill(0xFF00_0000, &mut brush) != 0 || brush.is_null() {
                GdipDeleteGraphics(graphics);
                GdipDisposeImage(bitmap);
                return false;
            }
            let status =
                GdipFillEllipse(graphics, brush, pad as f32, pad as f32, w as f32, h as f32);
            GdipDeleteBrush(brush);
            GdipDeleteGraphics(graphics);
            GdipDisposeImage(bitmap);
            status == 0
        };
        if !drawn {
            return false;
        }
        for py in 0..bh {
            for px in 0..bw {
                let row = pixels[py as usize * bw as usize + px as usize];
                let red = ((row >> 16) & 0xff) as u8;
                let green = ((row >> 8) & 0xff) as u8;
                let blue = (row & 0xff) as u8;
                if red == 255 && green == 255 && blue == 255 {
                    continue;
                }
                let cov = (255 - u16::from(red))
                    .max(255 - u16::from(green))
                    .max(255 - u16::from(blue))
                    .min(255) as u8;
                if cov == 0 {
                    continue;
                }
                blend(x - pad + px, y - pad + py, cov);
            }
        }
        true
    }

    /// 用与椭圆相同的 GDI+ 抗锯齿和半像素偏移，填充圆角矩形。半径不超过短边的一半。
    pub fn fill_device_round_rect(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        radius: f64,
        mut blend: impl FnMut(i32, i32, u8),
    ) -> bool {
        if w <= 0 || h <= 0 || gdiplus_token().is_none() {
            return false;
        }
        let pad = 2i32;
        let bw = w + pad * 2;
        let bh = h + pad * 2;
        let mut pixels = vec![0xFFFF_FFFFu32; (bw as usize) * (bh as usize)];
        let max_r = f64::from(w.min(h)) / 2.0;
        let r = radius.clamp(0.0, max_r) as f32;
        let drawn = unsafe {
            let mut bitmap = std::ptr::null_mut();
            if GdipCreateBitmapFromScan0(
                bw,
                bh,
                bw * 4,
                PIXEL_FORMAT_32BPP_ARGB,
                pixels.as_mut_ptr().cast(),
                &mut bitmap,
            ) != 0
                || bitmap.is_null()
            {
                return false;
            }
            GdipBitmapSetResolution(bitmap, 96.0, 96.0);
            let mut graphics = std::ptr::null_mut();
            if GdipGetImageGraphicsContext(bitmap, &mut graphics) != 0 || graphics.is_null() {
                GdipDisposeImage(bitmap);
                return false;
            }
            GdipSetPageUnit(graphics, UNIT_PIXEL);
            GdipSetSmoothingMode(graphics, SMOOTHING_ANTIALIAS);
            GdipSetPixelOffsetMode(graphics, PIXEL_OFFSET_HALF);
            let mut brush = std::ptr::null_mut();
            if GdipCreateSolidFill(0xFF00_0000, &mut brush) != 0 || brush.is_null() {
                GdipDeleteGraphics(graphics);
                GdipDisposeImage(bitmap);
                return false;
            }
            let fx = pad as f32;
            let fy = pad as f32;
            let fw = w as f32;
            let fh = h as f32;
            let status = if r <= 0.0 {
                GdipFillRectangle(graphics, brush, fx, fy, fw, fh)
            } else {
                let d = r * 2.0;
                let mut path = std::ptr::null_mut();
                let ok = GdipCreatePath(0, &mut path) == 0
                    && !path.is_null()
                    && GdipAddPathArc(path, fx, fy, d, d, 180.0, 90.0) == 0
                    && GdipAddPathArc(path, fx + fw - d, fy, d, d, 270.0, 90.0) == 0
                    && GdipAddPathArc(path, fx + fw - d, fy + fh - d, d, d, 0.0, 90.0) == 0
                    && GdipAddPathArc(path, fx, fy + fh - d, d, d, 90.0, 90.0) == 0
                    && GdipClosePathFigure(path) == 0
                    && GdipFillPath(graphics, brush, path) == 0;
                if !path.is_null() {
                    GdipDeletePath(path);
                }
                if ok {
                    0
                } else {
                    1
                }
            };
            GdipDeleteBrush(brush);
            GdipDeleteGraphics(graphics);
            GdipDisposeImage(bitmap);
            status == 0
        };
        if !drawn {
            return false;
        }
        for py in 0..bh {
            for px in 0..bw {
                let row = pixels[py as usize * bw as usize + px as usize];
                let red = ((row >> 16) & 0xff) as u8;
                let green = ((row >> 8) & 0xff) as u8;
                let blue = (row & 0xff) as u8;
                if red == 255 && green == 255 && blue == 255 {
                    continue;
                }
                let cov = (255 - u16::from(red))
                    .max(255 - u16::from(green))
                    .max(255 - u16::from(blue))
                    .min(255) as u8;
                if cov == 0 {
                    continue;
                }
                blend(x - pad + px, y - pad + py, cov);
            }
        }
        true
    }

    fn gdiplus_token() -> Option<usize> {
        static TOKEN: OnceLock<Option<usize>> = OnceLock::new();
        *TOKEN.get_or_init(|| unsafe {
            let input = GdiplusStartupInput {
                version: 1,
                callback: std::ptr::null_mut(),
                suppress_background: 0,
                suppress_codecs: 0,
            };
            let mut token = 0usize;
            let status = GdiplusStartup(&mut token, &input, std::ptr::null_mut());
            (status == 0 && token != 0).then_some(token)
        })
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
    fn slide_legend_layout_is_wider_than_the_glyph_advance() {
        let face = Face::open(10.0 * 96.0 / 72.0, false).expect("本机应有图表字体");
        let advance = face.width("2025 年");
        let layout = face.layout_width("2025 年");
        assert!(
            layout > advance + 4.0,
            "排版宽度应比前进宽度多出留白 layout={layout} advance={advance}"
        );
    }

    #[test]
    fn chart_text_prefers_dengxian() {
        let face = Face::open(13.0, true).expect("本机应有图表字体");
        let name = face.text_face();
        if name == "等线" || name.eq_ignore_ascii_case("DengXian") {
            assert_eq!(face.width("1400"), 28.0);
            return;
        }
        // 没有等线时按名单退到雅黑或 Calibri。前进宽度随字体变，不再套等线的 28。
        assert!(
            name == "微软雅黑"
                || name.eq_ignore_ascii_case("Microsoft YaHei")
                || name.eq_ignore_ascii_case("Calibri"),
            "实际字体是 {name}"
        );
    }

    #[test]
    fn yahei_and_song_do_not_share_an_em_top() {
        let px = 36.0 * 96.0 / 72.0;
        let mut boxes = Vec::new();
        for (name, text) in [("Microsoft YaHei", "16.9"), ("宋体", "万")] {
            let face = Face::open_named(px, true, name).expect(name);
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;
            face.draw(text, 8.0, 8.0, [0, 0, 0], |_, y, _, cov| {
                if cov > 30 {
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            });
            boxes.push((face.line_height(), face.em_box(), min_y, max_y));
        }
        let (yahei_asc, _, yahei_top, yahei_bot) = boxes[0];
        let (song_asc, _, song_top, song_bot) = boxes[1];
        assert!(
            (yahei_asc - song_asc).abs() > 1.0,
            "两种字体的基线偏移应该不同 yahei={yahei_asc} song={song_asc}"
        );
        assert!(
            song_top < yahei_top,
            "字顶对齐时宋体墨迹应更高 song={song_top} yahei={yahei_top}"
        );
        assert!(
            (yahei_bot - song_bot).abs() > 4,
            "同一 top 下两种字体的字底应该分开"
        );
    }

    #[test]
    fn device_ellipse_edge_is_partial() {
        let mut solid = 0u8;
        let mut partial = 0;
        assert!(fill_device_ellipse(10, 20, 74, 74, |x, y, cov| {
            if (36..=48).contains(&x) && (46..=58).contains(&y) && cov == 255 {
                solid = cov;
            }
            if cov > 0 && cov < 255 {
                partial += 1;
            }
        }));
        assert_eq!(solid, 255);
        assert!(partial > 8, "边缘应有过渡，实际 {partial}");
    }

    #[test]
    fn chart_text_antialiases() {
        let face = Face::open(13.0, false).expect("本机应有图表字体");
        let mut smooth = 0;
        face.draw("0", 4.0, 4.0, [0, 0, 0], |_, _, _, cov| {
            let gray8 = (0u16..=64).any(|sample| (sample * 255) / 64 == u16::from(cov));
            if cov > 0 && !gray8 {
                smooth += 1;
            }
        });
        assert!(smooth > 0, "应走 AntiAliasGridFit，而不是 Gray8");
    }
}

#[cfg(windows)]
pub use win::{fill_device_ellipse, fill_device_round_rect, Face};

#[cfg(not(windows))]
pub struct Face;

#[cfg(not(windows))]
impl Face {
    pub fn open(_: f32, _: bool) -> Option<Self> {
        None
    }

    pub fn open_named(_: f32, _: bool, _: &str) -> Option<Self> {
        None
    }

    pub fn open_cell(_: f32, _: bool, _: bool, _: &str) -> Option<Self> {
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

    pub fn layout_width(&self, text: &str) -> f64 {
        self.width(text)
    }

    pub fn cell_pad(&self) -> f64 {
        2.0
    }

    pub fn draw(&self, _: &str, _: f64, _: f64, _: [u8; 3], _: impl FnMut(i32, i32, [u8; 3], u8)) {}

    pub fn draw_cells(
        &self,
        text: &str,
        x: f64,
        top: f64,
        color: [u8; 3],
        blend: impl FnMut(i32, i32, [u8; 3], u8),
    ) {
        self.draw(text, x, top, color, blend);
    }
}

#[cfg(not(windows))]
pub fn fill_device_ellipse(_: i32, _: i32, _: i32, _: i32, _: impl FnMut(i32, i32, u8)) -> bool {
    false
}

#[cfg(not(windows))]
pub fn fill_device_round_rect(
    _: i32,
    _: i32,
    _: i32,
    _: i32,
    _: f64,
    _: impl FnMut(i32, i32, u8),
) -> bool {
    false
}
