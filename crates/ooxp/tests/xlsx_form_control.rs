//! 窗体控件：三处对上才进树，标题和链接两处一起改。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-xlsx-form");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn create_book(tag: &str) -> PathBuf {
    let path = tmp(tag);
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [
                    { "ref": "A1", "value": "甲" },
                    { "ref": "A2", "value": "乙" }
                ]
            }]
        }
    })
    .to_string())
    .unwrap();
    path
}

fn inspect(path: &Path, scope: Value) -> Value {
    let RunOutput::Inspect(tree) = run(&json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": path.to_string_lossy(),
        "scope": scope
    })
    .to_string())
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn apply(src: &Path, out: &Path, ops: Value) -> Result<RunOutput, ooxp::Error> {
    run(&json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops
    })
    .to_string())
}

fn object_keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

fn part_text(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap_or_else(|_| panic!("缺少 {name}"));
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    text
}

fn rewrite(path: &Path, edit: impl Fn(&str, Vec<u8>) -> Vec<u8>, extra: Vec<(String, Vec<u8>)>) {
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
            data = edit(&name, data);
            zip_out.start_file(&name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        for (name, data) in extra {
            zip_out.start_file(name, opt).unwrap();
            zip_out.write_all(&data).unwrap();
        }
        zip_out.finish().unwrap();
    }
    std::fs::write(path, out.into_inner()).unwrap();
}

fn sp(id: &str, text: Option<&str>) -> String {
    let body = match text {
        Some(text) => {
            format!(r#"<xdr:txBody><a:p><a:r><a:t>{text}</a:t></a:r></a:p></xdr:txBody>"#)
        }
        None => String::new(),
    };
    format!(
        r#"<mc:AlternateContent><mc:Choice Requires="a14"><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="{id}" name="n"><a:extLst><a:ext uri="{{63B3BB69-23CF-44E3-9099-C40C66FF867C}}"><a14:compatExt spid="_x0000_s{id}"/></a:ext></a:extLst></xdr:cNvPr></xdr:nvSpPr>{body}</xdr:sp></mc:Choice><mc:Fallback/></mc:AlternateContent>"#
    )
}

fn seed() -> PathBuf {
    let path = create_book("seed");
    let controls = r#"<drawing r:id="rId1"/><legacyDrawing r:id="rId2"/><controls><mc:AlternateContent><mc:Choice Requires="x14"><control shapeId="1025" r:id="rId3" name="Button 1"/><control shapeId="1026" r:id="rId4" name="Check Box 2"/><control shapeId="1027" r:id="rId5" name="Drop Down 3"/><control shapeId="1999" r:id="rId9" name="孤儿"/><control shapeId="2000" r:id="rId3" name="链接" link="http://example.com"/></mc:Choice></mc:AlternateContent></controls>"#;
    let drawing = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">{button}{check}{drop}</xdr:wsDr>"#,
        button = sp("1025", Some("按钮 1")),
        check = sp("1026", Some("复选框 2")),
        drop = sp("1027", None),
    );
    let vml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="_x0000_s1025"><v:textbox><div><font>按钮 1</font></div></v:textbox><x:ClientData ObjectType="Button"></x:ClientData></v:shape><v:shape id="_x0000_s1026"><v:textbox><div><font>复选框 2</font></div></v:textbox><x:ClientData ObjectType="Checkbox"><x:FmlaLink>$B$1</x:FmlaLink></x:ClientData></v:shape><v:shape id="_x0000_s1027"><x:ClientData ObjectType="Drop"><x:FmlaRange>$A$1:$A$2</x:FmlaRange><x:Sel>1</x:Sel><x:Val>0</x:Val></x:ClientData></v:shape></xml>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/ctrlProp" Target="../ctrlProps/ctrlProp1.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/ctrlProp" Target="../ctrlProps/ctrlProp2.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/ctrlProp" Target="../ctrlProps/ctrlProp3.xml"/></Relationships>"#;
    let button = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><formControlPr xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" objectType="Button" lockText="1"/>"#;
    let check = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><formControlPr xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" objectType="CheckBox" fmlaLink="$B$1" lockText="1"/>"#;
    let drop = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><formControlPr xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" objectType="Drop" fmlaRange="$A$1:$A$2" sel="1" val="0"/>"#;
    let types = r#"<Default Extension="vml" ContentType="application/vnd.openxmlformats-officedocument.vmlDrawing"/><Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/><Override PartName="/xl/ctrlProps/ctrlProp1.xml" ContentType="application/vnd.ms-excel.controlproperties+xml"/><Override PartName="/xl/ctrlProps/ctrlProp2.xml" ContentType="application/vnd.ms-excel.controlproperties+xml"/><Override PartName="/xl/ctrlProps/ctrlProp3.xml" ContentType="application/vnd.ms-excel.controlproperties+xml"/>"#;
    rewrite(
        &path,
        |name, data| {
            let name = name.replace('\\', "/");
            if name.ends_with("xl/worksheets/sheet1.xml") {
                let mut sheet = String::from_utf8(data).unwrap();
                if !sheet.contains("xmlns:mc=") {
                    sheet = sheet.replacen(
                        "<worksheet ",
                        "<worksheet xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" ",
                        1,
                    );
                }
                sheet = sheet.replacen("</worksheet>", &format!("{controls}</worksheet>"), 1);
                return sheet.into_bytes();
            }
            if name.ends_with("[Content_Types].xml") {
                let mut text = String::from_utf8(data).unwrap();
                text = text.replacen("</Types>", &format!("{types}</Types>"), 1);
                return text.into_bytes();
            }
            data
        },
        vec![
            ("xl/drawings/drawing1.xml".into(), drawing.into_bytes()),
            (
                "xl/drawings/vmlDrawing1.vml".into(),
                vml.as_bytes().to_vec(),
            ),
            (
                "xl/worksheets/_rels/sheet1.xml.rels".into(),
                rels.as_bytes().to_vec(),
            ),
            (
                "xl/ctrlProps/ctrlProp1.xml".into(),
                button.as_bytes().to_vec(),
            ),
            (
                "xl/ctrlProps/ctrlProp2.xml".into(),
                check.as_bytes().to_vec(),
            ),
            (
                "xl/ctrlProps/ctrlProp3.xml".into(),
                drop.as_bytes().to_vec(),
            ),
        ],
    );
    path
}

