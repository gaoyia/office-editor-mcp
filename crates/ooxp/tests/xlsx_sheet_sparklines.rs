//! Existing spark painter composed into the common sheet/tile path.
#![cfg(windows)]
use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use zip::write::SimpleFileOptions;
static SEQ: AtomicU32 = AtomicU32::new(0);
fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join("ooxp-sheet-spark-composite");
    std::fs::create_dir_all(&d).unwrap();
    d.join(format!(
        "{}-{}.xlsx",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}
#[allow(clippy::too_many_arguments)]
fn book(
    host: &str,
    row_ht: f64,
    flags: &str,
    cols: &str,
    tail: &str,
    fill: bool,
    color: &str,
    present: bool,
) -> PathBuf {
    let seed = tmp();
    run(&json!({"schemaVersion":1,"action":"create","path":seed,"workbook":{"sheets":[{"name":"Sheet1","cells":[{"ref":"B1","value":1}]}]}}).to_string()).unwrap();
    let non_a1 = host == "B2";
    let (row, refs, range, formula) = if non_a1 {
        (2, ["C2", "D2", "E2"], "B2:E2", "C2:E2")
    } else {
        (1, ["B1", "C1", "D1"], "A1:D1", "B1:D1")
    };
    let range = if tail == "narrow-dimension" {
        "B1:D1"
    } else {
        range
    };
    let extension = if present {
        format!(
            r#"<extLst><ext uri="{{05C60535-1F16-4fd2-B633-F4F36F0B64E0}}" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"><x14:sparklineGroups><x14:sparklineGroup displayEmptyCellsAs="gap" {flags}><x14:colorSeries rgb="{color}"/><x14:sparklines><x14:sparkline><xm:f>Sheet1!{formula}</xm:f><xm:sqref>{host}</xm:sqref></x14:sparkline></x14:sparklines></x14:sparklineGroup></x14:sparklineGroups></ext></extLst>"#
        )
    } else {
        String::new()
    };
    let values = if flags.contains("stacked") {
        [1, -4, 2]
    } else {
        [1, 4, 2]
    };
    let mut cells = String::new();
    for (r, v) in refs.iter().zip(values) {
        cells.push_str(&format!("<c r=\"{r}\"><v>{v}</v></c>"));
    }
    let hidden = if tail == "hidden-row" {
        " hidden=\"1\""
    } else {
        ""
    };
    let view = if tail == "rtl" {
        " rightToLeft=\"1\""
    } else {
        ""
    };
    let merge = if tail == "merge" {
        r#"<mergeCells count="1"><mergeCell ref="A1:A2"/></mergeCells>"#
    } else {
        ""
    };
    let host_value = if tail == "host-text" {
        format!(r#"<c r="{host}" t="inlineStr"><is><t>host</t></is></c>"#)
    } else if fill {
        format!(r#"<c r="{host}"/>"#)
    } else {
        String::new()
    };
    let sheet = format!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="{range}"/><sheetViews><sheetView workbookViewId="0" showGridLines="0"{view}/></sheetViews><sheetFormatPr defaultRowHeight="{row_ht}" customHeight="1"/>{cols}<sheetData><row r="{row}" ht="{row_ht}" customHeight="1"{hidden}>{host_value}{cells}</row></sheetData>{merge}{extension}</worksheet>"#
    );
    let mut z = zip::ZipArchive::new(Cursor::new(std::fs::read(&seed).unwrap())).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if name == "xl/worksheets/sheet1.xml" {
            data = sheet.as_bytes().to_vec();
        }
        if fill && name == "xl/styles.xml" {
            let text = String::from_utf8(data).unwrap();
            let start = text.find("<fills").unwrap();
            let end = text.find("</fills>").unwrap() + 8;
            let text=format!("{}<fills count=\"1\"><fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF285078\"/></patternFill></fill></fills>{}",&text[..start],&text[end..]);
            data = text.into_bytes();
        }
        out.start_file(name, SimpleFileOptions::default()).unwrap();
        out.write_all(&data).unwrap();
    }
    let p = tmp();
    std::fs::write(&p, out.finish().unwrap().into_inner()).unwrap();
    p
}
fn image(book: &Path, action: &str) -> Result<(u32, u32, Vec<u8>), ooxp::Error> {
    let output = tmp().with_extension("png");
    let id = if action == "exportSparklineImage" {
        "sheet:Sheet1/spark:1"
    } else {
        "sheet:Sheet1"
    };
    run(
        &json!({"schemaVersion":1,"action":action,"path":book,"id":id,"output":output}).to_string(),
    )?;
    Ok(decode(&std::fs::read(output).unwrap()))
}
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut r = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut b = vec![0; r.output_buffer_size()];
    let f = r.next_frame(&mut b).unwrap();
    b.truncate(f.buffer_size());
    (f.width, f.height, b)
}
fn same_host(book: &Path) {
    let (sw, _, sheet) = image(book, "exportSheetImage").unwrap();
    let (w, h, spark) = image(book, "exportSparklineImage").unwrap();
    for y in 0..h {
        for x in 0..w {
            let a = ((y * sw + x) * 3) as usize;
            let b = ((y * w + x) * 3) as usize;
            assert_eq!(&sheet[a..a + 3], &spark[b..b + 3], "host pixel {x},{y}");
        }
    }
    assert!(
        spark.iter().any(|v| *v != 255),
        "fixture must actually paint a spark"
    );
}
#[test]
fn default_line_host_equals_independent_and_does_not_need_materialized_cell() {
    same_host(&book("A1", 15., "", "", "", false, "FF376092", true));
}
#[test]
fn non_a1_row_13_host_equals_independent() {
    same_host(&book("B2", 13., "", "", "", false, "FF376092", true));
}
#[test]
fn column_and_win_loss_reuse_the_same_painter() {
    for kind in ["column", "stacked"] {
        same_host(&book(
            "A1",
            15.,
            &format!("type=\"{kind}\""),
            "",
            "",
            false,
            "FF376092",
            true,
        ));
    }
}
#[test]
fn hidden_source_is_ni_but_explicit_display_hidden_is_supported() {
    let cols = r#"<cols><col min="3" max="3" hidden="1"/></cols>"#;
    let p = book("A1", 15., "", cols, "", false, "FF376092", true);
    assert_eq!(
        image(&p, "exportSheetImage").unwrap_err().code(),
        ErrorCode::NotImplemented
    );
    same_host(&book(
        "A1",
        15.,
        "displayHidden=\"1\"",
        cols,
        "",
        false,
        "FF376092",
        true,
    ));
}
#[test]
fn hidden_host_does_not_reject_invisible_unsupported_config() {
    for (cols, tail) in [
        (r#"<cols><col min="1" max="1" hidden="1"/></cols>"#, ""),
        ("", "hidden-row"),
    ] {
        let p = book(
            "A1",
            15.,
            "markers=\"1\"",
            cols,
            tail,
            false,
            "FF376092",
            true,
        );
        let no = book(
            "A1",
            15.,
            "markers=\"1\"",
            cols,
            tail,
            false,
            "FF376092",
            false,
        );
        assert_eq!(
            image(&p, "exportSheetImage").unwrap(),
            image(&no, "exportSheetImage").unwrap()
        );
    }
}
#[test]
fn visible_unsupported_markers_date_axis_rtl_merge_and_text_are_honest_ni() {
    for (flags, tail) in [
        ("markers=\"1\"", ""),
        ("dateAxis=\"1\"", ""),
        ("", "rtl"),
        ("", "merge"),
        ("", "host-text"),
    ] {
        let p = book("A1", 15., flags, "", tail, false, "FF376092", true);
        assert_eq!(
            image(&p, "exportSheetImage").unwrap_err().code(),
            ErrorCode::NotImplemented,
            "{flags} {tail}"
        );
    }
}
#[test]
fn white_spark_on_colored_fill_is_not_a_white_key_mask() {
    let p = book("A1", 15., "", "", "", true, "FFFFFFFF", true);
    let (w, _, rgb) = image(&p, "exportSheetImage").unwrap();
    let mut aa = 0;
    for y in 2..19 {
        for x in 2..63 {
            let i = ((y * w + x) * 3) as usize;
            let r = rgb[i];
            if r > 40 && r < 255 {
                aa += 1;
                assert!(rgb[i + 1] > 80 && rgb[i + 2] > 120);
            }
        }
    }
    assert!(
        aa > 0,
        "white stroke must retain float coverage over the colored cell"
    );
}
#[test]
fn tile_edges_crossing_the_stroke_stitch_exactly() {
    let p = book("B2", 15., "", "", "", false, "FF376092", true);
    let (w, h, mono) = image(&p, "exportSheetImage").unwrap();
    for size in [7u32, 17, 37] {
        let output = tmp().with_extension("png");
        let r=run(&json!({"schemaVersion":1,"action":"exportSheetImage","path":p,"id":"sheet:Sheet1","output":output,"tilePx":size}).to_string()).unwrap();
        let RunOutput::Written {
            paths: Some(paths), ..
        } = r
        else {
            panic!("tiles")
        };
        let mut all = vec![255; (w * h * 3) as usize];
        for path in paths {
            let n = path.file_stem().unwrap().to_string_lossy();
            let rc = n.rsplit('-').next().unwrap();
            let (r, c) = rc.split_once('c').unwrap();
            let row: u32 = r.trim_start_matches('r').parse().unwrap();
            let col: u32 = c.parse().unwrap();
            let (tw, th, b) = decode(&std::fs::read(path).unwrap());
            for y in 0..th {
                for x in 0..tw {
                    let a = ((y * tw + x) * 3) as usize;
                    let d = (((row * size + y) * w + col * size + x) * 3) as usize;
                    all[d..d + 3].copy_from_slice(&b[a..a + 3]);
                }
            }
        }
        assert_eq!(
            all, mono,
            "tile {size}: full-host AA phase must not restart per tile"
        );
    }
}

#[test]
fn host_outside_stale_dimension_contributes_to_used_range() {
    let p = book(
        "A1",
        15.0,
        "",
        "",
        "narrow-dimension",
        false,
        "FF376092",
        true,
    );
    same_host(&p);
    assert_eq!(image(&p, "exportSheetImage").unwrap().0, 257);
}
