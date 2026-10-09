//! docx 批注：计数、名单、范围、回复、改字、插删、完成标记。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/><Override PartName="/word/commentsExtended.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml"/><Override PartName="/word/commentsIds.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.commentsIds+xml"/><Override PartName="/word/commentsExtensible.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtensible+xml"/><Override PartName="/word/people.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.people+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/><Relationship Id="rId3" Type="http://schemas.microsoft.com/office/2011/relationships/commentsExtended" Target="commentsExtended.xml"/><Relationship Id="rId4" Type="http://schemas.microsoft.com/office/2011/relationships/people" Target="people.xml"/><Relationship Id="rId5" Type="http://schemas.microsoft.com/office/2016/09/relationships/commentsIds" Target="commentsIds.xml"/><Relationship Id="rId6" Type="http://schemas.microsoft.com/office/2018/08/relationships/commentsExtensible" Target="commentsExtensible.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p w14:paraId="AAAA0001"><w:commentRangeStart w:id="0"/><w:r><w:t>标题</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="0"/></w:r></w:p><w:p w14:paraId="AAAA0002"><w:commentRangeStart w:id="2"/><w:r><w:t>正文</w:t></w:r></w:p><w:p w14:paraId="AAAA0003"><w:r><w:t>第三段</w:t></w:r><w:commentRangeEnd w:id="2"/><w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="2"/></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const COMMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:comment w:id="0" w:author="Alice" w:initials="A" w:date="2026-01-01T00:00:00Z"><w:p w14:paraId="C0000001"><w:pPr><w:pStyle w:val="CommentText"/></w:pPr><w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:annotationRef/></w:r><w:r><w:t>批注甲</w:t></w:r></w:p></w:comment><w:comment w:id="1" w:author="Bob" w:initials="B"><w:p w14:paraId="C0000002"><w:r><w:annotationRef/></w:r><w:r><w:t>回复乙</w:t></w:r></w:p></w:comment><w:comment w:id="2" w:author="Alice" w:initials="A"><w:p w14:paraId="C0000003"><w:r><w:annotationRef/></w:r><w:r><w:t>批注丙</w:t></w:r></w:p></w:comment></w:comments>"#;

const COMMENTS_EX: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w15:commentsEx xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml"><w15:commentEx w15:paraId="C0000001" w15:done="0"/><w15:commentEx w15:paraId="C0000002" w15:paraIdParent="C0000001" w15:done="0"/><w15:commentEx w15:paraId="C0000003" w15:done="1"/></w15:commentsEx>"#;

const PEOPLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w15:people xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml"><w15:person w15:author="Alice"><w15:presenceInfo w15:providerId="None" w15:userId="Alice"/></w15:person></w15:people>"#;

const COMMENTS_IDS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w16cid:commentsIds xmlns:w16cid="http://schemas.microsoft.com/office/word/2016/wordml/cid"><w16cid:commentId w16cid:paraId="C0000001" w16cid:durableId="11111111"/><w16cid:commentId w16cid:paraId="C0000002" w16cid:durableId="22222222"/><w16cid:commentId w16cid:paraId="C0000003" w16cid:durableId="33333333"/></w16cid:commentsIds>"#;

const COMMENTS_EXTENSIBLE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w16cex:commentsExtensible xmlns:w16cex="http://schemas.microsoft.com/office/word/2018/wordml/cex"><w16cex:commentExtensible w16cex:durableId="11111111"/><w16cex:commentExtensible w16cex:durableId="22222222"/><w16cex:commentExtensible w16cex:durableId="33333333"/></w16cex:commentsExtensible>"#;

const INK_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body><w:p w14:paraId="AAAA0001"><w:commentRangeStart w:id="0"/><w:r><w:t>墨迹</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#;

const INK_COMMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:inkml="http://www.w3.org/2003/InkML"><w:comment w:id="0" w:author="Ink" w:initials="I"><w:p w14:paraId="C0000001"><w:r><w:annotationRef/></w:r><inkml:ink/></w:p></w:comment></w:comments>"#;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-comments");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.docx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opt).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn write_sample(tag: &str) -> PathBuf {
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/comments.xml", COMMENTS.as_bytes()),
            ("word/commentsExtended.xml", COMMENTS_EX.as_bytes()),
            ("word/commentsIds.xml", COMMENTS_IDS.as_bytes()),
            (
                "word/commentsExtensible.xml",
                COMMENTS_EXTENSIBLE.as_bytes(),
            ),
            ("word/people.xml", PEOPLE.as_bytes()),
        ],
    )
}

