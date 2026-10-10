//! 单元格数字格式与精简样式：inspect 摘要，apply 复用或追加 xf。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::apply;
use super::picture;
use super::request::{
    AlignPatch, BorderPatch, BorderSidePatch, ColorSpec, DxfPatch, FillPatch, FontPatch, Patch,
    SetCellStyleOp,
};
use super::{CT_STYLES, REL_STYLES, SML_NS};

/// inspect 用的只读样式表。没有 styles 部件时全部当作默认。
pub struct Stylebook {
    num_fmts: Vec<(u32, String)>,
    fonts: Vec<Element>,
    fills: Vec<Element>,
    borders: Vec<Element>,
    xfs: Vec<Element>,
    dxfs: Vec<Element>,
    cell_styles: Vec<Element>,
}

impl Stylebook {
    fn empty() -> Self {
        Self {
            num_fmts: Vec::new(),
            fonts: Vec::new(),
            fills: Vec::new(),
            borders: Vec::new(),
            xfs: Vec::new(),
            dxfs: Vec::new(),
            cell_styles: Vec::new(),
        }
    }

    pub fn dxf_json(&self, id: usize) -> Option<Value> {
        self.dxfs.get(id).and_then(dxf_summary)
    }

    pub fn named_count(&self) -> usize {
        self.cell_styles.len()
    }

    pub fn named_json(&self) -> Value {
        let items: Vec<Value> = self
            .cell_styles
            .iter()
            .enumerate()
            .map(|(i, el)| named_style_json(el, i + 1))
            .collect();
        Value::Array(items)
    }
}

/// 追加命名单元格样式。格式字段与 `setCellStyle` 相同。
#[derive(Debug, Clone)]
pub struct InsertCellStyleOp {
    pub name: String,
    pub num_fmt: Patch<String>,
    pub font: Patch<FontPatch>,
    pub fill: Patch<FillPatch>,
    pub border: Patch<BorderPatch>,
    pub align: Patch<AlignPatch>,
}

#[derive(Debug, Clone)]
pub struct DeleteCellStyleOp {
    #[allow(dead_code)]
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SetCellNamedStyleOp {
    pub id: String,
    pub style_index: usize,
}

fn named_style_json(el: &Element, n: usize) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("cellStyle:{n}")));
    obj.insert("index".into(), json!(n));
    obj.insert("name".into(), json!(el.attr("name").unwrap_or("")));
    let xf_id = el
        .attr("xfId")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    obj.insert("xfId".into(), json!(xf_id));
    let builtin = el.attr("builtinId").and_then(|s| s.parse::<i64>().ok());
    obj.insert("builtinId".into(), json!(builtin));
    Value::Object(obj)
}

pub fn parse_cell_style_id(id: &str, json_path: &str) -> Result<usize, Error> {
    let rest = id.strip_prefix("cellStyle:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CellStylePrefix,
            },
            json_path.to_string(),
        )
    })?;
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CellStylePrefix,
            },
            json_path.to_string(),
        ));
    }
    let n: usize = rest.parse().map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CellStyleIndexNotInt,
            },
            json_path.to_string(),
        )
    })?;
    if n < 1 {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::CellStyleIndexFromOne,
            },
            json_path.to_string(),
        ));
    }
    Ok(n)
}

pub fn load_for_inspect(
    pkg: &Package,
    wb_part: &str,
    rels: &Relationships,
) -> Result<Stylebook, Error> {
    let Some(part) = styles_part(wb_part, rels) else {
        return Ok(Stylebook::empty());
    };
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(Stylebook::empty());
    };
    let doc = xml::parse(bytes)?;
    Ok(stylebook_from_root(&doc.root))
}

