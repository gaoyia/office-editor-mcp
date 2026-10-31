//! create 写出的骨架 XML。

use crate::error::{Error, ErrorKind};
use crate::xml;

use super::a1;
use super::request::create_types::*;
use super::request::SetCellValue;
use super::{REL_NS, SML_NS};

pub(crate) fn workbook_xml(sheets: &[NewSheet], has_formula: bool) -> String {
    let mut body = String::new();
    let mut i = 0u32;
    for sh in sheets {
        if sh.chartsheet.is_some() {
            continue;
        }
        i += 1;
        body.push_str(&format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            xml::escape_attr(&sh.name),
            i,
            i
        ));
    }
    let calc = if has_formula {
        r#"<calcPr fullCalcOnLoad="1"/>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="{SML_NS}" xmlns:r="{REL_NS}"><sheets>{body}</sheets>{calc}</workbook>"#
    )
}

pub(crate) fn sheet_xml(sheet: &NewSheet, sst: &[String]) -> Result<String, Error> {
    use std::collections::BTreeMap;
    let mut rows: BTreeMap<u32, Vec<&NewCell>> = BTreeMap::new();
    for c in &sheet.cells {
        rows.entry(c.refer.row).or_default().push(c);
    }
    let mut dim = "A1".to_string();
    if !sheet.cells.is_empty() {
        let min_col = sheet.cells.iter().map(|c| c.refer.col).min().unwrap();
        let max_col = sheet.cells.iter().map(|c| c.refer.col).max().unwrap();
        let min_row = sheet.cells.iter().map(|c| c.refer.row).min().unwrap();
        let max_row = sheet.cells.iter().map(|c| c.refer.row).max().unwrap();
        dim = format!(
            "{}:{}",
            a1::format_cell(min_col, min_row),
            a1::format_cell(max_col, max_row)
        );
    }
    let mut data = String::new();
    for (r, cells) in rows {
        let mut inner = String::new();
        for c in cells {
            inner.push_str(&cell_xml(c, sst)?);
        }
        data.push_str(&format!(r#"<row r="{r}">{inner}</row>"#));
    }
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="{SML_NS}" xmlns:r="{REL_NS}"><dimension ref="{dim}"/><sheetData>{data}</sheetData></worksheet>"#
    ))
}

pub(crate) fn cell_xml(c: &NewCell, sst: &[String]) -> Result<String, Error> {
    if let Some(f) = &c.formula {
        let body = xml::escape_text(f.strip_prefix('=').unwrap_or(f));
        return Ok(format!(r#"<c r="{}"><f>{body}</f></c>"#, c.a1));
    }
    match c.value.as_ref() {
        Some(SetCellValue::Number(n)) => {
            let v = if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                n.to_string()
            };
            Ok(format!(r#"<c r="{}"><v>{v}</v></c>"#, c.a1))
        }
        Some(SetCellValue::Bool(b)) => Ok(format!(
            r#"<c r="{}" t="b"><v>{}</v></c>"#,
            c.a1,
            if *b { "1" } else { "0" }
        )),
        Some(SetCellValue::Text(s)) => {
            let idx = sst
                .iter()
                .position(|x| x == s)
                .ok_or_else(|| Error::new(ErrorKind::SharedStringNotInTable { text: s.clone() }))?;
            Ok(format!(r#"<c r="{}" t="s"><v>{idx}</v></c>"#, c.a1))
        }
        _ => Ok(format!(r#"<c r="{}"/>"#, c.a1)),
    }
}

pub(crate) fn sst_xml(sst: &[String]) -> String {
    let count = sst.len();
    let mut body = String::new();
    for s in sst {
        let space = if s.starts_with(' ') || s.ends_with(' ') || s.contains('\n') {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        body.push_str(&format!("<si><t{space}>{}</t></si>", xml::escape_text(s)));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="{SML_NS}" count="{count}" uniqueCount="{count}">{body}</sst>"#
    )
}

pub(crate) fn styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="{SML_NS}"><fonts count="1"><font><sz val="11"/><color rgb="FF000000"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
    )
}

pub(crate) fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

pub(crate) fn app_xml(sheets: &[NewSheet]) -> String {
    let n = sheets.len();
    let n_ws = sheets.iter().filter(|s| s.chartsheet.is_none()).count();
    let n_cs = n - n_ws;
    let mut titles = String::new();
    for sh in sheets {
        titles.push_str(&format!(
            "<vt:lpstr>{}</vt:lpstr>",
            xml::escape_text(&sh.name)
        ));
    }
    let pairs = if n_cs == 0 {
        format!(
            r#"<HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_ws}</vt:i4></vt:variant></vt:vector></HeadingPairs>"#
        )
    } else {
        format!(
            r#"<HeadingPairs><vt:vector size="4" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_ws}</vt:i4></vt:variant><vt:variant><vt:lpstr>Charts</vt:lpstr></vt:variant><vt:variant><vt:i4>{n_cs}</vt:i4></vt:variant></vt:vector></HeadingPairs>"#
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application>{pairs}<TitlesOfParts><vt:vector size="{n}" baseType="lpstr">{titles}</vt:vector></TitlesOfParts></Properties>"#
    )
}
