//! 工作表表格：inspect 名单，apply 写 `/xl/tables/` 与 `tableParts`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{resolve_rel_target, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::a1::{self, CellRef};
use super::apply;
use super::cell;
use super::layout;
use super::picture;
use super::request::{DeleteSheetTableOp, InsertSheetTableOp, SetSheetTableOp};
use super::{REL_NS, SML_NS};

pub const REL_TABLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/table";
pub const CT_TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";

#[derive(Clone)]
struct TableMeta {
    sheet_part: String,
    /// 该工作表 `tableParts` 上从 1 计的序号。
    index: usize,
    xml_id: u32,
    name: String,
    display_name: String,
    start: CellRef,
    end: CellRef,
    part: String,
}

pub(crate) fn count(root: &Element) -> usize {
    root.child_by_local("tableParts")
        .map(|p| p.children_by_local("tablePart").len())
        .unwrap_or(0)
}

pub(crate) fn inspect(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
    sheet: &str,
) -> Result<Value, Error> {
    let Some(parent) = root.child_by_local("tableParts") else {
        return Ok(json!([]));
    };
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let mut out = Vec::new();
    for (i, el) in parent
        .children_by_local("tablePart")
        .into_iter()
        .enumerate()
    {
        let rid = el.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "tablePart".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Part(sheet_part.to_string()),
                rid: rid.to_string(),
            })
        })?;
        let part = resolve_rel_target(sheet_part, &rel.target)?;
        let bytes = pkg.get(&part)?;
        let doc = xml::parse(bytes)?;
        out.push(inspect_one(&doc.root, sheet, i + 1));
    }
    Ok(Value::Array(out))
}

