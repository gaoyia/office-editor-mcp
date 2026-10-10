//! docx 51：例外编辑区、节 formProt、writePassword、move 修订、OMML 属性、参考文献样式。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC_MOVE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:moveFromRangeStart w:id="5" w:name="mv1" w:author="A" w:date="2026-01-01T00:00:00Z"/><w:moveFrom w:id="5" w:author="A"><w:r><w:t>搬走</w:t></w:r></w:moveFrom><w:moveFromRangeEnd w:id="5"/></w:p><w:p><w:moveToRangeStart w:id="6" w:name="mv1" w:author="A" w:date="2026-01-01T00:00:00Z"/><w:moveTo w:id="6" w:author="A"><w:r><w:t>搬走</w:t></w:r></w:moveTo><w:moveToRangeEnd w:id="6"/></w:p><w:sectPr/></w:body></w:document>"#;

const SETTINGS_EMPTY: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;

const SETTINGS_WRITE_OTHER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:writeProtection w:hash="YWJj" w:salt="ZGVm"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-protect-rev-math");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seq_path(tag: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.{ext}"))
}

fn run_json(req: &Value) -> Result<RunOutput, ooxp::Error> {
    run(&req.to_string(), None)
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run_json(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    }))
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn inspect_bare(path: &Path) -> Value {
    let RunOutput::Inspect(tree) = run_json(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy()
    }))
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) {
    run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }))
    .unwrap();
}

fn apply_err(src: &Path, ops: Value) -> ooxp::Error {
    let out = seq_path("should-not-write", "docx");
    let _ = std::fs::remove_file(&out);
    let err = run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }))
    .unwrap_err();
    assert!(!out.exists(), "失败的 apply 不应写出文件");
    err
}

fn create_docx(tag: &str, document: Value) -> PathBuf {
    let path = seq_path(tag, "docx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": document
    }))
    .unwrap();
    path
}

fn write_zip(tag: &str, document: &str, settings: &str) -> PathBuf {
    let path = seq_path(tag, "docx");
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in [
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", document.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/settings.xml", settings.as_bytes()),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, buf).unwrap();
    path
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn first_para_id(tree: &Value) -> String {
    tree["paragraphs"][0]["id"].as_str().unwrap().to_string()
}