fn write_ink_sample(tag: &str) -> PathBuf {
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", INK_DOCUMENT.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/comments.xml", INK_COMMENTS.as_bytes()),
            ("word/commentsExtended.xml", COMMENTS_EX.as_bytes()),
            ("word/commentsIds.xml", COMMENTS_IDS.as_bytes()),
            (
                "word/commentsExtensible.xml",
                COMMENTS_EXTENSIBLE.as_bytes(),
            ),
            ("word/people.xml", PEOPLE.as_bytes()),
        ],
    )
}

fn write_png(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.png"));
    std::fs::write(&path, PNG_1X1).unwrap();
    path
}

fn inspect(path: &Path, scope: serde_json::Value) -> serde_json::Value {
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

fn apply(path: &Path, out: &Path, ops: serde_json::Value) {
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
    .unwrap();
}

fn apply_err(path: &Path, ops: serde_json::Value) -> ooxp::Error {
    let out = tmp_dir().join("err.docx");
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
    .unwrap_err()
}

fn object_keys(v: &serde_json::Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_bytes(path: &Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name.trim_start_matches('/')).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn part_text(path: &Path, name: &str) -> String {
    String::from_utf8(part_bytes(path, name)).unwrap()
}

fn comment_by_text<'a>(tree: &'a serde_json::Value, text: &str) -> &'a serde_json::Value {
    tree["comments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["paragraphs"][0]["text"] == text)
        .unwrap_or_else(|| panic!("找不到批注文本 {text}"))
}

#[test]
fn inspect_counts_without_list() {
    let path = write_sample("counts");
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["commentCount"], 3);
    assert!(tree.get("comments").is_none());
    assert_eq!(
        object_keys(&tree),
        [
            "schemaVersion",
            "format",
            "id",
            "paraCount",
            "pictureCount",
            "textBoxCount",
            "connectorCount",
            "inkCanvasCount",
            "footnoteCount",
            "endnoteCount",
            "commentCount",
            "chartCount",
            "smartArtCount",
            "oleObjectCount",
            "model3dCount",
            "watermarkCount",
            "tableCount",
            "revisionCount",
            "hyperlinkCount",
            "bookmarkCount",
            "fieldCount",
            "tocCount",
            "captionCount",
            "crossRefCount",
            "indexCount",
            "indexEntryCount",
            "sourceCount",
            "citationCount",
            "bibliographyCount",
            "docPartCount",
            "mathCount",
            "contentControlCount",
            "numberingCount",
            "styleCount",
            "embeddedFontCount",
            "outline",
            "headers",
            "footers",
            "sections",
            "mirrorMargins",
            "gutterAtTop",
            "props",
            "protection",
            "theme",
            "fontEmbed",
            "pageBackground"
        ]
    );
}

#[test]
fn inspect_lists_parent_range_and_comment_ids() {
    let path = write_sample("lists");
    let tree = inspect(
        &path,
        json!({ "from": 1, "maxParas": 20, "comments": true }),
    );
    assert_eq!(tree["comments"][0]["id"], "comment:0");
    assert_eq!(tree["comments"][0]["commentId"], "0");
    assert_eq!(tree["comments"][0]["author"], "Alice");
    assert_eq!(tree["comments"][0]["initials"], "A");
    assert_eq!(tree["comments"][0]["date"], "2026-01-01T00:00:00Z");
    assert_eq!(tree["comments"][0]["done"], false);
    assert!(tree["comments"][0]["parent"].is_null());
    assert_eq!(tree["comments"][0]["kind"], "normal");
    assert_eq!(tree["comments"][0]["start"], "para:AAAA0001");
    assert_eq!(tree["comments"][0]["end"], "para:AAAA0001");
    assert_eq!(
        tree["comments"][0]["paragraphs"][0]["id"],
        "comment:0/para:C0000001"
    );
    assert_eq!(tree["comments"][0]["paragraphs"][0]["text"], "批注甲");

    assert_eq!(tree["comments"][1]["id"], "comment:1");
    assert_eq!(tree["comments"][1]["parent"], "comment:0");
    assert_eq!(tree["comments"][1]["start"], "para:AAAA0001");
    assert_eq!(tree["comments"][1]["end"], "para:AAAA0001");
    assert!(tree["comments"][1]["date"].is_null());

    assert_eq!(tree["comments"][2]["id"], "comment:2");
    assert_eq!(tree["comments"][2]["done"], true);
    assert_eq!(tree["comments"][2]["start"], "para:AAAA0002");
    assert_eq!(tree["comments"][2]["end"], "para:AAAA0003");

    let ids0 = tree["paragraphs"][0]["commentIds"].as_array().unwrap();
    assert!(ids0.iter().any(|v| v == "comment:0"));
    assert!(ids0.iter().any(|v| v == "comment:1"));
    assert_eq!(tree["paragraphs"][1]["commentIds"][0], "comment:2");
    assert_eq!(tree["paragraphs"][2]["commentIds"][0], "comment:2");

    assert_eq!(
        object_keys(&tree["comments"][0]),
        [
            "id",
            "commentId",
            "author",
            "initials",
            "date",
            "done",
            "parent",
            "kind",
            "start",
            "end",
            "paraCount",
            "pictureCount",
            "textBoxCount",
            "connectorCount",
            "chartCount",
            "smartArtCount",
            "paragraphs"
        ]
    );
}

