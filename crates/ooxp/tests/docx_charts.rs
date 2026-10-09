//! docx 嵌入图表：计数、名单、改系列、插删、改类型、导出 embedding。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#;

const CHART_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="../embeddings/Microsoft_Excel_Worksheet1.xlsx"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart1.xml"/></Relationships>"#;

const CT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="xlsx" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/></Types>"#;

const SER0: &str = r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>系列甲</c:v></c:pt></c:strCache></c:strRef></c:tx><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>甲</c:v></c:pt><c:pt idx="1"><c:v>乙</c:v></c:pt><c:pt idx="2"><c:v>丙</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>"#;

const SER1: &str = r#"<c:ser><c:idx val="1"/><c:order val="1"/><c:tx><c:strRef><c:f>Sheet1!$C$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>系列乙</c:v></c:pt></c:strCache></c:strRef></c:tx><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>甲</c:v></c:pt><c:pt idx="1"><c:v>乙</c:v></c:pt><c:pt idx="2"><c:v>丙</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$C$2:$C$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>30</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>"#;

const AXES: &str = r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>分类</a:t></a:r></a:p></c:rich></c:tx></c:title><c:crossAx val="2"/></c:catAx><c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>数值</a:t></a:r></a:p></c:rich></c:tx></c:title><c:crossAx val="1"/></c:valAx>"#;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-docx-charts");
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

fn embed_xlsx() -> Vec<u8> {
    let path = tmp_dir().join(format!(
        "embed-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A2", "value": "甲" },
                        { "ref": "A3", "value": "乙" },
                        { "ref": "A4", "value": "丙" },
                        { "ref": "B1", "value": "系列甲" },
                        { "ref": "B2", "value": 1 },
                        { "ref": "B3", "value": 2 },
                        { "ref": "B4", "value": 3 },
                        { "ref": "C1", "value": "系列乙" },
                        { "ref": "C2", "value": 10 },
                        { "ref": "C3", "value": 20 },
                        { "ref": "C4", "value": 30 }
                    ]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    std::fs::read(&path).unwrap()
}

fn document_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><w:body><w:p w14:paraId="AAAA0001"><w:r><w:t>带图</w:t></w:r><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="5486400" cy="3200400"/><wp:docPr id="7" name="图表 1"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="rId2"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#.into()
}

fn chart_xml(combo: bool) -> String {
    let plot = if combo {
        format!(
            r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{SER0}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:lineChart><c:grouping val="standard"/>{SER1}<c:axId val="1"/><c:axId val="2"/></c:lineChart>"#
        )
    } else {
        format!(
            r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{SER0}{SER1}<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/></c:dLbls><c:axId val="1"/><c:axId val="2"/></c:barChart>"#
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>销量</a:t></a:r></a:p></c:rich></c:tx></c:title><c:autoTitleDeleted val="0"/><c:plotArea><c:layout/>{plot}{AXES}</c:plotArea><c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/></c:chart><c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData></c:chartSpace>"#
    )
}

fn write_sample(tag: &str, combo: bool) -> PathBuf {
    let embed = embed_xlsx();
    let doc = document_xml();
    let chart = chart_xml(combo);
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/charts/chart1.xml", chart.as_bytes()),
            ("word/charts/_rels/chart1.xml.rels", CHART_RELS.as_bytes()),
            ("word/embeddings/Microsoft_Excel_Worksheet1.xlsx", &embed),
        ],
    )
}

