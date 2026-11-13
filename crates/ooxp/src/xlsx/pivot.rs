//! 工作表透视表：inspect 名单，apply 写缓存与透视表部件。
//! 行字段、列字段（见 `plan_display`）、筛选，以及计数、平均值和合计占比，求值把合计写进 `sheetData`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1::{self, CellRef};
use super::picture;
use super::request::{
    DeletePivotTableOp, InsertPivotTableOp, PivotAgg, PivotValue, SetPivotTableOp,
};
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
    values: Vec<PivotValue>,
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
    items: Vec<Vec<String>>,
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
    let mut fields = Vec::new();
    let mut items = Vec::new();
    if let Some(cf) = root.child_by_local("cacheFields") {
        for field in cf.children_by_local("cacheField") {
            fields.push(field.attr("name").unwrap_or("").to_string());
            items.push(shared_labels(field));
        }
    }
    CacheInfo {
        source,
        fields,
        items,
    }
}

fn shared_labels(field: &Element) -> Vec<String> {
    let Some(items) = field.child_by_local("sharedItems") else {
        return Vec::new();
    };
    items.element_children().map(shared_item_label).collect()
}

fn shared_item_label(el: &Element) -> String {
    match el.local_name() {
        "b" => {
            if el.attr("v") == Some("1") {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        "m" => String::new(),
        _ => el.attr("v").unwrap_or("").to_string(),
    }
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
    let covered = covered_origin(op.location, axes.filters.len());
    if src_part == sheet_part && ranges_overlap(src_start, src_end, covered, loc_end) {
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
        let covered = covered_origin(origin, axes.filters.len());
        if src_part == sheet_part && ranges_overlap(src.range.0, src.range.1, covered, loc_end) {
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
        values: read_value_fields(root, fields),
    }
}

fn read_value_fields(root: &Element, fields: &[String]) -> Vec<PivotValue> {
    let Some(parent) = root.child_by_local("dataFields") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for field in parent.children_by_local("dataField") {
        let Some(idx) = field.attr("fld").and_then(|s| s.parse::<usize>().ok()) else {
            continue;
        };
        let Some(name) = fields.get(idx) else {
            continue;
        };
        let shown = field
            .attr("name")
            .filter(|s| !s.is_empty())
            .unwrap_or(name)
            .to_string();
        let agg = match field.attr("subtotal").unwrap_or("sum") {
            s if s.eq_ignore_ascii_case("count") => PivotAgg::Count,
            s if s.eq_ignore_ascii_case("average") => PivotAgg::Average,
            s if s.eq_ignore_ascii_case("max") => PivotAgg::Max,
            s if s.eq_ignore_ascii_case("min") => PivotAgg::Min,
            _ => PivotAgg::Sum,
        };
        let percent = field
            .attr("showDataAs")
            .is_some_and(|s| s.eq_ignore_ascii_case("percentOfTotal"));
        out.push(PivotValue {
            field: name.clone(),
            name: shown,
            agg,
            percent,
        });
    }
    out
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

fn value_names(values: &[PivotValue]) -> Vec<String> {
    values.iter().map(|v| v.field.clone()).collect()
}

fn check_axes(axes: &Axes, headers: &[String], op_index: usize) -> Result<(), Error> {
    check_axis_unique(axes, op_index)?;
    let values = value_names(&axes.values);
    for (field, names) in [
        ("rows", axes.rows.as_slice()),
        ("cols", axes.cols.as_slice()),
        ("filters", axes.filters.as_slice()),
        ("values", values.as_slice()),
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
    let values = value_names(&axes.values);
    for (field, names) in [
        ("rows", axes.rows.as_slice()),
        ("cols", axes.cols.as_slice()),
        ("filters", axes.filters.as_slice()),
        ("values", values.as_slice()),
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

fn covered_origin(origin: CellRef, filters: usize) -> CellRef {
    if filters == 0 || origin.row <= 1 {
        return origin;
    }
    CellRef {
        col: origin.col,
        row: origin.row.saturating_sub(filters as u32 + 1).max(1),
    }
}

fn location_end(
    origin: CellRef,
    axes: &Axes,
    headers: &[String],
    unique_counts: &[usize],
) -> CellRef {
    let value_n = axes.values.len().max(1);
    let header = if axes.cols.is_empty() {
        1
    } else {
        axes.cols.len() as u32 + u32::from(value_n > 1) + 1
    };
    let leaves = if axes.rows.is_empty() {
        1
    } else {
        axis_unique_product(&axes.rows, headers, unique_counts)
    };
    let subs = subtotal_bound(&axes.rows, headers, unique_counts);
    let value_cols = if axes.cols.is_empty() {
        value_n as u32
    } else {
        let slots = col_tree_slots(&axes.cols, headers, unique_counts);
        let n = value_n as u32;
        slots.saturating_mul(n).saturating_add(n)
    };
    let row_cols = if axes.rows.is_empty() {
        0
    } else {
        axes.rows.len() as u32
    };
    let width = (row_cols + value_cols).max(1);
    let height = header + leaves + subs + 1;
    CellRef {
        col: origin.col + width - 1,
        row: origin.row + height - 1,
    }
}

fn subtotal_bound(names: &[String], headers: &[String], unique_counts: &[usize]) -> u32 {
    if names.len() < 2 {
        return 0;
    }
    let mut n = 1usize;
    for name in &names[..names.len() - 1] {
        let u = header_index(headers, name)
            .and_then(|i| unique_counts.get(i).copied())
            .unwrap_or(1)
            .max(1);
        n = n.saturating_mul(u).min(256);
    }
    n as u32
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

/// 列轴占位的上界：末级只算项，非末级再加一列「某项 汇总」。
fn col_tree_slots(names: &[String], headers: &[String], unique_counts: &[usize]) -> u32 {
    fn walk(names: &[String], headers: &[String], unique_counts: &[usize], i: usize) -> u32 {
        let u = header_index(headers, &names[i])
            .and_then(|idx| unique_counts.get(idx).copied())
            .unwrap_or(1)
            .max(1) as u32;
        let u = u.min(256);
        if i + 1 == names.len() {
            return u;
        }
        let child = walk(names, headers, unique_counts, i + 1);
        u.saturating_mul(child.saturating_add(1)).min(256)
    }
    if names.is_empty() {
        0
    } else {
        walk(names, headers, unique_counts, 0)
    }
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
    let mut loc = Element::new("location")
        .with_attr("ref", a1::format_range(origin, end))
        .with_attr("firstHeaderRow", "1")
        .with_attr("firstDataRow", "2")
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
        } else if axes
            .values
            .iter()
            .any(|v| v.field.eq_ignore_ascii_case(name))
        {
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
        for v in &axes.values {
            if let Some(i) = header_index(headers, &v.field) {
                let mut field = Element::new("dataField")
                    .with_attr("name", v.name.clone())
                    .with_attr("fld", i.to_string())
                    .with_attr("baseField", "0")
                    .with_attr("baseItem", "0");
                let sub = match v.agg {
                    PivotAgg::Sum => None,
                    PivotAgg::Count => Some("count"),
                    PivotAgg::Average => Some("average"),
                    PivotAgg::Max => Some("max"),
                    PivotAgg::Min => Some("min"),
                };
                if let Some(sub) = sub {
                    field.set_attr("subtotal", sub);
                }
                if v.percent {
                    field.set_attr("showDataAs", "percentOfTotal");
                }
                df.push_element(field);
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

/// 透视区域。一个行字段且没有列字段时带上各组。
pub(crate) struct PivotCover {
    pub host: usize,
    pub start: CellRef,
    pub end: CellRef,
    pub row_sum: Option<RowSumView>,
}

/// 一个行字段、没有列字段时的分组。数字是已经写在格子上的结果。
#[derive(Clone)]
pub(crate) struct RowSumView {
    pub row_name: String,
    pub value_names: Vec<String>,
    pub groups: Vec<(String, Vec<f64>)>,
    pub grand: Vec<f64>,
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
            let row_sum = plan_display(&table.root, host_i, &caches, sheet_docs, names, sst)
                .and_then(|plan| plan.view);
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

/// 把能确定的透视合计写进工作表。多个列字段，或列字段再加多个行字段、多个值，跳过。
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
            if let Some(plan) = plan_display(&table.root, host_i, &caches, sheet_docs, names, sst) {
                plans.push(plan);
            }
        }
    }
    let mut hosts = Vec::new();
    for plan in plans {
        let host_i = plan.host;
        let host = &mut sheet_docs[host_i].1.root;
        if paint_display(host, &plan) {
            hosts.push(host_i);
        }
    }
    Ok(hosts)
}

#[derive(Clone, Copy)]
enum AggKind {
    Sum,
    Count,
    Average,
    Max,
    Min,
}

struct ValSpec {
    name: String,
    idx: usize,
    kind: AggKind,
    percent: bool,
}

#[derive(Clone, Copy)]
struct Measure {
    sum: f64,
    n_num: u32,
    n_all: u32,
    max: f64,
    min: f64,
    has_bound: bool,
    seen: bool,
}

impl Measure {
    fn new() -> Self {
        Self {
            sum: 0.0,
            n_num: 0,
            n_all: 0,
            max: 0.0,
            min: 0.0,
            has_bound: false,
            seen: false,
        }
    }

    fn add(&mut self, v: &CacheVal) {
        self.seen = true;
        match v {
            CacheVal::Empty => {}
            CacheVal::Number(s) => {
                let n = s.parse::<f64>().unwrap_or(0.0);
                if !self.has_bound {
                    self.max = n;
                    self.min = n;
                    self.has_bound = true;
                } else {
                    self.max = self.max.max(n);
                    self.min = self.min.min(n);
                }
                self.sum += n;
                self.n_num += 1;
                self.n_all += 1;
            }
            CacheVal::Text(_) | CacheVal::Bool(_) => self.n_all += 1,
        }
    }

    fn merge(&mut self, other: &Measure) {
        if !other.seen {
            return;
        }
        if !self.seen {
            *self = *other;
            return;
        }
        self.sum += other.sum;
        self.n_num += other.n_num;
        self.n_all += other.n_all;
        if !other.has_bound {
            return;
        }
        if !self.has_bound {
            self.max = other.max;
            self.min = other.min;
            self.has_bound = true;
            return;
        }
        self.max = self.max.max(other.max);
        self.min = self.min.min(other.min);
    }

    fn finish(&self, kind: AggKind) -> Option<f64> {
        if !self.seen {
            return None;
        }
        Some(match kind {
            AggKind::Sum => self.sum,
            AggKind::Count => f64::from(self.n_all),
            AggKind::Average => {
                if self.n_num == 0 {
                    0.0
                } else {
                    self.sum / f64::from(self.n_num)
                }
            }
            AggKind::Max => {
                if self.has_bound {
                    self.max
                } else {
                    0.0
                }
            }
            AggKind::Min => {
                if self.has_bound {
                    self.min
                } else {
                    0.0
                }
            }
        })
    }
}

struct RowGroup {
    key: Vec<String>,
    measures: Vec<Measure>,
}

struct RowBody {
    field_names: Vec<String>,
    groups: Vec<RowGroup>,
    specs: Vec<ValSpec>,
}

struct CrossSlot {
    row: String,
    col: String,
    measure: Measure,
}

struct CrossBody {
    value_name: String,
    row_name: String,
    col_name: String,
    row_labels: Vec<String>,
    col_labels: Vec<String>,
    cells: Vec<CrossSlot>,
    spec: ValSpec,
}

enum ColSlot {
    Leaf { path: Vec<String> },
    Subtotal { path: Vec<String> },
    Grand,
}

struct ColHit {
    row: String,
    path: Vec<String>,
    measure: Measure,
}

struct ColTreeBody {
    row_name: String,
    col_names: Vec<String>,
    value_name: String,
    columns: Vec<ColSlot>,
    row_labels: Vec<String>,
    hits: Vec<ColHit>,
    spec: ValSpec,
}

struct RowColHit {
    key: Vec<String>,
    col: String,
    measure: Measure,
}

struct RowColBody {
    field_names: Vec<String>,
    col_name: String,
    value_name: String,
    col_labels: Vec<String>,
    row_keys: Vec<Vec<String>>,
    hits: Vec<RowColHit>,
    spec: ValSpec,
}

struct ValueHit {
    row: String,
    col: String,
    measures: Vec<Measure>,
}

struct MultiValueBody {
    row_name: String,
    col_name: String,
    caption: String,
    value_names: Vec<String>,
    row_labels: Vec<String>,
    col_labels: Vec<String>,
    hits: Vec<ValueHit>,
    specs: Vec<ValSpec>,
}

enum DisplayBody {
    Rows(RowBody),
    Cross(CrossBody),
    ColTree(ColTreeBody),
    RowCol(RowColBody),
    MultiValue(MultiValueBody),
}

struct DisplayPlan {
    host: usize,
    origin: CellRef,
    pages: Vec<(String, String)>,
    body: DisplayBody,
    view: Option<RowSumView>,
}

struct PagePick {
    idx: usize,
    item: Option<usize>,
    name: String,
}

fn plan_display(
    root: &Element,
    host: usize,
    caches: &[(u32, CacheInfo)],
    sheet_docs: &[(String, Document)],
    names: &[String],
    sst: &[String],
) -> Option<DisplayPlan> {
    let row_idxs = axis_indexes(root.child_by_local("rowFields"))?;
    let col_idxs = axis_indexes(root.child_by_local("colFields"))?;
    if row_idxs.is_empty() {
        return None;
    }
    let specs = value_specs(root)?;
    let cache_id = root.attr("cacheId")?.parse::<u32>().ok()?;
    let cache = caches
        .iter()
        .find(|(id, _)| *id == cache_id)
        .map(|(_, c)| c)?;
    if row_idxs.iter().any(|i| *i >= cache.fields.len())
        || col_idxs.iter().any(|i| *i >= cache.fields.len())
        || specs.iter().any(|s| s.idx >= cache.fields.len())
    {
        return None;
    }
    let pages = page_picks(root, cache)?;
    let (src_sheet, src_ref) = cache.source.as_ref()?;
    let src_i = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(src_sheet))?;
    let (start, end) = a1::parse_range(src_ref).ok()?;
    let (_headers, records) = read_source(&sheet_docs[src_i].1.root, start, end, sst);
    let kept = filter_records(records, cache, &pages);
    let (origin, _) = a1::parse_range(root.child_by_local("location")?.attr("ref")?).ok()?;
    let field_names: Vec<String> = row_idxs.iter().map(|i| cache.fields[*i].clone()).collect();
    let page_labels = page_labels(&pages, cache);
    if col_idxs.is_empty() {
        let groups = row_groups(&kept, &row_idxs, &specs);
        let view = if row_idxs.len() == 1 {
            Some(row_view(&field_names, &groups, &specs))
        } else {
            None
        };
        return Some(DisplayPlan {
            host,
            origin,
            pages: page_labels,
            body: DisplayBody::Rows(RowBody {
                field_names,
                groups,
                specs,
            }),
            view,
        });
    }
    let fields = &cache.fields;
    if col_idxs.len() >= 2 && row_idxs.len() == 1 && specs.len() == 1 {
        let spec = specs.into_iter().next()?;
        return Some(DisplayPlan {
            host,
            origin,
            pages: page_labels,
            body: DisplayBody::ColTree(col_tree_body(&kept, row_idxs[0], &col_idxs, fields, spec)),
            view: None,
        });
    }
    if col_idxs.len() == 1 && row_idxs.len() >= 2 && specs.len() == 1 {
        let spec = specs.into_iter().next()?;
        return Some(DisplayPlan {
            host,
            origin,
            pages: page_labels,
            body: DisplayBody::RowCol(row_col_body(&kept, &row_idxs, col_idxs[0], fields, spec)),
            view: None,
        });
    }
    if col_idxs.len() == 1 && row_idxs.len() == 1 && specs.len() >= 2 {
        let caption = root.attr("dataCaption").unwrap_or("Values").to_string();
        return Some(DisplayPlan {
            host,
            origin,
            pages: page_labels,
            body: DisplayBody::MultiValue(multi_value_body(
                &kept,
                row_idxs[0],
                col_idxs[0],
                fields,
                caption,
                specs,
            )),
            view: None,
        });
    }
    if col_idxs.len() == 1 && row_idxs.len() == 1 && specs.len() == 1 {
        let spec = specs.into_iter().next()?;
        return Some(DisplayPlan {
            host,
            origin,
            pages: page_labels,
            body: DisplayBody::Cross(cross_body(&kept, row_idxs[0], col_idxs[0], fields, spec)),
            view: None,
        });
    }
    None
}

fn axis_indexes(parent: Option<&Element>) -> Option<Vec<usize>> {
    let Some(parent) = parent else {
        return Some(Vec::new());
    };
    let mut out = Vec::new();
    for field in parent.children_by_local("field") {
        let raw = field.attr("x")?;
        if raw == "-2" {
            continue;
        }
        out.push(raw.parse().ok()?);
    }
    Some(out)
}

fn value_specs(root: &Element) -> Option<Vec<ValSpec>> {
    let parent = root.child_by_local("dataFields")?;
    let fields = parent.children_by_local("dataField");
    if fields.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for field in fields {
        out.push(value_spec(field)?);
    }
    Some(out)
}

fn value_spec(field: &Element) -> Option<ValSpec> {
    let idx = field.attr("fld")?.parse().ok()?;
    let name = field.attr("name").unwrap_or("").to_string();
    let kind = match field.attr("subtotal") {
        None => AggKind::Sum,
        Some(s) if s.eq_ignore_ascii_case("sum") => AggKind::Sum,
        Some(s) if s.eq_ignore_ascii_case("count") => AggKind::Count,
        Some(s) if s.eq_ignore_ascii_case("average") => AggKind::Average,
        Some(s) if s.eq_ignore_ascii_case("max") => AggKind::Max,
        Some(s) if s.eq_ignore_ascii_case("min") => AggKind::Min,
        Some(_) => return None,
    };
    let percent = match field.attr("showDataAs") {
        None => false,
        Some(s) if s.eq_ignore_ascii_case("normal") => false,
        Some(s) if s.eq_ignore_ascii_case("percentOfTotal") => true,
        Some(_) => return None,
    };
    Some(ValSpec {
        name,
        idx,
        kind,
        percent,
    })
}

fn page_picks(root: &Element, cache: &CacheInfo) -> Option<Vec<PagePick>> {
    let Some(parent) = root.child_by_local("pageFields") else {
        return Some(Vec::new());
    };
    let mut out = Vec::new();
    for field in parent.children_by_local("pageField") {
        let idx = field.attr("fld")?.parse::<usize>().ok()?;
        let name = cache.fields.get(idx)?.clone();
        let item = match field.attr("item") {
            None => None,
            Some(s) => Some(s.parse().ok()?),
        };
        out.push(PagePick { idx, item, name });
    }
    Some(out)
}

fn filter_records(
    mut records: Vec<Vec<CacheVal>>,
    cache: &CacheInfo,
    pages: &[PagePick],
) -> Vec<Vec<CacheVal>> {
    for page in pages {
        let Some(item) = page.item else {
            continue;
        };
        let Some(want) = cache.items.get(page.idx).and_then(|list| list.get(item)) else {
            continue;
        };
        records.retain(|rec| rec.get(page.idx).is_some_and(|v| cache_label(v) == *want));
    }
    records
}

fn page_labels(pages: &[PagePick], cache: &CacheInfo) -> Vec<(String, String)> {
    pages
        .iter()
        .map(|page| {
            let text = page
                .item
                .and_then(|item| {
                    cache
                        .items
                        .get(page.idx)
                        .and_then(|list| list.get(item))
                        .cloned()
                })
                .unwrap_or_else(|| "(全部)".to_string());
            (page.name.clone(), text)
        })
        .collect()
}

fn row_groups(records: &[Vec<CacheVal>], row_idxs: &[usize], specs: &[ValSpec]) -> Vec<RowGroup> {
    let mut groups = Vec::new();
    for rec in records {
        if row_idxs.iter().any(|i| *i >= rec.len()) || specs.iter().any(|s| s.idx >= rec.len()) {
            continue;
        }
        let key: Vec<String> = row_idxs.iter().map(|i| cache_label(&rec[*i])).collect();
        let pos = if let Some(pos) = groups.iter().position(|g: &RowGroup| g.key == key) {
            pos
        } else {
            groups.push(RowGroup {
                key,
                measures: vec![Measure::new(); specs.len()],
            });
            groups.len() - 1
        };
        for (measure, spec) in groups[pos].measures.iter_mut().zip(specs) {
            measure.add(&rec[spec.idx]);
        }
    }
    groups
}

fn cross_body(
    records: &[Vec<CacheVal>],
    row_idx: usize,
    col_idx: usize,
    fields: &[String],
    spec: ValSpec,
) -> CrossBody {
    let mut row_labels = Vec::new();
    let mut col_labels = Vec::new();
    let mut cells: Vec<CrossSlot> = Vec::new();
    for rec in records {
        if row_idx >= rec.len() || col_idx >= rec.len() || spec.idx >= rec.len() {
            continue;
        }
        let row = cache_label(&rec[row_idx]);
        let col = cache_label(&rec[col_idx]);
        if !row_labels.iter().any(|s| s == &row) {
            row_labels.push(row.clone());
        }
        if !col_labels.iter().any(|s| s == &col) {
            col_labels.push(col.clone());
        }
        if let Some(slot) = cells.iter_mut().find(|c| c.row == row && c.col == col) {
            slot.measure.add(&rec[spec.idx]);
        } else {
            let mut measure = Measure::new();
            measure.add(&rec[spec.idx]);
            cells.push(CrossSlot { row, col, measure });
        }
    }
    CrossBody {
        value_name: spec.name.clone(),
        row_name: fields.get(row_idx).cloned().unwrap_or_default(),
        col_name: fields.get(col_idx).cloned().unwrap_or_default(),
        row_labels,
        col_labels,
        cells,
        spec,
    }
}

fn col_tree_body(
    records: &[Vec<CacheVal>],
    row_idx: usize,
    col_idxs: &[usize],
    fields: &[String],
    spec: ValSpec,
) -> ColTreeBody {
    let mut hits: Vec<ColHit> = Vec::new();
    let mut row_labels = Vec::new();
    let mut keys = Vec::new();
    for rec in records {
        if row_idx >= rec.len() || col_idxs.iter().any(|i| *i >= rec.len()) || spec.idx >= rec.len()
        {
            continue;
        }
        let row = cache_label(&rec[row_idx]);
        let path: Vec<String> = col_idxs.iter().map(|i| cache_label(&rec[*i])).collect();
        if !row_labels.iter().any(|s| s == &row) {
            row_labels.push(row.clone());
        }
        if !keys.iter().any(|k| k == &path) {
            keys.push(path.clone());
        }
        if let Some(hit) = hits.iter_mut().find(|h| h.row == row && h.path == path) {
            hit.measure.add(&rec[spec.idx]);
        } else {
            let mut measure = Measure::new();
            measure.add(&rec[spec.idx]);
            hits.push(ColHit { row, path, measure });
        }
    }
    let mut columns = layout_cols(&keys, 0, col_idxs.len(), &[]);
    columns.push(ColSlot::Grand);
    ColTreeBody {
        row_name: fields.get(row_idx).cloned().unwrap_or_default(),
        col_names: col_idxs
            .iter()
            .map(|i| fields.get(*i).cloned().unwrap_or_default())
            .collect(),
        value_name: spec.name.clone(),
        columns,
        row_labels,
        hits,
        spec,
    }
}

fn layout_cols(
    keys: &[Vec<String>],
    level: usize,
    depth: usize,
    prefix: &[String],
) -> Vec<ColSlot> {
    if depth == 0 || level >= depth {
        return Vec::new();
    }
    let mut order = Vec::new();
    let mut buckets: Vec<Vec<Vec<String>>> = Vec::new();
    for key in keys {
        if level >= key.len() {
            continue;
        }
        let label = &key[level];
        if let Some(i) = order.iter().position(|s: &String| s == label) {
            buckets[i].push(key.clone());
        } else {
            order.push(label.clone());
            buckets.push(vec![key.clone()]);
        }
    }
    let mut out = Vec::new();
    for (label, bucket) in order.into_iter().zip(buckets) {
        let mut path = prefix.to_vec();
        path.push(label);
        if level + 1 == depth {
            out.push(ColSlot::Leaf { path });
        } else {
            out.extend(layout_cols(&bucket, level + 1, depth, &path));
            out.push(ColSlot::Subtotal { path });
        }
    }
    out
}

fn row_col_body(
    records: &[Vec<CacheVal>],
    row_idxs: &[usize],
    col_idx: usize,
    fields: &[String],
    spec: ValSpec,
) -> RowColBody {
    let mut hits: Vec<RowColHit> = Vec::new();
    let mut row_keys = Vec::new();
    let mut col_labels = Vec::new();
    for rec in records {
        if row_idxs.iter().any(|i| *i >= rec.len()) || col_idx >= rec.len() || spec.idx >= rec.len()
        {
            continue;
        }
        let key: Vec<String> = row_idxs.iter().map(|i| cache_label(&rec[*i])).collect();
        let col = cache_label(&rec[col_idx]);
        if !row_keys.iter().any(|item: &Vec<String>| item == &key) {
            row_keys.push(key.clone());
        }
        if !col_labels.iter().any(|s| s == &col) {
            col_labels.push(col.clone());
        }
        if let Some(hit) = hits.iter_mut().find(|h| h.key == key && h.col == col) {
            hit.measure.add(&rec[spec.idx]);
        } else {
            let mut measure = Measure::new();
            measure.add(&rec[spec.idx]);
            hits.push(RowColHit { key, col, measure });
        }
    }
    RowColBody {
        field_names: row_idxs
            .iter()
            .map(|i| fields.get(*i).cloned().unwrap_or_default())
            .collect(),
        col_name: fields.get(col_idx).cloned().unwrap_or_default(),
        value_name: spec.name.clone(),
        col_labels,
        row_keys: grouped_keys(&row_keys),
        hits,
        spec,
    }
}

fn grouped_keys(keys: &[Vec<String>]) -> Vec<Vec<String>> {
    if keys.is_empty() {
        return Vec::new();
    }
    fn walk(keys: &[Vec<String>], level: usize, depth: usize) -> Vec<Vec<String>> {
        if level + 1 >= depth {
            let mut out = Vec::new();
            for key in keys {
                if !out.iter().any(|item: &Vec<String>| item == key) {
                    out.push(key.clone());
                }
            }
            return out;
        }
        let mut order = Vec::new();
        let mut buckets: Vec<Vec<Vec<String>>> = Vec::new();
        for key in keys {
            if level >= key.len() {
                continue;
            }
            let label = &key[level];
            if let Some(i) = order.iter().position(|s: &String| s == label) {
                buckets[i].push(key.clone());
            } else {
                order.push(label.clone());
                buckets.push(vec![key.clone()]);
            }
        }
        let mut out = Vec::new();
        for bucket in buckets {
            out.extend(walk(&bucket, level + 1, depth));
        }
        out
    }
    walk(keys, 0, keys[0].len())
}

fn multi_value_body(
    records: &[Vec<CacheVal>],
    row_idx: usize,
    col_idx: usize,
    fields: &[String],
    caption: String,
    specs: Vec<ValSpec>,
) -> MultiValueBody {
    let mut hits: Vec<ValueHit> = Vec::new();
    let mut row_labels = Vec::new();
    let mut col_labels = Vec::new();
    for rec in records {
        if row_idx >= rec.len() || col_idx >= rec.len() || specs.iter().any(|s| s.idx >= rec.len())
        {
            continue;
        }
        let row = cache_label(&rec[row_idx]);
        let col = cache_label(&rec[col_idx]);
        if !row_labels.iter().any(|s| s == &row) {
            row_labels.push(row.clone());
        }
        if !col_labels.iter().any(|s| s == &col) {
            col_labels.push(col.clone());
        }
        if let Some(hit) = hits.iter_mut().find(|h| h.row == row && h.col == col) {
            for (measure, spec) in hit.measures.iter_mut().zip(&specs) {
                measure.add(&rec[spec.idx]);
            }
        } else {
            let mut measures = Vec::new();
            for spec in &specs {
                let mut measure = Measure::new();
                measure.add(&rec[spec.idx]);
                measures.push(measure);
            }
            hits.push(ValueHit { row, col, measures });
        }
    }
    MultiValueBody {
        row_name: fields.get(row_idx).cloned().unwrap_or_default(),
        col_name: fields.get(col_idx).cloned().unwrap_or_default(),
        caption,
        value_names: specs.iter().map(|s| s.name.clone()).collect(),
        row_labels,
        col_labels,
        hits,
        specs,
    }
}

fn row_view(field_names: &[String], groups: &[RowGroup], specs: &[ValSpec]) -> RowSumView {
    let grands = merged_measures(groups, 0, groups.len(), specs.len());
    let grand = specs
        .iter()
        .enumerate()
        .map(|(i, spec)| shown(&grands[i], &grands[i], spec))
        .collect();
    let rows = groups
        .iter()
        .map(|group| {
            let nums = specs
                .iter()
                .enumerate()
                .map(|(i, spec)| shown(&group.measures[i], &grands[i], spec))
                .collect();
            (group.key[0].clone(), nums)
        })
        .collect();
    RowSumView {
        row_name: field_names.first().cloned().unwrap_or_default(),
        value_names: specs.iter().map(|s| s.name.clone()).collect(),
        groups: rows,
        grand,
    }
}

fn merged_measures(groups: &[RowGroup], start: usize, end: usize, n: usize) -> Vec<Measure> {
    let mut acc = vec![Measure::new(); n];
    for group in &groups[start..end] {
        for (slot, measure) in acc.iter_mut().zip(&group.measures) {
            slot.merge(measure);
        }
    }
    acc
}

fn shown(measure: &Measure, grand: &Measure, spec: &ValSpec) -> f64 {
    let Some(raw) = measure.finish(spec.kind) else {
        return 0.0;
    };
    if !spec.percent {
        return raw;
    }
    match grand.finish(spec.kind) {
        Some(base) if base.abs() > 1e-12 => raw / base,
        _ => 0.0,
    }
}

fn put_measure(
    host: &mut Element,
    cell: CellRef,
    measure: &Measure,
    grand: &Measure,
    spec: &ValSpec,
) -> bool {
    if !measure.seen {
        return false;
    }
    put_number(host, cell, shown(measure, grand, spec))
}

fn paint_display(host: &mut Element, plan: &DisplayPlan) -> bool {
    let mut changed = paint_pages(host, plan.origin, &plan.pages);
    changed |= match &plan.body {
        DisplayBody::Rows(body) => paint_row_body(host, plan.origin, body),
        DisplayBody::Cross(body) => paint_cross(host, plan.origin, body),
        DisplayBody::ColTree(body) => paint_col_tree(host, plan.origin, body),
        DisplayBody::RowCol(body) => paint_row_col(host, plan.origin, body),
        DisplayBody::MultiValue(body) => paint_multi_value(host, plan.origin, body),
    };
    changed
}

fn paint_pages(host: &mut Element, origin: CellRef, pages: &[(String, String)]) -> bool {
    let n = pages.len() as u32;
    let Some(top) = origin.row.checked_sub(n + 1) else {
        return false;
    };
    if n == 0 || top < 1 {
        return false;
    }
    let mut changed = false;
    for (i, (name, label)) in pages.iter().enumerate() {
        let row = top + i as u32;
        changed |= put_text(
            host,
            CellRef {
                col: origin.col,
                row,
            },
            name,
        );
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + 1,
                row,
            },
            label,
        );
    }
    changed
}

fn paint_row_body(host: &mut Element, origin: CellRef, body: &RowBody) -> bool {
    let n = body.field_names.len() as u32;
    let mut changed = false;
    for (c, name) in body.field_names.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + c as u32,
                row: origin.row,
            },
            name,
        );
    }
    for (c, spec) in body.specs.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + n + c as u32,
                row: origin.row,
            },
            &spec.name,
        );
    }
    let grands = merged_measures(&body.groups, 0, body.groups.len(), body.specs.len());
    let (total_row, body_changed) = {
        let mut cursor = LevelPaint {
            host,
            origin,
            body,
            grands: &grands,
            changed: false,
            row: origin.row + 1,
        };
        if !body.field_names.is_empty() {
            cursor.paint(0, 0, body.groups.len());
        }
        (cursor.row, cursor.changed)
    };
    changed |= body_changed;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (i, spec) in body.specs.iter().enumerate() {
        changed |= put_number(
            host,
            CellRef {
                col: origin.col + n + i as u32,
                row: total_row,
            },
            shown(&grands[i], &grands[i], spec),
        );
    }
    changed
}

