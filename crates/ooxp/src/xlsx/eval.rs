//! 单元格公式缓存。算不了的不写 `<v>`。
//! 另外写数据表结果、本机外部引用、图表连续区域缓存，以及只有行字段时的透视合计。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::a1::{self, CellRef};
use super::dbcs;
use super::formula;
use super::inspect;
use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

const DIV0: &str = "#DIV/0!";
const VALUE: &str = "#VALUE!";
const REF: &str = "#REF!";
const NAME: &str = "#NAME?";
const NUM: &str = "#NUM!";
const NA: &str = "#N/A";
const NULL: &str = "#NULL!";

#[derive(Clone, Debug)]
enum Scalar {
    Num(f64),
    Text(String),
    Bool(bool),
    Err(&'static str),
}

#[derive(Clone, Debug)]
enum Loc {
    Here,
    At(usize),
    Name(String),
}

#[derive(Clone, Debug)]
struct Rect {
    from: Loc,
    to: Loc,
    r1: CellRef,
    r2: CellRef,
}

#[derive(Clone, Debug)]
enum Value {
    Scalar(Scalar),
    Area(Rect),
    Areas(Vec<Rect>),
    Grid(Vec<Vec<Scalar>>),
}

#[derive(Clone, Debug)]
enum Expr {
    Value(Value),
    Name(String),
    TableCol {
        table: String,
        col: String,
    },
    Call(String, Vec<Expr>),
    External {
        index: usize,
        sheet: String,
        r1: CellRef,
        r2: CellRef,
    },
    Neg(Box<Expr>),
    Pct(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    Cross(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Cat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy)]
enum Mode {
    Scalar,
    Array,
}

enum Calc {
    Ready(Value),
    Skip,
}

struct CellRec {
    formula: Option<String>,
    cached: Option<Scalar>,
    /// 数组从格指向主格。
    array_master: Option<CellRef>,
    array_ref: Option<(CellRef, CellRef)>,
}

struct Sheet {
    name: String,
    cells: HashMap<(u32, u32), CellRec>,
    /// `sheetView` 的 `topLeftCell`。没有时是 A1。
    origin: CellRef,
    /// `row/@hidden="1"` 的行号。
    hidden_rows: HashSet<u32>,
}

struct NameDef {
    name: String,
    local: Option<usize>,
    body: String,
}

struct TableDef {
    name: String,
    sheet: usize,
    start: CellRef,
    columns: Vec<String>,
    data_row: u32,
    data_end: u32,
}

struct ExtBook {
    sheets: Vec<Sheet>,
    /// 外部链 `sheetNames` 里的序号，从 0 起。文件里有、名单里没有的表不进这里。
    sheet_ids: HashMap<String, u32>,
}

struct DataTableRec {
    sheet: usize,
    start: CellRef,
    end: CellRef,
    dt2d: bool,
    dtr: bool,
    r1: CellRef,
    r2: Option<CellRef>,
}

struct Book {
    sheets: Vec<Sheet>,
    names: Vec<NameDef>,
    tables: Vec<TableDef>,
    date1904: bool,
    memo: HashMap<(usize, u32, u32), MemoSlot>,
    stack: Vec<(usize, u32, u32)>,
    overrides: HashMap<(usize, u32, u32), Scalar>,
    cyclic: HashSet<(usize, u32, u32)>,
    /// `CELL` / `INFO` 写出结果的格子。公式写 `ca="1"`。
    volatile: HashSet<(usize, u32, u32)>,
    /// `calcPr` 的 `calcMode="manual"`。
    manual: bool,
    /// 下标 0 空着，`[1]` 用下标 1。
    externals: Vec<Option<ExtBook>>,
    /// `(链序号, sheetId, 列, 行)`。
    ext_used: HashMap<(usize, u32, u32, u32), Scalar>,
}

enum MemoSlot {
    Ready(Scalar),
    Skip,
}

struct Lex<'a> {
    s: &'a str,
    i: usize,
    space: bool,
}

#[derive(Clone, Debug)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Err(&'static str),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Amp,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Pct,
    LParen,
    RParen,
    Comma,
    Colon,
    LBrace,
    RBrace,
    LBrack,
    RBrack,
    Bang,
    Semi,
}

/// 重新打开并写入缓存。没有写出新缓存时保留原来的字节，避免无公式文件被重打包。
pub(crate) fn fill_saved(bytes: Vec<u8>, base_dir: Option<&Path>) -> Result<Vec<u8>, Error> {
    let original = bytes.clone();
    let mut pkg = Package::open_bytes(bytes)?;
    if fill_formula_caches(&mut pkg, base_dir)? {
        pkg.save_to_vec()
    } else {
        Ok(original)
    }
}

/// 写缓存。没有任何新缓存时返回 `false`。
pub(crate) fn fill_formula_caches(
    pkg: &mut Package,
    base_dir: Option<&Path>,
) -> Result<bool, Error> {
    fill_formula_caches_depth(pkg, base_dir, 1)
}

fn fill_formula_caches_depth(
    pkg: &mut Package,
    base_dir: Option<&Path>,
    external_depth: u8,
) -> Result<bool, Error> {
    let wb_part = opc::office_document_part(pkg)?;
    let wb_rels = Relationships::from_package(pkg, &wb_part)?;
    let wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    let date1904 = wb_doc
        .root
        .child_by_local("workbookPr")
        .and_then(|e| e.attr("date1904"))
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let manual = wb_doc
        .root
        .child_by_local("calcPr")
        .and_then(|e| e.attr("calcMode"))
        .is_some_and(|v| v.eq_ignore_ascii_case("manual"));
    let sst = inspect::load_shared_strings(pkg, &wb_part, &wb_rels)?;
    let listed = sheet_list(&wb_doc.root, &wb_rels, &wb_part)?;
    let mut sheets = Vec::new();
    let mut sheet_docs = Vec::new();
    for (name, part, kind) in &listed {
        if *kind != "worksheet" {
            continue;
        }
        let doc = xml::parse(pkg.get(part)?)?;
        let rec = load_sheet(&doc.root, &sst);
        sheets.push(Sheet {
            name: name.clone(),
            cells: rec,
            origin: sheet_origin(&doc.root),
            hidden_rows: hidden_rows(&doc.root),
        });
        sheet_docs.push((part.clone(), doc));
    }
    let names = load_names(&wb_doc.root);
    let tables = load_tables(pkg, &sheet_docs, &sheets)?;
    let data_tables = load_data_tables(&sheet_docs);
    let (externals, ext_parts) = if external_depth == 0 {
        (vec![None], vec![String::new()])
    } else {
        load_externals(pkg, &wb_part, base_dir)?
    };
    let mut book = Book {
        sheets,
        names,
        tables,
        date1904,
        memo: HashMap::new(),
        stack: Vec::new(),
        overrides: HashMap::new(),
        cyclic: HashSet::new(),
        volatile: HashSet::new(),
        manual,
        externals,
        ext_used: HashMap::new(),
    };
    let mut writes: Vec<HashMap<(u32, u32), Scalar>> = Vec::new();
    for si in 0..book.sheets.len() {
        let keys: Vec<(u32, u32)> = book.sheets[si].cells.keys().copied().collect();
        let mut out = HashMap::new();
        for key in keys {
            let rec = &book.sheets[si].cells[&key];
            let interesting = rec.formula.is_some() || rec.array_master.is_some();
            if !interesting {
                continue;
            }
            match book.scalar_at(
                si,
                CellRef {
                    col: key.0,
                    row: key.1,
                },
            ) {
                Calc::Ready(Value::Scalar(s)) => {
                    out.insert(key, s);
                }
                Calc::Ready(_) | Calc::Skip => {}
            }
        }
        writes.push(out);
    }
    for (si, key, val) in book.eval_data_tables(&data_tables) {
        writes[si].insert(key, val);
    }
    let mut cyclic_sheets = vec![HashSet::new(); writes.len()];
    for (si, col, row) in &book.cyclic {
        if let Some(set) = cyclic_sheets.get_mut(*si) {
            set.insert((*col, *row));
        }
    }
    let mut volatile_sheets = vec![HashSet::new(); writes.len()];
    for (si, col, row) in &book.volatile {
        if let Some(set) = volatile_sheets.get_mut(*si) {
            set.insert((*col, *row));
        }
    }
    let ext_used = book.ext_used.clone();
    let mut dirty = vec![false; sheet_docs.len()];
    for (doc_i, (_part, doc)) in sheet_docs.iter_mut().enumerate() {
        let Some(data) = doc.root.child_by_local_mut("sheetData") else {
            continue;
        };
        let map = &writes[doc_i];
        let cycles = &cyclic_sheets[doc_i];
        let volatile = &volatile_sheets[doc_i];
        if map.is_empty() && cycles.is_empty() && volatile.is_empty() {
            continue;
        }
        formula::for_each_cell_mut(data, |cell, el| {
            let key = (cell.col, cell.row);
            if let Some(val) = map.get(&key) {
                write_cache(el, val);
                dirty[doc_i] = true;
            }
            if cycles.contains(&key) || volatile.contains(&key) {
                if let Some(f) = el.child_by_local_mut("f") {
                    if f.attr("ca") != Some("1") {
                        f.set_attr("ca", "1");
                        dirty[doc_i] = true;
                    }
                }
            }
        });
    }
    let sheet_names: Vec<String> = sheet_docs
        .iter()
        .enumerate()
        .map(|(i, _)| {
            book.sheets
                .get(i)
                .map(|s| s.name.clone())
                .unwrap_or_default()
        })
        .collect();
    for host in super::pivot::fill_row_sum_display(pkg, &mut sheet_docs, &sheet_names, &sst)? {
        if let Some(flag) = dirty.get_mut(host) {
            *flag = true;
        }
    }
    let charts = refresh_chart_caches(pkg, &sheet_docs, &sheet_names, &sst)?;
    let links = write_external_caches(pkg, &ext_parts, &ext_used)?;
    let mut changed = charts || links || dirty.iter().any(|d| *d);
    for (i, (part, doc)) in sheet_docs.into_iter().enumerate() {
        if !dirty[i] {
            continue;
        }
        pkg.set(&part, doc.to_bytes()?)?;
        changed = true;
    }
    Ok(changed)
}

fn sheet_list(
    workbook: &Element,
    rels: &Relationships,
    wb_part: &str,
) -> Result<Vec<(String, String, &'static str)>, Error> {
    let sheets = workbook.child_by_local("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    let mut out = Vec::new();
    for sh in sheets.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        let rid = sh.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "sheet".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Unspecified,
                rid: rid.to_string(),
            })
        })?;
        let kind = inspect::sheet_kind(&rel.rel_type);
        let part = opc::resolve_rel_target(wb_part, &rel.target)?;
        out.push((name, part, kind));
    }
    Ok(out)
}

fn hidden_rows(root: &Element) -> HashSet<u32> {
    let mut rows = HashSet::new();
    let Some(data) = root.child_by_local("sheetData") else {
        return rows;
    };
    for row in data.children_by_local("row") {
        let hidden = row
            .attr("hidden")
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if !hidden {
            continue;
        }
        if let Some(n) = row.attr("r").and_then(|s| s.parse().ok()) {
            rows.insert(n);
        }
    }
    rows
}

fn sheet_origin(root: &Element) -> CellRef {
    super::view::first_sheet_view(root)
        .and_then(|view| view.attr("topLeftCell"))
        .and_then(|raw| a1::parse_cell(raw).ok())
        .unwrap_or(CellRef { col: 1, row: 1 })
}

fn load_sheet(root: &Element, sst: &[String]) -> HashMap<(u32, u32), CellRec> {
    let Some(data) = root.child_by_local("sheetData") else {
        return HashMap::new();
    };
    let index = formula::index_sheet(data);
    let mut cells = HashMap::new();
    formula::for_each_cell(data, |here, el| {
        let cached = read_cached(el, sst);
        let mut formula = None;
        let mut array_master = None;
        let mut array_ref = None;
        if let Some(f) = el.child_by_local("f") {
            let kind = f.attr("t").unwrap_or("");
            let body = f.text_content().trim().trim_start_matches('=').to_string();
            if kind == "dataTable" {
                // 结果由数据表求值另写，主格公式体是空的。
            } else if kind == "shared" && body.is_empty() {
                if let Some(si) = f.attr("si") {
                    if let Some(group) = index.shared.get(si) {
                        formula = Some(formula::shift_a1_formula(
                            &group.formula,
                            group.master,
                            here,
                        ));
                    }
                }
            } else if kind == "array" {
                array_ref = f.attr("ref").and_then(|r| a1::parse_range(r).ok());
                if !body.is_empty() {
                    formula = Some(body);
                }
            } else if !body.is_empty() && !body.to_ascii_uppercase().starts_with("TABLE") {
                formula = Some(body);
            }
        } else if let Some(formula::Occupied::Array { master }) = formula::occupy_at(&index, here) {
            if master != here {
                array_master = Some(master);
            }
        }
        cells.insert(
            (here.col, here.row),
            CellRec {
                formula,
                cached,
                array_master,
                array_ref,
            },
        );
    });
    cells
}

fn read_cached(el: &Element, sst: &[String]) -> Option<Scalar> {
    let t = el.attr("t").unwrap_or("");
    let raw = el.child_by_local("v").map(|v| v.text_content());
    match t {
        "b" => Some(Scalar::Bool(raw.as_deref() == Some("1"))),
        "e" => Some(Scalar::Err(canonical_err(raw.as_deref().unwrap_or("")))),
        "str" | "inlineStr" => Some(Scalar::Text(
            el.child_by_local("is")
                .map(|e| e.text_content())
                .or(raw)
                .unwrap_or_default(),
        )),
        "s" => {
            let idx = raw?.trim().parse::<usize>().ok()?;
            Some(Scalar::Text(sst.get(idx).cloned().unwrap_or_default()))
        }
        _ => {
            let raw = raw?;
            if let Ok(n) = raw.trim().parse::<f64>() {
                Some(Scalar::Num(n))
            } else if raw.is_empty() {
                None
            } else {
                Some(Scalar::Text(raw))
            }
        }
    }
}

fn canonical_err(raw: &str) -> &'static str {
    match raw.trim() {
        "#DIV/0!" => DIV0,
        "#VALUE!" => VALUE,
        "#REF!" => REF,
        "#NAME?" => NAME,
        "#NUM!" => NUM,
        "#N/A" => NA,
        "#NULL!" => NULL,
        _ => VALUE,
    }
}

fn load_names(workbook: &Element) -> Vec<NameDef> {
    let Some(names) = workbook.child_by_local("definedNames") else {
        return Vec::new();
    };
    names
        .children_by_local("definedName")
        .into_iter()
        .filter_map(|el| {
            let name = el.attr("name")?.to_string();
            let local = el.attr("localSheetId").and_then(|s| s.parse().ok());
            let body = el.text_content().trim().trim_start_matches('=').to_string();
            if body.is_empty() {
                return None;
            }
            Some(NameDef { name, local, body })
        })
        .collect()
}

fn load_tables(
    pkg: &Package,
    docs: &[(String, xml::Document)],
    sheets: &[Sheet],
) -> Result<Vec<TableDef>, Error> {
    let mut out = Vec::new();
    for (si, (part, _)) in docs.iter().enumerate() {
        let rels = Relationships::from_package(pkg, part)?;
        for rel in &rels.rels {
            if !rel.rel_type.ends_with("/table") || rel.is_external() {
                continue;
            }
            let tpart = opc::resolve_rel_target(part, &rel.target)?;
            let Ok(bytes) = pkg.get(&tpart) else {
                continue;
            };
            let doc = xml::parse(bytes)?;
            let Some(def) = table_def(&doc.root, si) else {
                continue;
            };
            let _ = sheets;
            out.push(def);
        }
    }
    Ok(out)
}

fn table_def(root: &Element, sheet: usize) -> Option<TableDef> {
    let name = root
        .attr("displayName")
        .or_else(|| root.attr("name"))?
        .to_string();
    let (start, end) = a1::parse_range(root.attr("ref")?).ok()?;
    let mut columns = Vec::new();
    if let Some(cols) = root.child_by_local("tableColumns") {
        for col in cols.children_by_local("tableColumn") {
            columns.push(col.attr("name").unwrap_or("").to_string());
        }
    }
    let totals = root
        .attr("totalsRowCount")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let header = root
        .attr("headerRowCount")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let data_row = start.row + header;
    let data_end = end.row.saturating_sub(totals);
    Some(TableDef {
        name,
        sheet,
        start,
        columns,
        data_row,
        data_end,
    })
}

enum Fact {
    Blank,
    Ready(Scalar),
    Skip,
}

enum Hit {
    Fact(Fact),
    Err(&'static str),
}

enum SumX {
    X2My2,
    X2Py2,
    Xmy2,
}

enum Mat {
    Ok(Vec<Vec<f64>>),
    Err(&'static str),
    Skip,
}

impl Book {
    fn scalar_at(&mut self, sheet: usize, here: CellRef) -> Calc {
        let key = (sheet, here.col, here.row);
        if let Some(slot) = self.memo.get(&key) {
            return match slot {
                MemoSlot::Ready(s) => Calc::Ready(Value::Scalar(s.clone())),
                MemoSlot::Skip => Calc::Skip,
            };
        }
        if let Some(s) = self.overrides.get(&key) {
            return Calc::Ready(Value::Scalar(s.clone()));
        }
        if self.stack.contains(&key) {
            for frame in &self.stack {
                self.cyclic.insert(*frame);
            }
            self.cyclic.insert(key);
            return Calc::Ready(Value::Scalar(Scalar::Err(VALUE)));
        }
        let rec = match self
            .sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(here.col, here.row)))
        {
            Some(rec) => rec.clone_rec(),
            None => return Calc::Ready(Value::Scalar(Scalar::Num(0.0))),
        };
        self.stack.push(key);
        let calc = if let Some(master) = rec.array_master {
            self.eval_array_master(sheet, master);
            self.memo
                .get(&key)
                .map(|slot| match slot {
                    MemoSlot::Ready(s) => Calc::Ready(Value::Scalar(s.clone())),
                    MemoSlot::Skip => Calc::Skip,
                })
                .unwrap_or(Calc::Skip)
        } else if let Some(body) = rec.formula.clone() {
            if rec.array_ref.is_some() {
                self.eval_array_master(sheet, here);
                self.memo
                    .get(&key)
                    .map(|slot| match slot {
                        MemoSlot::Ready(s) => Calc::Ready(Value::Scalar(s.clone())),
                        MemoSlot::Skip => Calc::Skip,
                    })
                    .unwrap_or(Calc::Skip)
            } else {
                match parse_expr(&body) {
                    None => Calc::Skip,
                    Some(expr) => match self.eval_expr(&expr, sheet, here, Mode::Scalar) {
                        Calc::Ready(v) => self.coerce(v, sheet, here, Mode::Scalar),
                        Calc::Skip => Calc::Skip,
                    },
                }
            }
        } else if let Some(cached) = rec.cached.clone() {
            Calc::Ready(Value::Scalar(cached))
        } else {
            Calc::Ready(Value::Scalar(Scalar::Num(0.0)))
        };
        self.stack.pop();
        match &calc {
            Calc::Ready(Value::Scalar(s)) => {
                self.memo.insert(key, MemoSlot::Ready(s.clone()));
            }
            Calc::Skip => {
                self.memo.insert(key, MemoSlot::Skip);
            }
            Calc::Ready(_) => {}
        }
        calc
    }

