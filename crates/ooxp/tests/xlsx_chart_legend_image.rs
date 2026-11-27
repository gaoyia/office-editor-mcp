//! Existing clustered-column legend configuration must not disappear silently.
#![cfg(windows)]
use ooxp::run;
use serde_json::json;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
static SEQ: AtomicU32 = AtomicU32::new(0);
fn path(ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-legend-regression");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!(
        "{}-{}.{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed),
        ext
    ))
}
fn book(legend: &str) -> PathBuf {
    let src = path("xlsx");
    let chart = path("xlsx");
    let out = path("xlsx");
    run(&json!({"schemaVersion":1,"action":"create","path":src,"workbook":{"sheets":[{"name":"Sheet1"}]}}).to_string()).unwrap();
    run(&json!({"schemaVersion":1,"action":"apply","path":src,"output":chart,"ops":[{"op":"insertChart","id":"sheet:Sheet1","type":"col","anchor":"twoCell","from":"A3","to":"J20","dataFrom":"A25","series":[{"name":"S1","categories":["A","B","C","D"],"values":[1,4,3,2]},{"name":"S2","categories":["A","B","C","D"],"values":[2,1,4,3]}]}]}).to_string()).unwrap();
    let mut z = ZipArchive::new(Cursor::new(std::fs::read(chart).unwrap())).unwrap();
    let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        if name == "xl/charts/chart1.xml" {
            let mut s = String::from_utf8(b).unwrap();
            if legend.starts_with("<!--title-->") {
                s=s.replace("<c:autoTitleDeleted val=\"1\"/>","<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Existing title</a:t></a:r></a:p></c:rich></c:tx></c:title><c:autoTitleDeleted val=\"0\"/>");
            }
            let legend = legend.strip_prefix("<!--title-->").unwrap_or(legend);
            if let Some(start) = s.find("<c:legend>") {
                let end = start + s[start..].find("</c:legend>").unwrap() + "</c:legend>".len();
                s.replace_range(start..end, legend);
            } else {
                s = s.replace("</c:chart>", &format!("{legend}</c:chart>"));
            }
            b = s.into_bytes();
        }
        dst.start_file(name, SimpleFileOptions::default()).unwrap();
        dst.write_all(&b).unwrap();
    }
    std::fs::write(&out, dst.finish().unwrap().into_inner()).unwrap();
    out
}
fn bytes(legend: &str) -> Result<Vec<u8>, ooxp::Error> {
    let src = book(legend);
    let dst = path("png");
    run(&json!({"schemaVersion":1,"action":"exportChartImage","path":src,"id":"sheet:Sheet1/chart:1","output":dst}).to_string())?;
    Ok(std::fs::read(dst).unwrap())
}
fn pixels(legend: &str) -> (usize, usize, Vec<[u8; 3]>) {
    let b = bytes(legend).unwrap();
    let mut r = png::Decoder::new(Cursor::new(b)).read_info().unwrap();
    let mut v = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut v).unwrap();
    let rgb = v[..info.buffer_size()]
        .chunks_exact(info.color_type.samples())
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    (info.width as usize, info.height as usize, rgb)
}
#[test]
fn valid_style_is_drawn_and_not_treated_as_a_broken_default() {
    let default = bytes("<c:legend><c:legendPos val='r'/></c:legend>").unwrap();
    let styled="<c:legend><c:legendPos val='r'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz='1800' b='1' i='1'><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill><a:latin typeface='Arial'/></a:defRPr></a:pPr></a:p></c:txPr></c:legend>";
    let (w, _, p) = pixels(styled);
    assert!(
        p.iter()
            .enumerate()
            .filter(|(i, c)| i % w > w / 2 && c[0] > 180 && c[1] < 70 && c[2] < 70)
            .count()
            > 30
    );
    assert_ne!(default, bytes(styled).unwrap());
}
#[test]
fn bottom_left_top_and_top_right_are_visible() {
    for pos in ["b", "l", "t", "tr"] {
        let (w, h, p) = pixels(&format!("<c:legend><c:legendPos val='{pos}'/></c:legend>"));
        let n = p
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                let x = i % w;
                let y = i / w;
                **c == [21, 96, 130]
                    && match pos {
                        "b" => y > h - 32,
                        "l" => x < 40,
                        "t" => y < 35,
                        _ => x > w - 80 && y < 35,
                    }
            })
            .count();
        assert!(n > 10, "{pos}: {n}");
    }
}
#[test]
fn malformed_optional_values_fall_back_but_explicit_close_stays_closed() {
    let baseline = bytes("<c:legend><c:legendPos val='r'/></c:legend>").unwrap();
    assert_eq!(baseline,bytes("<c:legend><c:legendPos val='not-a-position'/><c:txPr><a:p><a:pPr><a:defRPr sz='NaN'/></a:pPr></a:p></c:txPr></c:legend>").unwrap());
    let (w, h, p) = pixels("");
    assert!(!p
        .iter()
        .enumerate()
        .any(|(i, c)| i / w > h - 28 && *c == [21, 96, 130]));
    assert_eq!(w, 576);
}
#[test]
fn manual_factor_edge_overlay_and_entry_delete_have_real_effects() {
    let normal = "<c:legend><c:legendPos val='r'/></c:legend>";
    let overlay = "<c:legend><c:legendPos val='r'/><c:overlay val='1'/></c:legend>";
    assert_eq!(bytes(normal).unwrap(), bytes(overlay).unwrap());
    assert_ne!(
        bytes(normal).unwrap(),
        bytes("<c:legend><c:legendPos val='r'/><c:overlay val='0'/></c:legend>").unwrap()
    );
    let manual = |mode| {
        format!("<c:legend><c:legendPos val='r'/><c:layout><c:manualLayout><c:xMode val='{mode}'/><c:yMode val='{mode}'/><c:x val='.1'/><c:y val='.1'/><c:w val='.4'/><c:h val='.3'/></c:manualLayout></c:layout></c:legend>")
    };
    assert_ne!(
        bytes(&manual("edge")).unwrap(),
        bytes(&manual("factor")).unwrap()
    );
    let(w,_,p)=pixels("<c:legend><c:legendPos val='r'/><c:legendEntry><c:idx val='0'/><c:delete val='1'/></c:legendEntry></c:legend>");
    assert!(!p
        .iter()
        .enumerate()
        .any(|(i, c)| i % w > w - 55 && *c == [21, 96, 130]));
}
#[test]
fn valid_unimplemented_complex_text_is_not_silently_ignored() {
    for content in [
        "<a:bodyPr lIns='10000'/><a:p/>",
        "<a:p><a:pPr><a:lnSpc><a:spcPct val='150000'/></a:lnSpc></a:pPr></a:p>",
        "<a:p><a:pPr><a:defRPr><a:solidFill><a:srgbClr val='FF0000'><a:alphaMod val='50000'/></a:srgbClr></a:solidFill></a:defRPr></a:pPr></a:p>",
        "<a:p><a:pPr><a:defRPr><a:solidFill><a:schemeClr val='tx1'/></a:solidFill></a:defRPr></a:pPr></a:p>",
        "<a:p><a:pPr><a:defRPr><a:highlight><a:srgbClr val='FFFF00'/></a:highlight></a:defRPr></a:pPr></a:p>",
    ] { assert!(bytes(&format!("<c:legend><c:txPr>{content}</c:txPr></c:legend>")).is_err(), "{content}"); }

    assert!(bytes(
        "<c:legend><c:txPr><a:p><a:pPr><a:defRPr u='wavy'/></a:pPr></a:p></c:txPr></c:legend>"
    )
    .is_err());
    assert!(bytes("<c:legend><c:layout><c:manualLayout><c:wMode val='edge'/><c:w val='.4'/></c:manualLayout></c:layout></c:legend>").is_ok());
    assert!(bytes("<c:legend><c:layout><c:manualLayout><c:layoutTarget val='other'/><c:w val='.4'/></c:manualLayout></c:layout></c:legend>").is_err());
}