struct LevelPaint<'a> {
    host: &'a mut Element,
    origin: CellRef,
    body: &'a RowBody,
    grands: &'a [Measure],
    changed: bool,
    row: u32,
}

impl LevelPaint<'_> {
    fn paint(&mut self, level: usize, start: usize, end: usize) {
        let n = self.body.field_names.len();
        if n == 0 {
            return;
        }
        if level + 1 == n {
            for i in start..end {
                self.leaf(i);
            }
            return;
        }
        let mut s = start;
        while s < end {
            let label = self.body.groups[s].key[level].clone();
            let mut e = s + 1;
            while e < end && self.body.groups[e].key[level] == label {
                e += 1;
            }
            self.paint(level + 1, s, e);
            self.subtotal(level, &label, s, e);
            s = e;
        }
    }

    fn leaf(&mut self, i: usize) {
        let n = self.body.field_names.len();
        for c in 0..n {
            let show = c + 1 == n
                || i == 0
                || self.body.groups[i].key[..=c] != self.body.groups[i - 1].key[..=c];
            if !show {
                continue;
            }
            let text = self.body.groups[i].key[c].clone();
            self.changed |= put_text(
                self.host,
                CellRef {
                    col: self.origin.col + c as u32,
                    row: self.row,
                },
                &text,
            );
        }
        let measures = self.body.groups[i].measures.clone();
        self.write_values(&measures);
        self.row += 1;
    }

    fn subtotal(&mut self, level: usize, label: &str, start: usize, end: usize) {
        self.changed |= put_text(
            self.host,
            CellRef {
                col: self.origin.col + level as u32,
                row: self.row,
            },
            &format!("{label} 汇总"),
        );
        let merged = merged_measures(&self.body.groups, start, end, self.body.specs.len());
        self.write_values(&merged);
        self.row += 1;
    }

    fn write_values(&mut self, measures: &[Measure]) {
        let n = self.body.field_names.len() as u32;
        for (i, spec) in self.body.specs.iter().enumerate() {
            self.changed |= put_number(
                self.host,
                CellRef {
                    col: self.origin.col + n + i as u32,
                    row: self.row,
                },
                shown(&measures[i], &self.grands[i], spec),
            );
        }
    }
}

