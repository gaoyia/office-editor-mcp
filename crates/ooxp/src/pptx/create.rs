//! 从领域 JSON 生成 Transitional 演示文稿。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml;

use super::{
    CT_APP, CT_CORE, CT_PRESENTATION, CT_PRES_PROPS, CT_SLIDE, CT_SLIDE_LAYOUT, CT_SLIDE_MASTER,
    CT_THEME, DML_NS, PML_NS, REL_APP, REL_CORE, REL_NS, REL_OFFICE_DOCUMENT, REL_PRES_PROPS,
    REL_SLIDE, REL_SLIDE_LAYOUT, REL_SLIDE_MASTER, REL_THEME,
};

struct NewSlide {
    shapes: Vec<String>,
}

pub fn create(presentation: &Value) -> Result<Vec<u8>, Error> {
    let obj = presentation.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "presentation".into(),
                expected: TypeName::Object,
            },
            "presentation",
        )
    })?;
    for k in obj.keys() {
        if k != "slides" {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("presentation.{k}"),
            ));
        }
    }
    let arr = obj.get("slides").ok_or_else(|| {
        Error::with_path(
            ErrorKind::MissingField {
                field: "slides".into(),
            },
            "presentation.slides",
        )
    })?;
    let arr = arr.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "slides".into(),
                expected: TypeName::Array,
            },
            "presentation.slides",
        )
    })?;

    let mut slides = Vec::new();
    for (i, s) in arr.iter().enumerate() {
        slides.push(parse_slide(s, i)?);
    }
    if slides.is_empty() {
        slides.push(NewSlide {
            shapes: vec![String::new()],
        });
    }
    for s in &mut slides {
        if s.shapes.is_empty() {
            s.shapes.push(String::new());
        }
    }

    let mut pkg = Package::create();
    let mut overrides = vec![
        ("/ppt/presentation.xml".into(), CT_PRESENTATION.into()),
        ("/ppt/presProps.xml".into(), CT_PRES_PROPS.into()),
        (
            "/ppt/slideMasters/slideMaster1.xml".into(),
            CT_SLIDE_MASTER.into(),
        ),
        (
            "/ppt/slideLayouts/slideLayout1.xml".into(),
            CT_SLIDE_LAYOUT.into(),
        ),
        ("/ppt/theme/theme1.xml".into(), CT_THEME.into()),
        ("/docProps/core.xml".into(), CT_CORE.into()),
        ("/docProps/app.xml".into(), CT_APP.into()),
    ];
    for i in 1..=slides.len() {
        overrides.push((format!("/ppt/slides/slide{i}.xml"), CT_SLIDE.into()));
    }
    let types = ContentTypes {
        defaults: vec![
            (
                "rels".into(),
                "application/vnd.openxmlformats-package.relationships+xml".into(),
            ),
            ("xml".into(), "application/xml".into()),
        ],
        overrides,
    };

    let mut root_rels = Relationships { rels: Vec::new() };
    root_rels.push(rel("rId1", REL_OFFICE_DOCUMENT, "ppt/presentation.xml"));
    root_rels.push(rel("rId2", REL_CORE, "docProps/core.xml"));
    root_rels.push(rel("rId3", REL_APP, "docProps/app.xml"));

    let mut pres_rels = Relationships { rels: Vec::new() };
    pres_rels.push(rel(
        "rId1",
        REL_SLIDE_MASTER,
        "slideMasters/slideMaster1.xml",
    ));
    for i in 1..=slides.len() {
        pres_rels.push(rel(
            &format!("rId{}", i + 1),
            REL_SLIDE,
            &format!("slides/slide{i}.xml"),
        ));
    }
    let props_rid = format!("rId{}", slides.len() + 2);
    pres_rels.push(rel(&props_rid, REL_PRES_PROPS, "presProps.xml"));

    let mut master_rels = Relationships { rels: Vec::new() };
    master_rels.push(rel(
        "rId1",
        REL_SLIDE_LAYOUT,
        "../slideLayouts/slideLayout1.xml",
    ));
    master_rels.push(rel("rId2", REL_THEME, "../theme/theme1.xml"));

    let mut layout_rels = Relationships { rels: Vec::new() };
    layout_rels.push(rel(
        "rId1",
        REL_SLIDE_MASTER,
        "../slideMasters/slideMaster1.xml",
    ));

    pkg.set(CONTENT_TYPES_PART, types.to_xml().into_bytes())?;
    pkg.set(ROOT_RELS_PART, root_rels.to_xml().into_bytes())?;
    pkg.set(
        "/ppt/presentation.xml",
        presentation_xml(slides.len()).into_bytes(),
    )?;
    pkg.set(
        "/ppt/_rels/presentation.xml.rels",
        pres_rels.to_xml().into_bytes(),
    )?;
    pkg.set("/ppt/presProps.xml", pres_props_xml().into_bytes())?;
    pkg.set(
        "/ppt/slideMasters/slideMaster1.xml",
        slide_master_xml().into_bytes(),
    )?;
    pkg.set(
        "/ppt/slideMasters/_rels/slideMaster1.xml.rels",
        master_rels.to_xml().into_bytes(),
    )?;
    pkg.set(
        "/ppt/slideLayouts/slideLayout1.xml",
        slide_layout_xml().into_bytes(),
    )?;
    pkg.set(
        "/ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        layout_rels.to_xml().into_bytes(),
    )?;
    pkg.set("/ppt/theme/theme1.xml", theme_xml().into_bytes())?;

    for (i, slide) in slides.iter().enumerate() {
        let n = i + 1;
        pkg.set(
            &format!("/ppt/slides/slide{n}.xml"),
            slide_xml(&slide.shapes).into_bytes(),
        )?;
        let mut slide_rels = Relationships { rels: Vec::new() };
        slide_rels.push(rel(
            "rId1",
            REL_SLIDE_LAYOUT,
            "../slideLayouts/slideLayout1.xml",
        ));
        pkg.set(
            &format!("/ppt/slides/_rels/slide{n}.xml.rels"),
            slide_rels.to_xml().into_bytes(),
        )?;
    }

    pkg.set("/docProps/core.xml", core_xml().into_bytes())?;
    pkg.set("/docProps/app.xml", app_xml(slides.len()).into_bytes())?;
    pkg.save_to_vec()
}

