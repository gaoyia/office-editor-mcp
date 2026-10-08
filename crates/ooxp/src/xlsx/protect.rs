//! 工作表 `sheetProtection` 与工作簿 `workbookProtection`。
//! 可写 Excel 16.0 的 SHA-512 哈希，不破解密码，不写遗留 XOR。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha512};

use super::layout;
use super::names;
use super::request::{SetSheetProtectionOp, SetWorkbookProtectionOp};
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

const HASH_ATTRS: &[&str] = &[
    "hashValue",
    "saltValue",
    "algorithmName",
    "workbookHashValue",
    "workbookSaltValue",
    "workbookAlgorithmName",
    "revisionsHashValue",
    "revisionsSaltValue",
    "revisionsAlgorithmName",
];

const LEGACY_PASSWORD: &[&str] = &["password", "workbookPassword", "revisionsPassword"];

const SPIN_COUNT: u32 = 100_000;

/// 工作表权限：JSON 键、XML 属性、schema 缺省（true 表示锁住）。
const SHEET_FLAGS: &[(&str, &str, bool)] = &[
    ("objects", "objects", false),
    ("scenarios", "scenarios", false),
    ("formatCells", "formatCells", true),
    ("formatColumns", "formatColumns", true),
    ("formatRows", "formatRows", true),
    ("insertColumns", "insertColumns", true),
    ("insertRows", "insertRows", true),
    ("insertHyperlinks", "insertHyperlinks", true),
    ("deleteColumns", "deleteColumns", true),
    ("deleteRows", "deleteRows", true),
    ("selectLockedCells", "selectLockedCells", false),
    ("sort", "sort", true),
    ("autoFilter", "autoFilter", true),
    ("pivotTables", "pivotTables", true),
    ("selectUnlockedCells", "selectUnlockedCells", false),
];

struct Sha512Spec {
    salt: Vec<u8>,
    hash: Vec<u8>,
    spin: u32,
}

pub(crate) fn inspect_sheet(root: &Element, kind: &str) -> Value {
    if kind == "chartsheet" {
        inspect_chartsheet(root)
    } else {
        inspect_worksheet(root)
    }
}

pub(crate) fn inspect_workbook(wb: &Element) -> Value {
    let el = wb.child_by_local("workbookProtection");
    let mut obj = Map::new();
    obj.insert(
        "lockStructure".into(),
        json!(attr_or(el, "lockStructure", false)),
    );
    obj.insert(
        "lockWindows".into(),
        json!(attr_or(el, "lockWindows", false)),
    );
    obj.insert(
        "lockRevision".into(),
        json!(attr_or(el, "lockRevision", false)),
    );
    obj.insert("hasPassword".into(), json!(el.is_some_and(has_password)));
    Value::Object(obj)
}

fn inspect_worksheet(root: &Element) -> Value {
    let el = root.child_by_local("sheetProtection");
    let mut obj = Map::new();
    obj.insert("enforced".into(), json!(attr_or(el, "sheet", false)));
    obj.insert("hasPassword".into(), json!(el.is_some_and(has_password)));
    for (json_key, xml, default) in SHEET_FLAGS {
        obj.insert((*json_key).into(), json!(attr_or(el, xml, *default)));
    }
    Value::Object(obj)
}

fn inspect_chartsheet(root: &Element) -> Value {
    let el = root.child_by_local("sheetProtection");
    let mut obj = Map::new();
    obj.insert("enforced".into(), json!(attr_or(el, "content", false)));
    obj.insert("hasPassword".into(), json!(el.is_some_and(has_password)));
    obj.insert("objects".into(), json!(attr_or(el, "objects", false)));
    Value::Object(obj)
}

