//! docx 限制编辑：setProtection 写 Word 16.0 的 SHA-512，inspect 只报 hasPassword。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>hello</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/></Types>"#;

const SETTINGS_OTHER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:documentProtection w:edit="readOnly" w:enforcement="1" w:hash="YWJj" w:salt="ZGVm"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-protect-hash");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}.docx"))
}

fn run_json(req: &Value) -> Result<RunOutput, ooxp::Error> {
    run(&req.to_string())
}

fn inspect(path: &Path) -> Value {
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

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run_json(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    }))
}

fn assert_fail(src: &Path, ops: Value, want: ErrorCode) {
    let out = tmp("fail");
    let _ = std::fs::remove_file(&out);
    let err = apply(src, &out, ops).unwrap_err();
    assert_eq!(err.code(), want);
    assert!(!out.exists(), "失败的 apply 不应写出文件");
}

fn create_docx(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [
                { "style": "Heading1", "text": "标题" },
                { "text": "hello" }
            ]
        }
    }))
    .unwrap();
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

fn write_zip(tag: &str, settings: &str) -> PathBuf {
    let path = tmp(tag);
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in [
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", DOC.as_bytes()),
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

#[test]
fn password_sets_crypt_sha512_and_stays_out_of_tree() {
    let src = create_docx("host");
    let out = tmp("prot");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "readOnly",
            "password": "secret"
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["protection"]["enforced"], true);
    assert_eq!(tree["protection"]["edit"], "readOnly");
    assert_eq!(tree["protection"]["hasPassword"], true);
    assert!(tree["protection"].get("hash").is_none());
    assert!(tree["protection"].get("salt").is_none());
    assert!(tree["protection"].get("password").is_none());
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    assert!(!dump.contains("hashValue"));
    let xml = part_text(&out, "word/settings.xml");
    assert!(xml.contains(r#"cryptAlgorithmSid="14""#), "{xml}");
    assert!(xml.contains(r#"cryptSpinCount="100000""#), "{xml}");
    assert!(xml.contains(r#"cryptProviderType="rsaAES""#), "{xml}");
    assert!(xml.contains("w:hash="), "{xml}");
    assert!(xml.contains("w:salt="), "{xml}");
    assert!(!xml.contains("algorithmName="), "{xml}");
    assert!(!xml.contains("hashValue="), "{xml}");
    assert!(!xml.contains("secret"), "{xml}");
}

#[test]
fn matching_password_unprotects_and_wrong_password_fails() {
    let src = create_docx("round");
    let prot = tmp("prot");
    apply(
        &src,
        &prot,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();

    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false
        }]),
        ErrorCode::NotImplemented,
    );
    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );

    let opened = tmp("open");
    apply(
        &prot,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "word/settings.xml");
    assert!(!xml.contains("documentProtection"), "{xml}");
    assert_eq!(inspect(&opened)["protection"]["hasPassword"], false);
    assert_eq!(inspect(&opened)["protection"]["enforced"], false);
}

#[test]
fn blank_password_and_password_only_are_invalid() {
    let src = create_docx("blank");
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "password": ""
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "password": "secret"
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "readOnlyRecommended": true,
            "password": "secret"
        }]),
        ErrorCode::InvalidField,
    );
}

fn protection_attr_names(xml: &str) -> Vec<String> {
    let start = xml.find("documentProtection").expect("documentProtection");
    let end = xml[start..].find('>').unwrap() + start;
    let mut names = Vec::new();
    for part in xml[start..end].split_whitespace() {
        let Some((name, _)) = part.split_once('=') else {
            continue;
        };
        names.push(name.rsplit(':').next().unwrap().to_string());
    }
    names.sort();
    names
}

#[test]
fn create_password_matches_apply_hash_attrs_and_unlocks() {
    let created = tmp("create-pw");
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": created.to_string_lossy(),
        "format": "docx",
        "document": {
            "paragraphs": [{ "text": "hello" }],
            "protection": { "enforced": true, "edit": "readOnly", "password": "secret" }
        }
    }))
    .unwrap();
    let applied = tmp("apply-pw");
    let host = create_docx("apply-host");
    apply(
        &host,
        &applied,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": true,
            "edit": "readOnly",
            "password": "secret"
        }]),
    )
    .unwrap();
    let created_xml = part_text(&created, "word/settings.xml");
    let applied_xml = part_text(&applied, "word/settings.xml");
    assert_eq!(
        protection_attr_names(&created_xml),
        protection_attr_names(&applied_xml)
    );
    assert!(created_xml.contains(r#"cryptAlgorithmSid="14""#));
    assert!(created_xml.contains(r#"cryptSpinCount="100000""#));
    assert_ne!(created_xml, applied_xml, "盐应当不同");

    let opened = tmp("create-open");
    apply(
        &created,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "word/settings.xml");
    assert!(!xml.contains("documentProtection"), "{xml}");
}

#[test]
fn other_hash_stays_not_implemented_even_with_password() {
    let src = write_zip("other", SETTINGS_OTHER);
    assert_eq!(inspect(&src)["protection"]["hasPassword"], true);
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "secret"
        }]),
        ErrorCode::NotImplemented,
    );
}

#[test]
fn reservation_password_unlocks_and_rejects_the_wrong_one() {
    let settings = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:documentProtection w:edit="readOnly" w:enforcement="1" w:reservationPassword="A7DA4D01"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;
    let src = write_zip("reserv", settings);
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );
    let opened = tmp("reserv-open");
    apply(
        &src,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "docx:document",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "word/settings.xml");
    assert!(!xml.contains("documentProtection"), "{xml}");
}

#[test]
fn pptx_rejects_word_edit_field() {
    let pptx = tmp_dir().join(format!("rej-{}.pptx", SEQ.fetch_add(1, Ordering::Relaxed)));
    run_json(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": pptx.to_string_lossy(),
        "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
    }))
    .unwrap();
    assert_fail(
        &pptx,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "edit": "readOnly"
        }]),
        ErrorCode::InvalidField,
    );
}
