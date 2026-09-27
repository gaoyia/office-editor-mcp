//! 共享公式与数组公式：inspect 索引、相对 A1 展开、apply 占用检查。
//!
//! 展开只平移公式里的 A1 单元格与 A1 区域（尊重 `$`）。这不是插行引用平移引擎。

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

fn for_each_cell(sheet_data: &Element, mut visit: impl FnMut(CellRef, &Element)) {
    for row in sheet_data.children_by_local("row") {
        for c in row.children_by_local("c") {
            let Some(r) = c.attr("r") else {
                continue;
            };
            let Ok(cell) = a1::parse_cell(r) else {
                continue;
            };
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
}
