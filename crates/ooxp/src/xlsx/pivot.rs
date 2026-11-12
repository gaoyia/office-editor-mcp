//! 工作表透视表：inspect 名单，apply 写缓存与透视表部件。
//! 只有行字段和求和时，求值把合计写进 `sheetData`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1::{self, CellRef};
use super::picture;
use super::request::{DeletePivotTableOp, InsertPivotTableOp, SetPivotTableOp};
use super::{REL_NS, SML_NS};

pub const REL_PIVOT_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable";
pub const REL_PIVOT_CACHE_DEF: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition";
pub const REL_PIVOT_CACHE_REC: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheRecords";
pub const CT_PIVOT_TABLE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotTable+xml";
pub const CT_PIVOT_CACHE_DEF: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheDefinition+xml";
pub const CT_PIVOT_CACHE_REC: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheRecords+xml";

/// `CT_Workbook` 子元素顺序（§A.2）。未知元素插在 `extLst` 之前。
const WORKBOOK_CHILD_ORDER: &[&str] = &[
    "fileVersion",
    "fileSharing",
    "workbookPr",
    "workbookProtection",
    "bookViews",
    "sheets",
    "functionGroups",
    "externalReferences",
    "definedNames",
    "calcPr",
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

#[derive(Clone)]
struct Axes {
    rows: Vec<String>,
    cols: Vec<String>,
    filters: Vec<String>,
    values: Vec<String>,
}

#[derive(Clone)]
enum CacheVal {
    Empty,
    Number(String),
    Bool(bool),
    Text(String),
}

pub(crate) fn count(pkg: &Package, sheet_part: &str) -> Result<usize, Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    Ok(rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/pivotTable"))
        .count())
}

pub(crate) fn inspect(
    pkg: &Package,
    wb_part: &str,
    sheet_part: &str,
    sheet: &str,
) -> Result<Value, Error> {
    let caches = load_caches(pkg, wb_part)?;
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    let mut n = 0usize;
    for rel in &rels.rels {
        if !rel.rel_type.ends_with("/pivotTable") {
            continue;
        }
        n += 1;
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        let bytes = pkg.get(&part)?;
        let doc = xml::parse(bytes)?;
        out.push(inspect_one(&doc.root, sheet, n, &caches));
    }
    Ok(Value::Array(out))
}

fn inspect_one(root: &Element, sheet: &str, n: usize, caches: &[(u32, CacheInfo)]) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/pivot:{n}")));
    obj.insert("name".into(), json!(root.attr("name").unwrap_or("")));
    let cache_id = root.attr("cacheId").and_then(|s| s.parse::<u32>().ok());
    obj.insert(
        "cacheId".into(),
        match cache_id {
            Some(id) => json!(id),
            None => Value::Null,
        },
    );
    let loc_raw = root
        .child_by_local("location")
        .and_then(|l| l.attr("ref"))
        .unwrap_or("");
    let location = match a1::parse_range(loc_raw) {
        Ok((s, e)) => a1::format_range(s, e),
        Err(_) => loc_raw.to_string(),
    };
    obj.insert("location".into(), json!(location));
    let info = cache_id.and_then(|id| caches.iter().find(|(c, _)| *c == id).map(|(_, i)| i));
    if let Some(src) = info.and_then(|i| i.source.as_ref()) {
        let mut src_obj = Map::new();
        src_obj.insert("sheet".into(), json!(src.0));
        src_obj.insert("ref".into(), json!(src.1));
        obj.insert("source".into(), Value::Object(src_obj));
    } else {
        obj.insert("source".into(), Value::Null);
    }
    let fields = info.map(|i| i.fields.as_slice()).unwrap_or(&[]);
    obj.insert(
        "rows".into(),
        json!(axis_names(root, "rowFields", "field", "x", fields, false)),
    );
    obj.insert(
        "cols".into(),
        json!(axis_names(root, "colFields", "field", "x", fields, false)),
    );
    obj.insert(
        "filters".into(),
        json!(axis_names(
            root,
            "pageFields",
            "pageField",
            "fld",
            fields,
            false
        )),
    );
    obj.insert(
        "values".into(),
        json!(axis_names(
            root,
            "dataFields",
            "dataField",
            "fld",
            fields,
            true
        )),
    );
    Value::Object(obj)
}

struct CacheInfo {
    source: Option<(String, String)>,
    fields: Vec<String>,
}