#[test]
fn edit_range_insert_list_delete_and_header() {
    let src = create_docx(
        "perm-src",
        json!({
            "paragraphs": [{ "text": "hello" }],
            "headers": [{ "type": "default", "paragraphs": [{ "text": "页眉" }] }]
        }),
    );
    let bare = inspect_bare(&src);
    assert_eq!(bare["editRangeCount"], 0);
    assert!(bare.get("editRanges").is_none());
    assert!(bare["bibliographyStyle"].is_null());
    assert!(bare["sections"][0]["formProt"].is_null());

    let tree0 = inspect(
        &src,
        json!({ "from": 1, "maxParas": 20, "editRanges": true, "headerFooter": true }),
    );
    let p1 = first_para_id(&tree0);
    let hp = tree0["headers"][0]["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let out = seq_path("perm-out", "docx");
    apply(
        &src,
        &out,
        json!([
            { "op": "insertEditRange", "id": p1 },
            { "op": "insertEditRange", "id": hp, "edGrp": "editors" }
        ]),
    );
    let tree = inspect(
        &out,
        json!({ "from": 1, "maxParas": 20, "editRanges": true, "headerFooter": true }),
    );
    assert_eq!(tree["editRangeCount"], 1);
    assert_eq!(
        object_keys(&tree["editRanges"][0]),
        ["id", "index", "xmlId", "edGrp", "ed", "host", "text"]
    );
    assert_eq!(tree["editRanges"][0]["id"], "perm:1");
    assert_eq!(tree["editRanges"][0]["edGrp"], "everyone");
    assert!(tree["editRanges"][0]["ed"].is_null());
    assert_eq!(tree["editRanges"][0]["host"], p1);
    assert_eq!(tree["editRanges"][0]["text"], "hello");
    assert_eq!(tree["headers"][0]["editRangeCount"], 1);
    assert_eq!(
        tree["headers"][0]["editRanges"][0]["id"],
        "header:default:1/perm:1"
    );
    assert_eq!(tree["headers"][0]["editRanges"][0]["edGrp"], "editors");
    let xml = part_text(&out, "word/document.xml");
    assert!(xml.contains("permStart"), "{xml}");
    assert!(xml.contains(r#"edGrp="everyone""#), "{xml}");
    assert!(xml.contains("hello"), "{xml}");

    let gone = seq_path("perm-del", "docx");
    apply(
        &out,
        &gone,
        json!([{ "op": "deleteEditRange", "id": "perm:1" }]),
    );
    let tree2 = inspect_bare(&gone);
    assert_eq!(tree2["editRangeCount"], 0);
    let xml2 = part_text(&gone, "word/document.xml");
    assert!(!xml2.contains("permStart"), "{xml2}");
    assert!(xml2.contains("hello"), "{xml2}");

    let err_both = apply_err(
        &src,
        json!([{ "op": "insertEditRange", "id": p1, "edGrp": "everyone", "ed": "alice" }]),
    );
    assert_eq!(err_both.code(), ErrorCode::InvalidField);
    let err_grp = apply_err(
        &src,
        json!([{ "op": "insertEditRange", "id": p1, "edGrp": "nope" }]),
    );
    assert_eq!(err_grp.code(), ErrorCode::InvalidField);
    let err_ed = apply_err(
        &src,
        json!([{ "op": "insertEditRange", "id": p1, "ed": "" }]),
    );
    assert_eq!(err_ed.code(), ErrorCode::InvalidField);
    let err_math = apply_err(&src, json!([{ "op": "insertEditRange", "id": "math:1" }]));
    assert_eq!(err_math.code(), ErrorCode::InvalidId);
    let err_miss = apply_err(&src, json!([{ "op": "deleteEditRange", "id": "perm:1" }]));
    assert_eq!(err_miss.code(), ErrorCode::IdNotFound);
}

#[test]
fn form_prot_three_states_and_forms_protection() {
    let src = create_docx("fp-src", json!({ "paragraphs": [{ "text": "hello" }] }));
    let forms = seq_path("fp-forms", "docx");
    apply(
        &src,
        &forms,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "forms"
        }]),
    );
    let tree = inspect_bare(&forms);
    assert_eq!(tree["sections"][0]["formProt"], true);
    let xml = part_text(&forms, "word/document.xml");
    assert!(xml.contains("formProt"), "{xml}");
    assert!(!xml.contains(r#"formProt w:val="0""#), "{xml}");

    let off = seq_path("fp-off", "docx");
    apply(
        &forms,
        &off,
        json!([{ "op": "setFormProt", "id": "section:1", "formProt": false }]),
    );
    let tree2 = inspect_bare(&off);
    assert_eq!(tree2["sections"][0]["formProt"], false);
    let xml2 = part_text(&off, "word/document.xml");
    assert!(
        xml2.contains(r#"formProt w:val="0""#) || xml2.contains(r#"w:val="0""#),
        "{xml2}"
    );

    let gone = seq_path("fp-null", "docx");
    apply(
        &off,
        &gone,
        json!([{ "op": "setFormProt", "id": "section:1", "formProt": null }]),
    );
    let tree3 = inspect_bare(&gone);
    assert!(tree3["sections"][0]["formProt"].is_null());

    let err = apply_err(&src, json!([{ "op": "setFormProt", "id": "section:1" }]));
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn write_password_hash_gate_and_create_reject() {
    let src = create_docx("wp-src", json!({ "paragraphs": [{ "text": "hello" }] }));
    let out = seq_path("wp-out", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": "secret"
        }]),
    );
    let tree = inspect_bare(&out);
    assert_eq!(tree["protection"]["hasPassword"], true);
    assert_eq!(tree["protection"]["readOnlyRecommended"], false);
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    let xml = part_text(&out, "word/settings.xml");
    assert!(xml.contains("writeProtection"), "{xml}");
    assert!(xml.contains(r#"cryptAlgorithmSid="14""#), "{xml}");
    assert!(xml.contains(r#"cryptSpinCount="100000""#), "{xml}");
    assert!(!xml.contains("algorithmName="), "{xml}");
    assert!(!xml.contains("secret"), "{xml}");
    let salt1 = xml
        .split("w:salt=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();

    let err_blank = apply_err(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": ""
        }]),
    );
    assert_eq!(err_blank.code(), ErrorCode::InvalidField);
    let err_wrong = apply_err(
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": "wrong",
            "readOnlyRecommended": false
        }]),
    );
    assert_eq!(err_wrong.code(), ErrorCode::InvalidField);
    let err_need = apply_err(
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "readOnlyRecommended": true
        }]),
    );
    assert_eq!(err_need.code(), ErrorCode::NotImplemented);

    let rewritten = seq_path("wp-rewrite", "docx");
    apply(
        &out,
        &rewritten,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": "secret"
        }]),
    );
    let xml2 = part_text(&rewritten, "word/settings.xml");
    let salt2 = xml2
        .split("w:salt=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    assert_ne!(salt1, salt2, "{xml} vs {xml2}");

    let opened = seq_path("wp-off", "docx");
    apply(
        &out,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": "secret",
            "readOnlyRecommended": false
        }]),
    );
    let tree3 = inspect_bare(&opened);
    assert_eq!(tree3["protection"]["hasPassword"], false);
    let xml3 = part_text(&opened, "word/settings.xml");
    assert!(!xml3.contains("writeProtection"), "{xml3}");

    let both = seq_path("wp-both", "docx");
    apply(
        &src,
        &both,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "readOnly",
            "password": "secret",
            "writePassword": "secret"
        }]),
    );
    let xml4 = part_text(&both, "word/settings.xml");
    assert!(xml4.contains("documentProtection"), "{xml4}");
    assert!(xml4.contains("writeProtection"), "{xml4}");

    let create_err = run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": seq_path("wp-create", "docx").to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "a" }],
            "protection": { "writePassword": "secret" }
        }
    }))
    .unwrap_err();
    assert_eq!(create_err.code(), ErrorCode::NotImplemented);

    let other = write_zip("wp-other", DOC_MOVE, SETTINGS_WRITE_OTHER);
    let err_other = apply_err(
        &other,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "writePassword": "secret"
        }]),
    );
    assert_eq!(err_other.code(), ErrorCode::NotImplemented);
}