    fn eval_array_master(&mut self, sheet: usize, master: CellRef) {
        let key = (sheet, master.col, master.row);
        if self.memo.contains_key(&key) {
            return;
        }
        let Some(rec) = self
            .sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(master.col, master.row)))
            .map(|r| r.clone_rec())
        else {
            return;
        };
        let Some(body) = rec.formula.clone() else {
            self.memo.insert(key, MemoSlot::Skip);
            return;
        };
        let Some((start, end)) = rec.array_ref else {
            self.memo.insert(key, MemoSlot::Skip);
            return;
        };
        self.stack.push(key);
        let grid = match parse_expr(&body) {
            None => None,
            Some(expr) => match self.eval_expr(&expr, sheet, master, Mode::Array) {
                Calc::Ready(v) => Some(self.as_grid(v, sheet, master)),
                Calc::Skip => None,
            },
        };
        self.stack.pop();
        let Some(grid) = grid else {
            self.memo.insert(key, MemoSlot::Skip);
            return;
        };
        let (r1, r2) = ordered(start, end);
        for row in r1.row..=r2.row {
            for col in r1.col..=r2.col {
                let gr = (row - r1.row) as usize;
                let gc = (col - r1.col) as usize;
                let scalar = grid
                    .get(gr)
                    .and_then(|r| r.get(gc))
                    .cloned()
                    .unwrap_or(Scalar::Err(NA));
                self.memo.insert((sheet, col, row), MemoSlot::Ready(scalar));
            }
        }
    }

    fn as_grid(&mut self, value: Value, sheet: usize, _here: CellRef) -> Vec<Vec<Scalar>> {
        match value {
            Value::Grid(g) => g,
            Value::Scalar(s) => vec![vec![s]],
            Value::Area(rect) => self.grid_from_rect(&rect, sheet),
            Value::Areas(parts) => {
                if parts.len() == 1 {
                    self.grid_from_rect(&parts[0], sheet)
                } else {
                    vec![vec![Scalar::Err(VALUE)]]
                }
            }
        }
    }

    fn resolve_span(&self, rect: &Rect, here: usize) -> Option<(usize, usize)> {
        let a = self.resolve_loc(&rect.from, here)?;
        let b = self.resolve_loc(&rect.to, here)?;
        Some(ord_pair(a, b))
    }

    fn resolve_loc(&self, loc: &Loc, here: usize) -> Option<usize> {
        match loc {
            Loc::Here => Some(here),
            Loc::At(i) => Some(*i),
            Loc::Name(n) => self
                .sheets
                .iter()
                .position(|s| s.name.eq_ignore_ascii_case(n)),
        }
    }

    fn grid_from_rect(&mut self, rect: &Rect, sheet: usize) -> Vec<Vec<Scalar>> {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return vec![vec![Scalar::Err(REF)]];
        };
        if s1 != s2 {
            return vec![vec![Scalar::Err(VALUE)]];
        }
        let sheet = s1;
        let mut rows = Vec::new();
        for row in r1.row..=r2.row {
            let mut cols = Vec::new();
            for col in r1.col..=r2.col {
                cols.push(self.cell_scalar(sheet, CellRef { col, row }));
            }
            rows.push(cols);
        }
        rows
    }

    fn eval_data_tables(&mut self, tables: &[DataTableRec]) -> Vec<(usize, (u32, u32), Scalar)> {
        let mut out = Vec::new();
        for table in tables {
            let (r1, r2) = ordered(table.start, table.end);
            if table.dt2d {
                let Some(col_input) = table.r2 else {
                    continue;
                };
                let Some(formula) = outside_corner(r1) else {
                    continue;
                };
                if !self.has_formula(table.sheet, formula) {
                    continue;
                }
                for row in r1.row..=r2.row {
                    for col in r1.col..=r2.col {
                        let above = self.plain(
                            table.sheet,
                            CellRef {
                                col,
                                row: r1.row - 1,
                            },
                        );
                        let left = self.plain(
                            table.sheet,
                            CellRef {
                                col: r1.col - 1,
                                row,
                            },
                        );
                        if let Some(val) = self.subst_eval(
                            table.sheet,
                            formula,
                            &[(table.r1, above), (col_input, left)],
                        ) {
                            out.push((table.sheet, (col, row), val));
                        }
                    }
                }
            } else if table.dtr {
                if r1.col == 1 || r1.row == 1 {
                    continue;
                }
                for row in r1.row..=r2.row {
                    let formula = CellRef {
                        col: r1.col - 1,
                        row,
                    };
                    if !self.has_formula(table.sheet, formula) {
                        continue;
                    }
                    for col in r1.col..=r2.col {
                        let above = self.plain(
                            table.sheet,
                            CellRef {
                                col,
                                row: r1.row - 1,
                            },
                        );
                        if let Some(val) =
                            self.subst_eval(table.sheet, formula, &[(table.r1, above)])
                        {
                            out.push((table.sheet, (col, row), val));
                        }
                    }
                }
            } else {
                if r1.col == 1 || r1.row == 1 {
                    continue;
                }
                for col in r1.col..=r2.col {
                    let formula = CellRef {
                        col,
                        row: r1.row - 1,
                    };
                    if !self.has_formula(table.sheet, formula) {
                        continue;
                    }
                    for row in r1.row..=r2.row {
                        let left = self.plain(
                            table.sheet,
                            CellRef {
                                col: r1.col - 1,
                                row,
                            },
                        );
                        if let Some(val) =
                            self.subst_eval(table.sheet, formula, &[(table.r1, left)])
                        {
                            out.push((table.sheet, (col, row), val));
                        }
                    }
                }
            }
        }
        out
    }

    fn has_formula(&self, sheet: usize, cell: CellRef) -> bool {
        self.sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(cell.col, cell.row)))
            .and_then(|c| c.formula.as_ref())
            .is_some()
    }

    fn plain(&mut self, sheet: usize, cell: CellRef) -> Scalar {
        match self.scalar_at(sheet, cell) {
            Calc::Ready(Value::Scalar(s)) => s,
            _ => Scalar::Num(0.0),
        }
    }

    fn subst_eval(
        &mut self,
        sheet: usize,
        formula: CellRef,
        overs: &[(CellRef, Scalar)],
    ) -> Option<Scalar> {
        let saved_memo = std::mem::take(&mut self.memo);
        let saved_over = std::mem::take(&mut self.overrides);
        for (cell, val) in overs {
            self.overrides
                .insert((sheet, cell.col, cell.row), val.clone());
        }
        let got = match self.scalar_at(sheet, formula) {
            Calc::Ready(Value::Scalar(s)) => Some(s),
            _ => None,
        };
        self.memo = saved_memo;
        self.overrides = saved_over;
        got
    }

    fn external_value(
        &mut self,
        index: usize,
        sheet: &str,
        r1: CellRef,
        r2: CellRef,
    ) -> Option<Value> {
        let (a, b) = ordered(r1, r2);
        let (sheet_id, grid) = {
            let book = self.externals.get(index)?.as_ref()?;
            let si = book
                .sheets
                .iter()
                .position(|s| s.name.eq_ignore_ascii_case(sheet))?;
            let sheet_id = book.sheet_ids.get(&sheet.to_ascii_lowercase()).copied();
            let mut grid = Vec::new();
            for row in a.row..=b.row {
                let mut line = Vec::new();
                for col in a.col..=b.col {
                    let scalar = book.sheets[si]
                        .cells
                        .get(&(col, row))
                        .and_then(|c| c.cached.clone())
                        .unwrap_or(Scalar::Num(0.0));
                    line.push(scalar);
                }
                grid.push(line);
            }
            (sheet_id, grid)
        };
        if let Some(id) = sheet_id {
            for (row_i, line) in grid.iter().enumerate() {
                for (col_i, scalar) in line.iter().enumerate() {
                    self.ext_used.insert(
                        (index, id, a.col + col_i as u32, a.row + row_i as u32),
                        scalar.clone(),
                    );
                }
            }
        }
        if grid.len() == 1 && grid[0].len() == 1 {
            Some(Value::Scalar(grid[0][0].clone()))
        } else {
            Some(Value::Grid(grid))
        }
    }

    fn cell_scalar(&mut self, sheet: usize, here: CellRef) -> Scalar {
        match self.scalar_at(sheet, here) {
            Calc::Ready(Value::Scalar(s)) => s,
            Calc::Ready(_) => Scalar::Err(VALUE),
            Calc::Skip => Scalar::Err(VALUE),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, sheet: usize, here: CellRef, mode: Mode) -> Calc {
        match expr {
            Expr::Value(v) => Calc::Ready(v.clone()),
            Expr::External {
                index,
                sheet,
                r1,
                r2,
            } => match self.external_value(*index, sheet, *r1, *r2) {
                Some(v) => Calc::Ready(v),
                None => Calc::Skip,
            },
            Expr::Name(name) => self.eval_name(name, sheet, here, mode),
            Expr::TableCol { table, col } => Calc::Ready(self.table_area(table, col)),
            Expr::Neg(inner) => self.map_scalar(inner, sheet, here, mode, |s| match s {
                Scalar::Num(n) => Scalar::Num(-n),
                Scalar::Bool(b) => Scalar::Num(if b { -1.0 } else { 0.0 }),
                Scalar::Err(e) => Scalar::Err(e),
                Scalar::Text(_) => Scalar::Err(VALUE),
            }),
            Expr::Pct(inner) => self.map_scalar(inner, sheet, here, mode, |s| match s {
                Scalar::Num(n) => Scalar::Num(n / 100.0),
                Scalar::Bool(b) => Scalar::Num(if b { 0.01 } else { 0.0 }),
                Scalar::Err(e) => Scalar::Err(e),
                Scalar::Text(_) => Scalar::Err(VALUE),
            }),
            Expr::Bin(op, l, r) => self.eval_bin(*op, l, r, sheet, here, mode),
            Expr::Union(l, r) => match (
                self.eval_expr(l, sheet, here, mode),
                self.eval_expr(r, sheet, here, mode),
            ) {
                (Calc::Skip, _) | (_, Calc::Skip) => Calc::Skip,
                (Calc::Ready(a), Calc::Ready(b)) => Calc::Ready(union_value(a, b)),
            },
            Expr::Cross(l, r) => match (
                self.eval_expr(l, sheet, here, mode),
                self.eval_expr(r, sheet, here, mode),
            ) {
                (Calc::Skip, _) | (_, Calc::Skip) => Calc::Skip,
                (Calc::Ready(a), Calc::Ready(b)) => Calc::Ready(self.cross_value(a, b, sheet)),
            },
            Expr::Call(name, args) => self.eval_call(name, args, sheet, here, mode),
        }
    }

    fn map_scalar(
        &mut self,
        inner: &Expr,
        sheet: usize,
        here: CellRef,
        mode: Mode,
        f: impl Fn(Scalar) -> Scalar,
    ) -> Calc {
        match self.eval_expr(inner, sheet, here, mode) {
            Calc::Skip => Calc::Skip,
            Calc::Ready(v) => match self.coerce(v, sheet, here, mode) {
                Calc::Skip => Calc::Skip,
                Calc::Ready(Value::Scalar(s)) => Calc::Ready(Value::Scalar(f(s))),
                Calc::Ready(Value::Grid(g)) => Calc::Ready(Value::Grid(
                    g.into_iter()
                        .map(|row| row.into_iter().map(&f).collect())
                        .collect(),
                )),
                Calc::Ready(_) => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
            },
        }
    }

    fn eval_name(&mut self, name: &str, sheet: usize, here: CellRef, mode: Mode) -> Calc {
        let body = self
            .names
            .iter()
            .find(|n| n.local == Some(sheet) && n.name.eq_ignore_ascii_case(name))
            .or_else(|| {
                self.names
                    .iter()
                    .find(|n| n.local.is_none() && n.name.eq_ignore_ascii_case(name))
            })
            .map(|n| n.body.clone());
        let Some(def) = body else {
            return Calc::Ready(Value::Scalar(Scalar::Err(NAME)));
        };
        match parse_expr(&def) {
            None => Calc::Skip,
            Some(expr) => self.eval_expr(&expr, sheet, here, mode),
        }
    }

    fn table_area(&self, table: &str, col: &str) -> Value {
        let Some(def) = self
            .tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(table))
        else {
            return Value::Scalar(Scalar::Err(NAME));
        };
        let Some(idx) = def.columns.iter().position(|c| c.eq_ignore_ascii_case(col)) else {
            return Value::Scalar(Scalar::Err(REF));
        };
        if def.data_row > def.data_end {
            return Value::Scalar(Scalar::Num(0.0));
        }
        let col = def.start.col + idx as u32;
        Value::Area(Rect {
            from: Loc::At(def.sheet),
            to: Loc::At(def.sheet),
            r1: CellRef {
                col,
                row: def.data_row,
            },
            r2: CellRef {
                col,
                row: def.data_end,
            },
        })
    }

    fn eval_call(
        &mut self,
        name: &str,
        args: &[Expr],
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Calc {
        let upper = name.to_ascii_uppercase();
        match upper.as_str() {
            "NA" => return Calc::Ready(Value::Scalar(Scalar::Err(NA))),
            "NOW" => {
                return Calc::Ready(Value::Scalar(Scalar::Num(now_serial(self.date1904))));
            }
            // 逻辑类。ECMA. / ISO. 是函数名的一部分，不是可以套到 IF 上的前缀。
            "TRUE" => return logic_const(args, true),
            "FALSE" => return logic_const(args, false),
            "IF" => return self.eval_if(args, sheet, here, mode),
            "IFERROR" => return self.eval_iferror(args, sheet, here, mode),
            "NOT" => return self.eval_not(args, sheet, here, mode),
            "AND" => return self.eval_and_or(args, sheet, here, mode, true),
            "OR" => return self.eval_and_or(args, sheet, here, mode, false),
            // 信息类。`ERROR.TYPE` 的点是名字的一部分。
            "ISBLANK" => return self.eval_isblank(args, sheet, here),
            "ISERR" => return self.eval_iserr(args, sheet, here, false),
            "ISERROR" => return self.eval_iserr(args, sheet, here, true),
            "ISEVEN" => return self.eval_even_odd(args, sheet, here, true),
            "ISLOGICAL" => return self.eval_islogical(args, sheet, here),
            "ISNA" => return self.eval_isna(args, sheet, here),
            "ISNONTEXT" => return self.eval_isnontext(args, sheet, here),
            "ISNUMBER" => return self.eval_isnumber(args, sheet, here),
            "ISODD" => return self.eval_even_odd(args, sheet, here, false),
            "ISREF" => return self.eval_isref(args, sheet, here),
            "ISTEXT" => return self.eval_istext(args, sheet, here),
            "N" => return self.eval_n(args, sheet, here),
            "TYPE" => return self.eval_type(args, sheet, here),
            "ERROR.TYPE" => return self.eval_error_type(args, sheet, here),
            "CELL" => return self.eval_cell(args, sheet, here),
            "INFO" => return self.eval_info(args, sheet, here),
            _ => {}
        }
        let mut vals = Vec::new();
        for arg in args {
            match self.eval_expr(arg, sheet, here, mode) {
                Calc::Skip => return Calc::Skip,
                Calc::Ready(v) => vals.push(v),
            }
        }
        match upper.as_str() {
            "SUM" => Calc::Ready(Value::Scalar(self.sum_args(&vals, sheet))),
            "ABS" => self.scalar_fn(&vals, sheet, here, mode, |s| match s {
                Scalar::Num(n) => Scalar::Num(n.abs()),
                Scalar::Bool(b) => Scalar::Num(if b { 1.0 } else { 0.0 }),
                Scalar::Err(e) => Scalar::Err(e),
                Scalar::Text(_) => Scalar::Err(VALUE),
            }),
            "SQRT" => self.scalar_fn(&vals, sheet, here, mode, |s| match num_of(s) {
                Scalar::Num(n) if n < 0.0 => Scalar::Err(NUM),
                other => other_map(other, |n| n.sqrt()),
            }),
            "DATE" => {
                if vals.len() != 3 {
                    return Calc::Skip;
                }
                let y = self.one_num(&vals[0], sheet, here);
                let m = self.one_num(&vals[1], sheet, here);
                let d = self.one_num(&vals[2], sheet, here);
                match (y, m, d) {
                    (Calc::Skip, _, _) | (_, Calc::Skip, _) | (_, _, Calc::Skip) => Calc::Skip,
                    (
                        Calc::Ready(Value::Scalar(a)),
                        Calc::Ready(Value::Scalar(b)),
                        Calc::Ready(Value::Scalar(c)),
                    ) => Calc::Ready(Value::Scalar(date_serial(a, b, c, self.date1904))),
                    _ => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
                }
            }
            "HYPERLINK" => {
                let pick = vals.last();
                match pick {
                    None => Calc::Skip,
                    Some(v) => self.coerce(v.clone(), sheet, here, Mode::Scalar),
                }
            }
            other => match self.eval_text(other, &vals, sheet, here) {
                Some(done) => done,
                None => self.eval_math(other, &vals, sheet, here, mode),
            },
        }
    }

    fn eval_math(
        &mut self,
        name: &str,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Calc {
        match name {
            "ACOS" => self.unary_num(vals, sheet, here, acos_of),
            "ACOSH" => self.unary_num(vals, sheet, here, acosh_of),
            "ASIN" => self.unary_num(vals, sheet, here, asin_of),
            "ASINH" => self.unary_num(vals, sheet, here, asinh_of),
            "ATAN" => self.unary_num(vals, sheet, here, atan_of),
            "ATANH" => self.unary_num(vals, sheet, here, atanh_of),
            "COS" => self.unary_num(vals, sheet, here, cos_of),
            "COSH" => self.unary_num(vals, sheet, here, cosh_of),
            "SIN" => self.unary_num(vals, sheet, here, sin_of),
            "SINH" => self.unary_num(vals, sheet, here, sinh_of),
            "TAN" => self.unary_num(vals, sheet, here, tan_of),
            "TANH" => self.unary_num(vals, sheet, here, tanh_of),
            "EXP" => self.unary_num(vals, sheet, here, exp_of),
            "LN" => self.unary_num(vals, sheet, here, ln_of),
            "LOG10" => self.unary_num(vals, sheet, here, log10_of),
            "SQRTPI" => self.unary_num(vals, sheet, here, sqrtpi_of),
            "DEGREES" => self.unary_num(vals, sheet, here, degrees_of),
            "RADIANS" => self.unary_num(vals, sheet, here, radians_of),
            "SIGN" => self.unary_num(vals, sheet, here, sign_of),
            "EVEN" => self.unary_num(vals, sheet, here, even_of),
            "ODD" => self.unary_num(vals, sheet, here, odd_of),
            "INT" => self.unary_num(vals, sheet, here, int_of),
            "FACT" => self.unary_num(vals, sheet, here, fact_of),
            "FACTDOUBLE" => self.unary_num(vals, sheet, here, fact_double_of),
            "ROMAN" => self.eval_roman(vals, sheet, here),
            "PI" => {
                if vals.is_empty() {
                    ready_num(std::f64::consts::PI)
                } else {
                    Calc::Skip
                }
            }
            "ATAN2" => self.binary_num(vals, sheet, here, atan2_of),
            "POWER" => self.binary_num(vals, sheet, here, power_of),
            "MOD" => self.binary_num(vals, sheet, here, mod_of),
            "QUOTIENT" => self.binary_num(vals, sheet, here, quotient_of),
            "COMBIN" => self.binary_num(vals, sheet, here, combin_of),
            "CEILING" | "ECMA.CEILING" => self.binary_num(vals, sheet, here, ceiling_of),
            "ISO.CEILING" => self.binary_num(vals, sheet, here, iso_ceiling_of),
            "FLOOR" => self.binary_num(vals, sheet, here, floor_of),
            "MROUND" => self.binary_num(vals, sheet, here, mround_of),
            "ROUND" => self.binary_num(vals, sheet, here, |n, d| round_of(n, d, 0)),
            "ROUNDDOWN" => self.binary_num(vals, sheet, here, |n, d| round_of(n, d, -1)),
            "ROUNDUP" => self.binary_num(vals, sheet, here, |n, d| round_of(n, d, 1)),
            "LOG" => self.eval_log(vals, sheet, here),
            "TRUNC" => self.eval_trunc(vals, sheet, here),
            "GCD" => self.eval_gcd_lcm(vals, sheet, here, true),
            "LCM" => self.eval_gcd_lcm(vals, sheet, here, false),
            "MULTINOMIAL" => self.eval_multinomial(vals, sheet, here),
            "RAND" => self.eval_rand(vals, sheet, here),
            "RANDBETWEEN" => self.eval_randbetween(vals, sheet, here),
            "PRODUCT" => Calc::Ready(Value::Scalar(self.product_args(vals, sheet))),
            "SUMSQ" => Calc::Ready(Value::Scalar(self.sumsq_args(vals, sheet))),
            "SUMIF" => self.eval_sumif(vals, sheet, here),
            "SUMIFS" => self.eval_sumifs(vals, sheet, here),
            "SUMPRODUCT" => self.eval_sumproduct(vals, sheet),
            "SUMX2MY2" => self.eval_sumx(vals, sheet, SumX::X2My2),
            "SUMX2PY2" => self.eval_sumx(vals, sheet, SumX::X2Py2),
            "SUMXMY2" => self.eval_sumx(vals, sheet, SumX::Xmy2),
            "SERIESSUM" => self.eval_series(vals, sheet, here),
            "SUBTOTAL" => self.eval_subtotal(vals, sheet, here),
            "MDETERM" => self.eval_mdeterm(vals, sheet),
            "MMULT" => self.eval_mmult(vals, sheet, mode),
            "MINVERSE" => self.eval_minverse(vals, sheet, mode),
            _ => Calc::Skip,
        }
    }

    fn number_of(&mut self, value: &Value, sheet: usize, here: CellRef) -> Calc {
        match self.coerce(value.clone(), sheet, here, Mode::Scalar) {
            Calc::Skip => Calc::Skip,
            Calc::Ready(Value::Scalar(s)) => match num_of(s) {
                Scalar::Num(n) => ready_num(n),
                Scalar::Err(e) => ready_err(e),
                _ => ready_err(VALUE),
            },
            Calc::Ready(_) => ready_err(VALUE),
        }
    }

    fn unary_num(
        &mut self,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        map: fn(f64) -> Scalar,
    ) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.number_of(&vals[0], sheet, here) {
            Calc::Ready(Value::Scalar(Scalar::Num(n))) => Calc::Ready(Value::Scalar(map(n))),
            other => other,
        }
    }

    fn binary_num(
        &mut self,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        map: fn(f64, f64) -> Scalar,
    ) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let a = self.number_of(&vals[0], sheet, here);
        let b = self.number_of(&vals[1], sheet, here);
        match (a, b) {
            (Calc::Skip, _) | (_, Calc::Skip) => Calc::Skip,
            (
                Calc::Ready(Value::Scalar(Scalar::Num(x))),
                Calc::Ready(Value::Scalar(Scalar::Num(y))),
            ) => Calc::Ready(Value::Scalar(map(x, y))),
            (Calc::Ready(Value::Scalar(Scalar::Err(e))), _)
            | (_, Calc::Ready(Value::Scalar(Scalar::Err(e)))) => ready_err(e),
            _ => ready_err(VALUE),
        }
    }

    fn eval_log(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() == 1 {
            return self.unary_num(vals, sheet, here, log10_of);
        }
        if vals.len() != 2 {
            return Calc::Skip;
        }
        self.binary_num(vals, sheet, here, log_of)
    }

    fn eval_trunc(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() == 1 {
            return self.unary_num(vals, sheet, here, |n| trunc_digits(n, 0.0));
        }
        if vals.len() != 2 {
            return Calc::Skip;
        }
        self.binary_num(vals, sheet, here, trunc_digits)
    }

    fn eval_roman(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.is_empty() || vals.len() > 2 {
            return Calc::Skip;
        }
        if vals.len() == 2 {
            match self.number_of(&vals[1], sheet, here) {
                Calc::Ready(Value::Scalar(Scalar::Num(0.0))) => {}
                Calc::Ready(Value::Scalar(Scalar::Err(e))) => return ready_err(e),
                Calc::Skip => return Calc::Skip,
                _ => return Calc::Skip,
            }
        }
        match self.number_of(&vals[0], sheet, here) {
            Calc::Ready(Value::Scalar(Scalar::Num(n))) => match roman_classic(n) {
                Some(text) => Calc::Ready(Value::Scalar(Scalar::Text(text))),
                None => ready_err(VALUE),
            },
            other => other,
        }
    }

    fn eval_rand(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if !vals.is_empty() {
            return Calc::Skip;
        }
        self.volatile_ready(sheet, here, Scalar::Num(unit_random()))
    }

    fn eval_randbetween(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let bottom = self.number_of(&vals[0], sheet, here);
        let top = self.number_of(&vals[1], sheet, here);
        let (lo, hi) = match (bottom, top) {
            (Calc::Skip, _) | (_, Calc::Skip) => return Calc::Skip,
            (
                Calc::Ready(Value::Scalar(Scalar::Num(a))),
                Calc::Ready(Value::Scalar(Scalar::Num(b))),
            ) => (a, b),
            (Calc::Ready(Value::Scalar(Scalar::Err(e))), _)
            | (_, Calc::Ready(Value::Scalar(Scalar::Err(e)))) => {
                return self.volatile_ready(sheet, here, Scalar::Err(e));
            }
            _ => return self.volatile_ready(sheet, here, Scalar::Err(VALUE)),
        };
        if !lo.is_finite() || !hi.is_finite() {
            return self.volatile_ready(sheet, here, Scalar::Err(NUM));
        }
        let lo = lo.trunc() as i64;
        let hi = hi.trunc() as i64;
        if lo > hi {
            return self.volatile_ready(sheet, here, Scalar::Err(NUM));
        }
        let span = (hi as i128 - lo as i128) as u64 + 1;
        let pick = lo + (random_u64() % span) as i64;
        self.volatile_ready(sheet, here, Scalar::Num(pick as f64))
    }

    fn eval_gcd_lcm(&mut self, vals: &[Value], sheet: usize, here: CellRef, gcd: bool) -> Calc {
        if vals.is_empty() {
            return Calc::Skip;
        }
        let mut acc: Option<u64> = None;
        for value in vals {
            match self.int_abs(value, sheet, here) {
                None => return Calc::Skip,
                Some(Err(e)) => return ready_err(e),
                Some(Ok(n)) => {
                    acc = Some(match acc {
                        None => n,
                        Some(prev) if gcd => gcd_u64(prev, n),
                        Some(prev) => lcm_u64(prev, n),
                    });
                }
            }
        }
        ready_num(acc.unwrap_or(0) as f64)
    }

    fn int_abs(
        &mut self,
        value: &Value,
        sheet: usize,
        here: CellRef,
    ) -> Option<Result<u64, &'static str>> {
        match self.number_of(value, sheet, here) {
            Calc::Skip => None,
            Calc::Ready(Value::Scalar(Scalar::Num(n))) if n.is_finite() => {
                let t = n.trunc().abs();
                if t > u64::MAX as f64 {
                    Some(Err(NUM))
                } else {
                    Some(Ok(t as u64))
                }
            }
            Calc::Ready(Value::Scalar(Scalar::Num(_))) => Some(Err(NUM)),
            Calc::Ready(Value::Scalar(Scalar::Err(e))) => Some(Err(e)),
            _ => Some(Err(VALUE)),
        }
    }

    fn eval_multinomial(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.is_empty() {
            return Calc::Skip;
        }
        let mut parts = Vec::new();
        let mut total = 0u64;
        for value in vals {
            match self.int_abs(value, sheet, here) {
                None => return Calc::Skip,
                Some(Err(e)) => return ready_err(e),
                Some(Ok(n)) => {
                    total = total.saturating_add(n);
                    parts.push(n);
                }
            }
        }
        if total > 170 {
            return ready_err(NUM);
        }
        ready_num(multinomial_f64(total, &parts))
    }

    fn product_args(&mut self, vals: &[Value], sheet: usize) -> Scalar {
        let mut acc = 1.0;
        let mut seen = false;
        for value in vals {
            if let Some(e) = self.fold_nums(value, sheet, |n| {
                acc *= n;
                seen = true;
                None
            }) {
                return Scalar::Err(e);
            }
        }
        if seen {
            Scalar::Num(acc)
        } else {
            Scalar::Num(0.0)
        }
    }

    fn sumsq_args(&mut self, vals: &[Value], sheet: usize) -> Scalar {
        let mut acc = 0.0;
        for value in vals {
            if let Some(e) = self.fold_nums(value, sheet, |n| {
                acc += n * n;
                None
            }) {
                return Scalar::Err(e);
            }
        }
        Scalar::Num(acc)
    }

    /// 把数字交给 `step`。`step` 返回 `Some` 时立刻停。
    fn fold_nums(
        &mut self,
        value: &Value,
        sheet: usize,
        mut step: impl FnMut(f64) -> Option<&'static str>,
    ) -> Option<&'static str> {
        match value {
            Value::Scalar(Scalar::Num(n)) => step(*n),
            Value::Scalar(Scalar::Err(e)) => Some(e),
            Value::Scalar(Scalar::Bool(_) | Scalar::Text(_)) => None,
            Value::Area(rect) => self.fold_num_rect(rect, sheet, &mut step),
            Value::Areas(parts) => {
                for part in parts {
                    if let Some(e) = self.fold_num_rect(part, sheet, &mut step) {
                        return Some(e);
                    }
                }
                None
            }
            Value::Grid(g) => {
                for row in g {
                    for cell in row {
                        match cell {
                            Scalar::Num(n) => {
                                if let Some(e) = step(*n) {
                                    return Some(e);
                                }
                            }
                            Scalar::Err(e) => return Some(e),
                            Scalar::Bool(_) | Scalar::Text(_) => {}
                        }
                    }
                }
                None
            }
        }
    }

    fn fold_num_rect(
        &mut self,
        rect: &Rect,
        sheet: usize,
        step: &mut impl FnMut(f64) -> Option<&'static str>,
    ) -> Option<&'static str> {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Some(REF);
        };
        if s1 != s2 {
            return Some(VALUE);
        }
        for row in r1.row..=r2.row {
            for col in r1.col..=r2.col {
                match self.cell_fact(s1, CellRef { col, row }) {
                    Fact::Skip => return Some(VALUE),
                    Fact::Blank => {}
                    Fact::Ready(Scalar::Num(n)) => {
                        if let Some(e) = step(n) {
                            return Some(e);
                        }
                    }
                    Fact::Ready(Scalar::Err(e)) => return Some(e),
                    Fact::Ready(Scalar::Bool(_) | Scalar::Text(_)) => {}
                }
            }
        }
        None
    }

    fn eval_sumif(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 2 && vals.len() != 3 {
            return Calc::Skip;
        }
        let criteria = match self.coerce(vals[1].clone(), sheet, here, Mode::Scalar) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(Value::Scalar(s)) => s,
            _ => return ready_err(VALUE),
        };
        let sum_range = if vals.len() == 3 { &vals[2] } else { &vals[0] };
        match self.sum_where(&vals[0], sum_range, &criteria, sheet) {
            None => Calc::Skip,
            Some(s) => Calc::Ready(Value::Scalar(s)),
        }
    }

    fn eval_sumifs(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() < 3 || vals.len().is_multiple_of(2) {
            return Calc::Skip;
        }
        let mut criteria = Vec::new();
        for pair in vals[1..].chunks(2) {
            match self.coerce(pair[1].clone(), sheet, here, Mode::Scalar) {
                Calc::Skip => return Calc::Skip,
                Calc::Ready(Value::Scalar(s)) => criteria.push((&pair[0], s)),
                _ => return ready_err(VALUE),
            }
        }
        match self.sum_where_all(&vals[0], &criteria, sheet) {
            None => Calc::Skip,
            Some(s) => Calc::Ready(Value::Scalar(s)),
        }
    }

    fn sum_where(
        &mut self,
        keys: &Value,
        sums: &Value,
        criteria: &Scalar,
        sheet: usize,
    ) -> Option<Scalar> {
        let key_cells = self.rect_cells(keys, sheet)?;
        let sum_cells = self.rect_cells(sums, sheet)?;
        if key_cells.len() != sum_cells.len() {
            return Some(Scalar::Err(VALUE));
        }
        let mut total = 0.0;
        for (key, sum) in key_cells.into_iter().zip(sum_cells) {
            if !criteria_match(&key, criteria) {
                continue;
            }
            match sum {
                Fact::Skip => return None,
                Fact::Blank => {}
                Fact::Ready(Scalar::Num(n)) => total += n,
                Fact::Ready(Scalar::Err(e)) => return Some(Scalar::Err(e)),
                Fact::Ready(Scalar::Bool(_) | Scalar::Text(_)) => {}
            }
        }
        Some(Scalar::Num(total))
    }

    fn sum_where_all(
        &mut self,
        sums: &Value,
        criteria: &[(&Value, Scalar)],
        sheet: usize,
    ) -> Option<Scalar> {
        let sum_cells = self.rect_cells(sums, sheet)?;
        let mut masks = Vec::new();
        for (keys, crit) in criteria {
            let cells = self.rect_cells(keys, sheet)?;
            if cells.len() != sum_cells.len() {
                return Some(Scalar::Err(VALUE));
            }
            masks.push((cells, crit));
        }
        let mut total = 0.0;
        for (i, sum) in sum_cells.into_iter().enumerate() {
            let ok = masks
                .iter()
                .all(|(cells, crit)| criteria_match(&cells[i], crit));
            if !ok {
                continue;
            }
            match sum {
                Fact::Skip => return None,
                Fact::Blank => {}
                Fact::Ready(Scalar::Num(n)) => total += n,
                Fact::Ready(Scalar::Err(e)) => return Some(Scalar::Err(e)),
                Fact::Ready(_) => {}
            }
        }
        Some(Scalar::Num(total))
    }

    fn rect_cells(&mut self, value: &Value, sheet: usize) -> Option<Vec<Fact>> {
        match value {
            Value::Area(rect) => self.cells_of_rect(rect, sheet),
            Value::Areas(parts) if parts.len() == 1 => self.cells_of_rect(&parts[0], sheet),
            Value::Scalar(s) => Some(vec![Fact::Ready(s.clone())]),
            Value::Grid(g) => Some(
                g.iter()
                    .flat_map(|row| row.iter().cloned().map(Fact::Ready))
                    .collect(),
            ),
            Value::Areas(_) => Some(vec![Fact::Ready(Scalar::Err(VALUE))]),
        }
    }

    fn cells_of_rect(&mut self, rect: &Rect, sheet: usize) -> Option<Vec<Fact>> {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Some(vec![Fact::Ready(Scalar::Err(REF))]);
        };
        if s1 != s2 {
            return Some(vec![Fact::Ready(Scalar::Err(VALUE))]);
        }
        let mut out = Vec::new();
        for row in r1.row..=r2.row {
            for col in r1.col..=r2.col {
                out.push(self.cell_fact(s1, CellRef { col, row }));
            }
        }
        Some(out)
    }

    fn eval_sumproduct(&mut self, vals: &[Value], sheet: usize) -> Calc {
        if vals.is_empty() {
            return Calc::Skip;
        }
        let mut grids = Vec::new();
        for value in vals {
            match self.as_matrix(value, sheet) {
                Mat::Skip => return Calc::Skip,
                Mat::Err(e) => return ready_err(e),
                Mat::Ok(g) => grids.push(g),
            }
        }
        let rows = grids[0].len();
        let cols = grids[0].first().map(|r| r.len()).unwrap_or(0);
        if grids
            .iter()
            .any(|g| g.len() != rows || g.iter().any(|r| r.len() != cols))
        {
            return ready_err(VALUE);
        }
        let mut total = 0.0;
        for r in 0..rows {
            for c in 0..cols {
                let mut term = 1.0;
                for grid in &grids {
                    term *= grid[r][c];
                }
                total += term;
            }
        }
        ready_num(total)
    }

    fn eval_sumx(&mut self, vals: &[Value], sheet: usize, kind: SumX) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let (left, right) = match (
            self.as_matrix(&vals[0], sheet),
            self.as_matrix(&vals[1], sheet),
        ) {
            (Mat::Skip, _) | (_, Mat::Skip) => return Calc::Skip,
            (Mat::Err(e), _) | (_, Mat::Err(e)) => return ready_err(e),
            (Mat::Ok(a), Mat::Ok(b)) => (a, b),
        };
        if left.len() != right.len() || left.iter().zip(&right).any(|(a, b)| a.len() != b.len()) {
            return ready_err(VALUE);
        }
        let mut total = 0.0;
        for (row_a, row_b) in left.iter().zip(&right) {
            for (x, y) in row_a.iter().zip(row_b) {
                total += match kind {
                    SumX::X2My2 => x * x - y * y,
                    SumX::X2Py2 => x * x + y * y,
                    SumX::Xmy2 => (x - y) * (x - y),
                };
            }
        }
        ready_num(total)
    }

    fn eval_series(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 4 {
            return Calc::Skip;
        }
        let x = self.number_of(&vals[0], sheet, here);
        let n = self.number_of(&vals[1], sheet, here);
        let m = self.number_of(&vals[2], sheet, here);
        let (x, n, m) = match (x, n, m) {
            (Calc::Skip, _, _) | (_, Calc::Skip, _) | (_, _, Calc::Skip) => return Calc::Skip,
            (
                Calc::Ready(Value::Scalar(Scalar::Num(x))),
                Calc::Ready(Value::Scalar(Scalar::Num(n))),
                Calc::Ready(Value::Scalar(Scalar::Num(m))),
            ) => (x, n, m),
            (Calc::Ready(Value::Scalar(Scalar::Err(e))), _, _)
            | (_, Calc::Ready(Value::Scalar(Scalar::Err(e))), _)
            | (_, _, Calc::Ready(Value::Scalar(Scalar::Err(e)))) => return ready_err(e),
            _ => return ready_err(VALUE),
        };
        let coeffs = match self.as_matrix(&vals[3], sheet) {
            Mat::Skip => return Calc::Skip,
            Mat::Err(e) => return ready_err(e),
            Mat::Ok(g) => g.into_iter().flatten().collect::<Vec<_>>(),
        };
        let mut total = 0.0;
        for (i, coeff) in coeffs.iter().enumerate() {
            let exp = n + i as f64 * m;
            if x < 0.0 && exp.fract() != 0.0 {
                return ready_err(NUM);
            }
            let term = coeff * x.powf(exp);
            if !term.is_finite() {
                return ready_err(NUM);
            }
            total += term;
        }
        ready_num(total)
    }

    fn eval_subtotal(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() < 2 {
            return Calc::Skip;
        }
        let code = match self.number_of(&vals[0], sheet, here) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(Value::Scalar(Scalar::Num(n))) => n,
            Calc::Ready(Value::Scalar(Scalar::Err(e))) => return ready_err(e),
            _ => return ready_err(VALUE),
        };
        if code.fract() != 0.0 {
            return ready_err(VALUE);
        }
        let skip_hidden = code >= 100.0;
        let kind = if skip_hidden { code - 100.0 } else { code };
        if !(1.0..=11.0).contains(&kind) {
            return ready_err(VALUE);
        }
        let mut nums = Vec::new();
        let mut count_a = 0.0;
        for value in &vals[1..] {
            match self.collect_subtotal(value, sheet, skip_hidden, &mut nums, &mut count_a) {
                None => return Calc::Skip,
                Some(Some(e)) => return ready_err(e),
                Some(None) => {}
            }
        }
        Calc::Ready(Value::Scalar(subtotal_finish(kind, &nums, count_a)))
    }

    fn collect_subtotal(
        &mut self,
        value: &Value,
        sheet: usize,
        skip_hidden: bool,
        nums: &mut Vec<f64>,
        count_a: &mut f64,
    ) -> Option<Option<&'static str>> {
        match value {
            Value::Area(rect) => self.collect_rect(rect, sheet, skip_hidden, nums, count_a),
            Value::Areas(parts) => {
                for part in parts {
                    if let Some(e) = self.collect_rect(part, sheet, skip_hidden, nums, count_a)? {
                        return Some(Some(e));
                    }
                }
                Some(None)
            }
            Value::Scalar(Scalar::Err(e)) => Some(Some(e)),
            Value::Scalar(Scalar::Num(n)) => {
                nums.push(*n);
                *count_a += 1.0;
                Some(None)
            }
            Value::Scalar(_) => {
                *count_a += 1.0;
                Some(None)
            }
            Value::Grid(_) => Some(Some(VALUE)),
        }
    }

    fn collect_rect(
        &mut self,
        rect: &Rect,
        sheet: usize,
        skip_hidden: bool,
        nums: &mut Vec<f64>,
        count_a: &mut f64,
    ) -> Option<Option<&'static str>> {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Some(Some(REF));
        };
        if s1 != s2 {
            return Some(Some(VALUE));
        }
        let hidden = self
            .sheets
            .get(s1)
            .map(|s| s.hidden_rows.clone())
            .unwrap_or_default();
        for row in r1.row..=r2.row {
            if skip_hidden && hidden.contains(&row) {
                continue;
            }
            for col in r1.col..=r2.col {
                let cell = CellRef { col, row };
                if self.subtotal_formula(s1, cell) {
                    continue;
                }
                match self.cell_fact(s1, cell) {
                    Fact::Skip => return None,
                    Fact::Blank => {}
                    Fact::Ready(Scalar::Num(n)) => {
                        nums.push(n);
                        *count_a += 1.0;
                    }
                    Fact::Ready(Scalar::Bool(_) | Scalar::Text(_)) => *count_a += 1.0,
                    Fact::Ready(Scalar::Err(e)) => return Some(Some(e)),
                }
            }
        }
        Some(None)
    }

    fn subtotal_formula(&self, sheet: usize, cell: CellRef) -> bool {
        self.sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(cell.col, cell.row)))
            .and_then(|rec| rec.formula.as_ref())
            .is_some_and(|body| body.trim().to_ascii_uppercase().starts_with("SUBTOTAL("))
    }

    fn eval_mdeterm(&mut self, vals: &[Value], sheet: usize) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.as_matrix(&vals[0], sheet) {
            Mat::Skip => Calc::Skip,
            Mat::Err(e) => ready_err(e),
            Mat::Ok(grid) => {
                if !square(&grid) {
                    ready_err(VALUE)
                } else {
                    ready_num(mat_det(grid))
                }
            }
        }
    }

    fn eval_mmult(&mut self, vals: &[Value], sheet: usize, _mode: Mode) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let left = self.as_matrix(&vals[0], sheet);
        let right = self.as_matrix(&vals[1], sheet);
        match (left, right) {
            (Mat::Skip, _) | (_, Mat::Skip) => Calc::Skip,
            (Mat::Err(e), _) | (_, Mat::Err(e)) => ready_err(e),
            (Mat::Ok(a), Mat::Ok(b)) => match mat_mul(&a, &b) {
                Some(grid) => Calc::Ready(Value::Grid(num_grid(grid))),
                None => ready_err(VALUE),
            },
        }
    }

    fn eval_minverse(&mut self, vals: &[Value], sheet: usize, _mode: Mode) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.as_matrix(&vals[0], sheet) {
            Mat::Skip => Calc::Skip,
            Mat::Err(e) => ready_err(e),
            Mat::Ok(grid) => {
                if !square(&grid) {
                    return ready_err(VALUE);
                }
                match mat_inv(grid) {
                    Some(grid) => Calc::Ready(Value::Grid(num_grid(grid))),
                    None => ready_err(NUM),
                }
            }
        }
    }

    fn as_matrix(&mut self, value: &Value, sheet: usize) -> Mat {
        match value {
            Value::Scalar(Scalar::Num(n)) => Mat::Ok(vec![vec![*n]]),
            Value::Scalar(Scalar::Bool(b)) => Mat::Ok(vec![vec![if *b { 1.0 } else { 0.0 }]]),
            Value::Scalar(Scalar::Err(e)) => Mat::Err(e),
            Value::Scalar(Scalar::Text(_)) => Mat::Err(VALUE),
            Value::Grid(g) => {
                let mut rows = Vec::new();
                for row in g {
                    let mut line = Vec::new();
                    for cell in row {
                        match cell {
                            Scalar::Num(n) => line.push(*n),
                            Scalar::Bool(b) => line.push(if *b { 1.0 } else { 0.0 }),
                            Scalar::Err(e) => return Mat::Err(e),
                            Scalar::Text(_) => return Mat::Err(VALUE),
                        }
                    }
                    rows.push(line);
                }
                Mat::Ok(rows)
            }
            Value::Area(rect) => self.matrix_rect(rect, sheet),
            Value::Areas(parts) if parts.len() == 1 => self.matrix_rect(&parts[0], sheet),
            Value::Areas(_) => Mat::Err(VALUE),
        }
    }

    fn matrix_rect(&mut self, rect: &Rect, sheet: usize) -> Mat {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Mat::Err(REF);
        };
        if s1 != s2 {
            return Mat::Err(VALUE);
        }
        let mut rows = Vec::new();
        for row in r1.row..=r2.row {
            let mut line = Vec::new();
            for col in r1.col..=r2.col {
                match self.cell_fact(s1, CellRef { col, row }) {
                    Fact::Skip => return Mat::Skip,
                    Fact::Blank => line.push(0.0),
                    Fact::Ready(Scalar::Num(n)) => line.push(n),
                    Fact::Ready(Scalar::Bool(b)) => line.push(if b { 1.0 } else { 0.0 }),
                    Fact::Ready(Scalar::Err(e)) => return Mat::Err(e),
                    Fact::Ready(Scalar::Text(_)) => return Mat::Err(VALUE),
                }
            }
            rows.push(line);
        }
        Mat::Ok(rows)
    }

    fn mark_volatile(&mut self, sheet: usize, here: CellRef) {
        self.volatile.insert((sheet, here.col, here.row));
    }

    fn volatile_ready(&mut self, sheet: usize, here: CellRef, scalar: Scalar) -> Calc {
        self.mark_volatile(sheet, here);
        Calc::Ready(Value::Scalar(scalar))
    }

    fn cell_fact(&mut self, sheet: usize, cell: CellRef) -> Fact {
        let kind = self
            .sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(cell.col, cell.row)))
            .map(|rec| {
                if rec.formula.is_some() || rec.array_master.is_some() {
                    1u8
                } else if rec.cached.is_some() {
                    2
                } else {
                    0
                }
            });
        match kind {
            None | Some(0) => Fact::Blank,
            Some(2) => {
                let cached = self.sheets[sheet].cells[&(cell.col, cell.row)]
                    .cached
                    .clone()
                    .expect("有缓存");
                Fact::Ready(cached)
            }
            Some(1) => match self.scalar_at(sheet, cell) {
                Calc::Ready(Value::Scalar(s)) => Fact::Ready(s),
                _ => Fact::Skip,
            },
            _ => Fact::Blank,
        }
    }

    /// 普通格子里的区域按隐式交集。对不上时是 `#VALUE!`，表名对不上时是 `#REF!`。
    fn hit_rect(&mut self, rect: &Rect, sheet: usize, here: CellRef) -> Hit {
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Hit::Err(REF);
        };
        if s1 != s2 {
            return Hit::Err(VALUE);
        }
        let (r1, r2) = ordered(rect.r1, rect.r2);
        if r1 == r2 {
            return Hit::Fact(self.cell_fact(s1, r1));
        }
        if s1 != sheet {
            return Hit::Err(VALUE);
        }
        if r1.col == r2.col && here.row >= r1.row && here.row <= r2.row {
            return Hit::Fact(self.cell_fact(
                s1,
                CellRef {
                    col: r1.col,
                    row: here.row,
                },
            ));
        }
        if r1.row == r2.row && here.col >= r1.col && here.col <= r2.col {
            return Hit::Fact(self.cell_fact(
                s1,
                CellRef {
                    col: here.col,
                    row: r1.row,
                },
            ));
        }
        Hit::Err(VALUE)
    }

    fn hit_value(&mut self, value: Value, sheet: usize, here: CellRef) -> Hit {
        match value {
            Value::Scalar(s) => Hit::Fact(Fact::Ready(s)),
            Value::Grid(g) => match g.first().and_then(|row| row.first()) {
                Some(s) => Hit::Fact(Fact::Ready(s.clone())),
                None => Hit::Err(VALUE),
            },
            Value::Area(rect) => self.hit_rect(&rect, sheet, here),
            Value::Areas(mut parts) => {
                if parts.len() == 1 {
                    self.hit_rect(&parts.remove(0), sheet, here)
                } else {
                    Hit::Err(VALUE)
                }
            }
        }
    }

    fn reduce_arg(&mut self, expr: &Expr, sheet: usize, here: CellRef) -> Option<Hit> {
        match self.eval_expr(expr, sheet, here, Mode::Scalar) {
            Calc::Skip => None,
            Calc::Ready(v) => Some(self.hit_value(v, sheet, here)),
        }
    }

    fn arg_scalar(&mut self, expr: &Expr, sheet: usize, here: CellRef) -> Option<Scalar> {
        let value = match self.eval_expr(expr, sheet, here, Mode::Scalar) {
            Calc::Skip => return None,
            Calc::Ready(v) => v,
        };
        match self.coerce(value, sheet, here, Mode::Scalar) {
            Calc::Ready(Value::Scalar(s)) => Some(s),
            _ => None,
        }
    }

    fn eval_isblank(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let Some(hit) = self.reduce_arg(&args[0], sheet, here) else {
            return Calc::Skip;
        };
        let blank = matches!(hit, Hit::Fact(Fact::Blank));
        ready_bool(blank)
    }

    fn eval_iserr(&mut self, args: &[Expr], sheet: usize, here: CellRef, any: bool) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let flag = match error_of(&hit) {
            Some(e) => any || e != NA,
            None => false,
        };
        ready_bool(flag)
    }

    fn eval_isna(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        ready_bool(error_of(&hit) == Some(NA))
    }

    fn eval_islogical(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let flag = matches!(hit, Hit::Fact(Fact::Ready(Scalar::Bool(_))));
        ready_bool(flag)
    }

    fn eval_isnumber(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let flag = matches!(hit, Hit::Fact(Fact::Ready(Scalar::Num(_))));
        ready_bool(flag)
    }

    fn eval_istext(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let flag = matches!(hit, Hit::Fact(Fact::Ready(Scalar::Text(_))));
        ready_bool(flag)
    }

    fn eval_isnontext(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let flag = !matches!(hit, Hit::Fact(Fact::Ready(Scalar::Text(_))));
        ready_bool(flag)
    }

    fn eval_even_odd(&mut self, args: &[Expr], sheet: usize, here: CellRef, even: bool) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        let scalar = match hit {
            Hit::Fact(Fact::Skip) => return Calc::Skip,
            Hit::Fact(Fact::Blank) => Scalar::Num(0.0),
            Hit::Fact(Fact::Ready(s)) => s,
            Hit::Err(e) => return ready_err(e),
        };
        match scalar {
            Scalar::Num(n) if n.is_finite() => {
                let odd = n.trunc() % 2.0 != 0.0;
                ready_bool(if even { !odd } else { odd })
            }
            Scalar::Err(e) => ready_err(e),
            Scalar::Num(_) | Scalar::Bool(_) | Scalar::Text(_) => ready_err(VALUE),
        }
    }

    fn eval_isref(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let value = match self.eval_expr(&args[0], sheet, here, Mode::Scalar) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        let flag = match &value {
            Value::Area(rect) => self.same_sheet_ref(rect, sheet),
            Value::Areas(parts) => {
                !parts.is_empty() && parts.iter().all(|rect| self.same_sheet_ref(rect, sheet))
            }
            Value::Scalar(_) | Value::Grid(_) => false,
        };
        ready_bool(flag)
    }

    fn same_sheet_ref(&self, rect: &Rect, sheet: usize) -> bool {
        match self.resolve_span(rect, sheet) {
            Some((a, b)) => a == b,
            None => true,
        }
    }

    fn eval_n(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let value = match self.eval_expr(&args[0], sheet, here, Mode::Scalar) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        match value {
            Value::Scalar(s) => Calc::Ready(Value::Scalar(n_convert(s))),
            Value::Grid(g) => match g.first().and_then(|row| row.first()).cloned() {
                Some(s) => Calc::Ready(Value::Scalar(n_convert(s))),
                None => ready_err(VALUE),
            },
            Value::Area(rect) => self.n_corner(&rect, sheet),
            Value::Areas(parts) => {
                if parts.len() == 1 {
                    self.n_corner(&parts[0], sheet)
                } else {
                    ready_err(VALUE)
                }
            }
        }
    }

    fn n_corner(&mut self, rect: &Rect, sheet: usize) -> Calc {
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return ready_err(REF);
        };
        if s1 != s2 {
            return ready_err(VALUE);
        }
        let (r1, _) = ordered(rect.r1, rect.r2);
        match self.cell_fact(s1, r1) {
            Fact::Skip => Calc::Skip,
            Fact::Blank => ready_num(0.0),
            Fact::Ready(s) => Calc::Ready(Value::Scalar(n_convert(s))),
        }
    }

    fn eval_type(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let value = match self.eval_expr(&args[0], sheet, here, Mode::Scalar) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        if matches!(&value, Value::Grid(_)) {
            return ready_num(64.0);
        }
        match self.hit_value(value, sheet, here) {
            Hit::Fact(Fact::Skip) => Calc::Skip,
            Hit::Fact(Fact::Blank) => ready_num(1.0),
            Hit::Fact(Fact::Ready(s)) => ready_num(type_code(&s)),
            Hit::Err(_) => ready_num(16.0),
        }
    }

    fn eval_error_type(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        let Some(hit) = self.one_hit(args, sheet, here) else {
            return Calc::Skip;
        };
        match error_of(&hit) {
            Some(e) => match error_type_code(e) {
                Some(n) => ready_num(n),
                None => ready_err(NA),
            },
            None => ready_err(NA),
        }
    }

    fn one_hit(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Option<Hit> {
        if args.len() != 1 {
            return None;
        }
        match self.reduce_arg(&args[0], sheet, here) {
            None => None,
            Some(Hit::Fact(Fact::Skip)) => None,
            Some(hit) => Some(hit),
        }
    }

    fn eval_cell(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 2 {
            return Calc::Skip;
        }
        let cat = match self.arg_scalar(&args[0], sheet, here) {
            None => return Calc::Skip,
            Some(Scalar::Text(s)) => s,
            Some(Scalar::Err(e)) => return self.volatile_ready(sheet, here, Scalar::Err(e)),
            Some(_) => return self.volatile_ready(sheet, here, Scalar::Err(VALUE)),
        };
        let value = match self.eval_expr(&args[1], sheet, here, Mode::Scalar) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        let rect = match value {
            Value::Area(rect) => rect,
            Value::Areas(parts) => match parts.first() {
                Some(rect) if parts.len() == 1 => rect.clone(),
                _ => return self.volatile_ready(sheet, here, Scalar::Err(VALUE)),
            },
            _ => return self.volatile_ready(sheet, here, Scalar::Err(VALUE)),
        };
        let Some((s1, _)) = self.resolve_span(&rect, sheet) else {
            return self.volatile_ready(sheet, here, Scalar::Err(REF));
        };
        let (corner, _) = ordered(rect.r1, rect.r2);
        match cat.to_ascii_lowercase().as_str() {
            "address" => {
                let text = format!("${}${}", a1::col_to_letters(corner.col), corner.row);
                self.volatile_ready(sheet, here, Scalar::Text(text))
            }
            "col" => self.volatile_ready(sheet, here, Scalar::Num(corner.col as f64)),
            "row" => self.volatile_ready(sheet, here, Scalar::Num(corner.row as f64)),
            "contents" => match self.cell_fact(s1, corner) {
                Fact::Skip => Calc::Skip,
                Fact::Blank => self.volatile_ready(sheet, here, Scalar::Num(0.0)),
                Fact::Ready(s) => self.volatile_ready(sheet, here, s),
            },
            "type" => {
                let kind = match self.cell_fact(s1, corner) {
                    Fact::Skip => return Calc::Skip,
                    Fact::Blank => "b",
                    Fact::Ready(Scalar::Text(_)) => "l",
                    Fact::Ready(_) => "v",
                };
                self.volatile_ready(sheet, here, Scalar::Text(kind.to_string()))
            }
            _ => Calc::Skip,
        }
    }

    fn eval_info(&mut self, args: &[Expr], sheet: usize, here: CellRef) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let cat = match self.arg_scalar(&args[0], sheet, here) {
            None => return Calc::Skip,
            Some(Scalar::Text(s)) => s,
            Some(Scalar::Err(e)) => return self.volatile_ready(sheet, here, Scalar::Err(e)),
            Some(_) => return self.volatile_ready(sheet, here, Scalar::Err(VALUE)),
        };
        let scalar = match cat.to_ascii_lowercase().as_str() {
            "system" => {
                if cfg!(windows) {
                    Scalar::Text("pcdos".to_string())
                } else {
                    return Calc::Skip;
                }
            }
            "recalc" => Scalar::Text(if self.manual { "Manual" } else { "Automatic" }.to_string()),
            "release" => Scalar::Text("16.0".to_string()),
            "numfile" => Scalar::Num(self.sheets.len() as f64),
            "origin" => {
                let cell = self
                    .sheets
                    .get(sheet)
                    .map(|s| s.origin)
                    .unwrap_or(CellRef { col: 1, row: 1 });
                Scalar::Text(format!("$A:${}${}", a1::col_to_letters(cell.col), cell.row))
            }
            "directory" | "memavail" | "memused" | "osversion" | "totmem" => return Calc::Skip,
            _ => Scalar::Err(VALUE),
        };
        self.volatile_ready(sheet, here, scalar)
    }

    /// `IF` 只算被选中的分支。条件不是逻辑值时直接 `#VALUE!`，两个分支都不算。
    fn eval_if(&mut self, args: &[Expr], sheet: usize, here: CellRef, mode: Mode) -> Calc {
        if args.len() != 2 && args.len() != 3 {
            return Calc::Skip;
        }
        let Some(cond) = ready_value(self.eval_expr(&args[0], sheet, here, mode)) else {
            return Calc::Skip;
        };
        if matches!(mode, Mode::Scalar) {
            let Some(flag) = self.scalar_flag(cond, sheet, here) else {
                return Calc::Skip;
            };
            return self.if_branch(flag, args, sheet, here, mode);
        }
        let cond = match self.coerce(cond, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        match cond {
            Value::Scalar(s) => self.if_branch(if_flag(&s), args, sheet, here, mode),
            Value::Grid(g) => self.eval_if_grid(&g, args, sheet, here, mode),
            _ => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
        }
    }

    fn if_branch(
        &mut self,
        flag: Result<bool, &'static str>,
        args: &[Expr],
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Calc {
        match flag {
            Err(e) => Calc::Ready(Value::Scalar(Scalar::Err(e))),
            Ok(true) => self.eval_expr(&args[1], sheet, here, mode),
            Ok(false) => {
                if args.len() == 3 {
                    self.eval_expr(&args[2], sheet, here, mode)
                } else {
                    Calc::Ready(Value::Scalar(Scalar::Bool(false)))
                }
            }
        }
    }

    fn eval_if_grid(
        &mut self,
        grid: &[Vec<Scalar>],
        args: &[Expr],
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Calc {
        let mut need_true = false;
        let mut need_false = false;
        for cell in grid.iter().flatten() {
            match if_flag(cell) {
                Ok(true) => need_true = true,
                Ok(false) => need_false = true,
                Err(_) => {}
            }
        }
        let yes = if need_true {
            match self.logic_branch(&args[1], sheet, here, mode) {
                None => return Calc::Skip,
                Some(b) => Some(b),
            }
        } else {
            None
        };
        let no = if need_false {
            if args.len() == 3 {
                match self.logic_branch(&args[2], sheet, here, mode) {
                    None => return Calc::Skip,
                    Some(b) => Some(b),
                }
            } else {
                Some(Branch::Scalar(Scalar::Bool(false)))
            }
        } else {
            None
        };
        let yes = yes.unwrap_or(Branch::Scalar(Scalar::Err(VALUE)));
        let no = no.unwrap_or(Branch::Scalar(Scalar::Bool(false)));
        let out = grid
            .iter()
            .enumerate()
            .map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .map(|(c, cell)| match if_flag(cell) {
                        Err(e) => Scalar::Err(e),
                        Ok(true) => pick_branch(&yes, r, c),
                        Ok(false) => pick_branch(&no, r, c),
                    })
                    .collect()
            })
            .collect();
        Calc::Ready(Value::Grid(out))
    }

    /// 第一支不是错误就不算第二支。普通格子里多格数组常量 Excel 16.0 打不开，不写缓存。
    fn eval_iferror(&mut self, args: &[Expr], sheet: usize, here: CellRef, mode: Mode) -> Calc {
        if args.len() != 2 {
            return Calc::Skip;
        }
        let Some(value) = ready_value(self.eval_expr(&args[0], sheet, here, mode)) else {
            return Calc::Skip;
        };
        if matches!(mode, Mode::Scalar) {
            if let Value::Grid(g) = &value {
                if grid_multi(g) {
                    return Calc::Skip;
                }
            }
        }
        let value = match self.coerce(value, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        match value {
            Value::Scalar(Scalar::Err(_)) => self.eval_expr(&args[1], sheet, here, mode),
            Value::Scalar(s) => Calc::Ready(Value::Scalar(s)),
            Value::Grid(g) => {
                let needs = g.iter().flatten().any(|s| matches!(s, Scalar::Err(_)));
                if !needs {
                    return Calc::Ready(Value::Grid(g));
                }
                let Some(alt) = self.logic_branch(&args[1], sheet, here, mode) else {
                    return Calc::Skip;
                };
                let out = g
                    .iter()
                    .enumerate()
                    .map(|(r, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(c, cell)| {
                                if matches!(cell, Scalar::Err(_)) {
                                    pick_branch(&alt, r, c)
                                } else {
                                    cell.clone()
                                }
                            })
                            .collect()
                    })
                    .collect();
                Calc::Ready(Value::Grid(out))
            }
            _ => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
        }
    }

    fn eval_not(&mut self, args: &[Expr], sheet: usize, here: CellRef, mode: Mode) -> Calc {
        if args.len() != 1 {
            return Calc::Skip;
        }
        let Some(value) = ready_value(self.eval_expr(&args[0], sheet, here, mode)) else {
            return Calc::Skip;
        };
        let value = match self.coerce(value, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        match value {
            Value::Scalar(s) => match if_flag(&s) {
                Ok(b) => Calc::Ready(Value::Scalar(Scalar::Bool(!b))),
                Err(e) => Calc::Ready(Value::Scalar(Scalar::Err(e))),
            },
            Value::Grid(g) => Calc::Ready(Value::Grid(
                g.into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|s| match if_flag(&s) {
                                Ok(b) => Scalar::Bool(!b),
                                Err(e) => Scalar::Err(e),
                            })
                            .collect()
                    })
                    .collect(),
            )),
            _ => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
        }
    }

    /// `AND` / `OR` 把参数全部算完。错误按从左到右留第一个，不因为已经能确定真假就停。
    fn eval_and_or(
        &mut self,
        args: &[Expr],
        sheet: usize,
        here: CellRef,
        mode: Mode,
        and: bool,
    ) -> Calc {
        if args.is_empty() {
            return Calc::Skip;
        }
        let mut vals = Vec::new();
        for arg in args {
            match self.eval_expr(arg, sheet, here, mode) {
                Calc::Skip => return Calc::Skip,
                Calc::Ready(v) => vals.push(v),
            }
        }
        let mut fold = LogicFold::new();
        for value in vals {
            if self.fold_logic(value, sheet, &mut fold).is_err() {
                return Calc::Skip;
            }
        }
        Calc::Ready(Value::Scalar(fold.finish(and)))
    }

    fn fold_logic(&mut self, value: Value, sheet: usize, fold: &mut LogicFold) -> Result<(), ()> {
        match value {
            Value::Scalar(s) => {
                fold.push_direct(&s);
                Ok(())
            }
            Value::Grid(g) => {
                for cell in g.into_iter().flatten() {
                    fold.push_ref(&cell);
                }
                Ok(())
            }
            Value::Area(rect) => self.fold_rect(&rect, sheet, fold),
            Value::Areas(parts) => {
                for part in parts {
                    self.fold_rect(&part, sheet, fold)?;
                }
                Ok(())
            }
        }
    }

    fn fold_rect(&mut self, rect: &Rect, here: usize, fold: &mut LogicFold) -> Result<(), ()> {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, here) else {
            fold.note_err(REF);
            return Ok(());
        };
        for sheet in s1..=s2 {
            for row in r1.row..=r2.row {
                for col in r1.col..=r2.col {
                    match self.logic_ref(sheet, CellRef { col, row }) {
                        LogicRef::Skip => return Err(()),
                        LogicRef::Ignore => {}
                        LogicRef::Value(s) => fold.push_ref(&s),
                    }
                }
            }
        }
        Ok(())
    }

    /// 引用里的空格子和文本不参加 `AND` / `OR`。没有公式、也没有缓存的格子当空。
    fn logic_ref(&mut self, sheet: usize, here: CellRef) -> LogicRef {
        let rec = self
            .sheets
            .get(sheet)
            .and_then(|s| s.cells.get(&(here.col, here.row)))
            .map(|r| r.clone_rec());
        let Some(rec) = rec else {
            return LogicRef::Ignore;
        };
        let has_formula =
            rec.formula.is_some() || rec.array_master.is_some() || rec.array_ref.is_some();
        if !has_formula {
            return match rec.cached {
                Some(s) => LogicRef::Value(s),
                None => LogicRef::Ignore,
            };
        }
        match self.scalar_at(sheet, here) {
            Calc::Skip => LogicRef::Skip,
            Calc::Ready(Value::Scalar(s)) => LogicRef::Value(s),
            Calc::Ready(_) => LogicRef::Value(Scalar::Err(VALUE)),
        }
    }

    fn scalar_flag(
        &mut self,
        value: Value,
        sheet: usize,
        here: CellRef,
    ) -> Option<Result<bool, &'static str>> {
        match self.coerce(value, sheet, here, Mode::Scalar) {
            Calc::Skip => None,
            Calc::Ready(Value::Scalar(s)) => Some(if_flag(&s)),
            Calc::Ready(_) => Some(Err(VALUE)),
        }
    }

    fn logic_branch(
        &mut self,
        expr: &Expr,
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Option<Branch> {
        let value = ready_value(self.eval_expr(expr, sheet, here, mode))?;
        let value = match self.coerce(value, sheet, here, mode) {
            Calc::Skip => return None,
            Calc::Ready(v) => v,
        };
        Some(match value {
            Value::Scalar(s) => Branch::Scalar(s),
            Value::Grid(g) => Branch::Grid(g),
            _ => Branch::Scalar(Scalar::Err(VALUE)),
        })
    }

    fn scalar_fn(
        &mut self,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        mode: Mode,
        f: impl Fn(Scalar) -> Scalar,
    ) -> Calc {
        let Some(v) = vals.first() else {
            return Calc::Skip;
        };
        self.map_ready(v.clone(), sheet, here, mode, f)
    }

    fn map_ready(
        &mut self,
        v: Value,
        sheet: usize,
        here: CellRef,
        mode: Mode,
        f: impl Fn(Scalar) -> Scalar,
    ) -> Calc {
        match self.coerce(v, sheet, here, mode) {
            Calc::Skip => Calc::Skip,
            Calc::Ready(Value::Scalar(s)) => Calc::Ready(Value::Scalar(f(s))),
            Calc::Ready(Value::Grid(g)) => Calc::Ready(Value::Grid(
                g.into_iter()
                    .map(|row| row.into_iter().map(&f).collect())
                    .collect(),
            )),
            Calc::Ready(_) => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
        }
    }

    fn one_num(&mut self, v: &Value, sheet: usize, here: CellRef) -> Calc {
        match self.coerce(v.clone(), sheet, here, Mode::Scalar) {
            Calc::Ready(Value::Scalar(s)) => Calc::Ready(Value::Scalar(num_of(s))),
            other => other,
        }
    }

    fn eval_bin(
        &mut self,
        op: BinOp,
        l: &Expr,
        r: &Expr,
        sheet: usize,
        here: CellRef,
        mode: Mode,
    ) -> Calc {
        let left = match self.eval_expr(l, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        let right = match self.eval_expr(r, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        let left = match self.coerce(left, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        let right = match self.coerce(right, sheet, here, mode) {
            Calc::Skip => return Calc::Skip,
            Calc::Ready(v) => v,
        };
        match (left, right) {
            (Value::Scalar(a), Value::Scalar(b)) => Calc::Ready(Value::Scalar(apply_bin(op, a, b))),
            (Value::Grid(g), Value::Scalar(s)) => {
                Calc::Ready(Value::Grid(map_grid(g, |a| apply_bin(op, a, s.clone()))))
            }
            (Value::Scalar(s), Value::Grid(g)) => {
                Calc::Ready(Value::Grid(map_grid(g, |b| apply_bin(op, s.clone(), b))))
            }
            (Value::Grid(a), Value::Grid(b)) => match zip_grid(a, b, |x, y| apply_bin(op, x, y)) {
                Some(g) => Calc::Ready(Value::Grid(g)),
                None => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
            },
            _ => Calc::Ready(Value::Scalar(Scalar::Err(VALUE))),
        }
    }

    fn coerce(&mut self, value: Value, sheet: usize, here: CellRef, mode: Mode) -> Calc {
        match mode {
            Mode::Array => match value {
                Value::Area(rect) => Calc::Ready(Value::Grid(self.grid_from_rect(&rect, sheet))),
                Value::Areas(parts) if parts.len() == 1 => {
                    Calc::Ready(Value::Grid(self.grid_from_rect(&parts[0], sheet)))
                }
                other => Calc::Ready(other),
            },
            Mode::Scalar => match value {
                Value::Area(rect) => Calc::Ready(Value::Scalar(self.implicit(&rect, sheet, here))),
                Value::Areas(parts) => {
                    if parts.len() == 1 {
                        Calc::Ready(Value::Scalar(self.implicit(&parts[0], sheet, here)))
                    } else {
                        Calc::Ready(Value::Scalar(Scalar::Err(VALUE)))
                    }
                }
                Value::Grid(g) => {
                    let s = g
                        .first()
                        .and_then(|r| r.first())
                        .cloned()
                        .unwrap_or(Scalar::Err(VALUE));
                    Calc::Ready(Value::Scalar(s))
                }
                other => Calc::Ready(other),
            },
        }
    }

    fn implicit(&mut self, rect: &Rect, sheet: usize, here: CellRef) -> Scalar {
        let Some((s1, s2)) = self.resolve_span(rect, sheet) else {
            return Scalar::Err(REF);
        };
        if s1 != s2 {
            return Scalar::Err(VALUE);
        }
        let (r1, r2) = ordered(rect.r1, rect.r2);
        if r1 == r2 {
            return self.cell_scalar(s1, r1);
        }
        if s1 != sheet {
            return Scalar::Err(VALUE);
        }
        if r1.col == r2.col && here.row >= r1.row && here.row <= r2.row {
            return self.cell_scalar(
                s1,
                CellRef {
                    col: r1.col,
                    row: here.row,
                },
            );
        }
        if r1.row == r2.row && here.col >= r1.col && here.col <= r2.col {
            return self.cell_scalar(
                s1,
                CellRef {
                    col: here.col,
                    row: r1.row,
                },
            );
        }
        Scalar::Err(VALUE)
    }

    fn sum_args(&mut self, vals: &[Value], sheet: usize) -> Scalar {
        let mut total = 0.0;
        for v in vals {
            match self.sum_value(v, sheet) {
                Scalar::Num(n) => total += n,
                Scalar::Err(e) => return Scalar::Err(e),
                Scalar::Bool(_) | Scalar::Text(_) => {}
            }
        }
        Scalar::Num(total)
    }

    fn sum_value(&mut self, v: &Value, sheet: usize) -> Scalar {
        match v {
            Value::Scalar(Scalar::Num(n)) => Scalar::Num(*n),
            Value::Scalar(Scalar::Err(e)) => Scalar::Err(e),
            Value::Scalar(Scalar::Bool(_) | Scalar::Text(_)) => Scalar::Num(0.0),
            Value::Area(rect) => self.sum_rect(rect, sheet),
            Value::Areas(parts) => {
                let mut total = 0.0;
                for part in parts {
                    match self.sum_rect(part, sheet) {
                        Scalar::Num(n) => total += n,
                        Scalar::Err(e) => return Scalar::Err(e),
                        _ => {}
                    }
                }
                Scalar::Num(total)
            }
            Value::Grid(g) => {
                let mut total = 0.0;
                for row in g {
                    for cell in row {
                        match cell {
                            Scalar::Num(n) => total += n,
                            Scalar::Err(e) => return Scalar::Err(e),
                            Scalar::Bool(_) | Scalar::Text(_) => {}
                        }
                    }
                }
                Scalar::Num(total)
            }
        }
    }

    fn sum_rect(&mut self, rect: &Rect, here: usize) -> Scalar {
        let (r1, r2) = ordered(rect.r1, rect.r2);
        let Some((s1, s2)) = self.resolve_span(rect, here) else {
            return Scalar::Err(REF);
        };
        let mut total = 0.0;
        for sheet in s1..=s2 {
            for row in r1.row..=r2.row {
                for col in r1.col..=r2.col {
                    let key = (col, row);
                    let present = self
                        .sheets
                        .get(sheet)
                        .is_some_and(|s| s.cells.contains_key(&key));
                    if !present {
                        continue;
                    }
                    match self.cell_scalar(sheet, CellRef { col, row }) {
                        Scalar::Num(n) => total += n,
                        Scalar::Err(e) => return Scalar::Err(e),
                        Scalar::Bool(_) | Scalar::Text(_) => {}
                    }
                }
            }
        }
        Scalar::Num(total)
    }

    fn cross_value(&self, left: Value, right: Value, here: usize) -> Value {
        let mut hits = Vec::new();
        for a in flatten_areas(left) {
            for b in flatten_areas(right.clone()) {
                if let Some(hit) = self.cross_rect(&a, &b, here) {
                    hits.push(hit);
                }
            }
        }
        match hits.len() {
            0 => Value::Scalar(Scalar::Err(NULL)),
            1 => Value::Area(hits.remove(0)),
            _ => Value::Areas(hits),
        }
    }

    fn cross_rect(&self, a: &Rect, b: &Rect, here: usize) -> Option<Rect> {
        let (as1, as2) = self.resolve_span(a, here)?;
        let (bs1, bs2) = self.resolve_span(b, here)?;
        let s1 = as1.max(bs1);
        let s2 = as2.min(bs2);
        if s1 > s2 {
            return None;
        }
        let (a1, a2) = ordered(a.r1, a.r2);
        let (b1, b2) = ordered(b.r1, b.r2);
        let c1 = a1.col.max(b1.col);
        let c2 = a2.col.min(b2.col);
        let r1 = a1.row.max(b1.row);
        let r2 = a2.row.min(b2.row);
        if c1 > c2 || r1 > r2 {
            return None;
        }
        Some(Rect {
            from: Loc::At(s1),
            to: Loc::At(s2),
            r1: CellRef { col: c1, row: r1 },
            r2: CellRef { col: c2, row: r2 },
        })
    }

    /// 文本类。不是文本函数时返回 `None`，交给数学类。
    fn eval_text(
        &mut self,
        name: &str,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
    ) -> Option<Calc> {
        let done = match name {
            "LEFT" => self.eval_side(vals, sheet, here, false, true),
            "RIGHT" => self.eval_side(vals, sheet, here, false, false),
            "LEFTB" => self.eval_side(vals, sheet, here, true, true),
            "RIGHTB" => self.eval_side(vals, sheet, here, true, false),
            "MID" => self.eval_mid(vals, sheet, here, false),
            "MIDB" => self.eval_mid(vals, sheet, here, true),
            "LEN" => self.eval_len(vals, sheet, here, false),
            "LENB" => self.eval_len(vals, sheet, here, true),
            "FIND" => self.eval_find(vals, sheet, here, false, false),
            "FINDB" => self.eval_find(vals, sheet, here, true, false),
            "SEARCH" => self.eval_find(vals, sheet, here, false, true),
            "SEARCHB" => self.eval_find(vals, sheet, here, true, true),
            "REPLACE" => self.eval_replace(vals, sheet, here, false),
            "REPLACEB" => self.eval_replace(vals, sheet, here, true),
            "SUBSTITUTE" => self.eval_substitute(vals, sheet, here),
            "LOWER" => self.eval_case(vals, sheet, here, CaseKind::Lower),
            "UPPER" => self.eval_case(vals, sheet, here, CaseKind::Upper),
            "PROPER" => self.eval_case(vals, sheet, here, CaseKind::Proper),
            "TRIM" => self.eval_trim(vals, sheet, here),
            "CLEAN" => self.eval_clean(vals, sheet, here),
            "REPT" => self.eval_rept(vals, sheet, here),
            "CONCATENATE" => self.eval_concatenate(vals, sheet, here),
            "EXACT" => self.eval_exact(vals, sheet, here),
            "T" => self.eval_t(vals, sheet, here),
            "VALUE" => self.eval_value_fn(vals, sheet, here),
            "CHAR" => self.eval_char(vals, sheet, here),
            "CODE" => self.eval_code(vals, sheet, here),
            "TEXT" => self.eval_text_fn(vals, sheet, here),
            "DOLLAR" => self.eval_dollar(vals, sheet, here),
            "FIXED" => self.eval_fixed(vals, sheet, here),
            "BAHTTEXT" => self.eval_baht(vals, sheet, here),
            "ASC" => self.eval_asc(vals, sheet, here),
            "PHONETIC" => self.eval_phonetic(vals, sheet, here),
            _ => return None,
        };
        Some(done)
    }

    fn eval_side(
        &mut self,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        bytes: bool,
        from_left: bool,
    ) -> Calc {
        if vals.is_empty() || vals.len() > 2 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let count = if vals.len() == 1 {
            1
        } else {
            match self.take_count(&vals[1], sheet, here) {
                Ok(count) => count,
                Err(done) => return done,
            }
        };
        if bytes {
            return side_bytes(&text, count, from_left);
        }
        ready_text(side_chars(&text, count, from_left))
    }

    fn eval_mid(&mut self, vals: &[Value], sheet: usize, here: CellRef, bytes: bool) -> Calc {
        if vals.len() != 3 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let start = match self.take_count(&vals[1], sheet, here) {
            Ok(start) => start,
            Err(done) => return done,
        };
        let count = match self.take_count(&vals[2], sheet, here) {
            Ok(count) => count,
            Err(done) => return done,
        };
        if start == 0 {
            return ready_err(VALUE);
        }
        let start0 = start - 1;
        if bytes {
            return mid_bytes(&text, start0, count);
        }
        ready_text(mid_chars(&text, start0, count))
    }

    fn eval_len(&mut self, vals: &[Value], sheet: usize, here: CellRef, bytes: bool) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        if bytes {
            let Some(units) = dbcs::encode(&text) else {
                return Calc::Skip;
            };
            return ready_num(units.len() as f64);
        }
        ready_num(text.encode_utf16().count() as f64)
    }

    fn eval_find(
        &mut self,
        vals: &[Value],
        sheet: usize,
        here: CellRef,
        bytes: bool,
        wild: bool,
    ) -> Calc {
        if vals.len() < 2 || vals.len() > 3 {
            return Calc::Skip;
        }
        let needle = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let hay = match self.take_text(&vals[1], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let start = if vals.len() == 2 {
            1
        } else {
            match self.take_count(&vals[2], sheet, here) {
                Ok(start) => start,
                Err(done) => return done,
            }
        };
        if start == 0 {
            return ready_err(VALUE);
        }
        let hay_units: Vec<u16> = hay.encode_utf16().collect();
        let needle_units: Vec<u16> = needle.encode_utf16().collect();
        let start0 = start - 1;
        if start0 > hay_units.len() {
            return ready_err(VALUE);
        }
        let found = if wild {
            find_wild(&hay_units, &needle_units, start0)
        } else {
            find_literal(&hay_units, &needle_units, start0)
        };
        let Some(index) = found else {
            return ready_err(VALUE);
        };
        if !bytes {
            return ready_num((index + 1) as f64);
        }
        match byte_index_at(&hay, index) {
            Some(pos) => ready_num((pos + 1) as f64),
            None => Calc::Skip,
        }
    }

    fn eval_replace(&mut self, vals: &[Value], sheet: usize, here: CellRef, bytes: bool) -> Calc {
        if vals.len() != 4 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let start = match self.take_count(&vals[1], sheet, here) {
            Ok(start) => start,
            Err(done) => return done,
        };
        let count = match self.take_count(&vals[2], sheet, here) {
            Ok(count) => count,
            Err(done) => return done,
        };
        let new_text = match self.take_text(&vals[3], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        if start == 0 {
            return ready_err(VALUE);
        }
        let start0 = start - 1;
        if bytes {
            return replace_bytes(&text, start0, count, &new_text);
        }
        ready_text(replace_chars(&text, start0, count, &new_text))
    }

    fn eval_substitute(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() < 3 || vals.len() > 4 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let old = match self.take_text(&vals[1], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let new_text = match self.take_text(&vals[2], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let instance = if vals.len() == 3 {
            None
        } else {
            match self.take_count(&vals[3], sheet, here) {
                Ok(0) => return ready_err(VALUE),
                Ok(n) => Some(n),
                Err(done) => return done,
            }
        };
        ready_text(substitute_text(&text, &old, &new_text, instance))
    }

    fn eval_case(&mut self, vals: &[Value], sheet: usize, here: CellRef, kind: CaseKind) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        ready_text(match kind {
            CaseKind::Lower => text.to_lowercase(),
            CaseKind::Upper => text.to_uppercase(),
            CaseKind::Proper => proper_case(&text),
        })
    }

    fn eval_trim(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let parts: Vec<&str> = text.split(' ').filter(|part| !part.is_empty()).collect();
        ready_text(parts.join(" "))
    }

    fn eval_clean(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        ready_text(text.chars().filter(|ch| (*ch as u32) >= 32).collect())
    }

    fn eval_rept(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let count = match self.take_count(&vals[1], sheet, here) {
            Ok(count) => count,
            Err(done) => return done,
        };
        let units = text.encode_utf16().count();
        if units.saturating_mul(count) > 32767 {
            return ready_err(VALUE);
        }
        ready_text(text.repeat(count))
    }

    fn eval_concatenate(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.is_empty() {
            return Calc::Skip;
        }
        let mut out = String::new();
        for value in vals {
            match self.take_text(value, sheet, here) {
                Ok(text) => out.push_str(&text),
                Err(done) => return done,
            }
        }
        ready_text(out)
    }

    fn eval_exact(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let left = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        let right = match self.take_text(&vals[1], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        ready_bool(left == right)
    }

    fn eval_t(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.text_hit(&vals[0], sheet, here) {
            Hit::Fact(Fact::Ready(Scalar::Text(text))) => ready_text(text),
            Hit::Fact(Fact::Ready(Scalar::Err(token))) | Hit::Err(token) => ready_err(token),
            Hit::Fact(Fact::Skip) => Calc::Skip,
            Hit::Fact(Fact::Blank | Fact::Ready(_)) => ready_text(String::new()),
        }
    }

    fn eval_value_fn(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.text_hit(&vals[0], sheet, here) {
            Hit::Fact(Fact::Blank) => ready_num(0.0),
            Hit::Fact(Fact::Ready(Scalar::Num(n))) => ready_num(n),
            Hit::Fact(Fact::Ready(Scalar::Text(text))) => parse_excel_number(&text),
            Hit::Fact(Fact::Ready(Scalar::Bool(_))) => ready_err(VALUE),
            Hit::Fact(Fact::Ready(Scalar::Err(token))) | Hit::Err(token) => ready_err(token),
            Hit::Fact(Fact::Skip) => Calc::Skip,
        }
    }

    fn eval_char(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let code = match self.take_count(&vals[0], sheet, here) {
            Ok(code) => code,
            Err(done) => return done,
        };
        if !(1..=65535).contains(&code) {
            return ready_err(VALUE);
        }
        match dbcs::char_of(code as u32) {
            Some(text) => ready_text(text),
            None => Calc::Skip,
        }
    }

    fn eval_code(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        if text.is_empty() {
            return ready_err(VALUE);
        }
        match dbcs::code_of(&text) {
            Some(code) => ready_num(code as f64),
            None => Calc::Skip,
        }
    }

    fn eval_text_fn(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 2 {
            return Calc::Skip;
        }
        let number = match self.take_num(&vals[0], sheet, here) {
            Ok(number) => number,
            Err(done) => return done,
        };
        let format = match self.take_text(&vals[1], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        match format_sample(number, &format) {
            Some(text) => ready_text(text),
            None => Calc::Skip,
        }
    }

    fn eval_dollar(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.is_empty() || vals.len() > 2 {
            return Calc::Skip;
        }
        let number = match self.take_num(&vals[0], sheet, here) {
            Ok(number) => number,
            Err(done) => return done,
        };
        let decimals = if vals.len() == 1 {
            2
        } else {
            match self.take_signed(&vals[1], sheet, here) {
                Ok(decimals) => decimals,
                Err(done) => return done,
            }
        };
        if !(0..=10).contains(&decimals) {
            return Calc::Skip;
        }
        let (neg, body) = format_fixed(number, decimals as usize, true);
        let text = if neg {
            format!("¥-{body}")
        } else {
            format!("¥{body}")
        };
        ready_text(text)
    }

    fn eval_fixed(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.is_empty() || vals.len() > 3 {
            return Calc::Skip;
        }
        let number = match self.take_num(&vals[0], sheet, here) {
            Ok(number) => number,
            Err(done) => return done,
        };
        let decimals = if vals.len() == 1 {
            2
        } else {
            match self.take_signed(&vals[1], sheet, here) {
                Ok(decimals) => decimals,
                Err(done) => return done,
            }
        };
        if !(0..=10).contains(&decimals) {
            return Calc::Skip;
        }
        let no_comma = if vals.len() == 3 {
            match self.take_flag(&vals[2], sheet, here) {
                Ok(flag) => flag,
                Err(done) => return done,
            }
        } else {
            false
        };
        let (neg, body) = format_fixed(number, decimals as usize, !no_comma);
        let text = if neg { format!("-{body}") } else { body };
        ready_text(text)
    }

    fn eval_baht(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let number = match self.take_num(&vals[0], sheet, here) {
            Ok(number) => number,
            Err(done) => return done,
        };
        match baht_text(number) {
            Some(text) => ready_text(text),
            None => Calc::Skip,
        }
    }

    fn eval_asc(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        let text = match self.take_text(&vals[0], sheet, here) {
            Ok(text) => text,
            Err(done) => return done,
        };
        ready_text(asc_text(&text))
    }

    fn eval_phonetic(&mut self, vals: &[Value], sheet: usize, here: CellRef) -> Calc {
        if vals.len() != 1 {
            return Calc::Skip;
        }
        match self.take_text(&vals[0], sheet, here) {
            Ok(text) => ready_text(text),
            Err(done) => done,
        }
    }

    fn text_hit(&mut self, value: &Value, sheet: usize, here: CellRef) -> Hit {
        self.hit_value(value.clone(), sheet, here)
    }

    fn take_text(&mut self, value: &Value, sheet: usize, here: CellRef) -> Result<String, Calc> {
        match self.text_hit(value, sheet, here) {
            Hit::Fact(Fact::Blank) => Ok(String::new()),
            Hit::Fact(Fact::Ready(Scalar::Text(text))) => Ok(text),
            Hit::Fact(Fact::Ready(Scalar::Num(n))) => Ok(fmt_num(n)),
            Hit::Fact(Fact::Ready(Scalar::Bool(flag))) => {
                Ok(if flag { "TRUE" } else { "FALSE" }.to_string())
            }
            Hit::Fact(Fact::Ready(Scalar::Err(token))) | Hit::Err(token) => Err(ready_err(token)),
            Hit::Fact(Fact::Skip) => Err(Calc::Skip),
        }
    }

    fn take_num(&mut self, value: &Value, sheet: usize, here: CellRef) -> Result<f64, Calc> {
        match self.text_hit(value, sheet, here) {
            Hit::Fact(Fact::Blank) => Ok(0.0),
            Hit::Fact(Fact::Ready(scalar)) => match num_of(scalar) {
                Scalar::Num(n) => Ok(n),
                Scalar::Err(token) => Err(ready_err(token)),
                _ => Err(ready_err(VALUE)),
            },
            Hit::Err(token) => Err(ready_err(token)),
            Hit::Fact(Fact::Skip) => Err(Calc::Skip),
        }
    }

    fn take_count(&mut self, value: &Value, sheet: usize, here: CellRef) -> Result<usize, Calc> {
        let number = self.take_num(value, sheet, here)?;
        if !(0.0..=1_000_000.0).contains(&number) {
            return Err(ready_err(VALUE));
        }
        Ok(number.trunc() as usize)
    }

    fn take_signed(&mut self, value: &Value, sheet: usize, here: CellRef) -> Result<i32, Calc> {
        let number = self.take_num(value, sheet, here)?;
        if !number.is_finite() || !(0.0..=1_000_000.0).contains(&number.abs()) {
            return Err(ready_err(VALUE));
        }
        Ok(number.trunc() as i32)
    }

    fn take_flag(&mut self, value: &Value, sheet: usize, here: CellRef) -> Result<bool, Calc> {
        match self.text_hit(value, sheet, here) {
            Hit::Fact(Fact::Blank) => Ok(false),
            Hit::Fact(Fact::Ready(Scalar::Bool(flag))) => Ok(flag),
            Hit::Fact(Fact::Ready(Scalar::Num(n))) => Ok(n != 0.0),
            Hit::Fact(Fact::Ready(Scalar::Text(text))) => {
                if text.eq_ignore_ascii_case("TRUE") {
                    Ok(true)
                } else if text.eq_ignore_ascii_case("FALSE") {
                    Ok(false)
                } else {
                    Err(ready_err(VALUE))
                }
            }
            Hit::Fact(Fact::Ready(Scalar::Err(token))) | Hit::Err(token) => Err(ready_err(token)),
            Hit::Fact(Fact::Skip) => Err(Calc::Skip),
        }
    }
}

enum CaseKind {
    Lower,
    Upper,
    Proper,
}

fn ready_text(text: String) -> Calc {
    Calc::Ready(Value::Scalar(Scalar::Text(text)))
}

fn xml_text_ok(text: &str) -> bool {
    text.chars().all(|ch| {
        let code = ch as u32;
        code == 0x9 || code == 0xA || code == 0xD || code >= 0x20
    })
}

fn side_chars(text: &str, count: usize, from_left: bool) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let count = count.min(units.len());
    let slice = if from_left {
        &units[..count]
    } else {
        &units[units.len() - count..]
    };
    String::from_utf16_lossy(slice)
}

fn side_bytes(text: &str, count: usize, from_left: bool) -> Calc {
    let Some(units) = dbcs::encode(text) else {
        return Calc::Skip;
    };
    let count = count.min(units.len());
    let start = if from_left { 0 } else { units.len() - count };
    match dbcs::decode(&dbcs::slice(&units, start, count)) {
        Some(text) => ready_text(text),
        None => Calc::Skip,
    }
}

fn mid_chars(text: &str, start0: usize, count: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    if start0 >= units.len() || count == 0 {
        return String::new();
    }
    let end = start0.saturating_add(count).min(units.len());
    String::from_utf16_lossy(&units[start0..end])
}

fn mid_bytes(text: &str, start0: usize, count: usize) -> Calc {
    let Some(units) = dbcs::encode(text) else {
        return Calc::Skip;
    };
    if start0 >= units.len() || count == 0 {
        return ready_text(String::new());
    }
    match dbcs::decode(&dbcs::slice(&units, start0, count)) {
        Some(text) => ready_text(text),
        None => Calc::Skip,
    }
}

fn find_literal(hay: &[u16], needle: &[u16], start0: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(start0);
    }
    hay[start0..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|pos| pos + start0)
}

fn find_wild(hay: &[u16], needle: &[u16], start0: usize) -> Option<usize> {
    (start0..=hay.len()).find(|&index| wild_prefix(&hay[index..], needle))
}

fn wild_prefix(hay: &[u16], pat: &[u16]) -> bool {
    if pat.is_empty() {
        return true;
    }
    let mut hi = 0;
    let mut pi = 0;
    let mut star_p: Option<usize> = None;
    let mut star_h = 0;
    loop {
        if pi == pat.len() {
            return true;
        }
        if pat[pi] == u16::from(b'*') {
            star_p = Some(pi);
            star_h = hi;
            pi += 1;
            if pi == pat.len() {
                return true;
            }
            continue;
        }
        if hi < hay.len() && (pat[pi] == u16::from(b'?') || eq_ci(hay[hi], pat[pi])) {
            hi += 1;
            pi += 1;
            continue;
        }
        let Some(sp) = star_p else {
            return false;
        };
        if star_h >= hay.len() {
            return false;
        }
        star_h += 1;
        hi = star_h;
        pi = sp + 1;
    }
}

fn eq_ci(left: u16, right: u16) -> bool {
    if left == right {
        return true;
    }
    if left < 128 && right < 128 {
        return (left as u8).eq_ignore_ascii_case(&(right as u8));
    }
    let Some(a) = char::from_u32(u32::from(left)) else {
        return false;
    };
    let Some(b) = char::from_u32(u32::from(right)) else {
        return false;
    };
    a.to_lowercase().to_string() == b.to_lowercase().to_string()
}

fn byte_index_at(text: &str, utf16_index: usize) -> Option<usize> {
    let mut bytes = 0usize;
    let mut seen = 0usize;
    for ch in text.chars() {
        if seen == utf16_index {
            return Some(bytes);
        }
        let part = dbcs::encode(&ch.to_string())?;
        bytes += part.len();
        seen += ch.len_utf16();
    }
    if seen == utf16_index {
        Some(bytes)
    } else {
        None
    }
}

fn replace_chars(text: &str, start0: usize, count: usize, new_text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let new_units: Vec<u16> = new_text.encode_utf16().collect();
    let mut out = Vec::new();
    if start0 >= units.len() {
        out.extend_from_slice(&units);
        out.extend_from_slice(&new_units);
    } else {
        out.extend_from_slice(&units[..start0]);
        out.extend_from_slice(&new_units);
        let end = start0.saturating_add(count).min(units.len());
        out.extend_from_slice(&units[end..]);
    }
    String::from_utf16_lossy(&out)
}

fn replace_bytes(text: &str, start0: usize, count: usize, new_text: &str) -> Calc {
    let Some(units) = dbcs::encode(text) else {
        return Calc::Skip;
    };
    let Some(new_units) = dbcs::encode(new_text) else {
        return Calc::Skip;
    };
    let replaced = if start0 >= units.len() {
        let mut out = units;
        out.extend(new_units);
        out
    } else {
        dbcs::replace(&units, start0, count, &new_units)
    };
    match dbcs::decode(&replaced) {
        Some(text) => ready_text(text),
        None => Calc::Skip,
    }
}

fn substitute_text(text: &str, old: &str, new_text: &str, instance: Option<usize>) -> String {
    if old.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut rest = text;
    let mut seen = 0usize;
    while let Some(pos) = rest.find(old) {
        seen += 1;
        let replace_this = match instance {
            None => true,
            Some(n) => n == seen,
        };
        out.push_str(&rest[..pos]);
        if replace_this {
            out.push_str(new_text);
        } else {
            out.push_str(old);
        }
        rest = &rest[pos + old.len()..];
        if instance == Some(seen) {
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn proper_case(text: &str) -> String {
    let mut out = String::new();
    let mut cap = true;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            if cap {
                out.extend(ch.to_uppercase());
                cap = false;
            } else {
                out.extend(ch.to_lowercase());
            }
        } else {
            out.push(ch);
            cap = true;
        }
    }
    out
}

fn parse_excel_number(text: &str) -> Calc {
    let trimmed = text.trim().replace(',', "");
    if trimmed.is_empty() {
        return ready_err(VALUE);
    }
    match trimmed.parse::<f64>() {
        Ok(n) if n.is_finite() => ready_num(n),
        _ => ready_err(VALUE),
    }
}

fn format_sample(number: f64, format: &str) -> Option<String> {
    if format.is_empty()
        || !format
            .chars()
            .all(|ch| matches!(ch, '0' | '#' | ',' | '.' | '%'))
    {
        return None;
    }
    let percent = format.contains('%');
    let body: String = format.chars().filter(|ch| *ch != '%').collect();
    if body.is_empty() {
        return None;
    }
    let (int_part, decimals) = match body.split_once('.') {
        Some((int_part, frac)) => {
            if !frac.chars().all(|ch| ch == '0' || ch == '#') {
                return None;
            }
            (int_part, frac.len())
        }
        None => (body.as_str(), 0),
    };
    let shown = if percent { number * 100.0 } else { number };
    let (neg, text) = format_fixed(shown, decimals, int_part.contains(','));
    let mut out = if neg { format!("-{text}") } else { text };
    if percent {
        out.push('%');
    }
    Some(out)
}

fn format_fixed(number: f64, decimals: usize, comma: bool) -> (bool, String) {
    let rounded = match round_of(number, decimals as f64, 0) {
        Scalar::Num(n) => n,
        _ => number,
    };
    let neg = rounded < 0.0;
    let abs = rounded.abs();
    let raw = if decimals == 0 {
        format!("{}", abs.round() as u64)
    } else {
        format!("{abs:.decimals$}")
    };
    let body = if comma { with_commas(&raw) } else { raw };
    (neg, body)
}

fn with_commas(raw: &str) -> String {
    let (int_part, frac) = match raw.split_once('.') {
        Some((int_part, frac)) => (int_part, Some(frac)),
        None => (raw, None),
    };
    let digits: Vec<char> = int_part.chars().collect();
    let mut grouped = String::new();
    for (i, ch) in digits.iter().rev().enumerate() {
        if i > 0 && i.is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(*ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    match frac {
        Some(frac) => format!("{grouped}.{frac}"),
        None => grouped,
    }
}

fn baht_text(number: f64) -> Option<String> {
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let rounded = number.round();
    if (number - rounded).abs() > 1e-9 || rounded > 1e15 {
        return None;
    }
    let whole = rounded as u64;
    if whole == 0 {
        return Some("ศูนย์บาทถ้วน".to_string());
    }
    Some(format!("{}บาทถ้วน", baht_int(whole)))
}

fn baht_int(mut number: u64) -> String {
    let mut groups = Vec::new();
    while number > 0 {
        groups.push(number % 1_000_000);
        number /= 1_000_000;
    }
    let mut parts = Vec::new();
    for (i, group) in groups.iter().enumerate().rev() {
        if *group == 0 {
            continue;
        }
        let mut text = baht_group(*group);
        for _ in 0..i {
            text.push_str("ล้าน");
        }
        parts.push(text);
    }
    parts.concat()
}

fn baht_group(number: u64) -> String {
    const DIGIT: [&str; 10] = [
        "ศูนย์",
        "หนึ่ง",
        "สอง",
        "สาม",
        "สี่",
        "ห้า",
        "หก",
        "เจ็ด",
        "แปด",
        "เก้า",
    ];
    let d = [
        (number / 100_000) % 10,
        (number / 10_000) % 10,
        (number / 1_000) % 10,
        (number / 100) % 10,
        (number / 10) % 10,
        number % 10,
    ];
    let place = ["แสน", "หมื่น", "พัน", "ร้อย", "สิบ", ""];
    let mut text = String::new();
    for i in 0..6 {
        let digit = d[i] as usize;
        if digit == 0 {
            continue;
        }
        if i == 4 {
            if digit == 1 {
                text.push_str("สิบ");
            } else if digit == 2 {
                text.push_str("ยี่สิบ");
            } else {
                text.push_str(DIGIT[digit]);
                text.push_str("สิบ");
            }
        } else if i == 5 {
            if digit == 1 && d[4] != 0 {
                text.push_str("เอ็ด");
            } else {
                text.push_str(DIGIT[digit]);
            }
        } else {
            text.push_str(DIGIT[digit]);
            text.push_str(place[i]);
        }
    }
    text
}

fn asc_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch == '\u{3000}' {
                ' '
            } else if ('\u{FF01}'..='\u{FF5E}').contains(&ch) {
                char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect()
}

fn acos_of(n: f64) -> Scalar {
    if (-1.0..=1.0).contains(&n) {
        Scalar::Num(n.acos())
    } else {
        Scalar::Err(NUM)
    }
}

fn asin_of(n: f64) -> Scalar {
    if (-1.0..=1.0).contains(&n) {
        Scalar::Num(n.asin())
    } else {
        Scalar::Err(NUM)
    }
}

fn atan_of(n: f64) -> Scalar {
    finite_or(n.atan())
}

fn atan2_of(x: f64, y: f64) -> Scalar {
    if x == 0.0 && y == 0.0 {
        Scalar::Err(DIV0)
    } else {
        finite_or(y.atan2(x))
    }
}

fn cos_of(n: f64) -> Scalar {
    finite_or(n.cos())
}

fn sin_of(n: f64) -> Scalar {
    finite_or(n.sin())
}

fn tan_of(n: f64) -> Scalar {
    let v = n.tan();
    if v.is_finite() {
        Scalar::Num(v)
    } else {
        Scalar::Err(NUM)
    }
}

fn acosh_of(n: f64) -> Scalar {
    if n >= 1.0 {
        finite_or(n.acosh())
    } else {
        Scalar::Err(NUM)
    }
}

fn asinh_of(n: f64) -> Scalar {
    finite_or(n.asinh())
}

fn atanh_of(n: f64) -> Scalar {
    if n > -1.0 && n < 1.0 {
        finite_or(n.atanh())
    } else {
        Scalar::Err(NUM)
    }
}

fn cosh_of(n: f64) -> Scalar {
    finite_or(n.cosh())
}

fn sinh_of(n: f64) -> Scalar {
    finite_or(n.sinh())
}

fn tanh_of(n: f64) -> Scalar {
    finite_or(n.tanh())
}

fn exp_of(n: f64) -> Scalar {
    finite_or(n.exp())
}

fn ln_of(n: f64) -> Scalar {
    if n > 0.0 {
        finite_or(n.ln())
    } else {
        Scalar::Err(NUM)
    }
}

fn log10_of(n: f64) -> Scalar {
    if n > 0.0 {
        finite_or(n.log10())
    } else {
        Scalar::Err(NUM)
    }
}

fn log_of(n: f64, base: f64) -> Scalar {
    if n <= 0.0 || base <= 0.0 || base == 1.0 {
        Scalar::Err(NUM)
    } else {
        finite_or(n.log(base))
    }
}

fn sqrtpi_of(n: f64) -> Scalar {
    if n < 0.0 {
        Scalar::Err(NUM)
    } else {
        finite_or((n * std::f64::consts::PI).sqrt())
    }
}

fn degrees_of(n: f64) -> Scalar {
    Scalar::Num(n * 180.0 / std::f64::consts::PI)
}

fn radians_of(n: f64) -> Scalar {
    Scalar::Num(n * std::f64::consts::PI / 180.0)
}

fn sign_of(n: f64) -> Scalar {
    Scalar::Num(if n > 0.0 {
        1.0
    } else if n < 0.0 {
        -1.0
    } else {
        0.0
    })
}

fn power_of(base: f64, exp: f64) -> Scalar {
    if base < 0.0 && exp.fract() != 0.0 {
        return Scalar::Err(NUM);
    }
    if base == 0.0 && exp < 0.0 {
        return Scalar::Err(DIV0);
    }
    let exp_i = exp as i32;
    let value = if base < 0.0 && exp == f64::from(exp_i) {
        base.powi(exp_i)
    } else {
        base.powf(exp)
    };
    finite_or(value)
}

fn mod_of(n: f64, d: f64) -> Scalar {
    if d == 0.0 {
        return Scalar::Err(DIV0);
    }
    let q = (n / d).floor();
    finite_or(n - d * q)
}

fn quotient_of(n: f64, d: f64) -> Scalar {
    if d == 0.0 {
        Scalar::Err(DIV0)
    } else {
        finite_or((n / d).trunc())
    }
}

fn ceiling_of(number: f64, significance: f64) -> Scalar {
    if significance == 0.0 {
        return Scalar::Num(0.0);
    }
    if number > 0.0 && significance < 0.0 {
        return Scalar::Err(NUM);
    }
    finite_or(significance * (number / significance).ceil())
}

fn iso_ceiling_of(number: f64, significance: f64) -> Scalar {
    if significance == 0.0 {
        return Scalar::Num(0.0);
    }
    let step = significance.abs();
    finite_or(step * (number / step).ceil())
}

fn floor_of(number: f64, significance: f64) -> Scalar {
    if significance == 0.0 {
        return if number == 0.0 {
            Scalar::Num(0.0)
        } else {
            Scalar::Err(DIV0)
        };
    }
    if number > 0.0 && significance < 0.0 {
        return Scalar::Err(NUM);
    }
    finite_or(significance * (number / significance).floor())
}

fn mround_of(number: f64, significance: f64) -> Scalar {
    if number == 0.0 || significance == 0.0 {
        return Scalar::Num(0.0);
    }
    if number.signum() != significance.signum() {
        return Scalar::Err(NUM);
    }
    finite_or(significance * (number / significance).round())
}

fn round_of(number: f64, digits: f64, mode: i32) -> Scalar {
    let factor = 10_f64.powf(digits.trunc());
    if factor == 0.0 || !factor.is_finite() {
        return Scalar::Err(NUM);
    }
    let scaled = number * factor;
    // 二进制小数会略小于十进制边界，补一个很小的量才和 Excel 的 5 进位一致。
    let scaled = scaled + scaled.signum() * 1e-10;
    let whole = match mode {
        1 => scaled.signum() * scaled.abs().ceil(),
        -1 => scaled.signum() * scaled.abs().floor(),
        _ => scaled.signum() * scaled.abs().round(),
    };
    finite_or(whole / factor)
}

fn trunc_digits(number: f64, digits: f64) -> Scalar {
    round_of(number, digits, -1)
}

fn int_of(n: f64) -> Scalar {
    finite_or(n.floor())
}

fn even_of(n: f64) -> Scalar {
    if n == 0.0 {
        return Scalar::Num(0.0);
    }
    let mag = (n.abs() / 2.0).ceil() * 2.0;
    finite_or(n.signum() * mag)
}

fn odd_of(n: f64) -> Scalar {
    if n == 0.0 {
        return Scalar::Num(1.0);
    }
    let mag = n.abs();
    let next = if mag.ceil() == mag && mag % 2.0 != 0.0 {
        mag
    } else {
        (mag / 2.0).floor() * 2.0 + 1.0
    };
    let next = if next < mag { next + 2.0 } else { next };
    finite_or(n.signum() * next)
}

fn fact_of(n: f64) -> Scalar {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return Scalar::Err(NUM);
    }
    let mut acc = 1.0;
    for k in 2..=(n as i32) {
        acc *= f64::from(k);
    }
    finite_or(acc)
}

fn fact_double_of(n: f64) -> Scalar {
    if n < 0.0 || n.fract() != 0.0 {
        return Scalar::Err(NUM);
    }
    let mut acc = 1.0;
    let mut k = n as i32;
    while k > 0 {
        acc *= f64::from(k);
        k -= 2;
    }
    finite_or(acc)
}

fn combin_of(n: f64, k: f64) -> Scalar {
    if n < 0.0 || k < 0.0 || n.fract() != 0.0 || k.fract() != 0.0 || k > n {
        return Scalar::Err(NUM);
    }
    let n = n as u64;
    let k = k as u64;
    let k = k.min(n - k);
    let mut acc = 1.0;
    for i in 0..k {
        acc *= (n - i) as f64;
        acc /= (i + 1) as f64;
    }
    finite_or(acc)
}

fn finite_or(n: f64) -> Scalar {
    if n.is_finite() {
        Scalar::Num(n)
    } else {
        Scalar::Err(NUM)
    }
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn lcm_u64(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd_u64(a, b) * b
    }
}

fn multinomial_f64(total: u64, parts: &[u64]) -> f64 {
    let mut acc = 1.0;
    let mut used = 0u64;
    for part in parts {
        for i in 1..=*part {
            used += 1;
            acc *= used as f64;
            acc /= i as f64;
        }
    }
    let _ = total;
    acc
}

fn roman_classic(n: f64) -> Option<String> {
    if !(1.0..4000.0).contains(&n) || n.fract() != 0.0 {
        return None;
    }
    let mut n = n as i32;
    const PARTS: &[(i32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (value, glyph) in PARTS {
        while n >= *value {
            out.push_str(glyph);
            n -= value;
        }
    }
    Some(out)
}

fn criteria_match(cell: &Fact, criteria: &Scalar) -> bool {
    let value = match cell {
        Fact::Ready(s) => s,
        Fact::Blank | Fact::Skip => return false,
    };
    match criteria {
        Scalar::Num(n) => matches!(value, Scalar::Num(v) if (*v - n).abs() < 1e-9),
        Scalar::Bool(b) => matches!(value, Scalar::Bool(v) if v == b),
        Scalar::Err(e) => matches!(value, Scalar::Err(v) if v == e),
        Scalar::Text(text) => text_criteria(value, text),
    }
}

fn text_criteria(value: &Scalar, text: &str) -> bool {
    let (op, rest) = criteria_op(text);
    if op.is_empty() {
        return match value {
            Scalar::Text(v) => v.eq_ignore_ascii_case(text),
            Scalar::Num(n) => text
                .parse::<f64>()
                .ok()
                .is_some_and(|c| (n - c).abs() < 1e-9),
            _ => false,
        };
    }
    let rest = rest.trim();
    match value {
        Scalar::Num(n) => rest.parse::<f64>().ok().is_some_and(|c| cmp_num(op, *n, c)),
        Scalar::Text(v) => cmp_text(op, v, rest),
        _ => false,
    }
}

fn criteria_op(text: &str) -> (&str, &str) {
    if let Some(rest) = text.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = text.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = text.strip_prefix("<>") {
        ("<>", rest)
    } else if let Some(rest) = text.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = text.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = text.strip_prefix('=') {
        ("=", rest)
    } else {
        ("", text)
    }
}

fn cmp_num(op: &str, left: f64, right: f64) -> bool {
    match op {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        "<>" => (left - right).abs() >= 1e-9,
        _ => (left - right).abs() < 1e-9,
    }
}

fn cmp_text(op: &str, left: &str, right: &str) -> bool {
    let order = left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase());
    match op {
        ">" => order.is_gt(),
        ">=" => order.is_ge(),
        "<" => order.is_lt(),
        "<=" => order.is_le(),
        "<>" => order.is_ne(),
        _ => order.is_eq(),
    }
}

fn subtotal_finish(kind: f64, nums: &[f64], count_a: f64) -> Scalar {
    let count = nums.len() as f64;
    match kind as i32 {
        1 => {
            if count == 0.0 {
                Scalar::Err(DIV0)
            } else {
                finite_or(nums.iter().sum::<f64>() / count)
            }
        }
        2 => Scalar::Num(count),
        3 => Scalar::Num(count_a),
        4 => nums
            .iter()
            .copied()
            .reduce(f64::max)
            .map_or(Scalar::Num(0.0), finite_or),
        5 => nums
            .iter()
            .copied()
            .reduce(f64::min)
            .map_or(Scalar::Num(0.0), finite_or),
        6 => {
            if nums.is_empty() {
                Scalar::Num(0.0)
            } else {
                finite_or(nums.iter().product())
            }
        }
        7 => stat_or(stdev(nums, true)),
        8 => stat_or(stdev(nums, false)),
        9 => finite_or(nums.iter().sum()),
        10 => stat_or(variance(nums, true)),
        11 => stat_or(variance(nums, false)),
        _ => Scalar::Err(VALUE),
    }
}

fn stat_or(n: f64) -> Scalar {
    if n.is_finite() {
        Scalar::Num(n)
    } else {
        Scalar::Err(DIV0)
    }
}

fn variance(nums: &[f64], sample: bool) -> f64 {
    let denom = nums.len() as f64 - if sample { 1.0 } else { 0.0 };
    if denom <= 0.0 {
        return f64::NAN;
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / denom
}

fn stdev(nums: &[f64], sample: bool) -> f64 {
    variance(nums, sample).sqrt()
}

fn square(grid: &[Vec<f64>]) -> bool {
    !grid.is_empty() && grid.iter().all(|row| row.len() == grid.len())
}

fn num_grid(grid: Vec<Vec<f64>>) -> Vec<Vec<Scalar>> {
    grid.into_iter()
        .map(|row| row.into_iter().map(Scalar::Num).collect())
        .collect()
}

fn mat_mul(left: &[Vec<f64>], right: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let n = left[0].len();
    if right.len() != n || left.iter().any(|row| row.len() != n) {
        return None;
    }
    let p = right[0].len();
    if right.iter().any(|row| row.len() != p) {
        return None;
    }
    let mut out = vec![vec![0.0; p]; left.len()];
    for (i, row) in left.iter().enumerate() {
        for (k, &a) in row.iter().enumerate() {
            for j in 0..p {
                out[i][j] += a * right[k][j];
            }
        }
    }
    Some(out)
}

#[allow(clippy::needless_range_loop)]
fn mat_det(mut grid: Vec<Vec<f64>>) -> f64 {
    let n = grid.len();
    let mut sign = 1.0;
    for i in 0..n {
        let mut pivot = i;
        for row in i + 1..n {
            if grid[row][i].abs() > grid[pivot][i].abs() {
                pivot = row;
            }
        }
        if grid[pivot][i].abs() < 1e-12 {
            return 0.0;
        }
        if pivot != i {
            grid.swap(i, pivot);
            sign = -sign;
        }
        for row in i + 1..n {
            let factor = grid[row][i] / grid[i][i];
            for col in i..n {
                let next = grid[row][col] - factor * grid[i][col];
                grid[row][col] = next;
            }
        }
    }
    let mut det = sign;
    for i in 0..n {
        det *= grid[i][i];
    }
    det
}

#[allow(clippy::needless_range_loop)]
fn mat_inv(mut grid: Vec<Vec<f64>>) -> Option<Vec<Vec<f64>>> {
    let n = grid.len();
    let mut inv = vec![vec![0.0; n]; n];
    for (i, row) in inv.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for i in 0..n {
        let mut pivot = i;
        for row in i + 1..n {
            if grid[row][i].abs() > grid[pivot][i].abs() {
                pivot = row;
            }
        }
        if grid[pivot][i].abs() < 1e-12 {
            return None;
        }
        if pivot != i {
            grid.swap(i, pivot);
            inv.swap(i, pivot);
        }
        let div = grid[i][i];
        for col in 0..n {
            grid[i][col] /= div;
            inv[i][col] /= div;
        }
        for row in 0..n {
            if row == i {
                continue;
            }
            let factor = grid[row][i];
            for col in 0..n {
                grid[row][col] -= factor * grid[i][col];
                inv[row][col] -= factor * inv[i][col];
            }
        }
    }
    Some(inv)
}

fn unit_random() -> f64 {
    (random_u64() >> 11) as f64 / ((1u64 << 53) as f64)
}

fn random_u64() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0x6a09_e667_f3bc_c909);
    let mut x = STATE.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x.wrapping_mul(0x94d0_49bb_1331_11eb)
}

fn ready_bool(value: bool) -> Calc {
    Calc::Ready(Value::Scalar(Scalar::Bool(value)))
}

fn ready_num(value: f64) -> Calc {
    Calc::Ready(Value::Scalar(Scalar::Num(value)))
}

fn ready_err(token: &'static str) -> Calc {
    Calc::Ready(Value::Scalar(Scalar::Err(token)))
}

fn error_of(hit: &Hit) -> Option<&'static str> {
    match hit {
        Hit::Fact(Fact::Ready(Scalar::Err(e))) | Hit::Err(e) => Some(*e),
        _ => None,
    }
}

fn error_type_code(token: &str) -> Option<f64> {
    match token {
        NULL => Some(1.0),
        DIV0 => Some(2.0),
        VALUE => Some(3.0),
        REF => Some(4.0),
        NAME => Some(5.0),
        NUM => Some(6.0),
        NA => Some(7.0),
        _ => None,
    }
}

fn n_convert(value: Scalar) -> Scalar {
    match value {
        Scalar::Num(n) => Scalar::Num(n),
        Scalar::Bool(true) => Scalar::Num(1.0),
        Scalar::Bool(false) => Scalar::Num(0.0),
        Scalar::Text(_) => Scalar::Num(0.0),
        Scalar::Err(e) => Scalar::Err(e),
    }
}

fn type_code(value: &Scalar) -> f64 {
    match value {
        Scalar::Num(_) => 1.0,
        Scalar::Text(_) => 2.0,
        Scalar::Bool(_) => 4.0,
        Scalar::Err(_) => 16.0,
    }
}

fn num_of(s: Scalar) -> Scalar {
    match s {
        Scalar::Num(n) => Scalar::Num(n),
        Scalar::Bool(b) => Scalar::Num(if b { 1.0 } else { 0.0 }),
        Scalar::Err(e) => Scalar::Err(e),
        Scalar::Text(_) => Scalar::Err(VALUE),
    }
}

fn other_map(s: Scalar, f: impl Fn(f64) -> f64) -> Scalar {
    match s {
        Scalar::Num(n) => Scalar::Num(f(n)),
        other => other,
    }
}

fn apply_bin(op: BinOp, left: Scalar, right: Scalar) -> Scalar {
    if let Scalar::Err(e) = &left {
        return Scalar::Err(e);
    }
    if let Scalar::Err(e) = &right {
        return Scalar::Err(e);
    }
    if matches!(op, BinOp::Cat) {
        return Scalar::Text(format!("{}{}", as_text(&left), as_text(&right)));
    }
    if matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    ) {
        return cmp_bin(op, left, right);
    }
    let a = num_of(left);
    let b = num_of(right);
    let (a, b) = match (a, b) {
        (Scalar::Num(a), Scalar::Num(b)) => (a, b),
        (Scalar::Err(e), _) | (_, Scalar::Err(e)) => return Scalar::Err(e),
        _ => return Scalar::Err(VALUE),
    };
    match op {
        BinOp::Add => Scalar::Num(a + b),
        BinOp::Sub => Scalar::Num(a - b),
        BinOp::Mul => Scalar::Num(a * b),
        BinOp::Div => {
            if b == 0.0 {
                Scalar::Err(DIV0)
            } else {
                Scalar::Num(a / b)
            }
        }
        BinOp::Pow => Scalar::Num(a.powf(b)),
        BinOp::Cat | BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            Scalar::Err(VALUE)
        }
    }
}

fn cmp_bin(op: BinOp, left: Scalar, right: Scalar) -> Scalar {
    let ord = match (&left, &right) {
        (Scalar::Text(a), Scalar::Text(b)) => Some(a.cmp(b)),
        (Scalar::Bool(a), Scalar::Bool(b)) => Some(a.cmp(b)),
        _ => match (num_of(left), num_of(right)) {
            (Scalar::Num(a), Scalar::Num(b)) => a.partial_cmp(&b),
            (Scalar::Err(e), _) | (_, Scalar::Err(e)) => return Scalar::Err(e),
            _ => return Scalar::Err(VALUE),
        },
    };
    let Some(ord) = ord else {
        return Scalar::Err(VALUE);
    };
    let ok = match op {
        BinOp::Eq => ord.is_eq(),
        BinOp::Ne => ord.is_ne(),
        BinOp::Lt => ord.is_lt(),
        BinOp::Le => ord.is_le(),
        BinOp::Gt => ord.is_gt(),
        BinOp::Ge => ord.is_ge(),
        _ => false,
    };
    Scalar::Bool(ok)
}

fn as_text(s: &Scalar) -> String {
    match s {
        Scalar::Num(n) => fmt_num(*n),
        Scalar::Text(t) => t.clone(),
        Scalar::Bool(true) => "TRUE".to_string(),
        Scalar::Bool(false) => "FALSE".to_string(),
        Scalar::Err(e) => (*e).to_string(),
    }
}

fn union_value(left: Value, right: Value) -> Value {
    Value::Areas(
        flatten_areas(left)
            .into_iter()
            .chain(flatten_areas(right))
            .collect(),
    )
}

fn flatten_areas(v: Value) -> Vec<Rect> {
    match v {
        Value::Area(r) => vec![r],
        Value::Areas(v) => v,
        Value::Scalar(_) | Value::Grid(_) => Vec::new(),
    }
}

fn ordered(a: CellRef, b: CellRef) -> (CellRef, CellRef) {
    (
        CellRef {
            col: a.col.min(b.col),
            row: a.row.min(b.row),
        },
        CellRef {
            col: a.col.max(b.col),
            row: a.row.max(b.row),
        },
    )
}

fn ord_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn map_grid(grid: Vec<Vec<Scalar>>, f: impl Fn(Scalar) -> Scalar) -> Vec<Vec<Scalar>> {
    grid.into_iter()
        .map(|row| row.into_iter().map(&f).collect())
        .collect()
}

fn zip_grid(
    left: Vec<Vec<Scalar>>,
    right: Vec<Vec<Scalar>>,
    f: impl Fn(Scalar, Scalar) -> Scalar,
) -> Option<Vec<Vec<Scalar>>> {
    if left.len() != right.len() {
        return None;
    }
    let mut out = Vec::new();
    for (a, b) in left.into_iter().zip(right) {
        if a.len() != b.len() {
            return None;
        }
        out.push(a.into_iter().zip(b).map(|(x, y)| f(x, y)).collect());
    }
    Some(out)
}

fn fmt_num(n: f64) -> String {
    if !n.is_finite() {
        return NUM.to_string();
    }
    if (n - n.round()).abs() < 1e-9 && n.abs() < 1e15 {
        return format!("{}", n.round() as i64);
    }
    let s = format!("{n}");
    s
}

fn now_serial(date1904: bool) -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400) as f64 / 86400.0;
    let unix_days = days_from_civil(1970, 1, 1);
    let excel = if date1904 {
        days_from_civil(1904, 1, 1)
    } else {
        days_from_civil(1899, 12, 30)
    };
    (unix_days + days - excel) as f64 + tod
}

fn date_serial(y: Scalar, m: Scalar, d: Scalar, date1904: bool) -> Scalar {
    let y = num_of(y);
    let m = num_of(m);
    let d = num_of(d);
    let (y, m, d) = match (y, m, d) {
        (Scalar::Num(y), Scalar::Num(m), Scalar::Num(d)) => (y, m, d),
        (Scalar::Err(e), _, _) | (_, Scalar::Err(e), _) | (_, _, Scalar::Err(e)) => {
            return Scalar::Err(e)
        }
        _ => return Scalar::Err(VALUE),
    };
    let yi = y.round() as i32;
    let mi = m.round() as i32;
    let di = d.round() as i32;
    if !(1..=9999).contains(&yi) || !(1..=12).contains(&mi) || !(1..=31).contains(&di) {
        return Scalar::Err(NUM);
    }
    let epoch = if date1904 {
        days_from_civil(1904, 1, 1)
    } else {
        days_from_civil(1899, 12, 30)
    };
    Scalar::Num((days_from_civil(yi, mi as u32, di as u32) - epoch) as f64)
}

fn days_from_civil(mut y: i32, m: u32, d: u32) -> i64 {
    y -= i32::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

fn parse_expr(src: &str) -> Option<Expr> {
    let mut lex = Lex {
        s: src,
        i: 0,
        space: false,
    };
    let expr = parse_cmp(&mut lex, false)?;
    if lex.peek().is_some() {
        return None;
    }
    Some(expr)
}

fn parse_cmp(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_concat(lex, union_ok)?;
    loop {
        let op = match lex.peek() {
            Some(Tok::Eq) => BinOp::Eq,
            Some(Tok::Ne) => BinOp::Ne,
            Some(Tok::Lt) => BinOp::Lt,
            Some(Tok::Le) => BinOp::Le,
            Some(Tok::Gt) => BinOp::Gt,
            Some(Tok::Ge) => BinOp::Ge,
            _ => break,
        };
        lex.bump();
        let right = parse_concat(lex, union_ok)?;
        left = Expr::Bin(op, Box::new(left), Box::new(right));
    }
    if union_ok {
        while matches!(lex.peek(), Some(Tok::Comma)) {
            lex.bump();
            let right = parse_cmp(lex, false)?;
            left = Expr::Union(Box::new(left), Box::new(right));
        }
    }
    Some(left)
}

fn parse_concat(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_add(lex, union_ok)?;
    while matches!(lex.peek(), Some(Tok::Amp)) {
        lex.bump();
        let right = parse_add(lex, union_ok)?;
        left = Expr::Bin(BinOp::Cat, Box::new(left), Box::new(right));
    }
    Some(left)
}

fn parse_add(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_mul(lex, union_ok)?;
    loop {
        let op = match lex.peek() {
            Some(Tok::Plus) => BinOp::Add,
            Some(Tok::Minus) => BinOp::Sub,
            _ => break,
        };
        lex.bump();
        let right = parse_mul(lex, union_ok)?;
        left = Expr::Bin(op, Box::new(left), Box::new(right));
    }
    Some(left)
}

fn parse_mul(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_pow(lex, union_ok)?;
    loop {
        let op = match lex.peek() {
            Some(Tok::Star) => BinOp::Mul,
            Some(Tok::Slash) => BinOp::Div,
            _ => break,
        };
        lex.bump();
        let right = parse_pow(lex, union_ok)?;
        left = Expr::Bin(op, Box::new(left), Box::new(right));
    }
    Some(left)
}

fn parse_pow(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_unary(lex, union_ok)?;
    if matches!(lex.peek(), Some(Tok::Caret)) {
        lex.bump();
        let right = parse_pow(lex, union_ok)?;
        left = Expr::Bin(BinOp::Pow, Box::new(left), Box::new(right));
    }
    Some(left)
}

fn parse_unary(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    if matches!(lex.peek(), Some(Tok::Minus)) {
        lex.bump();
        let inner = parse_unary(lex, union_ok)?;
        return Some(Expr::Neg(Box::new(inner)));
    }
    if matches!(lex.peek(), Some(Tok::Plus)) {
        lex.bump();
        return parse_unary(lex, union_ok);
    }
    let mut expr = parse_cross(lex, union_ok)?;
    while matches!(lex.peek(), Some(Tok::Pct)) {
        lex.bump();
        expr = Expr::Pct(Box::new(expr));
    }
    Some(expr)
}

fn parse_cross(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    let mut left = parse_primary(lex, union_ok)?;
    while lex.has_gap() && starts_primary(lex) {
        let right = parse_primary(lex, union_ok)?;
        left = Expr::Cross(Box::new(left), Box::new(right));
    }
    Some(left)
}

fn starts_primary(lex: &Lex<'_>) -> bool {
    matches!(
        lex.peek(),
        Some(Tok::Num(_) | Tok::Str(_) | Tok::Ident(_) | Tok::Err(_) | Tok::LParen | Tok::LBrace)
    )
}

fn parse_primary(lex: &mut Lex<'_>, union_ok: bool) -> Option<Expr> {
    match lex.peek() {
        Some(Tok::Num(n)) => {
            lex.bump();
            Some(Expr::Value(Value::Scalar(Scalar::Num(n))))
        }
        Some(Tok::Str(s)) => {
            lex.bump();
            Some(Expr::Value(Value::Scalar(Scalar::Text(s))))
        }
        Some(Tok::Err(e)) => {
            lex.bump();
            Some(Expr::Value(Value::Scalar(Scalar::Err(e))))
        }
        Some(Tok::LParen) => {
            lex.bump();
            let inner = parse_cmp(lex, true)?;
            if !matches!(lex.peek(), Some(Tok::RParen)) {
                return None;
            }
            lex.bump();
            let _ = union_ok;
            Some(inner)
        }
        Some(Tok::LBrace) => parse_array(lex),
        Some(Tok::LBrack) => parse_external_ref(lex),
        Some(Tok::Ident(id)) => parse_ident(lex, id),
        _ => None,
    }
}

fn parse_external_ref(lex: &mut Lex<'_>) -> Option<Expr> {
    lex.bump();
    let Tok::Num(n) = lex.peek()? else {
        return None;
    };
    let index = n as usize;
    if index == 0 || (n - index as f64).abs() > 0.0 {
        return None;
    }
    lex.bump();
    if !matches!(lex.peek(), Some(Tok::RBrack)) {
        return None;
    }
    lex.bump();
    let Tok::Ident(sheet) = lex.peek()? else {
        return None;
    };
    let sheet = sheet.clone();
    lex.bump();
    if !matches!(lex.peek(), Some(Tok::Bang)) {
        return None;
    }
    lex.bump();
    let (r1, r2) = parse_cell_span(lex, None)?;
    Some(Expr::External {
        index,
        sheet,
        r1,
        r2,
    })
}

fn split_external_label(id: &str) -> Option<(usize, String)> {
    let rest = id.strip_prefix('[')?;
    let end = rest.find(']')?;
    let index: usize = rest[..end].parse().ok()?;
    if index == 0 {
        return None;
    }
    let sheet = rest[end + 1..].trim();
    if sheet.is_empty() {
        return None;
    }
    Some((index, sheet.to_string()))
}

fn parse_ident(lex: &mut Lex<'_>, id: String) -> Option<Expr> {
    lex.bump();
    if let Some((index, sheet)) = split_external_label(&id) {
        if !matches!(lex.peek(), Some(Tok::Bang)) {
            return None;
        }
        lex.bump();
        let (r1, r2) = parse_cell_span(lex, None)?;
        return Some(Expr::External {
            index,
            sheet,
            r1,
            r2,
        });
    }
    if matches!(lex.peek(), Some(Tok::LParen)) {
        lex.bump();
        let mut args = Vec::new();
        if !matches!(lex.peek(), Some(Tok::RParen)) {
            loop {
                args.push(parse_cmp(lex, false)?);
                if matches!(lex.peek(), Some(Tok::Comma)) {
                    lex.bump();
                    continue;
                }
                break;
            }
        }
        if !matches!(lex.peek(), Some(Tok::RParen)) {
            return None;
        }
        lex.bump();
        return Some(Expr::Call(id, args));
    }
    if matches!(lex.peek(), Some(Tok::LBrack)) {
        lex.bump();
        let Tok::Ident(col) = lex.peek()? else {
            return None;
        };
        lex.bump();
        if !matches!(lex.peek(), Some(Tok::RBrack)) {
            return None;
        }
        lex.bump();
        return Some(Expr::TableCol { table: id, col });
    }
    if matches!(lex.peek(), Some(Tok::Bang)) {
        lex.bump();
        let rect = parse_cell_span(lex, None)?;
        return Some(Expr::Value(Value::Area(named_rect(&id, &id, rect))));
    }
    if matches!(lex.peek(), Some(Tok::Colon)) {
        let saved = lex.clone();
        lex.bump();
        if let Some(Tok::Ident(second)) = lex.peek() {
            let second = second.clone();
            let after = {
                let mut look = lex.clone();
                look.bump();
                look.peek()
            };
            if matches!(after, Some(Tok::Bang)) && !is_cell(&id) {
                lex.bump();
                lex.bump();
                let rect = parse_cell_span(lex, None)?;
                return Some(Expr::Value(Value::Area(named_rect(&id, &second, rect))));
            }
        }
        *lex = saved;
    }
    if is_cell(&id) {
        let cell = a1::parse_cell(&id).ok()?;
        if matches!(lex.peek(), Some(Tok::Colon)) {
            let saved = lex.clone();
            lex.bump();
            if let Some(Tok::Ident(other)) = lex.peek() {
                if is_cell(&other) {
                    let other = other.clone();
                    lex.bump();
                    let end = a1::parse_cell(&other).ok()?;
                    return Some(Expr::Value(Value::Area(here_rect(cell, end))));
                }
            }
            *lex = saved;
        }
        return Some(Expr::Value(Value::Area(here_rect(cell, cell))));
    }
    if id.eq_ignore_ascii_case("TRUE") {
        return Some(Expr::Value(Value::Scalar(Scalar::Bool(true))));
    }
    if id.eq_ignore_ascii_case("FALSE") {
        return Some(Expr::Value(Value::Scalar(Scalar::Bool(false))));
    }
    Some(Expr::Name(id))
}

fn parse_cell_span(lex: &mut Lex<'_>, _sheet: Option<&str>) -> Option<(CellRef, CellRef)> {
    let Tok::Ident(a) = lex.peek()? else {
        return None;
    };
    if !is_cell(&a) {
        return None;
    }
    let a = a.clone();
    lex.bump();
    let start = a1::parse_cell(&a).ok()?;
    if matches!(lex.peek(), Some(Tok::Colon)) {
        lex.bump();
        let Tok::Ident(b) = lex.peek()? else {
            return None;
        };
        if !is_cell(&b) {
            return None;
        }
        let b = b.clone();
        lex.bump();
        let end = a1::parse_cell(&b).ok()?;
        return Some((start, end));
    }
    Some((start, start))
}

fn parse_array(lex: &mut Lex<'_>) -> Option<Expr> {
    lex.bump();
    let mut rows = Vec::new();
    loop {
        let mut cols = Vec::new();
        loop {
            cols.push(parse_cmp(lex, false)?);
            if matches!(lex.peek(), Some(Tok::Comma)) {
                lex.bump();
                continue;
            }
            break;
        }
        rows.push(cols);
        if matches!(lex.peek(), Some(Tok::Semi)) {
            lex.bump();
            continue;
        }
        break;
    }
    if !matches!(lex.peek(), Some(Tok::RBrace)) {
        return None;
    }
    lex.bump();
    Some(Expr::Value(Value::Grid(eval_const_grid(rows)?)))
}

fn eval_const_grid(rows: Vec<Vec<Expr>>) -> Option<Vec<Vec<Scalar>>> {
    let mut out = Vec::new();
    for row in rows {
        let mut cols = Vec::new();
        for cell in row {
            cols.push(match cell {
                Expr::Value(Value::Scalar(s)) => s,
                _ => return None,
            });
        }
        out.push(cols);
    }
    Some(out)
}

fn is_cell(text: &str) -> bool {
    a1::parse_cell(text).is_ok()
}

fn here_rect(r1: CellRef, r2: CellRef) -> Rect {
    Rect {
        from: Loc::Here,
        to: Loc::Here,
        r1,
        r2,
    }
}

fn named_rect(a: &str, b: &str, span: (CellRef, CellRef)) -> Rect {
    Rect {
        from: Loc::Name(a.to_string()),
        to: Loc::Name(b.to_string()),
        r1: span.0,
        r2: span.1,
    }
}

fn logic_const(args: &[Expr], value: bool) -> Calc {
    if !args.is_empty() {
        return Calc::Skip;
    }
    Calc::Ready(Value::Scalar(Scalar::Bool(value)))
}

fn ready_value(calc: Calc) -> Option<Value> {
    match calc {
        Calc::Skip => None,
        Calc::Ready(v) => Some(v),
    }
}

/// 公式里直接写下的 `TRUE` / `FALSE` 文本。前后有空格不算。大小写不敏感。
fn bool_word(text: &str) -> Option<bool> {
    if text.eq_ignore_ascii_case("TRUE") {
        Some(true)
    } else if text.eq_ignore_ascii_case("FALSE") {
        Some(false)
    } else {
        None
    }
}

fn if_flag(value: &Scalar) -> Result<bool, &'static str> {
    match value {
        Scalar::Bool(b) => Ok(*b),
        Scalar::Num(n) => Ok(*n != 0.0),
        Scalar::Err(e) => Err(*e),
        Scalar::Text(t) => bool_word(t).ok_or(VALUE),
    }
}

fn grid_multi(grid: &[Vec<Scalar>]) -> bool {
    grid.len() > 1 || grid.first().is_some_and(|row| row.len() > 1)
}

enum LogicRef {
    Skip,
    Ignore,
    Value(Scalar),
}

struct LogicFold {
    saw: bool,
    all_true: bool,
    any_true: bool,
    err: Option<&'static str>,
}

impl LogicFold {
    fn new() -> Self {
        Self {
            saw: false,
            all_true: true,
            any_true: false,
            err: None,
        }
    }

    fn note_err(&mut self, err: &'static str) {
        if self.err.is_none() {
            self.err = Some(err);
        }
    }

    fn note_bool(&mut self, value: bool) {
        self.saw = true;
        self.all_true &= value;
        self.any_true |= value;
    }

    /// 直接写在参数位置上的值。`TRUE` / `FALSE` 文本算逻辑值，其它文本先放下。
    fn push_direct(&mut self, value: &Scalar) {
        match value {
            Scalar::Err(e) => self.note_err(e),
            Scalar::Bool(b) => self.note_bool(*b),
            Scalar::Num(n) => self.note_bool(*n != 0.0),
            Scalar::Text(t) => {
                if let Some(b) = bool_word(t) {
                    self.note_bool(b);
                }
            }
        }
    }

    /// 引用和数组常量里的文本一律不参加，包括写成 `TRUE` 的文本。
    fn push_ref(&mut self, value: &Scalar) {
        match value {
            Scalar::Err(e) => self.note_err(e),
            Scalar::Bool(b) => self.note_bool(*b),
            Scalar::Num(n) => self.note_bool(*n != 0.0),
            Scalar::Text(_) => {}
        }
    }

    fn finish(&self, and: bool) -> Scalar {
        if let Some(err) = self.err {
            return Scalar::Err(err);
        }
        if !self.saw {
            return Scalar::Err(VALUE);
        }
        Scalar::Bool(if and { self.all_true } else { self.any_true })
    }
}

enum Branch {
    Scalar(Scalar),
    Grid(Vec<Vec<Scalar>>),
}

fn pick_branch(branch: &Branch, row: usize, col: usize) -> Scalar {
    match branch {
        Branch::Scalar(s) => s.clone(),
        Branch::Grid(g) => g
            .get(row)
            .and_then(|line| line.get(col))
            .cloned()
            .unwrap_or(Scalar::Err(NA)),
    }
}

fn write_cache(el: &mut Element, val: &Scalar) {
    el.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "v" | "is"),
        Node::Text(_) | Node::Raw(_) => true,
    });
    match val {
        Scalar::Num(n) => {
            el.remove_attr("t");
            let mut v = Element::new("v");
            v.set_text(fmt_num(*n));
            el.push_element(v);
        }
        Scalar::Bool(b) => {
            el.set_attr("t", "b");
            let mut v = Element::new("v");
            v.set_text(if *b { "1" } else { "0" });
            el.push_element(v);
        }
        Scalar::Text(s) => {
            if !xml_text_ok(s) {
                return;
            }
            el.set_attr("t", "str");
            let mut v = Element::new("v");
            v.set_text(s);
            el.push_element(v);
        }
        Scalar::Err(e) => {
            el.set_attr("t", "e");
            let mut v = Element::new("v");
            v.set_text(*e);
            el.push_element(v);
        }
    }
}