fn load_caches(pkg: &Package, wb_part: &str) -> Result<Vec<(u32, CacheInfo)>, Error> {
    let bytes = pkg.get(wb_part)?;
    let wb = xml::parse(bytes)?;
    let rels = Relationships::from_package(pkg, wb_part)?;
    let Some(parent) = wb.root.child_by_local("pivotCaches") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for el in parent.children_by_local("pivotCache") {
        let Some(id) = el.attr("cacheId").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Some(rid) = el.attr_local("id") else {
            continue;
        };
        let Some(rel) = rels.find_by_id(rid) else {
            continue;
        };
        let Ok(part) = resolve_rel_target(wb_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        out.push((id, parse_cache_info(&doc.root)));
    }
    Ok(out)
}

fn parse_cache_info(root: &Element) -> CacheInfo {
    let source = root.child_by_local("cacheSource").and_then(|cs| {
        let ty = cs.attr("type").unwrap_or("");
        if ty != "worksheet" {
            return None;
        }
        let ws = cs.child_by_local("worksheetSource")?;
        let sheet = ws.attr("sheet")?.to_string();
        let refer = ws.attr("ref")?.to_string();
        let refer = match a1::parse_range(&refer) {
            Ok((s, e)) => a1::format_range(s, e),
            Err(_) => refer,
        };
        Some((sheet, refer))
    });
    let fields = root
        .child_by_local("cacheFields")
        .map(|cf| {
            cf.children_by_local("cacheField")
                .into_iter()
                .map(|f| f.attr("name").unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();
    CacheInfo { source, fields }
}

fn axis_names(
    root: &Element,
    parent: &str,
    child: &str,
    idx_attr: &str,
    fields: &[String],
    prefer_name: bool,
) -> Vec<String> {
    let Some(el) = root.child_by_local(parent) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in el.children_by_local(child) {
        if prefer_name {
            if let Some(n) = item.attr("name") {
                if !n.is_empty() {
                    out.push(n.to_string());
                    continue;
                }
            }
        }
        let Some(raw) = item.attr(idx_attr) else {
            out.push(String::new());
            continue;
        };
        if raw == "-2" {
            continue;
        }
        let Ok(idx) = raw.parse::<usize>() else {
            out.push(String::new());
            continue;
        };
        out.push(fields.get(idx).cloned().unwrap_or_default());
    }
    out
}

pub(crate) fn apply_insert(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut [(String, Document)],
    sheet_part: &str,
    op: &InsertPivotTableOp,
    op_index: usize,
) -> Result<(), Error> {
    let src_sheet = op.source.sheet.as_deref().unwrap_or(&op.sheet);
    let src_part = require_source_sheet(sheet_index, src_sheet, op_index)?;
    let (src_start, src_end) = op.source.range;
    check_source_span(src_start, src_end, op_index)?;
    let sst = load_sst(pkg, wb_part)?;
    let (headers, records) =
        read_source_owned(pkg, dirty_sheets, &src_part, src_start, src_end, &sst)?;
    let axes = Axes {
        rows: op.rows.clone(),
        cols: op.cols.clone(),
        filters: op.filters.clone(),
        values: op.values.clone(),
    };
    check_axes(&axes, &headers, op_index)?;
    let counts = unique_counts_from_records(headers.len(), &records);
    let loc_end = location_end(op.location, &axes, &headers, &counts);
    if src_part == sheet_part && ranges_overlap(src_start, src_end, op.location, loc_end) {
        return Err(bad_field("location", op_index));
    }
    let taken = collect_pivot_names(pkg, sheet_index)?;
    let name = match &op.name {
        Some(n) => {
            validate_name(n, op_index)?;
            if name_taken(&taken, n, None) {
                return Err(bad_field("name", op_index));
            }
            n.clone()
        }
        None => next_pivot_name(&taken),
    };
    let cache_id = next_cache_id(pkg, wb_doc, sheet_index);
    let def_part = next_part(pkg, "/xl/pivotCache/pivotCacheDefinition", "xml");
    let rec_part = next_part(pkg, "/xl/pivotCache/pivotCacheRecords", "xml");
    let table_part = next_part(pkg, "/xl/pivotTables/pivotTable", "xml");
    write_cache_parts(
        pkg, &def_part, &rec_part, src_sheet, src_start, src_end, &headers, &records,
    )?;
    let table_xml = build_pivot_table_xml(cache_id, &name, op.location, &axes, &headers, &counts);
    pkg.set(&table_part, table_xml.to_bytes()?)?;
    add_sheet_pivot_rel(pkg, sheet_part, &table_part)?;
    set_table_cache_rel(pkg, &table_part, &def_part)?;
    add_workbook_cache(pkg, wb_doc, wb_part, cache_id, &def_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&def_part, CT_PIVOT_CACHE_DEF)?;
    cts.set_override(&rec_part, CT_PIVOT_CACHE_REC)?;
    cts.set_override(&table_part, CT_PIVOT_TABLE)?;
    cts.write_to(pkg)?;
    Ok(())
}

pub(crate) fn apply_set(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut [(String, Document)],
    sheet_part: &str,
    op: &SetPivotTableOp,
    op_index: usize,
) -> Result<bool, Error> {
    let (table_part, _rid) = pivot_part_at(pkg, sheet_part, &op.sheet, op.index, op_index)?;
    let mut table_doc = xml::parse(pkg.get(&table_part)?)?;
    let cache_id = table_doc
        .root
        .attr("cacheId")
        .and_then(|s| s.parse::<u32>().ok());
    let Some(cache_id) = cache_id else {
        return Err(bad_field("source", op_index));
    };
    let (mut def_part, rec_part) = cache_parts_of(pkg, wb_doc, wb_part, cache_id, op_index)?;
    let def_doc = xml::parse(pkg.get(&def_part)?)?;
    let info = parse_cache_info(&def_doc.root);
    if info.source.is_none() && (op.source.is_some() || axes_patched(op)) {
        return Err(bad_field("source", op_index));
    }
    let mut wb_dirty = false;
    let headers_now = info.fields.clone();
    let current = current_axes(&table_doc.root, &headers_now);
    let axes = Axes {
        rows: op.rows.clone().unwrap_or(current.rows),
        cols: op.cols.clone().unwrap_or(current.cols),
        filters: op.filters.clone().unwrap_or(current.filters),
        values: op.values.clone().unwrap_or(current.values),
    };
    if axes.values.is_empty() {
        return Err(bad_field("values", op_index));
    }
    check_axis_unique(&axes, op_index)?;
    let mut origin = table_doc
        .root
        .child_by_local("location")
        .and_then(|l| l.attr("ref"))
        .and_then(|r| a1::parse_range(r).ok())
        .map(|(s, _)| s)
        .unwrap_or(CellRef { col: 1, row: 1 });
    if let Some(loc) = op.location {
        origin = loc;
    }
    if let Some(src) = &op.source {
        let src_sheet = src.sheet.as_deref().unwrap_or(&op.sheet);
        let src_part = require_source_sheet(sheet_index, src_sheet, op_index)?;
        check_source_span(src.range.0, src.range.1, op_index)?;
        let sst = load_sst(pkg, wb_part)?;
        let (headers, records) =
            read_source_owned(pkg, dirty_sheets, &src_part, src.range.0, src.range.1, &sst)?;
        check_axes(&axes, &headers, op_index)?;
        let loc_end = location_end(
            origin,
            &axes,
            &headers,
            &unique_counts_from_records(headers.len(), &records),
        );
        if src_part == sheet_part && ranges_overlap(src.range.0, src.range.1, origin, loc_end) {
            return Err(bad_field("location", op_index));
        }
        let users = count_cache_users(pkg, sheet_index, cache_id);
        let (write_def, write_rec) = if users > 1 {
            let new_id = next_cache_id(pkg, wb_doc, sheet_index);
            let new_def = next_part(pkg, "/xl/pivotCache/pivotCacheDefinition", "xml");
            let new_rec = next_part(pkg, "/xl/pivotCache/pivotCacheRecords", "xml");
            write_cache_parts(
                pkg,
                &new_def,
                &new_rec,
                src_sheet,
                src.range.0,
                src.range.1,
                &headers,
                &records,
            )?;
            add_workbook_cache(pkg, wb_doc, wb_part, new_id, &new_def)?;
            table_doc.root.set_attr("cacheId", new_id.to_string());
            def_part = new_def;
            wb_dirty = true;
            let mut cts = ContentTypes::from_package(pkg)?;
            cts.set_override(&def_part, CT_PIVOT_CACHE_DEF)?;
            cts.set_override(&new_rec, CT_PIVOT_CACHE_REC)?;
            cts.write_to(pkg)?;
            (def_part.clone(), new_rec)
        } else {
            (def_part.clone(), rec_part)
        };
        write_cache_parts(
            pkg,
            &write_def,
            &write_rec,
            src_sheet,
            src.range.0,
            src.range.1,
            &headers,
            &records,
        )?;
        replace_layout(
            &mut table_doc.root,
            origin,
            &axes,
            &headers,
            &unique_counts_from_records(headers.len(), &records),
        );
    } else if axes_patched(op) {
        check_axes(&axes, &headers_now, op_index)?;
        replace_layout(
            &mut table_doc.root,
            origin,
            &axes,
            &headers_now,
            &unique_counts_from_cache(&def_doc.root),
        );
    } else if op.location.is_some() {
        replace_location(
            &mut table_doc.root,
            origin,
            &axes,
            &headers_now,
            &unique_counts_from_cache(&def_doc.root),
        );
    }
    if let Some(n) = &op.name {
        validate_name(n, op_index)?;
        let taken = collect_pivot_names(pkg, sheet_index)?;
        let current_name = table_doc.root.attr("name").unwrap_or("");
        if name_taken(&taken, n, Some(current_name)) {
            return Err(bad_field("name", op_index));
        }
        table_doc.root.set_attr("name", n.clone());
    }
    set_table_cache_rel(pkg, &table_part, &def_part)?;
    pkg.set(&table_part, table_doc.to_bytes()?)?;
    Ok(wb_dirty)
}

pub(crate) fn apply_delete(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    sheet_part: &str,
    op: &DeletePivotTableOp,
    op_index: usize,
) -> Result<(), Error> {
    let (table_part, rid) = pivot_part_at(pkg, sheet_part, &op.sheet, op.index, op_index)?;
    let table_doc = xml::parse(pkg.get(&table_part)?)?;
    let cache_id = table_doc
        .root
        .attr("cacheId")
        .and_then(|s| s.parse::<u32>().ok());
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    rels.remove_by_id(&rid);
    rels.write_to(pkg, sheet_part)?;
    pkg.remove(&table_part);
    pkg.remove(&rels_part_name(&table_part));
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&table_part);
    if let Some(cid) = cache_id {
        let left = count_cache_users(pkg, sheet_index, cid);
        // 当前表的关系已去掉，剩下的就是别人还在用的。
        if left == 0 {
            if let Ok((def_part, rec_part)) = cache_parts_of(pkg, wb_doc, wb_part, cid, op_index) {
                remove_workbook_cache(pkg, wb_doc, wb_part, cid)?;
                pkg.remove(&def_part);
                pkg.remove(&rels_part_name(&def_part));
                pkg.remove(&rec_part);
                cts.remove_override(&def_part);
                cts.remove_override(&rec_part);
            }
        }
    }
    cts.write_to(pkg)?;
    Ok(())
}

fn axes_patched(op: &SetPivotTableOp) -> bool {
    op.rows.is_some() || op.cols.is_some() || op.filters.is_some() || op.values.is_some()
}

fn current_axes(root: &Element, fields: &[String]) -> Axes {
    Axes {
        rows: axis_names(root, "rowFields", "field", "x", fields, false),
        cols: axis_names(root, "colFields", "field", "x", fields, false),
        filters: axis_names(root, "pageFields", "pageField", "fld", fields, false),
        values: axis_names(root, "dataFields", "dataField", "fld", fields, true),
    }
}

fn require_source_sheet(
    sheet_index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    if let Some((_, part, kind)) = sheet_index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
    {
        if *kind != "worksheet" {
            return Err(Error::with_path(
                ErrorKind::LayoutWorksheetOnly,
                format!("ops[{op_index}].source.sheet"),
            ));
        }
        return Ok(part.clone());
    }
    Err(Error::with_path(
        ErrorKind::SheetNotFound {
            name: name.to_string(),
        },
        format!("ops[{op_index}].source.sheet"),
    ))
}

fn check_source_span(start: CellRef, end: CellRef, op_index: usize) -> Result<(), Error> {
    if start.row == end.row {
        return Err(bad_field("source", op_index));
    }
    Ok(())
}

fn check_axes(axes: &Axes, headers: &[String], op_index: usize) -> Result<(), Error> {
    check_axis_unique(axes, op_index)?;
    for (field, names) in [
        ("rows", axes.rows.as_slice()),
        ("cols", axes.cols.as_slice()),
        ("filters", axes.filters.as_slice()),
        ("values", axes.values.as_slice()),
    ] {
        for n in names {
            if header_index(headers, n).is_none() {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: field.into(),
                    },
                    format!("ops[{op_index}].{field}"),
                ));
            }
        }
    }
    Ok(())
}

