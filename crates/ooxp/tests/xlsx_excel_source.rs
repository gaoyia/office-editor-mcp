//! 对着入库的 Excel 另存夹具跑清单 A2–A7 与 O2。
//!
//! 文件是 `tests/fixtures/excel-source.xlsx`。不要用库自己的 `create` 产物冒充。
//! A1（Office 打开无 Repair）仍靠 `manual-check/office/cases/xlsx-apply.ps1`。
//! O1 已放弃，见 `docs/opc.md`。

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use ooxp::{run, RunOutput};
use serde_json::{json, Value};
use zip::ZipArchive;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/excel-source.xlsx")
}

fn apply_ops() -> Value {
    json!([
        { "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 },
        { "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=SUM(A1:A10)" },
        { "op": "setCell", "id": "cell:Sheet1!C1", "value": null }
    ])
}

/// `tag` 按用例取名。测试是并行跑的，共用一个输出路径会撞上 `write_atomic`
/// 的中间文件：那两个名字由目标路径派生，两个用例同时写就会互相抢改名。
fn apply_out(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-excel-source-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join(format!("excel-applied-{tag}.xlsx"));
    run(
        &json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": fixture().to_string_lossy(),
            "output": out.to_string_lossy(),
            "ops": apply_ops()
        })
        .to_string(),
        None,
    )
    .unwrap();
    out
}

fn inspect(path: &Path, sheet: &str, range: &str) -> Value {
    let RunOutput::Inspect(tree) = run(
        &json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": sheet, "range": range }
        })
        .to_string(),
        None,
    )
    .unwrap() else {
        panic!("inspect 应返回树");
    };
    tree
}

fn cell<'a>(tree: &'a Value, id: &str) -> &'a Value {
    tree["sheets"][0]["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("树里没有 {id}"))
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

fn part_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

/// 读 `<c r="C1" s="2" …>` 上的样式索引。另存之后索引号会变，不能写死。
fn cell_style_index<'a>(sheet: &'a str, addr: &str) -> Option<&'a str> {
    let open = format!(r#"<c r="{addr}""#);
    let start = sheet.find(&open)?;
    let tag = sheet.get(start..)?.split('>').next()?;
    let after = tag.split(r#" s=""#).nth(1)?;
    after.split('"').next()
}

#[test]
fn fixture_is_excel_saved_workbook() {
    let src = fixture();
    assert!(
        src.is_file(),
        "缺少 {src:?}，请跑 manual-check/office/export-excel-fixture.ps1"
    );
    let names = part_names(&src);
    assert!(
        names.iter().any(|n| n == "xl/theme/theme1.xml"),
        "{names:?}"
    );
    let wb = part(&src, "xl/workbook.xml");
    assert!(wb.contains(r#"appName="xl""#), "{wb}");
    assert!(wb.contains(r#"name="Sheet1""#));
    assert!(wb.contains(r#"name="Sheet2""#));
}

#[test]
fn inspect_reads_excel_source() {
    let tree = inspect(&fixture(), "Sheet1", "A1:E3");
    assert_eq!(cell(&tree, "cell:Sheet1!A1")["value"], 1);
    assert_eq!(cell(&tree, "cell:Sheet1!A1")["type"], "number");
    assert_eq!(cell(&tree, "cell:Sheet1!B1")["formula"], "=SUM(A1)");
    assert_eq!(cell(&tree, "cell:Sheet1!C1")["value"], 123.45);
    assert_eq!(cell(&tree, "cell:Sheet1!D1")["formula"], "=A1*2");
    assert_eq!(cell(&tree, "cell:Sheet1!E1")["value"], "keep");
    assert_eq!(cell(&tree, "cell:Sheet1!E1")["type"], "text");
}

/// 清单 A2 / A3 / A4。
#[test]
fn apply_hits_a1_b1_c1() {
    let out = apply_out("a1b1c1");
    let tree = inspect(&out, "Sheet1", "A1:E3");
    assert_eq!(cell(&tree, "cell:Sheet1!A1")["value"], 42);
    assert_eq!(cell(&tree, "cell:Sheet1!B1")["formula"], "=SUM(A1:A10)");
    let c1 = cell(&tree, "cell:Sheet1!C1");
    assert_eq!(c1["type"], "empty");
    assert!(c1["value"].is_null());
    let src_sheet = part(&fixture(), "xl/worksheets/sheet1.xml");
    let style = cell_style_index(&src_sheet, "C1").expect("源文件 C1 应带数字格式");
    let sheet = part(&out, "xl/worksheets/sheet1.xml");
    let kept = format!(r#"<c r="C1" s="{style}"/>"#);
    let kept_pair = format!(r#"<c r="C1" s="{style}"></c>"#);
    assert!(
        sheet.contains(&kept) || sheet.contains(&kept_pair),
        "清空后数字格式索引应留下（源是 s={style}）：{sheet}"
    );
}

/// 清单 A5 / A6 / O2：未改表、样式部件、主题字节不变。
#[test]
fn apply_leaves_untouched_excel_parts() {
    let src = fixture();
    let out = apply_out("untouched");
    for name in [
        "xl/worksheets/sheet2.xml",
        "xl/styles.xml",
        "xl/theme/theme1.xml",
        "xl/sharedStrings.xml",
    ] {
        assert_eq!(
            part_bytes(&src, name),
            part_bytes(&out, name),
            "{name} 被动过"
        );
    }
    let sheet = part(&out, "xl/worksheets/sheet1.xml");
    assert!(
        sheet.contains(r#"<c r="E1" s="3" t="s"><v>0</v></c>"#),
        "{sheet}"
    );
    assert!(sheet.contains(r#"width="20.625""#), "E 列宽被改了：{sheet}");
}

/// 清单 A7：库不改 D1，只标打开时重算并删掉 calcChain。
#[test]
fn apply_marks_recalc_for_dependent_formula() {
    let out = apply_out("recalc");
    let sheet = part(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<f>A1*2</f>"), "D1 公式丢了：{sheet}");
    let names = part_names(&out);
    assert!(!names.iter().any(|n| n == "xl/calcChain.xml"), "{names:?}");
    let wb = part(&out, "xl/workbook.xml");
    assert!(wb.contains(r#"fullCalcOnLoad="1""#), "{wb}");
}
