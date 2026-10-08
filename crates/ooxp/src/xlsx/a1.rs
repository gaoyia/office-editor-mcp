//! A1 地址与区域。

use crate::error::{Error, ErrorKind, RangeReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellRef {
    pub col: u32,
    pub row: u32,
}

/// 带 `$` 的 A1 引用，供共享公式相对展开使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsCellRef {
    pub col: u32,
    pub row: u32,
    pub col_abs: bool,
    pub row_abs: bool,
}

impl AbsCellRef {
    pub fn cell(self) -> CellRef {
        CellRef {
            col: self.col,
            row: self.row,
        }
    }
}

pub fn parse_cell(input: &str) -> Result<CellRef, Error> {
    Ok(parse_cell_abs(input)?.cell())
}

pub fn parse_cell_abs(input: &str) -> Result<AbsCellRef, Error> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Empty,
        }));
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    let col_abs = bytes[i] == b'$';
    if col_abs {
        i += 1;
    }
    let col_start = i;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == col_start {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::NotA1 {
                input: input.to_string(),
            },
        }));
    }
    let col = letters_to_col(&s[col_start..i])?;
    let row_abs = i < bytes.len() && bytes[i] == b'$';
    if row_abs {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::NotA1 {
                input: input.to_string(),
            },
        }));
    }
    let row: u32 = s[i..].parse().map_err(|_| {
        Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::NotA1 {
                input: input.to_string(),
            },
        })
    })?;
    if row == 0 || row > 1_048_576 {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::RowOutOfRange { row },
        }));
    }
    if col == 0 || col > 16_384 {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::ColOutOfRange { col },
        }));
    }
    Ok(AbsCellRef {
        col,
        row,
        col_abs,
        row_abs,
    })
}

/// 从 `input[from..]` 解析一个 A1（可带 `$`）。成功返回引用与消费后的字节下标。
pub fn parse_abs_prefix(input: &str, from: usize) -> Option<(AbsCellRef, usize)> {
    if from >= input.len() {
        return None;
    }
    let s = &input[from..];
    let bytes = s.as_bytes();
    let mut i = 0;
    let col_abs = bytes[i] == b'$';
    if col_abs {
        i += 1;
        if i >= bytes.len() {
            return None;
        }
    }
    let col_start = i;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == col_start {
        return None;
    }
    let col = letters_to_col(&s[col_start..i]).ok()?;
    let row_abs = i < bytes.len() && bytes[i] == b'$';
    if row_abs {
        i += 1;
    }
    let row_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == row_start {
        return None;
    }
    let row: u32 = s[row_start..i].parse().ok()?;
    if row == 0 || row > 1_048_576 || col == 0 || col > 16_384 {
        return None;
    }
    Some((
        AbsCellRef {
            col,
            row,
            col_abs,
            row_abs,
        },
        from + i,
    ))
}

pub fn parse_range(input: &str) -> Result<(CellRef, CellRef), Error> {
    let s = input.trim();
    if let Some((a, b)) = s.split_once(':') {
        let start = parse_cell(a)?;
        let end = parse_cell(b)?;
        if start.col > end.col || start.row > end.row {
            return Err(Error::new(ErrorKind::InvalidRange {
                reason: RangeReason::Reversed {
                    input: input.to_string(),
                },
            }));
        }
        Ok((start, end))
    } else {
        let c = parse_cell(s)?;
        Ok((c, c))
    }
}

pub fn letters_to_col(letters: &str) -> Result<u32, Error> {
    let mut n: u32 = 0;
    for ch in letters.chars() {
        if !ch.is_ascii_alphabetic() {
            return Err(Error::new(ErrorKind::InvalidRange {
                reason: RangeReason::InvalidColLetters {
                    letters: letters.to_string(),
                },
            }));
        }
        let d = (ch.to_ascii_uppercase() as u8 - b'A' + 1) as u32;
        n = n.saturating_mul(26).saturating_add(d);
    }
    Ok(n)
}