fn check_axis_unique(axes: &Axes, op_index: usize) -> Result<(), Error> {
    let mut seen: Vec<String> = Vec::new();
    for (field, names) in [
        ("rows", axes.rows.as_slice()),
        ("cols", axes.cols.as_slice()),
        ("filters", axes.filters.as_slice()),
        ("values", axes.values.as_slice()),
    ] {
        for n in names {
            let k = n.to_ascii_lowercase();
            if seen.iter().any(|s| s == &k) {
                return Err(Error::with_path(
                    ErrorKind::InvalidPivotTable {
                        field: field.into(),
                    },
                    format!("ops[{op_index}].{field}"),
                ));
            }
            seen.push(k);
        }
    }
    Ok(())
}

fn header_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h.eq_ignore_ascii_case(name))
}

fn location_end(
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) -> CellRef {
    let page = if axes.filters.is_empty() {
        0
    } else {
        axes.filters.len() as u32 + 1
    };
    let row_n = axis_unique_product(&axes.rows, headers, unique_counts);
    let col_n = axis_unique_product(&axes.cols, headers, unique_counts)
        .max(axes.values.len().max(1) as u32);
    let width = if axes.rows.is_empty() {
        col_n
    } else {
        1 + col_n
    };
    let height = page + 1 + row_n + 1;
    CellRef {
        col: origin.col + width - 1,
        row: origin.row + height - 1,
    }
}

fn axis_unique_product(names: &[String], headers: &[String], unique_counts: &[usize]) -> u32 {
    if names.is_empty() {
        return 1;
    }
    let mut n = 1usize;
    for name in names {
        let u = header_index(headers, name)
            .and_then(|i| unique_counts.get(i).copied())
            .unwrap_or(1)
            .max(1);
        n = n.saturating_mul(u).min(256);
    }
    n as u32
}

fn field_unique_count(headers: &[String], name: &str, unique_counts: &[usize]) -> usize {
    header_index(headers, name)
        .and_then(|i| unique_counts.get(i).copied())
        .unwrap_or(0)
}

fn ranges_overlap(a0: CellRef, a1: CellRef, b0: CellRef, b1: CellRef) -> bool {
    a0.col <= b1.col && b0.col <= a1.col && a0.row <= b1.row && b0.row <= a1.row
}

fn collect_pivot_names(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for (_, part, kind) in sheet_index {
        if *kind != "worksheet" {
            continue;
        }
        let rels = Relationships::from_package(pkg, part)?;
        for rel in &rels.rels {
            if !rel.rel_type.ends_with("/pivotTable") {
                continue;
            }
            let Ok(tp) = resolve_rel_target(part, &rel.target) else {
                continue;
            };
            let Ok(bytes) = pkg.get(&tp) else {
                continue;
            };
            let Ok(doc) = xml::parse(bytes) else {
                continue;
            };
            if let Some(n) = doc.root.attr("name") {
                out.push(n.to_string());
            }
        }
    }
    Ok(out)
}

fn name_taken(taken: &[String], name: &str, except: Option<&str>) -> bool {
    taken.iter().any(|n| {
        if except.is_some_and(|e| n.eq_ignore_ascii_case(e)) {
            return false;
        }
        n.eq_ignore_ascii_case(name)
    })
}

fn next_pivot_name(taken: &[String]) -> String {
    let mut n = 1u32;
    loop {
        let cand = format!("PivotTable{n}");
        if !name_taken(taken, &cand, None) {
            return cand;
        }
        n += 1;
    }
}

fn validate_name(name: &str, op_index: usize) -> Result<(), Error> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(bad_field("name", op_index));
    }
    Ok(())
}

