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

// —— 页眉页脚 ——

const HF_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/header2.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/header3.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/></Types>"#;

const HF_DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header2.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header3.xml"/><Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/></Relationships>"#;

const HF_SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:evenAndOddHeaders/></w:settings>"#;

/// 两节：第一节有 default/first/even 页眉与 default 页脚；第二节省略引用，继承 default 页眉。
const HF_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p w14:paraId="BODY0001"><w:r><w:t>正文一</w:t></w:r><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rId3"/><w:headerReference w:type="first" r:id="rId4"/><w:headerReference w:type="even" r:id="rId5"/><w:footerReference w:type="default" r:id="rId6"/><w:titlePg/></w:sectPr></w:pPr></w:p><w:p w14:paraId="BODY0002"><w:r><w:t>正文二</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const HF_HEADER1: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="AAAA1001"><w:r><w:t>默认页眉</w:t></w:r></w:p></w:hdr>"#;

const HF_HEADER2: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="AAAA1001"><w:r><w:t>首页页眉</w:t></w:r></w:p></w:hdr>"#;

const HF_HEADER3: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="BBBB1001"><w:r><w:t>偶数页眉</w:t></w:r></w:p></w:hdr>"#;

const HF_FOOTER1: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:p w14:paraId="CCCC1001"><w:r><w:t>默认页脚</w:t></w:r></w:p></w:ftr>"#;

fn write_hf_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", HF_CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", HF_DOCUMENT),
            ("word/_rels/document.xml.rels", HF_DOC_RELS),
            ("word/styles.xml", STYLES),
            ("word/settings.xml", HF_SETTINGS),
            ("word/header1.xml", HF_HEADER1),
            ("word/header2.xml", HF_HEADER2),
            ("word/header3.xml", HF_HEADER3),
            ("word/footer1.xml", HF_FOOTER1),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn inspect_hf(path: &Path, expand: bool) -> serde_json::Value {
    let mut scope = json!({});
    if expand {
        scope = json!({ "headerFooter": true });
    }
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": scope
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

#[test]
fn inspect_lists_headers_footers_and_shared_sections() {
    let path = write_hf_sample("hf-list");
    let tree = inspect_hf(&path, false);
    assert!(tree.get("paragraphs").is_none());
    let headers = tree["headers"].as_array().unwrap();
    let footers = tree["footers"].as_array().unwrap();
    assert_eq!(headers.len(), 3);
    assert_eq!(footers.len(), 1);
    assert_eq!(headers[0]["id"], "header:default:1");
    assert_eq!(headers[0]["type"], "default");
    assert_eq!(headers[0]["sections"], json!([1, 2]));
    assert_eq!(headers[0]["paraCount"], 1);
    assert!(headers[0].get("paragraphs").is_none());
    assert_eq!(headers[1]["id"], "header:even:1");
    assert_eq!(headers[1]["sections"], json!([1, 2]));
    assert_eq!(headers[2]["id"], "header:first:1");
    assert_eq!(headers[2]["sections"], json!([1]));
    assert_eq!(footers[0]["id"], "footer:default:1");
    // 第二节继承 default 页脚
    assert_eq!(footers[0]["sections"], json!([1, 2]));
}

#[test]
fn inspect_expands_header_footer_paragraphs() {
    let path = write_hf_sample("hf-expand");
    let tree = inspect_hf(&path, true);
    let paras = tree["headers"][0]["paragraphs"].as_array().unwrap();
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0]["id"], "header:default:1/para:AAAA1001");
    assert_eq!(paras[0]["text"], "默认页眉");
    assert_eq!(paras[0]["stable"], true);
    assert_eq!(
        tree["headers"][2]["paragraphs"][0]["id"],
        "header:first:1/para:AAAA1001"
    );
    assert_eq!(
        tree["footers"][0]["paragraphs"][0]["text"],
        "默认页脚"
    );
}

#[test]
fn apply_header_leaves_document_bytes() {
    let path = write_hf_sample("hf-apply");
    let before_doc = part_bytes(&path, "word/document.xml");
    let before_footer = part_bytes(&path, "word/footer1.xml");
    let before_styles = part_bytes(&path, "word/styles.xml");
    let out = tmp_dir().join("hf-apply-out.docx");
    apply(
        &path,
        &out,
        "header:default:1/para:AAAA1001",
        "新页眉",
    )
    .unwrap();
    assert_eq!(before_doc, part_bytes(&out, "word/document.xml"));
    assert_eq!(before_footer, part_bytes(&out, "word/footer1.xml"));
    assert_eq!(before_styles, part_bytes(&out, "word/styles.xml"));
    let tree = inspect_hf(&out, true);
    assert_eq!(tree["headers"][0]["paragraphs"][0]["text"], "新页眉");
    // 首页页眉同 hex 未改
    assert_eq!(tree["headers"][2]["paragraphs"][0]["text"], "首页页眉");
}