fn paint_cross(host: &mut Element, origin: CellRef, body: &CrossBody) -> bool {
    let mut changed = put_text(host, origin, &body.value_name);
    changed |= put_text(
        host,
        CellRef {
            col: origin.col + 1,
            row: origin.row,
        },
        &body.col_name,
    );
    let head = origin.row + 1;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: head,
        },
        &body.row_name,
    );
    for (i, label) in body.col_labels.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + 1 + i as u32,
                row: head,
            },
            label,
        );
    }
    let grand_col = origin.col + 1 + body.col_labels.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: grand_col,
            row: head,
        },
        "总计",
    );
    let grand = grand_measure(&body.cells);
    for (r, row_label) in body.row_labels.iter().enumerate() {
        let row = origin.row + 2 + r as u32;
        changed |= put_text(
            host,
            CellRef {
                col: origin.col,
                row,
            },
            row_label,
        );
        let mut row_acc = Measure::new();
        for (c, col_label) in body.col_labels.iter().enumerate() {
            let Some(slot) = body
                .cells
                .iter()
                .find(|s| s.row == *row_label && s.col == *col_label)
            else {
                continue;
            };
            row_acc.merge(&slot.measure);
            changed |= put_number(
                host,
                CellRef {
                    col: origin.col + 1 + c as u32,
                    row,
                },
                shown(&slot.measure, &grand, &body.spec),
            );
        }
        changed |= put_number(
            host,
            CellRef {
                col: grand_col,
                row,
            },
            shown(&row_acc, &grand, &body.spec),
        );
    }
    let total_row = origin.row + 2 + body.row_labels.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (c, col_label) in body.col_labels.iter().enumerate() {
        let mut acc = Measure::new();
        for slot in body.cells.iter().filter(|s| s.col == *col_label) {
            acc.merge(&slot.measure);
        }
        changed |= put_number(
            host,
            CellRef {
                col: origin.col + 1 + c as u32,
                row: total_row,
            },
            shown(&acc, &grand, &body.spec),
        );
    }
    changed |= put_number(
        host,
        CellRef {
            col: grand_col,
            row: total_row,
        },
        shown(&grand, &grand, &body.spec),
    );
    changed
}