fn next_cache_id(
    pkg: &Package,
    wb_doc: &Document,
    sheet_index: &[(String, String, &'static str)],
) -> u32 {
    let mut max = 0u32;
    if let Some(parent) = wb_doc.root.child_by_local("pivotCaches") {
        for el in parent.children_by_local("pivotCache") {
            if let Some(id) = el.attr("cacheId").and_then(|s| s.parse::<u32>().ok()) {
                max = max.max(id);
            }
        }
    }
    if let Ok(names) = collect_all_cache_ids(pkg, sheet_index) {
        for id in names {
            max = max.max(id);
        }
    }
    max + 1
}

fn collect_all_cache_ids(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
) -> Result<Vec<u32>, Error> {
    let mut out = Vec::new();
    for (_, part, kind) in sheet_index {
        if *kind != "worksheet" {
            continue;
        }
        let rels = Relationships::from_package(pkg, part)?;
        for rel in &rels.rels {
            if !rel.rel_type.ends_with("/pivotTable") {
                continue;
            }
            let Ok(tp) = resolve_rel_target(part, &rel.target) else {
                continue;
            };
            let Ok(bytes) = pkg.get(&tp) else {
                continue;
            };
            let Ok(doc) = xml::parse(bytes) else {
                continue;
            };
            if let Some(id) = doc.root.attr("cacheId").and_then(|s| s.parse().ok()) {
                out.push(id);
            }
        }
    }
    Ok(out)
}

fn count_cache_users(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
    cache_id: u32,
) -> usize {
    collect_all_cache_ids(pkg, sheet_index)
        .unwrap_or_default()
        .into_iter()
        .filter(|id| *id == cache_id)
        .count()
}

fn next_part(pkg: &Package, prefix: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{prefix}{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn write_cache_parts(
    pkg: &mut Package,
    def_part: &str,
    rec_part: &str,
    src_sheet: &str,
    start: CellRef,
    end: CellRef,
    headers: &[String],
    records: &[Vec<CacheVal>],
) -> Result<(), Error> {
    let def = build_cache_def(src_sheet, start, end, headers, records);
    let rec = build_cache_records(records);
    pkg.set(def_part, def.to_bytes()?)?;
    pkg.set(rec_part, rec.to_bytes()?)?;
    let mut rels = Relationships { rels: Vec::new() };
    rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_PIVOT_CACHE_REC.into(),
        target: picture::rel_target_from(def_part, rec_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, def_part)?;
    Ok(())
}

fn build_cache_def(
    sheet: &str,
    start: CellRef,
    end: CellRef,
    headers: &[String],
    records: &[Vec<CacheVal>],
) -> Document {
    let mut src = Element::new("cacheSource").with_attr("type", "worksheet");
    src.push_element(
        Element::new("worksheetSource")
            .with_attr("ref", a1::format_range(start, end))
            .with_attr("sheet", sheet),
    );
    let mut fields = Element::new("cacheFields").with_attr("count", headers.len().to_string());
    for (i, name) in headers.iter().enumerate() {
        let col: Vec<&CacheVal> = records.iter().filter_map(|r| r.get(i)).collect();
        let mut field = Element::new("cacheField")
            .with_attr("name", name.clone())
            .with_attr("numFmtId", "0");
        field.push_element(shared_items(&col));
        fields.push_element(field);
    }
    let mut root = Element::new("pivotCacheDefinition")
        .with_attr("xmlns", SML_NS)
        .with_attr("xmlns:r", REL_NS)
        .with_attr("r:id", "rId1")
        .with_attr("refreshOnLoad", "1")
        .with_attr("createdVersion", "4")
        .with_attr("refreshedVersion", "4")
        .with_attr("minRefreshableVersion", "3")
        .with_attr("recordCount", records.len().to_string());
    root.push_element(src);
    root.push_element(fields);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn shared_items(col: &[&CacheVal]) -> Element {
    let mut unique: Vec<CacheVal> = Vec::new();
    let mut contains_blank = false;
    for v in col {
        match v {
            CacheVal::Empty => contains_blank = true,
            other => {
                if !unique.iter().any(|u| cache_eq(u, other)) {
                    unique.push((*other).clone());
                }
            }
        }
    }
    let all_num = !unique.is_empty() && unique.iter().all(|v| matches!(v, CacheVal::Number(_)));
    let mut el = Element::new("sharedItems");
    if contains_blank {
        el.set_attr("containsBlank", "1");
    }
    if all_num {
        // 数字字段按 Excel 16 写出类型旗标。containsSemiMixedTypes="0" 只给数字。
        el.set_attr("containsSemiMixedTypes", "0");
        el.set_attr("containsString", "0");
        el.set_attr("containsNumber", "1");
        let nums: Vec<f64> = unique
            .iter()
            .filter_map(|v| match v {
                CacheVal::Number(n) => n.parse().ok(),
                _ => None,
            })
            .collect();
        if let (Some(min), Some(max)) = (
            nums.iter().copied().reduce(f64::min),
            nums.iter().copied().reduce(f64::max),
        ) {
            el.set_attr("minValue", min.to_string());
            el.set_attr("maxValue", max.to_string());
        }
        if unique.iter().all(|v| match v {
            CacheVal::Number(n) => n.parse::<i64>().is_ok(),
            _ => false,
        }) {
            el.set_attr("containsInteger", "1");
        }
    }
    // 字符串或混合字段不要写 containsSemiMixedTypes="0"：Excel 16 会让 Workbooks.Open 直接失败。
    el.set_attr("count", unique.len().to_string());
    for v in unique {
        el.push_element(cache_item(&v));
    }
    el
}

fn cache_eq(a: &CacheVal, b: &CacheVal) -> bool {
    match (a, b) {
        (CacheVal::Empty, CacheVal::Empty) => true,
        (CacheVal::Number(x), CacheVal::Number(y)) => x == y,
        (CacheVal::Bool(x), CacheVal::Bool(y)) => x == y,
        (CacheVal::Text(x), CacheVal::Text(y)) => x == y,
        _ => false,
    }
}

fn cache_item(v: &CacheVal) -> Element {
    match v {
        CacheVal::Empty => Element::new("m"),
        CacheVal::Number(n) => Element::new("n").with_attr("v", n.clone()),
        CacheVal::Bool(b) => Element::new("b").with_attr("v", if *b { "1" } else { "0" }),
        CacheVal::Text(s) => Element::new("s").with_attr("v", s.clone()),
    }
}

fn build_cache_records(records: &[Vec<CacheVal>]) -> Document {
    let ncols = records.first().map(|r| r.len()).unwrap_or(0);
    let uniques: Vec<Vec<CacheVal>> = (0..ncols).map(|i| col_uniques(records, i)).collect();
    let mut root = Element::new("pivotCacheRecords")
        .with_attr("xmlns", SML_NS)
        .with_attr("count", records.len().to_string());
    for rec in records {
        let mut r = Element::new("r");
        for (i, v) in rec.iter().enumerate() {
            r.push_element(record_item(
                v,
                uniques.get(i).map(|u| u.as_slice()).unwrap_or(&[]),
            ));
        }
        root.push_element(r);
    }
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn col_uniques(records: &[Vec<CacheVal>], col: usize) -> Vec<CacheVal> {
    let mut unique = Vec::new();
    for rec in records {
        let Some(v) = rec.get(col) else { continue };
        if matches!(v, CacheVal::Empty) {
            continue;
        }
        if !unique.iter().any(|u| cache_eq(u, v)) {
            unique.push(v.clone());
        }
    }
    unique
}

fn unique_counts_from_records(ncols: usize, records: &[Vec<CacheVal>]) -> Vec<usize> {
    (0..ncols).map(|i| col_uniques(records, i).len()).collect()
}

fn unique_counts_from_cache(def: &Element) -> Vec<usize> {
    def.child_by_local("cacheFields")
        .map(|cf| {
            cf.children_by_local("cacheField")
                .into_iter()
                .map(|f| {
                    f.child_by_local("sharedItems")
                        .map(|s| s.element_children().count())
                        .unwrap_or(0)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn record_item(v: &CacheVal, uniques: &[CacheVal]) -> Element {
    if let CacheVal::Text(_) = v {
        if let Some(i) = uniques.iter().position(|u| cache_eq(u, v)) {
            return Element::new("x").with_attr("v", i.to_string());
        }
    }
    cache_item(v)
}

fn build_pivot_table_xml(
    cache_id: u32,
    name: &str,
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) -> Document {
    let mut root = Element::new("pivotTableDefinition")
        .with_attr("xmlns", SML_NS)
        .with_attr("name", name)
        .with_attr("cacheId", cache_id.to_string())
        .with_attr("applyNumberFormats", "0")
        .with_attr("applyBorderFormats", "0")
        .with_attr("applyFontFormats", "0")
        .with_attr("applyPatternFormats", "0")
        .with_attr("applyAlignmentFormats", "0")
        .with_attr("applyWidthHeightFormats", "1")
        .with_attr("dataCaption", "Values")
        .with_attr("createdVersion", "4")
        .with_attr("minRefreshableVersion", "3")
        .with_attr("updatedVersion", "4")
        .with_attr("showCalcMbrs", "0")
        .with_attr("useAutoFormatting", "1")
        .with_attr("itemPrintTitles", "1")
        .with_attr("indent", "0")
        .with_attr("compact", "0")
        .with_attr("compactData", "0")
        .with_attr("outline", "1")
        .with_attr("outlineData", "1")
        .with_attr("multipleFieldFilters", "0");
    push_location(&mut root, origin, axes, headers, unique_counts);
    push_layout(&mut root, axes, headers, unique_counts);
    push_style(&mut root);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
}

fn replace_layout(
    root: &mut Element,
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) {
    replace_location(root, origin, axes, headers, unique_counts);
    root.children.retain(|n| match n {
        Node::Element(e) => !matches!(
            e.local_name(),
            "pivotFields"
                | "rowFields"
                | "rowItems"
                | "colFields"
                | "colItems"
                | "pageFields"
                | "dataFields"
                | "pivotTableStyleInfo"
        ),
        _ => true,
    });
    push_layout(root, axes, headers, unique_counts);
    push_style(root);
}

fn replace_location(
    root: &mut Element,
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) {
    root.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "location",
        _ => true,
    });
    let mut loc_parent = Element::new("dummy");
    push_location(&mut loc_parent, origin, axes, headers, unique_counts);
    if let Some(Node::Element(loc)) = loc_parent.children.pop() {
        root.children.insert(0, Node::Element(loc));
    }
}

fn push_location(
    root: &mut Element,
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) {
    let end = location_end(origin, axes, headers, unique_counts);
    let page = if axes.filters.is_empty() {
        0u32
    } else {
        axes.filters.len() as u32 + 1
    };
    let first_header = page + 1;
    let mut loc = Element::new("location")
        .with_attr("ref", a1::format_range(origin, end))
        .with_attr("firstHeaderRow", first_header.to_string())
        .with_attr("firstDataRow", (first_header + 1).to_string())
        .with_attr("firstDataCol", "1");
    if !axes.filters.is_empty() {
        loc.set_attr("rowPageCount", axes.filters.len().to_string());
        loc.set_attr("colPageCount", "1");
    }
    root.push_element(loc);
}

fn push_layout(root: &mut Element, axes: &Axes, headers: &[String], unique_counts: &[usize]) {
    let mut fields = Element::new("pivotFields").with_attr("count", headers.len().to_string());
    for name in headers {
        let mut pf = Element::new("pivotField")
            .with_attr("compact", "0")
            .with_attr("outline", "0")
            .with_attr("showAll", "0");
        if axes.rows.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            pf.set_attr("axis", "axisRow");
            pf.push_element(shared_axis_items(field_unique_count(
                headers,
                name,
                unique_counts,
            )));
        } else if axes.cols.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            pf.set_attr("axis", "axisCol");
            pf.push_element(shared_axis_items(field_unique_count(
                headers,
                name,
                unique_counts,
            )));
        } else if axes.filters.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            pf.set_attr("axis", "axisPage");
            pf.push_element(shared_axis_items(field_unique_count(
                headers,
                name,
                unique_counts,
            )));
        } else if axes.values.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            pf.set_attr("dataField", "1");
        }
        fields.push_element(pf);
    }
    root.push_element(fields);
    if !axes.rows.is_empty() {
        let mut rf = Element::new("rowFields").with_attr("count", axes.rows.len().to_string());
        let mut sizes = Vec::new();
        for n in &axes.rows {
            if let Some(i) = header_index(headers, n) {
                rf.push_element(Element::new("field").with_attr("x", i.to_string()));
                sizes.push(field_unique_count(headers, n, unique_counts).max(1));
            }
        }
        root.push_element(rf);
        root.push_element(axis_items_el("rowItems", &sizes, true));
    } else {
        let mut ri = Element::new("rowItems").with_attr("count", "1");
        ri.push_element(Element::new("i"));
        root.push_element(ri);
    }
    let mut col_xs: Vec<String> = Vec::new();
    let mut col_sizes: Vec<usize> = Vec::new();
    for n in &axes.cols {
        if let Some(i) = header_index(headers, n) {
            col_xs.push(i.to_string());
            col_sizes.push(field_unique_count(headers, n, unique_counts).max(1));
        }
    }
    if axes.values.len() > 1 {
        col_xs.push("-2".into());
        col_sizes.push(axes.values.len());
    }
    if !col_xs.is_empty() {
        let mut cf = Element::new("colFields").with_attr("count", col_xs.len().to_string());
        for x in col_xs {
            cf.push_element(Element::new("field").with_attr("x", x));
        }
        root.push_element(cf);
        root.push_element(axis_items_el("colItems", &col_sizes, false));
    } else {
        let mut ci = Element::new("colItems").with_attr("count", "1");
        ci.push_element(Element::new("i"));
        root.push_element(ci);
    }
    if !axes.filters.is_empty() {
        let mut pf = Element::new("pageFields").with_attr("count", axes.filters.len().to_string());
        for n in &axes.filters {
            if let Some(i) = header_index(headers, n) {
                pf.push_element(
                    Element::new("pageField")
                        .with_attr("fld", i.to_string())
                        .with_attr("hier", "-1"),
                );
            }
        }
        root.push_element(pf);
    }
    if !axes.values.is_empty() {
        let mut df = Element::new("dataFields").with_attr("count", axes.values.len().to_string());
        for n in &axes.values {
            if let Some(i) = header_index(headers, n) {
                df.push_element(
                    Element::new("dataField")
                        .with_attr("name", n.clone())
                        .with_attr("fld", i.to_string())
                        .with_attr("baseField", "0")
                        .with_attr("baseItem", "0"),
                );
            }
        }
        root.push_element(df);
    }
}

fn push_style(root: &mut Element) {
    root.push_element(
        Element::new("pivotTableStyleInfo")
            .with_attr("name", "PivotStyleLight16")
            .with_attr("showRowHeaders", "1")
            .with_attr("showColHeaders", "1")
            .with_attr("showRowStripes", "0")
            .with_attr("showColStripes", "0")
            .with_attr("showLastColumn", "1"),
    );
}

fn shared_axis_items(n_unique: usize) -> Element {
    let count = n_unique + 1;
    let mut items = Element::new("items").with_attr("count", count.to_string());
    for i in 0..n_unique {
        items.push_element(Element::new("item").with_attr("x", i.to_string()));
    }
    items.push_element(Element::new("item").with_attr("t", "default"));
    items
}

fn axis_items_el(tag: &str, sizes: &[usize], with_grand: bool) -> Element {
    let combos = enumerate_combos(sizes);
    let count = combos.len() + usize::from(with_grand);
    let mut el = Element::new(tag).with_attr("count", count.to_string());
    for combo in combos {
        let mut i = Element::new("i");
        if combo.is_empty() {
            i.push_element(Element::new("x"));
        } else {
            for v in combo {
                i.push_element(Element::new("x").with_attr("v", v.to_string()));
            }
        }
        el.push_element(i);
    }
    if with_grand {
        let mut g = Element::new("i").with_attr("t", "grand");
        g.push_element(Element::new("x"));
        el.push_element(g);
    }
    el
}

fn enumerate_combos(sizes: &[usize]) -> Vec<Vec<usize>> {
    if sizes.is_empty() {
        return vec![Vec::new()];
    }
    let mut out = vec![Vec::new()];
    for &n in sizes {
        let n = n.max(1);
        let mut next = Vec::with_capacity(out.len().saturating_mul(n).min(256));
        for prefix in &out {
            for v in 0..n {
                if next.len() >= 256 {
                    break;
                }
                let mut row = prefix.clone();
                row.push(v);
                next.push(row);
            }
        }
        out = next;
    }
    out
}

fn add_sheet_pivot_rel(pkg: &mut Package, sheet_part: &str, table_part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid,
        rel_type: REL_PIVOT_TABLE.into(),
        target: picture::rel_target_from(sheet_part, table_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, sheet_part)
}

fn set_table_cache_rel(pkg: &mut Package, table_part: &str, def_part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, table_part)?;
    let target = picture::rel_target_from(table_part, def_part);
    let mut found = false;
    for r in &mut rels.rels {
        if r.rel_type.ends_with("/pivotCacheDefinition") {
            r.target = target.clone();
            found = true;
        }
    }
    if !found {
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid,
            rel_type: REL_PIVOT_CACHE_DEF.into(),
            target,
            target_mode: None,
            extra: Vec::new(),
        });
    }
    rels.write_to(pkg, table_part)
}

fn add_workbook_cache(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    cache_id: u32,
    def_part: &str,
) -> Result<(), Error> {
    picture::ensure_xmlns(&mut wb_doc.root, "r", REL_NS);
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_PIVOT_CACHE_DEF.into(),
        target: picture::rel_target_from(wb_part, def_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, wb_part)?;
    let cache = Element::new("pivotCache")
        .with_attr("cacheId", cache_id.to_string())
        .with_attr("r:id", rid);
    if let Some(parent) = wb_doc.root.child_by_local_mut("pivotCaches") {
        parent.push_element(cache);
        let n = parent.children_by_local("pivotCache").len();
        parent.set_attr("count", n.to_string());
    } else {
        let mut parent = Element::new("pivotCaches").with_attr("count", "1");
        parent.push_element(cache);
        insert_workbook_child(&mut wb_doc.root, parent, "pivotCaches");
    }
    Ok(())
}

fn remove_workbook_cache(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    cache_id: u32,
) -> Result<(), Error> {
    let rid = {
        let Some(parent) = wb_doc.root.child_by_local("pivotCaches") else {
            return Ok(());
        };
        parent
            .children_by_local("pivotCache")
            .into_iter()
            .find(|el| el.attr("cacheId") == Some(cache_id.to_string().as_str()))
            .and_then(|el| el.attr_local("id").map(str::to_string))
    };
    if let Some(parent) = wb_doc.root.child_by_local_mut("pivotCaches") {
        let want = cache_id.to_string();
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "pivotCache" => {
                e.attr("cacheId") != Some(want.as_str())
            }
            _ => true,
        });
        let left = parent.children_by_local("pivotCache").len();
        if left == 0 {
            wb_doc.root.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != "pivotCaches",
                _ => true,
            });
        } else {
            parent.set_attr("count", left.to_string());
        }
    }
    if let Some(rid) = rid {
        let mut rels = Relationships::from_package(pkg, wb_part)?;
        rels.remove_by_id(&rid);
        rels.write_to(pkg, wb_part)?;
    }
    Ok(())
}