#[test]
fn top_legend_does_not_cover_the_existing_title_band() {
    let (w, _, p) = pixels("<!--title--><c:legend><c:legendPos val='t'/></c:legend>");
    let first = p
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == [21, 96, 130])
        .map(|(i, _)| i / w)
        .min()
        .unwrap();
    assert!((30..70).contains(&first), "first legend key at {first}");
}

#[test]
fn manual_width_distributes_free_space_without_scaling_text() {
    for style in ["", "<c:txPr><a:p><a:pPr><a:defRPr sz='1800' b='1'><a:latin typeface='Arial'/></a:defRPr></a:pPr></a:p></c:txPr>"] {
        let pitch = |fraction| {
            let (w,_,p)=pixels(&format!("<c:legend><c:legendPos val='r'/><c:layout><c:manualLayout><c:xMode val='edge'/><c:yMode val='edge'/><c:x val='0'/><c:y val='0'/><c:w val='{fraction}'/><c:h val='.12'/></c:manualLayout></c:layout><c:overlay val='1'/>{style}</c:legend>"));
            let first = |rgb| p.iter().enumerate().filter(|(i,c)| i/w < 30 && **c==rgb).map(|(i,_)|i%w).min().unwrap();
            (w,(first([233,113,50])-first([21,96,130])) as f64)
        };
        let(w,narrow)=pitch(0.4);let(_,wide)=pitch(0.8);
        assert!((wide-narrow-w as f64*0.4/3.0).abs()<=2.0,"{style}: {narrow} -> {wide}");
    }
}