fn grand_measure(cells: &[CrossSlot]) -> Measure {
    let mut acc = Measure::new();
    for slot in cells {
        acc.merge(&slot.measure);
    }
    acc
}

fn paint_col_tree(host: &mut Element, origin: CellRef, body: &ColTreeBody) -> bool {
    let depth = body.col_names.len();
    let mut changed = put_text(host, origin, &body.value_name);
    for (i, name) in body.col_names.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + 1 + i as u32,
                row: origin.row,
            },
            name,
        );
    }
    for level in 0..depth {
        let row = origin.row + 1 + level as u32;
        if level + 1 == depth {
            changed |= put_text(
                host,
                CellRef {
                    col: origin.col,
                    row,
                },
                &body.row_name,
            );
        }
        changed |= paint_col_header(host, origin, row, level, depth, &body.columns);
    }
    let grand = merge_col_hits(&body.hits, None, &ColSlot::Grand);
    let data = origin.row + depth as u32 + 1;
    for (r, label) in body.row_labels.iter().enumerate() {
        let row = data + r as u32;
        changed |= put_text(
            host,
            CellRef {
                col: origin.col,
                row,
            },
            label,
        );
        for (c, slot) in body.columns.iter().enumerate() {
            let measure = merge_col_hits(&body.hits, Some(label), slot);
            changed |= put_measure(
                host,
                CellRef {
                    col: origin.col + 1 + c as u32,
                    row,
                },
                &measure,
                &grand,
                &body.spec,
            );
        }
    }
    let total_row = data + body.row_labels.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (c, slot) in body.columns.iter().enumerate() {
        let measure = merge_col_hits(&body.hits, None, slot);
        changed |= put_measure(
            host,
            CellRef {
                col: origin.col + 1 + c as u32,
                row: total_row,
            },
            &measure,
            &grand,
            &body.spec,
        );
    }
    changed
}