impl CellRec {
    fn clone_rec(&self) -> Self {
        self.clone()
    }
}

impl Clone for CellRec {
    fn clone(&self) -> Self {
        Self {
            formula: self.formula.clone(),
            cached: self.cached.clone(),
            array_master: self.array_master,
            array_ref: self.array_ref,
        }
    }
}

impl Lex<'_> {
    fn peek(&self) -> Option<Tok> {
        self.scan().map(|(tok, _, _)| tok)
    }

    fn has_gap(&self) -> bool {
        self.scan().is_some_and(|(_, _, space)| space)
    }

    fn bump(&mut self) -> Option<Tok> {
        let (tok, end, space) = self.scan()?;
        self.space = space;
        self.i = end;
        Some(tok)
    }

    fn scan(&self) -> Option<(Tok, usize, bool)> {
        let b = self.s.as_bytes();
        let mut i = self.i;
        let mut space = false;
        while i < b.len() && b[i].is_ascii_whitespace() {
            space = true;
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        let rest = &self.s[i..];
        for lit in [
            "#DIV/0!",
            "#GETTING_DATA",
            "#VALUE!",
            "#NULL!",
            "#NAME?",
            "#NUM!",
            "#REF!",
            "#N/A",
        ] {
            if rest.starts_with(lit) {
                let err = match lit {
                    "#DIV/0!" => DIV0,
                    "#NULL!" => NULL,
                    "#NAME?" => NAME,
                    "#NUM!" => NUM,
                    "#REF!" => REF,
                    "#N/A" => NA,
                    _ => VALUE,
                };
                return Some((Tok::Err(err), i + lit.len(), space));
            }
        }
        let two = rest.as_bytes();
        if two.len() >= 2 && rest.is_char_boundary(2) {
            let pair = &rest[..2];
            let tok = match pair {
                "<>" => Some(Tok::Ne),
                "<=" => Some(Tok::Le),
                ">=" => Some(Tok::Ge),
                _ => None,
            };
            if let Some(tok) = tok {
                return Some((tok, i + 2, space));
            }
        }
        let ch = rest.chars().next().unwrap();
        let simple = match ch {
            '+' => Some(Tok::Plus),
            '-' => Some(Tok::Minus),
            '*' => Some(Tok::Star),
            '/' => Some(Tok::Slash),
            '^' => Some(Tok::Caret),
            '&' => Some(Tok::Amp),
            '=' => Some(Tok::Eq),
            '<' => Some(Tok::Lt),
            '>' => Some(Tok::Gt),
            '%' => Some(Tok::Pct),
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            ',' => Some(Tok::Comma),
            ':' => Some(Tok::Colon),
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            '[' => Some(Tok::LBrack),
            ']' => Some(Tok::RBrack),
            '!' => Some(Tok::Bang),
            ';' => Some(Tok::Semi),
            _ => None,
        };
        if let Some(tok) = simple {
            return Some((tok, i + ch.len_utf8(), space));
        }
        if ch == '"' {
            let mut j = i + 1;
            let bytes = self.s.as_bytes();
            let mut text = String::new();
            while j < bytes.len() {
                if bytes[j] == b'"' {
                    if j + 1 < bytes.len() && bytes[j + 1] == b'"' {
                        text.push('"');
                        j += 2;
                        continue;
                    }
                    return Some((Tok::Str(text), j + 1, space));
                }
                let c = self.s[j..].chars().next().unwrap();
                text.push(c);
                j += c.len_utf8();
            }
            return None;
        }
        if ch == '\'' {
            let mut j = i + 1;
            let bytes = self.s.as_bytes();
            let mut text = String::new();
            while j < bytes.len() {
                if bytes[j] == b'\'' {
                    if j + 1 < bytes.len() && bytes[j + 1] == b'\'' {
                        text.push('\'');
                        j += 2;
                        continue;
                    }
                    return Some((Tok::Ident(text), j + 1, space));
                }
                let c = self.s[j..].chars().next().unwrap();
                text.push(c);
                j += c.len_utf8();
            }
            return None;
        }
        if ch.is_ascii_digit()
            || (ch == '.' && rest.len() > 1 && rest.as_bytes()[1].is_ascii_digit())
        {
            let mut j = i;
            let bytes = self.s.as_bytes();
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            let n: f64 = self.s[i..j].parse().ok()?;
            return Some((Tok::Num(n), j, space));
        }
        let mut j = i;
        let bytes = self.s.as_bytes();
        while j < bytes.len() {
            let c = self.s[j..].chars().next().unwrap();
            if c.is_ascii_alphanumeric()
                || c == '_'
                || c == '.'
                || c == '$'
                || c == '\\'
                || !c.is_ascii()
            {
                j += c.len_utf8();
            } else {
                break;
            }
        }
        if j == i {
            return None;
        }
        Some((Tok::Ident(self.s[i..j].to_string()), j, space))
    }
}