pub fn col_to_letters(mut col: u32) -> String {
    let mut s = String::new();
    while col > 0 {
        col -= 1;
        s.insert(0, (b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    s
}

pub fn format_cell(col: u32, row: u32) -> String {
    format!("{}{row}", col_to_letters(col))
}

pub fn format_abs_cell(c: AbsCellRef) -> String {
    format!(
        "{}{}{}{}",
        if c.col_abs { "$" } else { "" },
        col_to_letters(c.col),
        if c.row_abs { "$" } else { "" },
        c.row
    )
}

pub fn format_range(start: CellRef, end: CellRef) -> String {
    if start == end {
        format_cell(start.col, start.row)
    } else {
        format!(
            "{}:{}",
            format_cell(start.col, start.row),
            format_cell(end.col, end.row)
        )
    }
}

/// 闭区间是否相交。两端须已规范化（左上到右下）。
pub fn ranges_overlap(a0: CellRef, a1: CellRef, b0: CellRef, b1: CellRef) -> bool {
    a0.col <= b1.col && b0.col <= a1.col && a0.row <= b1.row && b0.row <= a1.row
}

/// 闭区间包含：`inner` 的四角都落在 `outer` 内。两端须已规范化。
pub fn range_contains(outer: (CellRef, CellRef), inner: (CellRef, CellRef)) -> bool {
    inner.0.col >= outer.0.col
        && inner.0.row >= outer.0.row
        && inner.1.col <= outer.1.col
        && inner.1.row <= outer.1.row
}

/// 相对引用按行列差平移；绝对引用不动。出界返回 `None`。
pub fn shift_abs(c: AbsCellRef, dcol: i64, drow: i64) -> Option<AbsCellRef> {
    let col = if c.col_abs {
        c.col
    } else {
        let n = c.col as i64 + dcol;
        if !(1..=16_384).contains(&n) {
            return None;
        }
        n as u32
    };
    let row = if c.row_abs {
        c.row
    } else {
        let n = c.row as i64 + drow;
        if !(1..=1_048_576).contains(&n) {
            return None;
        }
        n as u32
    };
    Some(AbsCellRef {
        col,
        row,
        col_abs: c.col_abs,
        row_abs: c.row_abs,
    })
}

/// 列字母或列区间，例如 `A`、`A:C`。不要带行号。
pub fn parse_col_span(input: &str) -> Result<(u32, u32), Error> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Empty,
        }));
    }
    let (left, right) = match s.split_once(':') {
        Some((a, b)) => (a, b),
        None => (s, s),
    };
    let start = parse_col_letters(left)?;
    let end = parse_col_letters(right)?;
    if start > end {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Reversed {
                input: input.to_string(),
            },
        }));
    }
    Ok((start, end))
}

pub fn parse_row_span(input: &str) -> Result<(u32, u32), Error> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Empty,
        }));
    }
    let (left, right) = match s.split_once(':') {
        Some((a, b)) => (a, b),
        None => (s, s),
    };
    let start = parse_row_number(left)?;
    let end = parse_row_number(right)?;
    if start > end {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Reversed {
                input: input.to_string(),
            },
        }));
    }
    Ok((start, end))
}

fn parse_row_number(input: &str) -> Result<u32, Error> {
    let s = input.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::NotA1 {
                input: input.to_string(),
            },
        }));
    }
    let row: u32 = s.parse().map_err(|_| {
        Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::NotA1 {
                input: input.to_string(),
            },
        })
    })?;
    if !(1..=1_048_576).contains(&row) {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::RowOutOfRange { row },
        }));
    }
    Ok(row)
}

fn parse_col_letters(input: &str) -> Result<u32, Error> {
    let s = input.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::InvalidColLetters {
                letters: input.to_string(),
            },
        }));
    }
    let col = letters_to_col(s)?;
    if col == 0 || col > 16_384 {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::ColOutOfRange { col },
        }));
    }
    Ok(col)
}

pub fn normalize_a1(input: &str) -> Result<String, Error> {
    let c = parse_cell(input)?;
    Ok(format_cell(c.col, c.row))
}

pub fn in_range(cell: CellRef, start: CellRef, end: CellRef) -> bool {
    cell.col >= start.col && cell.col <= end.col && cell.row >= start.row && cell.row <= end.row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_roundtrip() {
        assert_eq!(parse_cell("A1").unwrap(), CellRef { col: 1, row: 1 });
        assert_eq!(parse_cell("AA100").unwrap(), CellRef { col: 27, row: 100 });
        assert_eq!(format_cell(27, 100), "AA100");
        assert_eq!(normalize_a1("a1").unwrap(), "A1");
        let abs = parse_cell_abs("$A$1").unwrap();
        assert!(abs.col_abs && abs.row_abs);
        assert_eq!(format_abs_cell(abs), "$A$1");
        assert_eq!(
            format_range(CellRef { col: 1, row: 1 }, CellRef { col: 2, row: 2 }),
            "A1:B2"
        );
        assert!(ranges_overlap(
            CellRef { col: 2, row: 2 },
            CellRef { col: 2, row: 3 },
            CellRef { col: 2, row: 1 },
            CellRef { col: 2, row: 3 },
        ));
        assert!(!ranges_overlap(
            CellRef { col: 2, row: 1 },
            CellRef { col: 2, row: 2 },
            CellRef { col: 4, row: 1 },
            CellRef { col: 4, row: 2 },
        ));
    }

    #[test]
    fn range_order() {
        assert!(parse_range("D20:A1").is_err());
        let (a, b) = parse_range("A1:D20").unwrap();
        assert_eq!(a, CellRef { col: 1, row: 1 });
        assert_eq!(b, CellRef { col: 4, row: 20 });
    }

    #[test]
    fn col_span_letters() {
        assert_eq!(parse_col_span("A").unwrap(), (1, 1));
        assert_eq!(parse_col_span("a:c").unwrap(), (1, 3));
        assert!(parse_col_span("A1").is_err());
        assert!(parse_col_span("C:A").is_err());
    }
}