#[test]
fn all_five_excel_positions_distinguish_reserved_from_overlay_but_not_omitted_true() {
    for pos in ["l", "r", "t", "b", "tr"] {
        let legend =
            |overlay: &str| format!("<c:legend><c:legendPos val='{pos}'/>{overlay}</c:legend>");
        let absent = bytes(&legend("")).unwrap();
        assert_eq!(
            absent,
            bytes(&legend("<c:overlay/>")).unwrap(),
            "{pos}: empty bool defaults true"
        );
        assert_eq!(
            absent,
            bytes(&legend("<c:overlay val='1'/>")).unwrap(),
            "{pos}: absent overlay follows Excel"
        );
        assert_ne!(
            absent,
            bytes(&legend("<c:overlay val='0'/>")).unwrap(),
            "{pos}: explicit 0 must reserve plot space"
        );
    }
}

// Controlled packages below are library-generated regressions, not Office gold.
fn entry_style(idx: usize, props: &str) -> String {
    format!("<c:legendEntry><c:idx val='{idx}'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr {props}</a:defRPr></a:pPr></a:p></c:txPr></c:legendEntry>")
}
#[test]
fn entry_color_inherits_metrics_and_recolors_only_its_own_text() {
    let tx = |color| {
        entry_style(
            0,
            &format!("><a:solidFill><a:srgbClr val='{color}'/></a:solidFill>"),
        )
    };
    let legend = |color| {
        format!(
            "<c:legend><c:legendPos val='r'/><c:overlay val='0'/>{}</c:legend>",
            tx(color)
        )
    };
    let (w, h, a) = pixels(&legend("000000"));
    let (ww, hh, b) = pixels(&legend("FF0000"));
    assert_eq!((w, h), (ww, hh));
    let changed: Vec<_> = a
        .iter()
        .zip(&b)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .collect();
    assert!(!changed.is_empty());
    assert!(changed.iter().all(|(idx, _)| idx % w > w / 2));
    assert!(b.iter().any(|p| p[0] > 180 && p[1] < 70 && p[2] < 70));
    // Series keys remain their series colors, not the text override.
    for c in [[21, 96, 130], [233, 113, 50]] {
        let pa: Vec<_> = a
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == c)
            .map(|(i, _)| i)
            .collect();
        let pb: Vec<_> = b
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == c)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(pa, pb);
    }
}
#[test]
fn entry_font_size_bold_and_family_have_real_ink_and_pack_together() {
    let legend = |props: &str| {
        format!(
            "<c:legend><c:legendPos val='r'/><c:overlay val='0'/>{}</c:legend>",
            entry_style(0, props)
        )
    };
    let ordinary = bytes(&legend("> ")).unwrap();
    for props in ["sz='1800'>", "b='1'>", "><a:latin typeface='Arial'/><a:ea typeface='Arial'/><a:cs typeface='Arial'/>", "sz='1800' b='1'><a:latin typeface='Arial'/><a:ea typeface='Arial'/><a:cs typeface='Arial'/>"] {
        assert_ne!(ordinary,bytes(&legend(props)).unwrap(),"{props}");
    }
    let (w, _, small) = pixels(&legend(">"));
    let (_, _, large) = pixels(&legend("sz='1800'>"));
    let key_count = |p: &Vec<[u8; 3]>| {
        p.iter()
            .enumerate()
            .filter(|(i, c)| i % w > w - 100 && **c == [21, 96, 130])
            .count()
    };
    assert!(key_count(&large) > key_count(&small));
    assert!(bytes(&legend(
        "><a:latin typeface='Arial'/><a:cs typeface='Courier New'/> "
    ))
    .is_err());
    assert!(bytes(&legend("u='wavy'>")).is_err());
}