impl Clone for Lex<'_> {
    fn clone(&self) -> Self {
        Self {
            s: self.s,
            i: self.i,
            space: self.space,
        }
    }
}

fn outside_corner(start: CellRef) -> Option<CellRef> {
    if start.col <= 1 || start.row <= 1 {
        None
    } else {
        Some(CellRef {
            col: start.col - 1,
            row: start.row - 1,
        })
    }
}

fn load_data_tables(docs: &[(String, Document)]) -> Vec<DataTableRec> {
    let mut out = Vec::new();
    for (si, (_, doc)) in docs.iter().enumerate() {
        let Some(data) = doc.root.child_by_local("sheetData") else {
            continue;
        };
        formula::for_each_cell(data, |here, el| {
            let Some(f) = el.child_by_local("f") else {
                return;
            };
            if f.attr("t") != Some("dataTable") {
                return;
            }
            let Some((start, end)) = f.attr("ref").and_then(|r| a1::parse_range(r).ok()) else {
                return;
            };
            if here != start {
                return;
            }
            let Some(r1) = f.attr("r1").and_then(|s| a1::parse_cell(s).ok()) else {
                return;
            };
            out.push(DataTableRec {
                sheet: si,
                start,
                end,
                dt2d: formula::xml_true(f.attr("dt2D")),
                dtr: formula::xml_true(f.attr("dtr")),
                r1,
                r2: f.attr("r2").and_then(|s| a1::parse_cell(s).ok()),
            });
        });
    }
    out
}

