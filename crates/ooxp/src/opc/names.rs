//! OPC 部件名规范化。

use crate::error::{Error, ErrorKind, OpcReason};

/// 把 ZIP 条目名或任意路径收成以 `/` 开头的 OPC 名。
pub fn normalize_part_name(name: &str) -> Result<String, Error> {
    let unified = name.replace('\\', "/");
    let trimmed = unified.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(Error::new(ErrorKind::OpcOpen {
            reason: OpcReason::EmptyPartName,
        }));
    }
    for seg in trimmed.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(Error::new(ErrorKind::OpcOpen {
                reason: OpcReason::InvalidPartName {
                    name: name.to_string(),
                },
            }));
        }
    }
    Ok(format!("/{trimmed}"))
}

/// ZIP 里通常不带前导 `/`。
pub fn zip_name_from_opc(opc: &str) -> &str {
    opc.strip_prefix('/').unwrap_or(opc)
}

/// `/xl/workbook.xml` → (`/xl`, `workbook.xml`)；`/[Content_Types].xml` → (`/`, `[Content_Types].xml`)。
pub fn split_dir_file(opc: &str) -> (&str, &str) {
    match opc.rfind('/') {
        Some(0) => ("/", &opc[1..]),
        Some(i) => (&opc[..i], &opc[i + 1..]),
        None => ("/", opc),
    }
}

/// 源部件对应的关系文件路径。包根用 `source = "/"`。
pub fn rels_part_name(source: &str) -> String {
    if source == "/" {
        return "/_rels/.rels".to_string();
    }
    let opc = if source.starts_with('/') {
        source.to_string()
    } else {
        format!("/{source}")
    };
    let (dir, file) = split_dir_file(&opc);
    if dir == "/" {
        format!("/_rels/{file}.rels")
    } else {
        format!("{dir}/_rels/{file}.rels")
    }
}

/// 把关系里的相对 Target 解析成 OPC 绝对路径。
pub fn resolve_rel_target(source_part: &str, target: &str) -> Result<String, Error> {
    let t = target.replace('\\', "/");
    if t.starts_with('/') {
        return normalize_part_name(&t);
    }
    let base_dir = if source_part == "/" {
        "/"
    } else {
        split_dir_file(source_part).0
    };
    let mut segs: Vec<&str> = Vec::new();
    if base_dir != "/" {
        segs.extend(
            base_dir
                .trim_start_matches('/')
                .split('/')
                .filter(|s| !s.is_empty()),
        );
    }
    for seg in t.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            segs.pop();
            continue;
        }
        segs.push(seg);
    }
    if segs.is_empty() {
        return Err(Error::new(ErrorKind::OpcOpen {
            reason: OpcReason::RelTarget {
                target: target.to_string(),
            },
        }));
    }
    normalize_part_name(&segs.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_and_unifies() {
        assert_eq!(
            normalize_part_name("xl/workbook.xml").unwrap(),
            "/xl/workbook.xml"
        );
        assert_eq!(
            normalize_part_name("/xl/workbook.xml").unwrap(),
            "/xl/workbook.xml"
        );
        assert!(normalize_part_name("xl/../workbook.xml").is_err());
    }

    #[test]
    fn rels_paths() {
        assert_eq!(rels_part_name("/"), "/_rels/.rels");
        assert_eq!(
            rels_part_name("/xl/workbook.xml"),
            "/xl/_rels/workbook.xml.rels"
        );
        assert_eq!(
            rels_part_name("/[Content_Types].xml"),
            "/_rels/[Content_Types].xml.rels"
        );
    }

    #[test]
    fn resolve_relative_target() {
        let p = resolve_rel_target("/xl/workbook.xml", "worksheets/sheet1.xml").unwrap();
        assert_eq!(p, "/xl/worksheets/sheet1.xml");
        let p = resolve_rel_target("/", "xl/workbook.xml").unwrap();
        assert_eq!(p, "/xl/workbook.xml");
    }
}