fn insert_workbook_child(root: &mut Element, el: Element, local: &str) {
    let want = WORKBOOK_CHILD_ORDER
        .iter()
        .position(|n| *n == local)
        .unwrap_or(WORKBOOK_CHILD_ORDER.len());
    let mut at = root.children.len();
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        let pos = WORKBOOK_CHILD_ORDER
            .iter()
            .position(|n| *n == e.local_name())
            .unwrap_or(WORKBOOK_CHILD_ORDER.len());
        if pos > want {
            at = i;
            break;
        }
    }
    root.children.insert(at, Node::Element(el));
}

fn pivot_part_at(
    pkg: &Package,
    sheet_part: &str,
    sheet: &str,
    index: usize,
    op_index: usize,
) -> Result<(String, String), Error> {
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let pts: Vec<&Relationship> = rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/pivotTable"))
        .collect();
    let rel = if super::list_pin::pivots_frozen(sheet_part) {
        let Some(rid) = super::list_pin::pivot_rid(sheet_part, index) else {
            return Err(not_found(sheet, index, op_index));
        };
        pts.iter()
            .copied()
            .find(|r| r.id == rid)
            .ok_or_else(|| not_found(sheet, index, op_index))?
    } else {
        if index == 0 || index > pts.len() {
            return Err(not_found(sheet, index, op_index));
        }
        pts[index - 1]
    };
    let part = resolve_rel_target(sheet_part, &rel.target)?;
    Ok((part, rel.id.clone()))
}

