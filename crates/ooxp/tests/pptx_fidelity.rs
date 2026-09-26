//! pptx 保真度：AlternateContent 只走一支、不改 timing、未改页原样写回。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/><Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#;

/// 名单顺序是 slide2 再 slide1，用来核对 n 不跟文件名走。
const PRESENTATION: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#;

const PRES_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#;

/// 文件名是 slide1，但关系顺序里排第二。含 timing、Choice/Fallback、组合、重复 id。
const SLIDE1: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><p:cSld name="复杂页"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><mc:AlternateContent><mc:Choice Requires="p14"><p:sp><p:nvSpPr><p:cNvPr id="4" name="choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr b="1"/><a:t>choice</a:t></a:r></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="4" name="fallback"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>fallback</a:t></a:r></a:p></p:txBody></p:sp></mc:Fallback></mc:AlternateContent><p:grpSp><p:nvGrpSpPr><p:cNvPr id="10" name="组"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="5" name="组内"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>grouped</a:t></a:r></a:p></p:txBody></p:sp></p:grpSp><p:sp><p:nvSpPr><p:cNvPr id="4" name="重复"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>dup</a:t></a:r></a:p></p:txBody></p:sp><p:pic><p:nvPicPr><p:cNvPr id="9" name="图"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill/><p:spPr/></p:pic></p:spTree></p:cSld><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never"/></p:par></p:tnLst></p:timing></p:sld>"#;

const SLIDE2: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" show="0"><p:cSld name="首页"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="keep"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>keep-me</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-fidelity");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.pptx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("ppt/presentation.xml", PRESENTATION),
            ("ppt/_rels/presentation.xml.rels", PRES_RELS),
            ("ppt/slides/slide1.xml", SLIDE1),
            ("ppt/slides/slide2.xml", SLIDE2),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

#[test]
fn inspect_follows_sld_id_list_not_filename() {
    let path = write_sample("order");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 1 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["slideCount"], 2);
    assert_eq!(tree["slides"][0]["name"], "首页");
    assert_eq!(tree["slides"][0]["hidden"], true);
    assert_eq!(tree["slides"][0]["shapes"][0]["text"], "keep-me");
    assert_eq!(tree["slides"][1]["name"], "复杂页");
    assert!(tree["slides"][1].get("shapes").is_none());
}

#[test]
fn inspect_skips_fallback_and_pictures() {
    let path = write_sample("choice");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 2 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    let shapes = tree["slides"][1]["shapes"].as_array().unwrap();
    let texts: Vec<_> = shapes.iter().map(|s| s["text"].as_str().unwrap()).collect();
    assert_eq!(texts, ["choice", "grouped", "dup"]);
    assert!(!texts.contains(&"fallback"));
    assert_eq!(shapes[0]["id"], "slide:2/shape:4");
    assert_eq!(shapes[2]["id"], "slide:2/shape:4");
}

