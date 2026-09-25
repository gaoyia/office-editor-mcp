//! 从领域 JSON 生成 Transitional 文档。

use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml;

use super::{
    CT_APP, CT_CORE, CT_DOCUMENT, CT_STYLES, MC_NS, REL_APP, REL_CORE, REL_NS, REL_OFFICE_DOCUMENT,
    REL_STYLES, W14_NS, WML_NS,
};

struct NewPara {
    text: String,
    style: Option<&'static str>,
}

pub fn create(document: &Value) -> Result<Vec<u8>, Error> {
    let obj = document.as_object().ok_or_else(|| {
        Error::with_path(ErrorCode::InvalidField, "document 必须是对象", "document")
    })?;
    for k in obj.keys() {
        if k != "paragraphs" {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                format!("不认识的字段：{k}"),
                format!("document.{k}"),
            ));
        }
    }
    let arr = obj.get("paragraphs").ok_or_else(|| {
        Error::with_path(
            ErrorCode::InvalidField,
            "缺少 paragraphs",
            "document.paragraphs",
        )
    })?;
    let arr = arr.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorCode::InvalidField,
            "paragraphs 必须是数组",
            "document.paragraphs",
        )
    })?;

    let mut paras = Vec::new();
    for (i, p) in arr.iter().enumerate() {
        paras.push(parse_para(p, i)?);
    }
    if paras.is_empty() {
        paras.push(NewPara {
            text: String::new(),
            style: None,
        });
    }

    let mut pkg = Package::create();
    let types = ContentTypes {
        defaults: vec![
            (
                "rels".into(),
                "application/vnd.openxmlformats-package.relationships+xml".into(),
            ),
            ("xml".into(), "application/xml".into()),
        ],
        overrides: vec![
            ("/word/document.xml".into(), CT_DOCUMENT.into()),
            ("/word/styles.xml".into(), CT_STYLES.into()),
            ("/docProps/core.xml".into(), CT_CORE.into()),
            ("/docProps/app.xml".into(), CT_APP.into()),
        ],
    };

    let mut root_rels = Relationships { rels: Vec::new() };
    root_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_OFFICE_DOCUMENT.into(),
        target: "word/document.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId2".into(),
        rel_type: REL_CORE.into(),
        target: "docProps/core.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });
    root_rels.push(Relationship {
        id: "rId3".into(),
        rel_type: REL_APP.into(),
        target: "docProps/app.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });

    let mut doc_rels = Relationships { rels: Vec::new() };
    doc_rels.push(Relationship {
        id: "rId1".into(),
        rel_type: REL_STYLES.into(),
        target: "styles.xml".into(),
        target_mode: None,
        extra: Vec::new(),
    });

    pkg.set(CONTENT_TYPES_PART, types.to_xml().into_bytes())?;
    pkg.set(ROOT_RELS_PART, root_rels.to_xml().into_bytes())?;
    pkg.set("/word/document.xml", document_xml(&paras).into_bytes())?;
    pkg.set(
        "/word/_rels/document.xml.rels",
        doc_rels.to_xml().into_bytes(),
    )?;
    pkg.set("/word/styles.xml", styles_xml().into_bytes())?;
    pkg.set("/docProps/core.xml", core_xml().into_bytes())?;
    pkg.set("/docProps/app.xml", app_xml().into_bytes())?;
    pkg.save_to_vec()
}

fn parse_para(v: &Value, i: usize) -> Result<NewPara, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorCode::InvalidField,
            "段落必须是对象",
            format!("document.paragraphs[{i}]"),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "text" | "style") {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                format!("不认识的字段：{k}"),
                format!("document.paragraphs[{i}].{k}"),
            ));
        }
    }
    let text = match obj.get("text") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "text 必须是字符串",
                format!("document.paragraphs[{i}].text"),
            ));
        }
    };
    let style = match obj.get("style") {
        None => None,
        Some(Value::String(s)) => canonicalize_style(s, i)?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorCode::InvalidField,
                "style 必须是字符串",
                format!("document.paragraphs[{i}].style"),
            ));
        }
    };
    Ok(NewPara { text, style })
}

fn canonicalize_style(s: &str, i: usize) -> Result<Option<&'static str>, Error> {
    if s.eq_ignore_ascii_case("Normal") {
        return Ok(None);
    }
    for c in ["Heading1", "Heading2", "Heading3"] {
        if s.eq_ignore_ascii_case(c) {
            return Ok(Some(c));
        }
    }
    Err(Error::with_path(
        ErrorCode::InvalidField,
        "style 只允许 Normal、Heading1、Heading2、Heading3",
        format!("document.paragraphs[{i}].style"),
    ))
}

fn document_xml(paras: &[NewPara]) -> String {
    let mut body = String::new();
    for (i, p) in paras.iter().enumerate() {
        let id = format!("{:08X}", i as u32 + 1);
        let style = if let Some(s) = p.style {
            format!(r#"<w:pPr><w:pStyle w:val="{s}"/></w:pPr>"#)
        } else {
            String::new()
        };
        let space = if needs_xml_space(&p.text) {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        let t = if p.text.is_empty() {
            String::new()
        } else {
            format!("<w:r><w:t{space}>{}</w:t></w:r>", xml::escape_text(&p.text))
        };
        body.push_str(&format!(
            r#"<w:p w14:paraId="{id}" w14:textId="{id}">{style}{t}</w:p>"#
        ));
    }
    body.push_str(
        r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>"#,
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" mc:Ignorable="w14"><w:body>{body}</w:body></w:document>"#
    )
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ')
        || text.starts_with('\t')
        || text.ends_with(' ')
        || text.ends_with('\t')
        || text.contains('\n')
}

fn styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{WML_NS}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="宋体"/><w:sz w:val="22"/><w:color w:val="000000"/></w:rPr></w:rPrDefault><w:pPrDefault/></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>{h1}{h2}{h3}</w:styles>"#,
        h1 = heading_style("Heading1", "heading 1", "0", "32"),
        h2 = heading_style("Heading2", "heading 2", "1", "26"),
        h3 = heading_style("Heading3", "heading 3", "2", "24"),
    )
}

fn heading_style(id: &str, name: &str, outline: &str, sz: &str) -> String {
    format!(
        r#"<w:style w:type="paragraph" w:styleId="{id}"><w:name w:val="{name}"/><w:basedOn w:val="Normal"/><w:uiPriority w:val="9"/><w:qFormat/><w:pPr><w:outlineLvl w:val="{outline}"/><w:spacing w:before="240" w:after="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="{sz}"/><w:color w:val="000000"/></w:rPr></w:style>"#
    )
}

fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

fn app_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application></Properties>"#.to_string()
}