#[test]
fn official_like_legend_underline_strike_and_subscript_export() {
    let ctrl = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/></c:legend>";
    let underline = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr u='sng'/></a:pPr><a:endParaRPr lang='zh-CN'/></a:p></c:txPr></c:legend>";
    let strike = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr strike='sngStrike'/></a:pPr><a:endParaRPr lang='zh-CN'/></a:p></c:txPr></c:legend>";
    let sub = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr baseline='-25000'/></a:pPr><a:endParaRPr lang='zh-CN'/></a:p></c:txPr></c:legend>";
    let dbl = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr u='dbl'/></a:pPr><a:endParaRPr lang='zh-CN'/></a:p></c:txPr></c:legend>";
    let (cw, ch, cp) = pixels(ctrl);
    for (name, xml) in [
        ("underline", underline),
        ("strike", strike),
        ("subscript", sub),
        ("doubleunderline", dbl),
    ] {
        let (w, h, p) = pixels(xml);
        assert_eq!((cw, ch), (w, h), "{name} must not grow the canvas");
        assert_ne!(bytes(ctrl).unwrap(), bytes(xml).unwrap(), "{name}");
        let changed = cp.iter().zip(&p).filter(|(a, b)| a != b).count();
        assert!(changed > 10, "{name} changed={changed}");
    }
}

#[test]
fn legend_solid_alpha_and_explicit_rectangular_border_paint_in_all_positions() {
    for pos in ["l", "r", "t", "b", "tr"] {
        let legend = |sp: &str| {
            format!("<c:legend><c:legendPos val='{pos}'/><c:spPr>{sp}</c:spPr></c:legend>")
        };
        let (_, _, solid) = pixels(&legend(
            "<a:solidFill><a:srgbClr val='FFFF00'/></a:solidFill>",
        ));
        assert!(
            solid.iter().filter(|c| **c == [255, 255, 0]).count() > 30,
            "{pos}"
        );
        let (_, _, alpha) = pixels(&legend(
            "<a:solidFill><a:srgbClr val='FFFF00'><a:alpha val='50000'/></a:srgbClr></a:solidFill>",
        ));
        assert!(
            alpha
                .iter()
                .any(|c| c[0] == 255 && c[1] == 255 && (120..=135).contains(&c[2])),
            "{pos}"
        );
        let (_, _, border) = pixels(&legend(
            "<a:ln w='25400'><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln>",
        ));
        assert!(
            border.iter().filter(|c| **c == [255, 0, 0]).count() > 20,
            "{pos}"
        );
    }
    let normal = bytes("<c:legend><c:legendPos val='r'/></c:legend>").unwrap();
    assert_eq!(
        normal,
        bytes("<c:legend><c:legendPos val='r'/><c:spPr><a:noFill/></c:spPr></c:legend>").unwrap()
    );
}
#[test]
fn unimplemented_legend_effects_gradients_and_complex_lines_remain_explicit_ni() {
    for sp in [
        "<a:gradFill/>",
        "<a:effectLst><a:innerShdw/></a:effectLst>",
        "<a:effectLst><a:glow/></a:effectLst>",
        "<a:ln w='25400'><a:prstDash val='dash'/><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln>",
        "<a:ln><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln>",
        "<a:solidFill><a:schemeClr val='accent1'/></a:solidFill>",
    ] {
        assert!(
            bytes(&format!(
                "<c:legend><c:legendPos val='r'/><c:spPr>{sp}</c:spPr></c:legend>"
            ))
            .is_err(),
            "{sp}"
        );
    }
}

#[test]
fn official_like_legend_linear_gradient_and_outer_shadow_export() {
    let ctrl = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/></c:legend>";
    let grad = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='FF0000'/></a:gs><a:gs pos='100000'><a:srgbClr val='FFFFFF'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr></c:legend>";
    let shadow = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:spPr><a:effectLst><a:outerShdw blurRad='50800' dist='71842' dir='2700013' rotWithShape='0'><a:srgbClr val='000000'><a:alpha val='60000'/></a:srgbClr></a:outerShdw></a:effectLst></c:spPr></c:legend>";
    let empty_shadow = "<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:spPr><a:effectLst><a:outerShdw/></a:effectLst></c:spPr></c:legend>";
    let (cw, ch, _) = pixels(ctrl);
    let (gw, gh, gp) = pixels(grad);
    let (sw, sh, spx) = pixels(shadow);
    let (ew, eh, _) = pixels(empty_shadow);
    assert_eq!((cw, ch), (gw, gh), "gradFill must not grow the canvas");
    assert_eq!((cw, ch), (sw, sh), "outerShdw must not grow the canvas");
    assert_eq!((cw, ch), (ew, eh));
    assert_eq!(bytes(ctrl).unwrap(), bytes(empty_shadow).unwrap());
    assert_ne!(bytes(ctrl).unwrap(), bytes(grad).unwrap());
    assert_ne!(bytes(ctrl).unwrap(), bytes(shadow).unwrap());
    // Close-enough evidence only: a vertical sRGB gradient appears in the
    // right legend band, and an official-like shadow darkens chart-fill
    // pixels there. Residuals are not a unique official-matrix lock.
    let reds = gp
        .iter()
        .enumerate()
        .filter(|(i, c)| i % gw > gw * 2 / 3 && c[0] > 200 && c[1] < 80 && c[2] < 80)
        .count();
    assert!(reds > 20, "legend gradFill reds={reds}");
    let fill = [234_u8, 247, 247];
    let darkened = spx
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            i % sw > sw * 2 / 3
                && c[0] < fill[0]
                && c[1] < fill[1]
                && c[2] < fill[2]
                && **c != [0, 0, 0]
                && **c != [137, 137, 137]
                && **c != [21, 96, 130]
                && **c != [233, 113, 50]
        })
        .count();
    assert!(darkened > 20, "legend outerShdw darkened={darkened}");
}

