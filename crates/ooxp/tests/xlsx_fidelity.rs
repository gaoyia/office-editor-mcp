//! 保真度回归测试。
//!
//! 样例刻意照 Excel 真实存盘的写法造：行上省略 `r`、单元格带样式索引 `s`、文本走共享字符串、
//! 有一组共享公式、未改表带注释、还有一个内容类型只靠 Default 认出来的未知部件。
//! 库自己 `create` 出来的文件不含这些写法，只用它做测试会漏掉真实文件上的问题。

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, ErrorCode, RunOutput};
use serde_json::json;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="bin" ContentType="application/vnd.ms-office.unknown"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/><Override PartName="/xl/calcChain.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;

const WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x15"><workbookPr defaultThemeVersion="166925"/><bookViews><workbookView xWindow="0" yWindow="0" windowWidth="20490" windowHeight="7755"/></bookViews><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/><sheet name="Sheet2" sheetId="2" r:id="rId2"/></sheets><calcPr calcId="191029" iterateCount="42"/></workbook>"#;

const WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain" Target="calcChain.xml"/></Relationships>"#;

/// 第 3 行故意不写 `r`；A1、A2 带样式索引 `s="3"`；C1 走共享字符串；B1 是共享公式主格，B2 是从属格。
const SHEET1: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><dimension ref="A1:C3"/><cols><col min="1" max="1" width="18.5" customWidth="1"/></cols><sheetData><row r="1" spans="1:3" ht="20" customHeight="1"><c r="A1" s="3"><v>10</v></c><c r="B1"><f t="shared" ref="B1:B2" si="0">A1*2</f><v>20</v></c><c r="C1" t="s"><v>0</v></c></row><row r="2" spans="1:3"><c r="A2" s="3"><v>11</v></c><c r="B2"><f t="shared" si="0"/><v>22</v></c></row><row spans="1:3"><c r="A3"><v>12</v></c></row></sheetData><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/></worksheet>"#;

const SHEET2: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><!-- SHEET2-KEEP --><dimension ref="A1"/><sheetData><row r="1"><c r="A1" t="s"><v>1</v></c></row></sheetData></worksheet>"#;

const SHARED_STRINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2"><si><t>共享文本</t></si><si><t xml:space="preserve">  带空格  </t></si></sst>"#;

const CALC_CHAIN: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><calcChain xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><c r="B1" i="1"/><c r="B2" i="1"/></calcChain>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="0.00"/></numFmts><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0"/></cellStyleXfs><cellXfs count="4"><xf numFmtId="0" xfId="0"/><xf numFmtId="0" xfId="0"/><xf numFmtId="0" xfId="0"/><xf numFmtId="164" xfId="0" applyNumberFormat="1"/></cellXfs></styleSheet>"#;

const UNKNOWN_PART: &str = "UNKNOWN-PART-BYTES";

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-fidelity-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 每个用例一份独立样例，避免用例之间互相踩文件。
fn write_sample(tag: &str) -> PathBuf {
    write_with_sheet1(tag, SHEET1)
}

fn write_with_sheet1(tag: &str, sheet1: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = tmp_dir().join(format!("{tag}-{n}.xlsx"));
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in [
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", ROOT_RELS),
            ("xl/workbook.xml", WORKBOOK),
            ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS),
            ("xl/worksheets/sheet1.xml", sheet1),
            ("xl/worksheets/sheet2.xml", SHEET2),
            ("xl/sharedStrings.xml", SHARED_STRINGS),
            ("xl/calcChain.xml", CALC_CHAIN),
            ("xl/styles.xml", STYLES),
            ("customXml/unknown.bin", UNKNOWN_PART),
        ] {
            zip.start_file(name, opt).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    std::fs::write(&path, cursor.into_inner()).unwrap();
    path
}

fn part(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn apply(src: &Path, out: &Path, ops: serde_json::Value) -> Result<RunOutput, ooxp::Error> {
    let req = json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": out.to_string_lossy(),
        "ops": ops,
    });
    run(&req.to_string(), None)
}

