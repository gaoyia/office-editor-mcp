//! 共享公式、数组公式与数据表公式：inspect 索引、相对 A1 展开、apply 占用检查。
//!
//! 展开只平移公式里的 A1 单元格与 A1 区域（尊重 `$`）。这不是插行引用平移引擎。
//! 数据表不求值；主格 `TABLE` 只对应 `r1` / `r2` / `dt2D` / `dtr`。

use std::collections::HashMap;

use super::a1::{self, AbsCellRef, CellRef};
use crate::xml::Element;

#[derive(Debug, Clone)]
pub struct SharedGroup {
    pub si: String,
    pub master: CellRef,
    pub formula: String,
    pub start: CellRef,
    pub end: CellRef,
}

#[derive(Debug, Clone)]
pub struct RangeGroup {
    pub master: CellRef,
    pub formula: String,
    pub start: CellRef,
    pub end: CellRef,
}

#[derive(Debug, Clone, Default)]
pub struct FormulaIndex {
    pub shared: HashMap<String, SharedGroup>,
    pub arrays: Vec<RangeGroup>,
    pub tables: Vec<RangeGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Occupied {
    Shared { si: String, master: CellRef },
    Array { master: CellRef },
    DataTable { master: CellRef },
}

pub fn shown_formula(body: &str) -> String {
    let body = body.trim();
    if body.starts_with('=') {
        body.to_string()
    } else {
        format!("={body}")
    }
}

pub fn formula_body(formula: &str) -> &str {
    formula.strip_prefix('=').unwrap_or(formula)
}

pub fn xml_true(val: Option<&str>) -> bool {
    matches!(val.map(str::trim), Some("1" | "true" | "TRUE" | "on"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableShape {
    Column,
    Row,
    TwoD,
}

pub fn table_shape(dt2d: bool, dtr: bool) -> TableShape {
    if dt2d {
        TableShape::TwoD
    } else if dtr {
        TableShape::Row
    } else {
        TableShape::Column
    }
}

pub fn shown_cell_ref(raw: &str) -> String {
    match a1::parse_cell(raw) {
        Ok(c) => a1::format_cell(c.col, c.row),
        Err(_) => raw.to_string(),
    }
}

pub fn shown_table_formula(dt2d: bool, dtr: bool, r1: &str, r2: Option<&str>) -> String {
    let r1 = shown_cell_ref(r1);
    if dt2d {
        let r2 = r2.map(shown_cell_ref).unwrap_or_default();
        format!("=TABLE({r1},{r2})")
    } else if dtr {
        format!("=TABLE({r1},)")
    } else {
        format!("=TABLE(,{r1})")
    }
}

pub fn shown_table_from_elem(f: &Element) -> Option<String> {
    let r1 = f.attr("r1")?;
    if r1.trim().is_empty() {
        return None;
    }
    Some(shown_table_formula(
        xml_true(f.attr("dt2D")),
        xml_true(f.attr("dtr")),
        r1,
        f.attr("r2"),
    ))
}

/// 解析 `=TABLE(a,b)`。空参数为 `None`。不是恰好一个逗号的 `TABLE(...)` 则 `None`。
pub fn parse_table_formula(formula: &str) -> Option<(Option<CellRef>, Option<CellRef>)> {
    let body = formula_body(formula).trim();
    let rest = strip_prefix_ci(body, "TABLE")?;
    let rest = rest.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let inner = &rest[1..rest.len() - 1];
    let comma = inner.find(',')?;
    if inner[comma + 1..].contains(',') {
        return None;
    }
    let a = inner[..comma].trim();
    let b = inner[comma + 1..].trim();
    let first = if a.is_empty() {
        None
    } else {
        Some(a1::parse_cell(a).ok()?)
    };
    let second = if b.is_empty() {
        None
    } else {
        Some(a1::parse_cell(b).ok()?)
    };
    Some((first, second))
}

pub fn args_match_shape(
    first: Option<CellRef>,
    second: Option<CellRef>,
    shape: TableShape,
) -> bool {
    match shape {
        TableShape::TwoD => first.is_some() && second.is_some(),
        TableShape::Row => first.is_some() && second.is_none(),
        TableShape::Column => first.is_none() && second.is_some(),
    }
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    if s.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

pub fn index_sheet(sheet_data: &Element) -> FormulaIndex {
    let mut shared_acc: HashMap<String, SharedAcc> = HashMap::new();
    let mut arrays = Vec::new();
    let mut tables = Vec::new();

    for_each_cell(sheet_data, |cell, c| {
        let Some(f) = c.child_by_local("f") else {
            return;
        };
        let ft = f.attr("t").unwrap_or("");
        let text = f.text_content();
        let body = text.trim().trim_start_matches('=').to_string();
        match ft {
            "shared" => {
                let si = f.attr("si").unwrap_or("").to_string();
                if si.is_empty() {
                    return;
                }
                let acc = shared_acc.entry(si.clone()).or_insert_with(|| SharedAcc {
                    si,
                    master: None,
                    formula: String::new(),
                    start: cell,
                    end: cell,
                });
                if let Some(r) = f.attr("ref") {
                    if let Ok((start, end)) = a1::parse_range(r) {
                        acc.start = start;
                        acc.end = end;
                    }
                    acc.master = Some(cell);
                }
                if !body.is_empty() {
                    acc.formula = body;
                    if acc.master.is_none() {
                        acc.master = Some(cell);
                    }
                }
            }
            "array" => {
                let (start, end) = f
                    .attr("ref")
                    .and_then(|r| a1::parse_range(r).ok())
                    .unwrap_or((cell, cell));
                arrays.push(RangeGroup {
                    master: cell,
                    formula: body,
                    start,
                    end,
                });
            }
            "dataTable" => {
                let (start, end) = f
                    .attr("ref")
                    .and_then(|r| a1::parse_range(r).ok())
                    .unwrap_or((cell, cell));
                tables.push(RangeGroup {
                    master: cell,
                    formula: body,
                    start,
                    end,
                });
            }
            _ => {}
        }
    });

    let mut shared = HashMap::new();
    for (si, acc) in shared_acc {
        let Some(master) = acc.master else {
            continue;
        };
        shared.insert(
            si,
            SharedGroup {
                si: acc.si,
                master,
                formula: acc.formula,
                start: acc.start,
                end: acc.end,
            },
        );
    }

    FormulaIndex {
        shared,
        arrays,
        tables,
    }
}

struct SharedAcc {
    si: String,
    master: Option<CellRef>,
    formula: String,
    start: CellRef,
    end: CellRef,
}

pub fn occupy_at(index: &FormulaIndex, cell: CellRef) -> Option<Occupied> {
    for g in &index.tables {
        if a1::in_range(cell, g.start, g.end) {
            return Some(Occupied::DataTable { master: g.master });
        }
    }
    for g in &index.arrays {
        if a1::in_range(cell, g.start, g.end) {
            return Some(Occupied::Array { master: g.master });
        }
    }
    for g in index.shared.values() {
        if a1::in_range(cell, g.start, g.end) {
            return Some(Occupied::Shared {
                si: g.si.clone(),
                master: g.master,
            });
        }
    }
    None
}

pub fn shared_members(sheet_data: &Element, si: &str) -> Vec<CellRef> {
    let mut out = Vec::new();
    for_each_cell(sheet_data, |cell, c| {
        let Some(f) = c.child_by_local("f") else {
            return;
        };
        if f.attr("t") == Some("shared") && f.attr("si") == Some(si) {
            out.push(cell);
        }
    });
    out
}

pub fn next_si(sheet_data: &Element) -> u32 {
    let mut max: Option<u32> = None;
    for_each_cell(sheet_data, |_, c| {
        let Some(f) = c.child_by_local("f") else {
            return;
        };
        if f.attr("t") != Some("shared") {
            return;
        }
        let Some(si) = f.attr("si") else {
            return;
        };
        if let Ok(n) = si.parse::<u32>() {
            max = Some(max.map_or(n, |m| m.max(n)));
        }
    });
    max.map_or(0, |n| n.saturating_add(1))
}

pub fn has_shared_dependents(sheet_data: &Element, si: &str, master: CellRef) -> bool {
    shared_members(sheet_data, si)
        .into_iter()
        .any(|c| c != master)
}

/// 按主格到目标格的行列差平移 A1 引用。
pub fn shift_a1_formula(formula: &str, from: CellRef, to: CellRef) -> String {
    let dcol = to.col as i64 - from.col as i64;
    let drow = to.row as i64 - from.row as i64;
    if dcol == 0 && drow == 0 {
        return formula.to_string();
    }
    let bytes = formula.as_bytes();
    let mut i = 0;
    let mut out = String::with_capacity(formula.len() + 8);
    while i < bytes.len() {
        match bytes[i] {
            b'"' => copy_quoted(&mut out, formula, &mut i),
            b'\'' => copy_quoted_sheet(&mut out, formula, &mut i),
            b'[' => copy_brackets(&mut out, formula, &mut i),
            c if c.is_ascii_alphabetic() || c == b'$' => {
                if let Some((end, repl)) = try_take_refs(formula, i, dcol, drow) {
                    out.push_str(&repl);
                    i = end;
                } else {
                    let ch = formula[i..].chars().next().unwrap();
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
            _ => {
                let ch = formula[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    out
}

pub(crate) fn for_each_cell(sheet_data: &Element, mut visit: impl FnMut(CellRef, &Element)) {
    let mut prev_row = 0u32;
    for row in sheet_data.children_by_local("row") {
        let row_num = a1::row_after(prev_row, row.attr("r"));
        prev_row = row_num;
        let mut prev_col = 0u32;
        for c in row.children_by_local("c") {
            let place = a1::place_cell(row_num, prev_col, c.attr("r"));
            let cell = place.cell();
            prev_col = cell.col;
            visit(cell, c);
        }
    }
}

fn copy_quoted(out: &mut String, s: &str, i: &mut usize) {
    let bytes = s.as_bytes();
    out.push('"');
    *i += 1;
    while *i < bytes.len() {
        let ch = s[*i..].chars().next().unwrap();
        out.push(ch);
        *i += ch.len_utf8();
        if ch == '"' {
            if *i < bytes.len() && bytes[*i] == b'"' {
                out.push('"');
                *i += 1;
                continue;
            }
            break;
        }
    }
}

fn copy_quoted_sheet(out: &mut String, s: &str, i: &mut usize) {
    let bytes = s.as_bytes();
    out.push('\'');
    *i += 1;
    while *i < bytes.len() {
        let ch = s[*i..].chars().next().unwrap();
        out.push(ch);
        *i += ch.len_utf8();
        if ch == '\'' {
            if *i < bytes.len() && bytes[*i] == b'\'' {
                out.push('\'');
                *i += 1;
                continue;
            }
            break;
        }
    }
    if *i < bytes.len() && bytes[*i] == b'!' {
        out.push('!');
        *i += 1;
    }
}

fn copy_brackets(out: &mut String, s: &str, i: &mut usize) {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    while *i < bytes.len() {
        let b = bytes[*i];
        out.push(b as char);
        *i += 1;
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            b'"' => {
                *i -= 1;
                out.pop();
                copy_quoted(out, s, i);
            }
            _ => {}
        }
    }
}

fn try_take_refs(s: &str, start: usize, dcol: i64, drow: i64) -> Option<(usize, String)> {
    let mut j = start;
    if let Some(end) = sheet_prefix_end(s, j) {
        j = end;
    }
    let a1_start = j;
    let (cell1, j1) = a1::parse_abs_prefix(s, j)?;
    j = j1;
    if s[j..].starts_with('(') {
        return Some((j, s[start..j].to_string()));
    }
    if s[j..].starts_with(':') {
        if let Some((cell2, j2)) = a1::parse_abs_prefix(s, j + 1) {
            if !s[j2..].starts_with('(') {
                let prefix = &s[start..a1_start];
                let left = shift_token(cell1, dcol, drow);
                let right = shift_token(cell2, dcol, drow);
                return Some((j2, format!("{prefix}{left}:{right}")));
            }
        }
    }
    let prefix = &s[start..a1_start];
    let body = shift_token(cell1, dcol, drow);
    Some((j, format!("{prefix}{body}")))
}

fn shift_token(cell: AbsCellRef, dcol: i64, drow: i64) -> String {
    match a1::shift_abs(cell, dcol, drow) {
        Some(n) => a1::format_abs_cell(n),
        None => "#REF!".to_string(),
    }
}

/// 改表名或删表时改写公式里的本地表引用。
/// 字符串和外部工作簿引用原样留下。引号不配对时返回错误。
#[derive(Clone, Copy)]
pub(crate) enum SheetRefEdit<'a> {
    Rename(&'a str),
    Delete,
}

pub(crate) fn rewrite_sheet_formula(
    formula: &str,
    old: &str,
    edit: SheetRefEdit<'_>,
) -> Result<String, ()> {
    let bytes = formula.as_bytes();
    let mut i = 0;
    let mut out = String::with_capacity(formula.len());
    while i < bytes.len() {
        if bytes[i] == b'"' {
            copy_string_checked(&mut out, formula, &mut i)?;
            continue;
        }
        if bytes[i] == b'\'' {
            rewrite_quoted_prefix(&mut out, formula, &mut i, old, edit)?;
            continue;
        }
        if bytes[i] == b'[' && external_index_at(formula, i) {
            let start = i;
            let mut j = i;
            let mut skipped = String::new();
            copy_brackets(&mut skipped, formula, &mut j);
            if let Some(end) = sheet_prefix_span(formula, j) {
                j = end;
            }
            out.push_str(&formula[start..j]);
            i = j;
            continue;
        }
        if rewrite_unquoted_prefix(&mut out, formula, &mut i, old, edit)? {
            continue;
        }
        let ch = formula[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    Ok(out)
}

/// 删表时，把指向该表上表格的结构化引用换成 `#REF!`。
pub(crate) fn rewrite_table_formula(formula: &str, table: &str) -> Result<String, ()> {
    let bytes = formula.as_bytes();
    let mut i = 0;
    let mut out = String::with_capacity(formula.len());
    while i < bytes.len() {
        if bytes[i] == b'"' {
            copy_string_checked(&mut out, formula, &mut i)?;
            continue;
        }
        if let Some(end) = table_ref_end(formula, i, table) {
            out.push_str("#REF!");
            i = end;
            continue;
        }
        let ch = formula[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    Ok(out)
}

fn copy_string_checked(out: &mut String, s: &str, i: &mut usize) -> Result<(), ()> {
    let start = *i;
    copy_quoted(out, s, i);
    if *i <= start + 1 || !out.ends_with('"') {
        return Err(());
    }
    Ok(())
}

fn external_index_at(s: &str, i: usize) -> bool {
    let bytes = s.as_bytes();
    if i >= bytes.len() || bytes[i] != b'[' {
        return false;
    }
    let mut j = i + 1;
    if j >= bytes.len() || !bytes[j].is_ascii_digit() {
        return false;
    }
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    j < bytes.len() && bytes[j] == b']'
}

fn sheet_prefix_span(s: &str, i: usize) -> Option<usize> {
    if i >= s.len() {
        return None;
    }
    if s.as_bytes()[i] == b'\'' {
        return quoted_prefix_end(s, i);
    }
    unquoted_prefix_end(s, i)
}

fn quoted_prefix_end(s: &str, i: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if i >= bytes.len() || bytes[i] != b'\'' {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() {
        if bytes[j] == b'\'' {
            j += 1;
            if j < bytes.len() && bytes[j] == b'\'' {
                j += 1;
                continue;
            }
            if j < bytes.len() && bytes[j] == b'!' {
                return Some(j + 1);
            }
            return None;
        }
        j += 1;
    }
    None
}

fn unquoted_prefix_end(s: &str, i: usize) -> Option<usize> {
    let name1 = unquoted_name_end(s, i)?;
    let bytes = s.as_bytes();
    if name1 < bytes.len() && bytes[name1] == b'!' {
        return Some(name1 + 1);
    }
    if name1 < bytes.len() && bytes[name1] == b':' {
        let name2 = unquoted_name_end(s, name1 + 1)?;
        if name2 < bytes.len() && bytes[name2] == b'!' {
            return Some(name2 + 1);
        }
    }
    None
}

fn unquoted_name_end(s: &str, i: usize) -> Option<usize> {
    let mut j = i;
    let mut saw = false;
    for ch in s[i..].chars() {
        if is_unquoted_sheet_char(ch) {
            saw = true;
            j += ch.len_utf8();
        } else {
            break;
        }
    }
    saw.then_some(j)
}

fn is_unquoted_sheet_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

fn rewrite_quoted_prefix(
    out: &mut String,
    formula: &str,
    i: &mut usize,
    old: &str,
    edit: SheetRefEdit<'_>,
) -> Result<(), ()> {
    let Some(end) = quoted_prefix_end(formula, *i) else {
        return Err(());
    };
    let raw = &formula[*i..end];
    let inside = decode_quoted_sheet(&raw[1..raw.len() - 2])?;
    if inside.contains(']') {
        out.push_str(raw);
        *i = end;
        return Ok(());
    }
    let prefix = parse_local_names(&inside)?;
    emit_prefix(out, formula, i, end, &prefix, old, edit)
}

fn rewrite_unquoted_prefix(
    out: &mut String,
    formula: &str,
    i: &mut usize,
    old: &str,
    edit: SheetRefEdit<'_>,
) -> Result<bool, ()> {
    let Some(end) = unquoted_prefix_end(formula, *i) else {
        return Ok(false);
    };
    let raw = &formula[*i..end];
    let inside = &raw[..raw.len() - 1];
    let prefix = parse_local_names(inside)?;
    emit_prefix(out, formula, i, end, &prefix, old, edit)?;
    Ok(true)
}

fn decode_quoted_sheet(inside: &str) -> Result<String, ()> {
    let mut out = String::new();
    let mut chars = inside.chars();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.next() != Some('\'') {
                return Err(());
            }
            out.push('\'');
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn parse_local_names(inside: &str) -> Result<SheetPrefix, ()> {
    match inside.split_once(':') {
        None => Ok(SheetPrefix::One(inside.to_string())),
        Some((a, b)) => {
            if a.is_empty() || b.is_empty() || b.contains(':') {
                return Err(());
            }
            Ok(SheetPrefix::Three(a.to_string(), b.to_string()))
        }
    }
}

enum SheetPrefix {
    One(String),
    Three(String, String),
}

fn emit_prefix(
    out: &mut String,
    formula: &str,
    i: &mut usize,
    end: usize,
    prefix: &SheetPrefix,
    old: &str,
    edit: SheetRefEdit<'_>,
) -> Result<(), ()> {
    let raw = &formula[*i..end];
    match prefix {
        SheetPrefix::One(name) if name.eq_ignore_ascii_case(old) => match edit {
            SheetRefEdit::Rename(new_name) => out.push_str(&format_sheet_prefix(new_name)),
            SheetRefEdit::Delete => {
                out.push_str("#REF!");
                *i = consume_ref_tail(formula, end);
                return Ok(());
            }
        },
        SheetPrefix::Three(a, b) => {
            let hit_a = a.eq_ignore_ascii_case(old);
            let hit_b = b.eq_ignore_ascii_case(old);
            if hit_a || hit_b {
                match edit {
                    SheetRefEdit::Rename(new_name) => {
                        let left = if hit_a { new_name } else { a.as_str() };
                        let right = if hit_b { new_name } else { b.as_str() };
                        out.push_str(&format_sheet_3d(left, right));
                    }
                    SheetRefEdit::Delete => {
                        if hit_a && hit_b {
                            out.push_str("#REF!");
                            *i = consume_ref_tail(formula, end);
                            return Ok(());
                        }
                        let keep = if hit_a { b.as_str() } else { a.as_str() };
                        out.push_str(&format_sheet_prefix(keep));
                    }
                }
                *i = end;
                return Ok(());
            }
            out.push_str(raw);
        }
        _ => out.push_str(raw),
    }
    *i = end;
    Ok(())
}

fn format_sheet_prefix(name: &str) -> String {
    if sheet_name_needs_quotes(name) {
        format!("'{}'!", name.replace('\'', "''"))
    } else {
        format!("{name}!")
    }
}

fn format_sheet_3d(a: &str, b: &str) -> String {
    if sheet_name_needs_quotes(a) || sheet_name_needs_quotes(b) {
        format!("'{}:{}'!", a.replace('\'', "''"), b.replace('\'', "''"))
    } else {
        format!("{a}:{b}!")
    }
}

fn sheet_name_needs_quotes(name: &str) -> bool {
    if name.is_empty()
        || name.eq_ignore_ascii_case("true")
        || name.eq_ignore_ascii_case("false")
        || is_r1c1_sheet_name(name)
        || a1::parse_cell(name).is_ok()
    {
        return true;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    if !first.is_alphabetic() && first != '_' {
        return true;
    }
    !name.chars().all(is_unquoted_sheet_char)
}

fn is_r1c1_sheet_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 2 || !bytes[0].eq_ignore_ascii_case(&b'R') {
        return false;
    }
    let mut i = 1;
    i = skip_r1c1_index(bytes, i);
    if i >= bytes.len() || !bytes[i].eq_ignore_ascii_case(&b'C') {
        return false;
    }
    i = skip_r1c1_index(bytes, i + 1);
    i == bytes.len()
}

fn skip_r1c1_index(bytes: &[u8], mut i: usize) -> usize {
    if i < bytes.len() && bytes[i] == b'[' {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > start && i < bytes.len() && bytes[i] == b']' {
            return i + 1;
        }
        return start.saturating_sub(1);
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn consume_ref_tail(s: &str, i: usize) -> usize {
    let Some(end) = consume_ref_atom(s, i) else {
        return i;
    };
    if s[end..].starts_with(':') {
        if let Some(end2) = consume_ref_atom(s, end + 1) {
            return end2;
        }
    }
    end
}

fn consume_ref_atom(s: &str, i: usize) -> Option<usize> {
    if let Some((_, end)) = a1::parse_abs_prefix(s, i) {
        return Some(end);
    }
    if let Some(end) = consume_column(s, i) {
        let next = s[end..].chars().next();
        if !matches!(next, Some('(' | '[')) {
            return Some(end);
        }
    }
    if let Some(end) = consume_row_atom(s, i) {
        return Some(end);
    }
    consume_struct_name(s, i)
}

fn consume_column(s: &str, i: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut j = i;
    if j < bytes.len() && bytes[j] == b'$' {
        j += 1;
    }
    let start = j;
    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
        j += 1;
    }
    if j == start || j - start > 3 {
        return None;
    }
    let col = a1::letters_to_col(&s[start..j]).ok()?;
    (col > 0 && col <= 16_384).then_some(j)
}

fn consume_row_atom(s: &str, i: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut j = i;
    if j < bytes.len() && bytes[j] == b'$' {
        j += 1;
    }
    let start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == start {
        return None;
    }
    let row: u32 = s[start..j].parse().ok()?;
    (row > 0 && row <= 1_048_576).then_some(j)
}

fn consume_struct_name(s: &str, i: usize) -> Option<usize> {
    let end = unquoted_name_end(s, i)?;
    let bytes = s.as_bytes();
    if end < bytes.len() && bytes[end] == b'[' {
        return Some(consume_balanced_brackets(s, end));
    }
    if end < bytes.len() && bytes[end] == b'(' {
        return None;
    }
    Some(end)
}

fn consume_balanced_brackets(s: &str, i: usize) -> usize {
    let bytes = s.as_bytes();
    let mut j = i;
    let mut depth = 0i32;
    while j < bytes.len() {
        match bytes[j] {
            b'[' => {
                depth += 1;
                j += 1;
            }
            b']' => {
                depth -= 1;
                j += 1;
                if depth == 0 {
                    break;
                }
            }
            b'"' => {
                j += 1;
                while j < bytes.len() {
                    if bytes[j] == b'"' {
                        j += 1;
                        if j < bytes.len() && bytes[j] == b'"' {
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    j += 1;
                }
            }
            _ => {
                let ch = s[j..].chars().next().unwrap();
                j += ch.len_utf8();
            }
        }
    }
    j
}

fn table_ref_end(s: &str, i: usize, table: &str) -> Option<usize> {
    if i > 0 {
        let prev = s[..i].chars().next_back()?;
        if is_unquoted_sheet_char(prev) {
            return None;
        }
    }
    let end = i + table.len();
    if !s.is_char_boundary(end) {
        return None;
    }
    let rest = s.get(i..end)?;
    if !rest.eq_ignore_ascii_case(table) {
        return None;
    }
    if !s[end..].starts_with('[') {
        return None;
    }
    Some(consume_balanced_brackets(s, end))
}

fn sheet_prefix_end(s: &str, i: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'\'' {
        let mut j = i + 1;
        while j < bytes.len() {
            if bytes[j] == b'\'' {
                j += 1;
                if j < bytes.len() && bytes[j] == b'\'' {
                    j += 1;
                    continue;
                }
                if j < bytes.len() && bytes[j] == b'!' {
                    return Some(j + 1);
                }
                return None;
            }
            j += 1;
        }
        return None;
    }
    if !bytes[i].is_ascii_alphabetic() && bytes[i] != b'_' {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len()
        && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'.')
    {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b':' {
        let mut k = j + 1;
        if k >= bytes.len() {
            return None;
        }
        if !bytes[k].is_ascii_alphabetic() && bytes[k] != b'_' {
            return None;
        }
        k += 1;
        while k < bytes.len()
            && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_' || bytes[k] == b'.')
        {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == b'!' {
            return Some(k + 1);
        }
        return None;
    }
    if j < bytes.len() && bytes[j] == b'!' {
        return Some(j + 1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(col: u32, row: u32) -> CellRef {
        CellRef { col, row }
    }

    #[test]
    fn shift_relative_down() {
        assert_eq!(shift_a1_formula("A1*2", c(2, 1), c(2, 2)), "A2*2");
        assert_eq!(
            shift_a1_formula("SUM(A1:A10)", c(2, 1), c(2, 2)),
            "SUM(A2:A11)"
        );
    }

    #[test]
    fn shift_respects_absolute() {
        assert_eq!(shift_a1_formula("$A1+B$1", c(2, 1), c(3, 2)), "$A2+C$1");
    }

    #[test]
    fn shift_skips_function_and_string() {
        assert_eq!(shift_a1_formula("LOG10(A1)", c(2, 1), c(2, 2)), "LOG10(A2)");
        assert_eq!(shift_a1_formula("\"A1\"&B1", c(2, 1), c(2, 2)), "\"A1\"&B2");
    }

    #[test]
    fn shift_sheet_prefix() {
        assert_eq!(shift_a1_formula("Sheet1!A1", c(2, 1), c(2, 2)), "Sheet1!A2");
        assert_eq!(
            shift_a1_formula("'My Sheet'!A1", c(2, 1), c(2, 2)),
            "'My Sheet'!A2"
        );
    }

    #[test]
    fn shift_out_of_bounds_is_ref() {
        assert_eq!(shift_a1_formula("A1048576", c(1, 1), c(1, 2)), "#REF!");
    }

    #[test]
    fn parse_table_column_and_row() {
        assert_eq!(
            parse_table_formula("=TABLE(,C1)"),
            Some((None, Some(c(3, 1))))
        );
        assert_eq!(
            parse_table_formula("=TABLE(C10,)"),
            Some((Some(c(3, 10)), None))
        );
        assert_eq!(
            parse_table_formula("=TABLE(X1,Y1)"),
            Some((Some(c(24, 1)), Some(c(25, 1))))
        );
        assert!(parse_table_formula("=A1").is_none());
        assert!(parse_table_formula("=TABLE(C1)").is_none());
    }

    #[test]
    fn rename_and_delete_match_excel_sheet_refs() {
        let ren =
            |f: &str| rewrite_sheet_formula(f, "Src", SheetRefEdit::Rename("New Name")).unwrap();
        let del = |f: &str| rewrite_sheet_formula(f, "Src", SheetRefEdit::Delete).unwrap();
        assert_eq!(ren("Src!$B$1"), "'New Name'!$B$1");
        assert_eq!(
            ren("SUM(Src!B1,Src!B2)"),
            "SUM('New Name'!B1,'New Name'!B2)"
        );
        assert_eq!(ren("Src!B1:B2"), "'New Name'!B1:B2");
        assert_eq!(ren("\"Src!B1\""), "\"Src!B1\"");
        assert_eq!(ren("SUM(Src:Host!B1)"), "SUM('New Name:Host'!B1)");
        assert_eq!(ren("[1]Src!A1"), "[1]Src!A1");
        assert_eq!(ren("数据!$B$1"), "数据!$B$1");
        assert_eq!(ren("'Data 1'!$B$1"), "'Data 1'!$B$1");
        assert_eq!(
            rewrite_sheet_formula("'Data 1'!$B$1", "Data 1", SheetRefEdit::Rename("Data2"))
                .unwrap(),
            "Data2!$B$1"
        );
        assert_eq!(
            rewrite_sheet_formula("'1Data'!A1", "1Data", SheetRefEdit::Rename("2 Data")).unwrap(),
            "'2 Data'!A1"
        );
        assert_eq!(
            rewrite_sheet_formula("'O''Brien'!A1", "O'Brien", SheetRefEdit::Rename("O'Neil"))
                .unwrap(),
            "'O''Neil'!A1"
        );
        assert_eq!(
            rewrite_sheet_formula("'A1'!B1", "A1", SheetRefEdit::Rename("B2")).unwrap(),
            "'B2'!B1"
        );
        assert_eq!(
            rewrite_sheet_formula("数据!A1", "数据", SheetRefEdit::Rename("销售 表")).unwrap(),
            "'销售 表'!A1"
        );
        assert_eq!(
            rewrite_sheet_formula("'true'!A1", "true", SheetRefEdit::Rename("Src")).unwrap(),
            "Src!A1"
        );
        assert_eq!(del("Src!$B$1"), "#REF!");
        assert_eq!(del("SUM(Src!B1,Src!B2)"), "SUM(#REF!,#REF!)");
        assert_eq!(del("Src!B1:B2"), "#REF!");
        assert_eq!(del("Src!B1>0"), "#REF!>0");
        assert_eq!(del("\"Src!B1\""), "\"Src!B1\"");
        assert_eq!(del("SUM(Src:Host!B1)"), "SUM(Host!B1)");
        assert_eq!(del("SUM(Host:Src!B1)"), "SUM(Host!B1)");
        assert_eq!(del("[1]Src!A1"), "[1]Src!A1");
        assert_eq!(del("Src!A:A"), "#REF!");
        assert_eq!(del("Src!1:1"), "#REF!");
        assert_eq!(
            rewrite_table_formula("SUM(Tbl[Col])", "Tbl").unwrap(),
            "SUM(#REF!)"
        );
        assert_eq!(
            rewrite_table_formula("SUM(Tbl[[#This Row],[Col]])", "Tbl").unwrap(),
            "SUM(#REF!)"
        );
        assert_eq!(
            rewrite_table_formula("\"Tbl[Col]\"", "Tbl").unwrap(),
            "\"Tbl[Col]\""
        );
        assert!(rewrite_sheet_formula("'Src!A1", "Src", SheetRefEdit::Delete).is_err());
    }

    #[test]
    fn shown_table_matches_excel() {
        assert_eq!(shown_table_formula(false, false, "C1", None), "=TABLE(,C1)");
        assert_eq!(
            shown_table_formula(false, true, "C10", None),
            "=TABLE(C10,)"
        );
        assert_eq!(
            shown_table_formula(true, true, "X1", Some("Y1")),
            "=TABLE(X1,Y1)"
        );
    }
}