fn parse_slide(v: &Value, i: usize) -> Result<NewSlide, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("presentation.slides[{i}]"),
                expected: TypeName::Object,
            },
            format!("presentation.slides[{i}]"),
        )
    })?;
    for k in obj.keys() {
        if k != "shapes" {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("presentation.slides[{i}].{k}"),
            ));
        }
    }
    let mut shapes = Vec::new();
    if let Some(arr) = obj.get("shapes") {
        let arr = arr.as_array().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: format!("presentation.slides[{i}].shapes"),
                    expected: TypeName::Array,
                },
                format!("presentation.slides[{i}].shapes"),
            )
        })?;
        for (j, sh) in arr.iter().enumerate() {
            shapes.push(parse_shape(sh, i, j)?);
        }
    }
    Ok(NewSlide { shapes })
}

fn parse_shape(v: &Value, slide: usize, i: usize) -> Result<String, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: format!("presentation.slides[{slide}].shapes[{i}]"),
                expected: TypeName::Object,
            },
            format!("presentation.slides[{slide}].shapes[{i}]"),
        )
    })?;
    for k in obj.keys() {
        if k != "text" {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("presentation.slides[{slide}].shapes[{i}].{k}"),
            ));
        }
    }
    match obj.get("text") {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "text".into(),
                expected: TypeName::String,
            },
            format!("presentation.slides[{slide}].shapes[{i}].text"),
        )),
    }
}

fn rel(id: &str, rel_type: &str, target: &str) -> Relationship {
    Relationship {
        id: id.into(),
        rel_type: rel_type.into(),
        target: target.into(),
        target_mode: None,
        extra: Vec::new(),
    }
}

fn presentation_xml(slide_count: usize) -> String {
    let mut ids = String::new();
    for i in 0..slide_count {
        let sid = 256 + i as u32;
        let rid = i + 2;
        ids.push_str(&format!(r#"<p:sldId id="{sid}" r:id="rId{rid}"/>"#));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#
    )
}

fn pres_props_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:clrMru/></p:presentationPr>"#
    )
}