#[test]
fn entry_only_color_preserves_unstyled_default_geometry_and_other_ink() {
    for title in ["", "<!--title-->"] {
        for overlay in ["0", "1"] {
            let make = |entry: &str| {
                format!("{title}<c:legend><c:legendPos val='r'/><c:overlay val='{overlay}'/>{entry}</c:legend>")
            };
            let base = make("");
            let (w, h, before) = pixels(&base);
            for index in 0..2 {
                let entry = |color| {
                    format!("<c:legendEntry><c:idx val='{index}'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr><a:solidFill><a:srgbClr val='{color}'/></a:solidFill></a:defRPr></a:pPr></a:p></c:txPr></c:legendEntry>")
                };
                assert_eq!(
                    bytes(&base).unwrap(),
                    bytes(&make(&entry("000000"))).unwrap()
                );
                let (ww, hh, after) = pixels(&make(&entry("FF0000")));
                assert_eq!((w, h), (ww, hh));
                let mut changed = 0;
                for (i, (a, b)) in before.iter().zip(&after).enumerate() {
                    if a != b {
                        changed += 1;
                        assert!(i % w > w * 4 / 5, "color moved non-legend ink");
                        assert!(a[0] == a[1] && a[1] == a[2], "color changed a series key");
                        assert!(b[0] > b[1] && b[1] == b[2], "not just red glyph coverage");
                    }
                }
                assert!(changed > 10);
            }
        }
    }
}

#[test]
fn mixed_entry_font_widths_keep_a_shared_vertical_key_column() {
    for properties in ["sz='1800'", "sz='1800' b='1'"] {
        let legend = format!("<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:legendEntry><c:idx val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr {properties}><a:latin typeface='Arial'/></a:defRPr></a:pPr></a:p></c:txPr></c:legendEntry></c:legend>");
        let (w, _, p) = pixels(&legend);
        let right = |color| {
            p.iter()
                .enumerate()
                .filter(|(_, c)| **c == color)
                .map(|(i, _)| i % w)
                .max()
                .unwrap()
        };
        assert_eq!(right([21, 96, 130]), right([233, 113, 50]));
    }
}

#[test]
fn legend_key_and_pitch_follow_family_cells_not_only_point_size() {
    let measure = |family, size| {
        let legend = format!("<c:legend><c:legendPos val='r'/><c:overlay val='0'/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz='{size}'><a:latin typeface='{family}'/></a:defRPr></a:pPr></a:p></c:txPr></c:legend>");
        let (w, _, pixels) = pixels(&legend);
        let key = |color| {
            let x = pixels
                .iter()
                .enumerate()
                .filter(|(_, c)| **c == color)
                .map(|(i, _)| i % w)
                .max()
                .unwrap();
            let ys: Vec<_> = pixels
                .iter()
                .enumerate()
                .filter(|(i, c)| **c == color && i % w == x)
                .map(|(i, _)| i / w)
                .collect();
            (*ys.iter().min().unwrap(), *ys.iter().max().unwrap())
        };
        let a = key([21, 96, 130]);
        let b = key([233, 113, 50]);
        (a.1 - a.0 + 1, b.0 - a.0)
    };
    let arial = measure("Arial", 1800);
    let segoe = measure("Segoe UI", 1800);
    let large = measure("Arial", 2400);
    assert!(segoe.0 > arial.0 && segoe.1 > arial.1);
    assert!(large.0 > arial.0 && large.1 > arial.1);
}