fn sheet_data_of(text: &str) -> &str {
    let start = text.find("<sheetData>").unwrap();
    let end = text.find("</sheetData>").unwrap() + "</sheetData>".len();
    &text[start..end]
}

#[test]
fn rewritten_parts_keep_xml_declaration() {
    let src = write_sample("decl");
    let out = tmp_dir().join("decl-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 }]),
    )
    .unwrap();

    for name in ["xl/worksheets/sheet1.xml", "xl/workbook.xml"] {
        let text = part(&out, name);
        assert!(
            text.starts_with("<?xml "),
            "{name} 丢了 XML 声明：{}",
            &text[..text.len().min(40)]
        );
    }
}

#[test]
fn row_without_r_is_not_duplicated() {
    let src = write_sample("implicit-row");
    let out = tmp_dir().join("implicit-row-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A3", "value": 999 }]),
    )
    .unwrap();

    let data = part(&out, "xl/worksheets/sheet1.xml");
    let data = sheet_data_of(&data);
    assert_eq!(
        data.matches("<row").count(),
        3,
        "隐式行号的行被当成新行插了一遍：{data}"
    );
    assert_eq!(
        data.matches(r#"<c r="A3""#).count(),
        1,
        "A3 出现了两次：{data}"
    );
    assert!(data.contains("<v>999</v>"), "A3 没写进去：{data}");
    assert!(!data.contains("<v>12</v>"), "A3 的旧值还在：{data}");
}

#[test]
fn clearing_cell_in_row_without_r_takes_effect() {
    let src = write_sample("implicit-clear");
    let out = tmp_dir().join("implicit-clear-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A3", "value": null }]),
    )
    .unwrap();

    let data = part(&out, "xl/worksheets/sheet1.xml");
    let data = sheet_data_of(&data);
    assert!(!data.contains("<v>12</v>"), "清空没生效：{data}");
}

#[test]
fn shared_formula_master_keeps_group() {
    let src = write_sample("shared-master");
    let out = tmp_dir().join("shared-master-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=A1*3" }]),
    )
    .unwrap();

    let data = part(&out, "xl/worksheets/sheet1.xml");
    let data = sheet_data_of(&data);
    assert!(
        data.contains(r#"<f t="shared" ref="B1:B2" si="0">A1*3</f>"#),
        "主格公式没改或拆组了：{data}"
    );
    assert!(
        data.contains(r#"<f t="shared" si="0"/>"#),
        "从属格没保住：{data}"
    );
}

#[test]
fn data_table_formula_is_rejected() {
    let src = write_with_sheet1(
        "dt",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c><c r="B1"><f t="dataTable" ref="B1:B2" dt2D="0" dtr="0" r1="A1"/></c></row><row r="2"><c r="B2"><v>2</v></c></row></sheetData></worksheet>"#,
    );
    let out = tmp_dir().join("dt-out.xlsx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=A1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotImplemented);
    assert!(!out.exists() || std::fs::read(&out).unwrap() == std::fs::read(&src).unwrap());
}

#[test]
fn shared_formula_dependent_can_be_overwritten() {
    let src = write_sample("shared-dep");
    let out = tmp_dir().join("shared-dep-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!B2", "formula": "=A2*3" }]),
    )
    .unwrap();

    let data = part(&out, "xl/worksheets/sheet1.xml");
    let data = sheet_data_of(&data);
    assert!(data.contains("<f>A2*3</f>"), "从属格没改成独立公式：{data}");
    assert!(
        data.contains(r#"<f t="shared" ref="B1:B2" si="0">A1*2</f>"#),
        "主格被动了：{data}"
    );
}