fn paint_col_header(
    host: &mut Element,
    origin: CellRef,
    row: u32,
    level: usize,
    depth: usize,
    columns: &[ColSlot],
) -> bool {
    let mut changed = false;
    let mut seen: Option<Vec<String>> = None;
    for (c, slot) in columns.iter().enumerate() {
        let text = match slot {
            ColSlot::Grand if level == 0 => Some("总计".to_string()),
            ColSlot::Subtotal { path } if path.len() == level + 1 => {
                seen = None;
                path.last().map(|label| format!("{label} 汇总"))
            }
            ColSlot::Leaf { path } if level + 1 == depth => path.get(level).cloned(),
            ColSlot::Leaf { path } => {
                let prefix = path.get(..=level).map(|item| item.to_vec());
                if prefix.is_some() && seen.as_ref() != prefix.as_ref() {
                    let label = path.get(level).cloned();
                    seen = prefix;
                    label
                } else {
                    None
                }
            }
            ColSlot::Grand | ColSlot::Subtotal { .. } => None,
        };
        let Some(text) = text else {
            continue;
        };
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + 1 + c as u32,
                row,
            },
            &text,
        );
    }
    changed
}

fn merge_col_hits(hits: &[ColHit], row: Option<&str>, slot: &ColSlot) -> Measure {
    let mut acc = Measure::new();
    for hit in hits {
        if row.is_some_and(|name| hit.row != name) || !slot_covers(slot, &hit.path) {
            continue;
        }
        acc.merge(&hit.measure);
    }
    acc
}