fn inspect_one(root: &Element, sheet: &str, n: usize) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/table:{n}")));
    let name = root
        .attr("displayName")
        .or_else(|| root.attr("name"))
        .unwrap_or("");
    obj.insert("name".into(), json!(name));
    let raw = root.attr("ref").unwrap_or("");
    let refer = match a1::parse_range(raw) {
        Ok((s, e)) => a1::format_range(s, e),
        Err(_) => raw.to_string(),
    };
    obj.insert("ref".into(), json!(refer));
    let cols = root
        .child_by_local("tableColumns")
        .map(|c| {
            c.children_by_local("tableColumn")
                .into_iter()
                .map(|col| col.attr("name").unwrap_or("").to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    obj.insert("columns".into(), json!(cols));
    Value::Object(obj)
}

pub(crate) fn apply_insert(
    pkg: &mut Package,
    wb_doc: &Document,
    wb_part: &str,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut [(String, Document)],
    sheet_part: &str,
    op: &InsertSheetTableOp,
    op_index: usize,
) -> Result<(), Error> {
    let (start, end) = op.range;
    check_table_span(start, end, op_index)?;
    let ncols = (end.col - start.col + 1) as usize;
    let taken = collect_workbook_tables(pkg, wb_doc, sheet_index, dirty_sheets)?;
    if ranges_overlap_any(&taken, sheet_part, start, end) {
        return Err(bad_field("ref", op_index));
    }
    let display = match &op.name {
        Some(n) => {
            validate_table_name(n, op_index, "name")?;
            if name_taken(&taken, n, None) {
                return Err(bad_field("name", op_index));
            }
            n.clone()
        }
        None => next_table_name(&taken),
    };
    let sst = load_sst(pkg, wb_part)?;
    let columns = {
        let sheet = apply::dirty_of(dirty_sheets, sheet_part);
        match &op.columns {
            Some(cols) => {
                check_columns(cols, ncols, op_index)?;
                cols.clone()
            }
            None => read_header_names(&sheet.root, start, end, &sst),
        }
    };
    {
        let sheet = apply::dirty_of(dirty_sheets, sheet_part);
        for (i, name) in columns.iter().enumerate() {
            let cell = CellRef {
                col: start.col + i as u32,
                row: start.row,
            };
            write_header(&mut sheet.root, cell, name);
        }
        picture::ensure_xmlns(&mut sheet.root, "r", REL_NS);
    }
    let xml_id = taken
        .iter()
        .filter(|t| !t.sheet_part.is_empty())
        .map(|t| t.xml_id)
        .max()
        .unwrap_or(0)
        + 1;
    let table_part = next_table_part(pkg);
    let refer = a1::format_range(start, end);
    let table_xml = build_table_xml(xml_id, &display, &refer, &columns);
    pkg.set(&table_part, table_xml.to_bytes()?)?;
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let rid = rels.next_rid();
    rels.push(Relationship {
        id: rid.clone(),
        rel_type: REL_TABLE.into(),
        target: picture::rel_target_from(sheet_part, &table_part),
        target_mode: None,
        extra: Vec::new(),
    });
    rels.write_to(pkg, sheet_part)?;
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&table_part, CT_TABLE)?;
    cts.write_to(pkg)?;
    let sheet = apply::dirty_of(dirty_sheets, sheet_part);
    append_table_part(&mut sheet.root, &rid);
    Ok(())
}

pub(crate) fn apply_set(
    pkg: &mut Package,
    wb_doc: &Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &mut [(String, Document)],
    sheet_part: &str,
    op: &SetSheetTableOp,
    op_index: usize,
) -> Result<(), Error> {
    let taken = collect_workbook_tables(pkg, wb_doc, sheet_index, dirty_sheets)?;
    let current = taken
        .iter()
        .find(|t| t.sheet_part == sheet_part && t.index == op.index)
        .cloned()
        .ok_or_else(|| not_found(&op.sheet, op.index, op_index))?;
    let mut start = current.start;
    let mut end = current.end;
    if let Some((s, e)) = op.range {
        check_table_span(s, e, op_index)?;
        if ranges_overlap_except(&taken, sheet_part, op.index, s, e) {
            return Err(bad_field("ref", op_index));
        }
        start = s;
        end = e;
    }
    let ncols = (end.col - start.col + 1) as usize;
    if let Some(name) = &op.name {
        validate_table_name(name, op_index, "name")?;
        if name_taken(&taken, name, Some((sheet_part, op.index))) {
            return Err(bad_field("name", op_index));
        }
    }
    if let Some(cols) = &op.columns {
        check_columns(cols, ncols, op_index)?;
    }
    let mut table_doc = xml::parse(pkg.get(&current.part)?)?;
    if op.range.is_some() {
        let refer = a1::format_range(start, end);
        table_doc.root.set_attr("ref", refer.clone());
        if let Some(af) = table_doc.root.child_by_local_mut("autoFilter") {
            af.set_attr("ref", refer);
        }
        resize_columns(&mut table_doc.root, ncols, op.columns.as_deref());
    } else if let Some(cols) = &op.columns {
        rename_columns(&mut table_doc.root, cols);
    }
    if let Some(name) = &op.name {
        table_doc.root.set_attr("name", name.clone());
        table_doc.root.set_attr("displayName", name.clone());
    }
    let header_names = column_names_of(&table_doc.root);
    pkg.set(&current.part, table_doc.to_bytes()?)?;
    if op.columns.is_some() || op.range.is_some() {
        let sheet = apply::dirty_of(dirty_sheets, sheet_part);
        for (i, name) in header_names.iter().enumerate() {
            let cell = CellRef {
                col: start.col + i as u32,
                row: start.row,
            };
            write_header(&mut sheet.root, cell, name);
        }
    }
    Ok(())
}

pub(crate) fn apply_delete(
    pkg: &mut Package,
    dirty_sheets: &mut [(String, Document)],
    sheet_part: &str,
    op: &DeleteSheetTableOp,
    op_index: usize,
) -> Result<(), Error> {
    let rid = {
        let sheet = apply::dirty_of(dirty_sheets, sheet_part);
        let Some(parent) = sheet.root.child_by_local("tableParts") else {
            return Err(not_found(&op.sheet, op.index, op_index));
        };
        let Some(el) = super::list_pin::find_child(parent, "tablePart", "table", op.index) else {
            return Err(not_found(&op.sheet, op.index, op_index));
        };
        el.attr_local("id")
            .ok_or_else(|| {
                Error::new(ErrorKind::XmlMissing {
                    parent: "tablePart".into(),
                    child: "r:id".into(),
                })
            })?
            .to_string()
    };
    let mut rels = Relationships::from_package(pkg, sheet_part)?;
    let rel = rels.find_by_id(&rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(sheet_part.to_string()),
            rid: rid.clone(),
        })
    })?;
    let table_part = resolve_rel_target(sheet_part, &rel.target)?;
    rels.remove_by_id(&rid);
    rels.write_to(pkg, sheet_part)?;
    pkg.remove(&table_part);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(&table_part);
    cts.write_to(pkg)?;
    let sheet = apply::dirty_of(dirty_sheets, sheet_part);
    let left = {
        let parent = sheet.root.child_by_local_mut("tableParts").unwrap();
        let attr = crate::xml::pin_attr("table");
        let mut live = 0usize;
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "tablePart" => {
                live += 1;
                !crate::xml::pin_matches(e, &attr, op.index, live)
            }
            _ => true,
        });
        parent.children_by_local("tablePart").len()
    };
    if left == 0 {
        sheet.root.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "tableParts",
            _ => true,
        });
    } else if let Some(parent) = sheet.root.child_by_local_mut("tableParts") {
        parent.set_attr("count", left.to_string());
    }
    Ok(())
}

