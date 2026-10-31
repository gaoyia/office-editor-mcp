//! create 写出的骨架 XML。

use crate::xml;

use super::request::create_types::NewShape;
use super::{DML_NS, PML_NS, REL_NS};

pub(crate) fn presentation_xml(slide_count: usize, notes_cx: i64, notes_cy: i64) -> String {
    let mut ids = String::new();
    for i in 0..slide_count {
        let sid = 256 + i as u32;
        let rid = i + 2;
        ids.push_str(&format!(r#"<p:sldId id="{sid}" r:id="rId{rid}"/>"#));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="{notes_cx}" cy="{notes_cy}"/></p:presentation>"#
    )
}

pub(crate) fn pres_props_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:clrMru/></p:presentationPr>"#
    )
}

pub(crate) fn empty_sp_tree() -> String {
    r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#
        .to_string()
}

pub(crate) fn slide_xml(shapes: &[NewShape]) -> String {
    let tree = empty_sp_tree();
    let mut body = String::new();
    for (i, sh) in shapes.iter().enumerate() {
        let id = i as u32 + 2;
        let name = format!("文本框 {}", i + 1);
        let y = 685800 + i as i64 * 914400;
        body.push_str(&text_box_xml(
            id,
            &name,
            838200,
            y,
            &sh.text,
            sh.format.as_ref(),
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:spTree>{tree}{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    )
}

pub(crate) fn text_box_xml(
    id: u32,
    name: &str,
    x: i64,
    y: i64,
    text: &str,
    format: Option<&crate::drawing_text::TextFormat>,
) -> String {
    let paras = text_paragraphs(text, format);
    let body_anchor = match format.and_then(|f| f.text_anchor.as_ref()) {
        Some(Some(v)) => format!(r#" anchor="{v}""#),
        _ => String::new(),
    };
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="10515600" cy="800100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap="square" lIns="91440" tIns="45720" rIns="91440" bIns="45720"{body_anchor}/><a:lstStyle/>{paras}</p:txBody></p:sp>"#,
        xml::escape_attr(name),
    )
}

pub(crate) fn text_paragraphs(
    text: &str,
    format: Option<&crate::drawing_text::TextFormat>,
) -> String {
    let ppr = match format.and_then(|f| f.align.as_ref()) {
        Some(Some(v)) => format!(r#"<a:pPr algn="{v}"/>"#),
        _ => String::new(),
    };
    let rpr = create_rpr(format);
    let mut out = format!("<a:p>{ppr}");
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push_str("<a:br/>");
        }
        let space = if needs_xml_space(line) {
            r#" xml:space="preserve""#
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<a:r>{rpr}<a:t{space}>{}</a:t></a:r>"#,
            xml::escape_text(line)
        ));
    }
    out.push_str("</a:p>");
    out
}

pub(crate) fn create_rpr(format: Option<&crate::drawing_text::TextFormat>) -> String {
    let mut sz = Some(1800i64);
    let mut color = Some("000000".to_string());
    let mut fill: Option<Option<crate::drawing_text::TextFill>> = None;
    let mut outline: Option<Option<crate::drawing_text::TextOutline>> = None;
    let mut bold = None;
    let mut italic = None;
    let mut underline = None;
    let mut latin = Some("Calibri".to_string());
    let mut ea = Some("宋体".to_string());
    if let Some(f) = format {
        if let Some(v) = &f.sz {
            sz = *v;
        }
        if f.color.is_some() || f.text_fill.is_some() {
            color = None;
        }
        if let Some(v) = &f.color {
            color = v.clone();
        }
        fill = f.text_fill.clone();
        outline = f.text_outline.clone();
        bold = f.bold;
        italic = f.italic;
        if let Some(v) = &f.underline {
            underline = v.clone();
        }
        if let Some(v) = &f.font {
            latin = Some(v.clone());
        }
        if let Some(v) = &f.east_asia_font {
            ea = Some(v.clone());
        }
    }
    let mut attrs = String::from(r#" lang="zh-CN" dirty="0""#);
    if let Some(n) = sz {
        attrs.push_str(&format!(r#" sz="{n}""#));
    }
    if let Some(true) = bold {
        attrs.push_str(r#" b="1""#);
    } else if let Some(false) = bold {
        attrs.push_str(r#" b="0""#);
    }
    if let Some(true) = italic {
        attrs.push_str(r#" i="1""#);
    } else if let Some(false) = italic {
        attrs.push_str(r#" i="0""#);
    }
    if let Some(u) = underline {
        attrs.push_str(&format!(r#" u="{u}""#));
    }
    let mut inner = String::new();
    if let Some(Some(o)) = &outline {
        inner.push_str(&format!(
            r#"<a:ln w="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:ln>"#,
            o.width,
            o.color.to_ascii_uppercase()
        ));
    }
    if let Some(Some(crate::drawing_text::TextFill::None)) = &fill {
        inner.push_str("<a:noFill/>");
    } else if let Some(Some(crate::drawing_text::TextFill::Gradient { ang, stops })) = &fill {
        inner.push_str(r#"<a:gradFill flip="none" rotWithShape="1"><a:gsLst>"#);
        for stop in stops {
            inner.push_str(&format!(
                r#"<a:gs pos="{}"><a:srgbClr val="{}"/></a:gs>"#,
                stop.pos,
                stop.color.to_ascii_uppercase()
            ));
        }
        inner.push_str(&format!(
            r#"</a:gsLst><a:lin ang="{ang}" scaled="1"/><a:tileRect/></a:gradFill>"#
        ));
    } else if let Some(hex) = color {
        inner.push_str(&format!(
            r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#,
            hex.to_ascii_uppercase()
        ));
    }
    if let Some(face) = latin {
        inner.push_str(&format!(
            r#"<a:latin typeface="{}"/>"#,
            xml::escape_attr(&face)
        ));
    }
    if let Some(face) = ea {
        inner.push_str(&format!(
            r#"<a:ea typeface="{}"/>"#,
            xml::escape_attr(&face)
        ));
    }
    format!("<a:rPr{attrs}>{inner}</a:rPr>")
}

pub(crate) fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

pub(crate) fn slide_layout_xml() -> String {
    let tree = empty_sp_tree();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}" type="blank" preserve="1"><p:cSld name="空白"><p:spTree>{tree}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#
    )
}

pub(crate) fn slide_master_xml() -> String {
    let tree = empty_sp_tree();
    let styles = tx_styles();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree>{tree}</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>{styles}</p:sldMaster>"#
    )
}

pub(crate) fn tx_styles() -> String {
    let title = levels("4400");
    let body = levels("1800");
    let other = levels("1800");
    format!(
        r#"<p:txStyles><p:titleStyle>{title}</p:titleStyle><p:bodyStyle>{body}</p:bodyStyle><p:otherStyle>{other}</p:otherStyle></p:txStyles>"#
    )
}

pub(crate) fn levels(sz: &str) -> String {
    (1..=9)
        .map(|n| format!(r#"<a:lvl{n}pPr algn="l" rtl="0"><a:defRPr sz="{sz}"/></a:lvl{n}pPr>"#))
        .collect()
}

pub(crate) fn theme_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="{DML_NS}" name="Office Theme"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="4F81BD"/></a:accent1><a:accent2><a:srgbClr val="C0504D"/></a:accent2><a:accent3><a:srgbClr val="9BBB59"/></a:accent3><a:accent4><a:srgbClr val="8064A2"/></a:accent4><a:accent5><a:srgbClr val="4BACC6"/></a:accent5><a:accent6><a:srgbClr val="F79646"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Office">{fmt}</a:fmtScheme></a:themeElements></a:theme>"#,
        fmt = fmt_scheme(),
    )
}

pub(crate) fn fmt_scheme() -> String {
    r#"<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="50000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="35000"><a:schemeClr val="phClr"><a:tint val="37000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="15000"/><a:satMod val="350000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="1"/></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="100000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="80000"><a:schemeClr val="phClr"><a:shade val="30000"/><a:satMod val="200000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="200000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"><a:shade val="95000"/><a:satMod val="105000"/></a:schemeClr></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="25400" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="38100" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="38000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="40000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="40000"><a:schemeClr val="phClr"><a:tint val="45000"/><a:shade val="99000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="255000"/></a:schemeClr></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="50000" t="-80000" r="50000" b="180000"/></a:path></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="80000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="30000"/><a:satMod val="200000"/></a:schemeClr></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="50000" t="50000" r="50000" b="50000"/></a:path></a:gradFill></a:bgFillStyleLst>"#
        .to_string()
}

pub(crate) fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

pub(crate) fn app_xml(slides: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application><Slides>{slides}</Slides></Properties>"#
    )
}