fn slot_covers(slot: &ColSlot, path: &[String]) -> bool {
    match slot {
        ColSlot::Leaf { path: leaf } => path == leaf.as_slice(),
        ColSlot::Subtotal { path: prefix } => path.starts_with(prefix.as_slice()),
        ColSlot::Grand => true,
    }
}

fn paint_row_col(host: &mut Element, origin: CellRef, body: &RowColBody) -> bool {
    let n = body.field_names.len() as u32;
    let mut changed = put_text(host, origin, &body.value_name);
    changed |= put_text(
        host,
        CellRef {
            col: origin.col + n,
            row: origin.row,
        },
        &body.col_name,
    );
    let head = origin.row + 1;
    for (c, name) in body.field_names.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + c as u32,
                row: head,
            },
            name,
        );
    }
    for (i, label) in body.col_labels.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + n + i as u32,
                row: head,
            },
            label,
        );
    }
    let grand_col = origin.col + n + body.col_labels.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: grand_col,
            row: head,
        },
        "总计",
    );
    let grand = merge_row_hits(&body.hits, None, None);
    let (total_row, body_changed) = {
        let mut cursor = RowColPaint {
            host,
            origin,
            body,
            grand,
            changed: false,
            row: origin.row + 2,
        };
        cursor.paint(0, 0, body.row_keys.len());
        (cursor.row, cursor.changed)
    };
    changed |= body_changed;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (i, label) in body.col_labels.iter().enumerate() {
        let measure = merge_row_hits(&body.hits, None, Some(label));
        changed |= put_measure(
            host,
            CellRef {
                col: origin.col + n + i as u32,
                row: total_row,
            },
            &measure,
            &grand,
            &body.spec,
        );
    }
    changed |= put_measure(
        host,
        CellRef {
            col: grand_col,
            row: total_row,
        },
        &grand,
        &grand,
        &body.spec,
    );
    changed
}