fn collect_workbook_tables(
    pkg: &Package,
    wb_doc: &Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
) -> Result<Vec<TableMeta>, Error> {
    let mut out = Vec::new();
    for (_, part, kind) in sheet_index {
        if *kind != "worksheet" {
            continue;
        }
        if let Some((_, doc)) = dirty_sheets.iter().find(|(p, _)| p == part) {
            append_tables_from(pkg, part, &doc.root, &mut out)?;
        } else {
            let bytes = pkg.get(part)?;
            let owned = xml::parse(bytes)?;
            append_tables_from(pkg, part, &owned.root, &mut out)?;
        }
    }
    if let Some(dns) = wb_doc.root.child_by_local("definedNames") {
        for dn in dns.children_by_local("definedName") {
            if let Some(n) = dn.attr("name") {
                out.push(TableMeta {
                    sheet_part: String::new(),
                    index: 0,
                    xml_id: 0,
                    name: n.to_string(),
                    display_name: n.to_string(),
                    start: CellRef { col: 1, row: 1 },
                    end: CellRef { col: 1, row: 1 },
                    part: String::new(),
                });
            }
        }
    }
    Ok(out)
}

/// 整本工作簿表格的 `name` / `displayName`，不含定义名。
pub(crate) fn table_taken_names(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
) -> Result<Vec<String>, Error> {
    let mut metas = Vec::new();
    for (_, part, kind) in sheet_index {
        if *kind != "worksheet" {
            continue;
        }
        if let Some((_, doc)) = dirty_sheets.iter().find(|(p, _)| p == part) {
            append_tables_from(pkg, part, &doc.root, &mut metas)?;
        } else {
            let bytes = pkg.get(part)?;
            let owned = xml::parse(bytes)?;
            append_tables_from(pkg, part, &owned.root, &mut metas)?;
        }
    }
    let mut out = Vec::new();
    for t in metas {
        out.push(t.name);
        out.push(t.display_name);
    }
    Ok(out)
}

