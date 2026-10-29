//! 整篇页面背景与节上页面边框。
//! 不算排版引擎。渐变写 DrawingML；图片填写 VML（Word 16.0 另存如此）。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::opc::{self, ContentTypes, Package, Relationship, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::hf;
use super::notes;
use super::ole::{OFFICE_NS, VML_NS};
use super::picture;
use super::textbox::WPS_NS;
use super::{A_NS, CT_SETTINGS, REL_NS, REL_SETTINGS, WML_NS, WP_NS};

const BORDER_VALS: [&str; 193] = [
    "nil",
    "none",
    "single",
    "thick",
    "double",
    "dotted",
    "dashed",
    "dotDash",
    "dotDotDash",
    "triple",
    "thinThickSmallGap",
    "thickThinSmallGap",
    "thinThickThinSmallGap",
    "thinThickMediumGap",
    "thickThinMediumGap",
    "thinThickThinMediumGap",
    "thinThickLargeGap",
    "thickThinLargeGap",
    "thinThickThinLargeGap",
    "wave",
    "doubleWave",
    "dashSmallGap",
    "dashDotStroked",
    "threeDEmboss",
    "threeDEngrave",
    "outset",
    "inset",
    "apples",
    "archedScallops",
    "babyPacifier",
    "babyRattle",
    "balloons3Colors",
    "balloonsHotAir",
    "basicBlackDashes",
    "basicBlackDots",
    "basicBlackSquares",
    "basicThinLines",
    "basicWhiteDashes",
    "basicWhiteDots",
    "basicWhiteSquares",
    "basicWideInline",
    "basicWideMidline",
    "basicWideOutline",
    "bats",
    "birds",
    "birdsFlight",
    "cabins",
    "cakeSlice",
    "candyCorn",
    "celticKnotwork",
    "certificateBanner",
    "chainLink",
    "champagneBottle",
    "checkedBarBlack",
    "checkedBarColor",
    "checkered",
    "christmasTree",
    "circlesLines",
    "circlesRectangles",
    "classicalWave",
    "clocks",
    "compass",
    "confetti",
    "confettiGrays",
    "confettiOutline",
    "confettiStreamers",
    "confettiWhite",
    "cornerTriangles",
    "couponCutoutDashes",
    "couponCutoutDots",
    "crazyMaze",
    "creaturesButterfly",
    "creaturesFish",
    "creaturesInsects",
    "creaturesLadyBug",
    "crossStitch",
    "cup",
    "decoArch",
    "decoArchColor",
    "decoBlocks",
    "diamondsGray",
    "doubleD",
    "doubleDiamonds",
    "earth1",
    "earth2",
    "earth3",
    "eclipsingSquares1",
    "eclipsingSquares2",
    "eggsBlack",
    "fans",
    "film",
    "firecrackers",
    "flowersBlockPrint",
    "flowersDaisies",
    "flowersModern1",
    "flowersModern2",
    "flowersPansy",
    "flowersRedRose",
    "flowersRoses",
    "flowersTeacup",
    "flowersTiny",
    "gems",
    "gingerbreadMan",
    "gradient",
    "handmade1",
    "handmade2",
    "heartBalloon",
    "heartGray",
    "hearts",
    "heebieJeebies",
    "holly",
    "houseFunky",
    "hypnotic",
    "iceCreamCones",
    "lightBulb",
    "lightning1",
    "lightning2",
    "mapPins",
    "mapleLeaf",
    "mapleMuffins",
    "marquee",
    "marqueeToothed",
    "moons",
    "mosaic",
    "musicNotes",
    "northwest",
    "ovals",
    "packages",
    "palmsBlack",
    "palmsColor",
    "paperClips",
    "papyrus",
    "partyFavor",
    "partyGlass",
    "pencils",
    "people",
    "peopleWaving",
    "peopleHats",
    "poinsettias",
    "postageStamp",
    "pumpkin1",
    "pushPinNote2",
    "pushPinNote1",
    "pyramids",
    "pyramidsAbove",
    "quadrants",
    "rings",
    "safari",
    "sawtooth",
    "sawtoothGray",
    "scaredCat",
    "seattle",
    "shadowedSquares",
    "sharksTeeth",
    "shorebirdTracks",
    "skyrocket",
    "snowflakeFancy",
    "snowflakes",
    "sombrero",
    "southwest",
    "stars",
    "starsTop",
    "stars3d",
    "starsBlack",
    "starsShadowed",
    "sun",
    "swirligig",
    "tornPaper",
    "tornPaperBlack",
    "trees",
    "triangleParty",
    "triangles",
    "triangle1",
    "triangle2",
    "triangleCircle1",
    "triangleCircle2",
    "shapes1",
    "shapes2",
    "twistedLines1",
    "twistedLines2",
    "vine",
    "waveline",
    "weavingAngles",
    "weavingBraid",
    "weavingRibbon",
    "weavingStrips",
    "whiteFlowers",
    "woodwork",
    "xIllusions",
    "zanyTriangles",
    "zigZag",
    "zigZagStitch",
    "custom",
];