pub(crate) fn apply_sheet(
    sheet: &mut Element,
    kind: &str,
    op: &SetSheetProtectionOp,
    i: usize,
) -> Result<(), Error> {
    if kind == "dialogsheet" || kind == "macrosheet" {
        return Err(Error::with_path(
            ErrorKind::DialogOrMacroSheet,
            format!("ops[{i}].id"),
        ));
    }
    let chart = kind == "chartsheet";
    if chart {
        reject_chartsheet_flags(op, i)?;
    }
    gate_existing_hash(
        sheet.child_by_local("sheetProtection"),
        false,
        op.password.as_deref(),
        i,
    )?;
    if op.enforced == Some(false) {
        remove_child(sheet, "sheetProtection");
        return Ok(());
    }
    let missing = sheet.child_by_local("sheetProtection").is_none();
    if missing && op.enforced != Some(true) && !op.has_flag() {
        return Ok(());
    }
    if missing {
        let mut el = Element::new("sheetProtection");
        if chart {
            el.set_attr("content", "1");
            let objects = op.objects.unwrap_or(true);
            write_bool_attr(&mut el, "objects", objects, false);
        } else {
            el.set_attr("sheet", "1");
            write_bool_attr(&mut el, "objects", op.objects.unwrap_or(true), false);
            write_bool_attr(&mut el, "scenarios", op.scenarios.unwrap_or(true), false);
            apply_sheet_flags(&mut el, op);
        }
        if let Some(pw) = &op.password {
            write_sha512(&mut el, pw, false);
        }
        layout::insert_child_ordered(sheet, el, "sheetProtection");
        return Ok(());
    }
    let el = sheet.child_by_local_mut("sheetProtection").unwrap();
    if chart {
        if op.enforced == Some(true) {
            el.set_attr("content", "1");
        }
        if let Some(v) = op.objects {
            write_bool_attr(el, "objects", v, false);
        }
    } else {
        if op.enforced == Some(true) {
            el.set_attr("sheet", "1");
        }
        if let Some(v) = op.objects {
            write_bool_attr(el, "objects", v, false);
        }
        if let Some(v) = op.scenarios {
            write_bool_attr(el, "scenarios", v, false);
        }
        apply_sheet_flags(el, op);
    }
    if let Some(pw) = &op.password {
        write_sha512(el, pw, false);
    }
    Ok(())
}

pub(crate) fn apply_workbook(
    wb: &mut Element,
    op: &SetWorkbookProtectionOp,
    i: usize,
) -> Result<(), Error> {
    gate_existing_hash(
        wb.child_by_local("workbookProtection"),
        true,
        op.password.as_deref(),
        i,
    )?;
    let cur = wb.child_by_local("workbookProtection");
    let mut lock_structure = attr_or(cur, "lockStructure", false);
    let mut lock_windows = attr_or(cur, "lockWindows", false);
    let mut lock_revision = attr_or(cur, "lockRevision", false);
    if let Some(v) = op.lock_structure {
        lock_structure = v;
    }
    if let Some(v) = op.lock_windows {
        lock_windows = v;
    }
    if let Some(v) = op.lock_revision {
        lock_revision = v;
    }
    if !lock_structure && !lock_windows && !lock_revision {
        remove_child(wb, "workbookProtection");
        return Ok(());
    }
    if wb.child_by_local("workbookProtection").is_none() {
        names::insert_workbook_child(wb, Element::new("workbookProtection"), "workbookProtection");
    }
    let el = wb.child_by_local_mut("workbookProtection").unwrap();
    write_bool_attr(el, "lockStructure", lock_structure, false);
    write_bool_attr(el, "lockWindows", lock_windows, false);
    write_bool_attr(el, "lockRevision", lock_revision, false);
    if let Some(pw) = &op.password {
        write_sha512(el, pw, true);
    }
    Ok(())
}

fn apply_sheet_flags(el: &mut Element, op: &SetSheetProtectionOp) {
    let patches = [
        (op.format_cells, "formatCells", true),
        (op.format_columns, "formatColumns", true),
        (op.format_rows, "formatRows", true),
        (op.insert_columns, "insertColumns", true),
        (op.insert_rows, "insertRows", true),
        (op.insert_hyperlinks, "insertHyperlinks", true),
        (op.delete_columns, "deleteColumns", true),
        (op.delete_rows, "deleteRows", true),
        (op.select_locked_cells, "selectLockedCells", false),
        (op.sort, "sort", true),
        (op.auto_filter, "autoFilter", true),
        (op.pivot_tables, "pivotTables", true),
        (op.select_unlocked_cells, "selectUnlockedCells", false),
    ];
    for (val, name, default) in patches {
        if let Some(v) = val {
            write_bool_attr(el, name, v, default);
        }
    }
}

fn reject_chartsheet_flags(op: &SetSheetProtectionOp, i: usize) -> Result<(), Error> {
    let bad = [
        (op.scenarios.is_some(), "scenarios"),
        (op.format_cells.is_some(), "formatCells"),
        (op.format_columns.is_some(), "formatColumns"),
        (op.format_rows.is_some(), "formatRows"),
        (op.insert_columns.is_some(), "insertColumns"),
        (op.insert_rows.is_some(), "insertRows"),
        (op.insert_hyperlinks.is_some(), "insertHyperlinks"),
        (op.delete_columns.is_some(), "deleteColumns"),
        (op.delete_rows.is_some(), "deleteRows"),
        (op.select_locked_cells.is_some(), "selectLockedCells"),
        (op.sort.is_some(), "sort"),
        (op.auto_filter.is_some(), "autoFilter"),
        (op.pivot_tables.is_some(), "pivotTables"),
        (op.select_unlocked_cells.is_some(), "selectUnlockedCells"),
    ];
    for (hit, field) in bad {
        if hit {
            return Err(Error::with_path(
                ErrorKind::InvalidSheetProtection {
                    field: field.into(),
                },
                format!("ops[{i}].{field}"),
            ));
        }
    }
    Ok(())
}