struct RowColPaint<'a> {
    host: &'a mut Element,
    origin: CellRef,
    body: &'a RowColBody,
    grand: Measure,
    changed: bool,
    row: u32,
}

impl RowColPaint<'_> {
    fn paint(&mut self, level: usize, start: usize, end: usize) {
        let n = self.body.field_names.len();
        if n == 0 || start >= end {
            return;
        }
        if level + 1 == n {
            for i in start..end {
                self.leaf(i);
            }
            return;
        }
        let mut s = start;
        while s < end {
            let label = self.body.row_keys[s][level].clone();
            let mut e = s + 1;
            while e < end && self.body.row_keys[e][level] == label {
                e += 1;
            }
            self.paint(level + 1, s, e);
            let prefix = self.body.row_keys[s][..=level].to_vec();
            self.subtotal(level, &label, &prefix);
            s = e;
        }
    }

    fn leaf(&mut self, i: usize) {
        let n = self.body.field_names.len();
        for c in 0..n {
            let show = c + 1 == n
                || i == 0
                || self.body.row_keys[i][..=c] != self.body.row_keys[i - 1][..=c];
            if !show {
                continue;
            }
            let text = self.body.row_keys[i][c].clone();
            self.changed |= put_text(
                self.host,
                CellRef {
                    col: self.origin.col + c as u32,
                    row: self.row,
                },
                &text,
            );
        }
        let key = self.body.row_keys[i].clone();
        self.write_cols(Some(&key));
        self.row += 1;
    }

    fn subtotal(&mut self, level: usize, label: &str, prefix: &[String]) {
        self.changed |= put_text(
            self.host,
            CellRef {
                col: self.origin.col + level as u32,
                row: self.row,
            },
            &format!("{label} 汇总"),
        );
        self.write_cols(Some(prefix));
        self.row += 1;
    }

    fn write_cols(&mut self, prefix: Option<&[String]>) {
        let n = self.body.field_names.len() as u32;
        for i in 0..self.body.col_labels.len() {
            let label = self.body.col_labels[i].clone();
            let measure = merge_row_hits(&self.body.hits, prefix, Some(&label));
            self.changed |= put_measure(
                self.host,
                CellRef {
                    col: self.origin.col + n + i as u32,
                    row: self.row,
                },
                &measure,
                &self.grand,
                &self.body.spec,
            );
        }
        let total = merge_row_hits(&self.body.hits, prefix, None);
        self.changed |= put_measure(
            self.host,
            CellRef {
                col: self.origin.col + n + self.body.col_labels.len() as u32,
                row: self.row,
            },
            &total,
            &self.grand,
            &self.body.spec,
        );
    }
}