const PAGE_CX: i64 = 7_560_310;
const PAGE_CY: i64 = 10_692_130;

#[derive(Debug, Clone)]
pub struct BorderSide {
    pub val: String,
    pub sz: Option<u32>,
    pub space: Option<u32>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PageFill {
    Gradient { color2: String },
    Picture { file: String },
}

pub fn background_json(pkg: &Package, doc_part: &str, root: &Element) -> Result<Value, Error> {
    let bg = root.child_by_local("background");
    let color = bg
        .and_then(|e| e.attr_local("color"))
        .map_or(Value::Null, |c| json!(c));
    Ok(json!({
        "color": color,
        "display": read_display(pkg, doc_part)?,
        "fill": fill_json(bg),
    }))
}

pub fn borders_json(sect: &Element) -> Value {
    let Some(el) = sect.child_by_local("pgBorders") else {
        return Value::Null;
    };
    json!({
        "offsetFrom": attr_or_null(el, "offsetFrom"),
        "display": attr_or_null(el, "display"),
        "zOrder": attr_or_null(el, "zOrder"),
        "top": side_json(el, "top"),
        "left": side_json(el, "left"),
        "bottom": side_json(el, "bottom"),
        "right": side_json(el, "right"),
    })
}

pub fn set_background(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    color: Option<&str>,
    fill: Option<&PageFill>,
) -> Result<(), Error> {
    let old_rids = {
        let doc = docs.get_mut(doc_part).expect("apply 已载入主文档");
        match color {
            None => {
                let rids = background_image_rids(doc.root.child_by_local("background"));
                hf::remove_local(&mut doc.root, "background");
                rids
            }
            Some(c) => {
                let rids = background_image_rids(doc.root.child_by_local("background"));
                {
                    let bg = ensure_background(&mut doc.root);
                    strip_fill(bg);
                    hf::set_local_attr(bg, "color", c);
                }
                if let Some(fill) = fill {
                    write_fill(pkg, doc_part, &mut doc.root, fill)?;
                }
                rids
            }
        }
    };
    dirty.insert(doc_part.to_string(), ());
    drop_old_media(pkg, doc_part, &old_rids)?;
    set_display(pkg, docs, dirty, doc_part, color.is_some())?;
    Ok(())
}

pub fn set_borders(
    sect: &mut Element,
    enabled: Option<bool>,
    offset_from: Option<&str>,
    display: Option<&str>,
    z_order: Option<&str>,
    top: Option<Option<&BorderSide>>,
    left: Option<Option<&BorderSide>>,
    bottom: Option<Option<&BorderSide>>,
    right: Option<Option<&BorderSide>>,
) {
    if enabled == Some(false) {
        hf::remove_local(sect, "pgBorders");
        return;
    }
    let el = hf::ensure_child_in_order(sect, "pgBorders", hf::TITLE_PG_ORDER);
    if let Some(v) = offset_from {
        hf::set_local_attr(el, "offsetFrom", v);
    }
    if let Some(v) = display {
        hf::set_local_attr(el, "display", v);
    }
    if let Some(v) = z_order {
        hf::set_local_attr(el, "zOrder", v);
    }
    write_side(el, "top", top);
    write_side(el, "left", left);
    write_side(el, "bottom", bottom);
    write_side(el, "right", right);
}

pub fn parse_fill(v: &Value, path: &str) -> Result<PageFill, Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "fill".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "kind" | "color" | "file") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let kind = match obj.get("kind") {
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPageBackground".into(),
                    field: "kind".into(),
                },
                format!("{path}.kind"),
            ));
        }
    };
    match kind {
        "gradient" => {
            let color2 = match obj.get("color") {
                None => "FFFFFF".into(),
                Some(Value::String(s)) => parse_color(s).ok_or_else(|| {
                    Error::with_path(
                        ErrorKind::InvalidPageBackgroundColor,
                        format!("{path}.color"),
                    )
                })?,
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "color".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.color"),
                    ));
                }
            };
            Ok(PageFill::Gradient { color2 })
        }
        "picture" => {
            let file = match obj.get("file") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                Some(Value::String(_)) | None => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBackgroundFill,
                        format!("{path}.file"),
                    ));
                }
                Some(_) => {
                    return Err(Error::with_path(
                        ErrorKind::WrongType {
                            field: "file".into(),
                            expected: TypeName::String,
                        },
                        format!("{path}.file"),
                    ));
                }
            };
            Ok(PageFill::Picture { file })
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidPageBackgroundFill,
            format!("{path}.kind"),
        )),
    }
}