fn load_externals(
    pkg: &Package,
    wb_part: &str,
    base_dir: Option<&Path>,
) -> Result<(Vec<Option<ExtBook>>, Vec<String>), Error> {
    let links = super::extlink::list_books(pkg, wb_part)?;
    let mut books = vec![None];
    let mut parts = vec![String::new()];
    for link in links {
        parts.push(link.part.clone());
        books.push(load_one_external(&link, base_dir));
    }
    Ok((books, parts))
}

fn load_one_external(
    link: &super::extlink::ExtLinkInfo,
    base_dir: Option<&Path>,
) -> Option<ExtBook> {
    let path = resolve_ext_path(&link.path, base_dir)?;
    let bytes = read_ext_file(&path)?;
    let mut pkg = Package::open_bytes(bytes).ok()?;
    let _ = fill_formula_caches_depth(&mut pkg, path.parent(), 0);
    let sheets = sheets_of(&pkg).ok()?;
    let mut sheet_ids = HashMap::new();
    for (i, name) in link.sheets.iter().enumerate() {
        sheet_ids.insert(name.to_ascii_lowercase(), i as u32);
    }
    Some(ExtBook { sheets, sheet_ids })
}

fn sheets_of(pkg: &Package) -> Result<Vec<Sheet>, Error> {
    let wb_part = opc::office_document_part(pkg)?;
    let wb_rels = Relationships::from_package(pkg, &wb_part)?;
    let wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    let sst = inspect::load_shared_strings(pkg, &wb_part, &wb_rels)?;
    let listed = sheet_list(&wb_doc.root, &wb_rels, &wb_part)?;
    let mut sheets = Vec::new();
    for (name, part, kind) in &listed {
        if *kind != "worksheet" {
            continue;
        }
        let doc = xml::parse(pkg.get(part)?)?;
        sheets.push(Sheet {
            name: name.clone(),
            cells: load_sheet(&doc.root, &sst),
            origin: sheet_origin(&doc.root),
            hidden_rows: hidden_rows(&doc.root),
        });
    }
    Ok(sheets)
}