fn append_tables_from(
    pkg: &Package,
    sheet_part: &str,
    root: &Element,
    out: &mut Vec<TableMeta>,
) -> Result<(), Error> {
    let Some(parent) = root.child_by_local("tableParts") else {
        return Ok(());
    };
    let rels = Relationships::from_package(pkg, sheet_part)?;
    for (i, el) in parent
        .children_by_local("tablePart")
        .into_iter()
        .enumerate()
    {
        let Some(rid) = el.attr_local("id") else {
            continue;
        };
        let Some(rel) = rels.find_by_id(rid) else {
            continue;
        };
        let Ok(part) = resolve_rel_target(sheet_part, &rel.target) else {
            continue;
        };
        let Ok(bytes) = pkg.get(&part) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        let xml_id = doc
            .root
            .attr("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let name = doc.root.attr("name").unwrap_or("").to_string();
        let display_name = doc.root.attr("displayName").unwrap_or(&name).to_string();
        let (start, end) = doc
            .root
            .attr("ref")
            .and_then(|r| a1::parse_range(r).ok())
            .unwrap_or((CellRef { col: 1, row: 1 }, CellRef { col: 1, row: 1 }));
        let index = el
            .attr(&crate::xml::pin_attr("table"))
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .unwrap_or(i + 1);
        out.push(TableMeta {
            sheet_part: sheet_part.to_string(),
            index,
            xml_id,
            name,
            display_name,
            start,
            end,
            part,
        });
    }
    Ok(())
}

fn ranges_overlap_any(taken: &[TableMeta], sheet_part: &str, start: CellRef, end: CellRef) -> bool {
    taken.iter().any(|t| {
        !t.sheet_part.is_empty()
            && t.sheet_part == sheet_part
            && ranges_overlap(t.start, t.end, start, end)
    })
}

fn ranges_overlap_except(
    taken: &[TableMeta],
    sheet_part: &str,
    index: usize,
    start: CellRef,
    end: CellRef,
) -> bool {
    taken.iter().any(|t| {
        t.sheet_part == sheet_part && t.index != index && ranges_overlap(t.start, t.end, start, end)
    })
}

fn ranges_overlap(a0: CellRef, a1: CellRef, b0: CellRef, b1: CellRef) -> bool {
    !(a1.col < b0.col || b1.col < a0.col || a1.row < b0.row || b1.row < a0.row)
}

pub(crate) fn range_overlaps_table(
    pkg: &Package,
    wb_doc: &Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    sheet_part: &str,
    start: CellRef,
    end: CellRef,
) -> Result<bool, Error> {
    let taken = collect_workbook_tables(pkg, wb_doc, sheet_index, dirty_sheets)?;
    Ok(ranges_overlap_any(&taken, sheet_part, start, end))
}

fn name_taken(taken: &[TableMeta], name: &str, skip: Option<(&str, usize)>) -> bool {
    taken.iter().any(|t| {
        if let Some((part, idx)) = skip {
            if t.sheet_part == part && t.index == idx {
                return false;
            }
        }
        t.name.eq_ignore_ascii_case(name) || t.display_name.eq_ignore_ascii_case(name)
    })
}

fn next_table_name(taken: &[TableMeta]) -> String {
    let mut n = 1u32;
    loop {
        let cand = format!("Table{n}");
        if !name_taken(taken, &cand, None) {
            return cand;
        }
        n += 1;
    }
}

fn check_table_span(start: CellRef, end: CellRef, op_index: usize) -> Result<(), Error> {
    if start.row == end.row {
        return Err(bad_field("ref", op_index));
    }
    Ok(())
}

fn validate_table_name(name: &str, op_index: usize, field: &str) -> Result<(), Error> {
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(Error::with_path(
            ErrorKind::InvalidSheetTable {
                field: field.into(),
            },
            format!("ops[{op_index}].{field}"),
        ));
    }
    Ok(())
}

fn check_columns(cols: &[String], ncols: usize, op_index: usize) -> Result<(), Error> {
    if cols.len() != ncols {
        return Err(bad_field("columns", op_index));
    }
    for (i, c) in cols.iter().enumerate() {
        if c.is_empty() {
            return Err(Error::with_path(
                ErrorKind::InvalidSheetTable {
                    field: "columns".into(),
                },
                format!("ops[{op_index}].columns[{i}]"),
            ));
        }
    }
    if has_dup_ci(cols) {
        return Err(bad_field("columns", op_index));
    }
    Ok(())
}