pub fn attach_style(obj: &mut Value, c: &Element, book: &Stylebook) {
    let xf_i = c
        .attr("s")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|i| *i < book.xfs.len())
        .unwrap_or(0);
    let xf = book.xfs.get(xf_i);
    let num_id = xf
        .and_then(|x| x.attr("numFmtId"))
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    obj["numFmt"] = json!(resolve_num_fmt(book, num_id));

    let font_id = xf
        .and_then(|x| x.attr("fontId"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    if let Some(font) = book.fonts.get(font_id) {
        if let Some(summary) = font_summary(font, book.fonts.first()) {
            obj["font"] = summary;
        }
    }

    let fill_id = xf
        .and_then(|x| x.attr("fillId"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    if let Some(fill) = book.fills.get(fill_id) {
        if let Some(summary) = fill_summary(fill) {
            obj["fill"] = summary;
        }
    }

    let border_id = xf
        .and_then(|x| x.attr("borderId"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    if let Some(border) = book.borders.get(border_id) {
        if let Some(summary) = border_summary(border) {
            obj["border"] = summary;
        }
    }

    if let Some(xf) = xf {
        if let Some(summary) = align_summary(xf.child_by_local("alignment")) {
            obj["align"] = summary;
        }
    }
}

/// 没有 styles 则按 create 最小表新建。同一轮 apply 里只解析一次。
pub fn ensure_styles<'a>(
    pkg: &mut Package,
    wb_part: &str,
    slot: &'a mut Option<(String, Document)>,
) -> Result<&'a mut Element, Error> {
    if slot.is_none() {
        let part = load_or_create_styles(pkg, wb_part)?;
        let doc = xml::parse(pkg.get(&part)?)?;
        *slot = Some((part, doc));
    }
    Ok(&mut slot.as_mut().unwrap().1.root)
}

pub fn apply_set_cell_style(
    worksheet: &mut Element,
    styles: &mut Element,
    cell: a1::CellRef,
    op: &SetCellStyleOp,
    _op_index: usize,
) -> Result<(), Error> {
    let sheet_data = apply::ensure_sheet_data(worksheet);
    let current_s = apply::find_cell(sheet_data, cell)
        .and_then(|c| c.attr("s"))
        .and_then(|s| s.parse::<usize>().ok());
    let xf_idx = intern_xf(styles, current_s, op)?;
    let c = apply::ensure_cell(sheet_data, cell);
    if xf_idx == 0 {
        c.remove_attr("s");
    } else {
        c.set_attr("s", xf_idx.to_string());
    }
    apply::expand_dimension(worksheet, cell);
    Ok(())
}

pub fn insert_cell_style(
    styles: &mut Element,
    op: &InsertCellStyleOp,
    i: usize,
) -> Result<(), Error> {
    if name_taken(styles, &op.name) {
        return Err(Error::with_path(
            ErrorKind::InvalidNamedCellStyle {
                field: "name".into(),
            },
            format!("ops[{i}].name"),
        ));
    }
    let xf_id = intern_xf_into(
        styles,
        "cellStyleXfs",
        None,
        false,
        &op.num_fmt,
        &op.font,
        &op.fill,
        &op.border,
        &op.align,
    )?;
    let el = Element::new("cellStyle")
        .with_attr("name", &op.name)
        .with_attr("xfId", xf_id.to_string());
    let idx = ensure_collection(styles, "cellStyles");
    if let Node::Element(col) = &mut styles.children[idx] {
        col.push_element(el);
        set_count(col, "cellStyle");
    }
    Ok(())
}

pub fn delete_cell_style(
    styles: &mut Element,
    op: &DeleteCellStyleOp,
    i: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{i}].id");
    let (exists, is_builtin) = {
        let Some(parent) = styles.child_by_local("cellStyles") else {
            return Err(Error::with_path(
                ErrorKind::CellStyleNotFound {
                    id: format!("cellStyle:{}", op.index),
                },
                json_path,
            ));
        };
        match parent
            .children_by_local("cellStyle")
            .get(op.index.wrapping_sub(1))
        {
            None => (false, false),
            Some(el) => (true, el.attr("builtinId").is_some()),
        }
    };
    if !exists {
        return Err(Error::with_path(
            ErrorKind::CellStyleNotFound {
                id: format!("cellStyle:{}", op.index),
            },
            json_path,
        ));
    }
    if is_builtin {
        return Err(Error::with_path(
            ErrorKind::CannotDeleteBuiltinCellStyle,
            json_path,
        ));
    }
    let parent = styles.child_by_local_mut("cellStyles").unwrap();
    let mut seen = 0usize;
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "cellStyle" => {
            seen += 1;
            seen != op.index
        }
        _ => true,
    });
    set_count(parent, "cellStyle");
    Ok(())
}

pub fn apply_set_cell_named_style(
    worksheet: &mut Element,
    styles: &mut Element,
    cell: a1::CellRef,
    op: &SetCellNamedStyleOp,
    i: usize,
) -> Result<(), Error> {
    let json_path = format!("ops[{i}].style");
    let (xf_id, mut xf) = {
        let items = styles
            .child_by_local("cellStyles")
            .map(|c| c.children_by_local("cellStyle"))
            .unwrap_or_default();
        let Some(style_el) = items.get(op.style_index.wrapping_sub(1)) else {
            return Err(Error::with_path(
                ErrorKind::CellStyleNotFound {
                    id: format!("cellStyle:{}", op.style_index),
                },
                json_path,
            ));
        };
        let xf_id = style_el
            .attr("xfId")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let style_xfs = styles
            .child_by_local("cellStyleXfs")
            .map(|c| c.children_by_local("xf"))
            .unwrap_or_default();
        let Some(src) = style_xfs.get(xf_id) else {
            return Err(Error::with_path(
                ErrorKind::InvalidNamedCellStyle {
                    field: "xfId".into(),
                },
                json_path,
            ));
        };
        (xf_id, (*src).clone())
    };
    xf.remove_attr("applyNumberFormat");
    xf.remove_attr("applyFont");
    xf.remove_attr("applyFill");
    xf.remove_attr("applyBorder");
    xf.remove_attr("applyAlignment");
    xf.remove_attr("applyProtection");
    xf.set_attr("xfId", xf_id.to_string());
    let xf_idx = intern_child(styles, "cellXfs", "xf", &xf);
    let sheet_data = apply::ensure_sheet_data(worksheet);
    let c = apply::ensure_cell(sheet_data, cell);
    if xf_idx == 0 {
        c.remove_attr("s");
    } else {
        c.set_attr("s", xf_idx.to_string());
    }
    apply::expand_dimension(worksheet, cell);
    Ok(())
}

