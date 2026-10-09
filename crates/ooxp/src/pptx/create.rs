//! 从领域 JSON 生成 Transitional 演示文稿。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};
use crate::opc::{
    ContentTypes, Package, Relationship, Relationships, CONTENT_TYPES_PART, ROOT_RELS_PART,
};
use crate::xml;

use super::{
    chart, comments, hyperlink, media, notes, picture,
    request::{
        self, AnimTarget, CreateAnim, CreateAnimHost, CreateHyperlink, CreateSection,
        InsertChartOp, InsertCommentOp, InsertHyperlinkOp, InsertMediaOp, InsertPictureOp,
        InsertSlideSectionOp, InsertSmartArtOp, InsertTableOp, PptxLinkKind, SetNotesTextOp,
        SetTransitionOp,
    },
    section,
    shape::{self, ShapeKey, ShapeTarget, SlideMeta},
    smartart, table, timing, transition, CT_APP, CT_CORE, CT_PRESENTATION, CT_PRES_PROPS, CT_SLIDE,
    CT_SLIDE_LAYOUT, CT_SLIDE_MASTER, CT_THEME, DML_NS, PML_NS, REL_APP, REL_CORE, REL_NS,
    REL_OFFICE_DOCUMENT, REL_PRES_PROPS, REL_SLIDE, REL_SLIDE_LAYOUT, REL_SLIDE_MASTER, REL_THEME,
};

struct NewShape {
    text: String,
    link: Option<CreateHyperlink>,
    format: Option<crate::drawing_text::TextFormat>,
}

struct NewSlide {
    shapes: Vec<NewShape>,
    pictures: Vec<InsertPictureOp>,
    videos: Vec<InsertMediaOp>,
    audios: Vec<InsertMediaOp>,
    charts: Vec<InsertChartOp>,
    smart_arts: Vec<InsertSmartArtOp>,
    tables: Vec<InsertTableOp>,
    notes: Option<String>,
    comments: Vec<InsertCommentOp>,
    transition: Option<SetTransitionOp>,
    anims: Vec<CreateAnim>,
    model3ds: Vec<super::model3d::InsertModel3dOp>,
    maths: Vec<super::omml::InsertMathOp>,
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
        if k != "slides" && k != "props" && k != "sections" {
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
        slides.push(empty_slide());
    }
    for s in &mut slides {
        if s.shapes.is_empty() {
            s.shapes.push(NewShape {
                text: String::new(),
                link: None,
                format: None,
            });
        }
    }
    let sections = parse_sections(obj.get("sections"), slides.len())?;
    validate_anims(&slides)?;

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

    attach_pictures(&mut pkg, &slides)?;
    attach_maths(&mut pkg, &slides)?;
    attach_model3ds(&mut pkg, &slides)?;
    attach_media(&mut pkg, &slides, media::MediaKind::Video)?;
    attach_media(&mut pkg, &slides, media::MediaKind::Audio)?;
    attach_charts(&mut pkg, &slides)?;
    attach_smartarts(&mut pkg, &slides)?;
    attach_tables(&mut pkg, &slides)?;
    attach_notes(&mut pkg, &slides)?;
    attach_comments(&mut pkg, &slides)?;
    attach_hyperlinks(&mut pkg, &slides)?;
    attach_transitions(&mut pkg, &slides)?;
    attach_anims(&mut pkg, &slides)?;
    attach_sections(&mut pkg, &sections)?;