pub fn apply_create(
    pkg: &mut Package,
    bg: Option<&Value>,
    borders: Option<&Value>,
) -> Result<(), Error> {
    let doc_part = opc::office_document_part(pkg)?;
    let mut docs = HashMap::new();
    let parsed = xml::parse(pkg.get(&doc_part)?)?;
    docs.insert(doc_part.clone(), parsed);
    let mut dirty = HashMap::new();
    if let Some(v) = bg {
        let (color, fill) = parse_create_background(v)?;
        set_background(
            pkg,
            &mut docs,
            &mut dirty,
            &doc_part,
            color.as_deref(),
            fill.as_ref(),
        )?;
    }
    if let Some(v) = borders {
        apply_create_borders(&mut docs, &mut dirty, &doc_part, v)?;
    }
    for name in dirty.keys() {
        let doc = docs.get(name).expect("脏部件已载入");
        pkg.set(name, doc.to_bytes()?)?;
    }
    Ok(())
}

fn parse_create_background(v: &Value) -> Result<(Option<String>, Option<PageFill>), Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pageBackground".into(),
                expected: TypeName::Object,
            },
            "document.pageBackground",
        )
    })?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "color" | "fill") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("document.pageBackground.{k}"),
            ));
        }
    }
    let color = match obj.get("color") {
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: "setPageBackground".into(),
                    field: "color".into(),
                },
                "document.pageBackground.color",
            ));
        }
        Some(Value::Null) => None,
        Some(Value::String(s)) => Some(parse_color(s).ok_or_else(|| {
            Error::with_path(
                ErrorKind::InvalidPageBackgroundColor,
                "document.pageBackground.color",
            )
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                "document.pageBackground.color",
            ));
        }
    };
    let fill = match obj.get("fill") {
        None => None,
        Some(v) => Some(parse_fill(v, "document.pageBackground.fill")?),
    };
    if color.is_none() && fill.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPageBackgroundFill,
            "document.pageBackground.fill",
        ));
    }
    Ok((color, fill))
}