#[test]
fn set_comment_text_keeps_document_bytes() {
    let path = write_sample("set-text");
    let before = part_bytes(&path, "word/document.xml");
    let out = tmp_dir().join("set-text-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setCommentText", "id": "comment:0", "text": "改过的批注" }]),
    );
    assert_eq!(part_bytes(&out, "word/document.xml"), before);
    let tree = inspect(&out, json!({ "comments": true, "from": 1, "maxParas": 5 }));
    assert_eq!(tree["comments"][0]["paragraphs"][0]["text"], "改过的批注");
    assert_eq!(tree["paragraphs"][0]["text"], "标题");
    let xml = part_text(&out, "word/comments.xml");
    assert!(xml.contains("annotationRef"));
    assert!(xml.contains("改过的批注"));
}

#[test]
fn insert_reply_delete_and_done() {
    let path = write_sample("ops");
    let inserted = tmp_dir().join("ins.docx");
    apply(
        &path,
        &inserted,
        json!([{
            "op": "insertComment",
            "start": "para:AAAA0003",
            "end": "para:AAAA0003",
            "text": "新批注"
        }]),
    );
    let tree = inspect(
        &inserted,
        json!({ "from": 1, "maxParas": 20, "comments": true }),
    );
    assert_eq!(tree["commentCount"], 4);
    let neu = comment_by_text(&tree, "新批注");
    assert_eq!(neu["id"], "comment:3");
    assert_eq!(neu["author"], "ooxp");
    assert_eq!(neu["initials"], "oo");
    assert_eq!(neu["start"], "para:AAAA0003");
    assert_eq!(neu["end"], "para:AAAA0003");
    assert!(tree["paragraphs"][2]["commentIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "comment:3"));
    assert!(part_text(&inserted, "word/document.xml").contains(r#"commentRangeStart w:id="3""#));
    assert!(part_text(&inserted, "word/people.xml").contains("ooxp"));

    let replied = tmp_dir().join("reply.docx");
    apply(
        &inserted,
        &replied,
        json!([{ "op": "replyComment", "id": "comment:0", "text": "新回复" }]),
    );
    let tree = inspect(&replied, json!({ "comments": true }));
    let reply = comment_by_text(&tree, "新回复");
    assert_eq!(reply["parent"], "comment:0");
    assert_eq!(reply["start"], "para:AAAA0001");
    assert!(part_text(&replied, "word/document.xml").contains(r#"commentRangeStart w:id="4""#));
    assert!(part_text(&replied, "word/document.xml").contains(r#"commentReference w:id="4""#));
    let reply_para = reply["paragraphs"][0]["id"]
        .as_str()
        .unwrap()
        .rsplit(':')
        .next()
        .unwrap()
        .to_ascii_lowercase();
    assert!(part_text(&replied, "word/commentsIds.xml")
        .to_ascii_lowercase()
        .contains(&format!("paraid=\"{reply_para}\"")));

    let done = tmp_dir().join("done.docx");
    apply(
        &replied,
        &done,
        json!([{ "op": "setCommentDone", "id": "comment:0", "done": true }]),
    );
    let tree = inspect(&done, json!({ "comments": true }));
    assert_eq!(tree["comments"][0]["done"], true);

    let deleted = tmp_dir().join("del.docx");
    apply(
        &done,
        &deleted,
        json!([{ "op": "deleteComment", "id": "comment:0" }]),
    );
    let tree = inspect(
        &deleted,
        json!({ "from": 1, "maxParas": 20, "comments": true }),
    );
    let ids: Vec<&str> = tree["comments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"comment:0"));
    assert!(!ids.iter().any(|id| tree["comments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == *id && c["parent"] == "comment:0")));
    assert!(ids.contains(&"comment:2"));
    assert!(ids.contains(&"comment:3"));
    assert!(!part_text(&deleted, "word/document.xml").contains(r#"w:id="0""#));
    assert!(!part_text(&deleted, "word/comments.xml").contains("批注甲"));
    assert!(!part_text(&deleted, "word/comments.xml").contains("回复乙"));
    assert!(!part_text(&deleted, "word/comments.xml").contains("新回复"));
}

#[test]
fn set_comment_meta_and_last_para() {
    let path = write_sample("meta");
    let out = tmp_dir().join("meta-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setCommentMeta", "id": "comment:0", "author": "Carol", "initials": "C" }]),
    );
    let tree = inspect(&out, json!({ "comments": true }));
    assert_eq!(tree["comments"][0]["author"], "Carol");
    assert_eq!(tree["comments"][0]["initials"], "C");
    assert_eq!(tree["comments"][0]["date"], "2026-01-01T00:00:00Z");
    assert!(part_text(&out, "word/people.xml").contains("Carol"));

    let err = apply_err(
        &out,
        json!([{ "op": "deletePara", "id": "comment:0/para:C0000001" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let two = tmp_dir().join("two-para.docx");
    apply(
        &out,
        &two,
        json!([{ "op": "insertPara", "id": "comment:0", "text": "第二段" }]),
    );
    let tree = inspect(&two, json!({ "comments": true }));
    assert_eq!(tree["comments"][0]["paraCount"], 2);
    let last_id = tree["comments"][0]["paragraphs"][1]["id"]
        .as_str()
        .unwrap()
        .rsplit(':')
        .next()
        .unwrap();
    let ex = part_text(&two, "word/commentsExtended.xml").to_ascii_lowercase();
    let needle = last_id.to_ascii_lowercase();
    assert!(ex.contains(&format!("paraid=\"{needle}\"")));
    assert!(ex.contains(&format!("paraidparent=\"{needle}\"")));
}

#[test]
fn insert_comment_creates_parts_on_create_file() {
    let path = tmp_dir().join("create-host.docx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "hello" }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let tree0 = inspect(&path, json!({ "from": 1, "maxParas": 5 }));
    assert_eq!(tree0["commentCount"], 0);
    let para = tree0["paragraphs"][0]["id"].as_str().unwrap();
    let out = tmp_dir().join("create-cmt.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "insertComment", "start": para, "end": para, "text": "从无到有" }]),
    );
    let tree = inspect(&out, json!({ "comments": true, "from": 1, "maxParas": 5 }));
    assert_eq!(tree["commentCount"], 1);
    assert_eq!(tree["comments"][0]["id"], "comment:0");
    assert_eq!(tree["comments"][0]["paragraphs"][0]["text"], "从无到有");
    assert!(part_text(&out, "[Content_Types].xml").contains("comments+xml"));
    assert!(part_text(&out, "[Content_Types].xml").contains(
        "application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml"
    ));
    assert!(part_text(&out, "word/_rels/document.xml.rels").contains("/comments"));
}

#[test]
fn insert_picture_in_comment_and_ink_rejected() {
    let path = write_sample("pic");
    let png = write_png("cmt");
    let out = tmp_dir().join("cmt-pic.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertPicture",
            "id": "comment:0/para:C0000001",
            "file": png.to_string_lossy()
        }]),
    );
    let tree = inspect(&out, json!({ "comments": true, "pictures": true }));
    assert_eq!(tree["comments"][0]["pictureCount"], 1);
    assert!(tree["comments"][0]["pictures"][0]["id"]
        .as_str()
        .unwrap()
        .starts_with("comment:0/para:C0000001/pic:"));

    let ink = write_ink_sample("ink");
    let err = apply_err(
        &ink,
        json!([{ "op": "setCommentText", "id": "comment:0", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply_err(
        &ink,
        json!([{ "op": "replyComment", "id": "comment:0", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let meta_out = tmp_dir().join("ink-meta.docx");
    apply(
        &ink,
        &meta_out,
        json!([
            { "op": "setCommentMeta", "id": "comment:0", "author": "Ink2", "initials": "K" },
            { "op": "setCommentDone", "id": "comment:0", "done": true }
        ]),
    );
    let tree = inspect(&meta_out, json!({ "comments": true }));
    assert_eq!(tree["comments"][0]["kind"], "ink");
    assert_eq!(tree["comments"][0]["author"], "Ink2");
    assert_eq!(tree["comments"][0]["initials"], "K");
    assert_eq!(tree["comments"][0]["done"], true);
    let del = tmp_dir().join("ink-del.docx");
    apply(
        &ink,
        &del,
        json!([{ "op": "deleteComment", "id": "comment:0" }]),
    );
    let tree = inspect(&del, json!({ "comments": true }));
    assert_eq!(tree["commentCount"], 0);
}

#[test]
fn rejects_container_id_and_xlsx() {
    let path = write_sample("bad-id");
    let err = apply_err(
        &path,
        json!([{ "op": "setParaText", "id": "comment:0", "text": "x" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertComment",
            "start": "para:AAAA0003",
            "end": "para:AAAA0001",
            "text": "反了"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let xlsx = tmp_dir().join("cmt.xlsx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": xlsx.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": xlsx.to_string_lossy(),
            "output": tmp_dir().join("cmt-x-out.xlsx").to_string_lossy(),
            "ops": [{ "op": "insertComment", "start": "para:1", "end": "para:1", "text": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
