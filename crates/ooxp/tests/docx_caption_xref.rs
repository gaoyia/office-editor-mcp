//! docx 题注与交叉引用：insertCaption / insertCrossRef、inspect 计数、xlsx/pptx 拒绝。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::{ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-caption-xref");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    })
    .to_string())
    .unwrap();
    path
}

fn inspect(path: &Path, captions: bool, cross_refs: bool, fields: bool) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": {
            "from": 1,
            "maxParas": 20,
            "captions": captions,
            "crossRefs": cross_refs,
            "fields": fields,
            "headerFooter": true
        }
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(path: &Path, out: &Path, ops: Value) {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap();
}

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": path.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
    .unwrap_err()
}

fn first_body_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(|k| k.as_str()).collect()
}

#[test]
fn insert_caption_lists_and_delete_keeps_heading() {
    let src = create_docx(
        "cap-src",
        json!({
            "paragraphs": [
                { "style": "Heading1", "text": "第一章" },
                { "text": "hello" }
            ]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["captionCount"], 0);
    assert_eq!(bare["crossRefCount"], 0);
    assert!(bare.get("captions").is_none());
    assert!(bare.get("crossRefs").is_none());

    let tree0 = inspect(&src, false, false, false);
    assert_eq!(tree0["captionCount"], 0);
    assert!(tree0.get("captions").is_none());
    let p1 = first_body_para_id(&tree0);
    let out = seq_path("cap-out", "docx");
    apply(
        &src,
        &out,
        json!([{ "op": "insertCaption", "id": p1, "label": "Figure", "title": "示意图" }]),
    );
    let tree = inspect(&out, true, true, true);
    assert_eq!(tree["captionCount"], 1);
    assert_eq!(tree["crossRefCount"], 0);
    assert_eq!(tree["bookmarkCount"], 0);
    assert_eq!(tree["fieldCount"], 1);
    assert_eq!(
        object_keys(&tree["captions"][0]),
        [
            "id",
            "index",
            "field",
            "instr",
            "identifier",
            "bookmark",
            "host",
            "end",
            "text"
        ]
    );
    assert_eq!(tree["captions"][0]["id"], "caption:1");
    assert_eq!(tree["captions"][0]["identifier"], "Figure");
    assert_eq!(tree["captions"][0]["field"], "field:1");
    let bm = tree["captions"][0]["bookmark"].as_str().unwrap();
    assert!(bm.starts_with("_Ref"));
    assert_eq!(bm.len(), 13);
    assert_eq!(tree["fields"][0]["code"], "SEQ");
    assert_eq!(tree["paragraphs"][0]["text"], "第一章");

    let xref_out = seq_path("xref-out", "docx");
    apply(
        &out,
        &xref_out,
        json!([{ "op": "insertCrossRef", "id": p1, "bookmark": bm, "kind": "ref" }]),
    );
    let tree_x = inspect(&xref_out, true, true, true);
    assert_eq!(tree_x["crossRefCount"], 1);
    assert_eq!(
        object_keys(&tree_x["crossRefs"][0]),
        ["id", "index", "kind", "field", "instr", "bookmark", "host", "end", "text"]
    );
    assert_eq!(tree_x["crossRefs"][0]["id"], "xref:1");
    assert_eq!(tree_x["crossRefs"][0]["kind"], "ref");
    assert_eq!(tree_x["crossRefs"][0]["bookmark"], bm);
    assert_eq!(tree_x["fields"][0]["code"], "REF");

    let gone_x = seq_path("xref-del", "docx");
    apply(
        &xref_out,
        &gone_x,
        json!([{ "op": "deleteCrossRef", "id": "xref:1" }]),
    );
    let tree_dx = inspect(&gone_x, true, true, true);
    assert_eq!(tree_dx["crossRefCount"], 0);
    assert_eq!(tree_dx["captionCount"], 1);
    assert_eq!(tree_dx["paragraphs"][0]["text"], "第一章");

    let gone = seq_path("cap-del", "docx");
    apply(
        &gone_x,
        &gone,
        json!([{ "op": "deleteCaption", "id": "caption:1" }]),
    );
    let tree2 = inspect(&gone, true, true, true);
    assert_eq!(tree2["captionCount"], 0);
    assert_eq!(tree2["fieldCount"], 0);
    let texts: Vec<&str> = tree2["paragraphs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect();
    assert!(texts.contains(&"第一章"));
    assert!(texts.contains(&"hello"));
    assert!(!texts.iter().any(|t| t.contains("示意图")));
}

#[test]
fn insert_caption_rejects_bad_id_and_label() {
    let src = create_docx(
        "bad-src",
        json!({
            "paragraphs": [{ "text": "甲" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let tree = inspect(&src, false, false, false);
    assert_eq!(tree["headers"][0]["captionCount"], 0);
    let hf_para = tree["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let hf_out = seq_path("hf-cap", "docx");
    apply(
        &src,
        &hf_out,
        json!([{ "op": "insertCaption", "id": hf_para, "label": "Figure", "title": "页眉图" }]),
    );
    let hf_tree = inspect(&hf_out, true, false, false);
    assert_eq!(hf_tree["captionCount"], 0);
    assert_eq!(hf_tree["headers"][0]["captionCount"], 1);
    assert!(hf_tree["headers"][0]["captions"][0]["id"]
        .as_str()
        .unwrap()
        .starts_with("header:default:1/caption:"));

    let err_label = apply_err(
        &src,
        json!([{ "op": "insertCaption", "id": "docx:document", "label": "1Figure" }]),
    );
    assert_eq!(err_label.code(), ErrorCode::InvalidField);

    let err_kind = apply_err(
        &src,
        json!([{ "op": "insertCrossRef", "id": first_body_para_id(&tree), "bookmark": "x", "kind": "nope" }]),
    );
    assert_eq!(err_kind.code(), ErrorCode::InvalidField);

    let err_bm = apply_err(
        &src,
        json!([{ "op": "insertCrossRef", "id": first_body_para_id(&tree), "bookmark": "_missing" }]),
    );
    assert_eq!(err_bm.code(), ErrorCode::IdNotFound);

    let err_del = apply_err(&src, json!([{ "op": "deleteCaption", "id": "caption:1" }]));
    assert_eq!(err_del.code(), ErrorCode::IdNotFound);

    let err_id = apply_err(&src, json!([{ "op": "deleteCaption", "id": "field:1" }]));
    assert_eq!(err_id.code(), ErrorCode::InvalidId);

    let err_xref = apply_err(&src, json!([{ "op": "deleteCrossRef", "id": "xref:1" }]));
    assert_eq!(err_xref.code(), ErrorCode::IdNotFound);
}

#[test]
fn caption_ops_not_for_xlsx_or_pptx() {
    let xlsx = seq_path("cap-x", "xlsx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1" }] }
    })
    .to_string())
    .unwrap();
    let xout = seq_path("cap-x-out", "xlsx");
    let err_x = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": xout.to_string_lossy(),
        "ops": [{ "op": "insertCaption", "id": "docx:document" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_x.code(), ErrorCode::InvalidField);

    let pptx = seq_path("cap-p", "pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    })
    .to_string())
    .unwrap();
    let pout = seq_path("cap-p-out", "pptx");
    let err_p = run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": pout.to_string_lossy(),
        "ops": [{ "op": "deleteCrossRef", "id": "xref:1" }]
    })
    .to_string())
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
}

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

fn write_docx_xml(tag: &str, document: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CT),
            ("_rels/.rels", ROOT_RELS),
            ("word/document.xml", document),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

#[test]
fn nested_seq_and_pageref_in_toc_are_not_listed() {
    let src = write_docx_xml(
        "nested",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC \o "1-3" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGEREF _Toc1 \h </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:fldSimple w:instr=" SEQ Figure \* ARABIC "><w:r><w:t>1</w:t></w:r></w:fldSimple><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0002"><w:bookmarkStart w:id="0" w:name="_Ref000000001"/><w:r><w:t xml:space="preserve">Figure </w:t></w:r><w:fldSimple w:instr=" SEQ Figure \* ARABIC "><w:r><w:t>1</w:t></w:r></w:fldSimple><w:bookmarkEnd w:id="0"/><w:r><w:t xml:space="preserve"> 示意图</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let tree = inspect(&src, true, true, true);
    assert_eq!(tree["captionCount"], 1);
    assert_eq!(tree["crossRefCount"], 0);
    assert_eq!(tree["captions"][0]["id"], "caption:1");
    assert_eq!(tree["captions"][0]["bookmark"], "_Ref000000001");
    assert_eq!(tree["captions"][0]["identifier"], "Figure");
    assert_eq!(tree["bookmarkCount"], 0);
    let codes: Vec<&str> = tree["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(codes.contains(&"TOC"));
    assert!(codes.contains(&"PAGEREF"));
    assert!(codes.contains(&"SEQ"));
}

#[test]
fn delete_caption_rejects_sect_pr_para() {
    let src = write_docx_xml(
        "sect",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>甲</w:t></w:r></w:p><w:p w14:paraId="AAAA0002"><w:pPr><w:sectPr><w:type w:val="nextPage"/><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:pPr><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> SEQ Figure \* ARABIC </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>乙</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#,
    );
    let err = apply_err(&src, json!([{ "op": "deleteCaption", "id": "caption:1" }]));
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

fn wrap_note_mark(src: &Path, dest: &Path, local: &str, bookmark: &str) {
    let bytes = std::fs::read(src).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zout = ZipWriter::new(&mut buf);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zin.len() {
            let mut file = zin.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if name == "word/document.xml" {
                let xml = String::from_utf8(data).unwrap();
                let needle = format!("<w:{local}");
                let at = xml.find(&needle).expect("引用标记");
                let head = &xml[..at];
                let run_at = ["<w:r>", "<w:r "]
                    .iter()
                    .filter_map(|pat| head.rfind(pat))
                    .max()
                    .expect("引用所在 run");
                let rel = xml[at..].find("</w:r>").expect("run 结束");
                let run_end = at + rel + "</w:r>".len();
                let wrapped = format!(
                    r#"<w:bookmarkStart w:id="90" w:name="{bookmark}"/>{}<w:bookmarkEnd w:id="90"/>"#,
                    &xml[run_at..run_end]
                );
                data = format!("{}{wrapped}{}", &xml[..run_at], &xml[run_end..]).into_bytes();
            }
            zout.start_file(&name, opt).unwrap();
            zout.write_all(&data).unwrap();
        }
        zout.finish().unwrap();
    }
    std::fs::write(dest, buf.into_inner()).unwrap();
}

fn note_cross_ref(tag: &str, op: &str, local: &str, bookmark: &str) {
    let src = create_docx(tag, json!({ "paragraphs": [{ "text": "甲" }] }));
    let tree = inspect(&src, false, false, false);
    let para = first_body_para_id(&tree);
    let noted = seq_path(&format!("{tag}-note"), "docx");
    apply(
        &src,
        &noted,
        json!([{ "op": op, "id": para, "text": "注一" }]),
    );
    let marked = seq_path(&format!("{tag}-mark"), "docx");
    wrap_note_mark(&noted, &marked, local, bookmark);
    let out = seq_path(&format!("{tag}-xref"), "docx");
    apply(
        &marked,
        &out,
        json!([{
            "op": "insertCrossRef",
            "id": para,
            "bookmark": bookmark,
            "kind": "note"
        }]),
    );
    let xml = std::fs::read(&out).unwrap();
    let mut zin = ZipArchive::new(Cursor::new(xml)).unwrap();
    let mut doc = String::new();
    zin.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut doc)
        .unwrap();
    let instr = format!(" NOTEREF {bookmark} \\h ");
    assert!(doc.contains(&instr), "{doc}");
    let start = format!(r#"<w:bookmarkStart w:id="90" w:name="{bookmark}"/>"#);
    let mark = format!("<w:{local}");
    let start_at = doc.find(&start).unwrap();
    let mark_at = doc.find(&mark).unwrap();
    let end_at = doc.find(r#"<w:bookmarkEnd w:id="90"/>"#).unwrap();
    assert!(start_at < mark_at && mark_at < end_at);
    let tree = inspect(&out, false, true, false);
    assert_eq!(tree["crossRefCount"], 1);
    assert_eq!(tree["crossRefs"][0]["kind"], "note");
    assert_eq!(tree["crossRefs"][0]["bookmark"], bookmark);
}

#[test]
fn noteref_footnote_and_endnote() {
    note_cross_ref("fn", "insertFootnote", "footnoteReference", "_RefFn1");
    note_cross_ref("en", "insertEndnote", "endnoteReference", "_RefEn1");
}