fn has_dup_ci(cols: &[String]) -> bool {
    let mut seen = Vec::new();
    for c in cols {
        let k = c.to_ascii_lowercase();
        if seen.iter().any(|s: &String| s == &k) {
            return true;
        }
        seen.push(k);
    }
    false
}

fn load_sst(pkg: &Package, wb_part: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    super::inspect::load_shared_strings(pkg, wb_part, &rels)
}

fn read_header_names(root: &Element, start: CellRef, end: CellRef, sst: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut used: Vec<String> = Vec::new();
    for col in start.col..=end.col {
        let cell = CellRef {
            col,
            row: start.row,
        };
        let raw = cell_text(root, cell, sst);
        let base = if raw.is_empty() {
            format!("Column{}", col - start.col + 1)
        } else {
            raw
        };
        names.push(unique_col_name(&base, &used));
        used.push(names.last().unwrap().to_ascii_lowercase());
    }
    names
}

fn unique_col_name(base: &str, used_lower: &[String]) -> String {
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

fn cell_text(root: &Element, cell: CellRef, sst: &[String]) -> String {
    let Some(sd) = root.child_by_local("sheetData") else {
        return String::new();
    };
    let want = a1::format_cell(cell.col, cell.row);
    for row in sd.children_by_local("row") {
        for c in row.children_by_local("c") {
            if c.attr("r") != Some(want.as_str()) {
                continue;
            }
            return match c.attr("t").unwrap_or("n") {
                "s" => c
                    .child_by_local("v")
                    .and_then(|v| v.text_content().parse::<usize>().ok())
                    .and_then(|i| sst.get(i).cloned())
                    .unwrap_or_default(),
                "inlineStr" => c
                    .child_by_local("is")
                    .map(|is| is.text_content())
                    .unwrap_or_default(),
                "str" | "e" | "b" | "n" | "d" => c
                    .child_by_local("v")
                    .map(|v| v.text_content())
                    .unwrap_or_default(),
                _ => c.text_content(),
            };
        }
    }
    String::new()
}

fn write_header(worksheet: &mut Element, cell: CellRef, text: &str) {
    let sheet_data = cell::ensure_sheet_data(worksheet);
    let c = cell::ensure_cell(sheet_data, cell);
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) | Node::Raw(_) => true,
    });
    c.set_attr("t", "inlineStr");
    let mut t = Element::new("t");
    if text.starts_with(' ') || text.ends_with(' ') {
        t.set_attr("xml:space", "preserve");
    }
    t.set_text(text.to_string());
    let mut is = Element::new("is");
    is.push_element(t);
    c.push_element(is);
    cell::expand_dimension(worksheet, cell);
}

fn build_table_xml(id: u32, name: &str, refer: &str, columns: &[String]) -> Document {
    let mut cols = Element::new("tableColumns").with_attr("count", columns.len().to_string());
    for (i, n) in columns.iter().enumerate() {
        cols.push_element(
            Element::new("tableColumn")
                .with_attr("id", (i + 1).to_string())
                .with_attr("name", n.clone()),
        );
    }
    let mut table = Element::new("table")
        .with_attr("xmlns", SML_NS)
        .with_attr("id", id.to_string())
        .with_attr("name", name)
        .with_attr("displayName", name)
        .with_attr("ref", refer)
        .with_attr("totalsRowShown", "0");
    table.push_element(Element::new("autoFilter").with_attr("ref", refer));
    table.push_element(cols);
    table.push_element(
        Element::new("tableStyleInfo")
            .with_attr("name", "TableStyleMedium2")
            .with_attr("showFirstColumn", "0")
            .with_attr("showLastColumn", "0")
            .with_attr("showRowStripes", "1")
            .with_attr("showColumnStripes", "0"),
    );
    Document::new(Some(DEFAULT_DECL.to_string()), table)
}