    pkg.set("/docProps/core.xml", core_xml().into_bytes())?;
    pkg.set("/docProps/app.xml", app_xml(slides.len()).into_bytes())?;
    crate::props::apply_create(&mut pkg, obj.get("props"), "presentation.props")?;
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
        if !matches!(
            k.as_str(),
            "shapes"
                | "pictures"
                | "videos"
                | "audios"
                | "charts"
                | "smartArts"
                | "tables"
                | "notes"
                | "comments"
                | "transition"
                | "anims"
                | "model3ds"
                | "maths"
        ) {
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
    let pictures = parse_pictures(obj.get("pictures"), i)?;
    let videos = parse_media(obj.get("videos"), i, "videos")?;
    let audios = parse_media(obj.get("audios"), i, "audios")?;
    let charts = parse_charts(obj.get("charts"), i)?;
    let smart_arts = parse_smartarts(obj.get("smartArts"), i)?;
    let tables = parse_tables(obj.get("tables"), i)?;
    let notes = parse_notes(obj.get("notes"), i)?;
    let comments = parse_comments(obj.get("comments"), i)?;
    let transition = parse_transition(obj.get("transition"), i)?;
    let anims = parse_anims(obj.get("anims"), i)?;
    let model3ds = parse_model3ds(obj.get("model3ds"), i)?;
    let maths = parse_maths(obj.get("maths"), i)?;
    Ok(NewSlide {
        shapes,
        pictures,
        videos,
        audios,
        charts,
        smart_arts,
        tables,
        notes,
        comments,
        transition,
        anims,
        model3ds,
        maths,
    })
}

fn parse_pictures(v: Option<&Value>, slide_i: usize) -> Result<Vec<InsertPictureOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].pictures");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pictures".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_picture(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_model3ds(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<super::model3d::InsertModel3dOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].model3ds");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "model3ds".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::model3d::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_maths(v: Option<&Value>, slide_i: usize) -> Result<Vec<super::omml::InsertMathOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].maths");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "maths".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::omml::parse_create(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_charts(v: Option<&Value>, slide_i: usize) -> Result<Vec<InsertChartOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_chart(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_smartarts(v: Option<&Value>, slide_i: usize) -> Result<Vec<InsertSmartArtOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].smartArts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "smartArts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_smartart(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_tables(v: Option<&Value>, slide_i: usize) -> Result<Vec<InsertTableOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].tables");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "tables".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_table(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_notes(v: Option<&Value>, slide_i: usize) -> Result<Option<String>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        Value::String(s) => Ok(Some(s.clone())),
        _ => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "notes".into(),
                expected: TypeName::String,
            },
            format!("presentation.slides[{slide_i}].notes"),
        )),
    }
}

fn parse_comments(v: Option<&Value>, slide_i: usize) -> Result<Vec<InsertCommentOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].comments");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "comments".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_comment(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn attach_pictures(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.pictures.is_empty()) {
        return Ok(());
    }
    let pres = xml::parse(pkg.get("/ppt/presentation.xml")?)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.pictures.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in slide.pictures.iter().enumerate() {
            picture::insert_picture(
                pkg,
                &mut doc,
                &part,
                &pres.root,
                op,
                &format!("presentation.slides[{i}].pictures[{j}].file"),
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_charts(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.charts.is_empty()) {
        return Ok(());
    }
    let pres = xml::parse(pkg.get("/ppt/presentation.xml")?)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.charts.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in slide.charts.iter().enumerate() {
            chart::insert_chart(pkg, &mut doc, &part, &pres.root, op, j)?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_smartarts(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.smart_arts.is_empty()) {
        return Ok(());
    }
    let pres = xml::parse(pkg.get("/ppt/presentation.xml")?)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.smart_arts.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in slide.smart_arts.iter().enumerate() {
            smartart::insert_smartart(pkg, &mut doc, &part, &pres.root, op, j)?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_tables(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.tables.is_empty()) {
        return Ok(());
    }
    let pres = xml::parse(pkg.get("/ppt/presentation.xml")?)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.tables.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in slide.tables.iter().enumerate() {
            table::insert_table(&mut doc, &pres.root, op, j)?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_notes(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.notes.is_none()) {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let mut pres = xml::parse(pkg.get(pres_part)?)?;
    for (i, slide) in slides.iter().enumerate() {
        let Some(text) = &slide.notes else {
            continue;
        };
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let op = SetNotesTextOp {
            slide: n,
            id: format!("slide:{n}"),
            text: text.clone(),
        };
        notes::apply_set_notes_text(pkg, pres_part, &mut pres, &part, &op, 0)?;
    }
    Ok(())
}

fn attach_comments(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.comments.is_empty()) {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let pres = xml::parse(pkg.get(pres_part)?)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.comments.is_empty() {
            continue;
        }
        let n = i + 1;
        let meta = SlideMeta {
            index: n,
            part: format!("/ppt/slides/slide{n}.xml"),
            name: None,
            hidden: false,
            sld_id: None,
        };
        for (j, op) in slide.comments.iter().enumerate() {
            comments::apply_insert(pkg, pres_part, &pres, &meta, op, j)?;
        }
    }
    Ok(())
}

fn empty_slide() -> NewSlide {
    NewSlide {
        shapes: vec![NewShape {
            text: String::new(),
            link: None,
            format: None,
        }],
        pictures: Vec::new(),
        videos: Vec::new(),
        audios: Vec::new(),
        charts: Vec::new(),
        smart_arts: Vec::new(),
        tables: Vec::new(),
        notes: None,
        comments: Vec::new(),
        transition: None,
        anims: Vec::new(),
        model3ds: Vec::new(),
        maths: Vec::new(),
    }
}

fn parse_media(
    v: Option<&Value>,
    slide_i: usize,
    field: &str,
) -> Result<Vec<InsertMediaOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].{field}");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_media(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}

fn parse_transition(v: Option<&Value>, slide_i: usize) -> Result<Option<SetTransitionOp>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    Ok(Some(request::parse_create_transition(
        v,
        &format!("presentation.slides[{slide_i}].transition"),
        slide_i + 1,
    )?))
}

fn parse_anims(v: Option<&Value>, slide_i: usize) -> Result<Vec<CreateAnim>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].anims");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "anims".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(request::parse_create_anim(item, &format!("{prefix}[{j}]"))?);
    }
    Ok(out)
}