fn resolve_ext_path(target: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let t = target.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("http:") || lower.starts_with("https:") {
        return None;
    }
    let decoded = if lower.starts_with("file:///") {
        percent_decode(&t["file:///".len()..])
    } else {
        percent_decode(t)
    };
    let path = PathBuf::from(decoded);
    if path.is_absolute() {
        Some(path)
    } else {
        base_dir.map(|dir| dir.join(path))
    }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_ext_file(path: &Path) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
}

fn write_external_caches(
    pkg: &mut Package,
    parts: &[String],
    used: &HashMap<(usize, u32, u32, u32), Scalar>,
) -> Result<bool, Error> {
    let mut changed = false;
    let indexes: HashSet<usize> = used.keys().map(|(index, _, _, _)| *index).collect();
    for index in indexes {
        let Some(part) = parts.get(index) else {
            continue;
        };
        if part.is_empty() || !pkg.contains(part) {
            continue;
        }
        let mut doc = xml::parse(pkg.get(part)?)?;
        let Some(book) = doc.root.child_by_local_mut("externalBook") else {
            continue;
        };
        let Some(set) = book.child_by_local_mut("sheetDataSet") else {
            continue;
        };
        let mut touched = false;
        for data in set.children.iter_mut() {
            let Node::Element(sheet) = data else {
                continue;
            };
            if sheet.local_name() != "sheetData" {
                continue;
            }
            let Some(id) = sheet.attr("sheetId").and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            let mut cells: Vec<((u32, u32), &Scalar)> = used
                .iter()
                .filter(|(key, _)| key.0 == index && key.1 == id)
                .map(|(key, val)| ((key.2, key.3), val))
                .collect();
            cells.sort_by_key(|(addr, _)| (addr.1, addr.0));
            if cells.is_empty() {
                continue;
            }
            if write_external_sheet(sheet, &cells) {
                touched = true;
            }
        }
        if touched {
            pkg.set(part, doc.to_bytes()?)?;
            changed = true;
        }
    }
    Ok(changed)
}