#[test]
fn move_revisions_accept_reject() {
    let src = write_zip("mv-src", DOC_MOVE, SETTINGS_EMPTY);
    let tree = inspect(
        &src,
        json!({ "revisions": true, "from": 1, "maxParas": 20 }),
    );
    assert_eq!(tree["revisionCount"], 2);
    assert_eq!(tree["revisions"][0]["kind"], "moveFrom");
    assert_eq!(tree["revisions"][0]["text"], "搬走");
    assert_eq!(tree["revisions"][1]["kind"], "moveTo");
    assert_eq!(tree["revisions"][1]["text"], "搬走");

    let acc_from = seq_path("mv-acc-from", "docx");
    apply(
        &src,
        &acc_from,
        json!([{ "op": "acceptRevision", "id": "rev:1" }]),
    );
    let xml = part_text(&acc_from, "word/document.xml");
    assert!(!xml.contains("moveFrom"), "{xml}");
    assert!(!xml.contains("moveFromRangeStart"), "{xml}");
    assert!(xml.contains("moveTo"), "{xml}");
    assert!(!xml.contains(">搬走</w:t></w:r></w:moveFrom>"), "{xml}");

    let rej_from = seq_path("mv-rej-from", "docx");
    apply(
        &src,
        &rej_from,
        json!([{ "op": "rejectRevision", "id": "rev:1" }]),
    );
    let xml_r = part_text(&rej_from, "word/document.xml");
    assert!(!xml_r.contains("moveFrom"), "{xml_r}");
    assert!(xml_r.contains("搬走"), "{xml_r}");

    let acc_to = seq_path("mv-acc-to", "docx");
    apply(
        &src,
        &acc_to,
        json!([{ "op": "acceptRevision", "id": "rev:2" }]),
    );
    let xml_t = part_text(&acc_to, "word/document.xml");
    assert!(!xml_t.contains("moveTo"), "{xml_t}");
    assert!(xml_t.contains("搬走"), "{xml_t}");

    let rej_to = seq_path("mv-rej-to", "docx");
    apply(
        &src,
        &rej_to,
        json!([{ "op": "rejectRevision", "id": "rev:2" }]),
    );
    let xml_rt = part_text(&rej_to, "word/document.xml");
    assert!(!xml_rt.contains("moveTo"), "{xml_rt}");

    let p1 = first_para_id(&tree);
    let err = apply_err(
        &src,
        json!([{ "op": "setParaText", "id": p1, "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn math_jc_phant_border_box_brk() {
    let src = create_docx("mj-src", json!({ "paragraphs": [{ "text": "甲" }] }));
    let tree0 = inspect(&src, json!({ "from": 1, "maxParas": 20 }));
    let p1 = first_para_id(&tree0);

    let err_inline = apply_err(
        &src,
        json!([{ "op": "insertMath", "id": p1, "jc": "center", "text": "x" }]),
    );
    assert_eq!(err_inline.code(), ErrorCode::InvalidField);
    let err_jc = apply_err(
        &src,
        json!([{
            "op": "insertMath",
            "id": p1,
            "kind": "display",
            "jc": "middle",
            "text": "x"
        }]),
    );
    assert_eq!(err_jc.code(), ErrorCode::InvalidField);

    let disp = seq_path("mj-disp", "docx");
    apply(
        &src,
        &disp,
        json!([{
            "op": "insertMath",
            "id": p1,
            "kind": "display",
            "jc": "center",
            "text": "x=1"
        }]),
    );
    let tree = inspect(&disp, json!({ "from": 1, "maxParas": 20, "maths": true }));
    assert_eq!(tree["maths"][0]["kind"], "display");
    assert_eq!(tree["maths"][0]["jc"], "center");

    let jc_null = seq_path("mj-jc-null", "docx");
    apply(
        &disp,
        &jc_null,
        json!([{ "op": "setMath", "id": "math:1", "jc": null }]),
    );
    let tree2 = inspect(
        &jc_null,
        json!({ "from": 1, "maxParas": 20, "maths": true }),
    );
    assert!(tree2["maths"][0]["jc"].is_null());

    let struct_out = seq_path("mj-struct", "docx");
    apply(
        &src,
        &struct_out,
        json!([{
            "op": "insertMath",
            "id": p1,
            "kind": "display",
            "expr": [
                {
                    "type": "phant",
                    "show": false,
                    "e": [{ "type": "text", "text": "x", "brk": true }]
                },
                {
                    "type": "borderBox",
                    "hideTop": true,
                    "e": [{ "type": "text", "text": "y" }]
                }
            ]
        }]),
    );
    let tree3 = inspect(
        &struct_out,
        json!({ "from": 1, "maxParas": 20, "maths": true }),
    );
    assert_eq!(tree3["maths"][0]["expr"][0]["type"], "phant");
    assert_eq!(tree3["maths"][0]["expr"][0]["show"], false);
    assert_eq!(tree3["maths"][0]["expr"][0]["e"][0]["brk"], true);
    assert_eq!(tree3["maths"][0]["expr"][1]["type"], "borderBox");
    assert_eq!(tree3["maths"][0]["expr"][1]["hideTop"], true);
    let xml = part_text(&struct_out, "word/document.xml");
    assert!(xml.contains("m:phant"), "{xml}");
    assert!(xml.contains("m:borderBox"), "{xml}");
    assert!(xml.contains("m:brk"), "{xml}");

    let created = create_docx(
        "mj-create",
        json!({
            "paragraphs": [{
                "text": "甲",
                "maths": [{
                    "kind": "display",
                    "jc": "left",
                    "expr": [{ "type": "text", "text": "z" }]
                }]
            }]
        }),
    );
    let tree4 = inspect(
        &created,
        json!({ "from": 1, "maxParas": 20, "maths": true }),
    );
    assert_eq!(tree4["maths"][0]["jc"], "left");

    let err_box = apply_err(
        &src,
        json!([{
            "op": "insertMath",
            "id": p1,
            "expr": [{ "type": "box", "e": [{ "type": "text", "text": "x" }] }]
        }]),
    );
    assert_eq!(err_box.code(), ErrorCode::InvalidField);
}

#[test]
fn bibliography_style_set_and_create_empty_part() {
    let src = create_docx("bs-src", json!({ "paragraphs": [{ "text": "hello" }] }));
    let bare = inspect_bare(&src);
    assert!(bare["bibliographyStyle"].is_null());

    let out = seq_path("bs-out", "docx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setBibliographyStyle",
            "id": "docx:document",
            "styleName": "MLA"
        }]),
    );
    let tree = inspect_bare(&out);
    assert_eq!(
        object_keys(&tree["bibliographyStyle"]),
        ["selectedStyle", "styleName", "version"]
    );
    assert_eq!(tree["bibliographyStyle"]["styleName"], "MLA");
    assert_eq!(tree["bibliographyStyle"]["version"], "7");
    assert_eq!(
        tree["bibliographyStyle"]["selectedStyle"],
        r"\MLASeventhEditionOfficeOnline.xsl"
    );

    let err_style = apply_err(
        &src,
        json!([{
            "op": "setBibliographyStyle",
            "id": "docx:document",
            "styleName": "AMA"
        }]),
    );
    assert_eq!(err_style.code(), ErrorCode::InvalidField);
    let err_id = apply_err(
        &src,
        json!([{
            "op": "setBibliographyStyle",
            "id": "para:1",
            "styleName": "APA"
        }]),
    );
    assert_eq!(err_id.code(), ErrorCode::InvalidId);
}

#[test]
fn xlsx_pptx_reject_new_docx_ops() {
    let xlsx = seq_path("pq-x", "xlsx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": xlsx.to_string_lossy(),
        "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
    }))
    .unwrap();
    for op in [
        json!({ "op": "insertEditRange", "id": "para:1" }),
        json!({ "op": "deleteEditRange", "id": "perm:1" }),
        json!({ "op": "setFormProt", "id": "section:1", "formProt": true }),
        json!({ "op": "setBibliographyStyle", "id": "docx:document", "styleName": "APA" }),
    ] {
        let out = seq_path("pq-x-out", "xlsx");
        let _ = std::fs::remove_file(&out);
        let err = run_json(&json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": [op]
        }))
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        assert!(!out.exists());
    }
    let err_wp = run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": xlsx.to_string_lossy(),
        "output": seq_path("pq-x-wp", "xlsx").to_string_lossy(),
        "ops": [{
            "op": "setProtection",
            "id": "xlsx:workbook",
            "lockStructure": true,
            "writePassword": "secret"
        }]
    }))
    .unwrap_err();
    assert_eq!(err_wp.code(), ErrorCode::InvalidField);
    let err_scope = run_json(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": xlsx.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1", "editRanges": true }
    }))
    .unwrap_err();
    assert_eq!(err_scope.code(), ErrorCode::InvalidField);

    let pptx = seq_path("pq-p", "pptx");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    }))
    .unwrap();
    let pout = seq_path("pq-p-out", "pptx");
    let err_p = run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": pptx.to_string_lossy(),
        "output": pout.to_string_lossy(),
        "ops": [{ "op": "setBibliographyStyle", "id": "docx:document", "styleName": "APA" }]
    }))
    .unwrap_err();
    assert_eq!(err_p.code(), ErrorCode::InvalidField);
    assert!(!pout.exists());
}