fn apply_create_borders(
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    v: &Value,
) -> Result<(), Error> {
    let obj = v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "pageBorders".into(),
                expected: TypeName::Object,
            },
            "document.pageBorders",
        )
    })?;
    for k in obj.keys() {
        if !matches!(
            k.as_str(),
            "offsetFrom" | "display" | "zOrder" | "top" | "left" | "bottom" | "right"
        ) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("document.pageBorders.{k}"),
            ));
        }
    }
    let offset_from = parse_create_enum(
        obj.get("offsetFrom"),
        "document.pageBorders.offsetFrom",
        ErrorKind::InvalidPageBorderOffset,
        &["page", "text"],
    )?;
    let display = parse_create_enum(
        obj.get("display"),
        "document.pageBorders.display",
        ErrorKind::InvalidPageBorderDisplay,
        &["allPages", "firstPage", "notFirstPage"],
    )?;
    let z_order = parse_create_enum(
        obj.get("zOrder"),
        "document.pageBorders.zOrder",
        ErrorKind::InvalidPageBorderZOrder,
        &["front", "back"],
    )?;
    let top = parse_create_side(obj.get("top"), "document.pageBorders.top")?;
    let left = parse_create_side(obj.get("left"), "document.pageBorders.left")?;
    let bottom = parse_create_side(obj.get("bottom"), "document.pageBorders.bottom")?;
    let right = parse_create_side(obj.get("right"), "document.pageBorders.right")?;
    if offset_from.is_none()
        && display.is_none()
        && z_order.is_none()
        && top.is_none()
        && left.is_none()
        && bottom.is_none()
        && right.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedPageBordersField,
            "document.pageBorders",
        ));
    }
    let doc = docs.get_mut(doc_part).expect("已载入主文档");
    let n = hf::collect_sect_prs(&doc.root).len();
    if n == 0 {
        return Ok(());
    }
    let sect = hf::find_sect_pr_mut(&mut doc.root, n).expect("末节");
    set_borders(
        sect,
        None,
        offset_from.as_deref(),
        display.as_deref(),
        z_order.as_deref(),
        top.as_ref().map(|s| s.as_ref()),
        left.as_ref().map(|s| s.as_ref()),
        bottom.as_ref().map(|s| s.as_ref()),
        right.as_ref().map(|s| s.as_ref()),
    );
    dirty.insert(doc_part.to_string(), ());
    Ok(())
}

fn parse_create_enum(
    v: Option<&Value>,
    path: &str,
    kind: ErrorKind,
    allowed: &[&str],
) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if allowed.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(kind, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "enum".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn parse_create_side(v: Option<&Value>, path: &str) -> Result<Option<Option<BorderSide>>, Error> {
    parse_opt_side(v, path, "setPageBorders")
}

pub fn parse_opt_side(
    v: Option<&Value>,
    path: &str,
    op: &str,
) -> Result<Option<Option<BorderSide>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(map)) => Ok(Some(Some(parse_border_side(map, path, op)?))),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "side".into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

pub fn parse_border_side(
    map: &Map<String, Value>,
    path: &str,
    op: &str,
) -> Result<BorderSide, Error> {
    for k in map.keys() {
        if !matches!(k.as_str(), "val" | "sz" | "space" | "color") {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    let val = match map.get("val") {
        Some(Value::String(s)) => parse_border_val(s)
            .ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidPageBorderVal, format!("{path}.val"))
            })?
            .to_string(),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "val".into(),
                    expected: TypeName::String,
                },
                format!("{path}.val"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
                    field: "val".into(),
                },
                format!("{path}.val"),
            ));
        }
    };
    let sz = parse_create_u32(map.get("sz"), &format!("{path}.sz"))?;
    let space = parse_create_u32(map.get("space"), &format!("{path}.space"))?;
    let color = match map.get("color") {
        None => None,
        Some(Value::String(s)) => Some(parse_side_color(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPageBorderColor, format!("{path}.color"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "color".into(),
                    expected: TypeName::String,
                },
                format!("{path}.color"),
            ));
        }
    };
    Ok(BorderSide {
        val,
        sz,
        space,
        color,
    })
}

fn parse_create_u32(v: Option<&Value>, path: &str) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                if u > u32::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: "n".into(),
                            min: 0,
                        },
                        path.to_string(),
                    ));
                }
                return Ok(Some(u as u32));
            }
            Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "n".into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            ))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "n".into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn fill_json(bg: Option<&Element>) -> Value {
    let Some(bg) = bg else {
        return Value::Null;
    };
    if bg.element_children().any(|e| e.local_name() == "drawing") {
        return json!({ "kind": "drawing" });
    }
    let Some(vb) = bg
        .element_children()
        .find(|e| e.local_name() == "background")
    else {
        return Value::Null;
    };
    let fill = vb.child_by_local("fill");
    let ty = fill.and_then(|e| e.attr_local("type")).unwrap_or("");
    let rid = fill.and_then(|e| e.attr_local("id"));
    let kind = if ty == "gradient" {
        "gradient"
    } else if ty == "frame" || ty == "tile" || rid.is_some() {
        "picture"
    } else {
        "gradient"
    };
    json!({ "kind": kind })
}

