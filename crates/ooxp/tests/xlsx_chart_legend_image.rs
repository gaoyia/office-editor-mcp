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
    assert_ne!(bytes(normal).unwrap(), bytes(overlay).unwrap());
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
        "<c:legend><c:txPr><a:p><a:pPr><a:defRPr u='sng'/></a:pPr></a:p></c:txPr></c:legend>"
    )
    .is_err());
    assert!(bytes("<c:legend><c:layout><c:manualLayout><c:wMode val='edge'/><c:w val='.4'/></c:manualLayout></c:layout></c:legend>").is_err());
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
