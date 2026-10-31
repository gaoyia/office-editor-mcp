//! create 写出的骨架 XML。

use super::hf::{HfKind, HfType};
use super::request::create_types::*;
use super::tbl;
use super::textbox::{self, WPS_NS};
use super::{A_NS, MC_NS, PIC_NS, REL_NS, W14_NS, WML_NS, WP_NS};

pub(crate) fn para_xml(
    p: &NewPara,
    id: &str,
    first_break: bool,
    refs_xml: &str,
    title_pg: bool,
) -> String {
    let mut ppr_inner = String::new();
    if let Some(s) = p.style {
        ppr_inner.push_str(&format!(r#"<w:pStyle w:val="{s}"/>"#));
    }
    if let Some(ty) = p.section_type {
        let refs = if first_break { refs_xml } else { "" };
        let title = first_break && title_pg;
        ppr_inner.push_str(&sect_pr_inner_xml(Some(ty.as_str()), refs, title));
    }
    let ppr = if ppr_inner.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{ppr_inner}</w:pPr>")
    };
    let t = if let Some(runs) = &p.runs {
        let mut s = String::new();
        for r in runs {
            s.push_str(&run_t_xml(&r.text));
        }
        s
    } else if p.text.is_empty() {
        String::new()
    } else {
        super::para::visible_run_xml(&p.text)
    };
    let mut boxes = String::new();
    for b in &p.boxes {
        let inner_id = format!("{:08X}", b.inner_para);
        boxes.push_str(&format!(
            "<w:r>{}</w:r>",
            textbox::textbox_ac_xml(b.doc_pr, b.cx, b.cy, &b.text, &inner_id)
        ));
    }
    format!(r#"<w:p w14:paraId="{id}" w14:textId="{id}">{ppr}{t}{boxes}</w:p>"#)
}

pub(crate) fn run_t_xml(text: &str) -> String {
    super::para::visible_run_xml(text)
}

pub(crate) fn sect_pr_inner_xml(ty: Option<&str>, refs_xml: &str, title_pg: bool) -> String {
    let type_xml = ty
        .map(|t| format!(r#"<w:type w:val="{t}"/>"#))
        .unwrap_or_default();
    let title = if title_pg { "<w:titlePg/>" } else { "" };
    format!(
        r#"<w:sectPr>{refs_xml}{type_xml}<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>{title}</w:sectPr>"#
    )
}

pub(crate) fn story_xmlns() -> String {
    format!(
        r#"xmlns:w="{WML_NS}" xmlns:r="{REL_NS}" xmlns:w14="{W14_NS}" xmlns:mc="{MC_NS}" xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:pic="{PIC_NS}" xmlns:wps="{WPS_NS}" mc:Ignorable="w14""#
    )
}

pub(crate) fn document_xml(
    paras: &[NewPara],
    tables: &[NewTable],
    refs: &[(HfKind, HfType, String)],
    title_pg: bool,
) -> String {
    let mut refs_xml = String::new();
    for (kind, hf_type, rid) in refs {
        let tag = match kind {
            HfKind::Header => "headerReference",
            HfKind::Footer => "footerReference",
        };
        refs_xml.push_str(&format!(
            r#"<w:{tag} w:type="{}" r:id="{rid}"/>"#,
            hf_type.as_str()
        ));
    }
    let first_break = paras.iter().position(|p| p.section_type.is_some());
    let body = emit_story_blocks(paras, tables, first_break, &refs_xml, title_pg);
    let last_refs = if first_break.is_some() { "" } else { &refs_xml };
    let last_title = first_break.is_none() && title_pg;
    let last_sect = sect_pr_inner_xml(None, last_refs, last_title);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {}><w:body>{body}{last_sect}</w:body></w:document>"#,
        story_xmlns()
    )
}

pub(crate) fn hf_xml(tag: &str, paras: &[NewPara], tables: &[NewTable]) -> String {
    let body = emit_story_blocks(paras, tables, None, "", false);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} {}>{body}</w:{tag}>"#,
        story_xmlns()
    )
}

pub(crate) fn emit_story_blocks(
    paras: &[NewPara],
    tables: &[NewTable],
    first_break: Option<usize>,
    refs_xml: &str,
    title_pg: bool,
) -> String {
    let mut para_xmls = Vec::new();
    let mut next_para = paras.len() as u32 + 1;
    for p in paras {
        next_para = next_para.saturating_add(p.boxes.len() as u32);
    }
    for (i, p) in paras.iter().enumerate() {
        let id = format!("{:08X}", i as u32 + 1);
        para_xmls.push(para_xml(p, &id, first_break == Some(i), refs_xml, title_pg));
    }
    let mut table_xmls = Vec::new();
    for t in tables {
        let n = t.cells.iter().map(|r| r.len()).sum::<usize>();
        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            ids.push(format!("{:08X}", next_para));
            next_para = next_para.saturating_add(1);
        }
        table_xmls.push(tbl::table_xml(&t.cells, &ids));
    }
    let mut body = String::new();
    append_tables_after(&mut body, tables, &table_xmls, 0);
    for (i, xml) in para_xmls.iter().enumerate() {
        body.push_str(xml);
        append_tables_after(&mut body, tables, &table_xmls, i as u32 + 1);
    }
    body
}

pub(crate) fn append_tables_after(
    body: &mut String,
    tables: &[NewTable],
    xmls: &[String],
    after: u32,
) {
    for (i, t) in tables.iter().enumerate() {
        if t.after == after {
            body.push_str(&xmls[i]);
        }
    }
}

pub(crate) fn styles_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{WML_NS}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="宋体"/><w:sz w:val="22"/><w:color w:val="000000"/></w:rPr></w:rPrDefault><w:pPrDefault/></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>{h1}{h2}{h3}</w:styles>"#,
        h1 = heading_style("Heading1", "heading 1", "0", "32"),
        h2 = heading_style("Heading2", "heading 2", "1", "26"),
        h3 = heading_style("Heading3", "heading 3", "2", "24"),
    )
}

pub(crate) fn settings_xml(even_and_odd: bool) -> String {
    let even = if even_and_odd {
        "<w:evenAndOddHeaders/>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}">{even}<w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

pub(crate) fn heading_style(id: &str, name: &str, outline: &str, sz: &str) -> String {
    format!(
        r#"<w:style w:type="paragraph" w:styleId="{id}"><w:name w:val="{name}"/><w:basedOn w:val="Normal"/><w:uiPriority w:val="9"/><w:qFormat/><w:pPr><w:outlineLvl w:val="{outline}"/><w:spacing w:before="240" w:after="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="{sz}"/><w:color w:val="000000"/></w:rPr></w:style>"#
    )
}

pub(crate) fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

pub(crate) fn app_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application></Properties>"#.to_string()
}
