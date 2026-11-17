//! Fixed96 Normal metrics are shared by sheets, tiles, sparks and two-cell charts.
#![cfg(windows)]
use ooxp::{run, RunOutput};
use serde_json::json;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
static SEQ: AtomicU32 = AtomicU32::new(0);
fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join("ooxp-fixed96-geometry");
    std::fs::create_dir_all(&d).unwrap();
    d.join(format!(
        "{tag}-{}-{}.xlsx",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}
fn ok(v: serde_json::Value) -> RunOutput {
    run(&v.to_string()).unwrap()
}
fn png(book: &Path, action: &str, id: &str) -> (u32, u32, Vec<u8>) {
    let dest = temp("png").with_extension("png");
    ok(
        json!({"schemaVersion":1,"action":action,"path":book.to_string_lossy(),"id":id,"output":dest.to_string_lossy()}),
    );
    let bytes = std::fs::read(dest).unwrap();
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}
fn corpus(face: &str, pt: u32, default_width: bool, hidden: bool, non_a1: bool) -> PathBuf {
    let seed = temp("seed");
    ok(
        json!({"schemaVersion":1,"action":"create","path":seed.to_string_lossy(),"workbook":{"sheets":[{"name":"Sheet1","cells":[{"ref":"B1","value":1},{"ref":"C1","value":4},{"ref":"D1","value":2}]}]}}),
    );
    let sparked = temp("sparked");
    ok(
        json!({"schemaVersion":1,"action":"apply","path":seed.to_string_lossy(),"output":sparked.to_string_lossy(),"ops":[{"op":"insertSparkline","id":"sheet:Sheet1","ref":"A1","formula":"B1:D1"}]}),
    );
    let mut src = ZipArchive::new(Cursor::new(std::fs::read(sparked).unwrap())).unwrap();
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let mut file = src.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if name == "xl/styles.xml" {
            data=format!(r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><sz val="{pt}"/><name val="{face}"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="2"><xf fontId="0"/><xf fontId="1"/></cellStyleXfs><cellXfs count="1"><xf fontId="0" fillId="0" borderId="0" numFmtId="0" xfId="0"/></cellXfs><cellStyles count="2"><cellStyle name="Normal" xfId="0"/><cellStyle name="常规重命名" builtinId="0" xfId="1"/></cellStyles></styleSheet>"#).into_bytes();
        } else if name == "xl/worksheets/sheet1.xml" {
            let old = String::from_utf8(data).unwrap();
            let ext = &old[old.find("<extLst>").unwrap()..old.rfind("</worksheet>").unwrap()];
            let (range, row, host, refs, formula) = if non_a1 {
                ("B2:E2", 2, "B2", ["C2", "D2", "E2"], "C2:E2")
            } else {
                ("A1:D1", 1, "A1", ["B1", "C1", "D1"], "B1:D1")
            };
            let ext = ext
                .replace("Sheet1!B1:D1", &format!("Sheet1!{formula}"))
                .replace(
                    "<xm:sqref>A1</xm:sqref>",
                    &format!("<xm:sqref>{host}</xm:sqref>"),
                );
            let mut cols = String::new();
            if !default_width || hidden {
                cols.push_str("<cols>");
                for c in 1..=10 {
                    let hide = if hidden && c == 3 {
                        " hidden=\"1\""
                    } else {
                        ""
                    };
                    cols.push_str(&format!(
                        "<col min=\"{c}\" max=\"{c}\" width=\"10.125\"{hide}/>"
                    ));
                }
                cols.push_str("</cols>");
            }
            let default = if default_width {
                " defaultColWidth=\"10\""
            } else {
                ""
            };
            let mut cells = format!("<c r=\"{host}\"/>");
            for (cr, v) in refs.iter().zip([1, 4, 2]) {
                cells.push_str(&format!("<c r=\"{cr}\"><v>{v}</v></c>"));
            }
            data=format!(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="{range}"/><sheetFormatPr defaultRowHeight="15" customHeight="1"{default}/>{cols}<sheetData><row r="{row}" ht="15" customHeight="1">{cells}</row></sheetData>{ext}</worksheet>"#).into_bytes();
        }
        out.start_file(name, SimpleFileOptions::default()).unwrap();
        out.write_all(&data).unwrap();
    }
    let book = temp("case");
    std::fs::write(&book, out.finish().unwrap().into_inner()).unwrap();
    book
}
fn chart(book: &Path, non_a1: bool) -> PathBuf {
    let out = temp("chart");
    let (from, to) = if non_a1 { ("B3", "F13") } else { ("A3", "E13") };
    ok(
        json!({"schemaVersion":1,"action":"apply","path":book.to_string_lossy(),"output":out.to_string_lossy(),"ops":[{"op":"insertChart","id":"sheet:Sheet1","type":"col","anchor":"twoCell","from":from,"to":to,"dataFrom":"A20","series":[{"name":"S","categories":["A","B","C"],"values":[1,4,2]}]}]}),
    );
    out
}
#[test]
fn normal_font_matrix_matches_across_exports() {
    for (face, pt, mdw) in [
        ("Calibri", 11, 7),
        ("Arial", 11, 8),
        ("Consolas", 11, 8),
        ("DengXian", 11, 8),
        ("Calibri", 14, 10),
    ] {
        for default in [false, true] {
            let book = corpus(face, pt, default, false, false);
            let col = if default {
                10 * mdw
            } else {
                ((256.0 * 10.125 + (128.0 / f64::from(mdw)).floor()) / 256.0 * f64::from(mdw))
                    .floor() as u32
            };
            let (sw, sh, _) = png(&book, "exportSheetImage", "sheet:Sheet1");
            let (pw, ph, _) = png(&book, "exportSparklineImage", "sheet:Sheet1/spark:1");
            assert_eq!(
                (sw, sh),
                (4 * col + 1, 21),
                "sheet {face} {pt} default={default}"
            );
            assert_eq!((pw, ph), (col, 20), "spark {face}");
            let (cw, ch, _) = png(
                &chart(&book, false),
                "exportChartImage",
                "sheet:Sheet1/chart:1",
            );
            assert_eq!((cw, ch), (4 * col, 200), "chart {face} {pt}");
        }
    }
}
#[test]
fn hidden_and_non_a1_columns_are_shared() {
    for (hidden, non_a1, expected) in [(true, false, 213), (false, true, 284)] {
        let book = corpus("Calibri", 11, false, hidden, non_a1);
        let (w, h, _) = png(&book, "exportSheetImage", "sheet:Sheet1");
        assert_eq!((w, h), (expected + 1, 21));
        let (cw, ch, _) = png(
            &chart(&book, non_a1),
            "exportChartImage",
            "sheet:Sheet1/chart:1",
        );
        assert_eq!((cw, ch), (expected, 200));
    }
}
#[test]
fn non_a1_tiles_use_same_geometry_and_pixels() {
    let book = corpus("Arial", 11, false, false, true);
    let (w, h, mono) = png(&book, "exportSheetImage", "sheet:Sheet1");
    let output = temp("tiles").with_extension("png");
    let result = ok(
        json!({"schemaVersion":1,"action":"exportSheetImage","path":book.to_string_lossy(),"id":"sheet:Sheet1","output":output.to_string_lossy(),"tilePx":100}),
    );
    let RunOutput::Written { paths, .. } = result else {
        panic!("written tiles")
    };
    let paths = paths.expect("multiple tiles");
    assert_eq!(paths.len(), 4);
    let mut stitched = vec![255_u8; (w * h * 3) as usize];
    for path in paths {
        let filename = path.file_name().unwrap().to_string_lossy();
        let rc = filename
            .rsplit('-')
            .next()
            .unwrap()
            .trim_end_matches(".png");
        let (r, c) = rc.split_once('c').unwrap();
        let row: u32 = r.trim_start_matches('r').parse().unwrap();
        let col: u32 = c.parse().unwrap();
        let mut reader = png::Decoder::new(Cursor::new(std::fs::read(path).unwrap()))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        for y in 0..info.height {
            for x in 0..info.width {
                let src = ((y * info.width + x) * 3) as usize;
                let dst = (((row * 100 + y) * w + col * 100 + x) * 3) as usize;
                stitched[dst..dst + 3].copy_from_slice(&buf[src..src + 3]);
            }
        }
    }
    assert_eq!((w, h), (325, 21));
    assert_eq!(
        stitched, mono,
        "same geometry and pixels for non-A1 Arial tiles"
    );
}