#[test]
fn lists_only_matched_controls() {
    let path = seed();
    let plain = inspect(&path, json!({}));
    assert_eq!(plain["formControlCount"], 3);
    assert_eq!(plain["sheets"][0]["formControlCount"], 3);
    assert!(plain["sheets"][0].get("formControls").is_none());

    let tree = inspect(&path, json!({ "formControls": true }));
    let items = tree["sheets"][0]["formControls"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(
        object_keys(&items[0]),
        [
            "id",
            "shapeId",
            "name",
            "objectType",
            "caption",
            "fmlaLink",
            "fmlaRange",
            "checked",
            "sel",
        ]
    );
    assert_eq!(items[0]["id"], "sheet:Sheet1/control:1025");
    assert_eq!(items[0]["name"], "Button 1");
    assert_eq!(items[0]["objectType"], "Button");
    assert_eq!(items[0]["caption"], "按钮 1");
    assert!(items[0]["fmlaLink"].is_null());
    assert!(items[0]["checked"].is_null());
    assert_eq!(items[1]["objectType"], "CheckBox");
    assert_eq!(items[1]["caption"], "复选框 2");
    assert_eq!(items[1]["fmlaLink"], "$B$1");
    assert_eq!(items[1]["checked"], false);
    assert_eq!(items[2]["objectType"], "Drop");
    assert!(items[2]["caption"].is_null());
    assert_eq!(items[2]["fmlaRange"], "$A$1:$A$2");
    assert_eq!(items[2]["sel"], 1);
    assert!(items[2]["checked"].is_null());
    let ids: Vec<_> = items
        .iter()
        .map(|item| item["shapeId"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"1999"));
    assert!(!ids.contains(&"2000"));
}

#[test]
fn set_writes_both_copies_and_rejects_the_wrong_field() {
    let path = seed();
    let button = "sheet:Sheet1/control:1025";
    let check = "sheet:Sheet1/control:1026";
    let drop = "sheet:Sheet1/control:1027";
    let sheet_before = part_text(&path, "xl/worksheets/sheet1.xml");
    let button_prop = part_text(&path, "xl/ctrlProps/ctrlProp1.xml");

    let err = apply(
        &path,
        &tmp("bad-link"),
        json!([{ "op": "setFormControl", "id": button, "fmlaLink": "$B$1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &path,
        &tmp("bad-caption"),
        json!([{ "op": "setFormControl", "id": drop, "caption": "下拉字" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &path,
        &tmp("bad-empty"),
        json!([{ "op": "setFormControl", "id": button }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply(
        &path,
        &tmp("bad-id"),
        json!([{ "op": "setFormControl", "id": "sheet:Sheet1", "caption": "按钮改" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let err = apply(
        &path,
        &tmp("missing"),
        json!([{ "op": "setFormControl", "id": "sheet:Sheet1/control:1999", "caption": "无" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::IdNotFound);
    assert_eq!(part_text(&path, "xl/worksheets/sheet1.xml"), sheet_before);

    let out = tmp("set");
    apply(
        &path,
        &out,
        json!([
            { "op": "setFormControl", "id": button, "caption": "按钮改" },
            {
                "op": "setFormControl",
                "id": check,
                "caption": "复选改",
                "fmlaLink": "$C$1",
                "checked": true
            },
            { "op": "setFormControl", "id": drop, "sel": 2 }
        ]),
    )
    .unwrap();
    let sheet = part_text(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"r="C1" t="b"><v>1</v>"#), "{sheet}");
    assert_eq!(part_text(&out, "xl/ctrlProps/ctrlProp1.xml"), button_prop);
    let drawing = part_text(&out, "xl/drawings/drawing1.xml");
    let vml = part_text(&out, "xl/drawings/vmlDrawing1.vml");
    assert!(drawing.contains("按钮改"));
    assert!(!drawing.contains("按钮 1"));
    assert!(vml.contains("按钮改"));
    assert!(!vml.contains("按钮 1"));
    assert!(drawing.contains("复选改"));
    assert!(vml.contains("复选改"));
    let prop = part_text(&out, "xl/ctrlProps/ctrlProp2.xml");
    assert!(prop.contains("fmlaLink=\"$C$1\""));
    assert!(prop.contains("checked=\"Checked\""));
    assert!(vml.contains("<x:FmlaLink>$C$1</x:FmlaLink>"));
    assert!(vml.contains("<x:Checked>1</x:Checked>"));
    let drop_prop = part_text(&out, "xl/ctrlProps/ctrlProp3.xml");
    assert!(drop_prop.contains("sel=\"2\""));
    assert!(drop_prop.contains("val=\"0\""));
    assert!(vml.contains("<x:Sel>2</x:Sel>"));
    assert!(vml.contains("<x:Val>0</x:Val>"));

    let tree = inspect(&out, json!({ "formControls": true }));
    let items = &tree["sheets"][0]["formControls"];
    assert_eq!(items[0]["caption"], "按钮改");
    assert_eq!(items[1]["caption"], "复选改");
    assert_eq!(items[1]["fmlaLink"], "$C$1");
    assert_eq!(items[1]["checked"], true);
    assert_eq!(items[2]["sel"], 2);

    let cleared = tmp("clear");
    apply(
        &out,
        &cleared,
        json!([{
            "op": "setFormControl",
            "id": check,
            "fmlaLink": null,
            "checked": false
        }]),
    )
    .unwrap();
    let prop = part_text(&cleared, "xl/ctrlProps/ctrlProp2.xml");
    let vml = part_text(&cleared, "xl/drawings/vmlDrawing1.vml");
    assert!(!prop.contains("fmlaLink"));
    assert!(!prop.contains("checked"));
    assert!(!vml.contains("FmlaLink"));
    assert!(!vml.contains("Checked"));
    let tree = inspect(&cleared, json!({ "formControls": true }));
    assert!(tree["sheets"][0]["formControls"][1]["fmlaLink"].is_null());
    assert_eq!(tree["sheets"][0]["formControls"][1]["checked"], false);
    let sheet = part_text(&cleared, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"r="C1" t="b"><v>0</v>"#), "{sheet}");
}
