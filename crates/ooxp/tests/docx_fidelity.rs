//! docx 保真度：文本框与 AlternateContent 边界、混排 run、修订、表格、重复 paraId。
//!
//! 样例刻意照 Word 真实存盘的写法造。库自己 `create` 出来的文件没有修订、表格
//! 和 AlternateContent 双支，只用往返测试会漏掉 A9 那种遍历缺陷。

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
    assert_eq!(tree["textBoxCount"], 3);
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
    assert_eq!(headers[0]["pictureCount"], 0);
    assert_eq!(headers[0]["textBoxCount"], 0);
    assert!(headers[0].get("paragraphs").is_none());
    assert!(headers[0].get("pictures").is_none());
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
    assert_eq!(tree["footers"][0]["paragraphs"][0]["text"], "默认页脚");
}

#[test]
fn apply_header_leaves_document_bytes() {
    let path = write_hf_sample("hf-apply");
    let before_doc = part_bytes(&path, "word/document.xml");
    let before_footer = part_bytes(&path, "word/footer1.xml");
    let before_styles = part_bytes(&path, "word/styles.xml");
    let out = tmp_dir().join("hf-apply-out.docx");
    apply(&path, &out, "header:default:1/para:AAAA1001", "新页眉").unwrap();
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
fn inspect_section_layout_null_when_missing() {
    let path = write_hf_sample("sect-null");
    let tree = inspect_hf(&path, false);
    let s1 = &tree["sections"][0];
    assert!(s1["top"].is_null());
    assert!(s1["pageWidth"].is_null());
    assert!(s1["orient"].is_null());
    assert!(s1["columns"].is_null());
    assert!(s1["vAlign"].is_null());
    assert!(s1["lineNumbering"].is_null());
    assert!(s1["docGrid"].is_null());
    assert!(s1["rtlGutter"].is_null());
    assert_eq!(tree["sections"][1]["pageWidth"], 11906);
    assert_eq!(tree["sections"][1]["pageHeight"], 16838);
    assert!(tree["sections"][1]["orient"].is_null());
    assert!(tree["mirrorMargins"].is_null());
    assert!(tree["gutterAtTop"].is_null());
}

#[test]
fn apply_page_margin_and_size_leave_unnamed() {
    let path = write_hf_sample("sect-margin");
    let out = tmp_dir().join("sect-margin-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            {
                "op": "setPageMargin",
                "id": "section:1",
                "top": 1440,
                "left": 1800,
                "right": 1800,
                "bottom": 1440
            },
            {
                "op": "setPageSize",
                "id": "section:1",
                "pageWidth": 16838,
                "pageHeight": 11906,
                "orient": "landscape"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["top"], 1440);
    assert_eq!(tree["sections"][0]["left"], 1800);
    assert!(tree["sections"][0]["gutter"].is_null());
    assert!(tree["sections"][0]["headerDistance"].is_null());
    assert_eq!(tree["sections"][0]["pageWidth"], 16838);
    assert_eq!(tree["sections"][0]["orient"], "landscape");
    let xml = part(&out, "word/document.xml");
    assert!(!xml.contains("w:gutter="));
    assert!(!xml.contains("w:header="));
}

#[test]
fn apply_page_margin_keeps_header_distance_on_create() {
    let dir = tmp_dir();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let created = dir.join(format!("sect-create-{n}.docx"));
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
    let out = dir.join(format!("sect-create-out-{n}.docx"));
    apply_ops(
        &created,
        &out,
        json!([{
            "op": "setPageMargin",
            "id": "section:1",
            "top": 720,
            "gutter": 360
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["top"], 720);
    assert_eq!(tree["sections"][0]["gutter"], 360);
    assert_eq!(tree["sections"][0]["headerDistance"], 708);
    assert_eq!(tree["sections"][0]["left"], 1440);
    apply_ops(
        &out,
        &out,
        json!([{
            "op": "setHeaderFooterDistance",
            "id": "section:1",
            "headerDistance": 1440
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["headerDistance"], 1440);
    assert_eq!(tree["sections"][0]["top"], 720);
}

#[test]
fn apply_columns_valign_linenum_grid_flags() {
    let path = write_hf_sample("sect-rest");
    let out = tmp_dir().join("sect-rest-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            { "op": "setColumns", "id": "section:1", "num": 2, "space": 720, "equalWidth": true },
            { "op": "setVerticalAlign", "id": "section:1", "vAlign": "center" },
            { "op": "setLineNumbering", "id": "section:1", "countBy": 1, "restart": "newPage" },
            { "op": "setDocGrid", "id": "section:1", "type": "lines", "linePitch": 360 },
            { "op": "setRtlGutter", "id": "section:1", "rtlGutter": true },
            { "op": "setMirrorMargins", "id": "docx:document", "mirrorMargins": true },
            { "op": "setGutterAtTop", "id": "docx:document", "gutterAtTop": true }
        ]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert_eq!(tree["sections"][0]["columns"]["num"], 2);
    assert_eq!(tree["sections"][0]["columns"]["equalWidth"], true);
    assert_eq!(tree["sections"][0]["vAlign"], "center");
    assert_eq!(tree["sections"][0]["lineNumbering"]["countBy"], 1);
    assert_eq!(tree["sections"][0]["docGrid"]["type"], "lines");
    assert_eq!(tree["sections"][0]["rtlGutter"], true);
    assert_eq!(tree["mirrorMargins"], true);
    assert_eq!(tree["gutterAtTop"], true);

    apply_ops(
        &out,
        &out,
        json!([
            { "op": "setColumns", "id": "section:1", "enabled": false },
            { "op": "setVerticalAlign", "id": "section:1", "vAlign": null },
            { "op": "setLineNumbering", "id": "section:1", "enabled": false },
            { "op": "setDocGrid", "id": "section:1", "enabled": false },
            { "op": "setRtlGutter", "id": "section:1", "rtlGutter": false },
            { "op": "setMirrorMargins", "id": "docx:document", "mirrorMargins": false }
        ]),
    )
    .unwrap();
    let tree = inspect_hf(&out, false);
    assert!(tree["sections"][0]["columns"].is_null());
    assert!(tree["sections"][0]["vAlign"].is_null());
    assert!(tree["sections"][0]["lineNumbering"].is_null());
    assert!(tree["sections"][0]["docGrid"].is_null());
    assert!(tree["sections"][0]["rtlGutter"].is_null());
    assert!(tree["mirrorMargins"].is_null());
}

#[test]
fn apply_section_bad_id_and_foreign() {
    let path = write_hf_sample("sect-bad");
    let out = tmp_dir().join("sect-bad-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "setPageMargin", "id": "para:1", "top": 1440 }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "setMirrorMargins", "id": "section:1", "mirrorMargins": true }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
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
    let defaults: Vec<_> = headers.iter().filter(|h| h["type"] == "default").collect();
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
    assert_eq!(
        before_rels,
        part_bytes(&out, "word/_rels/document.xml.rels")
    );
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

// —— 页眉页脚图文混排 ——

/// 1×1 红 PNG。
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// 另一份 1×1 PNG，字节不同，用来换图。
const PNG_1X1_B: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x01, 0xA3, 0x0A, 0x15, 0xE3, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const PIC_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#;

const PIC_DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

const PIC_SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#;

/// 两节：第一节显式 default 页眉，第二节继承。页眉里行内图 + 浮动图，另有一张无 pic 的 drawing 不应进名单。
const PIC_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type="default" r:id="rId3"/></w:sectPr></w:pPr></w:p><w:p><w:r><w:t>二</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const PIC_HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:p w14:paraId="AAAA1001"><w:r><w:t>带图页眉</w:t></w:r><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="9525" cy="9525"/><wp:docPr id="1" name="图片 1"/><wp:cNvGraphicFramePr/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="图片 1"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId1"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9525" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r><w:r><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="19050" cy="9525"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="2" name="图片 2"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="图片 2"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="19050" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r><w:r><w:drawing><wp:inline><wp:extent cx="1" cy="1"/><wp:docPr id="99" name="不是图"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"/></a:graphic></wp:inline></w:drawing></w:r></w:p></w:hdr>"#;

const PIC_HEADER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png"/></Relationships>"#;

fn write_pic_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let texts: &[(&str, &[u8])] = &[
            ("[Content_Types].xml", PIC_CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", PIC_DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", PIC_DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", PIC_SETTINGS.as_bytes()),
            ("word/header1.xml", PIC_HEADER.as_bytes()),
            ("word/_rels/header1.xml.rels", PIC_HEADER_RELS.as_bytes()),
            ("word/media/image1.png", PNG_1X1),
            ("word/media/image2.png", PNG_1X1),
        ];
        for (name, body) in texts {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn inspect_pics(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "pictures": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn write_temp_png(tag: &str, bytes: &[u8]) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.png"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

#[test]
fn inspect_lists_header_pictures() {
    let path = write_pic_sample("pic-list");
    let tree0 = inspect_hf(&path, false);
    assert_eq!(tree0["headers"][0]["pictureCount"], 2);
    assert!(tree0["headers"][0].get("pictures").is_none());

    let tree = inspect_pics(&path);
    let pics = tree["headers"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 2);
    assert_eq!(
        object_keys(&pics[0]),
        ["id", "docPrId", "name", "kind", "cx", "cy", "wrap", "stable"]
    );
    assert_eq!(pics[0]["id"], "header:default:1/pic:1");
    assert_eq!(pics[0]["docPrId"], "1");
    assert_eq!(pics[0]["name"], "图片 1");
    assert_eq!(pics[0]["kind"], "inline");
    assert_eq!(pics[0]["cx"], 9525);
    assert_eq!(pics[0]["cy"], 9525);
    assert!(pics[0]["wrap"].is_null());
    assert_eq!(pics[0]["stable"], true);
    assert_eq!(pics[1]["id"], "header:default:1/pic:2");
    assert_eq!(pics[1]["kind"], "anchor");
    assert_eq!(pics[1]["wrap"], "square");
    assert_eq!(pics[1]["cx"], 19050);
    assert_eq!(
        object_keys(&tree["headers"][0]),
        [
            "id",
            "type",
            "sections",
            "paraCount",
            "pictureCount",
            "textBoxCount",
            "chartCount",
            "smartArtCount",
            "tableCount",
            "pictures"
        ]
    );
}

#[test]
fn unlink_copies_header_rels_and_shares_media() {
    let path = write_pic_sample("pic-unlink");
    let out = tmp_dir().join("pic-unlink-out.docx");
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
    assert!(zip_has_part(&out, "word/header2.xml"));
    assert!(zip_has_part(&out, "word/_rels/header2.xml.rels"));
    assert!(zip_has_part(&out, "word/media/image1.png"));
    let rels = String::from_utf8(part_bytes(&out, "word/_rels/header2.xml.rels")).unwrap();
    assert!(rels.contains("media/image1.png"));
    let tree = inspect_pics(&out);
    let headers = tree["headers"].as_array().unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0]["pictureCount"], 2);
    assert_eq!(headers[1]["pictureCount"], 2);
    let edited = tmp_dir().join("pic-unlink-edit.docx");
    apply_ops(
        &out,
        &edited,
        json!([{
            "op": "setParaText",
            "id": "header:default:2/para:AAAA1001",
            "text": "第二节页眉"
        }]),
    )
    .unwrap();
    let after = inspect_pics(&edited);
    let h1 = after["headers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["id"] == "header:default:1")
        .unwrap();
    let h2 = after["headers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["id"] == "header:default:2")
        .unwrap();
    assert_eq!(h1["pictureCount"], 2);
    assert_eq!(h2["pictureCount"], 2);
}

#[test]
fn replace_picture_keeps_wrap_and_changes_bytes() {
    let path = write_pic_sample("pic-replace");
    let png = write_temp_png("rep", PNG_1X1_B);
    let out = tmp_dir().join("pic-replace-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "replacePicture",
            "id": "header:default:1/pic:1",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&out, "word/media/image1.png"), PNG_1X1_B);
    let tree = inspect_pics(&out);
    let pics = tree["headers"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics[0]["kind"], "inline");
    assert!(pics[0]["wrap"].is_null());
    assert_eq!(pics[1]["kind"], "anchor");
    assert_eq!(pics[1]["wrap"], "square");
}

#[test]
fn insert_and_delete_picture() {
    let path = write_pic_sample("pic-insdel");
    let png = write_temp_png("ins", PNG_1X1);
    let inserted = tmp_dir().join("pic-ins.docx");
    apply_ops(
        &path,
        &inserted,
        json!([{
            "op": "insertPicture",
            "id": "header:default:1/para:AAAA1001",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect_pics(&inserted);
    assert_eq!(tree["headers"][0]["pictureCount"], 3);
    let pics = tree["headers"][0]["pictures"].as_array().unwrap();
    let last = pics.last().unwrap();
    assert_eq!(last["kind"], "inline");
    assert_eq!(last["id"], "header:default:1/pic:100");
    assert!(zip_has_part(&inserted, "word/media/image3.png"));

    let deleted = tmp_dir().join("pic-del.docx");
    apply_ops(
        &inserted,
        &deleted,
        json!([{
            "op": "deletePicture",
            "id": "header:default:1/pic:100"
        }]),
    )
    .unwrap();
    let after = inspect_pics(&deleted);
    assert_eq!(after["headers"][0]["pictureCount"], 2);
    assert!(!zip_has_part(&deleted, "word/media/image3.png"));
}

#[test]
fn set_picture_size_and_wrap() {
    let path = write_pic_sample("pic-sizewrap");
    let out = tmp_dir().join("pic-sizewrap-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            {
                "op": "setPictureSize",
                "id": "header:default:1/pic:1",
                "cx": 19050,
                "cy": 19050
            },
            {
                "op": "setPictureWrap",
                "id": "header:default:1/pic:1",
                "kind": "anchor",
                "wrap": "square",
                "alignH": "right",
                "alignV": "bottom"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_pics(&out);
    let pic = &tree["headers"][0]["pictures"][0];
    assert_eq!(pic["kind"], "anchor");
    assert_eq!(pic["wrap"], "square");
    assert_eq!(pic["cx"], 19050);
    assert_eq!(pic["cy"], 19050);
    let xml = String::from_utf8(part_bytes(&out, "word/header1.xml")).unwrap();
    assert!(xml.contains("wp:anchor"));
    assert!(xml.contains("wrapSquare"));
    assert!(xml.contains(">right<"));
    assert!(xml.contains(">bottom<"));

    let back = tmp_dir().join("pic-inline-out.docx");
    apply_ops(
        &out,
        &back,
        json!([{
            "op": "setPictureWrap",
            "id": "header:default:1/pic:1",
            "kind": "inline"
        }]),
    )
    .unwrap();
    let tree2 = inspect_pics(&back);
    assert_eq!(tree2["headers"][0]["pictures"][0]["kind"], "inline");
    assert!(tree2["headers"][0]["pictures"][0]["wrap"].is_null());
}

#[test]
fn picture_id_errors() {
    let path = write_pic_sample("pic-err");
    let out = tmp_dir().join("pic-err-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "deletePicture", "id": "header:default:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "header:default:1/pic:1",
            "file": "x.png"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "deletePicture", "id": "para:index:2" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_ops(
        &path,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "para:AAAA0001",
            "file": "x.png"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "deletePicture", "id": "header:default:1/pic:9" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

// —— 正文图 ——

const BODY_PIC_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/></Types>"#;

const BODY_PIC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png"/></Relationships>"#;

/// 一段正文：行内图 + 浮动图，另有一张无 pic 的 drawing 不进名单。
const BODY_PIC_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body><w:p w14:paraId="BBBB2001"><w:r><w:t>带图正文</w:t></w:r><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="9525" cy="9525"/><wp:docPr id="1" name="图片 1"/><wp:cNvGraphicFramePr/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="图片 1"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId3"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9525" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r><w:r><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="19050" cy="9525"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="2" name="图片 2"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="0" name="图片 2"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId4"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="19050" cy="9525"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r><w:r><w:drawing><wp:inline><wp:extent cx="1" cy="1"/><wp:docPr id="99" name="不是图"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"/></a:graphic></wp:inline></w:drawing></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

fn write_body_pic_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let texts: &[(&str, &[u8])] = &[
            ("[Content_Types].xml", BODY_PIC_CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", BODY_PIC_DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", BODY_PIC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", PIC_SETTINGS.as_bytes()),
            ("word/media/image1.png", PNG_1X1),
            ("word/media/image2.png", PNG_1X1),
        ];
        for (name, body) in texts {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn inspect_body_pics(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 20, "pictures": true }
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
fn inspect_lists_body_pictures() {
    let path = write_body_pic_sample("body-pic-list");
    let tree0 = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree0) = tree0 else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree0["pictureCount"], 2);
    assert!(tree0.get("paragraphs").is_none());

    let only_pics = inspect_pics(&path);
    assert_eq!(only_pics["pictureCount"], 2);
    assert!(only_pics.get("paragraphs").is_none());

    let tree = inspect_body_pics(&path);
    let p = &tree["paragraphs"][0];
    assert_eq!(p["id"], "para:BBBB2001");
    assert_eq!(p["pictureCount"], 2);
    assert_eq!(p["textBoxCount"], 0);
    let pics = p["pictures"].as_array().unwrap();
    assert_eq!(pics.len(), 2);
    assert_eq!(pics[0]["id"], "para:BBBB2001/pic:1");
    assert_eq!(pics[0]["kind"], "inline");
    assert!(pics[0]["wrap"].is_null());
    assert_eq!(pics[1]["id"], "para:BBBB2001/pic:2");
    assert_eq!(pics[1]["kind"], "anchor");
    assert_eq!(pics[1]["wrap"], "square");
    assert_eq!(
        object_keys(p),
        [
            "id",
            "index",
            "style",
            "text",
            "stable",
            "pictureCount",
            "textBoxCount",
            "pictures"
        ]
    );
}

#[test]
fn replace_body_picture_keeps_wrap() {
    let path = write_body_pic_sample("body-pic-replace");
    let png = write_temp_png("body-rep", PNG_1X1_B);
    let out = tmp_dir().join("body-pic-replace-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "replacePicture",
            "id": "para:BBBB2001/pic:1",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    assert_eq!(part_bytes(&out, "word/media/image1.png"), PNG_1X1_B);
    let tree = inspect_body_pics(&out);
    let pics = tree["paragraphs"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics[0]["kind"], "inline");
    assert_eq!(pics[1]["kind"], "anchor");
    assert_eq!(pics[1]["wrap"], "square");
}

#[test]
fn insert_and_delete_body_picture() {
    let path = write_body_pic_sample("body-pic-insdel");
    let png = write_temp_png("body-ins", PNG_1X1);
    let inserted = tmp_dir().join("body-pic-ins.docx");
    apply_ops(
        &path,
        &inserted,
        json!([{
            "op": "insertPicture",
            "id": "para:BBBB2001",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let tree = inspect_body_pics(&inserted);
    assert_eq!(tree["pictureCount"], 3);
    assert_eq!(tree["paragraphs"][0]["pictureCount"], 3);
    let pics = tree["paragraphs"][0]["pictures"].as_array().unwrap();
    let last = pics.last().unwrap();
    assert_eq!(last["kind"], "inline");
    assert_eq!(last["id"], "para:BBBB2001/pic:100");
    assert!(zip_has_part(&inserted, "word/media/image3.png"));

    let deleted = tmp_dir().join("body-pic-del.docx");
    apply_ops(
        &inserted,
        &deleted,
        json!([{
            "op": "deletePicture",
            "id": "para:BBBB2001/pic:100"
        }]),
    )
    .unwrap();
    let after = inspect_body_pics(&deleted);
    assert_eq!(after["pictureCount"], 2);
    assert!(!zip_has_part(&deleted, "word/media/image3.png"));
}

#[test]
fn set_body_picture_size_and_wrap() {
    let path = write_body_pic_sample("body-pic-sizewrap");
    let out = tmp_dir().join("body-pic-sizewrap-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            {
                "op": "setPictureSize",
                "id": "para:BBBB2001/pic:1",
                "cx": 19050,
                "cy": 19050
            },
            {
                "op": "setPictureWrap",
                "id": "para:BBBB2001/pic:1",
                "kind": "anchor",
                "wrap": "square",
                "alignH": "right",
                "alignV": "bottom"
            }
        ]),
    )
    .unwrap();
    let tree = inspect_body_pics(&out);
    let pic = &tree["paragraphs"][0]["pictures"][0];
    assert_eq!(pic["kind"], "anchor");
    assert_eq!(pic["wrap"], "square");
    assert_eq!(pic["cx"], 19050);
    assert_eq!(pic["cy"], 19050);
}

#[test]
fn set_para_text_keeps_body_pictures() {
    let path = write_body_pic_sample("body-pic-text");
    let out = tmp_dir().join("body-pic-text-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setParaText",
            "id": "para:BBBB2001",
            "text": "改过的正文"
        }]),
    )
    .unwrap();
    let tree = inspect_body_pics(&out);
    assert_eq!(tree["paragraphs"][0]["text"], "改过的正文");
    assert_eq!(tree["paragraphs"][0]["pictureCount"], 2);
    let pics = tree["paragraphs"][0]["pictures"].as_array().unwrap();
    assert_eq!(pics[0]["id"], "para:BBBB2001/pic:1");
    assert_eq!(pics[1]["id"], "para:BBBB2001/pic:2");
}

fn export_pic(path: &Path, id: &str, output: &Path) -> Result<RunOutput, ooxp::Error> {
    run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": path.to_string_lossy(),
            "id": id,
            "output": output.to_string_lossy()
        })
        .to_string(),
        None,
    )
}

#[test]
fn export_header_picture_bytes_match() {
    let path = write_pic_sample("export-hdr");
    let out = tmp_dir().join("export-hdr.png");
    let result = export_pic(&path, "header:default:1/pic:1", &out).unwrap();
    let RunOutput::Written {
        path: written,
        ops_applied,
        content_type,
    } = result
    else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(written, out);
    assert!(ops_applied.is_none());
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(std::fs::read(&out).unwrap(), PNG_1X1);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        part_bytes(&path, "word/media/image1.png")
    );
}

#[test]
fn export_body_picture_bytes_match() {
    let path = write_body_pic_sample("export-body");
    let out = tmp_dir().join("export-body.png");
    let result = export_pic(&path, "para:BBBB2001/pic:2", &out).unwrap();
    let RunOutput::Written { content_type, .. } = result else {
        panic!("exportMedia 应写出文件");
    };
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(std::fs::read(&out).unwrap(), PNG_1X1);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        part_bytes(&path, "word/media/image2.png")
    );
}

#[test]
fn export_after_replace_gets_new_bytes() {
    let path = write_pic_sample("export-rep");
    let png = write_temp_png("export-rep-src", PNG_1X1_B);
    let replaced = tmp_dir().join("export-rep.docx");
    apply_ops(
        &path,
        &replaced,
        json!([{
            "op": "replacePicture",
            "id": "header:default:1/pic:1",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let out = tmp_dir().join("export-rep.png");
    export_pic(&replaced, "header:default:1/pic:1", &out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), PNG_1X1_B);
}

#[test]
fn export_rejects_para_id_and_missing_pic() {
    let path = write_body_pic_sample("export-err");
    let out = tmp_dir().join("export-err.png");
    let err = export_pic(&path, "para:BBBB2001", &out).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = export_pic(&path, "para:BBBB2001/pic:9", &out).unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    let err = export_pic(&path, "header:default:1/pic:1", &out).unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
}

#[test]
fn export_rejects_external_rel() {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("export-ext-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let ext_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://example.com/x.png" TargetMode="External"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png"/></Relationships>"#;
        let texts: &[(&str, &[u8])] = &[
            ("[Content_Types].xml", PIC_CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", PIC_DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", PIC_DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", PIC_SETTINGS.as_bytes()),
            ("word/header1.xml", PIC_HEADER.as_bytes()),
            ("word/_rels/header1.xml.rels", ext_rels.as_bytes()),
            ("word/media/image2.png", PNG_1X1),
        ];
        for (name, body) in texts {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    let out = tmp_dir().join("export-ext.png");
    let err = export_pic(&path, "header:default:1/pic:1", &out).unwrap_err();
    assert_eq!(err.code(), ErrorCode::OpcPartMissing);
}

#[test]
fn export_missing_output_is_invalid_field() {
    let path = write_pic_sample("export-noout");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportMedia",
            "path": path.to_string_lossy(),
            "id": "header:default:1/pic:1"
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

const TXBX_CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#;

const TXBX_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" mc:Ignorable="w14"><w:body><w:p w14:paraId="CCCC3001"><w:r><w:t>带框</w:t></w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="1828800" cy="914400"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="8" name="文本框 1"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp><wps:txbx id="1"><w:txbxContent><w:p w14:paraId="DDDD4001"><w:r><w:t>框内字</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict/></mc:Fallback></mc:AlternateContent></w:p><w:p w14:paraId="CCCC3002"><w:r><w:t>第二框</w:t></w:r><w:drawing><wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="914400" cy="914400"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="9" name="文本框 2"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp><wps:linkedTxbx id="1" seq="1"><w:txbxContent><w:p w14:paraId="DDDD4002"><w:r><w:t>链上</w:t></w:r></w:p></w:txbxContent></wps:linkedTxbx></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

fn write_txbx_sample(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let texts: &[(&str, &[u8])] = &[
            ("[Content_Types].xml", TXBX_CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", TXBX_DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
        ];
        for (name, body) in texts {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

#[test]
fn inspect_lists_body_textboxes() {
    let path = write_txbx_sample("txbx-list");
    let tree0 = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree0) = tree0 else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree0["textBoxCount"], 2);
    assert!(tree0.get("paragraphs").is_none());

    let only_box = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "textBoxes": true }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(only_box) = only_box else {
        panic!();
    };
    assert_eq!(only_box["textBoxCount"], 2);
    assert!(only_box.get("paragraphs").is_none());

    let tree = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 10, "textBoxes": true, "textBoxContent": true }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree) = tree else {
        panic!();
    };
    let p0 = &tree["paragraphs"][0];
    assert_eq!(p0["id"], "para:CCCC3001");
    assert_eq!(p0["text"], "带框");
    assert_eq!(p0["textBoxCount"], 1);
    let boxes = p0["textBoxes"].as_array().unwrap();
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0]["id"], "para:CCCC3001/txbx:8");
    assert_eq!(boxes[0]["kind"], "anchor");
    assert_eq!(boxes[0]["wrap"], "square");
    assert_eq!(boxes[0]["link"]["prev"], serde_json::Value::Null);
    assert_eq!(boxes[0]["link"]["next"], "para:CCCC3002/txbx:9");
    assert_eq!(boxes[0]["paraCount"], 1);
    assert_eq!(
        boxes[0]["paragraphs"][0]["id"],
        "para:CCCC3001/txbx:8/para:DDDD4001"
    );
    assert_eq!(boxes[0]["paragraphs"][0]["text"], "框内字");

    let p1 = &tree["paragraphs"][1];
    assert_eq!(p1["textBoxCount"], 1);
    assert_eq!(p1["textBoxes"][0]["id"], "para:CCCC3002/txbx:9");
    assert_eq!(p1["textBoxes"][0]["link"]["prev"], "para:CCCC3001/txbx:8");
    assert!(p1["textBoxes"][0]["link"]["next"].is_null());
}

#[test]
fn inspect_text_box_content_requires_text_boxes() {
    let path = write_txbx_sample("txbx-scope");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "textBoxContent": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

fn inspect_boxes(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 20, "textBoxes": true, "textBoxContent": true }
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
fn set_para_text_inside_textbox() {
    let path = write_txbx_sample("txbx-set");
    let out = tmp_dir().join("txbx-set-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setParaText",
            "id": "para:CCCC3001/txbx:8/para:DDDD4001",
            "text": "改过的框内"
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&out);
    assert_eq!(tree["paragraphs"][0]["text"], "带框");
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["text"],
        "改过的框内"
    );
}

#[test]
fn set_para_text_on_box_id_is_invalid() {
    let path = write_txbx_sample("txbx-set-box");
    let out = tmp_dir().join("txbx-set-box-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "setParaText", "id": "para:CCCC3001/txbx:8", "text": "x" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn insert_and_delete_textbox() {
    let path = write_txbx_sample("txbx-ins");
    let out = tmp_dir().join("txbx-ins-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "insertTextBox",
            "id": "para:CCCC3002",
            "text": "新框"
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&out);
    assert_eq!(tree["textBoxCount"], 3);
    let boxes = tree["paragraphs"][1]["textBoxes"].as_array().unwrap();
    assert_eq!(boxes.len(), 2);
    let new_id = boxes
        .iter()
        .find(|b| b["paragraphs"][0]["text"] == "新框")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out2 = tmp_dir().join("txbx-del-out.docx");
    apply_ops(
        &out,
        &out2,
        json!([{ "op": "deleteTextBox", "id": new_id }]),
    )
    .unwrap();
    let tree2 = inspect_boxes(&out2);
    assert_eq!(tree2["textBoxCount"], 2);
    assert_eq!(tree2["paragraphs"][1]["textBoxCount"], 1);
}

#[test]
fn set_textbox_size_fill_direction() {
    let path = write_txbx_sample("txbx-style");
    let out = tmp_dir().join("txbx-style-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            { "op": "setTextBoxSize", "id": "para:CCCC3001/txbx:8", "cx": 2000000, "cy": 1000000 },
            { "op": "setTextBoxFill", "id": "para:CCCC3001/txbx:8", "color": "FF0000" },
            { "op": "setTextBoxLine", "id": "para:CCCC3001/txbx:8", "color": "00FF00", "width": 12700 },
            { "op": "setTextBoxDirection", "id": "para:CCCC3001/txbx:8", "direction": "vertical" },
            { "op": "setTextBoxWrap", "id": "para:CCCC3001/txbx:8", "kind": "inline" }
        ]),
    )
    .unwrap();
    let tree = inspect_boxes(&out);
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["cx"], 2000000);
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["cy"], 1000000);
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["kind"], "inline");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("eaVert"));
}

#[test]
fn insert_and_delete_para_in_textbox() {
    let path = write_txbx_sample("txbx-para");
    let out = tmp_dir().join("txbx-para-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "insertPara",
            "id": "para:CCCC3001/txbx:8",
            "text": "第二段"
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&out);
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["paraCount"], 2);
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][1]["text"],
        "第二段"
    );
    let second = tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let out2 = tmp_dir().join("txbx-para-del.docx");
    apply_ops(&out, &out2, json!([{ "op": "deletePara", "id": second }])).unwrap();
    let tree2 = inspect_boxes(&out2);
    assert_eq!(tree2["paragraphs"][0]["textBoxes"][0]["paraCount"], 1);
    let last = tree2["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let err = apply_ops(
        &out2,
        &tmp_dir().join("txbx-para-last.docx"),
        json!([{ "op": "deletePara", "id": last }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_textbox_link_unlink() {
    let path = write_txbx_sample("txbx-link");
    let out = tmp_dir().join("txbx-unlink.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "setTextBoxLink",
            "id": "para:CCCC3001/txbx:8",
            "next": null
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&out);
    assert!(tree["paragraphs"][0]["textBoxes"][0]["link"]["next"].is_null());
    assert!(tree["paragraphs"][1]["textBoxes"][0]["link"]["prev"].is_null());
    let out2 = tmp_dir().join("txbx-relink.docx");
    apply_ops(
        &out,
        &out2,
        json!([{
            "op": "setTextBoxLink",
            "id": "para:CCCC3001/txbx:8",
            "next": "para:CCCC3002/txbx:9"
        }]),
    )
    .unwrap();
    let tree2 = inspect_boxes(&out2);
    assert_eq!(
        tree2["paragraphs"][0]["textBoxes"][0]["link"]["next"],
        "para:CCCC3002/txbx:9"
    );
    assert_eq!(
        tree2["paragraphs"][1]["textBoxes"][0]["link"]["prev"],
        "para:CCCC3001/txbx:8"
    );
}

#[test]
fn set_textbox_link_empty_overflow() {
    let xml = TXBX_DOCUMENT.replace(
        "<wps:linkedTxbx id=\"1\" seq=\"1\"><w:txbxContent><w:p w14:paraId=\"DDDD4002\"><w:r><w:t>链上</w:t></w:r></w:p></w:txbxContent></wps:linkedTxbx>",
        "<wps:linkedTxbx id=\"1\" seq=\"1\"/>",
    );
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("txbx-overflow-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", TXBX_CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", xml.as_str()),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    let unlinked = tmp_dir().join("txbx-overflow-unlink.docx");
    apply_ops(
        &path,
        &unlinked,
        json!([{
            "op": "setTextBoxLink",
            "id": "para:CCCC3001/txbx:8",
            "next": null
        }]),
    )
    .unwrap();
    let relinked = tmp_dir().join("txbx-overflow-relink.docx");
    apply_ops(
        &unlinked,
        &relinked,
        json!([{
            "op": "setTextBoxLink",
            "id": "para:CCCC3001/txbx:8",
            "next": "para:CCCC3002/txbx:9"
        }]),
    )
    .unwrap();
    let tree = inspect_boxes(&relinked);
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["link"]["next"],
        "para:CCCC3002/txbx:9"
    );
    assert_eq!(
        tree["paragraphs"][1]["textBoxes"][0]["link"]["prev"],
        "para:CCCC3001/txbx:8"
    );
}

#[test]
fn inspect_expands_group_textboxes() {
    const GROUP_DOC: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup" mc:Ignorable="w14"><w:body><w:p w14:paraId="EEEE5001"><w:r><w:t>组合</w:t></w:r><w:drawing><wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="margin"><wp:align>left</wp:align></wp:positionH><wp:positionV relativeFrom="margin"><wp:align>top</wp:align></wp:positionV><wp:extent cx="2000000" cy="1000000"/><wp:wrapSquare wrapText="bothSides"/><wp:docPr id="10" name="组合 1"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wpg:wgp><wps:wsp><wps:cNvPr id="11" name="框A"/><wps:txbx><w:txbxContent><w:p w14:paraId="FFFF6001"><w:r><w:t>组内</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></wpg:wgp></a:graphicData></a:graphic></wp:anchor></w:drawing></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("txbx-group-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", TXBX_CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", GROUP_DOC),
            ("word/_rels/document.xml.rels", DOC_RELS),
            ("word/styles.xml", STYLES),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    let tree = inspect_boxes(&path);
    assert_eq!(tree["textBoxCount"], 1);
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["id"],
        "para:EEEE5001/txbx:11"
    );
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["text"],
        "组内"
    );
}

#[test]
fn create_document_with_textbox() {
    let path = tmp_dir().join(format!(
        "txbx-create-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [
                    { "text": "外层", "textBoxes": [{ "text": "框字", "cx": 1828800, "cy": 914400 }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_boxes(&path);
    assert_eq!(tree["textBoxCount"], 1);
    assert_eq!(tree["paragraphs"][0]["text"], "外层");
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["kind"], "anchor");
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["wrap"], "square");
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["text"],
        "框字"
    );
}

fn create_docx_path(tag: &str) -> PathBuf {
    tmp_dir().join(format!(
        "{tag}-{}.docx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn create_body_inline_picture() {
    let png = write_temp_png("create-body", PNG_1X1);
    let path = create_docx_path("create-body-pic");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [
                    { "style": "Heading1", "text": "标题" },
                    { "text": "hello", "pictures": [{ "file": png.to_string_lossy() }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_body_pics(&path);
    assert_eq!(tree["pictureCount"], 1);
    assert_eq!(tree["paragraphs"][1]["text"], "hello");
    assert_eq!(tree["paragraphs"][1]["pictureCount"], 1);
    let pic = &tree["paragraphs"][1]["pictures"][0];
    assert_eq!(pic["id"], "para:00000002/pic:1");
    assert_eq!(pic["kind"], "inline");
    assert!(pic["wrap"].is_null());
    assert_eq!(pic["stable"], true);
    assert_eq!(pic["cx"], 9525);
    assert_eq!(pic["cy"], 9525);
    assert!(zip_has_part(&path, "word/media/image1.png"));
    assert_eq!(part_bytes(&path, "word/media/image1.png"), PNG_1X1);
}

#[test]
fn create_header_anchor_and_mixed_body() {
    let png = write_temp_png("create-hf", PNG_1X1);
    let path = create_docx_path("create-hf-pic");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [
                    { "text": "甲", "pictures": [{ "file": png.to_string_lossy() }] },
                    {
                        "text": "乙",
                        "pictures": [
                            { "file": png.to_string_lossy(), "kind": "anchor", "wrap": "square" },
                            { "file": png.to_string_lossy(), "cx": 19050, "cy": 9525 }
                        ]
                    }
                ],
                "headers": [{
                    "type": "default",
                    "paragraphs": [{
                        "text": "页眉",
                        "pictures": [{ "file": png.to_string_lossy(), "kind": "anchor", "wrap": "square" }]
                    }]
                }],
                "footers": [{
                    "paragraphs": [{ "text": "页脚" }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 20, "pictures": true, "headerFooter": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["pictureCount"], 3);
    assert_eq!(tree["headers"][0]["id"], "header:default:1");
    assert_eq!(tree["headers"][0]["pictureCount"], 1);
    assert_eq!(
        tree["headers"][0]["pictures"][0]["id"],
        "header:default:1/pic:1"
    );
    assert_eq!(tree["headers"][0]["pictures"][0]["kind"], "anchor");
    assert_eq!(tree["headers"][0]["pictures"][0]["wrap"], "square");
    assert_eq!(tree["headers"][0]["paragraphs"][0]["text"], "页眉");
    assert_eq!(tree["footers"][0]["id"], "footer:default:1");
    assert_eq!(tree["footers"][0]["paragraphs"][0]["text"], "页脚");
    assert_eq!(
        tree["paragraphs"][0]["pictures"][0]["id"],
        "para:00000001/pic:1"
    );
    assert_eq!(tree["paragraphs"][0]["pictures"][0]["kind"], "inline");
    assert_eq!(tree["paragraphs"][1]["pictureCount"], 2);
    assert_eq!(tree["paragraphs"][1]["pictures"][0]["kind"], "anchor");
    assert_eq!(tree["paragraphs"][1]["pictures"][0]["wrap"], "square");
    assert_eq!(
        tree["paragraphs"][1]["pictures"][0]["id"],
        "para:00000002/pic:2"
    );
    assert_eq!(tree["paragraphs"][1]["pictures"][1]["kind"], "inline");
    assert_eq!(
        tree["paragraphs"][1]["pictures"][1]["id"],
        "para:00000002/pic:3"
    );
    assert_eq!(tree["paragraphs"][1]["pictures"][1]["cx"], 19050);
    assert!(zip_has_part(&path, "word/header1.xml"));
    assert!(zip_has_part(&path, "word/footer1.xml"));
    assert!(zip_has_part(&path, "word/media/image1.png"));
}

#[test]
fn create_pictures_rejects_missing_file_and_one_size() {
    let png = write_temp_png("create-bad", PNG_1X1);
    let path = create_docx_path("create-pic-miss");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [{ "text": "a", "pictures": [{}] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(
        err.field_path(),
        Some("document.paragraphs[0].pictures[0].file")
    );

    let path2 = create_docx_path("create-pic-size");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path2.to_string_lossy(),
            "document": {
                "paragraphs": [{
                    "text": "a",
                    "pictures": [{ "file": png.to_string_lossy(), "cx": 9525 }]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("document.paragraphs[0].pictures[0]"));

    let path3 = create_docx_path("create-hf-even");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path3.to_string_lossy(),
            "document": {
                "paragraphs": [{ "text": "a" }],
                "headers": [{ "type": "even", "paragraphs": [{ "text": "偶" }] }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_hf(&path3, true);
    assert_eq!(tree["headers"][0]["type"], "even");
    assert_eq!(tree["headers"][0]["paragraphs"][0]["text"], "偶");
    assert_eq!(tree["sections"][0]["headerLink"]["even"], false);
    assert!(tree["sections"][0]["headerLink"]["first"].is_null());
    assert!(part(&path3, "word/settings.xml").contains("evenAndOddHeaders"));

    let path4 = create_docx_path("create-hf-dup");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path4.to_string_lossy(),
            "document": {
                "paragraphs": [{ "text": "a" }],
                "headers": [
                    { "type": "default", "paragraphs": [{ "text": "一" }] },
                    { "type": "default", "paragraphs": [{ "text": "二" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    assert_eq!(err.field_path(), Some("document.headers"));
}

#[test]
fn insert_picture_inside_textbox() {
    let path = write_txbx_sample("txbx-pic");
    let png = write_temp_png("txbx-pic", PNG_1X1);
    let out = tmp_dir().join("txbx-pic-out.docx");
    apply_ops(
        &path,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "para:CCCC3001/txbx:8/para:DDDD4001",
            "file": png.to_string_lossy()
        }]),
    )
    .unwrap();
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": out.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 20, "pictures": true, "textBoxes": true, "textBoxContent": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    assert_eq!(tree["paragraphs"][0]["pictureCount"], 0);
    assert!(tree["paragraphs"][0]["pictures"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true));
    assert_eq!(tree["paragraphs"][0]["textBoxes"][0]["pictureCount"], 1);
    assert_eq!(
        tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictureCount"],
        1
    );
    let pic_id = tree["paragraphs"][0]["textBoxes"][0]["paragraphs"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(pic_id.starts_with("para:CCCC3001/txbx:8/para:DDDD4001/pic:"));
    let exported = tmp_dir().join("txbx-pic-export.png");
    export_pic(&out, &pic_id, &exported).unwrap();
    assert_eq!(std::fs::read(&exported).unwrap(), PNG_1X1);
}

#[test]
fn insert_and_delete_body_para() {
    let path = write_sample("body-para");
    let before_styles = part_bytes(&path, "word/styles.xml");
    let out = tmp_dir().join("body-para-ins.docx");
    apply_ops(
        &path,
        &out,
        json!([{ "op": "insertPara", "id": "para:AAAA0001", "text": "新段" }]),
    )
    .unwrap();
    assert_eq!(before_styles, part_bytes(&out, "word/styles.xml"));
    let tree = inspect_all(&out);
    let texts: Vec<&str> = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts[0], "标题");
    assert_eq!(texts[1], "新段");
    assert_eq!(texts[2], "粗细");
    let xml = part(&out, "word/document.xml");
    assert!(xml.contains("w14:paraId"));
    assert!(xml.contains("新段"));

    let out_end = tmp_dir().join("body-para-end.docx");
    apply_ops(
        &path,
        &out_end,
        json!([{ "op": "insertPara", "id": "docx:document", "text": "末段" }]),
    )
    .unwrap();
    let tree_end = inspect_all(&out_end);
    let last = tree_end["paragraphs"].as_array().unwrap().last().unwrap();
    assert_eq!(last["text"], "末段");
    let xml_end = part(&out_end, "word/document.xml");
    let ins = xml_end.find(">末段<").unwrap();
    let sect = xml_end.find("<w:sectPr>").unwrap();
    assert!(ins < sect, "新段必须写在 body 级 sectPr 前面");

    let out_del = tmp_dir().join("body-para-del.docx");
    apply_ops(
        &path,
        &out_del,
        json!([{ "op": "deletePara", "id": "para:AAAA0006" }]),
    )
    .unwrap();
    let tree_del = inspect_all(&out_del);
    let texts_del: Vec<&str> = tree_del["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect();
    assert!(!texts_del.contains(&"外层"));
    assert!(texts_del.contains(&"标题"));
}

#[test]
fn delete_body_para_guards() {
    let path = write_sample("body-para-guard");
    let err = apply_ops(
        &path,
        &tmp_dir().join("body-rev.docx"),
        json!([{ "op": "deletePara", "id": "para:AAAA0004" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);

    let err = apply_ops(
        &path,
        &tmp_dir().join("body-cell-last.docx"),
        json!([{ "op": "deletePara", "id": "para:AAAA0005" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let out = tmp_dir().join("body-cell-ins.docx");
    apply_ops(
        &path,
        &out,
        json!([{ "op": "insertPara", "id": "para:AAAA0005", "text": "表内二" }]),
    )
    .unwrap();
    let tree = inspect_all(&out);
    let texts: Vec<&str> = tree["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["text"].as_str().unwrap())
        .collect();
    let i = texts.iter().position(|t| *t == "表内").unwrap();
    assert_eq!(texts[i + 1], "表内二");
}

#[test]
fn insert_header_para_and_refuse_last_delete() {
    let path = write_hf_sample("hf-para");
    let out = tmp_dir().join("hf-para-ins.docx");
    apply_ops(
        &path,
        &out,
        json!([{ "op": "insertPara", "id": "header:default:1", "text": "页眉第二段" }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, true);
    let h = tree["headers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["type"] == "default")
        .unwrap();
    assert_eq!(h["paraCount"], 2);
    assert_eq!(h["paragraphs"][1]["text"], "页眉第二段");

    let err = apply_ops(
        &path,
        &tmp_dir().join("hf-para-last.docx"),
        json!([{ "op": "deletePara", "id": "header:default:1/para:AAAA1001" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply_ops(
        &path,
        &tmp_dir().join("hf-para-container-del.docx"),
        json!([{ "op": "deletePara", "id": "header:default:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);

    let out2 = tmp_dir().join("body-last.docx");
    apply_ops(
        &path,
        &out2,
        json!([{ "op": "deletePara", "id": "para:index:2" }]),
    )
    .unwrap();
    let err = apply_ops(
        &out2,
        &tmp_dir().join("body-last-2.docx"),
        json!([{ "op": "deletePara", "id": "para:index:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply_ops(
        &path,
        &tmp_dir().join("body-sect-break.docx"),
        json!([{ "op": "deletePara", "id": "para:index:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn insert_header_footer_on_blank_and_guards() {
    let src = create_docx_path("nh-src");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": src.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "正文" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();

    let err = apply_ops(
        &src,
        &tmp_dir().join("nh-even-off.docx"),
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:1",
            "kind": "header",
            "type": "even"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let out = tmp_dir().join("nh-insert-default.docx");
    apply_ops(
        &src,
        &out,
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:1",
            "kind": "header",
            "type": "default"
        }]),
    )
    .unwrap();
    let tree = inspect_hf(&out, true);
    assert_eq!(tree["headers"].as_array().unwrap().len(), 1);
    assert_eq!(tree["headers"][0]["id"], "header:default:1");
    assert_eq!(tree["headers"][0]["paraCount"], 1);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert!(zip_has_part(&out, "word/header1.xml"));

    let again = tmp_dir().join("nh-insert-again.docx");
    apply_ops(
        &out,
        &again,
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:1",
            "kind": "header",
            "type": "default"
        }]),
    )
    .unwrap();
    assert_eq!(
        inspect_hf(&again, false)["headers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let pid = inspect_hf(&out, true)["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let edited = tmp_dir().join("nh-insert-edit.docx");
    apply(&out, &edited, &pid, "新页眉").unwrap();
    assert_eq!(
        inspect_hf(&edited, true)["headers"][0]["paragraphs"][0]["text"],
        "新页眉"
    );
}

#[test]
fn title_page_and_even_odd_flags() {
    let src = create_docx_path("nh-flags-src");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": src.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "正文" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();

    let out = tmp_dir().join("nh-flags.docx");
    apply_ops(
        &src,
        &out,
        json!([
            { "op": "setTitlePage", "id": "section:1", "titlePage": true },
            { "op": "setEvenAndOddHeaders", "id": "docx:document", "evenAndOdd": true },
            { "op": "insertHeaderFooter", "id": "section:1", "kind": "header", "type": "first" },
            { "op": "insertHeaderFooter", "id": "section:1", "kind": "header", "type": "even" }
        ]),
    )
    .unwrap();
    let tree = inspect_hf(&out, true);
    assert_eq!(tree["sections"][0]["headerLink"]["first"], false);
    assert_eq!(tree["sections"][0]["headerLink"]["even"], false);
    let types: Vec<_> = tree["headers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"first"));
    assert!(types.contains(&"even"));
    assert!(part(&out, "word/settings.xml").contains("evenAndOddHeaders"));
    assert!(part(&out, "word/document.xml").contains("titlePg"));

    let off = tmp_dir().join("nh-flags-off.docx");
    apply_ops(
        &out,
        &off,
        json!([
            { "op": "setTitlePage", "id": "section:1", "titlePage": false },
            { "op": "setEvenAndOddHeaders", "id": "docx:document", "evenAndOdd": false }
        ]),
    )
    .unwrap();
    let after = inspect_hf(&off, false);
    assert!(after["sections"][0]["headerLink"]["first"].is_null());
    assert!(after["sections"][0]["headerLink"]["even"].is_null());
    assert!(!part(&off, "word/settings.xml").contains("evenAndOddHeaders"));
}

#[test]
fn insert_header_refuses_linked_section() {
    let path = write_hf_sample("nh-linked");
    let err = apply_ops(
        &path,
        &tmp_dir().join("nh-linked-out.docx"),
        json!([{
            "op": "insertHeaderFooter",
            "id": "section:2",
            "kind": "header",
            "type": "default"
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let err = apply_ops(
        &path,
        &tmp_dir().join("nh-evenodd-id.docx"),
        json!([{
            "op": "setEvenAndOddHeaders",
            "id": "section:1",
            "evenAndOdd": true
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn create_first_and_even_headers() {
    let path = create_docx_path("create-hf-both");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [{ "text": "a" }],
                "headers": [
                    { "type": "default", "paragraphs": [{ "text": "奇" }] },
                    { "type": "even", "paragraphs": [{ "text": "偶" }] },
                    { "type": "first", "paragraphs": [{ "text": "首" }] }
                ]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_hf(&path, true);
    assert_eq!(tree["headers"].as_array().unwrap().len(), 3);
    assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
    assert_eq!(tree["sections"][0]["headerLink"]["even"], false);
    assert_eq!(tree["sections"][0]["headerLink"]["first"], false);
    let texts: Vec<_> = tree["headers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| {
            (
                h["type"].as_str().unwrap(),
                h["paragraphs"][0]["text"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(texts.contains(&("default", "奇")));
    assert!(texts.contains(&("even", "偶")));
    assert!(texts.contains(&("first", "首")));
    assert!(part(&path, "word/settings.xml").contains("evenAndOddHeaders"));
    assert!(part(&path, "word/document.xml").contains("titlePg"));
}

fn inspect_tables(path: &Path) -> serde_json::Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "tables": true, "from": 1, "maxParas": 200 }
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
fn inspect_tables_lists_cell_paragraphs() {
    let path = write_sample("tbl-list");
    let bare = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap();
    let RunOutput::Inspect(bare) = bare else {
        panic!();
    };
    assert_eq!(bare["tableCount"], 1);
    assert!(bare.get("tables").is_none());
    let tree = inspect_tables(&path);
    assert_eq!(tree["tableCount"], 1);
    let t = &tree["tables"][0];
    assert_eq!(t["id"], "table:1");
    assert_eq!(t["index"], 1);
    assert_eq!(t["rowCount"], 1);
    assert_eq!(t["colCount"], 1);
    assert_eq!(t["nested"], false);
    assert_eq!(t["rows"][0]["id"], "table:1/row:1");
    let cell = &t["rows"][0]["cells"][0];
    assert_eq!(cell["id"], "table:1/r1c1");
    assert_eq!(cell["gridSpan"], 1);
    assert!(cell["vMerge"].is_null());
    assert_eq!(cell["paragraphs"][0]["id"], "para:AAAA0005");
    assert_eq!(cell["paragraphs"][0]["text"], "表内");
}

#[test]
fn apply_insert_row_col_and_delete_table() {
    let path = write_sample("tbl-ops");
    let mid = tmp_dir().join("tbl-ops-mid.docx");
    apply_ops(
        &path,
        &mid,
        json!([
            { "op": "insertRow", "id": "table:1" },
            { "op": "insertCol", "id": "table:1", "after": 1 }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&mid);
    assert_eq!(tree["tables"][0]["rowCount"], 2);
    assert_eq!(tree["tables"][0]["colCount"], 2);
    let out = tmp_dir().join("tbl-ops-del.docx");
    apply_ops(
        &mid,
        &out,
        json!([{ "op": "deleteTable", "id": "table:1" }]),
    )
    .unwrap();
    let gone = inspect_tables(&out);
    assert_eq!(gone["tableCount"], 0);
    assert_eq!(gone["tables"], json!([]));
}

#[test]
fn apply_insert_table_then_merge() {
    let path = write_sample("tbl-merge");
    let out = tmp_dir().join("tbl-merge-out.docx");
    apply_ops(
        &path,
        &out,
        json!([
            {
                "op": "insertTable",
                "id": "docx:document",
                "rows": 2,
                "cols": 2,
                "cells": [["A1", "B1"], ["A2", "B2"]]
            },
            {
                "op": "mergeCells",
                "id": "table:2",
                "startRow": 1,
                "startCol": 1,
                "endRow": 1,
                "endCol": 2
            }
        ]),
    )
    .unwrap();
    let tree = inspect_tables(&out);
    assert_eq!(tree["tableCount"], 2);
    let t = &tree["tables"][1];
    assert_eq!(t["id"], "table:2");
    assert_eq!(t["rowCount"], 2);
    assert_eq!(t["colCount"], 2);
    assert_eq!(t["rows"][0]["cells"].as_array().unwrap().len(), 1);
    assert_eq!(t["rows"][0]["cells"][0]["gridSpan"], 2);
    assert_eq!(t["rows"][0]["cells"][0]["paragraphs"][0]["text"], "A1");
}

#[test]
fn delete_last_table_row_is_invalid_field() {
    let path = write_sample("tbl-last-row");
    let out = tmp_dir().join("tbl-last-row-out.docx");
    let err = apply_ops(
        &path,
        &out,
        json!([{ "op": "deleteRow", "id": "table:1/row:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn create_document_with_table() {
    let path = create_docx_path("create-tbl");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [{ "text": "前" }],
                "tables": [{
                    "cells": [["A1", "B1"], ["A2", "B2"]]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree = inspect_tables(&path);
    assert_eq!(tree["tableCount"], 1);
    assert_eq!(tree["tables"][0]["rowCount"], 2);
    assert_eq!(tree["tables"][0]["colCount"], 2);
    assert_eq!(
        tree["tables"][0]["rows"][0]["cells"][0]["paragraphs"][0]["text"],
        "A1"
    );
    assert_eq!(tree["paragraphs"][0]["text"], "前");
}