fn write_fill(
    pkg: &mut Package,
    doc_part: &str,
    root: &mut Element,
    fill: &PageFill,
) -> Result<(), Error> {
    match fill {
        PageFill::Gradient { color2 } => {
            ensure_drawing_ns(root);
            let color1 = root
                .child_by_local("background")
                .and_then(|e| e.attr_local("color"))
                .unwrap_or("FFFFFF")
                .to_string();
            let drawing = gradient_drawing(&color1, color2)?;
            let bg = root
                .child_by_local_mut("background")
                .expect("已有 background");
            bg.push_element(drawing);
        }
        PageFill::Picture { file } => {
            ensure_vml_ns(root);
            let media = picture::add_media(pkg, doc_part, file, None, None)?;
            let bg = root
                .child_by_local_mut("background")
                .expect("已有 background");
            let mut fill_el = Element::new("v:fill");
            fill_el.set_attr("r:id", &media.blip_rid);
            fill_el.set_attr("type", "frame");
            let mut vb = Element::new("v:background");
            vb.set_attr("id", "_x0000_s1025");
            vb.push_element(fill_el);
            bg.push_element(vb);
        }
    }
    Ok(())
}

fn gradient_drawing(color1: &str, color2: &str) -> Result<Element, Error> {
    let xml = format!(
        r#"<w:drawing xmlns:w="{WML_NS}" xmlns:wp="{WP_NS}" xmlns:a="{A_NS}" xmlns:wps="{WPS_NS}"><wp:anchor behindDoc="1" distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="page"><wp:posOffset>0</wp:posOffset></wp:positionH><wp:positionV relativeFrom="page"><wp:posOffset>0</wp:posOffset></wp:positionV><wp:extent cx="{PAGE_CX}" cy="{PAGE_CY}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:wrapNone/><wp:docPr id="1" name="PageBackground"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp><wps:cNvSpPr/><wps:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{PAGE_CX}" cy="{PAGE_CY}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="{c1}"/></a:gs><a:gs pos="100000"><a:srgbClr val="{c2}"/></a:gs></a:gsLst><a:lin ang="0" scaled="0"/></a:gradFill><a:ln><a:noFill/></a:ln></wps:spPr><wps:bodyPr/></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing>"#,
        c1 = xml::escape_attr(color1),
        c2 = xml::escape_attr(color2),
    );
    Ok(xml::parse(xml.as_bytes())?.take_root())
}

fn strip_fill(bg: &mut Element) {
    bg.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "drawing" | "background"),
        _ => true,
    });
}

fn background_image_rids(bg: Option<&Element>) -> Vec<String> {
    let Some(bg) = bg else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for child in bg.element_children() {
        if child.local_name() != "background" {
            continue;
        }
        if let Some(fill) = child.child_by_local("fill") {
            if let Some(id) = fill.attr_local("id") {
                out.push(id.to_string());
            }
        }
    }
    out
}

fn drop_old_media(pkg: &mut Package, doc_part: &str, rids: &[String]) -> Result<(), Error> {
    if rids.is_empty() {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, doc_part)?;
    let mut media_parts = Vec::new();
    for rid in rids {
        if let Ok(part) = opc::resolve_id(pkg, doc_part, rid) {
            media_parts.push(part);
        }
        rels.remove_by_id(rid);
    }
    rels.write_to(pkg, doc_part)?;
    for part in media_parts {
        if !picture::media_still_used(pkg, &part)? {
            pkg.remove(&part);
        }
    }
    Ok(())
}

fn ensure_vml_ns(root: &mut Element) {
    ensure_xmlns(root, "v", VML_NS);
    ensure_xmlns(root, "o", OFFICE_NS);
    ensure_xmlns(root, "r", REL_NS);
}