fn parse_sections(v: Option<&Value>, slide_count: usize) -> Result<Vec<CreateSection>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "sections".into(),
                expected: TypeName::Array,
            },
            "presentation.sections",
        )
    })?;
    let mut out = Vec::new();
    let mut prev = 0usize;
    for (j, item) in arr.iter().enumerate() {
        let sec = request::parse_create_section(item, &format!("presentation.sections[{j}]"))?;
        if sec.start <= prev || sec.start > slide_count {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "create".into(),
                    field: "start".into(),
                },
                format!("presentation.sections[{j}].start"),
            ));
        }
        prev = sec.start;
        out.push(sec);
    }
    Ok(out)
}

fn validate_anims(slides: &[NewSlide]) -> Result<(), Error> {
    for (i, slide) in slides.iter().enumerate() {
        for (j, anim) in slide.anims.iter().enumerate() {
            let (n, len, field) = match anim.host {
                CreateAnimHost::Shape(n) => (n, slide.shapes.len(), "shape"),
                CreateAnimHost::Picture(n) => (n, slide.pictures.len(), "picture"),
                CreateAnimHost::Video(n) => (n, slide.videos.len(), "video"),
                CreateAnimHost::Audio(n) => (n, slide.audios.len(), "audio"),
                CreateAnimHost::Table(n) => (n, slide.tables.len(), "table"),
                CreateAnimHost::Chart(n) => (n, slide.charts.len(), "chart"),
                CreateAnimHost::SmartArt(n) => (n, slide.smart_arts.len(), "smartArt"),
            };
            if n < 1 || n > len {
                return Err(Error::with_path(
                    ErrorKind::OpRequiresField {
                        op: "create".into(),
                        field: field.into(),
                    },
                    format!("presentation.slides[{i}].anims[{j}].{field}"),
                ));
            }
        }
    }
    Ok(())
}