fn gate_existing_hash(
    el: Option<&Element>,
    workbook: bool,
    password: Option<&str>,
    i: usize,
) -> Result<(), Error> {
    let Some(el) = el.filter(|e| has_password(e)) else {
        return Ok(());
    };
    let Some(pw) = password else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    let Some(spec) = sha512_spec(el, workbook) else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    if !password_matches(pw, &spec) {
        return Err(Error::with_path(
            ErrorKind::ProtectionPasswordWrong,
            format!("ops[{i}].password"),
        ));
    }
    Ok(())
}

fn sha512_spec(el: &Element, workbook: bool) -> Option<Sha512Spec> {
    let (algo_n, hash_n, salt_n, spin_n) = if workbook {
        (
            "workbookAlgorithmName",
            "workbookHashValue",
            "workbookSaltValue",
            "workbookSpinCount",
        )
    } else {
        ("algorithmName", "hashValue", "saltValue", "spinCount")
    };
    let algo = el.attr(algo_n)?;
    if !algo.eq_ignore_ascii_case("SHA-512") {
        return None;
    }
    let hash = STANDARD.decode(el.attr(hash_n)?).ok()?;
    let salt = STANDARD.decode(el.attr(salt_n)?).ok()?;
    let spin: u32 = el.attr(spin_n)?.parse().ok()?;
    if hash.is_empty() || salt.is_empty() || spin == 0 {
        return None;
    }
    Some(Sha512Spec { salt, hash, spin })
}

fn password_matches(password: &str, spec: &Sha512Spec) -> bool {
    sha512_spin(password, &spec.salt, spec.spin) == spec.hash
}

fn write_sha512(el: &mut Element, password: &str, workbook: bool) {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt).expect("系统随机数不可用");
    let hash = sha512_spin(password, &salt, SPIN_COUNT);
    let (algo_n, hash_n, salt_n, spin_n) = if workbook {
        (
            "workbookAlgorithmName",
            "workbookHashValue",
            "workbookSaltValue",
            "workbookSpinCount",
        )
    } else {
        ("algorithmName", "hashValue", "saltValue", "spinCount")
    };
    el.set_attr(algo_n, "SHA-512");
    el.set_attr(hash_n, STANDARD.encode(&hash));
    el.set_attr(salt_n, STANDARD.encode(salt));
    el.set_attr(spin_n, SPIN_COUNT.to_string());
    el.remove_attr("password");
    el.remove_attr("workbookPassword");
    el.remove_attr("revisionsPassword");
}

pub(crate) fn sha512_spin(password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    let mut pwd = Vec::with_capacity(password.len() * 2);
    for u in password.encode_utf16() {
        pwd.extend_from_slice(&u.to_le_bytes());
    }
    let mut digest: [u8; 64] = {
        let mut h = Sha512::new();
        h.update(salt);
        h.update(&pwd);
        h.finalize().into()
    };
    for i in 0..spin {
        let mut h = Sha512::new();
        h.update(digest);
        h.update(i.to_le_bytes());
        digest = h.finalize().into();
    }
    digest.to_vec()
}

fn has_password(el: &Element) -> bool {
    if HASH_ATTRS
        .iter()
        .any(|n| el.attr(n).is_some_and(|s| !s.is_empty()))
    {
        return true;
    }
    LEGACY_PASSWORD.iter().any(|n| {
        el.attr(n)
            .is_some_and(|s| !s.is_empty() && s != "0" && !s.chars().all(|c| c == '0'))
    })
}

fn attr_or(el: Option<&Element>, name: &str, default: bool) -> bool {
    match el.and_then(|e| e.attr(name)) {
        None => default,
        Some(s) => matches!(s, "1" | "true" | "TRUE" | "on"),
    }
}

fn write_bool_attr(el: &mut Element, name: &str, value: bool, default: bool) {
    if value == default {
        el.remove_attr(name);
    } else if value {
        el.set_attr(name, "1");
    } else {
        el.set_attr(name, "0");
    }
}

fn remove_child(parent: &mut Element, local: &str) {
    parent.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        _ => true,
    });
}

#[cfg(test)]
mod hash_tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    #[test]
    fn excel_sheet_sample_matches() {
        let salt = STANDARD.decode("XU0YjfLYkdb5QbNqCNOyHA==").unwrap();
        let want = STANDARD
            .decode("6nKP+s3+T6I4ifGSbzortB+RwF5ZmjfkV3J86fejw5GFCree1zkmdy4Q3+xVUx9SdeihNagWdWE4gIWNb2OkWg==")
            .unwrap();
        assert_eq!(sha512_spin("secret", &salt, 100_000), want);
    }
}