#[test]
fn style_index_and_unknown_attributes_survive() {
    let src = write_sample("keep-style");
    let out = tmp_dir().join("keep-style-out.xlsx");
    apply(
        &src,
        &out,
        json!([
            { "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 },
            { "op": "setCell", "id": "cell:Sheet1!A2", "value": null }
        ]),
    )
    .unwrap();

    let text = part(&out, "xl/worksheets/sheet1.xml");
    let data = sheet_data_of(&text);
    assert!(data.contains(r#"<c r="A1" s="3"><v>42</v></c>"#), "{data}");
    assert!(
        data.contains(r#"<c r="A2" s="3"/>"#),
        "清空后样式索引没保住：{data}"
    );
    assert!(text.contains(r#"<col min="1" max="1" width="18.5" customWidth="1"/>"#));
    assert!(text.contains(r#"spans="1:3" ht="20" customHeight="1""#));
    assert!(text.contains("<pageMargins"));
}

/// 契约「改动范围」：被改工作表整份重序列化，注释丢掉。
#[test]
fn dirty_sheet_drops_comments() {
    let sheet = SHEET1.replacen("<dimension", "<!-- DIRTY-DROP --><dimension", 1);
    let src = write_with_sheet1("dirty-comment", &sheet);
    assert!(part(&src, "xl/worksheets/sheet1.xml").contains("<!-- DIRTY-DROP -->"));
    let out = tmp_dir().join("dirty-comment-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 }]),
    )
    .unwrap();
    let text = part(&out, "xl/worksheets/sheet1.xml");
    assert!(!text.contains("<!--"), "脏表注释应丢掉：{text}");
    assert!(
        sheet_data_of(&text).contains(r#"<c r="A1" s="3"><v>42</v></c>"#),
        "{text}"
    );
}

#[test]
fn untouched_parts_stay_byte_identical() {
    let src = write_sample("untouched");
    let out = tmp_dir().join("untouched-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!C1", "value": "新文本" }]),
    )
    .unwrap();

    for name in [
        "xl/worksheets/sheet2.xml",
        "xl/styles.xml",
        "xl/sharedStrings.xml",
        "customXml/unknown.bin",
    ] {
        assert_eq!(part(&src, name), part(&out, name), "{name} 被动过");
    }
    // 改文本走 inlineStr，不去改共享字符串表里的旧项。
    let data = part(&out, "xl/worksheets/sheet1.xml");
    assert!(
        sheet_data_of(&data).contains(r#"<c r="C1" t="inlineStr"><is><t>新文本</t></is></c>"#),
        "{data}"
    );
}

#[test]
fn calc_chain_and_its_traces_are_removed() {
    let src = write_sample("calcchain");
    let out = tmp_dir().join("calcchain-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A1", "value": 1 }]),
    )
    .unwrap();

    assert!(part_names(&src).iter().any(|n| n == "xl/calcChain.xml"));
    assert!(!part_names(&out).iter().any(|n| n == "xl/calcChain.xml"));
    assert!(!part(&out, "[Content_Types].xml").contains("calcChain"));
    assert!(!part(&out, "xl/_rels/workbook.xml.rels").contains("calcChain"));
    let wb = part(&out, "xl/workbook.xml");
    assert!(wb.contains(r#"fullCalcOnLoad="1""#));
    assert!(
        wb.contains(r#"calcId="191029""#),
        "calcPr 上原有属性丢了：{wb}"
    );
    assert!(
        wb.contains(r#"iterateCount="42""#),
        "calcPr 上原有属性丢了：{wb}"
    );
}

#[test]
fn zip_entry_order_is_preserved() {
    let src = write_sample("order");
    let out = tmp_dir().join("order-out.xlsx");
    apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!A1", "value": 1 }]),
    )
    .unwrap();

    let before: Vec<String> = part_names(&src)
        .into_iter()
        .filter(|n| n != "xl/calcChain.xml")
        .collect();
    assert_eq!(before, part_names(&out));
}

#[test]
fn inspect_reads_shared_string_and_shared_formula() {
    let src = write_sample("inspect");
    let req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1:C3" },
    });
    let RunOutput::Inspect(tree) = run(&req.to_string(), None).unwrap() else {
        panic!("inspect 应返回树");
    };
    let cells = tree["sheets"][0]["cells"].as_array().unwrap();
    let find = |id: &str| {
        cells
            .iter()
            .find(|c| c["id"] == id)
            .unwrap_or_else(|| panic!("树里没有 {id}"))
    };
    assert_eq!(find("cell:Sheet1!C1")["value"], "共享文本");
    assert_eq!(find("cell:Sheet1!B1")["formula"], "=A1*2");
    assert_eq!(find("cell:Sheet1!B1")["shared"], true);
    assert_eq!(find("cell:Sheet1!B1")["sharedRef"], "B1:B2");
    assert_eq!(find("cell:Sheet1!B2")["shared"], true);
    assert_eq!(find("cell:Sheet1!B2")["formula"], "=A2*2");
    assert_eq!(find("cell:Sheet1!A3")["value"], 12);
}

#[test]
fn inspect_bad_shared_string_index_is_xml_broken() {
    let src = write_with_sheet1(
        "bad-sst",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>99</v></c></row></sheetData></worksheet>"#,
    );
    let req = json!({
        "schemaVersion": 1,
        "action": "inspect",
        "path": src.to_string_lossy(),
        "scope": { "sheet": "Sheet1", "range": "A1" },
    });
    let err = run(&req.to_string(), None).unwrap_err();
    assert_eq!(err.code(), ErrorCode::XmlBroken);
}

#[test]
fn apply_clear_missing_cell_counts_as_applied() {
    let src = write_sample("clear-missing");
    let out = tmp_dir().join("clear-missing-out.xlsx");
    let RunOutput::Written { ops_applied, .. } = apply(
        &src,
        &out,
        json!([{ "op": "setCell", "id": "cell:Sheet1!Z99", "value": null }]),
    )
    .unwrap() else {
        panic!("apply 应写出文件");
    };
    assert_eq!(ops_applied, Some(1));
}

#[test]
fn insert_table_is_not_for_xlsx() {
    let src = write_sample("tbl-foreign");
    let out = tmp_dir().join("tbl-foreign-out.xlsx");
    let err = apply(
        &src,
        &out,
        json!([{
            "op": "insertTable",
            "id": "docx:document",
            "rows": 1,
            "cols": 1
        }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn scope_tables_is_not_for_xlsx() {
    let src = write_sample("tbl-scope");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "tables": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn accept_revision_is_not_for_xlsx() {
    let src = write_sample("rev-foreign");
    let out = tmp_dir().join("rev-foreign-out.xlsx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "acceptRevision", "id": "rev:1" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn scope_revisions_is_not_for_xlsx() {
    let src = write_sample("rev-scope");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "revisions": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn xlsx_accepts_hyperlinks_scope() {
    let src = write_sample("hl-scope");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "hyperlinks": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree["hyperlinkCount"], 0);
    assert_eq!(tree["sheets"][0]["hyperlinkCount"], 0);
    assert_eq!(tree["sheets"][0]["hyperlinks"], json!([]));
}

#[test]
fn xlsx_accepts_sparklines_scope() {
    let src = write_sample("sk-scope");
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "sparklines": true }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    assert_eq!(tree["sparklineCount"], 0);
    assert_eq!(tree["sheets"][0]["sparklineCount"], 0);
    assert_eq!(tree["oleObjectCount"], 0);
    assert_eq!(tree["sheets"][0]["oleObjectCount"], 0);
    assert_eq!(tree["slicerCount"], 0);
    assert_eq!(tree["sheets"][0]["slicerCount"], 0);
    assert_eq!(tree["connectionCount"], 0);
    assert_eq!(tree["customViewCount"], 0);
    assert_eq!(tree["sheets"][0]["sparklines"], json!([]));
}

#[test]
fn insert_hyperlink_is_not_for_xlsx() {
    let src = write_sample("hl-foreign");
    let out = tmp_dir().join("hl-foreign-out.xlsx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertHyperlink", "id": "para:1", "text": "x", "url": "https://example.com" }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn scope_fields_is_not_for_xlsx() {
    let src = write_sample("fld-scope");
    let err = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": src.to_string_lossy(),
            "scope": { "fields": true }
        })
        .to_string(),
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}

#[test]
fn insert_field_is_not_for_xlsx() {
    let src = write_sample("fld-foreign");
    let out = tmp_dir().join("fld-foreign-out.xlsx");
    let err = apply(
        &src,
        &out,
        json!([{ "op": "insertField", "id": "para:1", "instr": " PAGE " }]),
    )
    .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