fn attach_model3ds(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.model3ds.is_empty()) {
        return Ok(());
    }
    for (i, slide) in slides.iter().enumerate() {
        if slide.model3ds.is_empty() {
            continue;
        }
        let part = format!("/ppt/slides/slide{}.xml", i + 1);
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for (j, op) in slide.model3ds.iter().enumerate() {
            super::model3d::insert(
                pkg,
                &mut doc,
                &part,
                op,
                &format!("presentation.slides[{i}].model3ds[{j}].file"),
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_maths(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.maths.is_empty()) {
        return Ok(());
    }
    for (i, slide) in slides.iter().enumerate() {
        if slide.maths.is_empty() {
            continue;
        }
        let part = format!("/ppt/slides/slide{}.xml", i + 1);
        let mut doc = xml::parse(pkg.get(&part)?)?;
        for op in &slide.maths {
            super::omml::insert(pkg, &mut doc, &part, op)?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_media(
    pkg: &mut Package,
    slides: &[NewSlide],
    kind: media::MediaKind,
) -> Result<(), Error> {
    let empty = match kind {
        media::MediaKind::Video => slides.iter().all(|s| s.videos.is_empty()),
        media::MediaKind::Audio => slides.iter().all(|s| s.audios.is_empty()),
    };
    if empty {
        return Ok(());
    }
    let pres = xml::parse(pkg.get("/ppt/presentation.xml")?)?;
    for (i, slide) in slides.iter().enumerate() {
        let ops = match kind {
            media::MediaKind::Video => &slide.videos,
            media::MediaKind::Audio => &slide.audios,
        };
        if ops.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        let field = match kind {
            media::MediaKind::Video => "videos",
            media::MediaKind::Audio => "audios",
        };
        for (j, op) in ops.iter().enumerate() {
            media::insert(
                pkg,
                &mut doc,
                &part,
                &pres.root,
                op,
                &format!("presentation.slides[{i}].{field}[{j}].file"),
                kind,
            )?;
        }
        pkg.set(&part, doc.to_bytes()?)?;
    }
    Ok(())
}

fn attach_hyperlinks(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides
        .iter()
        .all(|s| s.shapes.iter().all(|sh| sh.link.is_none()))
    {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let pres = xml::parse(pkg.get(pres_part)?)?;
    let metas = shape::list_slides(pkg, pres_part, &pres.root)?;
    for (i, slide) in slides.iter().enumerate() {
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let mut doc = xml::parse(pkg.get(&part)?)?;
        let mut dirty = false;
        for (j, sh) in slide.shapes.iter().enumerate() {
            let Some(link) = &sh.link else {
                continue;
            };
            if let PptxLinkKind::Slide(target) = link.target {
                if target == n || target < 1 || target > slides.len() {
                    return Err(Error::with_path(
                        if target == n {
                            ErrorKind::PptxHyperlinkSelfSlide
                        } else {
                            ErrorKind::OpRequiresField {
                                op: "create".into(),
                                field: "slide".into(),
                            }
                        },
                        format!("presentation.slides[{i}].shapes[{j}].slide"),
                    ));
                }
            }
            let cnv = (j as u32 + 2).to_string();
            let op = InsertHyperlinkOp {
                id: format!("slide:{n}/shape:{cnv}"),
                shape: ShapeTarget {
                    slide: n,
                    on_notes: false,
                    master: None,
                    key: ShapeKey::Id(cnv),
                },
                target: link.target.clone(),
                tooltip: link.tooltip.clone(),
                on_run: false,
                text: None,
            };
            hyperlink::apply_insert(pkg, &mut doc, &part, &metas, &op, 0)?;
            dirty = true;
        }
        if dirty {
            pkg.set(&part, doc.to_bytes()?)?;
        }
    }
    Ok(())
}

fn attach_transitions(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.transition.is_none()) {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let pres = xml::parse(pkg.get(pres_part)?)?;
    let metas = shape::list_slides(pkg, pres_part, &pres.root)?;
    for (i, slide) in slides.iter().enumerate() {
        let Some(op) = &slide.transition else {
            continue;
        };
        transition::apply(
            pkg,
            &metas,
            transition::SetArgs {
                slide: op.slide,
                type_name: op.type_name.as_deref(),
                dur: op.dur,
                adv_click: op.adv_click,
                adv_tm: op.adv_tm,
                dir: op.dir.as_deref(),
                inv_x: op.inv_x,
                inv_y: op.inv_y,
            },
            i,
        )?;
    }
    Ok(())
}

fn attach_anims(pkg: &mut Package, slides: &[NewSlide]) -> Result<(), Error> {
    if slides.iter().all(|s| s.anims.is_empty()) {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let pres = xml::parse(pkg.get(pres_part)?)?;
    let metas = shape::list_slides(pkg, pres_part, &pres.root)?;
    for (i, slide) in slides.iter().enumerate() {
        if slide.anims.is_empty() {
            continue;
        }
        let n = i + 1;
        let part = format!("/ppt/slides/slide{n}.xml");
        let doc = xml::parse(pkg.get(&part)?)?;
        for (j, anim) in slide.anims.iter().enumerate() {
            let host_id = resolve_anim_host(
                pkg,
                &doc.root,
                &part,
                n,
                anim.host,
                &format!("presentation.slides[{i}].anims[{j}]"),
            )?;
            let click_on = match anim.click_on {
                Some(host) => Some(resolve_anim_host(
                    pkg,
                    &doc.root,
                    &part,
                    n,
                    host,
                    &format!("presentation.slides[{i}].anims[{j}].clickOn"),
                )?),
                None => None,
            };
            timing::insert(
                pkg,
                &metas,
                pres_part,
                &pres.root,
                &host_id,
                &AnimTarget::Slide { slide: n },
                timing::AnimWrite {
                    type_name: &anim.type_name,
                    trigger: &anim.trigger,
                    delay: anim.delay,
                    dur: anim.dur,
                    repeat: anim.repeat,
                    sound: anim.sound.as_deref(),
                    para: anim.para,
                    click_on: click_on.as_deref(),
                },
                j,
            )?;
        }
    }
    Ok(())
}

fn resolve_anim_host(
    pkg: &Package,
    root: &crate::xml::Element,
    part: &str,
    slide: usize,
    host: CreateAnimHost,
    prefix: &str,
) -> Result<String, Error> {
    let host_s = format!("slide:{slide}");
    match host {
        CreateAnimHost::Shape(n) => {
            let list = shape::collect_text_shapes(root);
            let sp = list
                .get(n - 1)
                .ok_or_else(|| anim_host_err(prefix, "shape"))?;
            Ok(shape::shape_stable_id(sp, &host_s, n).0)
        }
        CreateAnimHost::Picture(n) => {
            let list = picture::collect_pictures(root, &host_s);
            list.get(n - 1)
                .map(|p| p.id.clone())
                .ok_or_else(|| anim_host_err(prefix, "picture"))
        }
        CreateAnimHost::Video(n) => {
            let (videos, _) = media::collect(pkg, part, root, &host_s)?;
            videos
                .get(n - 1)
                .map(|p| p.id.clone())
                .ok_or_else(|| anim_host_err(prefix, "video"))
        }
        CreateAnimHost::Audio(n) => {
            let (_, audios) = media::collect(pkg, part, root, &host_s)?;
            audios
                .get(n - 1)
                .map(|p| p.id.clone())
                .ok_or_else(|| anim_host_err(prefix, "audio"))
        }
        CreateAnimHost::Table(n) => json_id(
            &table::collect_tables(root, &format!("slide:{slide}")),
            n,
            prefix,
            "table",
        ),
        CreateAnimHost::Chart(n) => json_id(
            &chart::collect_charts(pkg, root, &host_s, part)?,
            n,
            prefix,
            "chart",
        ),
        CreateAnimHost::SmartArt(n) => json_id(
            &smartart::collect_smartarts(pkg, root, &host_s, part)?,
            n,
            prefix,
            "smartArt",
        ),
    }
}

fn json_id(list: &[Value], n: usize, prefix: &str, field: &str) -> Result<String, Error> {
    list.get(n - 1)
        .and_then(|v| v.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anim_host_err(prefix, field))
}

fn anim_host_err(prefix: &str, field: &str) -> Error {
    Error::with_path(
        ErrorKind::OpRequiresField {
            op: "create".into(),
            field: field.into(),
        },
        format!("{prefix}.{field}"),
    )
}

fn attach_sections(pkg: &mut Package, sections: &[CreateSection]) -> Result<(), Error> {
    if sections.is_empty() {
        return Ok(());
    }
    let pres_part = "/ppt/presentation.xml";
    let mut pres = xml::parse(pkg.get(pres_part)?)?;
    for (j, sec) in sections.iter().enumerate() {
        let start = if j == 0 && sec.start == 1 {
            None
        } else {
            Some(sec.start)
        };
        section::insert(
            pkg,
            pres_part,
            &mut pres,
            &InsertSlideSectionOp {
                name: sec.name.clone(),
                start,
                after: None,
            },
            j,
        )?;
    }
    Ok(())
}

fn parse_shape(v: &Value, slide: usize, i: usize) -> Result<NewShape, Error> {
    let (text, link, format) =
        request::parse_create_shape(v, &format!("presentation.slides[{slide}].shapes[{i}]"))?;
    Ok(NewShape { text, link, format })
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
    r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#
        .to_string()
}

fn slide_xml(shapes: &[NewShape]) -> String {
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

fn text_box_xml(
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

fn text_paragraphs(text: &str, format: Option<&crate::drawing_text::TextFormat>) -> String {
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

fn create_rpr(format: Option<&crate::drawing_text::TextFormat>) -> String {
    let mut sz = Some(1800i64);
    let mut color = Some("000000".to_string());
    let mut bold = None;
    let mut italic = None;
    let mut underline = None;
    let mut latin = Some("Calibri".to_string());
    let mut ea = Some("宋体".to_string());
    if let Some(f) = format {
        if let Some(v) = &f.sz {
            sz = *v;
        }
        if let Some(v) = &f.color {
            color = v.clone();
        }
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
    if let Some(hex) = color {
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