fn inspect(path: &Path, scope: Value) -> Value {
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

fn apply(path: &Path, out: &Path, ops: Value) {
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

fn apply_err(path: &Path, ops: Value) -> ooxp::Error {
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

fn object_keys(v: &Value) -> Vec<&str> {
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

fn nums(v: &Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)).unwrap())
        .collect()
}

#[test]
fn inspect_count_without_list() {
    let path = write_sample("count", false);
    let tree = inspect(&path, json!({}));
    assert_eq!(tree["chartCount"], 1);
    assert!(tree.get("charts").is_none());
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
    assert_eq!(tree["headers"], json!([]));
}

#[test]
fn inspect_lists_type_title_and_series() {
    let path = write_sample("list", false);
    let tree = inspect(&path, json!({ "charts": true }));
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(object_keys(&tree)[object_keys(&tree).len() - 1], "charts");
    let c = &tree["charts"][0];
    assert_eq!(c["id"], "para:AAAA0001/chart:7");
    assert_eq!(c["docPrId"], "7");
    assert_eq!(c["name"], "图表 1");
    assert_eq!(c["kind"], "inline");
    assert_eq!(c["cx"], 5486400);
    assert_eq!(c["cy"], 3200400);
    assert!(c["wrap"].is_null());
    assert_eq!(c["type"], "col");
    assert_eq!(c["title"], "销量");
    assert_eq!(c["catAxisTitle"], "分类");
    assert_eq!(c["valAxisTitle"], "数值");
    assert_eq!(c["legend"]["show"], true);
    assert_eq!(c["legend"]["pos"], "r");
    assert_eq!(c["dataLabels"]["show"], false);
    assert_eq!(c["flags"], json!([]));
    assert_eq!(c["stable"], true);
    assert_eq!(
        object_keys(c),
        [
            "id",
            "docPrId",
            "name",
            "kind",
            "cx",
            "cy",
            "wrap",
            "type",
            "title",
            "catAxisTitle",
            "valAxisTitle",
            "series",
            "legend",
            "dataLabels",
            "flags",
            "stable"
        ]
    );
    let s0 = &c["series"][0];
    assert_eq!(s0["id"], "para:AAAA0001/chart:7/series:0");
    assert_eq!(s0["index"], 0);
    assert_eq!(s0["name"], "系列甲");
    assert_eq!(s0["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(nums(&s0["values"]), vec![1, 2, 3]);
    assert!(s0.get("cacheStale").is_none());
    assert_eq!(
        object_keys(s0),
        ["id", "index", "name", "categories", "values", "axis"]
    );
    let s1 = &c["series"][1];
    assert_eq!(s1["name"], "系列乙");
    assert_eq!(nums(&s1["values"]), vec![10, 20, 30]);
}

#[test]
fn set_series_updates_cache_and_embedding_leaves_other() {
    let path = write_sample("set-ser", false);
    let out = tmp_dir().join("set-ser-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [100, 200, 300]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let s0 = &tree["charts"][0]["series"][0];
    let s1 = &tree["charts"][0]["series"][1];
    assert_eq!(nums(&s0["values"]), vec![100, 200, 300]);
    assert_eq!(nums(&s1["values"]), vec![10, 20, 30]);
    assert!(s0.get("cacheStale").is_none());
    let chart = part_text(&out, "word/charts/chart1.xml");
    assert!(chart.contains("<c:v>100</c:v>"));
    assert!(chart.contains("<c:v>200</c:v>"));
    assert!(chart.contains("<c:v>300</c:v>"));
    assert!(chart.contains("<c:v>10</c:v>"));
    assert!(chart.contains("<c:v>20</c:v>"));
    assert!(chart.contains("<c:v>30</c:v>"));
    let xlsx = tmp_dir().join("set-ser-embed.xlsx");
    let RunOutput::Written { .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": out.to_string_lossy(),
            "id": "para:AAAA0001/chart:7",
            "output": xlsx.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportChartData 应写出文件");
    };
    let RunOutput::Inspect(book) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "B1:C4", "maxCells": 20 }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!();
    };
    let cells = book["sheets"][0]["cells"].as_array().unwrap();
    let by_ref: std::collections::HashMap<_, _> = cells
        .iter()
        .map(|c| (c["ref"].as_str().unwrap().to_string(), c["value"].clone()))
        .collect();
    assert_eq!(
        by_ref["B2"]
            .as_i64()
            .or_else(|| by_ref["B2"].as_f64().map(|f| f as i64)),
        Some(100)
    );
    assert_eq!(
        by_ref["B3"]
            .as_i64()
            .or_else(|| by_ref["B3"].as_f64().map(|f| f as i64)),
        Some(200)
    );
    assert_eq!(
        by_ref["B4"]
            .as_i64()
            .or_else(|| by_ref["B4"].as_f64().map(|f| f as i64)),
        Some(300)
    );
    assert_eq!(
        by_ref["C2"]
            .as_i64()
            .or_else(|| by_ref["C2"].as_f64().map(|f| f as i64)),
        Some(10)
    );
    assert_eq!(
        by_ref["C4"]
            .as_i64()
            .or_else(|| by_ref["C4"].as_f64().map(|f| f as i64)),
        Some(30)
    );
}

#[test]
fn insert_pie_then_delete_original() {
    let path = write_sample("ins", false);
    let out = tmp_dir().join("ins-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "pie",
            "title": "份额",
            "series": [{ "name": "份额", "categories": ["A", "B"], "values": [1, 2] }]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["chartCount"], 2);
    let pie = tree["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "pie")
        .unwrap();
    assert_eq!(pie["title"], "份额");
    assert_eq!(pie["kind"], "anchor");
    assert_eq!(pie["wrap"], "square");
    assert_eq!(pie["series"].as_array().unwrap().len(), 1);
    assert_eq!(nums(&pie["series"][0]["values"]), vec![1, 2]);
    assert!(part_text(&out, "[Content_Types].xml").contains("chart+xml"));
    assert!(part_text(&out, "word/_rels/document.xml.rels").contains("/chart"));

    let del = tmp_dir().join("del-out.docx");
    apply(
        &out,
        &del,
        json!([{ "op": "deleteChart", "id": "para:AAAA0001/chart:7" }]),
    );
    let tree = inspect(&del, json!({ "charts": true }));
    assert_eq!(tree["chartCount"], 1);
    assert_eq!(tree["charts"][0]["type"], "pie");
    let names: Vec<String> = {
        let bytes = std::fs::read(&del).unwrap();
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect()
    };
    assert!(!names.iter().any(|n| n == "word/charts/chart1.xml"));
}

#[test]
fn set_type_to_line() {
    let path = write_sample("line", false);
    let out = tmp_dir().join("line-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "line" }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "line");
    assert!(part_text(&out, "word/charts/chart1.xml").contains("lineChart"));
}

#[test]
fn combo_set_series_keeps_wrappers() {
    let path = write_sample("combo-ser", true);
    let tree = inspect(&path, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "combo");
    let out = tmp_dir().join("combo-ser-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [4, 5, 6]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "combo");
    assert_eq!(
        nums(&tree["charts"][0]["series"][0]["values"]),
        vec![4, 5, 6]
    );
    assert_eq!(
        nums(&tree["charts"][0]["series"][1]["values"]),
        vec![10, 20, 30]
    );
    let xml = part_text(&out, "word/charts/chart1.xml");
    assert!(xml.contains("barChart"));
    assert!(xml.contains("lineChart"));
}

#[test]
fn combo_set_type_collapses_to_col() {
    let path = write_sample("combo-col", true);
    let out = tmp_dir().join("combo-col-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "col" }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "col");
    assert_eq!(tree["charts"][0]["series"].as_array().unwrap().len(), 2);
    let xml = part_text(&out, "word/charts/chart1.xml");
    assert!(xml.contains("barChart"));
    assert!(!xml.contains("lineChart"));
}

#[test]
fn combo_set_type_combo_is_invalid_field() {
    let path = write_sample("combo-bad", true);
    let err = apply_err(
        &path,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "combo" }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn export_chart_data_matches_embedding_bytes() {
    let path = write_sample("exp", false);
    let embed = part_bytes(&path, "word/embeddings/Microsoft_Excel_Worksheet1.xlsx");
    let out = tmp_dir().join("chart-data.xlsx");
    let RunOutput::Written { content_type, .. } = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": path.to_string_lossy(),
            "id": "para:AAAA0001/chart:7",
            "output": out.to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("exportChartData 应写出文件");
    };
    assert_eq!(std::fs::read(&out).unwrap(), embed);
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    );
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": path.to_string_lossy(),
            "id": "para:AAAA0001",
            "output": tmp_dir().join("bad.xlsx").to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

#[test]
fn add_and_delete_series_and_title_legend_labels() {
    let path = write_sample("more", false);
    let out = tmp_dir().join("more-out.docx");
    apply(
        &path,
        &out,
        json!([
            { "op": "setChartTitle", "id": "para:AAAA0001/chart:7", "title": "新标题" },
            { "op": "setChartLegend", "id": "para:AAAA0001/chart:7", "show": false },
            { "op": "setChartDataLabels", "id": "para:AAAA0001/chart:7", "show": true },
            {
                "op": "addChartSeries",
                "id": "para:AAAA0001/chart:7",
                "name": "系列丙",
                "values": [7, 8, 9]
            }
        ]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let c = &tree["charts"][0];
    assert_eq!(c["title"], "新标题");
    assert_eq!(c["legend"]["show"], false);
    assert_eq!(c["dataLabels"]["show"], true);
    assert_eq!(c["series"].as_array().unwrap().len(), 3);
    assert_eq!(c["series"][2]["name"], "系列丙");
    assert_eq!(nums(&c["series"][2]["values"]), vec![7, 8, 9]);

    let del = tmp_dir().join("more-del.docx");
    apply(
        &out,
        &del,
        json!([{ "op": "deleteChartSeries", "id": "para:AAAA0001/chart:7/series:2" }]),
    );
    let tree = inspect(&del, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["series"].as_array().unwrap().len(), 2);
    let err = apply_err(
        &del,
        json!([
            { "op": "deleteChartSeries", "id": "para:AAAA0001/chart:7/series:0" },
            { "op": "deleteChartSeries", "id": "para:AAAA0001/chart:7/series:0" }
        ]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn xlsx_and_pptx_reject_charts() {
    let xlsx = tmp_dir().join("no-chart.xlsx");
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
            "output": tmp_dir().join("no-chart-out.xlsx").to_string_lossy(),
            "ops": [{
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "pie",
                "series": [{ "values": [1] }]
            }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": xlsx.to_string_lossy(),
            "scope": { "charts": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("xlsx 现在认 scope.charts");
    };
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["sheets"][0]["charts"], json!([]));
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "exportChartData",
            "path": xlsx.to_string_lossy(),
            "id": "para:AAAA0001/chart:7",
            "output": tmp_dir().join("no.xlsx").to_string_lossy()
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);

    let pptx = tmp_dir().join("no-chart.pptx");
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": pptx.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "hi" }] }] }
        })
        .to_string(),
        None,
    )
    .unwrap();
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": pptx.to_string_lossy(),
            "output": tmp_dir().join("no-chart-out.pptx").to_string_lossy(),
            "ops": [{ "op": "setChartTitle", "id": "para:AAAA0001/chart:7", "title": "x" }]
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidId);
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": pptx.to_string_lossy(),
            "scope": { "charts": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("pptx 现在认 scope.charts");
    };
    assert_eq!(tree["chartCount"], 0);
    assert_eq!(tree["slides"][0]["charts"], json!([]));
}

#[test]
fn pie_insert_requires_one_series() {
    let path = write_sample("pie-bad", false);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "pie",
            "series": [
                { "values": [1, 2] },
                { "values": [3, 4] }
            ]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{ "op": "insertChart", "id": "para:AAAA0001/chart:7", "type": "col", "series": [{ "values": [1] }] }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidId);
}

const BUBBLE_SER: &str = r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>系列甲</c:v></c:pt></c:strCache></c:strRef></c:tx><c:xVal><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>甲</c:v></c:pt><c:pt idx="1"><c:v>乙</c:v></c:pt><c:pt idx="2"><c:v>丙</c:v></c:pt></c:strCache></c:strRef></c:xVal><c:yVal><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt></c:numCache></c:numRef></c:yVal><c:bubbleSize><c:numRef><c:f>Sheet1!$C$2:$C$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>4</c:v></c:pt><c:pt idx="1"><c:v>5</c:v></c:pt><c:pt idx="2"><c:v>6</c:v></c:pt></c:numCache></c:numRef></c:bubbleSize></c:ser>"#;

const XY_AXES: &str = r#"<c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/></c:valAx><c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="1"/></c:valAx>"#;

fn embed_xlsx_bubble() -> Vec<u8> {
    let path = tmp_dir().join(format!(
        "embed-b-{}.xlsx",
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A2", "value": "甲" },
                        { "ref": "A3", "value": "乙" },
                        { "ref": "A4", "value": "丙" },
                        { "ref": "B1", "value": "系列甲" },
                        { "ref": "B2", "value": 1 },
                        { "ref": "B3", "value": 2 },
                        { "ref": "B4", "value": 3 },
                        { "ref": "C2", "value": 4 },
                        { "ref": "C3", "value": 5 },
                        { "ref": "C4", "value": 6 }
                    ]
                }]
            }
        })
        .to_string(),
        None,
    )
    .unwrap();
    std::fs::read(&path).unwrap()
}