fn cache_parts_of(
    pkg: &Package,
    wb_doc: &Document,
    wb_part: &str,
    cache_id: u32,
    op_index: usize,
) -> Result<(String, String), Error> {
    let parent = wb_doc.root.child_by_local("pivotCaches").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidPivotTable {
                field: "source".into(),
            },
            format!("ops[{op_index}].source"),
        )
    })?;
    let el = parent
        .children_by_local("pivotCache")
        .into_iter()
        .find(|e| e.attr("cacheId") == Some(cache_id.to_string().as_str()))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidPivotTable {
                    field: "source".into(),
                },
                format!("ops[{op_index}].source"),
            )
        })?;
    let rid = el.attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "pivotCache".into(),
            child: "r:id".into(),
        })
    })?;
    let wb_rels = Relationships::from_package(pkg, wb_part)?;
    let rel = wb_rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(wb_part.to_string()),
            rid: rid.to_string(),
        })
    })?;
    let def_part = resolve_rel_target(wb_part, &rel.target)?;
    let def_rels = Relationships::from_package(pkg, &def_part)?;
    let rec_rel = def_rels
        .rels
        .iter()
        .find(|r| r.rel_type.ends_with("/pivotCacheRecords"))
        .ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "pivotCacheDefinition".into(),
                child: "pivotCacheRecords".into(),
            })
        })?;
    let rec_part = resolve_rel_target(&def_part, &rec_rel.target)?;
    Ok((def_part, rec_part))
}

