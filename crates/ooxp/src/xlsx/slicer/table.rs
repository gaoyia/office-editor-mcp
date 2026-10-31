use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{
    office_document_part, resolve_id, resolve_rel_target, ContentTypes, Package, Relationships,
};
use crate::xml::{self, Document, Element, Node, DEFAULT_DECL};

use super::super::a1;
use super::super::picture::{self};
use super::common::*;
use super::InsertSlicerOp;

pub(crate) fn insert_table(
    pkg: &mut Package,
    wb_doc: &mut Document,
    wb_part: &str,
    sheet_doc: &mut Document,
    sheet_part: &str,
    op: &InsertSlicerOp,
    index: usize,
    op_index: usize,
) -> Result<(), Error> {
    let (table_id, display, col_id) = table_column(
        pkg,
        &sheet_doc.root,
        sheet_part,
        &op.sheet,
        index,
        &op.field,
        op_index,
    )?;
    let cx = op.cx.unwrap_or(DEFAULT_CX);
    let cy = op.cy.unwrap_or(DEFAULT_CY);
    let (to, to_off_x, to_off_y) = dest_to(&sheet_doc.root, op, cx, cy);
    let cache_name = unique_cache_name(pkg, wb_doc, &format!("Slicer_{}", op.field));
    let view_name = unique_view_name(pkg, &op.field);
    let cache_part = next_part(pkg, "/xl/slicerCaches/slicerCache", "xml");
    pkg.set(
        &cache_part,
        table_cache_xml(&cache_name, &op.field, table_id, col_id),
    )?;
    let cache_rid = add_rel(pkg, wb_part, REL_SLICER_CACHE, &cache_part)?;
    push_wb_cache(wb_doc, true, &cache_rid);
    push_cache_defined_name(wb_doc, &cache_name);
    let (slicer_part, slicer_rid, is_new) = ensure_slicers_part(pkg, sheet_doc, sheet_part)?;
    append_slicer_view(
        pkg,
        &slicer_part,
        &view_name,
        &cache_name,
        &op.field,
        &op.view,
    )?;
    if is_new {
        push_sheet_list(&mut sheet_doc.root, &slicer_rid);
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&cache_part, CT_SLICER_CACHE)?;
    cts.set_override(&slicer_part, CT_SLICER)?;
    cts.write_to(pkg)?;
    write_drawing(
        pkg,
        sheet_doc,
        sheet_part,
        op,
        &view_name,
        SlicerFrameKind::Table,
        cx,
        cy,
        to,
        to_off_x,
        to_off_y,
    )?;
    let _ = display;
    Ok(())
}

pub(crate) fn table_id_of(root: &Element) -> Option<u32> {
    let mut id = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "tableSlicerCache" && id.is_none() {
            id = e.attr("tableId").and_then(|s| s.parse().ok());
        }
    });
    id
}

pub(crate) fn table_display_by_id(
    pkg: &Package,
    wb_part: &str,
    table_id: Option<u32>,
) -> Option<String> {
    let want = table_id?;
    let wb = xml::parse(pkg.get(wb_part).ok()?).ok()?;
    let rels = Relationships::from_package(pkg, wb_part).ok()?;
    let sheets = wb.root.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        let rid = rid_attr(sh)?;
        let Ok(sheet_part) = resolve_id(pkg, wb_part, rid) else {
            continue;
        };
        let Ok(sheet) = xml::parse(pkg.get(&sheet_part).ok()?) else {
            continue;
        };
        let Some(parts) = sheet.root.child_by_local("tableParts") else {
            continue;
        };
        let sheet_rels = Relationships::from_package(pkg, &sheet_part).ok()?;
        for tp in parts.children_by_local("tablePart") {
            let Some(rid) = tp.attr_local("id") else {
                continue;
            };
            let Some(rel) = sheet_rels.find_by_id(rid) else {
                continue;
            };
            let Ok(part) = resolve_rel_target(&sheet_part, &rel.target) else {
                continue;
            };
            let Ok(doc) = xml::parse(pkg.get(&part).ok()?) else {
                continue;
            };
            let got = doc.root.attr("id").and_then(|s| s.parse::<u32>().ok());
            if got == Some(want) {
                return Some(
                    doc.root
                        .attr("displayName")
                        .or_else(|| doc.root.attr("name"))
                        .unwrap_or("")
                        .to_string(),
                );
            }
        }
        let _ = rels;
    }
    None
}