fn chart_xml_kind(kind: &str) -> String {
    let (plot, axes, extra) = match kind {
        "doughnut" => (
            format!(
                r#"<c:doughnutChart><c:varyColors val="1"/>{SER0}<c:holeSize val="50"/></c:doughnutChart>"#
            ),
            String::new(),
            "",
        ),
        "radar" => (
            format!(
                r#"<c:radarChart><c:radarStyle val="marker"/>{SER0}{SER1}<c:axId val="1"/><c:axId val="2"/></c:radarChart>"#
            ),
            AXES.to_string(),
            "",
        ),
        "bubble" => (
            format!(
                r#"<c:bubbleChart><c:varyColors val="0"/>{BUBBLE_SER}<c:bubbleScale val="100"/><c:axId val="1"/><c:axId val="2"/></c:bubbleChart>"#
            ),
            XY_AXES.to_string(),
            "",
        ),
        "stock" => (
            format!(
                r#"<c:stockChart>{SER0}{SER1}<c:axId val="1"/><c:axId val="2"/></c:stockChart>"#
            ),
            AXES.to_string(),
            "",
        ),
        "ofPie" => (
            format!(r#"<c:ofPieChart><c:ofPieType val="pie"/>{SER0}</c:ofPieChart>"#),
            String::new(),
            "",
        ),
        "surface" => (
            format!(
                r#"<c:surfaceChart>{SER0}{SER1}<c:axId val="1"/><c:axId val="2"/><c:axId val="3"/></c:surfaceChart>"#
            ),
            format!(
                r#"{AXES}<c:serAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/></c:serAx>"#
            ),
            "",
        ),
        "pivot" => (
            format!(
                r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{SER0}{SER1}<c:axId val="1"/><c:axId val="2"/></c:barChart>"#
            ),
            AXES.to_string(),
            r#"<c:pivotSource><c:name>[0]!Pivot</c:name><c:fmtId val="0"/></c:pivotSource>"#,
        ),
        _ => panic!("未知 kind {kind}"),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{extra}<c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>销量</a:t></a:r></a:p></c:rich></c:tx></c:title><c:autoTitleDeleted val="0"/><c:plotArea><c:layout/>{plot}{axes}</c:plotArea><c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/></c:chart><c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData></c:chartSpace>"#
    )
}

fn write_kind(tag: &str, kind: &str) -> PathBuf {
    let embed = if kind == "bubble" {
        embed_xlsx_bubble()
    } else {
        embed_xlsx()
    };
    let doc = document_xml();
    let chart = chart_xml_kind(kind);
    write_zip(
        tag,
        &[
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("word/document.xml", doc.as_bytes()),
            ("word/_rels/document.xml.rels", DOC_RELS.as_bytes()),
            ("word/styles.xml", STYLES.as_bytes()),
            ("word/charts/chart1.xml", chart.as_bytes()),
            ("word/charts/_rels/chart1.xml.rels", CHART_RELS.as_bytes()),
            ("word/embeddings/Microsoft_Excel_Worksheet1.xlsx", &embed),
        ],
    )
}

#[test]
fn inspect_doughnut_radar_bubble_types() {
    let d = write_kind("d", "doughnut");
    let r = write_kind("r", "radar");
    let b = write_kind("b", "bubble");
    let td = inspect(&d, json!({ "charts": true }));
    let tr = inspect(&r, json!({ "charts": true }));
    let tb = inspect(&b, json!({ "charts": true }));
    assert_eq!(td["charts"][0]["type"], "doughnut");
    assert_eq!(tr["charts"][0]["type"], "radar");
    assert_eq!(tb["charts"][0]["type"], "bubble");
    let s = &tb["charts"][0]["series"][0];
    assert_eq!(
        object_keys(s),
        [
            "id",
            "index",
            "name",
            "categories",
            "values",
            "sizes",
            "axis"
        ]
    );
    assert_eq!(s["categories"], json!(["甲", "乙", "丙"]));
    assert_eq!(nums(&s["values"]), vec![1, 2, 3]);
    assert_eq!(nums(&s["sizes"]), vec![4, 5, 6]);
}

#[test]
fn set_doughnut_and_radar_series() {
    let path = write_kind("dn-set", "doughnut");
    let out = tmp_dir().join("dn-set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [9, 8, 7]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "doughnut");
    assert_eq!(
        nums(&tree["charts"][0]["series"][0]["values"]),
        vec![9, 8, 7]
    );

    let path = write_kind("rd-set", "radar");
    let out = tmp_dir().join("rd-set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [11, 12, 13]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "radar");
    assert_eq!(
        nums(&tree["charts"][0]["series"][0]["values"]),
        vec![11, 12, 13]
    );
}

#[test]
fn set_bubble_values_and_sizes() {
    let path = write_kind("bb-set", "bubble");
    let out = tmp_dir().join("bb-set-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [10, 20, 30],
            "sizes": [40, 50, 60]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let s = &tree["charts"][0]["series"][0];
    assert_eq!(nums(&s["values"]), vec![10, 20, 30]);
    assert_eq!(nums(&s["sizes"]), vec![40, 50, 60]);
    let err = apply_err(
        &write_sample("sizes-col", false),
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "sizes": [1, 2, 3]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_doughnut_radar_bubble_combo() {
    let path = write_sample("ins-ext", false);
    let out = tmp_dir().join("ins-ext-out.docx");
    apply(
        &path,
        &out,
        json!([
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "doughnut",
                "title": "环",
                "series": [{ "name": "环", "categories": ["A", "B"], "values": [1, 2] }]
            },
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "radar",
                "series": [{ "name": "雷", "categories": ["A", "B"], "values": [3, 4] }]
            },
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "bubble",
                "series": [{ "name": "泡", "categories": [1, 2], "values": [3, 4], "sizes": [5, 6] }]
            },
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "combo",
                "series": [
                    { "type": "col", "name": "柱", "categories": ["A", "B"], "values": [1, 2] },
                    { "type": "line", "name": "线", "categories": ["A", "B"], "values": [3, 4] }
                ]
            }
        ]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let types: Vec<&str> = tree["charts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"doughnut"));
    assert!(types.contains(&"radar"));
    assert!(types.contains(&"bubble"));
    assert!(types.contains(&"combo"));
    let bubble = tree["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "bubble")
        .unwrap();
    assert_eq!(nums(&bubble["series"][0]["sizes"]), vec![5, 6]);
    let combo = tree["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "combo")
        .unwrap();
    assert_eq!(combo["series"].as_array().unwrap().len(), 2);
}

#[test]
fn insert_doughnut_two_series_and_combo_rules() {
    let path = write_sample("ins-bad", false);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "doughnut",
            "series": [{ "values": [1] }, { "values": [2] }]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "combo",
            "series": [
                { "type": "col", "values": [1, 2] },
                { "type": "col", "values": [3, 4] }
            ]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "col",
            "series": [{ "type": "line", "values": [1, 2] }]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn set_type_pie_doughnut_and_scatter_bubble() {
    let path = write_kind("pie-dn", "doughnut");
    let out = tmp_dir().join("pie-dn-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "pie" }]),
    );
    assert_eq!(
        inspect(&out, json!({ "charts": true }))["charts"][0]["type"],
        "pie"
    );

    let path = write_kind("bb-sc", "bubble");
    let out = tmp_dir().join("bb-sc-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "scatter" }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "scatter");
    assert!(tree["charts"][0]["series"][0].get("sizes").is_none());
}

#[test]
fn pivot_title_ok_series_blocked() {
    let path = write_kind("pv", "pivot");
    let tree = inspect(&path, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["flags"], json!(["pivot"]));
    let out = tmp_dir().join("pv-title.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartTitle", "id": "para:AAAA0001/chart:7", "title": "透视标题" }]),
    );
    assert_eq!(
        inspect(&out, json!({ "charts": true }))["charts"][0]["title"],
        "透视标题"
    );
    let before = std::fs::read(&path).unwrap();
    let err = apply_err(
        &path,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [1, 2, 3]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn stock_set_type_to_col() {
    let path = write_kind("st", "stock");
    let tree = inspect(&path, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "stock");
    assert_eq!(tree["charts"][0]["series"][0]["axis"], "primary");
    let out = tmp_dir().join("st-col-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "col" }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "col");
    assert!(part_text(&out, "word/charts/chart1.xml").contains("barChart"));
}

#[test]
fn of_pie_and_surface_types() {
    let p = write_kind("of", "ofPie");
    let s = write_kind("sf", "surface");
    let pt = inspect(&p, json!({ "charts": true }));
    assert_eq!(pt["charts"][0]["type"], "ofPie");
    assert_eq!(pt["charts"][0]["ofPieType"], "pie");
    assert_eq!(
        inspect(&s, json!({ "charts": true }))["charts"][0]["type"],
        "surface"
    );
}

#[test]
fn stock_set_series_and_reject_add() {
    let path = write_kind("st-ser", "stock");
    let out = tmp_dir().join("st-ser-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeries",
            "id": "para:AAAA0001/chart:7/series:0",
            "values": [7, 8, 9]
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "stock");
    assert_eq!(
        nums(&tree["charts"][0]["series"][0]["values"]),
        vec![7, 8, 9]
    );
    let err = apply_err(
        &out,
        json!([{
            "op": "addChartSeries",
            "id": "para:AAAA0001/chart:7",
            "values": [1, 2, 3]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
}

#[test]
fn trendline_insert_set_delete() {
    let path = write_sample("tl", false);
    let out = tmp_dir().join("tl-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertTrendline",
            "id": "para:AAAA0001/chart:7/series:0",
            "type": "linear"
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let s0 = &tree["charts"][0]["series"][0];
    assert_eq!(s0["axis"], "primary");
    assert_eq!(
        s0["trendlines"][0]["id"],
        "para:AAAA0001/chart:7/series:0/trendline:1"
    );
    assert_eq!(s0["trendlines"][0]["type"], "linear");
    let flags = tree["charts"][0]["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f == "trendline"));
    let out2 = tmp_dir().join("tl-set.docx");
    apply(
        &out,
        &out2,
        json!([{
            "op": "setTrendline",
            "id": "para:AAAA0001/chart:7/series:0/trendline:1",
            "type": "exp"
        }]),
    );
    let tree2 = inspect(&out2, json!({ "charts": true }));
    assert_eq!(
        tree2["charts"][0]["series"][0]["trendlines"][0]["type"],
        "exp"
    );
    let out3 = tmp_dir().join("tl-del.docx");
    apply(
        &out2,
        &out3,
        json!([{
            "op": "deleteTrendline",
            "id": "para:AAAA0001/chart:7/series:0/trendline:1"
        }]),
    );
    let tree3 = inspect(&out3, json!({ "charts": true }));
    assert!(tree3["charts"][0]["series"][0].get("trendlines").is_none());
}

#[test]
fn error_bars_insert_delete() {
    let path = write_sample("eb", false);
    let out = tmp_dir().join("eb-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "insertErrorBars",
            "id": "para:AAAA0001/chart:7/series:0",
            "valType": "fixedVal",
            "value": 1.5
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let eb = &tree["charts"][0]["series"][0]["errorBars"][0];
    assert_eq!(eb["id"], "para:AAAA0001/chart:7/series:0/errorBars:1");
    assert_eq!(eb["dir"], "y");
    assert_eq!(eb["valType"], "fixedVal");
    assert_eq!(eb["value"], 1.5);
    let out2 = tmp_dir().join("eb-del.docx");
    apply(
        &out,
        &out2,
        json!([{
            "op": "deleteErrorBars",
            "id": "para:AAAA0001/chart:7/series:0/errorBars:1"
        }]),
    );
    let tree2 = inspect(&out2, json!({ "charts": true }));
    assert!(tree2["charts"][0]["series"][0].get("errorBars").is_none());
}

#[test]
fn series_secondary_axis() {
    let path = write_sample("ax", false);
    let out = tmp_dir().join("ax-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartSeriesAxis",
            "id": "para:AAAA0001/chart:7/series:1",
            "axis": "secondary"
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["series"][0]["axis"], "primary");
    assert_eq!(tree["charts"][0]["series"][1]["axis"], "secondary");
    let flags = tree["charts"][0]["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f == "secondaryAxis"));
    let xml = part_text(&out, "word/charts/chart1.xml");
    let second = xml.split("<c:barChart>").nth(2).unwrap_or("");
    let wrap_end = second.find("</c:barChart>").unwrap_or(second.len());
    let wrap = &second[..wrap_end];
    let ser_at = wrap.find("<c:ser>").unwrap_or(usize::MAX);
    let dl_at = wrap.find("<c:dLbls>").unwrap_or(usize::MAX);
    assert!(ser_at < dl_at, "次轴包裹里 ser 必须在 dLbls 之前：{wrap}");
    assert!(
        !wrap.contains(r#"<c:axId val="1"/>"#),
        "次轴包裹不能共用主分类轴：{wrap}"
    );
    assert!(
        xml.contains(r#"<c:delete val="1"/>"#),
        "次轴必须带隐藏分类轴"
    );
}

#[test]
fn pie_rejects_trendline() {
    let path = write_kind("pie-tl", "doughnut");
    let err = apply_err(
        &path,
        json!([{
            "op": "insertTrendline",
            "id": "para:AAAA0001/chart:7/series:0",
            "type": "linear"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_stock_surface_ofpie() {
    let path = write_sample("ins-rare", false);
    let out = tmp_dir().join("ins-rare-out.docx");
    apply(
        &path,
        &out,
        json!([
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "stock",
                "title": "股价",
                "series": [
                    { "name": "高", "categories": ["甲", "乙", "丙"], "values": [30, 40, 50] },
                    { "name": "低", "categories": ["甲", "乙", "丙"], "values": [10, 15, 20] },
                    { "name": "收", "categories": ["甲", "乙", "丙"], "values": [20, 25, 35] }
                ]
            },
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "surface",
                "series": [
                    { "name": "甲", "categories": ["A", "B"], "values": [1, 2] },
                    { "name": "乙", "categories": ["A", "B"], "values": [3, 3] },
                    { "name": "丙", "categories": ["A", "B"], "values": [2, 4] }
                ]
            },
            {
                "op": "insertChart",
                "id": "para:AAAA0001",
                "type": "ofPie",
                "ofPieType": "bar",
                "series": [{ "name": "份额", "categories": ["A", "B", "C"], "values": [1, 2, 3] }]
            }
        ]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    let charts = tree["charts"].as_array().unwrap();
    let stock = charts.iter().find(|c| c["type"] == "stock").unwrap();
    assert_eq!(stock["series"].as_array().unwrap().len(), 3);
    let xml = part_text(&out, "word/charts/chart2.xml");
    assert!(xml.contains("stockChart"));
    assert!(xml.contains("hiLowLines"));
    let surface = charts.iter().find(|c| c["type"] == "surface").unwrap();
    assert_eq!(surface["series"].as_array().unwrap().len(), 3);
    let sxml = part_text(&out, "word/charts/chart3.xml");
    assert!(sxml.contains("surface3DChart"));
    assert!(sxml.contains("view3D"));
    assert!(sxml.contains("serAx"));
    let ofpie = charts.iter().find(|c| c["type"] == "ofPie").unwrap();
    assert_eq!(ofpie["ofPieType"], "bar");
    let oxml = part_text(&out, "word/charts/chart4.xml");
    assert!(oxml.contains("ofPieChart"));
    assert!(oxml.contains(r#"<c:ofPieType val="bar"/>"#));
    assert!(oxml.contains(r#"gapWidth val="150""#));
    let out2 = tmp_dir().join("ins-rare-type-out.docx");
    apply(
        &out,
        &out2,
        json!([
            { "op": "setChartType", "id": stock["id"], "type": "col" },
            { "op": "setChartType", "id": ofpie["id"], "type": "pie" }
        ]),
    );
    let tree2 = inspect(&out2, json!({ "charts": true }));
    let charts2 = tree2["charts"].as_array().unwrap();
    assert_eq!(
        charts2.iter().find(|c| c["id"] == stock["id"]).unwrap()["type"],
        "col"
    );
    assert_eq!(
        charts2.iter().find(|c| c["id"] == ofpie["id"]).unwrap()["type"],
        "pie"
    );
}

#[test]
fn insert_rare_rejects_wrong_series_and_chartex() {
    let path = write_sample("ins-rare-bad", false);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "stock",
            "series": [
                { "values": [1, 2] },
                { "values": [3, 4] }
            ]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "insertChart",
            "id": "para:AAAA0001",
            "type": "chartex",
            "series": [{ "values": [1, 2] }]
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
    let err = apply_err(
        &path,
        json!([{
            "op": "setChartType",
            "id": "para:AAAA0001/chart:7",
            "type": "stock"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    let err = apply_err(
        &path,
        json!([{
            "op": "setChartType",
            "id": "para:AAAA0001/chart:7",
            "type": "col",
            "ofPieType": "pie"
        }]),
    );
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn ofpie_set_type_to_pie_and_ofpie_type() {
    let path = write_kind("of-pie", "ofPie");
    let out = tmp_dir().join("of-pie-out.docx");
    apply(
        &path,
        &out,
        json!([{ "op": "setChartType", "id": "para:AAAA0001/chart:7", "type": "pie" }]),
    );
    assert_eq!(
        inspect(&out, json!({ "charts": true }))["charts"][0]["type"],
        "pie"
    );
    let path = write_kind("of-bar", "ofPie");
    let out = tmp_dir().join("of-bar-out.docx");
    apply(
        &path,
        &out,
        json!([{
            "op": "setChartType",
            "id": "para:AAAA0001/chart:7",
            "type": "ofPie",
            "ofPieType": "bar"
        }]),
    );
    let tree = inspect(&out, json!({ "charts": true }));
    assert_eq!(tree["charts"][0]["type"], "ofPie");
    assert_eq!(tree["charts"][0]["ofPieType"], "bar");
}