#[test]
fn apply_hits_first_duplicate_and_keeps_timing_and_other_slide() {
    let src = write_sample("apply");
    let src_slide2 = part_bytes(&src, "ppt/slides/slide2.xml");
    let out = tmp_dir().join("applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:2/shape:4", "text": "改过" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "slide": 2 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    let shapes = tree["slides"][1]["shapes"].as_array().unwrap();
    assert_eq!(shapes[0]["text"], "改过");
    assert_eq!(shapes[2]["text"], "dup");

    let slide1 = part(&out, "ppt/slides/slide1.xml");
    assert!(slide1.contains("<p:timing>"), "不得丢掉 p:timing");
    assert!(slide1.contains("dur=\"indefinite\""));
    assert!(!slide1.contains(">choice<"));
    assert!(slide1.contains(">fallback<"), "Fallback 那支不应被改");

    assert_eq!(
        part_bytes(&out, "ppt/slides/slide2.xml"),
        src_slide2,
        "未改的那一页必须原字节写回"
    );
}

#[test]
fn apply_missing_shape_is_id_not_found() {
    let src = write_sample("missing");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:99", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn apply_preserves_run_bold() {
    let src = write_sample("bold");
    let out = tmp_dir().join("bold-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:2/shape:4", "text": "仍粗" }]
        })
        .to_string(),
        None,
    )
    .unwrap();
    let slide1 = part(&out, "ppt/slides/slide1.xml");
    assert!(slide1.contains(r#"<a:rPr b="1"/>"#));
    assert!(slide1.contains("仍粗"));
}

/// 单页样本，一个形状三段：第一段没有 `a:pPr`，第二段居中，第三段斜体。
/// 用来核对写入部落在第一段、不把别段的 `a:pPr` 搬过来，掏空的兄弟段则删掉。
const MULTI_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/></Types>"#;

const MULTI_PRES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#;

const MULTI_PRES_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#;

const MULTI_SLIDE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="三段"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="三段"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr b="1"/><a:t>第一段</a:t></a:r></a:p><a:p><a:pPr algn="ctr"/><a:r><a:t>第二段</a:t></a:r></a:p><a:p><a:r><a:rPr i="1"/><a:t>第三段</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

/// 一个段落里用段落级 `a:br` 换行，这是 PowerPoint 的写法：`a:br` 与 `a:r` 平级。
const BR_SLIDE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="带换行"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="换行框"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr b="1"/><a:t>上</a:t></a:r><a:br/><a:r><a:t>下</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

/// 第一段空、只有右对齐；第二段有字。写入应落在第二段，空的第一段删掉。
const LEADING_EMPTY_SLIDE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="前空"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="前空"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="r"/></a:p><a:p><a:pPr algn="l"/><a:r><a:t>正文</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

/// 第二段是页码域。inspect 不收域缓存，apply 也不动这段。
const FLD_SLIDE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="带域"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="带域"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>标题</a:t></a:r></a:p><a:p><a:fld id="{00000000-0000-0000-0000-000000000001}" type="slidenum"><a:t>1</a:t></a:fld></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

fn write_slide_sample(tag: &str, slide: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.pptx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", MULTI_CT),
            ("_rels/.rels", ROOT_RELS),
            ("ppt/presentation.xml", MULTI_PRES),
            ("ppt/_rels/presentation.xml.rels", MULTI_PRES_RELS),
            ("ppt/slides/slide1.xml", slide),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn write_multi_sample(tag: &str) -> PathBuf {
    write_slide_sample(tag, MULTI_SLIDE)
}

/// 读回来的文本必须和写进去的一模一样，这是契约「setShapeText」第 9 条。
fn read_back(path: &Path) -> String {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 1 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    tree["slides"][0]["shapes"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

fn count_paras(xml: &str) -> usize {
    // `<a:p` 也是 `<a:pPr` 的前缀，只能按这两种收尾形式数。
    xml.matches("<a:p>").count() + xml.matches("<a:p/>").count()
}

#[test]
fn apply_keeps_written_para_and_drops_empty_siblings() {
    let src = write_multi_sample("multi");
    let out = tmp_dir().join("multi-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "新文本" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let slide = part(&out, "ppt/slides/slide1.xml");
    assert_eq!(count_paras(&slide), 1, "掏空的兄弟段应删掉：{slide}");
    // 第一段原样只换字：自己的 b=1 留着，且没有凭空多出 a:pPr。
    assert!(
        slide.contains(r#"<a:p><a:r><a:rPr b="1"/><a:t>新文本</a:t></a:r></a:p>"#),
        "第一段被改坏了：{slide}"
    );
    // 第二段的居中不能搬到写入段，空段删掉后也不该再出现。
    assert!(
        !slide.contains(r#"<a:pPr algn="ctr"/>"#),
        "别段的对齐还在：{slide}"
    );
    assert!(!slide.contains("第一段"));
    assert!(!slide.contains("第二段"));
    assert!(!slide.contains("第三段"));
    assert_eq!(read_back(&out), "新文本");
}

/// `a:br` 与 `a:r` 平级（`EG_TextRun`，§A.4），`CT_RegularTextRun` 只允许一个 `a:t`。
/// 换行拆错层级会生成不合 schema 的 run。
#[test]
fn apply_puts_br_between_runs_not_inside() {
    let src = write_multi_sample("br");
    let out = tmp_dir().join("br-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "上\n下" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let slide = part(&out, "ppt/slides/slide1.xml");
    assert!(
        slide.contains(
            r#"<a:r><a:rPr b="1"/><a:t>上</a:t></a:r><a:br/><a:r><a:rPr b="1"/><a:t>下</a:t></a:r>"#
        ),
        "换行要在段落级拆成 a:br 加新的 a:r，rPr 跟着走：{slide}"
    );
    assert!(
        !slide.contains("<a:br/><a:t>"),
        "a:br 不能待在 a:r 里面：{slide}"
    );
    assert_eq!(count_paras(&slide), 1);
    assert_eq!(read_back(&out), "上\n下");
}

/// 源文件本来就有 `a:br`，改成单行文本时它必须被清掉。inspect 把 `a:br` 读成 `\n`，
/// 留一个下来读回值就比写入值多一个换行。
#[test]
fn apply_single_line_clears_existing_br() {
    let src = write_slide_sample("existing-br", BR_SLIDE);
    assert_eq!(read_back(&src), "上\n下", "样本本身应当是两行");

    let out = tmp_dir().join("existing-br-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "单行" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let slide = part(&out, "ppt/slides/slide1.xml");
    assert!(!slide.contains("<a:br/>"), "原有的 a:br 没清掉：{slide}");
    assert_eq!(read_back(&out), "单行");
    assert_eq!(count_paras(&slide), 1);
}

/// 换行写进去再改回单行，来回一趟都要对得上。
#[test]
fn apply_multiline_then_single_line_roundtrips() {
    let src = write_slide_sample("roundtrip", BR_SLIDE);
    let two = tmp_dir().join("rt-two.pptx");
    let one = tmp_dir().join("rt-one.pptx");
    for (from, to, text) in [
        (&src, &two, "甲\n乙\n丙"),
        (&two, &one, "只剩一行"),
    ] {
        run(
            &json!({
                "schemaVersion": 1,
                "action": "apply",
                "path": from.to_string_lossy(),
                "output": to.to_string_lossy(),
                "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": text }]
            })
            .to_string(),
            None,
        )
        .unwrap();
        assert_eq!(read_back(to), text);
    }
    let slide = part(&one, "ppt/slides/slide1.xml");
    assert!(!slide.contains("<a:br/>"));
}

#[test]
fn apply_drops_leading_empty_para() {
    let src = write_slide_sample("leading-empty", LEADING_EMPTY_SLIDE);
    let out = tmp_dir().join("leading-empty-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "新" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let slide = part(&out, "ppt/slides/slide1.xml");
    assert_eq!(count_paras(&slide), 1, "空的前段应删掉：{slide}");
    assert!(
        slide.contains(r#"<a:p><a:pPr algn="l"/><a:r><a:t>新</a:t></a:r></a:p>"#),
        "写入段自己的对齐应留下：{slide}"
    );
    assert!(
        !slide.contains(r#"algn="r""#),
        "空的前段对齐还在：{slide}"
    );
    assert_eq!(read_back(&out), "新");
}

#[test]
fn inspect_skips_field_cached_text() {
    let src = write_slide_sample("fld-inspect", FLD_SLIDE);
    assert_eq!(read_back(&src), "标题", "域里的缓存字不应进 text");
}

#[test]
fn apply_keeps_sibling_that_still_has_field() {
    let src = write_slide_sample("fld", FLD_SLIDE);
    assert_eq!(read_back(&src), "标题");
    let out = tmp_dir().join("fld-applied.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": src.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "新" }]
        })
        .to_string(),
        None,
    )
    .unwrap();

    let slide = part(&out, "ppt/slides/slide1.xml");
    assert_eq!(count_paras(&slide), 2, "含域的兄弟段应留下：{slide}");
    assert!(
        slide.contains(r#"<a:fld id="{00000000-0000-0000-0000-000000000001}" type="slidenum"><a:t>1</a:t></a:fld>"#),
        "域被清掉了：{slide}"
    );
    assert!(slide.contains("<a:t>新</a:t>"), "写入文本不见了：{slide}");
    assert!(!slide.contains("标题"));
    assert_eq!(read_back(&out), "新");
}