#[test]
fn body_para_id_does_not_hit_header() {
    let path = write_hf_sample("hf-body-id");
    let out = tmp_dir().join("hf-body-id-out.docx");
    let err = apply(&path, &out, "para:AAAA1001", "不该写").unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn header_container_id_is_invalid() {
    let path = write_hf_sample("hf-container");
    let out = tmp_dir().join("hf-container-out.docx");
    let err = apply(&path, &out, "header:default:1", "不该写").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

fn apply_ops(path: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": ops
        })
        .to_string(),
        None,
    )
}

#[test]
fn inspect_section_distance_null_when_pgmar_missing() {
    let path = write_hf_sample("dist-null");
    let tree = inspect_hf(&path, false);
    let sects = tree["sections"].as_array().unwrap();
    assert_eq!(sects.len(), 2);
    assert_eq!(sects[0]["id"], "section:1");
    assert!(sects[0]["headerDistance"].is_null());
    assert!(sects[0]["footerDistance"].is_null());
    assert!(sects[1]["headerDistance"].is_null());
}

#[test]
fn apply_header_footer_distance_inserts_pgmar() {
    let path = write_hf_sample("dist-set");
    let before_styles = part_bytes(&path, "word/styles.xml");
    let before_header = part_bytes(&path, "word/header1.xml");
    let out = tmp_dir().join("dist-set-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 720,
            "footerDistance": 1440
        }]),
    )
    .unwrap();
    assert_eq!(before_styles, part_bytes(&out, "word/styles.xml"));
    assert_eq!(before_header, part_bytes(&out, "word/header1.xml"));
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["headerDistance"], 720);
    assert_eq!(tree["sections"][0]["footerDistance"], 1440);
    // 第二节未点名，仍为 null
    assert!(tree["sections"][1]["headerDistance"].is_null());
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains(r#"w:header="720""#));
    assert!(xml.contains(r#"w:footer="1440""#));
    assert!(!xml.contains("w:top="), "插入的 pgMar 不补其它边距");
}

#[test]
fn apply_distance_only_header_leaves_footer_null() {
    let path = write_hf_sample("dist-one");
    let out = tmp_dir().join("dist-one-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 360
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["headerDistance"], 360);
    assert!(tree["sections"][0]["footerDistance"].is_null());
}

#[test]
fn apply_distance_missing_section_is_not_found() {
    let path = write_hf_sample("dist-miss");
    let out = tmp_dir().join("dist-miss-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:9",
            "headerDistance": 720
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn apply_distance_second_op_keeps_other_attr() {
    let path = write_hf_sample("dist-keep");
    let mid = tmp_dir().join("dist-keep-mid.docx");
    let out = tmp_dir().join("dist-keep-out.docx");
    apply_ops(
        &path,
        &mid,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 720,
            "footerDistance": 1440
        }]),
    )
    .unwrap();
    apply_ops(
        &mid,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 360
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["headerDistance"], 360);
    assert_eq!(tree["sections"][0]["footerDistance"], 1440);
}

#[test]
fn apply_distance_on_create_keeps_page_margins() {
    let dir = tmp_dir();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let created = dir.join(format!("dist-create-{n}.docx"));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": created.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "a" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let before = part(&created, "word/document.xml");
    assert!(before.contains(r#"w:top="1440""#));
    let out = dir.join(format!("dist-create-out-{n}.docx"));
    apply_ops(
        &created,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 1440
        }]),
    )
    .unwrap();
    let after = part(&out, "word/document.xml");
    assert!(after.contains(r#"w:header="1440""#));
    assert!(after.contains(r#"w:footer="708""#));
    assert!(after.contains(r#"w:top="1440""#));
    assert!(after.contains(r#"w:left="1440""#));
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["headerDistance"], 1440);
    assert_eq!(tree["sections"][0]["footerDistance"], 708);
}

#[test]
fn apply_distance_bad_id_is_invalid() {
    let path = write_hf_sample("dist-bad-id");
    let out = tmp_dir().join("dist-bad-id-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "para:AAAA0001",
            "headerDistance": 720
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

// —— 链接前节 ——

/// 两节都显式指向同一 default 页眉，不算链接。
const SHARED_EXPLICIT_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rId3"/></w:sectPr></w:pPr></w:p><w:p><w:r><w:t>二</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="rId3"/></w:sectPr></w:body></w:document>"#;

const SHARED_EXPLICIT_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const SHARED_EXPLICIT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

fn write_shared_explicit(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", SHARED_EXPLICIT_CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", SHARED_EXPLICIT_DOCUMENT),
            ("word/_rels/document.xml.rels", SHARED_EXPLICIT_RELS),
            ("word/styles.xml", STYLES),
            ("word/header1.xml", HF_HEADER1),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn zip_has_part(path: &Path, name: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let ok = zip.by_name(name).is_ok();
    ok
}

#[test]
fn inspect_section_link_flags() {
    let path = write_hf_sample("link-flags");
    let tree = inspect_hf(&path, false);
    let s1 = &tree["sections"][0];
    let s2 = &tree["sections"][1];
    assert_eq!(s1["headerLink"]["default"], false);
    assert_eq!(s1["headerLink"]["even"], false);
    assert_eq!(s1["headerLink"]["first"], false);
    assert_eq!(s1["footerLink"]["default"], false);
    assert_eq!(s1["footerLink"]["even"], false);
    assert_eq!(s1["footerLink"]["first"], false);
    assert_eq!(s2["headerLink"]["default"], true);
    assert_eq!(s2["headerLink"]["even"], true);
    assert!(s2["headerLink"]["first"].is_null());
    assert_eq!(s2["footerLink"]["default"], true);
    assert_eq!(s2["footerLink"]["even"], true);
    assert!(s2["footerLink"]["first"].is_null());
}

#[test]
fn inspect_two_explicit_same_part_is_not_link() {
    let path = write_shared_explicit("link-shared");
    let tree = inspect_hf(&path, false);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], false);
    assert_eq!(tree["headers"][0]["sections"], json!([1, 2]));
}

#[test]
fn unlink_copies_part_and_edit_does_not_touch_previous() {
    let path = write_hf_sample("link-unlink");
    let out = tmp_dir().join("link-unlink-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:2",
            "kind": "header",
            "type": "default",
            "linked": false
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, true);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], false);
    let headers = tree["headers"].as_array().unwrap();
    let defaults: Vec<_> = headers
        .iter()
        .filter(|h| h["type"] == "default")
        .collect();
    assert_eq!(defaults.len(), 2);
    assert_eq!(defaults[0]["sections"], json!([1]));
    assert_eq!(defaults[1]["sections"], json!([2]));
    assert_eq!(defaults[0]["paragraphs"][0]["text"], "默认页眉");
    assert_eq!(defaults[1]["paragraphs"][0]["text"], "默认页眉");
    assert!(zip_has_part(&out, "word/header4.xml"));

    let edited = tmp_dir().join("link-unlink-edit.docx");
    let new_id = defaults[1]["paragraphs"][0]["id"].as_str().unwrap();
    apply(&out, &edited, new_id, "第二节页眉").unwrap();
    let after = inspect_hf(&edited, true);
    let defs: Vec<_> = after["headers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|h| h["type"] == "default")
        .collect();
    assert_eq!(defs[0]["paragraphs"][0]["text"], "默认页眉");
    assert_eq!(defs[1]["paragraphs"][0]["text"], "第二节页眉");
}

#[test]
fn relink_drops_orphan_part() {
    let path = write_hf_sample("link-relink");
    let mid = tmp_dir().join("link-relink-mid.docx");
    let out = tmp_dir().join("link-relink-out.docx");
    apply_ops(
        &path,
        &mid,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:2",
            "kind": "header",
            "type": "default",
            "linked": false
        }]),
    )
    .unwrap();
    assert!(zip_has_part(&mid, "word/header4.xml"));
    apply_ops(
        &mid,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:2",
            "kind": "header",
            "type": "default",
            "linked": true
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], true);
    let defaults: Vec<_> = tree["headers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|h| h["type"] == "default")
        .collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0]["sections"], json!([1, 2]));
    assert!(!zip_has_part(&out, "word/header4.xml"));
    let cts = part(&out, "[Content_Types].xml");
    assert!(!cts.contains("/word/header4.xml"));
    let rels = part(&out, "word/_rels/document.xml.rels");
    assert!(!rels.contains("header4.xml"));
}

#[test]
fn first_section_cannot_link() {
    let path = write_hf_sample("link-first");
    let out = tmp_dir().join("link-first-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:1",
            "kind": "header",
            "type": "default",
            "linked": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn inactive_even_cannot_link() {
    let path = write_sample("link-even-off");
    let out = tmp_dir().join("link-even-off-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:1",
            "kind": "header",
            "type": "even",
            "linked": false
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn link_missing_section_is_not_found() {
    let path = write_hf_sample("link-miss");
    let out = tmp_dir().join("link-miss-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:9",
            "kind": "header",
            "type": "default",
            "linked": false
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn link_already_target_is_noop() {
    let path = write_hf_sample("link-noop");
    let before_doc = part_bytes(&path, "word/document.xml");
    let before_rels = part_bytes(&path, "word/_rels/document.xml.rels");
    let out = tmp_dir().join("link-noop-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:2",
            "kind": "header",
            "type": "default",
            "linked": true
        }]),
    )
    .unwrap();
    assert_eq!(before_doc, part_bytes(&out, "word/document.xml"));
    assert_eq!(before_rels, part_bytes(&out, "word/_rels/document.xml.rels"));
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][1]["headerLink"]["default"], true);
}

#[test]
fn unlink_blank_writes_minimal_header() {
    let path = write_sample("link-blank");
    let out = tmp_dir().join("link-blank-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setHeaderFooterLink",
            "id": "section:1",
            "kind": "header",
            "type": "default",
            "linked": false
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, true);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert_eq!(tree["headers"].as_array().unwrap().len(), 1);
    assert_eq!(tree["headers"][0]["id"], "header:default:1");
    assert_eq!(tree["headers"][0]["paraCount"], 1);
    assert_eq!(tree["headers"][0]["paragraphs"][0]["text"], "");
    assert!(zip_has_part(&out, "word/header1.xml"));
}