fn name_taken(styles: &Element, name: &str) -> bool {
    styles
        .child_by_local("cellStyles")
        .map(|c| c.children_by_local("cellStyle"))
        .unwrap_or_default()
        .iter()
        .any(|el| {
            el.attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
}

fn load_or_create_styles(pkg: &mut Package, wb_part: &str) -> Result<String, Error> {
    let rels = Relationships::from_package(pkg, wb_part)?;
    if let Some(part) = styles_part(wb_part, &rels) {
        if pkg.contains(&part) {
            return Ok(part);
        }
    }
    let part = sibling_part(wb_part, "styles.xml");
    pkg.set(&part, minimal_styles_xml().into_bytes())?;
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    if styles_part(wb_part, &rels).is_none() {
        let rid = rels.next_rid();
        rels.push(Relationship {
            id: rid,
            rel_type: REL_STYLES.into(),
            target: picture::rel_target_from(wb_part, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        rels.write_to(pkg, wb_part)?;
    }
    let mut types = ContentTypes::from_package(pkg)?;
    types.set_override(&part, CT_STYLES)?;
    types.write_to(pkg)?;
    Ok(part)
}

fn styles_part(wb_part: &str, rels: &Relationships) -> Option<String> {
    let rel = rels
        .find_by_type_suffix("/styles")
        .into_iter()
        .next()
        .or_else(|| {
            rels.rels
                .iter()
                .find(|r| r.rel_type == REL_STYLES || r.rel_type.ends_with("/styles"))
        })?;
    if rel.is_external() {
        return None;
    }
    opc::resolve_rel_target(wb_part, &rel.target).ok()
}

fn sibling_part(wb_part: &str, name: &str) -> String {
    match wb_part.rsplit_once('/') {
        Some(("", _)) => format!("/{name}"),
        Some((dir, _)) => format!("{dir}/{name}"),
        None => format!("/{name}"),
    }
}

fn minimal_styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="{SML_NS}"><fonts count="1"><font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
    )
}

fn stylebook_from_root(root: &Element) -> Stylebook {
    let mut num_fmts = Vec::new();
    if let Some(col) = root.child_by_local("numFmts") {
        for n in col.children_by_local("numFmt") {
            let Some(id) = n.attr("numFmtId").and_then(|s| s.parse().ok()) else {
                continue;
            };
            let code = n.attr("formatCode").unwrap_or("").to_string();
            num_fmts.push((id, code));
        }
    }
    Stylebook {
        num_fmts,
        fonts: clone_children(root.child_by_local("fonts"), "font"),
        fills: clone_children(root.child_by_local("fills"), "fill"),
        borders: clone_children(root.child_by_local("borders"), "border"),
        xfs: clone_children(root.child_by_local("cellXfs"), "xf"),
        dxfs: clone_children(root.child_by_local("dxfs"), "dxf"),
        cell_styles: clone_children(root.child_by_local("cellStyles"), "cellStyle"),
    }
}

fn clone_children(col: Option<&Element>, local: &str) -> Vec<Element> {
    col.map(|c| c.children_by_local(local).into_iter().cloned().collect())
        .unwrap_or_default()
}

fn resolve_num_fmt(book: &Stylebook, id: u32) -> String {
    if let Some((_, code)) = book.num_fmts.iter().find(|(i, _)| *i == id) {
        if !code.is_empty() {
            return code.clone();
        }
    }
    builtin_num_fmt(id).unwrap_or("General").to_string()
}

fn builtin_num_fmt(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        5 => r#"$#,##0_);($#,##0)"#,
        6 => r#"$#,##0_);[Red]($#,##0)"#,
        7 => r#"$#,##0.00_);($#,##0.00)"#,
        8 => r#"$#,##0.00_);[Red]($#,##0.00)"#,
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        37 => r#"#,##0 ;(#,##0)"#,
        38 => r#"#,##0 ;[Red](#,##0)"#,
        39 => r#"#,##0.00;(#,##0.00)"#,
        40 => r#"#,##0.00;[Red](#,##0.00)"#,
        41 => r#"_(* #,##0_);_(* (#,##0);_(* "-"_);_(@_)"#,
        42 => r#"_($* #,##0_);_($* (#,##0);_($* "-"_);_(@_)"#,
        43 => r#"_(* #,##0.00_);_(* (#,##0.00);_(* "-"??_);_(@_)"#,
        44 => r#"_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)"#,
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

fn font_summary(font: &Element, default: Option<&Element>) -> Option<Value> {
    let mut m = Map::new();
    let name = val_of(font, "name");
    let def_name = default.and_then(|d| val_of(d, "name"));
    if name != def_name {
        if let Some(n) = name {
            m.insert("name".into(), json!(n));
        }
    }
    let sz = val_of(font, "sz");
    let def_sz = default.and_then(|d| val_of(d, "sz"));
    if sz != def_sz {
        if let Some(s) = sz.and_then(json_sz) {
            m.insert("sz".into(), s);
        }
    }
    let bold = bool_child(font, "b");
    if bold != default.map(|d| bool_child(d, "b")).unwrap_or(false) && bold {
        m.insert("bold".into(), json!(true));
    }
    let italic = bool_child(font, "i");
    if italic != default.map(|d| bool_child(d, "i")).unwrap_or(false) && italic {
        m.insert("italic".into(), json!(true));
    }
    let u = underline_of(font);
    let def_u = default.and_then(underline_of);
    if u != def_u {
        if let Some(v) = u {
            m.insert("underline".into(), json!(v));
        }
    }
    let color = font.child_by_local("color").and_then(color_json);
    let def_color = default.and_then(|d| d.child_by_local("color").and_then(color_json));
    if color != def_color {
        if let Some(c) = color {
            m.insert("color".into(), c);
        }
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

fn fill_summary(fill: &Element) -> Option<Value> {
    if fill.child_by_local("gradientFill").is_some() {
        return Some(json!({ "pattern": "gradient" }));
    }
    let pf = fill.child_by_local("patternFill")?;
    let pattern = pf.attr("patternType").unwrap_or("none");
    if pattern == "none" {
        return None;
    }
    let mut m = Map::new();
    m.insert("pattern".into(), json!(pattern));
    if let Some(fg) = pf.child_by_local("fgColor").and_then(color_json) {
        m.insert("fg".into(), fg);
    }
    if let Some(bg) = pf.child_by_local("bgColor").and_then(color_json) {
        m.insert("bg".into(), bg);
    }
    Some(Value::Object(m))
}

fn border_summary(border: &Element) -> Option<Value> {
    let mut m = Map::new();
    for side in ["left", "right", "top", "bottom"] {
        if let Some(s) = side_json(border.child_by_local(side)) {
            m.insert(side.into(), s);
        }
    }
    if let Some(s) = side_json(border.child_by_local("diagonal")) {
        m.insert("diagonal".into(), s);
    }
    if attr_bool(border, "diagonalUp") {
        m.insert("diagonalUp".into(), json!(true));
    }
    if attr_bool(border, "diagonalDown") {
        m.insert("diagonalDown".into(), json!(true));
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

fn side_json(side: Option<&Element>) -> Option<Value> {
    let side = side?;
    let style = side.attr("style")?;
    if style == "none" {
        return None;
    }
    let mut m = Map::new();
    m.insert("style".into(), json!(style));
    if let Some(c) = side.child_by_local("color").and_then(color_json) {
        m.insert("color".into(), c);
    }
    Some(Value::Object(m))
}

fn align_summary(al: Option<&Element>) -> Option<Value> {
    let al = al?;
    let mut m = Map::new();
    if let Some(h) = al.attr("horizontal") {
        m.insert("horizontal".into(), json!(h));
    }
    if let Some(v) = al.attr("vertical") {
        m.insert("vertical".into(), json!(v));
    }
    if attr_bool(al, "wrapText") {
        m.insert("wrap".into(), json!(true));
    }
    if let Some(n) = al.attr("indent").and_then(|s| s.parse::<u32>().ok()) {
        if n > 0 {
            m.insert("indent".into(), json!(n));
        }
    }
    if let Some(n) = al.attr("textRotation").and_then(|s| s.parse::<i64>().ok()) {
        m.insert("rotation".into(), json!(n));
    }
    if attr_bool(al, "shrinkToFit") {
        m.insert("shrink".into(), json!(true));
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

pub(crate) fn color_json(el: &Element) -> Option<Value> {
    if attr_bool(el, "auto") {
        return None;
    }
    let mut m = Map::new();
    if let Some(rgb) = el.attr("rgb") {
        m.insert("rgb".into(), json!(rgb.to_ascii_uppercase()));
    } else if let Some(theme) = el.attr("theme").and_then(|s| s.parse::<u32>().ok()) {
        m.insert("theme".into(), json!(theme));
        if let Some(tint) = el.attr("tint").and_then(|s| s.parse::<f64>().ok()) {
            m.insert("tint".into(), json!(tint));
        }
    } else if let Some(indexed) = el.attr("indexed").and_then(|s| s.parse::<u32>().ok()) {
        m.insert("indexed".into(), json!(indexed));
    } else {
        return None;
    }
    Some(Value::Object(m))
}

fn intern_xf(
    styles: &mut Element,
    current_s: Option<usize>,
    op: &SetCellStyleOp,
) -> Result<usize, Error> {
    intern_xf_into(
        styles,
        "cellXfs",
        current_s,
        true,
        &op.num_fmt,
        &op.font,
        &op.fill,
        &op.border,
        &op.align,
    )
}

fn intern_xf_into(
    styles: &mut Element,
    collection: &str,
    current_s: Option<usize>,
    write_apply: bool,
    num_fmt: &Patch<String>,
    font: &Patch<FontPatch>,
    fill: &Patch<FillPatch>,
    border: &Patch<BorderPatch>,
    align: &Patch<AlignPatch>,
) -> Result<usize, Error> {
    let xfs: Vec<Element> = styles
        .child_by_local(collection)
        .map(|c| c.children_by_local("xf").into_iter().cloned().collect())
        .unwrap_or_default();
    let src_i = current_s.filter(|i| *i < xfs.len()).unwrap_or(0);
    let mut xf = if collection == "cellStyleXfs" && current_s.is_none() {
        default_style_xf()
    } else {
        xfs.get(src_i).cloned().unwrap_or_else(default_xf)
    };
    if collection == "cellStyleXfs" {
        xf.remove_attr("xfId");
    }

    match num_fmt {
        Patch::Absent => {}
        Patch::Clear => {
            xf.set_attr("numFmtId", "0");
            if write_apply {
                xf.set_attr("applyNumberFormat", "1");
            }
        }
        Patch::Set(code) => {
            let id = intern_num_fmt(styles, code);
            xf.set_attr("numFmtId", id.to_string());
            if write_apply {
                xf.set_attr("applyNumberFormat", "1");
            }
        }
    }

    match font {
        Patch::Absent => {}
        Patch::Clear => {
            xf.set_attr("fontId", "0");
            if write_apply {
                xf.set_attr("applyFont", "1");
            }
        }
        Patch::Set(patch) => {
            let cur = font_at(styles, attr_usize(&xf, "fontId").unwrap_or(0));
            let next = apply_font_patch(cur, patch);
            let id = intern_child(styles, "fonts", "font", &next);
            xf.set_attr("fontId", id.to_string());
            if write_apply {
                xf.set_attr("applyFont", "1");
            }
        }
    }

    match fill {
        Patch::Absent => {}
        Patch::Clear => {
            xf.set_attr("fillId", "0");
            if write_apply {
                xf.set_attr("applyFill", "1");
            }
        }
        Patch::Set(patch) => {
            let cur = fill_at(styles, attr_usize(&xf, "fillId").unwrap_or(0));
            let next = apply_fill_patch(cur, patch);
            let id = intern_child(styles, "fills", "fill", &next);
            xf.set_attr("fillId", id.to_string());
            if write_apply {
                xf.set_attr("applyFill", "1");
            }
        }
    }

    match border {
        Patch::Absent => {}
        Patch::Clear => {
            xf.set_attr("borderId", "0");
            if write_apply {
                xf.set_attr("applyBorder", "1");
            }
        }
        Patch::Set(patch) => {
            let cur = border_at(styles, attr_usize(&xf, "borderId").unwrap_or(0));
            let next = apply_border_patch(cur, patch);
            let id = intern_child(styles, "borders", "border", &next);
            xf.set_attr("borderId", id.to_string());
            if write_apply {
                xf.set_attr("applyBorder", "1");
            }
        }
    }

    match align {
        Patch::Absent => {}
        Patch::Clear => {
            remove_children(&mut xf, "alignment");
            xf.remove_attr("applyAlignment");
        }
        Patch::Set(patch) => {
            let cur = xf.child_by_local("alignment").cloned();
            let next = apply_align_patch(cur, patch);
            remove_children(&mut xf, "alignment");
            if alignment_has_content(&next) {
                xf.children.insert(0, Node::Element(next));
                if write_apply {
                    xf.set_attr("applyAlignment", "1");
                }
            } else {
                xf.remove_attr("applyAlignment");
            }
        }
    }

    if collection == "cellStyleXfs" {
        xf.remove_attr("xfId");
    }
    if !write_apply {
        xf.remove_attr("applyNumberFormat");
        xf.remove_attr("applyFont");
        xf.remove_attr("applyFill");
        xf.remove_attr("applyBorder");
        xf.remove_attr("applyAlignment");
        xf.remove_attr("applyProtection");
    }

    Ok(intern_child(styles, collection, "xf", &xf))
}

/// 把条件格式差分写入 `<dxfs>`，相同定义复用下标。空补丁不写。
pub(crate) fn intern_dxf(styles: &mut Element, patch: &DxfPatch) -> Option<usize> {
    if patch.is_empty() {
        return None;
    }
    let mut dxf = Element::new("dxf");
    if let Some(font) = &patch.font {
        let el = apply_font_patch(Element::new("font"), font);
        if !el.children.is_empty() {
            dxf.push_element(el);
        }
    }
    if let Some(fill) = &patch.fill {
        let mut el = apply_fill_patch(Element::new("fill"), fill);
        copy_fg_to_bg_if_needed(&mut el);
        dxf.push_element(el);
    }
    if let Some(border) = &patch.border {
        let el = apply_border_patch(empty_border(), border);
        if border_summary(&el).is_some() {
            dxf.push_element(el);
        }
    }
    if dxf.children.is_empty() {
        return None;
    }
    Some(intern_child(styles, "dxfs", "dxf", &dxf))
}

fn copy_fg_to_bg_if_needed(fill: &mut Element) {
    let Some(pf) = fill.child_by_local_mut("patternFill") else {
        return;
    };
    if pf.child_by_local("bgColor").is_some() {
        return;
    }
    let Some(fg) = pf.child_by_local("fgColor").cloned() else {
        return;
    };
    let mut bg = fg;
    bg.name = "bgColor".into();
    pf.push_element(bg);
}

fn dxf_summary(dxf: &Element) -> Option<Value> {
    let mut m = Map::new();
    if let Some(font) = dxf
        .child_by_local("font")
        .and_then(|f| font_summary(f, None))
    {
        m.insert("font".into(), font);
    }
    if let Some(fill) = dxf.child_by_local("fill").and_then(dxf_fill_summary) {
        m.insert("fill".into(), fill);
    }
    if let Some(border) = dxf.child_by_local("border").and_then(border_summary) {
        m.insert("border".into(), border);
    }
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

fn dxf_fill_summary(fill: &Element) -> Option<Value> {
    if fill.child_by_local("gradientFill").is_some() {
        return Some(json!({ "pattern": "gradient" }));
    }
    let pf = fill.child_by_local("patternFill")?;
    let fg = pf.child_by_local("fgColor").and_then(color_json);
    let bg = pf.child_by_local("bgColor").and_then(color_json);
    let mut pattern = pf.attr("patternType").unwrap_or("none");
    if pattern == "none" && (fg.is_some() || bg.is_some()) {
        pattern = "solid";
    }
    if pattern == "none" {
        return None;
    }
    let mut m = Map::new();
    m.insert("pattern".into(), json!(pattern));
    if let Some(fg) = fg {
        m.insert("fg".into(), fg);
    }
    if let Some(bg) = bg {
        m.insert("bg".into(), bg);
    }
    Some(Value::Object(m))
}

pub(crate) fn color_el(name: &str, spec: &ColorSpec) -> Element {
    color_element(name, spec)
}

fn default_xf() -> Element {
    Element::new("xf")
        .with_attr("numFmtId", "0")
        .with_attr("fontId", "0")
        .with_attr("fillId", "0")
        .with_attr("borderId", "0")
        .with_attr("xfId", "0")
}

fn default_style_xf() -> Element {
    Element::new("xf")
        .with_attr("numFmtId", "0")
        .with_attr("fontId", "0")
        .with_attr("fillId", "0")
        .with_attr("borderId", "0")
}

fn intern_num_fmt(styles: &mut Element, code: &str) -> u32 {
    if code.eq_ignore_ascii_case("General") {
        return 0;
    }
    if let Some((id, _)) = BUILTIN_BY_CODE.iter().find(|(_, c)| *c == code) {
        return *id;
    }
    if let Some(col) = styles.child_by_local("numFmts") {
        for n in col.children_by_local("numFmt") {
            if n.attr("formatCode") == Some(code) {
                if let Some(id) = n.attr("numFmtId").and_then(|s| s.parse().ok()) {
                    return id;
                }
            }
        }
    }
    let mut max_id = 163u32;
    if let Some(col) = styles.child_by_local("numFmts") {
        for n in col.children_by_local("numFmt") {
            if let Some(id) = n.attr("numFmtId").and_then(|s| s.parse::<u32>().ok()) {
                max_id = max_id.max(id);
            }
        }
    }
    let id = max_id + 1;
    let mut el = Element::new("numFmt");
    el.set_attr("numFmtId", id.to_string());
    el.set_attr("formatCode", code);
    let idx = ensure_collection(styles, "numFmts");
    if let Node::Element(col) = &mut styles.children[idx] {
        col.push_element(el);
        set_count(col, "numFmt");
    }
    id
}

const BUILTIN_BY_CODE: &[(u32, &str)] = &[
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "mm-dd-yy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yy h:mm"),
    (37, r#"#,##0 ;(#,##0)"#),
    (38, r#"#,##0 ;[Red](#,##0)"#),
    (39, r#"#,##0.00;(#,##0.00)"#),
    (40, r#"#,##0.00;[Red](#,##0.00)"#),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mmss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
];

fn intern_child(styles: &mut Element, col: &str, child: &str, sample: &Element) -> usize {
    let idx = ensure_collection(styles, col);
    let key = elem_key(sample);
    if let Node::Element(col_el) = &styles.children[idx] {
        for (i, existing) in col_el.children_by_local(child).into_iter().enumerate() {
            if elem_key(existing) == key {
                return i;
            }
        }
    }
    if let Node::Element(col_el) = &mut styles.children[idx] {
        col_el.push_element(sample.clone());
        set_count(col_el, child);
        return col_el.children_by_local(child).len() - 1;
    }
    0
}

fn ensure_collection(root: &mut Element, name: &str) -> usize {
    const ORDER: &[&str] = &[
        "numFmts",
        "fonts",
        "fills",
        "borders",
        "cellStyleXfs",
        "cellXfs",
        "cellStyles",
        "dxfs",
        "tableStyles",
        "colors",
        "extLst",
    ];
    if let Some(i) = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == name))
    {
        return i;
    }
    let my = ORDER.iter().position(|n| *n == name).unwrap_or(ORDER.len());
    let insert_at = root
        .children
        .iter()
        .position(|n| {
            let Node::Element(e) = n else {
                return false;
            };
            ORDER
                .iter()
                .position(|n| *n == e.local_name())
                .is_some_and(|pos| pos > my)
        })
        .unwrap_or(root.children.len());
    let mut el = Element::new(name);
    el.set_attr("count", "0");
    root.children.insert(insert_at, Node::Element(el));
    insert_at
}

fn set_count(col: &mut Element, child: &str) {
    let n = col.children_by_local(child).len();
    col.set_attr("count", n.to_string());
}

fn font_at(styles: &Element, id: usize) -> Element {
    styles
        .child_by_local("fonts")
        .and_then(|c| c.children_by_local("font").into_iter().nth(id).cloned())
        .unwrap_or_else(|| Element::new("font"))
}

fn fill_at(styles: &Element, id: usize) -> Element {
    styles
        .child_by_local("fills")
        .and_then(|c| c.children_by_local("fill").into_iter().nth(id).cloned())
        .unwrap_or_else(|| {
            let mut f = Element::new("fill");
            let mut pf = Element::new("patternFill");
            pf.set_attr("patternType", "none");
            f.push_element(pf);
            f
        })
}

fn border_at(styles: &Element, id: usize) -> Element {
    styles
        .child_by_local("borders")
        .and_then(|c| c.children_by_local("border").into_iter().nth(id).cloned())
        .unwrap_or_else(empty_border)
}

fn empty_border() -> Element {
    let mut b = Element::new("border");
    for n in ["left", "right", "top", "bottom", "diagonal"] {
        b.push_element(Element::new(n));
    }
    b
}

fn apply_font_patch(mut font: Element, patch: &FontPatch) -> Element {
    if let Some(name) = &patch.name {
        set_val_child(&mut font, "name", name);
    }
    if let Some(sz) = patch.sz {
        set_val_child(&mut font, "sz", &fmt_sz(sz));
    }
    if let Some(bold) = patch.bold {
        set_bool_child(&mut font, "b", bold);
    }
    if let Some(italic) = patch.italic {
        set_bool_child(&mut font, "i", italic);
    }
    if let Some(u) = &patch.underline {
        match u {
            None => remove_children(&mut font, "u"),
            Some(v) => {
                remove_children(&mut font, "u");
                let mut el = Element::new("u");
                if v != "single" {
                    el.set_attr("val", v.clone());
                }
                font.push_element(el);
            }
        }
    }
    if let Some(color) = &patch.color {
        set_color_child(&mut font, color);
    }
    font
}

fn apply_fill_patch(mut fill: Element, patch: &FillPatch) -> Element {
    if fill.child_by_local("gradientFill").is_some() {
        fill = Element::new("fill");
        let mut pf = Element::new("patternFill");
        pf.set_attr("patternType", "none");
        fill.push_element(pf);
    }
    if fill.child_by_local("patternFill").is_none() {
        let mut pf = Element::new("patternFill");
        pf.set_attr("patternType", "none");
        fill.push_element(pf);
    }
    let pattern = patch.pattern.clone().unwrap_or_else(|| {
        if patch.fg.is_some() {
            "solid".into()
        } else {
            fill.child_by_local("patternFill")
                .and_then(|p| p.attr("patternType"))
                .unwrap_or("none")
                .to_string()
        }
    });
    let pf = fill.child_by_local_mut("patternFill").unwrap();
    pf.set_attr("patternType", pattern);
    if let Some(fg) = &patch.fg {
        set_named_color(pf, "fgColor", fg);
    }
    if let Some(bg) = &patch.bg {
        set_named_color(pf, "bgColor", bg);
    }
    fill
}

fn apply_border_patch(mut border: Element, patch: &BorderPatch) -> Element {
    for name in ["left", "right", "top", "bottom"] {
        if border.child_by_local(name).is_none() {
            border.push_element(Element::new(name));
        }
    }
    if border.child_by_local("diagonal").is_none() {
        border.push_element(Element::new("diagonal"));
    }
    let default_side = BorderSidePatch {
        style: patch.style.clone(),
        color: patch.color.clone(),
    };
    apply_side(
        &mut border,
        "left",
        patch.left.as_ref().unwrap_or(&default_side),
        patch.left.is_some() || patch.style.is_some() || patch.color.is_some(),
    );
    apply_side(
        &mut border,
        "right",
        patch.right.as_ref().unwrap_or(&default_side),
        patch.right.is_some() || patch.style.is_some() || patch.color.is_some(),
    );
    apply_side(
        &mut border,
        "top",
        patch.top.as_ref().unwrap_or(&default_side),
        patch.top.is_some() || patch.style.is_some() || patch.color.is_some(),
    );
    apply_side(
        &mut border,
        "bottom",
        patch.bottom.as_ref().unwrap_or(&default_side),
        patch.bottom.is_some() || patch.style.is_some() || patch.color.is_some(),
    );
    let style_given = patch
        .diagonal
        .as_ref()
        .and_then(|d| d.style.as_ref())
        .is_some();
    let dir_given = patch.diagonal_up.is_some() || patch.diagonal_down.is_some();
    if patch.diagonal.is_some() || dir_given {
        let mut side = patch.diagonal.clone().unwrap_or(BorderSidePatch {
            style: None,
            color: None,
        });
        if side.style.is_none() && dir_given {
            side.style = Some("thin".into());
        }
        apply_side(&mut border, "diagonal", &side, true);
        if style_given && !dir_given {
            border.set_attr("diagonalUp", "1");
            border.set_attr("diagonalDown", "1");
        } else {
            if let Some(up) = patch.diagonal_up {
                if up {
                    border.set_attr("diagonalUp", "1");
                } else {
                    border.remove_attr("diagonalUp");
                }
            }
            if let Some(down) = patch.diagonal_down {
                if down {
                    border.set_attr("diagonalDown", "1");
                } else {
                    border.remove_attr("diagonalDown");
                }
            }
        }
    }
    border
}

fn apply_side(border: &mut Element, name: &str, side: &BorderSidePatch, touched: bool) {
    if !touched {
        return;
    }
    let Some(el) = border.child_by_local_mut(name) else {
        return;
    };
    if let Some(style) = &side.style {
        if style == "none" {
            el.remove_attr("style");
        } else {
            el.set_attr("style", style.clone());
        }
    }
    if let Some(color) = &side.color {
        set_color_child(el, color);
    }
}

fn apply_align_patch(cur: Option<Element>, patch: &AlignPatch) -> Element {
    let mut al = cur.unwrap_or_else(|| Element::new("alignment"));
    if let Some(h) = &patch.horizontal {
        al.set_attr("horizontal", h.clone());
    }
    if let Some(v) = &patch.vertical {
        al.set_attr("vertical", v.clone());
    }
    if let Some(wrap) = patch.wrap {
        if wrap {
            al.set_attr("wrapText", "1");
        } else {
            al.remove_attr("wrapText");
        }
    }
    if let Some(n) = patch.indent {
        if n == 0 {
            al.remove_attr("indent");
        } else {
            al.set_attr("indent", n.to_string());
            if patch.horizontal.is_none() {
                al.set_attr("horizontal", "left");
            }
        }
    }
    if let Some(n) = patch.rotation {
        if n == 0 {
            al.remove_attr("textRotation");
        } else {
            al.set_attr("textRotation", n.to_string());
        }
    }
    if let Some(shrink) = patch.shrink {
        if shrink {
            al.set_attr("shrinkToFit", "1");
        } else {
            al.remove_attr("shrinkToFit");
        }
    }
    al
}

fn alignment_has_content(al: &Element) -> bool {
    al.attr("horizontal").is_some()
        || al.attr("vertical").is_some()
        || al.attr("wrapText").is_some()
        || al.attr("indent").is_some()
        || al.attr("textRotation").is_some()
        || al.attr("shrinkToFit").is_some()
        || al.attr("readingOrder").is_some()
}

fn set_val_child(parent: &mut Element, name: &str, val: &str) {
    if let Some(el) = parent.child_by_local_mut(name) {
        el.set_attr("val", val);
        return;
    }
    parent.push_element(Element::new(name).with_attr("val", val));
}

fn set_bool_child(parent: &mut Element, name: &str, on: bool) {
    remove_children(parent, name);
    if on {
        parent.push_element(Element::new(name));
    }
}

fn set_color_child(parent: &mut Element, spec: &ColorSpec) {
    remove_children(parent, "color");
    parent.push_element(color_element("color", spec));
}

fn set_named_color(parent: &mut Element, name: &str, spec: &ColorSpec) {
    remove_children(parent, name);
    parent.push_element(color_element(name, spec));
}

fn color_element(name: &str, spec: &ColorSpec) -> Element {
    let mut el = Element::new(name);
    if let Some(rgb) = &spec.rgb {
        el.set_attr("rgb", normalize_rgb(rgb));
    } else if let Some(theme) = spec.theme {
        el.set_attr("theme", theme.to_string());
        if let Some(tint) = spec.tint {
            el.set_attr("tint", tint.to_string());
        }
    } else if let Some(indexed) = spec.indexed {
        el.set_attr("indexed", indexed.to_string());
    }
    el
}

fn normalize_rgb(s: &str) -> String {
    let u = s.to_ascii_uppercase();
    if u.len() == 6 {
        format!("FF{u}")
    } else {
        u
    }
}

fn remove_children(el: &mut Element, local: &str) {
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        Node::Text(_) => true,
    });
}

fn val_of<'a>(el: &'a Element, name: &str) -> Option<&'a str> {
    el.child_by_local(name).and_then(|c| c.attr("val"))
}

fn bool_child(el: &Element, name: &str) -> bool {
    let Some(c) = el.child_by_local(name) else {
        return false;
    };
    match c.attr("val") {
        None | Some("1") | Some("true") | Some("on") => true,
        Some("0") | Some("false") | Some("off") => false,
        _ => true,
    }
}

fn underline_of(font: &Element) -> Option<String> {
    let u = font.child_by_local("u")?;
    match u.attr("val") {
        None | Some("single") => Some("single".into()),
        Some("none") => None,
        Some(v) => Some(v.to_string()),
    }
}

fn attr_bool(el: &Element, name: &str) -> bool {
    matches!(el.attr(name), Some("1") | Some("true") | Some("on"))
}

fn attr_usize(el: &Element, name: &str) -> Option<usize> {
    el.attr(name).and_then(|s| s.parse().ok())
}

fn json_sz(raw: &str) -> Option<Value> {
    let n: f64 = raw.parse().ok()?;
    if n.fract() == 0.0 {
        Some(json!(n as i64))
    } else {
        Some(json!(n))
    }
}

fn fmt_sz(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// 用来判断两条定义是否相同：属性与子树的规范化文本。
fn elem_key(el: &Element) -> String {
    let mut attrs: Vec<(String, String)> = el
        .attrs
        .iter()
        .map(|(k, v)| {
            if k == "rgb" {
                (k.clone(), normalize_rgb(v))
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect();
    attrs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut s = el.local_name().to_string();
    for (k, v) in attrs {
        s.push('|');
        s.push_str(&k);
        s.push('=');
        s.push_str(&v);
    }
    let mut kids: Vec<&Element> = el.element_children().collect();
    kids.sort_by_key(|e| e.local_name().to_string());
    for k in kids {
        s.push('{');
        s.push_str(&elem_key(k));
        s.push('}');
    }
    s
}