fn merge_row_hits(hits: &[RowColHit], prefix: Option<&[String]>, col: Option<&str>) -> Measure {
    let mut acc = Measure::new();
    for hit in hits {
        if prefix.is_some_and(|key| !hit.key.starts_with(key))
            || col.is_some_and(|name| hit.col != name)
        {
            continue;
        }
        acc.merge(&hit.measure);
    }
    acc
}

fn paint_multi_value(host: &mut Element, origin: CellRef, body: &MultiValueBody) -> bool {
    let n = body.specs.len() as u32;
    let mut changed = put_text(
        host,
        CellRef {
            col: origin.col + 1,
            row: origin.row,
        },
        &body.col_name,
    );
    changed |= put_text(
        host,
        CellRef {
            col: origin.col + 2,
            row: origin.row,
        },
        &body.caption,
    );
    let item_row = origin.row + 1;
    for (i, label) in body.col_labels.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: origin.col + 1 + (i as u32) * n,
                row: item_row,
            },
            label,
        );
    }
    let grand_base = origin.col + 1 + body.col_labels.len() as u32 * n;
    for (j, name) in body.value_names.iter().enumerate() {
        changed |= put_text(
            host,
            CellRef {
                col: grand_base + j as u32,
                row: item_row,
            },
            &format!("{name}汇总"),
        );
    }
    let name_row = origin.row + 2;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: name_row,
        },
        &body.row_name,
    );
    for i in 0..body.col_labels.len() {
        for (j, name) in body.value_names.iter().enumerate() {
            changed |= put_text(
                host,
                CellRef {
                    col: origin.col + 1 + (i as u32) * n + j as u32,
                    row: name_row,
                },
                name,
            );
        }
    }
    let data = origin.row + 3;
    for (r, label) in body.row_labels.iter().enumerate() {
        let row = data + r as u32;
        changed |= put_text(
            host,
            CellRef {
                col: origin.col,
                row,
            },
            label,
        );
        for (i, col) in body.col_labels.iter().enumerate() {
            for j in 0..body.specs.len() {
                let measure = value_measure(&body.hits, Some(label), Some(col), j);
                changed |= put_measure(
                    host,
                    CellRef {
                        col: origin.col + 1 + (i as u32) * n + j as u32,
                        row,
                    },
                    &measure,
                    &value_measure(&body.hits, None, None, j),
                    &body.specs[j],
                );
            }
        }
        for j in 0..body.specs.len() {
            let measure = value_measure(&body.hits, Some(label), None, j);
            changed |= put_measure(
                host,
                CellRef {
                    col: grand_base + j as u32,
                    row,
                },
                &measure,
                &value_measure(&body.hits, None, None, j),
                &body.specs[j],
            );
        }
    }
    let total_row = data + body.row_labels.len() as u32;
    changed |= put_text(
        host,
        CellRef {
            col: origin.col,
            row: total_row,
        },
        "总计",
    );
    for (i, col) in body.col_labels.iter().enumerate() {
        for j in 0..body.specs.len() {
            let measure = value_measure(&body.hits, None, Some(col), j);
            changed |= put_measure(
                host,
                CellRef {
                    col: origin.col + 1 + (i as u32) * n + j as u32,
                    row: total_row,
                },
                &measure,
                &value_measure(&body.hits, None, None, j),
                &body.specs[j],
            );
        }
    }
    for j in 0..body.specs.len() {
        let measure = value_measure(&body.hits, None, None, j);
        changed |= put_measure(
            host,
            CellRef {
                col: grand_base + j as u32,
                row: total_row,
            },
            &measure,
            &measure,
            &body.specs[j],
        );
    }
    changed
}

fn value_measure(hits: &[ValueHit], row: Option<&str>, col: Option<&str>, j: usize) -> Measure {
    let mut acc = Measure::new();
    for hit in hits {
        if row.is_some_and(|name| hit.row != name) || col.is_some_and(|name| hit.col != name) {
            continue;
        }
        if let Some(measure) = hit.measures.get(j) {
            acc.merge(measure);
        }
    }
    acc
}

fn cache_label(v: &CacheVal) -> String {
    match v {
        CacheVal::Text(s) | CacheVal::Number(s) => s.clone(),
        CacheVal::Bool(true) => "TRUE".into(),
        CacheVal::Bool(false) => "FALSE".into(),
        CacheVal::Empty => String::new(),
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