fn load_sst(pkg: &Package, wb_part: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    super::inspect::load_shared_strings(pkg, wb_part, &rels)
}

fn read_source_owned(
    pkg: &Package,
    dirty_sheets: &[(String, Document)],
    part: &str,
    start: CellRef,
    end: CellRef,
    sst: &[String],
) -> Result<(Vec<String>, Vec<Vec<CacheVal>>), Error> {
    if let Some((_, doc)) = dirty_sheets.iter().find(|(p, _)| p == part) {
        return Ok(read_source(&doc.root, start, end, sst));
    }
    let owned = xml::parse(pkg.get(part)?)?;
    Ok(read_source(&owned.root, start, end, sst))
}

fn read_source(
    root: &Element,
    start: CellRef,
    end: CellRef,
    sst: &[String],
) -> (Vec<String>, Vec<Vec<CacheVal>>) {
    let mut headers = Vec::new();
    let mut used: Vec<String> = Vec::new();
    for col in start.col..=end.col {
        let cell = CellRef {
            col,
            row: start.row,
        };
        let raw = match cell_val(root, cell, sst) {
            CacheVal::Text(s) => s,
            CacheVal::Number(n) => n,
            CacheVal::Bool(b) => if b { "TRUE" } else { "FALSE" }.into(),
            CacheVal::Empty => String::new(),
        };
        let base = if raw.is_empty() {
            format!("Column{}", col - start.col + 1)
        } else {
            raw
        };
        let name = unique_name(&base, &used);
        used.push(name.to_ascii_lowercase());
        headers.push(name);
    }
    let mut records = Vec::new();
    for row in (start.row + 1)..=end.row {
        let mut rec = Vec::new();
        for col in start.col..=end.col {
            rec.push(cell_val(root, CellRef { col, row }, sst));
        }
        records.push(rec);
    }
    (headers, records)
}

fn unique_name(base: &str, used_lower: &[String]) -> String {
    if !used_lower.iter().any(|s| s.eq_ignore_ascii_case(base)) {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let cand = format!("{base}{n}");
        if !used_lower.iter().any(|s| s.eq_ignore_ascii_case(&cand)) {
            return cand;
        }
        n += 1;
    }
}

fn cell_val(root: &Element, cell: CellRef, sst: &[String]) -> CacheVal {
    let Some(sd) = root.child_by_local("sheetData") else {
        return CacheVal::Empty;
    };
    let want = a1::format_cell(cell.col, cell.row);
    for row in sd.children_by_local("row") {
        for c in row.children_by_local("c") {
            if c.attr("r") != Some(want.as_str()) {
                continue;
            }
            let t = c.attr("t").unwrap_or("n");
            return match t {
                "s" => {
                    let text = c
                        .child_by_local("v")
                        .and_then(|v| v.text_content().parse::<usize>().ok())
                        .and_then(|i| sst.get(i).cloned())
                        .unwrap_or_default();
                    if text.is_empty() {
                        CacheVal::Empty
                    } else {
                        CacheVal::Text(text)
                    }
                }
                "inlineStr" => {
                    let text = c
                        .child_by_local("is")
                        .map(|is| is.text_content())
                        .unwrap_or_default();
                    if text.is_empty() {
                        CacheVal::Empty
                    } else {
                        CacheVal::Text(text)
                    }
                }
                "b" => {
                    let v = c
                        .child_by_local("v")
                        .map(|v| v.text_content())
                        .unwrap_or_default();
                    CacheVal::Bool(v == "1" || v.eq_ignore_ascii_case("true"))
                }
                "str" | "e" | "d" => {
                    let text = c
                        .child_by_local("v")
                        .map(|v| v.text_content())
                        .unwrap_or_default();
                    if text.is_empty() {
                        CacheVal::Empty
                    } else {
                        CacheVal::Text(text)
                    }
                }
                _ => {
                    let text = c
                        .child_by_local("v")
                        .map(|v| v.text_content())
                        .unwrap_or_default();
                    if text.is_empty() {
                        CacheVal::Empty
                    } else if text.parse::<f64>().is_ok() {
                        CacheVal::Number(text)
                    } else {
                        CacheVal::Text(text)
                    }
                }
            };
        }
    }
    CacheVal::Empty
}

struct RowSumPlan {
    host: usize,
    origin: CellRef,
    row_name: String,
    value_names: Vec<String>,
    groups: Vec<(String, Vec<f64>)>,
}

/// 透视区域。行字段求和时带上各组，其它版式只有范围。
pub(crate) struct PivotCover {
    pub host: usize,
    pub start: CellRef,
    pub end: CellRef,
    pub row_sum: Option<RowSumView>,
}

/// 一个行字段、求和、没有列字段和筛选时的分组。
#[derive(Clone)]
pub(crate) struct RowSumView {
    pub row_name: String,
    pub value_names: Vec<String>,
    pub groups: Vec<(String, Vec<f64>)>,
}

/// 每张透视的位置。求值用它判断 `GETPIVOTDATA`，不改工作表。
pub(crate) fn pivot_covers(
    pkg: &Package,
    sheet_docs: &[(String, Document)],
    names: &[String],
    sst: &[String],
) -> Result<Vec<PivotCover>, Error> {
    let wb_part = crate::opc::office_document_part(pkg)?;
    let caches = load_caches(pkg, &wb_part)?;
    let mut out = Vec::new();
    for (host_i, (part, _)) in sheet_docs.iter().enumerate() {
        let rels = Relationships::from_package(pkg, part)?;
        for rel in &rels.rels {
            if !rel.rel_type.ends_with("/pivotTable") {
                continue;
            }
            let Ok(target) = resolve_rel_target(part, &rel.target) else {
                continue;
            };
            let Ok(bytes) = pkg.get(&target) else {
                continue;
            };
            let Ok(table) = xml::parse(bytes) else {
                continue;
            };
            let Some(loc) = table
                .root
                .child_by_local("location")
                .and_then(|l| l.attr("ref"))
            else {
                continue;
            };
            let Ok((start, end)) = a1::parse_range(loc) else {
                continue;
            };
            let row_sum =
                plan_row_sum(&table.root, host_i, &caches, sheet_docs, names, sst).map(|plan| {
                    RowSumView {
                        row_name: plan.row_name,
                        value_names: plan.value_names,
                        groups: plan.groups,
                    }
                });
            out.push(PivotCover {
                host: host_i,
                start,
                end,
                row_sum,
            });
        }
    }
    Ok(out)
}

/// 只有一个行字段、若干求和、没有列字段和筛选时，把表头、各组和总计写进工作表。
pub(crate) fn fill_row_sum_display(
    pkg: &Package,
    sheet_docs: &mut [(String, Document)],
    names: &[String],
    sst: &[String],
) -> Result<Vec<usize>, Error> {
    let wb_part = crate::opc::office_document_part(pkg)?;
    let caches = load_caches(pkg, &wb_part)?;
    let mut plans = Vec::new();
    for (host_i, (part, _)) in sheet_docs.iter().enumerate() {
        let rels = Relationships::from_package(pkg, part)?;
        for rel in &rels.rels {
            if !rel.rel_type.ends_with("/pivotTable") {
                continue;
            }
            let Ok(target) = resolve_rel_target(part, &rel.target) else {
                continue;
            };
            let Ok(bytes) = pkg.get(&target) else {
                continue;
            };
            let Ok(table) = xml::parse(bytes) else {
                continue;
            };
            if let Some(plan) = plan_row_sum(&table.root, host_i, &caches, sheet_docs, names, sst) {
                plans.push(plan);
            }
        }
    }
    let mut hosts = Vec::new();
    for plan in plans {
        let host_i = plan.host;
        let host = &mut sheet_docs[host_i].1.root;
        if paint_row_sum(host, &plan) {
            hosts.push(host_i);
        }
    }
    Ok(hosts)
}