fn write_external_sheet(sheet: &mut Element, cells: &[((u32, u32), &Scalar)]) -> bool {
    let mut changed = false;
    for ((col, row), val) in cells {
        let addr = a1::format_cell(*col, *row);
        let row_s = row.to_string();
        let has_row = sheet.children.iter().any(|child| match child {
            Node::Element(el) => el.local_name() == "row" && el.attr("r") == Some(row_s.as_str()),
            _ => false,
        });
        if !has_row {
            let mut el = Element::new("row");
            el.set_attr("r", &row_s);
            sheet.push_element(el);
            changed = true;
        }
        let row_el = sheet
            .children
            .iter_mut()
            .find_map(|child| match child {
                Node::Element(el)
                    if el.local_name() == "row" && el.attr("r") == Some(row_s.as_str()) =>
                {
                    Some(el)
                }
                _ => None,
            })
            .expect("刚确保的 row");
        let cell = if let Some(found) = row_el.children.iter_mut().find_map(|child| match child {
            Node::Element(el)
                if el.local_name() == "cell" && el.attr("r") == Some(addr.as_str()) =>
            {
                Some(el)
            }
            _ => None,
        }) {
            found
        } else {
            let mut el = Element::new("cell");
            el.set_attr("r", addr);
            row_el.push_element(el);
            changed = true;
            row_el
                .children
                .iter_mut()
                .rev()
                .find_map(|child| match child {
                    Node::Element(el) => Some(el),
                    _ => None,
                })
                .expect("刚插入的 cell")
        };
        changed |= write_external_cell(cell, val);
    }
    changed
}