fn empty_sp_tree() -> String {
    format!(
        r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#
    )
}

fn slide_xml(shapes: &[String]) -> String {
    let tree = empty_sp_tree();
    let mut body = String::new();
    for (i, text) in shapes.iter().enumerate() {
        let id = i as u32 + 2;
        let name = format!("文本框 {}", i + 1);
        let y = 685800 + i as i64 * 914400;
        body.push_str(&text_box_xml(id, &name, 838200, y, text));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:spTree>{tree}{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    )
}

fn text_box_xml(id: u32, name: &str, x: i64, y: i64, text: &str) -> String {
    let paras = text_paragraphs(text);
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="10515600" cy="800100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap="square" lIns="91440" tIns="45720" rIns="91440" bIns="45720"/><a:lstStyle/>{paras}</p:txBody></p:sp>"#,
        xml::escape_attr(name),
    )
}

fn text_paragraphs(text: &str) -> String {
    let mut out = String::from("<a:p>");
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
            r#"<a:r><a:rPr lang="zh-CN" sz="1800" dirty="0"><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:latin typeface="Calibri"/><a:ea typeface="宋体"/></a:rPr><a:t{space}>{}</a:t></a:r>"#,
            xml::escape_text(line)
        ));
    }
    out.push_str("</a:p>");
    out
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

fn slide_layout_xml() -> String {
    let tree = empty_sp_tree();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}" type="blank" preserve="1"><p:cSld name="空白"><p:spTree>{tree}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#
    )
}

fn slide_master_xml() -> String {
    let tree = empty_sp_tree();
    let styles = tx_styles();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="{DML_NS}" xmlns:r="{REL_NS}" xmlns:p="{PML_NS}"><p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree>{tree}</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>{styles}</p:sldMaster>"#
    )
}

fn tx_styles() -> String {
    let title = levels("4400");
    let body = levels("1800");
    let other = levels("1800");
    format!(
        r#"<p:txStyles><p:titleStyle>{title}</p:titleStyle><p:bodyStyle>{body}</p:bodyStyle><p:otherStyle>{other}</p:otherStyle></p:txStyles>"#
    )
}

fn levels(sz: &str) -> String {
    (1..=9)
        .map(|n| format!(r#"<a:lvl{n}pPr algn="l" rtl="0"><a:defRPr sz="{sz}"/></a:lvl{n}pPr>"#))
        .collect()
}

fn theme_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="{DML_NS}" name="Office Theme"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="4F81BD"/></a:accent1><a:accent2><a:srgbClr val="C0504D"/></a:accent2><a:accent3><a:srgbClr val="9BBB59"/></a:accent3><a:accent4><a:srgbClr val="8064A2"/></a:accent4><a:accent5><a:srgbClr val="4BACC6"/></a:accent5><a:accent6><a:srgbClr val="F79646"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Office">{fmt}</a:fmtScheme></a:themeElements></a:theme>"#,
        fmt = fmt_scheme(),
    )
}

fn fmt_scheme() -> String {
    r#"<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="50000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="35000"><a:schemeClr val="phClr"><a:tint val="37000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="15000"/><a:satMod val="350000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="1"/></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="100000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="80000"><a:schemeClr val="phClr"><a:shade val="30000"/><a:satMod val="200000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="200000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"><a:shade val="95000"/><a:satMod val="105000"/></a:schemeClr></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="25400" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="38100" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="38000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="40000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="40000"><a:schemeClr val="phClr"><a:tint val="45000"/><a:shade val="99000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="255000"/></a:schemeClr></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="50000" t="-80000" r="50000" b="180000"/></a:path></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="80000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="30000"/><a:satMod val="200000"/></a:schemeClr></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="50000" t="50000" r="50000" b="50000"/></a:path></a:gradFill></a:bgFillStyleLst>"#
        .to_string()
}

fn core_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title/><dc:creator>ooxp</dc:creator></cp:coreProperties>"#.to_string()
}

fn app_xml(slides: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ooxp</Application><Slides>{slides}</Slides></Properties>"#
    )
}
