//! 修改保护：setProtection 写 SHA-512，inspect 只报 hasPassword。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-pptx-protect-hash");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tmp(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

fn create_deck(tag: &str) -> PathBuf {
    let path = tmp(tag).with_extension("pptx");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "presentation": {
            "slides": [{ "shapes": [{ "text": "页1" }] }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    })
    .to_string())
}

fn inspect(path: &Path) -> Value {
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

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn xml_attr(xml: &str, name: &str) -> Option<String> {
    let p = format!("{name}=\"");
    let i = xml.find(&p)?;
    let rest = &xml[i + p.len()..];
    let j = rest.find('"')?;
    Some(rest[..j].to_string())
}

fn rewrite_part(src: &Path, dst: &Path, name: &str, f: impl FnOnce(&str) -> String) {
    let bytes = std::fs::read(src).unwrap();
    let mut zip_in = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut transform = Some(f);
    {
        let mut zip_out = ZipWriter::new(&mut out);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for i in 0..zip_in.len() {
            let mut file = zip_in.by_index(i).unwrap();
            let n = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            if n == name {
                let xml = String::from_utf8(data).unwrap();
                data = transform.take().expect("部件只应出现一次")(&xml).into_bytes();
            }
            zip_out.start_file(&n, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(dst, out.into_inner()).unwrap();
}

fn assert_fail(src: &Path, ops: Value, want: ErrorCode) {
    let out = tmp("fail").with_extension("pptx");
    let err = apply(src, &out, ops).unwrap_err();
    assert_eq!(err.code(), want);
    assert!(!out.exists());
}

#[test]
fn password_sets_crypt_sha512_and_stays_out_of_tree() {
    let src = create_deck("set");
    let out = tmp("prot").with_extension("pptx");
    apply(
        &src,
        &out,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let tree = inspect(&out);
    assert_eq!(tree["modifyProtection"]["hasPassword"], true);
    assert!(tree["modifyProtection"].get("hashData").is_none());
    assert!(tree["modifyProtection"].get("saltData").is_none());
    assert!(tree["modifyProtection"].get("password").is_none());
    let dump = tree.to_string();
    assert!(!dump.contains("secret"), "{dump}");
    assert!(!dump.contains("hashData"));
    let xml = part_text(&out, "ppt/presentation.xml");
    assert!(xml.contains(r#"cryptProviderType="rsaAES""#), "{xml}");
    assert!(xml.contains(r#"cryptAlgorithmClass="hash""#), "{xml}");
    assert!(xml.contains(r#"cryptAlgorithmType="typeAny""#), "{xml}");
    assert!(xml.contains(r#"cryptAlgorithmSid="14""#), "{xml}");
    assert!(xml.contains(r#"spinCount="100000""#), "{xml}");
    assert!(xml.contains("hashData="), "{xml}");
    assert!(xml.contains("saltData="), "{xml}");
    assert!(!xml.contains("algorithmName="), "{xml}");
    assert!(!xml.contains("hashValue="), "{xml}");
    assert!(!xml.contains(">secret<"), "{xml}");
    let salt = xml_attr(&xml, "saltData").expect("应有 saltData");
    let hash = xml_attr(&xml, "hashData").expect("应有 hashData");
    assert_eq!(salt.len(), 24, "{salt}");
    assert_eq!(hash.len(), 88, "{hash}");
}

#[test]
fn matching_password_unprotects_and_wrong_password_fails() {
    let src = create_deck("round");
    let prot = tmp("prot").with_extension("pptx");
    apply(
        &src,
        &prot,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();

    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": false
        }]),
        ErrorCode::NotImplemented,
    );
    assert_fail(
        &prot,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": false,
            "password": "wrong"
        }]),
        ErrorCode::InvalidField,
    );

    let opened = tmp("open").with_extension("pptx");
    apply(
        &prot,
        &opened,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": false,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml = part_text(&opened, "ppt/presentation.xml");
    assert!(!xml.contains("modifyVerifier"), "{xml}");
    assert_eq!(inspect(&opened)["modifyProtection"]["hasPassword"], false);
}

#[test]
fn empty_verifier_without_password_and_rewrite_salt() {
    let src = create_deck("empty");
    let empty = tmp("empty").with_extension("pptx");
    apply(
        &src,
        &empty,
        json!([{ "op": "setProtection", "id": "pptx:presentation", "enforced": true }]),
    )
    .unwrap();
    let xml = part_text(&empty, "ppt/presentation.xml");
    assert!(xml.contains("<p:modifyVerifier/>"), "{xml}");
    assert!(!xml.contains("hashData="), "{xml}");
    assert_eq!(inspect(&empty)["modifyProtection"]["hasPassword"], false);

    let prot = tmp("prot").with_extension("pptx");
    apply(
        &empty,
        &prot,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml1 = part_text(&prot, "ppt/presentation.xml");
    let salt1 = xml_attr(&xml1, "saltData").expect("应有 saltData");

    let again = tmp("again").with_extension("pptx");
    apply(
        &prot,
        &again,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "password": "secret"
        }]),
    )
    .unwrap();
    let xml2 = part_text(&again, "ppt/presentation.xml");
    let salt2 = xml_attr(&xml2, "saltData").expect("应有新 saltData");
    assert_ne!(salt1, salt2);
    assert_eq!(inspect(&again)["modifyProtection"]["hasPassword"], true);
}

#[test]
fn blank_password_and_other_hash_are_rejected() {
    let src = create_deck("blank");
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": true,
            "password": ""
        }]),
        ErrorCode::InvalidField,
    );
    assert_fail(
        &src,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "password": "secret"
        }]),
        ErrorCode::InvalidField,
    );

    let iso = tmp("iso").with_extension("pptx");
    rewrite_part(&src, &iso, "ppt/presentation.xml", |xml| {
        xml.replace(
            "</p:presentation>",
            r#"<p:modifyVerifier algorithmName="SHA-512" hashValue="YWJj" saltValue="ZGVm" spinCount="100000"/></p:presentation>"#,
        )
    });
    assert_eq!(inspect(&iso)["modifyProtection"]["hasPassword"], true);
    assert_fail(
        &iso,
        json!([{
            "op": "setProtection",
            "id": "pptx:presentation",
            "enforced": false,
            "password": "secret"
        }]),
        ErrorCode::NotImplemented,
    );
}