fn plan_row_sum(
    root: &Element,
    host: usize,
    caches: &[(u32, CacheInfo)],
    sheet_docs: &[(String, Document)],
    names: &[String],
    sst: &[String],
) -> Option<RowSumPlan> {
    if root
        .child_by_local("pageFields")
        .is_some_and(|p| !p.children_by_local("pageField").is_empty())
    {
        return None;
    }
    if root.child_by_local("colFields").is_some_and(|p| {
        p.children_by_local("field")
            .iter()
            .any(|f| f.attr("x") != Some("-2"))
    }) {
        return None;
    }
    let row_fields = root
        .child_by_local("rowFields")
        .map(|p| p.children_by_local("field"))
        .unwrap_or_default();
    if row_fields.len() != 1 {
        return None;
    }
    let row_idx = row_fields[0]
        .attr("x")
        .and_then(|s| s.parse::<usize>().ok())?;
    let data_fields = root
        .child_by_local("dataFields")
        .map(|p| p.children_by_local("dataField"))
        .unwrap_or_default();
    if data_fields.is_empty() {
        return None;
    }
    let mut value_idxs = Vec::new();
    let mut value_names = Vec::new();
    for field in &data_fields {
        if let Some(sub) = field.attr("subtotal") {
            if !sub.eq_ignore_ascii_case("sum") {
                return None;
            }
        }
        let idx = field.attr("fld").and_then(|s| s.parse::<usize>().ok())?;
        value_idxs.push(idx);
        value_names.push(field.attr("name").unwrap_or("").to_string());
    }
    let cache_id = root.attr("cacheId").and_then(|s| s.parse::<u32>().ok())?;
    let cache = caches
        .iter()
        .find(|(id, _)| *id == cache_id)
        .map(|(_, c)| c)?;
    let (src_sheet, src_ref) = cache.source.as_ref()?;
    let src_i = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(src_sheet))?;
    let (start, end) = a1::parse_range(src_ref).ok()?;
    if row_idx >= cache.fields.len() || value_idxs.iter().any(|i| *i >= cache.fields.len()) {
        return None;
    }
    let (_headers, records) = read_source(&sheet_docs[src_i].1.root, start, end, sst);
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    for rec in &records {
        if row_idx >= rec.len() || value_idxs.iter().any(|i| *i >= rec.len()) {
            continue;
        }
        let label = cache_label(&rec[row_idx]);
        let nums: Vec<f64> = value_idxs.iter().map(|i| cache_num(&rec[*i])).collect();
        if let Some(found) = groups.iter_mut().find(|g| g.0 == label) {
            for (slot, n) in found.1.iter_mut().zip(nums) {
                *slot += n;
            }
        } else {
            groups.push((label, nums));
        }
    }
    let loc = root
        .child_by_local("location")
        .and_then(|l| l.attr("ref"))?;
    let (origin, _) = a1::parse_range(loc).ok()?;
    let row_name = cache.fields[row_idx].clone();
    Some(RowSumPlan {
        host,
        origin,
        row_name,
        value_names,
        groups,
    })
}

fn paint_row_sum(host: &mut Element, plan: &RowSumPlan) -> bool {
    let origin = plan.origin;
    let mut changed = put_text(host, origin, &plan.row_name);
    for (i, name) in plan.value_names.iter().enumerate() {
        let cell = CellRef {
            col: origin.col + 1 + i as u32,
            row: origin.row,
        };
        changed |= put_text(host, cell, name);
    }
    let mut totals = vec![0.0; plan.value_names.len()];
    for (offset, (label, nums)) in plan.groups.iter().enumerate() {
        let row = origin.row + 1 + offset as u32;
        changed |= put_text(
            host,
            CellRef {
                col: origin.col,
                row,
            },
            label,
        );
        for (i, n) in nums.iter().enumerate() {
            totals[i] += n;
            changed |= put_number(
                host,
                CellRef {
                    col: origin.col + 1 + i as u32,
                    row,
                },
                *n,
            );
        }
    }
    let total_row = origin.row + 1 + plan.groups.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (i, n) in totals.iter().enumerate() {
        changed |= put_number(
            host,
            CellRef {
                col: origin.col + 1 + i as u32,
                row: total_row,
            },
            *n,
        );
    }
    changed
}

fn cache_label(v: &CacheVal) -> String {
    match v {
        CacheVal::Text(s) | CacheVal::Number(s) => s.clone(),
        CacheVal::Bool(true) => "TRUE".into(),
        CacheVal::Bool(false) => "FALSE".into(),
        CacheVal::Empty => String::new(),
    }
}

fn cache_num(v: &CacheVal) -> f64 {
    match v {
        CacheVal::Number(s) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn put_number(root: &mut Element, cell: CellRef, n: f64) -> bool {
    let text = if (n - n.round()).abs() < 1e-9 && n.abs() < 1e15 {
        format!("{}", n.round() as i64)
    } else {
        format!("{n}")
    };
    let sheet_data = super::cell::ensure_sheet_data(root);
    let c = super::cell::ensure_cell(sheet_data, cell);
    let same = c.attr("t").is_none()
        && c.child_by_local("f").is_none()
        && c.child_by_local("v").map(|v| v.text_content()) == Some(text.clone());
    if same {
        return false;
    }
    c.remove_attr("t");
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "v" | "is" | "f"),
        Node::Text(_) | Node::Raw(_) => true,
    });
    let mut v = Element::new("v");
    v.set_text(text);
    c.push_element(v);
    true
}

fn put_text(root: &mut Element, cell: CellRef, text: &str) -> bool {
    let sheet_data = super::cell::ensure_sheet_data(root);
    let c = super::cell::ensure_cell(sheet_data, cell);
    let cur = if c.attr("t") == Some("inlineStr") {
        c.child_by_local("is").map(|is| is.text_content())
    } else {
        None
    };
    if cur.as_deref() == Some(text) && c.child_by_local("f").is_none() {
        return false;
    }
    c.set_attr("t", "inlineStr");
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "v" | "is" | "f"),
        Node::Text(_) | Node::Raw(_) => true,
    });
    let mut t = Element::new("t");
    if text.starts_with(' ') || text.ends_with(' ') || text.contains('\n') {
        t.set_attr("xml:space", "preserve");
    }
    t.set_text(text);
    let mut inline = Element::new("is");
    inline.push_element(t);
    c.push_element(inline);
    true
}

fn bad_field(field: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidPivotTable {
            field: field.into(),
        },
        format!("ops[{op_index}].{field}"),
    )
}

fn not_found(sheet: &str, index: usize, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::PivotTableNotFound {
            id: format!("sheet:{sheet}/pivot:{index}"),
        },
        format!("ops[{op_index}].id"),
    )
}
