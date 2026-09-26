//! A1 地址与区域。

use crate::error::{Error, ErrorKind, RangeReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellRef {
    pub col: u32,
    pub row: u32,
}

pub fn parse_cell(input: &str) -> Result<CellRef, Error> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::new(ErrorKind::InvalidRange {
            reason: RangeReason::Empty,
        }));
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    if bytes[i] == b'$' {
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
    if i < bytes.len() && bytes[i] == b'$' {
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
    Ok(CellRef { col, row })
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
    }

    #[test]
    fn range_order() {
        assert!(parse_range("D20:A1").is_err());
        let (a, b) = parse_range("A1:D20").unwrap();
        assert_eq!(a, CellRef { col: 1, row: 1 });
        assert_eq!(b, CellRef { col: 4, row: 20 });
    }
}
