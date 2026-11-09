//! layoutCheck 端到端：形状集合、组合换算、越界、边界相接、文字溢出与非 pptx 报错。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, Lang, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-layout-check-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("deck-{n}.pptx"))
}

fn create_deck() -> PathBuf {
    let path = tmp();
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": { "slides": [ { "shapes": [{"text": "占位"}] } ] }
    })
    .to_string())
    .unwrap();
    path
}

fn rewrite_slide(path: &Path, xml: &str) {
    let bytes = std::fs::read(path).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name.trim_start_matches('/') == "ppt/slides/slide1.xml" {
                data = xml.as_bytes().to_vec();
            }
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

/// 五个形状：越界 A、边界相接的 B / C、文字溢出 D、组合内的 F。
/// 组合放大 2 倍并平移 (1000, 2000)，F 的组内 (10,10,50,50) 应为 (1020,2020,100,100)。
const SLIDE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld><p:cSld name="S"><p:spTree>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Out"/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="-10" y="-100"/><a:ext cx="100" cy="50"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p/></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Left"/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="1000" y="1000"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p/></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="4" name="Right"/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="1100" y="1000"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p/></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="Tall Text"/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="3000" y="1000"/><a:ext cx="300000" cy="274320"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p><a:r><a:rPr sz="1800"/><a:t>a</a:t></a:r></a:p></p:txBody></p:sp>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="6" name="Group"/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="1000" y="2000"/><a:ext cx="2000" cy="2000"/><a:chOff x="0" y="0"/><a:chExt cx="1000" cy="1000"/></a:xfrm></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="7" name="In Group"/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="10" y="10"/><a:ext cx="50" cy="50"/></a:xfrm></p:spPr>
<p:txBody><a:bodyPr/><a:p/></p:txBody></p:sp>
</p:grpSp>
</p:spTree></p:cSld></p:sld>"#;

fn check(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "layoutCheck",
        "path": path.to_string_lossy(),
    })
    .to_string())
    .unwrap() else {
        panic!("layoutCheck 应返回 Inspect 树");
    };
    tree
}

#[test]
fn layout_check_reports_boxes_bounds_overlaps_and_overflow() {
    let deck = create_deck();
    rewrite_slide(&deck, SLIDE_XML);
    let tree = check(&deck);

    assert_eq!(tree["schemaVersion"], 1);
    assert_eq!(tree["format"], "pptx");
    assert_eq!(tree["slideSize"]["cx"], 12_192_000);
    assert_eq!(tree["slideSize"]["cy"], 6_858_000);
    assert!(tree.get("ok").is_none());
    assert!(tree.get("path").is_none());

    let slide = &tree["slides"][0];
    assert_eq!(slide["id"], "slide:1");
    let shapes = slide["shapes"].as_array().unwrap();
    // 组合本身不进 shapes，只展开里面带 txBody 的 sp。
    assert_eq!(shapes.len(), 5);
    assert_eq!(shapes[0]["name"], "Out");
    assert_eq!(shapes[0]["box"]["x"], -10);

    let grouped = shapes
        .iter()
        .find(|s| s["id"] == "slide:1/shape:7")
        .unwrap();
    assert_eq!(grouped["box"]["x"], 1_020);
    assert_eq!(grouped["box"]["y"], 2_020);
    assert_eq!(grouped["box"]["cx"], 100);
    assert_eq!(grouped["box"]["cy"], 100);

    assert_eq!(slide["outOfBounds"], json!(["slide:1/shape:2"]));
    assert_eq!(shapes[0]["outOfBounds"], true);
    assert_eq!(grouped["outOfBounds"], false);

    // B 与 C 边界相接不算相交；其余形状互不相交。
    assert_eq!(slide["overlaps"], json!([]));
    assert_eq!(
        slide["textOverflow"],
        json!([{ "id": "slide:1/shape:5", "estimated": true }])
    );
}

#[test]
fn layout_check_rejects_xlsx_and_docx_with_invalid_field() {
    let deck = create_deck();
    for format in ["xlsx", "docx"] {
        let err = run(&json!({
            "schemaVersion": 1,
            "action": "layoutCheck",
            "path": deck.to_string_lossy(),
            "format": format,
        })
        .to_string())
        .unwrap_err();
        let value = err.to_json(Lang::En);
        assert_eq!(value["error"]["code"], "InvalidField", "{value:?}");
        assert_eq!(value["error"]["path"], "format", "{value:?}");
    }
}