fn append_table_part(root: &mut Element, rid: &str) {
    // 新表接在后面，不记空编号。create 同一批还要用 table:1 点刚插入的表。
    let part = Element::new("tablePart").with_attr("r:id", rid);
    if let Some(parent) = root.child_by_local_mut("tableParts") {
        parent.push_element(part);
        let n = parent.children_by_local("tablePart").len();
        parent.set_attr("count", n.to_string());
        return;
    }
    let mut parent = Element::new("tableParts").with_attr("count", "1");
    parent.push_element(part);
    layout::insert_child_ordered(root, parent, "tableParts");
}

fn next_table_part(pkg: &Package) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("/xl/tables/table{n}.xml");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn column_names_of(root: &Element) -> Vec<String> {
    root.child_by_local("tableColumns")
        .map(|c| {
            c.children_by_local("tableColumn")
                .into_iter()
                .map(|col| col.attr("name").unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn rename_columns(root: &mut Element, names: &[String]) {
    let Some(cols) = root.child_by_local_mut("tableColumns") else {
        return;
    };
    let mut i = 0usize;
    for n in cols.children.iter_mut() {
        let Node::Element(el) = n else { continue };
        if el.local_name() != "tableColumn" {
            continue;
        }
        if let Some(name) = names.get(i) {
            el.set_attr("name", name.clone());
        }
        i += 1;
    }
}

fn resize_columns(root: &mut Element, ncols: usize, given: Option<&[String]>) {
    let existing = column_names_of(root);
    let mut names = Vec::new();
    for i in 0..ncols {
        if let Some(g) = given.and_then(|g| g.get(i)) {
            names.push(g.clone());
        } else if let Some(e) = existing.get(i) {
            names.push(e.clone());
        } else {
            names.push(format!("Column{}", i + 1));
        }
    }
    let max_id = root
        .child_by_local("tableColumns")
        .map(|c| {
            c.children_by_local("tableColumn")
                .into_iter()
                .filter_map(|el| el.attr("id")?.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let Some(cols) = root.child_by_local_mut("tableColumns") else {
        let mut parent = Element::new("tableColumns").with_attr("count", ncols.to_string());
        for (i, n) in names.iter().enumerate() {
            parent.push_element(
                Element::new("tableColumn")
                    .with_attr("id", (i as u32 + 1).to_string())
                    .with_attr("name", n.clone()),
            );
        }
        root.push_element(parent);
        return;
    };
    let mut kept = 0usize;
    cols.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "tableColumn" => {
            kept += 1;
            kept <= ncols
        }
        _ => true,
    });
    let mut next_id = max_id;
    while column_names_of_mut_count(cols) < ncols {
        let i = column_names_of_mut_count(cols);
        next_id += 1;
        cols.push_element(
            Element::new("tableColumn")
                .with_attr("id", next_id.to_string())
                .with_attr("name", names[i].clone()),
        );
    }
    rename_columns_in(cols, &names);
    cols.set_attr("count", ncols.to_string());
}

fn column_names_of_mut_count(cols: &Element) -> usize {
    cols.children_by_local("tableColumn").len()
}

fn rename_columns_in(cols: &mut Element, names: &[String]) {
    let mut i = 0usize;
    for n in cols.children.iter_mut() {
        let Node::Element(el) = n else { continue };
        if el.local_name() != "tableColumn" {
            continue;
        }
        if let Some(name) = names.get(i) {
            el.set_attr("name", name.clone());
        }
        i += 1;
    }
}

fn bad_field(field: &str, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidSheetTable {
            field: field.into(),
        },
        format!("ops[{op_index}].{field}"),
    )
}

fn not_found(sheet: &str, index: usize, op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::SheetTableNotFound {
            id: format!("sheet:{sheet}/table:{index}"),
        },
        format!("ops[{op_index}].id"),
    )
}