fn ensure_drawing_ns(root: &mut Element) {
    ensure_xmlns(root, "wp", WP_NS);
    ensure_xmlns(root, "a", A_NS);
    ensure_xmlns(root, "wps", WPS_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn attr_or_null(el: &Element, local: &str) -> Value {
    el.attr_local(local).map_or(Value::Null, |s| json!(s))
}

pub(crate) fn side_json(parent: &Element, local: &str) -> Value {
    let Some(el) = parent.child_by_local(local) else {
        return Value::Null;
    };
    json!({
        "val": el.attr_local("val").unwrap_or(""),
        "sz": parse_u32_attr(el, "sz"),
        "space": parse_u32_attr(el, "space"),
        "color": el.attr_local("color").map_or(Value::Null, |c| json!(c)),
    })
}

fn parse_u32_attr(el: &Element, local: &str) -> Value {
    el.attr_local(local)
        .and_then(|s| s.parse::<u32>().ok())
        .map_or(Value::Null, |n| json!(n))
}

fn write_side(parent: &mut Element, local: &str, side: Option<Option<&BorderSide>>) {
    let Some(slot) = side else {
        return;
    };
    match slot {
        None => {
            parent.children.retain(|n| match n {
                Node::Element(e) => e.local_name() != local,
                _ => true,
            });
        }
        Some(b) => {
            let has = parent.child_by_local(local).is_some();
            if !has {
                let el = Element::new(parent.prefixed(local));
                parent.push_element(el);
            }
            let el = parent
                .element_children_mut()
                .find(|e| e.local_name() == local)
                .expect("边框子元素");
            hf::set_local_attr(el, "val", &b.val);
            if let Some(sz) = b.sz {
                hf::set_local_attr(el, "sz", sz.to_string());
            }
            if let Some(space) = b.space {
                hf::set_local_attr(el, "space", space.to_string());
            }
            if let Some(color) = &b.color {
                hf::set_local_attr(el, "color", color);
            }
        }
    }
}

fn ensure_background(root: &mut Element) -> &mut Element {
    if root.child_by_local("background").is_some() {
        return root.child_by_local_mut("background").unwrap();
    }
    let el = Element::new(root.prefixed("background"));
    let dest = root
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "body"))
        .unwrap_or(root.children.len());
    root.children.insert(dest, Node::Element(el));
    root.child_by_local_mut("background").unwrap()
}

fn read_display(pkg: &Package, doc_part: &str) -> Result<bool, Error> {
    let Some(settings) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok(false);
    };
    let Ok(bytes) = pkg.get(&settings) else {
        return Ok(false);
    };
    let doc = xml::parse(bytes)?;
    Ok(hf::bool_prop(&doc.root, "displayBackgroundShape"))
}

fn set_display(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    on: bool,
) -> Result<(), Error> {
    let settings = match notes::settings_part_name(pkg, doc_part)? {
        Some(p) => p,
        None if on => ensure_settings(pkg, docs, dirty, doc_part)?,
        None => return Ok(()),
    };
    if !docs.contains_key(&settings) {
        let parsed = xml::parse(pkg.get(&settings)?)?;
        docs.insert(settings.clone(), parsed);
    }
    let doc = docs.get_mut(&settings).expect("已载入 settings");
    hf::set_on_off(
        &mut doc.root,
        "displayBackgroundShape",
        on,
        hf::SETTINGS_ORDER,
    );
    dirty.insert(settings, ());
    Ok(())
}

pub(crate) fn ensure_settings(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
) -> Result<String, Error> {
    let part = "/word/settings.xml".to_string();
    if !pkg.contains(&part) {
        pkg.set(
            &part,
            format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{WML_NS}"><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
            )
            .into_bytes(),
        )?;
    }
    if !docs.contains_key(&part) {
        let parsed = xml::parse(pkg.get(&part)?)?;
        docs.insert(part.clone(), parsed);
    }
    dirty.insert(part.clone(), ());
    let mut doc_rels = Relationships::from_package(pkg, doc_part)?;
    if doc_rels.find_by_type_suffix("/settings").is_empty() {
        doc_rels.push(Relationship {
            id: doc_rels.next_rid(),
            rel_type: REL_SETTINGS.into(),
            target: picture::rel_target_from(doc_part, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        doc_rels.write_to(pkg, doc_part)?;
    }
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&part, CT_SETTINGS)?;
    cts.write_to(pkg)?;
    Ok(part)
}

pub fn parse_color(raw: &str) -> Option<String> {
    if raw.len() != 6 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(raw.to_ascii_uppercase())
}

pub fn parse_border_val(raw: &str) -> Option<&str> {
    BORDER_VALS.iter().copied().find(|&v| v == raw)
}

pub fn parse_side_color(raw: &str) -> Option<String> {
    if raw.eq_ignore_ascii_case("auto") {
        return Some("auto".into());
    }
    parse_color(raw)
}