fn write_external_cell(cell: &mut Element, val: &Scalar) -> bool {
    let (ty, text) = match val {
        Scalar::Num(n) => (None, fmt_num(*n)),
        Scalar::Bool(b) => (
            Some("b"),
            if *b { "1".to_string() } else { "0".to_string() },
        ),
        Scalar::Text(s) => (Some("str"), s.clone()),
        Scalar::Err(e) => (Some("e"), (*e).to_string()),
    };
    let same_ty = cell.attr("t") == ty;
    let same_v = cell.child_by_local("v").map(|v| v.text_content()) == Some(text.clone());
    if same_ty && same_v {
        return false;
    }
    match ty {
        Some(t) => cell.set_attr("t", t),
        None => cell.remove_attr("t"),
    }
    cell.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "v",
        Node::Text(_) | Node::Raw(_) => true,
    });
    let mut v = Element::new("v");
    v.set_text(text);
    cell.push_element(v);
    true
}

fn refresh_chart_caches(
    pkg: &mut Package,
    sheet_docs: &[(String, Document)],
    names: &[String],
    sst: &[String],
) -> Result<bool, Error> {
    let mut changed = false;
    for part in pkg.part_names() {
        if !part.starts_with("/xl/charts/chart") || !part.ends_with(".xml") {
            continue;
        }
        let mut doc = xml::parse(pkg.get(&part)?)?;
        if refresh_chart_element(&mut doc.root, sheet_docs, names, sst) {
            pkg.set(&part, doc.to_bytes()?)?;
            changed = true;
        }
    }
    Ok(changed)
}

fn refresh_chart_element(
    el: &mut Element,
    sheet_docs: &[(String, Document)],
    names: &[String],
    sst: &[String],
) -> bool {
    let mut changed = false;
    if let Some(formula) = el.child_by_local("f").map(|f| f.text_content()) {
        if let Some((si, start, end)) = parse_chart_ref(&formula, names) {
            if let Some(values) = chart_values(&sheet_docs[si].1.root, start, end, sst) {
                let cache = if el.child_by_local("numCache").is_some() {
                    el.child_by_local_mut("numCache")
                } else {
                    el.child_by_local_mut("strCache")
                };
                if let Some(cache) = cache {
                    changed |= replace_cache_pts(cache, &values);
                }
            }
        }
    }
    for child in &mut el.children {
        if let Node::Element(inner) = child {
            changed |= refresh_chart_element(inner, sheet_docs, names, sst);
        }
    }
    changed
}

fn parse_chart_ref(formula: &str, names: &[String]) -> Option<(usize, CellRef, CellRef)> {
    let text = formula.trim().trim_start_matches('=');
    let (sheet, range) = text.rsplit_once('!')?;
    if sheet.contains(':') || sheet.contains('[') {
        return None;
    }
    let sheet = sheet.trim().trim_matches('\'').replace("''", "'");
    let si = names
        .iter()
        .position(|name| name.eq_ignore_ascii_case(&sheet))?;
    let range = range.replace('$', "");
    let (start, end) = a1::parse_range(&range).ok()?;
    Some((si, start, end))
}

fn chart_values(
    root: &Element,
    start: CellRef,
    end: CellRef,
    sst: &[String],
) -> Option<Vec<Scalar>> {
    let (r1, r2) = ordered(start, end);
    let data = root.child_by_local("sheetData");
    let mut out = Vec::new();
    let mut any = false;
    for row in r1.row..=r2.row {
        for col in r1.col..=r2.col {
            let cell = CellRef { col, row };
            if let Some(scalar) = data
                .and_then(|sheet| super::cell::find_cell(sheet, cell))
                .and_then(|el| read_cached(el, sst))
            {
                any = true;
                out.push(scalar);
            } else {
                out.push(Scalar::Num(0.0));
            }
        }
    }
    if any {
        Some(out)
    } else {
        None
    }
}

fn cache_text(val: &Scalar) -> String {
    match val {
        Scalar::Num(n) => fmt_num(*n),
        Scalar::Bool(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        Scalar::Text(s) => s.clone(),
        Scalar::Err(e) => (*e).to_string(),
    }
}

fn replace_cache_pts(cache: &mut Element, values: &[Scalar]) -> bool {
    let existing: Vec<String> = cache
        .children_by_local("pt")
        .into_iter()
        .map(|pt| {
            pt.child_by_local("v")
                .map(|v| v.text_content())
                .unwrap_or_default()
        })
        .collect();
    let next_text: Vec<String> = values.iter().map(cache_text).collect();
    if existing == next_text {
        return false;
    }
    cache.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "pt" && e.local_name() != "ptCount",
        Node::Text(_) | Node::Raw(_) => true,
    });
    let mut count = Element::new("c:ptCount");
    count.set_attr("val", values.len().to_string());
    cache.push_element(count);
    for (i, text) in next_text.into_iter().enumerate() {
        let mut v = Element::new("c:v");
        v.set_text(text);
        let mut pt = Element::new("c:pt");
        pt.set_attr("idx", i.to_string());
        pt.push_element(v);
        cache.push_element(pt);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::parse_expr;

    #[test]
    fn sheet_ref_parses() {
        assert!(parse_expr("来源!A1").is_some());
        assert!(parse_expr("SUM(数据:来源!A1)").is_some());
        assert!(parse_expr("A1:B2 B1:C1").is_some());
    }
}
