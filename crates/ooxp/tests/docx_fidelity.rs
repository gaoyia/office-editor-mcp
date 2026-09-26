//! docx 保真度：文本框与 AlternateContent 边界、混排 run、修订、表格、重复 paraId。
//!
//! 样例刻意照 Word 真实存盘的写法造。库自己 `create` 出来的文件没有文本框，
//! 只用往返测试会漏掉 A9 那种遍历缺陷。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

/// 未改部件，用来核对 apply 之后字节不变。
const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><!-- STYLES-MARKER --><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

/// 九段正文：标题、混排、超链接、修订、表格、带 Choice/Fallback 文本框的外层、裸文本框外层、重复 paraId、只有文本框的空外层。
const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="w14"><w:body><w:p w14:paraId="AAAA0001"><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>标题</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:r><w:rPr><w:b/></w:rPr><w:t>粗</w:t></w:r><w:r><w:t>细</w:t></w:r></w:p><w:p w14:paraId="AAAA0003"><w:hyperlink r:id="rId9"><w:r><w:t>链接字</w:t></w:r></w:hyperlink></w:p><w:p w14:paraId="AAAA0004"><w:r><w:t>留下</w:t></w:r><w:del w:id="0"><w:r><w:delText>删掉的</w:delText></w:r></w:del></w:p><w:tbl><w:tr><w:tc><w:p w14:paraId="AAAA0005"><w:r><w:t>表内</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p w14:paraId="AAAA0006"><w:r><w:t>外层</w:t></w:r><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><w:txbxContent><w:p w14:paraId="BBBB0001"><w:r><w:t>choice框</w:t></w:r></w:p></w:txbxContent></w:drawing></mc:Choice><mc:Fallback><w:pict><w:txbxContent><w:p w14:paraId="BBBB0002"><w:r><w:t>fallback框</w:t></w:r></w:p></w:txbxContent></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p><w:p w14:paraId="AAAA0007"><w:r><w:t>旁注</w:t></w:r><w:r><w:drawing><w:txbxContent><w:p w14:paraId="BBBB0003"><w:r><w:t>裸框</w:t></w:r></w:p></w:txbxContent></w:drawing></w:r></w:p><w:p w14:paraId="AAAA0002"><w:r><w:t>重复</w:t></w:r></w:p><w:p w14:paraId="AAAA0008"><w:r><w:drawing><w:txbxContent><w:p w14:paraId="BBBB0004"><w:r><w:t>仅框</w:t></w:r></w:p></w:txbxContent></w:drawing></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-fidelity");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", DOCUMENT),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn inspect_all(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 200 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, id: &str, text: &str) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [{ "op": "setParaText", "id": id, "text": text }]
        })
        .to_string(),
        None,
    )
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

fn texts(tree: &serde_json::Value) -> Vec<&str> {
    tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect()
}

fn ids(tree: &serde_json::Value) -> Vec<&str> {
    tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect()
}

#[test]
fn inspect_skips_textbox_and_does_not_double_alternate() {
    let path = write_sample("inspect");
    let tree = inspect_all(&path);
    assert_eq!(tree["paraCount"], 9);
    assert_eq!(
        texts(&tree),
        vec![
            "标题",
            "粗细",
            "链接字",
            "留下",
            "表内",
            "外层",
            "旁注",
            "重复",
            ""
        ]
    );
    assert_eq!(
        ids(&tree),
        vec![
            "para:AAAA0001",
            "para:AAAA0002",
            "para:AAAA0003",
            "para:AAAA0004",
            "para:AAAA0005",
            "para:AAAA0006",
            "para:AAAA0007",
            "para:AAAA0002",
            "para:AAAA0008",
        ]
    );
    assert_eq!(tree["outline"][0]["text"], "标题");
    assert_eq!(tree["outline"][0]["level"], 1);
}

#[test]
fn apply_textbox_para_id_is_not_found() {
    let path = write_sample("txbx-id");
    let out = tmp_dir().join("txbx-id-out.docx");
    let err = apply(&path, &out, "para:BBBB0001", "不该写进去").unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn apply_outer_para_keeps_textbox_and_roundtrips() {
    let path = write_sample("outer");
    let out = tmp_dir().join("outer-out.docx");
    apply(&path, &out, "para:AAAA0006", "改过的外层").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][5]["text"], "改过的外层");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("choice框"), "Choice 文本框应原样留下");
    assert!(xml.contains("fallback框"), "Fallback 文本框应原样留下");
    assert!(!xml.contains(">外层<"));
}

#[test]
fn apply_preserves_first_run_bold() {
    let path = write_sample("mixed");
    let out = tmp_dir().join("mixed-out.docx");
    apply(&path, &out, "para:AAAA0002", "新混排").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][1]["text"], "新混排");
    assert_eq!(tree["paragraphs"][7]["text"], "重复");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("<w:b/>"), "第一个 run 的加粗应留下");
    assert!(!xml.contains(">粗<"));
    assert!(!xml.contains(">细<"));
}

#[test]
fn apply_keeps_hyperlink_shell() {
    let path = write_sample("link");
    let out = tmp_dir().join("link-out.docx");
    apply(&path, &out, "para:AAAA0003", "新链接").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][2]["text"], "新链接");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("<w:hyperlink"));
    assert!(xml.contains("r:id=\"rId9\""));
}

#[test]
fn inspect_skips_deleted_text() {
    let path = write_sample("del");
    let tree = inspect_all(&path);
    assert_eq!(tree["paragraphs"][3]["text"], "留下");
}

#[test]
fn apply_table_cell_paragraph() {
    let path = write_sample("table");
    let out = tmp_dir().join("table-out.docx");
    apply(&path, &out, "para:AAAA0005", "新表内").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][4]["text"], "新表内");
}

#[test]
fn apply_duplicate_para_id_hits_first() {
    let path = write_sample("dup");
    let out = tmp_dir().join("dup-out.docx");
    apply(&path, &out, "para:AAAA0002", "先改到的").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][1]["text"], "先改到的");
    assert_eq!(tree["paragraphs"][7]["text"], "重复");
}

#[test]
fn apply_empty_outer_inserts_run_and_roundtrips() {
    let path = write_sample("empty-outer");
    let out = tmp_dir().join("empty-outer-out.docx");
    apply(&path, &out, "para:AAAA0008", "新插").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][8]["text"], "新插");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("仅框"), "文本框原文应不动");
}

#[test]
fn apply_by_index_matches_collect_order() {
    let path = write_sample("index");
    let out = tmp_dir().join("index-out.docx");
    apply(&path, &out, "para:index:6", "按序号改").unwrap();
    let tree = inspect_all(&out);
    assert_eq!(tree["paragraphs"][5]["id"], "para:AAAA0006");
    assert_eq!(tree["paragraphs"][5]["text"], "按序号改");
}

#[test]
fn apply_leaves_styles_byte_identical() {
    let path = write_sample("styles");
    let before = part_bytes(&path, "word/styles.xml");
    let out = tmp_dir().join("styles-out.docx");
    apply(&path, &out, "para:AAAA0001", "新标题").unwrap();
    let after = part_bytes(&out, "word/styles.xml");
    assert_eq!(before, after);
}