pub(crate) fn table_column(
    pkg: &Package,
    sheet: &Element,
    sheet_part: &str,
    sheet_name: &str,
    index: usize,
    field: &str,
    op_index: usize,
) -> Result<(u32, String, u32), Error> {
    let table_id = format!("sheet:{sheet_name}/table:{index}");
    let parent = sheet.child_by_local("tableParts").ok_or_else(|| {
        Error::with_path(
            ErrorKind::SheetTableNotFound {
                id: table_id.clone(),
            },
            format!("ops[{op_index}].source"),
        )
    })?;
    let Some(el) = super::super::list_pin::find_child(parent, "tablePart", "table", index) else {
        return Err(Error::with_path(
            ErrorKind::SheetTableNotFound { id: table_id },
            format!("ops[{op_index}].source"),
        ));
    };
    let rid = el.attr_local("id").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "tablePart".into(),
            child: "r:id".into(),
        })
    })?;
    let rels = Relationships::from_package(pkg, sheet_part)?;
    let rel = rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(sheet_part.to_string()),
            rid: rid.to_string(),
        })
    })?;
    let part = resolve_rel_target(sheet_part, &rel.target)?;
    let doc = super::super::list_pin::parse_part(pkg, &part)?;
    let xml_id = doc
        .root
        .attr("id")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(index as u32);
    let display = doc
        .root
        .attr("displayName")
        .or_else(|| doc.root.attr("name"))
        .unwrap_or("")
        .to_string();
    let Some(cols) = doc.root.child_by_local("tableColumns") else {
        return Err(field_err(field, op_index));
    };
    for col in cols.children_by_local("tableColumn") {
        if col.attr("name").is_some_and(|n| n == field) {
            let col_id = col
                .attr("id")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            return Ok((xml_id, display, col_id));
        }
    }
    Err(field_err(field, op_index))
}

pub(crate) fn table_cache_xml(name: &str, field: &str, table_id: u32, column: u32) -> Vec<u8> {
    let mut root = Element::new("slicerCacheDefinition")
        .with_attr("xmlns", X14_NS)
        .with_attr("xmlns:mc", MC_NS)
        .with_attr("mc:Ignorable", "x")
        .with_attr("xmlns:x", SML_NS)
        .with_attr("name", name)
        .with_attr("sourceName", field);
    let mut ext = Element::new("x:ext")
        .with_attr("uri", TABLE_SLICER_CACHE_URI)
        .with_attr("xmlns:x15", X15_NS);
    ext.push_element(
        Element::new("x15:tableSlicerCache")
            .with_attr("tableId", table_id.to_string())
            .with_attr("column", column.to_string()),
    );
    let mut lst = Element::new("extLst");
    lst.push_element(ext);
    root.push_element(lst);
    Document::new(Some(DEFAULT_DECL.to_string()), root)
        .to_bytes()
        .unwrap_or_default()
}

pub(crate) fn table_selected(pkg: &Package, cache_root: &Element) -> Vec<String> {
    let Some(tid) = table_id_of(cache_root) else {
        return Vec::new();
    };
    let col = table_column_id_of(cache_root);
    let Some(loc) = table_loc(pkg, tid, col) else {
        return Vec::new();
    };
    let Ok(bytes) = pkg.get(&loc.table_part) else {
        return Vec::new();
    };
    let Ok(doc) = xml::parse(bytes) else {
        return Vec::new();
    };
    filter_vals(&doc.root, loc.col_id)
}

pub(crate) fn set_table_selected(
    pkg: &mut Package,
    cache: &CacheMeta,
    selected: &[String],
    op_index: usize,
) -> Result<(), Error> {
    let bytes = pkg.get(&cache.part)?;
    let doc = xml::parse(bytes)?;
    let Some(tid) = table_id_of(&doc.root) else {
        return Err(unknown_selected(op_index));
    };
    let col = table_column_id_of(&doc.root);
    let Some(loc) = table_loc(pkg, tid, col) else {
        return Err(unknown_selected(op_index));
    };
    let allowed = table_column_values(pkg, &loc);
    for (k, name) in selected.iter().enumerate() {
        if !allowed.iter().any(|a| a == name) {
            return Err(Error::with_path(
                ErrorKind::InvalidSlicer {
                    field: "selected".into(),
                },
                format!("ops[{op_index}].selected[{k}]"),
            ));
        }
    }
    let mut table = super::super::list_pin::parse_part(pkg, &loc.table_part)?;
    write_table_filters(&mut table.root, loc.col_id, selected);
    super::super::list_pin::write_part(pkg, &loc.table_part, &table)?;
    Ok(())
}

pub(crate) fn table_column_id_of(root: &Element) -> Option<u32> {
    let mut id = None;
    picture::walk_all(root, &mut |e| {
        if e.local_name() == "tableSlicerCache" && id.is_none() {
            id = e.attr("column").and_then(|s| s.parse().ok());
        }
    });
    id
}

pub(crate) struct TableLoc {
    pub(crate) table_part: String,
    pub(crate) sheet_part: String,
    pub(crate) col_id: u32,
    pub(crate) start: a1::CellRef,
    pub(crate) end: a1::CellRef,
}

pub(crate) fn table_loc(pkg: &Package, table_id: u32, column: Option<u32>) -> Option<TableLoc> {
    let wb_part = office_document_part(pkg).ok()?;
    let wb = xml::parse(pkg.get(&wb_part).ok()?).ok()?;
    let sheets = wb.root.child_by_local("sheets")?;
    for sh in sheets.children_by_local("sheet") {
        let rid = rid_attr(sh)?;
        let Ok(sheet_part) = resolve_id(pkg, &wb_part, rid) else {
            continue;
        };
        let Ok(sheet) = xml::parse(pkg.get(&sheet_part).ok()?) else {
            continue;
        };
        let Some(parts) = sheet.root.child_by_local("tableParts") else {
            continue;
        };
        let sheet_rels = Relationships::from_package(pkg, &sheet_part).ok()?;
        for tp in parts.children_by_local("tablePart") {
            let Some(rid) = tp.attr_local("id") else {
                continue;
            };
            let Some(rel) = sheet_rels.find_by_id(rid) else {
                continue;
            };
            let Ok(part) = resolve_rel_target(&sheet_part, &rel.target) else {
                continue;
            };
            let Ok(doc) = xml::parse(pkg.get(&part).ok()?) else {
                continue;
            };
            let got = doc.root.attr("id").and_then(|s| s.parse::<u32>().ok());
            if got != Some(table_id) {
                continue;
            }
            let refer = doc.root.attr("ref")?;
            let (start, end) = a1::parse_range(refer).ok()?;
            let cols = doc.root.child_by_local("tableColumns")?;
            let list = cols.children_by_local("tableColumn");
            let col_id = if let Some(want) = column {
                list.iter()
                    .position(|c| c.attr("id").and_then(|s| s.parse::<u32>().ok()) == Some(want))
                    .unwrap_or(0) as u32
            } else {
                0
            };
            return Some(TableLoc {
                table_part: part,
                sheet_part,
                col_id,
                start,
                end,
            });
        }
    }
    None
}

pub(crate) fn table_column_values(pkg: &Package, loc: &TableLoc) -> Vec<String> {
    let Ok(wb_part) = office_document_part(pkg) else {
        return Vec::new();
    };
    let Ok(rels) = Relationships::from_package(pkg, &wb_part) else {
        return Vec::new();
    };
    let sst = super::super::inspect::load_shared_strings(pkg, &wb_part, &rels).unwrap_or_default();
    let Ok(bytes) = pkg.get(&loc.sheet_part) else {
        return Vec::new();
    };
    let Ok(sheet) = xml::parse(bytes) else {
        return Vec::new();
    };
    let col = loc.start.col + loc.col_id;
    let mut out = Vec::new();
    for row in (loc.start.row + 1)..=loc.end.row {
        let raw = sheet_cell_text(&sheet.root, a1::CellRef { col, row }, &sst);
        if raw.is_empty() {
            continue;
        }
        if !out.iter().any(|s| s == &raw) {
            out.push(raw);
        }
    }
    out
}

pub(crate) fn sheet_cell_text(root: &Element, cell: a1::CellRef, sst: &[String]) -> String {
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
                _ => c
                    .child_by_local("v")
                    .map(|v| v.text_content())
                    .unwrap_or_default(),
            };
        }
    }
    String::new()
}

pub(crate) fn filter_vals(table_root: &Element, col_id: u32) -> Vec<String> {
    let Some(af) = table_root.child_by_local("autoFilter") else {
        return Vec::new();
    };
    let want = col_id.to_string();
    for fc in af.children_by_local("filterColumn") {
        if fc.attr("colId") != Some(want.as_str()) {
            continue;
        }
        let mut out = Vec::new();
        if let Some(filters) = fc.child_by_local("filters") {
            for f in filters.children_by_local("filter") {
                if let Some(v) = f.attr("val") {
                    out.push(v.to_string());
                }
            }
        }
        return out;
    }
    Vec::new()
}

pub(crate) fn write_table_filters(table_root: &mut Element, col_id: u32, selected: &[String]) {
    let refer = table_root.attr("ref").unwrap_or("A1").to_string();
    if table_root.child_by_local("autoFilter").is_none() {
        table_root.push_element(Element::new("autoFilter").with_attr("ref", refer));
    }
    let Some(af) = table_root.child_by_local_mut("autoFilter") else {
        return;
    };
    let want = col_id.to_string();
    af.children.retain(|n| match n {
        Node::Element(e) => {
            !(e.local_name() == "filterColumn" && e.attr("colId") == Some(want.as_str()))
        }
        _ => true,
    });
    if selected.is_empty() {
        return;
    }
    let mut filters = Element::new("filters");
    for v in selected {
        filters.push_element(Element::new("filter").with_attr("val", v.clone()));
    }
    let mut col = Element::new("filterColumn").with_attr("colId", want);
    col.push_element(filters);
    af.push_element(col);
}
