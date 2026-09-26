//! 正文与页眉页脚里的 SmartArt：名单、改字、增删点、插删、配色与版式。不实现版式算法。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, IdReason, TypeName};
use crate::opc::{
    rels_part_name, resolve_rel_target, ContentTypes, Package, Relationship, Relationships,
};
use crate::xml::{self, Document, Element, Node};

use super::hf::{HfKind, HfPart, HfType};
use super::para::{self, BodyPara, ParaTarget};
use super::picture::{self, PicKind, WrapKind, WrapPos};
use super::{A_NS, REL_NS};

pub const DGM_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";

pub const REL_DM: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData";
pub const REL_LO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramLayout";
pub const REL_CS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramColors";
pub const REL_QS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramQuickStyle";

pub const CT_DATA: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml";
pub const CT_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml";
pub const CT_COLORS: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml";
pub const CT_STYLE: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramQuickStyle+xml";

const DEFAULT_CX: i64 = 5_486_400;
const DEFAULT_CY: i64 = 3_200_400;

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";
const UID_ORG: &str = "urn:microsoft.com/office/officeart/2005/8/layout/orgChart1";

const LAYOUT_LIST: &str = include_str!("../../templates/smartart/layout-list.xml");
const LAYOUT_ORG: &str = include_str!("../../templates/smartart/layout-org.xml");

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    List,
    Org,
}

impl LayoutKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "list" => Some(Self::List),
            "org" => Some(Self::Org),
            _ => None,
        }
    }

    fn unique_id(self) -> &'static str {
        match self {
            Self::List => UID_LIST,
            Self::Org => UID_ORG,
        }
    }

    fn layout_xml(self) -> &'static str {
        match self {
            Self::List => LAYOUT_LIST,
            Self::Org => LAYOUT_ORG,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Node,
    Asst,
}

impl Role {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "node" => Some(Self::Node),
            "asst" => Some(Self::Asst),
            _ => None,
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorsKind {
    Accent1,
    Accent2,
    Accent3,
    Accent4,
    Accent5,
    Accent6,
}

impl ColorsKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "accent1" => Some(Self::Accent1),
            "accent2" => Some(Self::Accent2),
            "accent3" => Some(Self::Accent3),
            "accent4" => Some(Self::Accent4),
            "accent5" => Some(Self::Accent5),
            "accent6" => Some(Self::Accent6),
            _ => None,
        }
    }

    fn n(self) -> u8 {
        match self {
            Self::Accent1 => 1,
            Self::Accent2 => 2,
            Self::Accent3 => 3,
            Self::Accent4 => 4,
            Self::Accent5 => 5,
            Self::Accent6 => 6,
        }
    }

    fn unique_id(self) -> String {
        format!(
            "urn:microsoft.com/office/officeart/2005/8/colors/accent{}_2",
            self.n()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Simple,
    Subtle,
    Intense,
}

impl StyleKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "simple" => Some(Self::Simple),
            "subtle" => Some(Self::Subtle),
            "intense" => Some(Self::Intense),
            _ => None,
        }
    }

    fn unique_id(self) -> &'static str {
        match self {
            Self::Simple => "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1",
            Self::Subtle => "urn:microsoft.com/office/officeart/2005/8/quickstyle/subtle1",
            Self::Intense => "urn:microsoft.com/office/officeart/2005/8/quickstyle/intense1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramKey {
    DocPr(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum DiagramTarget {
    Body {
        para: BodyPara,
        key: DiagramKey,
    },
    HeaderFooter {
        kind: HfKind,
        hf_type: HfType,
        n: u32,
        para: BodyPara,
        key: DiagramKey,
    },
}


#[derive(Debug, Clone)]
pub struct PointTarget {
    pub diagram: DiagramTarget,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub enum AddParent {
    Diagram(DiagramTarget),
    Point(PointTarget),
}

#[derive(Debug, Clone)]
pub struct NewPoint {
    pub text: String,
    pub parent: Option<usize>,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct InsertSpec {
    pub layout: LayoutKind,
    pub points: Option<Vec<NewPoint>>,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
    pub cx: i64,
    pub cy: i64,
}

#[derive(Debug, Clone)]
pub struct InsertSmartArtOp {
    pub id: String,
    pub para: ParaTarget,
    pub spec: InsertSpec,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtOp {
    pub id: String,
    pub target: DiagramTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtTextOp {
    pub id: String,
    pub target: PointTarget,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AddSmartArtPointOp {
    pub id: String,
    pub parent: AddParent,
    pub text: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct DeleteSmartArtPointOp {
    pub id: String,
    pub target: PointTarget,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtSizeOp {
    pub id: String,
    pub target: DiagramTarget,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtWrapOp {
    pub id: String,
    pub target: DiagramTarget,
    pub kind: PicKind,
    pub wrap: Option<WrapKind>,
    pub pos: WrapPos,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtColorsOp {
    pub id: String,
    pub target: DiagramTarget,
    pub colors: ColorsKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtStyleOp {
    pub id: String,
    pub target: DiagramTarget,
    pub style: StyleKind,
}

#[derive(Debug, Clone)]
pub struct SetSmartArtLayoutOp {
    pub id: String,
    pub target: DiagramTarget,
    pub layout: LayoutKind,
}

pub fn count_smartarts(root: &Element) -> Result<usize, Error> {
    let mut n = 0usize;
    picture::walk_drawings(root, &mut |d| {
        if is_smartart(d) {
            n += 1;
        }
        Ok(())
    })?;
    Ok(n)
}

pub fn collect_body(
    pkg: &Package,
    doc_part: &str,
    paras: &[&Element],
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id(p, i + 1);
        let mut index = 0usize;
        out.extend(collect_in_para(
            pkg, doc_part, p, &id, &mut index,
        )?);
    }
    Ok(out)
}

pub fn collect_hf(pkg: &Package, item: &HfPart, paras: &[&Element]) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    let mut index = 0usize;
    for (i, p) in paras.iter().enumerate() {
        let (id, _) = para::para_stable_id_in(&item.id, p, i + 1);
        out.extend(collect_in_para(pkg, &item.part, p, &id, &mut index)?);
    }
    Ok(out)
}

fn collect_in_para(
    pkg: &Package,
    host_part: &str,
    para: &Element,
    para_id: &str,
    index: &mut usize,
) -> Result<Vec<Value>, Error> {
    let mut out = Vec::new();
    picture::walk_drawings(para, &mut |drawing| {
        if !is_smartart(drawing) {
            return Ok(());
        }
        *index += 1;
        out.push(smartart_json(pkg, host_part, drawing, para_id, *index)?);
        Ok(())
    })?;
    Ok(out)
}

pub fn parse_diagram_id(id: &str, op_index: usize) -> Result<DiagramTarget, Error> {
    parse_diagram_id_at(id, &format!("ops[{op_index}].id"))
}

pub fn parse_diagram_id_at(id: &str, path: &str) -> Result<DiagramTarget, Error> {
    if id.contains("/pt:") {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    }
    let Some((left, key_s)) = id.rsplit_once("/smartdiagram:") else {
        if looks_like_para_or_container(id) {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtContainer,
                },
                path.to_string(),
            ));
        }
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    };
    if left.contains("/txbx:")
        || left.starts_with("footnote:")
        || left.starts_with("endnote:")
        || left.starts_with("comment:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    }
    let key = parse_diagram_key(key_s, path)?;
    match para::parse_para_id_at(left, path)? {
        ParaTarget::Body(para) => Ok(DiagramTarget::Body { para, key }),
        ParaTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
        } => Ok(DiagramTarget::HeaderFooter {
            kind,
            hf_type,
            n,
            para,
            key,
        }),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        )),
    }
}

pub fn parse_point_id(id: &str, op_index: usize) -> Result<PointTarget, Error> {
    let path = format!("ops[{op_index}].id");
    let Some((left, model_id)) = id.rsplit_once("/pt:") else {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    };
    if model_id.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPtPrefix,
            },
            path,
        ));
    }
    let diagram = parse_diagram_id_at(left, &path)?;
    Ok(PointTarget {
        diagram,
        model_id: model_id.to_string(),
    })
}

pub fn parse_add_parent(id: &str, op_index: usize) -> Result<AddParent, Error> {
    if id.contains("/pt:") {
        Ok(AddParent::Point(parse_point_id(id, op_index)?))
    } else {
        Ok(AddParent::Diagram(parse_diagram_id(id, op_index)?))
    }
}

pub fn parse_insert_para_id(id: &str, op_index: usize) -> Result<ParaTarget, Error> {
    let path = format!("ops[{op_index}].id");
    if id.contains("/smartdiagram:")
        || id.contains("/pt:")
        || id.contains("/chart:")
        || id.contains("/series:")
        || id.contains("/pic:")
        || id.starts_with("section:")
    {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSmartArtNeedsPara,
            },
            path,
        ));
    }
    if let Some((_, after)) = id.split_once("/txbx:") {
        if !after.contains("/para:") {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::InsertSmartArtNeedsPara,
                },
                path,
            ));
        }
    }
    let target = para::parse_para_id_at(id, &path)?;
    match target {
        ParaTarget::Body(_) | ParaTarget::HeaderFooter { .. } => Ok(target),
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSmartArtNeedsPara,
            },
            path,
        )),
    }
}

pub fn parse_spec(obj: &Map<String, Value>, path: &str) -> Result<InsertSpec, Error> {
    let layout = match obj.get("layout") {
        Some(Value::String(s)) => LayoutKind::parse(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidSmartArtLayout, format!("{path}.layout"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "layout".into(),
                    expected: TypeName::String,
                },
                format!("{path}.layout"),
            ));
        }
        None => {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "layout".into(),
                },
                format!("{path}.layout"),
            ));
        }
    };
    let points = parse_points(obj.get("points"), &format!("{path}.points"), layout)?;
    let kind = match obj.get("kind") {
        None => PicKind::Anchor,
        Some(Value::String(s)) => picture::parse_pic_kind(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicKind, format!("{path}.kind"))
        })?,
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "kind".into(),
                    expected: TypeName::String,
                },
                format!("{path}.kind"),
            ));
        }
    };
    let wrap = match obj.get("wrap") {
        None | Some(Value::Null) => {
            if kind == PicKind::Anchor {
                Some(WrapKind::Square)
            } else {
                None
            }
        }
        Some(Value::String(s)) => Some(picture::parse_wrap(s).ok_or_else(|| {
            Error::with_path(ErrorKind::InvalidPicWrap, format!("{path}.wrap"))
        })?),
        Some(_) => {
            return Err(Error::with_path(
                ErrorKind::WrongType {
                    field: "wrap".into(),
                    expected: TypeName::String,
                },
                format!("{path}.wrap"),
            ));
        }
    };
    if kind == PicKind::Inline && wrap.is_some() {
        return Err(Error::with_path(
            ErrorKind::InvalidPicWrap,
            format!("{path}.wrap"),
        ));
    }
    let cx = parse_emu(obj.get("cx"), path, "cx")?;
    let cy = parse_emu(obj.get("cy"), path, "cy")?;
    if cx.is_some() != cy.is_some() {
        return Err(Error::with_path(
            ErrorKind::OpRequiresField {
                op: "insertSmartArt".into(),
                field: "cx and cy together".into(),
            },
            path.to_string(),
        ));
    }
    Ok(InsertSpec {
        layout,
        points,
        kind,
        wrap,
        pos: WrapPos::default(),
        cx: cx.unwrap_or(DEFAULT_CX),
        cy: cy.unwrap_or(DEFAULT_CY),
    })
}

pub fn parse_role_field(v: Option<&Value>, path: &str) -> Result<Role, Error> {
    match v {
        None | Some(Value::Null) => Ok(Role::Node),
        Some(Value::String(s)) => Role::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidSmartArtRole, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "role".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

pub fn parse_colors_field(v: Option<&Value>, path: &str) -> Result<ColorsKind, Error> {
    match v {
        Some(Value::String(s)) => ColorsKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidSmartArtColors, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "colors".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "colors".into(),
            },
            path.to_string(),
        )),
    }
}

pub fn parse_style_field(v: Option<&Value>, path: &str) -> Result<StyleKind, Error> {
    match v {
        Some(Value::String(s)) => StyleKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidSmartArtStyle, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "style".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "style".into(),
            },
            path.to_string(),
        )),
    }
}

pub fn parse_layout_field(v: Option<&Value>, path: &str) -> Result<LayoutKind, Error> {
    match v {
        Some(Value::String(s)) => LayoutKind::parse(s)
            .ok_or_else(|| Error::with_path(ErrorKind::InvalidSmartArtLayout, path.to_string())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "layout".into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: "layout".into(),
            },
            path.to_string(),
        )),
    }
}

fn parse_points(
    v: Option<&Value>,
    path: &str,
    layout: LayoutKind,
) -> Result<Option<Vec<NewPoint>>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "points".into(),
                expected: TypeName::Array,
            },
            path.to_string(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{j}]");
        let obj = item.as_object().ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: item_path.clone(),
                    expected: TypeName::Object,
                },
                item_path.clone(),
            )
        })?;
        for k in obj.keys() {
            if !matches!(k.as_str(), "text" | "parent" | "role") {
                return Err(Error::with_path(
                    ErrorKind::UnknownField { field: k.clone() },
                    format!("{item_path}.{k}"),
                ));
            }
        }
        let text = match obj.get("text") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "text".into(),
                        expected: TypeName::String,
                    },
                    format!("{item_path}.text"),
                ));
            }
        };
        let parent = match obj.get("parent") {
            None | Some(Value::Null) => None,
            Some(Value::Number(n)) => {
                let Some(i) = n.as_u64() else {
                    return Err(Error::with_path(
                        ErrorKind::SmartArtPointParent,
                        format!("{item_path}.parent"),
                    ));
                };
                Some(i as usize)
            }
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "parent".into(),
                        expected: TypeName::Integer,
                    },
                    format!("{item_path}.parent"),
                ));
            }
        };
        let role = parse_role_field(obj.get("role"), &format!("{item_path}.role"))?;
        out.push(NewPoint { text, parent, role });
    }
    validate_points(layout, &out, path)?;
    Ok(Some(out))
}

fn validate_points(layout: LayoutKind, points: &[NewPoint], path: &str) -> Result<(), Error> {
    for (i, p) in points.iter().enumerate() {
        if layout == LayoutKind::List && p.role == Role::Asst {
            return Err(Error::with_path(
                ErrorKind::SmartArtAsstOnList,
                format!("{path}[{i}].role"),
            ));
        }
        if let Some(par) = p.parent {
            if par >= points.len() || par == i {
                return Err(Error::with_path(
                    ErrorKind::SmartArtPointParent,
                    format!("{path}[{i}].parent"),
                ));
            }
            if layout == LayoutKind::List {
                return Err(Error::with_path(
                    ErrorKind::SmartArtListDepth,
                    format!("{path}[{i}].parent"),
                ));
            }
        }
    }
    if has_cycle(points) {
        return Err(Error::with_path(
            ErrorKind::SmartArtPointParent,
            path.to_string(),
        ));
    }
    Ok(())
}

fn has_cycle(points: &[NewPoint]) -> bool {
    for i in 0..points.len() {
        let mut seen = HashSet::new();
        let mut cur = Some(i);
        while let Some(n) = cur {
            if !seen.insert(n) {
                return true;
            }
            cur = points.get(n).and_then(|p| p.parent);
        }
    }
    false
}

fn parse_emu(v: Option<&Value>, path: &str, field: &str) -> Result<Option<i64>, Error> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => {
            let Some(i) = n.as_i64().filter(|x| *x > 0) else {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: 1,
                    },
                    format!("{path}.{field}"),
                ));
            };
            Ok(Some(i))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            format!("{path}.{field}"),
        )),
    }
}

pub fn apply_insert(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &InsertSmartArtOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_para(catalog, doc_part, &op.para, &op.id, i)?;
    {
        let doc = load_part(pkg, docs, &host)?;
        if find_para_mut(&mut doc.root, &op.para).is_none() {
            return Err(Error::with_path(
                ErrorKind::ParaNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let rids = write_four_parts(pkg, &host, &op.spec)?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_dgm_ns(&mut doc.root);
    let doc_pr = picture::next_docpr_id(&doc.root);
    let p = find_para_mut(&mut doc.root, &op.para).ok_or_else(|| {
        Error::with_path(
            ErrorKind::ParaNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    let mut drawing = build_drawing(doc_pr, &rids, op.spec.cx, op.spec.cy);
    picture::apply_wrap(&mut drawing, op.spec.kind, op.spec.wrap, &op.spec.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    picture::insert_drawing_into_para(p, drawing);
    dirty.insert(host, ());
    Ok(())
}

pub fn attach_to_para(
    pkg: &mut Package,
    host: &str,
    root: &mut Element,
    para_index: usize,
    spec: &InsertSpec,
) -> Result<(), Error> {
    let rids = write_four_parts(pkg, host, spec)?;
    ensure_dgm_ns(root);
    let doc_pr = picture::next_docpr_id(root);
    let para = nth_story_para_mut(root, para_index).ok_or_else(|| {
        Error::xml_detail(
            crate::error::XmlIssue::Parse,
            format!("{host} missing paragraph {para_index}"),
        )
    })?;
    let mut drawing = build_drawing(doc_pr, &rids, spec.cx, spec.cy);
    picture::apply_wrap(&mut drawing, spec.kind, spec.wrap, &spec.pos)?;
    picture::insert_drawing_into_para(para, drawing);
    Ok(())
}

pub fn apply_delete(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteSmartArtOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let (rids, parts, blips) = {
        let doc = load_part(pkg, docs, &host)?;
        let drawing = find_drawing(&doc.root, &op.target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            )
        })?;
        let rids = rel_ids(drawing);
        let mut parts = Vec::new();
        for rid in rids.iter().flatten() {
            if let Ok(p) = crate::opc::resolve_id(pkg, &host, rid) {
                parts.push(p);
            }
        }
        if let Some(dm) = rids.dm.as_ref() {
            if let Ok(data) = crate::opc::resolve_id(pkg, &host, dm) {
                parts.extend(collect_drawing_caches(pkg, &host, &data)?);
            }
        }
        let blips = collect_blip_rids(drawing);
        (rids, parts, blips)
    };
    let mut media = Vec::new();
    for rid in &blips {
        if let Ok(m) = picture::resolve_embed(pkg, &host, rid) {
            media.push(m);
        }
    }
    {
        let doc = load_part(pkg, docs, &host)?;
        if !remove_smartart(&mut doc.root, &op.target) {
            return Err(Error::with_path(
                ErrorKind::SmartArtNotFound { id: op.id.clone() },
                format!("ops[{i}].id"),
            ));
        }
    }
    dirty.insert(host.clone(), ());
    let root = &docs.get(&host).unwrap().root;
    for rid in rids.iter().flatten() {
        remove_rel_if_unused(pkg, &host, rid, root)?;
    }
    for rid in &blips {
        picture::remove_rel_if_unused(pkg, &host, rid, root)?;
    }
    for p in &parts {
        drop_unused(pkg, p)?;
    }
    // data 部件卸掉之后，原先只被它的 .rels 指着的 drawing 缓存才能卸。
    for p in &parts {
        drop_unused(pkg, p)?;
    }
    for m in media {
        if !picture::media_still_used(pkg, &m)? {
            pkg.remove(&m);
        }
    }
    Ok(())
}

pub fn apply_text(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtTextOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target.diagram,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let data_part = data_part_of(pkg, docs, &host, &op.target.diagram, &op.id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    let pt = find_visible_pt_mut(&mut data.root, &op.target.model_id).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    set_point_text(pt, &op.text);
    pkg.set(&data_part, data.to_bytes()?)?;
    if !sync_drawing_text(pkg, &host, &data_part, &op.target.model_id, &op.text)? {
        drop_drawing_caches(pkg, &host, &data_part)?;
    }
    Ok(())
}

pub fn apply_add_point(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &AddSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let (diagram, parent_model, id) = match &op.parent {
        AddParent::Diagram(t) => (t.clone(), None, op.id.clone()),
        AddParent::Point(p) => (p.diagram.clone(), Some(p.model_id.clone()), op.id.clone()),
    };
    let host = part_for_target(catalog, doc_part, &diagram, &id, &format!("ops[{i}].id"))?;
    let layout_uid = layout_unique_id(pkg, docs, &host, &diagram, &id, i).ok();
    if family_of(layout_uid.as_deref().unwrap_or("")) == Some(LayoutKind::List) {
        if op.role == Role::Asst {
            return Err(Error::with_path(
                ErrorKind::SmartArtAsstOnList,
                format!("ops[{i}].role"),
            ));
        }
        if parent_model.is_some() {
            return Err(Error::with_path(
                ErrorKind::SmartArtListDepth,
                format!("ops[{i}].id"),
            ));
        }
    }
    let data_part = data_part_of(pkg, docs, &host, &diagram, &id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    let parent_id = match &parent_model {
        Some(mid) => {
            if find_visible_pt(&data.root, mid).is_none() {
                return Err(Error::with_path(
                    ErrorKind::SmartArtPointNotFound { id: id.clone() },
                    format!("ops[{i}].id"),
                ));
            }
            mid.clone()
        }
        None => ensure_doc_pt(&mut data.root),
    };
    let (pt_id, cxn_id) = next_model_ids(&data.root);
    append_visible_pt(&mut data.root, &pt_id, op.role, &op.text);
    append_par_of(&mut data.root, &cxn_id, &parent_id, &pt_id);
    pkg.set(&data_part, data.to_bytes()?)?;
    drop_drawing_caches(pkg, &host, &data_part)?;
    Ok(())
}

pub fn apply_delete_point(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &DeleteSmartArtPointOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(
        catalog,
        doc_part,
        &op.target.diagram,
        &op.id,
        &format!("ops[{i}].id"),
    )?;
    let data_part = data_part_of(pkg, docs, &host, &op.target.diagram, &op.id, i)?;
    let mut data = xml::parse(pkg.get(&data_part)?)?;
    if find_visible_pt(&data.root, &op.target.model_id).is_none() {
        return Err(Error::with_path(
            ErrorKind::SmartArtPointNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    let visible = visible_model_ids(&data.root);
    let doomed = descendants_of(&data.root, &op.target.model_id);
    if doomed.iter().filter(|id| visible.contains(*id)).count() >= visible.len() {
        return Err(Error::with_path(
            ErrorKind::LastSmartArtPoint,
            format!("ops[{i}].id"),
        ));
    }
    delete_points_and_cxns(&mut data.root, &doomed);
    pkg.set(&data_part, data.to_bytes()?)?;
    drop_drawing_caches(pkg, &host, &data_part)?;
    Ok(())
}

pub fn apply_size(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtSizeOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let doc = load_part(pkg, docs, &host)?;
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    if !picture::set_extent(drawing, op.cx, op.cy) {
        return Err(Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        ));
    }
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_wrap(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtWrapOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let doc = load_part(pkg, docs, &host)?;
    ensure_dgm_ns(&mut doc.root);
    let drawing = find_drawing_mut(&mut doc.root, &op.target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: op.id.clone() },
            format!("ops[{i}].id"),
        )
    })?;
    picture::apply_wrap(drawing, op.kind, op.wrap, &op.pos)
        .map_err(|e| e.at(format!("ops[{i}]")))?;
    dirty.insert(host, ());
    Ok(())
}

pub fn apply_colors(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtColorsOp,
    i: usize,
) -> Result<(), Error> {
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "cs",
        REL_CS,
        CT_COLORS,
        "colors",
        &op.colors.unique_id(),
        &colors_xml(op.colors),
    )
}

pub fn apply_style(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtStyleOp,
    i: usize,
) -> Result<(), Error> {
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "qs",
        REL_QS,
        CT_STYLE,
        "quickStyle",
        op.style.unique_id(),
        &style_xml(op.style),
    )
}

pub fn apply_layout(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    op: &SetSmartArtLayoutOp,
    i: usize,
) -> Result<(), Error> {
    let host = part_for_target(catalog, doc_part, &op.target, &op.id, &format!("ops[{i}].id"))?;
    let current = layout_unique_id(pkg, docs, &host, &op.target, &op.id, i)?;
    if current == op.layout.unique_id() {
        return Ok(());
    }
    let cur_fam = family_of(&current);
    let tgt_fam = Some(op.layout);
    if cur_fam != tgt_fam {
        return Err(Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].layout"),
        ));
    }
    retarget_satellite(
        pkg,
        docs,
        dirty,
        doc_part,
        catalog,
        &op.target,
        &op.id,
        i,
        "lo",
        REL_LO,
        CT_LAYOUT,
        "layout",
        op.layout.unique_id(),
        op.layout.layout_xml(),
    )?;
    if let Ok(data) = data_part_of(pkg, docs, &host, &op.target, &op.id, i) {
        drop_drawing_caches(pkg, &host, &data)?;
    }
    Ok(())
}

fn retarget_satellite(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    catalog: &[HfPart],
    target: &DiagramTarget,
    id: &str,
    i: usize,
    attr: &str,
    rel_type: &str,
    ct: &str,
    stem: &str,
    unique_id: &str,
    xml_body: &str,
) -> Result<(), Error> {
    let host = part_for_target(catalog, doc_part, target, id, &format!("ops[{i}].id"))?;
    {
        let doc = load_part(pkg, docs, &host)?;
        if find_drawing(&doc.root, target).is_none() {
            return Err(Error::with_path(
                ErrorKind::SmartArtNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            ));
        }
    }
    let old_part = {
        let doc = load_part(pkg, docs, &host)?;
        let drawing = find_drawing(&doc.root, target).unwrap();
        rel_ids(drawing)
            .get(attr)
            .and_then(|rid| crate::opc::resolve_id(pkg, &host, rid).ok())
    };
    let part = if let Some(existing) = find_part_by_unique_id(pkg, unique_id) {
        existing
    } else {
        let part = next_numbered(pkg, "/word/diagrams", stem, ".xml");
        pkg.set(&part, xml_body.as_bytes().to_vec())?;
        let mut cts = ContentTypes::from_package(pkg)?;
        cts.set_override(&part, ct)?;
        cts.write_to(pkg)?;
        part
    };
    let mut host_rels = Relationships::from_package(pkg, &host)?;
    let rid = if let Some(existing) = host_rels.rels.iter().find_map(|r| {
        if r.is_external() {
            return None;
        }
        resolve_rel_target(&host, &r.target)
            .ok()
            .filter(|p| p == &part)
            .map(|_| r.id.clone())
    }) {
        existing
    } else {
        let rid = host_rels.next_rid();
        host_rels.push(Relationship {
            id: rid.clone(),
            rel_type: rel_type.into(),
            target: picture::rel_target_from(&host, &part),
            target_mode: None,
            extra: Vec::new(),
        });
        host_rels.write_to(pkg, &host)?;
        rid
    };
    {
        let doc = load_part(pkg, docs, &host)?;
        let drawing = find_drawing_mut(&mut doc.root, target).ok_or_else(|| {
            Error::with_path(
                ErrorKind::SmartArtNotFound { id: id.to_string() },
                format!("ops[{i}].id"),
            )
        })?;
        if let Some(rel) = find_rel_ids_mut(drawing) {
            set_rid_attr(rel, attr, &rid);
        }
        dirty.insert(host.clone(), ());
    }
    if let Some(old) = old_part {
        if old != part {
            drop_unused(pkg, &old)?;
        }
    }
    Ok(())
}

fn smartart_json(
    pkg: &Package,
    host_part: &str,
    drawing: &Element,
    para_id: &str,
    index: usize,
) -> Result<Value, Error> {
    let frame = frame(drawing);
    let kind = match frame.map(Element::local_name) {
        Some("anchor") => PicKind::Anchor,
        _ => PicKind::Inline,
    };
    let doc_pr = frame.and_then(|f| f.child_by_local("docPr"));
    let raw_id = doc_pr
        .and_then(|d| d.attr_local("id"))
        .filter(|s| !s.is_empty());
    let name = doc_pr
        .and_then(|d| d.attr_local("name"))
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let (id, stable, doc_pr_id) = match raw_id {
        Some(v) => (
            format!("{para_id}/smartdiagram:{v}"),
            true,
            Some(v.to_string()),
        ),
        None => (
            format!("{para_id}/smartdiagram:index:{index}"),
            false,
            None,
        ),
    };
    let extent = frame.and_then(|f| f.child_by_local("extent"));
    let cx = parse_emu_attr(extent, "cx")?;
    let cy = parse_emu_attr(extent, "cy")?;
    let wrap = if kind == PicKind::Anchor {
        frame.and_then(|f| {
            f.element_children()
                .find_map(|c| WrapKind::from_local(c.local_name()))
        })
    } else {
        None
    };
    let rids = rel_ids(drawing);
    let layout = rids
        .lo
        .as_ref()
        .and_then(|rid| unique_id_of(pkg, host_part, rid));
    let colors = rids
        .cs
        .as_ref()
        .and_then(|rid| unique_id_of(pkg, host_part, rid));
    let style = rids
        .qs
        .as_ref()
        .and_then(|rid| unique_id_of(pkg, host_part, rid));
    let points = match &rids.dm {
        Some(rid) => match crate::opc::resolve_id(pkg, host_part, rid) {
            Ok(part) => match pkg.get(&part).ok().and_then(|b| xml::parse(b).ok()) {
                Some(doc) => points_json(&id, &doc.root),
                None => Vec::new(),
            },
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    Ok(json!({
        "id": id,
        "docPrId": doc_pr_id,
        "name": name,
        "kind": kind.as_str(),
        "cx": cx,
        "cy": cy,
        "wrap": wrap.map(WrapKind::as_str),
        "layout": layout,
        "colors": colors,
        "style": style,
        "points": points,
        "stable": stable,
    }))
}

fn points_json(diagram_id: &str, data: &Element) -> Vec<Value> {
    let parents = parent_map(data);
    let visible = visible_pts(data);
    let vis_ids: HashSet<&str> = visible
        .iter()
        .filter_map(|p| p.attr_local("modelId"))
        .collect();
    visible
        .into_iter()
        .filter_map(|pt| {
            let model_id = pt.attr_local("modelId")?;
            let role = match pt.attr_local("type").unwrap_or("node") {
                "asst" => "asst",
                _ => "node",
            };
            let text = pt
                .child_by_local("t")
                .map(collect_dgm_text)
                .unwrap_or_default();
            let parent = parents.get(model_id).and_then(|src| {
                if vis_ids.contains(src.as_str()) {
                    Some(src.clone())
                } else {
                    None
                }
            });
            Some(json!({
                "id": format!("{diagram_id}/pt:{model_id}"),
                "modelId": model_id,
                "role": role,
                "text": text,
                "parent": parent,
            }))
        })
        .collect()
}

fn collect_dgm_text(t: &Element) -> String {
    let mut out = String::new();
    // dgm:t 的 local name 也是 t，不能当可见文本收；只看子孙里的 a:t / a:br。
    for n in &t.children {
        if let Node::Element(e) = n {
            collect_t_br(e, &mut out);
        }
    }
    out
}

fn collect_t_br(el: &Element, out: &mut String) {
    match el.local_name() {
        "t" => {
            out.push_str(&el.text_content());
            return;
        }
        "br" => out.push('\n'),
        _ => {}
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_t_br(e, out);
        }
    }
}

fn parent_map(data: &Element) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(lst) = find_local(data, "cxnLst") else {
        return map;
    };
    for cxn in lst.element_children() {
        if cxn.local_name() != "cxn" {
            continue;
        }
        let ty = cxn.attr_local("type").unwrap_or("");
        if !ty.is_empty() && ty != "parOf" {
            continue;
        }
        let Some(src) = cxn.attr_local("srcId") else {
            continue;
        };
        let Some(dest) = cxn.attr_local("destId") else {
            continue;
        };
        map.entry(dest.to_string()).or_insert_with(|| src.to_string());
    }
    map
}

fn visible_pts(data: &Element) -> Vec<&Element> {
    let Some(lst) = find_local(data, "ptLst") else {
        return Vec::new();
    };
    lst.element_children()
        .filter(|e| e.local_name() == "pt" && is_visible_pt(e))
        .collect()
}

fn is_visible_pt(pt: &Element) -> bool {
    matches!(pt.attr_local("type").unwrap_or("node"), "node" | "asst")
}

fn is_smartart(drawing: &Element) -> bool {
    graphic_uri(drawing).is_some_and(|u| norm_uri(&u) == DGM_NS)
}

fn graphic_uri(drawing: &Element) -> Option<String> {
    let mut found = None;
    walk_outside_txbx(drawing, &mut |e| {
        if e.local_name() == "graphicData" && found.is_none() {
            found = e.attr_local("uri").map(str::to_string);
        }
    });
    found
}

fn norm_uri(u: &str) -> &str {
    u.trim_end_matches('/')
}

fn looks_like_para_or_container(id: &str) -> bool {
    id.starts_with("para:")
        || id.starts_with("header:")
        || id.starts_with("footer:")
        || id.starts_with("footnote:")
        || id.starts_with("endnote:")
        || id.starts_with("comment:")
}

fn parse_diagram_key(rest: &str, path: &str) -> Result<DiagramKey, Error> {
    if let Some(n) = rest.strip_prefix("index:") {
        let idx: usize = n.parse().map_err(|_| {
            Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexNotInt,
                },
                path.to_string(),
            )
        })?;
        if idx < 1 {
            return Err(Error::with_path(
                ErrorKind::InvalidId {
                    reason: IdReason::SmartArtIndexFromOne,
                },
                path.to_string(),
            ));
        }
        return Ok(DiagramKey::Index(idx));
    }
    if rest.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::SmartArtPrefix,
            },
            path.to_string(),
        ));
    }
    Ok(DiagramKey::DocPr(rest.to_string()))
}

fn matches_key(drawing: &Element, key: &DiagramKey, index: usize) -> bool {
    match key {
        DiagramKey::Index(n) => index == *n,
        DiagramKey::DocPr(want) => {
            let Some(got) = frame(drawing)
                .and_then(|f| f.child_by_local("docPr"))
                .and_then(|d| d.attr_local("id"))
            else {
                return false;
            };
            if let (Ok(a), Ok(b)) = (want.parse::<u32>(), got.parse::<u32>()) {
                a == b
            } else {
                want == got
            }
        }
    }
}

fn find_drawing<'a>(root: &'a Element, target: &DiagramTarget) -> Option<&'a Element> {
    match target {
        DiagramTarget::Body { para, key } => {
            let p = para::find_paragraph(root, para)?;
            find_walk(p, &mut 0, key)
        }
        DiagramTarget::HeaderFooter { para, key, .. } => match key {
            DiagramKey::DocPr(_) => {
                let p = para::find_paragraph(root, para)?;
                find_walk(p, &mut 0, key)
            }
            DiagramKey::Index(n) => find_hf_index(root, para, *n),
        },
    }
}

fn find_hf_index<'a>(root: &'a Element, para: &BodyPara, n: usize) -> Option<&'a Element> {
    let local = hf_local_index(root, para, n)?;
    let p = para::find_paragraph(root, para)?;
    find_walk(p, &mut 0, &DiagramKey::Index(local))
}

fn find_drawing_mut<'a>(root: &'a mut Element, target: &DiagramTarget) -> Option<&'a mut Element> {
    match target {
        DiagramTarget::Body { para, key } => {
            let p = para::find_paragraph_mut(root, para)?;
            find_walk_mut(p, &mut 0, key)
        }
        DiagramTarget::HeaderFooter { para, key, .. } => match key {
            DiagramKey::DocPr(_) => {
                let p = para::find_paragraph_mut(root, para)?;
                find_walk_mut(p, &mut 0, key)
            }
            DiagramKey::Index(n) => find_hf_index_mut(root, para, *n),
        },
    }
}

fn find_hf_index_mut<'a>(
    root: &'a mut Element,
    para: &BodyPara,
    n: usize,
) -> Option<&'a mut Element> {
    let local = hf_local_index(root, para, n)?;
    let p = para::find_paragraph_mut(root, para)?;
    find_walk_mut(p, &mut 0, &DiagramKey::Index(local))
}

fn hf_local_index(root: &Element, para: &BodyPara, n: usize) -> Option<usize> {
    let mut idx = 0usize;
    for (i, p) in para::collect_paragraphs(root).into_iter().enumerate() {
        let is_target = para_matches(p, i + 1, para);
        let start = idx;
        let _ = picture::walk_drawings(p, &mut |d| {
            if is_smartart(d) {
                idx += 1;
            }
            Ok(())
        });
        if is_target && start < n && n <= idx {
            return Some(n - start);
        }
    }
    None
}

fn para_matches(p: &Element, index: usize, para: &BodyPara) -> bool {
    match para {
        BodyPara::Index(n) => index == *n,
        BodyPara::Hex(hex) => p
            .attr_local("paraId")
            .is_some_and(|id| id.eq_ignore_ascii_case(hex)),
    }
}

fn find_walk<'a>(el: &'a Element, idx: &mut usize, key: &DiagramKey) -> Option<&'a Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &el.children[i] {
                return find_walk(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_smartart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            if let Some(found) = find_walk(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn find_walk_mut<'a>(
    el: &'a mut Element,
    idx: &mut usize,
    key: &DiagramKey,
) -> Option<&'a mut Element> {
    if el.local_name() == "txbxContent" {
        return None;
    }
    if el.local_name() == "AlternateContent" {
        if let Some(i) = first_alt_child_index(el) {
            if let Node::Element(e) = &mut el.children[i] {
                return find_walk_mut(e, idx, key);
            }
        }
        return None;
    }
    if el.local_name() == "drawing" && is_smartart(el) {
        *idx += 1;
        if matches_key(el, key, *idx) {
            return Some(el);
        }
        return None;
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = find_walk_mut(e, idx, key) {
                return Some(found);
            }
        }
    }
    None
}

fn remove_smartart(root: &mut Element, target: &DiagramTarget) -> bool {
    match target {
        DiagramTarget::Body { para, key } => match para::find_paragraph_mut(root, para) {
            Some(p) => remove_walk(p, &mut 0, key),
            None => false,
        },
        DiagramTarget::HeaderFooter { para, key, .. } => match key {
            DiagramKey::DocPr(_) => match para::find_paragraph_mut(root, para) {
                Some(p) => remove_walk(p, &mut 0, key),
                None => false,
            },
            DiagramKey::Index(n) => match hf_local_index(root, para, *n) {
                Some(local) => match para::find_paragraph_mut(root, para) {
                    Some(p) => remove_walk(p, &mut 0, &DiagramKey::Index(local)),
                    None => false,
                },
                None => false,
            },
        },
    }
}

fn remove_walk(el: &mut Element, idx: &mut usize, key: &DiagramKey) -> bool {
    if el.local_name() == "txbxContent" {
        return false;
    }
    let mut i = 0;
    while i < el.children.len() {
        let local = match &el.children[i] {
            Node::Element(c) => c.local_name().to_string(),
            Node::Text(_) => {
                i += 1;
                continue;
            }
        };
        if local == "AlternateContent" {
            if ac_has_smartart(&el.children[i], idx, key) {
                el.children.remove(i);
                return true;
            }
            i += 1;
            continue;
        }
        if local == "drawing" {
            let is_s = matches!(&el.children[i], Node::Element(c) if is_smartart(c));
            if is_s {
                *idx += 1;
                let hit = match &el.children[i] {
                    Node::Element(c) => matches_key(c, key, *idx),
                    Node::Text(_) => false,
                };
                if hit {
                    el.children.remove(i);
                    return true;
                }
            }
            i += 1;
            continue;
        }
        if local == "txbxContent" {
            i += 1;
            continue;
        }
        if let Node::Element(child) = &mut el.children[i] {
            if remove_walk(child, idx, key) {
                if child.local_name() == "r"
                    && child.element_children().all(|c| c.local_name() == "rPr")
                {
                    el.children.remove(i);
                }
                return true;
            }
        }
        i += 1;
    }
    false
}

fn ac_has_smartart(node: &Node, idx: &mut usize, key: &DiagramKey) -> bool {
    let Node::Element(ac) = node else {
        return false;
    };
    let Some(i) = first_alt_child_index(ac) else {
        return false;
    };
    let Node::Element(branch) = &ac.children[i] else {
        return false;
    };
    let mut hit = false;
    let _ = picture::walk_drawings(branch, &mut |d| {
        if is_smartart(d) {
            *idx += 1;
            if matches_key(d, key, *idx) {
                hit = true;
            }
        }
        Ok(())
    });
    hit
}

fn first_alt_child_index(el: &Element) -> Option<usize> {
    let mut fallback = None;
    for (i, n) in el.children.iter().enumerate() {
        if let Node::Element(e) = n {
            if e.local_name() == "Choice" {
                return Some(i);
            }
            if e.local_name() == "Fallback" && fallback.is_none() {
                fallback = Some(i);
            }
        }
    }
    fallback
}

fn frame(drawing: &Element) -> Option<&Element> {
    drawing
        .element_children()
        .find(|e| e.local_name() == "inline" || e.local_name() == "anchor")
}

fn parse_emu_attr(el: Option<&Element>, local: &str) -> Result<Option<i64>, Error> {
    let Some(el) = el else {
        return Ok(None);
    };
    let Some(raw) = el.attr_local(local) else {
        return Ok(None);
    };
    raw.parse::<i64>().map(Some).map_err(|_| {
        Error::xml_detail(
            crate::error::XmlIssue::AttrRead,
            format!("wp:extent @{local}={raw}"),
        )
    })
}

fn walk_outside_txbx(el: &Element, f: &mut impl FnMut(&Element)) {
    if el.local_name() == "txbxContent" {
        return;
    }
    f(el);
    for n in &el.children {
        if let Node::Element(e) = n {
            walk_outside_txbx(e, f);
        }
    }
}

#[derive(Clone, Default)]
struct RelIds {
    dm: Option<String>,
    lo: Option<String>,
    qs: Option<String>,
    cs: Option<String>,
}

impl RelIds {
    fn get(&self, local: &str) -> Option<&String> {
        match local {
            "dm" => self.dm.as_ref(),
            "lo" => self.lo.as_ref(),
            "qs" => self.qs.as_ref(),
            "cs" => self.cs.as_ref(),
            _ => None,
        }
    }

    fn iter(&self) -> impl Iterator<Item = Option<&String>> {
        [self.dm.as_ref(), self.lo.as_ref(), self.qs.as_ref(), self.cs.as_ref()].into_iter()
    }
}

fn rel_ids(drawing: &Element) -> RelIds {
    let mut out = RelIds::default();
    walk_outside_txbx(drawing, &mut |e| {
        if e.local_name() == "relIds" {
            out.dm = e.attr_local("dm").map(str::to_string);
            out.lo = e.attr_local("lo").map(str::to_string);
            out.qs = e.attr_local("qs").map(str::to_string);
            out.cs = e.attr_local("cs").map(str::to_string);
        }
    });
    out
}

fn find_rel_ids_mut(drawing: &mut Element) -> Option<&mut Element> {
    find_local_mut(drawing, "relIds")
}

fn find_local<'a>(el: &'a Element, local: &str) -> Option<&'a Element> {
    if el.local_name() == local {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(f) = find_local(c, local) {
            return Some(f);
        }
    }
    None
}

fn find_local_mut<'a>(el: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if el.local_name() == local {
        return Some(el);
    }
    for c in el.children.as_mut_slice() {
        if let Node::Element(e) = c {
            if let Some(f) = find_local_mut(e, local) {
                return Some(f);
            }
        }
    }
    None
}

fn set_rid_attr(el: &mut Element, local: &str, rid: &str) {
    if let Some((_, v)) = el
        .attrs
        .iter_mut()
        .find(|(k, _)| k.rsplit(':').next() == Some(local))
    {
        *v = rid.to_string();
        return;
    }
    el.set_attr(&format!("r:{local}"), rid);
}

fn unique_id_of(pkg: &Package, host: &str, rid: &str) -> Option<String> {
    let part = crate::opc::resolve_id(pkg, host, rid).ok()?;
    let doc = xml::parse(pkg.get(&part).ok()?).ok()?;
    doc.root.attr_local("uniqueId").map(str::to_string)
}

fn family_of(uid: &str) -> Option<LayoutKind> {
    let tail = uid.rsplit('/').next().unwrap_or(uid).to_ascii_lowercase();
    if tail == "default" || tail.starts_with("hlist") || tail.starts_with("vlist") {
        Some(LayoutKind::List)
    } else if tail == "orgchart1" || tail.starts_with("orgchart") {
        Some(LayoutKind::Org)
    } else {
        None
    }
}

fn write_four_parts(pkg: &mut Package, host: &str, spec: &InsertSpec) -> Result<RelIds, Error> {
    let resolved = resolve_points(spec);
    let data_xml = build_data_xml(&resolved);
    let data_part = next_numbered(pkg, "/word/diagrams", "data", ".xml");
    let layout_part = next_numbered(pkg, "/word/diagrams", "layout", ".xml");
    let colors_part = next_numbered(pkg, "/word/diagrams", "colors", ".xml");
    let style_part = next_numbered(pkg, "/word/diagrams", "quickStyle", ".xml");
    pkg.set(&data_part, data_xml.into_bytes())?;
    pkg.set(&layout_part, spec.layout.layout_xml().as_bytes().to_vec())?;
    pkg.set(&colors_part, colors_xml(ColorsKind::Accent1).into_bytes())?;
    pkg.set(&style_part, style_xml(StyleKind::Simple).into_bytes())?;

    let mut host_rels = Relationships::from_package(pkg, host)?;
    let dm = push_rel(&mut host_rels, host, REL_DM, &data_part);
    let lo = push_rel(&mut host_rels, host, REL_LO, &layout_part);
    let qs = push_rel(&mut host_rels, host, REL_QS, &style_part);
    let cs = push_rel(&mut host_rels, host, REL_CS, &colors_part);
    host_rels.write_to(pkg, host)?;

    let mut cts = ContentTypes::from_package(pkg)?;
    cts.set_override(&data_part, CT_DATA)?;
    cts.set_override(&layout_part, CT_LAYOUT)?;
    cts.set_override(&colors_part, CT_COLORS)?;
    cts.set_override(&style_part, CT_STYLE)?;
    cts.write_to(pkg)?;

    Ok(RelIds {
        dm: Some(dm),
        lo: Some(lo),
        qs: Some(qs),
        cs: Some(cs),
    })
}

fn push_rel(rels: &mut Relationships, host: &str, rel_type: &str, part: &str) -> String {
    let id = rels.next_rid();
    rels.push(Relationship {
        id: id.clone(),
        rel_type: rel_type.into(),
        target: picture::rel_target_from(host, part),
        target_mode: None,
        extra: Vec::new(),
    });
    id
}

struct ResolvedPt {
    model_id: String,
    parent: Option<String>,
    role: Role,
    text: String,
}

fn resolve_points(spec: &InsertSpec) -> Vec<ResolvedPt> {
    if let Some(pts) = &spec.points {
        return pts
            .iter()
            .enumerate()
            .map(|(i, p)| ResolvedPt {
                model_id: (i + 1).to_string(),
                parent: p.parent.map(|j| (j + 1).to_string()),
                role: p.role,
                text: p.text.clone(),
            })
            .collect();
    }
    match spec.layout {
        LayoutKind::List => (1..=3)
            .map(|i| ResolvedPt {
                model_id: i.to_string(),
                parent: None,
                role: Role::Node,
                text: format!("文本 {i}"),
            })
            .collect(),
        LayoutKind::Org => vec![
            ResolvedPt {
                model_id: "1".into(),
                parent: None,
                role: Role::Node,
                text: "文本 1".into(),
            },
            ResolvedPt {
                model_id: "2".into(),
                parent: Some("1".into()),
                role: Role::Node,
                text: "文本 2".into(),
            },
            ResolvedPt {
                model_id: "3".into(),
                parent: Some("1".into()),
                role: Role::Node,
                text: "文本 3".into(),
            },
        ],
    }
}

fn build_data_xml(points: &[ResolvedPt]) -> String {
    let mut pts = String::from(r#"<dgm:pt type="doc" modelId="0"/>"#);
    for p in points {
        let ty = match p.role {
            Role::Node => "",
            Role::Asst => r#" type="asst""#,
        };
        pts.push_str(&format!(
            r#"<dgm:pt{ty} modelId="{id}"><dgm:prSet/><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN" sz="1100"/><a:t{space}>{text}</a:t></a:r></a:p></dgm:t></dgm:pt>"#,
            id = xml::escape_attr(&p.model_id),
            space = if needs_xml_space(&p.text) {
                r#" xml:space="preserve""#
            } else {
                ""
            },
            text = xml::escape_text(&p.text),
        ));
    }
    let mut cxns = String::new();
    let mut cxn_id = points.len() as u64 + 1;
    let mut src_ord: HashMap<String, u32> = HashMap::new();
    for p in points {
        let src = p.parent.clone().unwrap_or_else(|| "0".into());
        let ord = src_ord.entry(src.clone()).or_insert(0);
        cxns.push_str(&format!(
            r#"<dgm:cxn modelId="{cxn}" srcId="{src}" destId="{dest}" srcOrd="{ord}" destOrd="0"/>"#,
            cxn = cxn_id,
            src = xml::escape_attr(&src),
            dest = xml::escape_attr(&p.model_id),
            ord = *ord,
        ));
        *ord += 1;
        cxn_id += 1;
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:dataModel xmlns:dgm="{DGM_NS}" xmlns:a="{A_NS}" xmlns:r="{REL_NS}"><dgm:ptLst>{pts}</dgm:ptLst><dgm:cxnLst>{cxns}</dgm:cxnLst><dgm:bg/><dgm:whole/></dgm:dataModel>"#
    )
}

fn build_drawing(doc_pr: u32, rids: &RelIds, cx: i64, cy: i64) -> Element {
    let mut drawing = Element::new("w:drawing");
    let mut inline = Element::new("wp:inline")
        .with_attr("distT", "0")
        .with_attr("distB", "0")
        .with_attr("distL", "0")
        .with_attr("distR", "0");
    inline.push_element(
        Element::new("wp:extent")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    inline.push_element(
        Element::new("wp:effectExtent")
            .with_attr("l", "0")
            .with_attr("t", "0")
            .with_attr("r", "0")
            .with_attr("b", "0"),
    );
    inline.push_element(
        Element::new("wp:docPr")
            .with_attr("id", doc_pr.to_string())
            .with_attr("name", format!("SmartArt {doc_pr}")),
    );
    let mut frame_pr = Element::new("wp:cNvGraphicFramePr");
    frame_pr.push_element(Element::new("a:graphicFrameLocks").with_attr("noGrp", "1"));
    inline.push_element(frame_pr);
    let mut graphic = Element::new("a:graphic");
    let mut data = Element::new("a:graphicData").with_attr("uri", DGM_NS);
    data.push_element(
        Element::new("dgm:relIds")
            .with_attr("xmlns:dgm", DGM_NS)
            .with_attr("xmlns:r", REL_NS)
            .with_attr("r:dm", rids.dm.as_deref().unwrap_or(""))
            .with_attr("r:lo", rids.lo.as_deref().unwrap_or(""))
            .with_attr("r:qs", rids.qs.as_deref().unwrap_or(""))
            .with_attr("r:cs", rids.cs.as_deref().unwrap_or("")),
    );
    graphic.push_element(data);
    inline.push_element(graphic);
    drawing.push_element(inline);
    drawing
}

fn ensure_dgm_ns(root: &mut Element) {
    picture::ensure_drawing_ns(root);
    ensure_xmlns(root, "dgm", DGM_NS);
}

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}

fn colors_xml(kind: ColorsKind) -> String {
    let n = kind.n();
    let uid = kind.unique_id();
    let labels = ["node0", "node1", "alignAcc", "asst0", "lnNode1"];
    let mut body = String::new();
    for name in labels {
        body.push_str(&format!(
            r#"<dgm:styleLbl name="{name}"><dgm:fillClrLst meth="repeat"><a:schemeClr val="accent{n}"/></dgm:fillClrLst><dgm:linClrLst meth="repeat"><a:schemeClr val="lt1"/></dgm:linClrLst><dgm:effectClrLst/><dgm:txLinClrLst/><dgm:txFillClrLst meth="repeat"><a:schemeClr val="lt1"/></dgm:txFillClrLst><dgm:txEffectClrLst/></dgm:styleLbl>"#
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:colorsDef xmlns:dgm="{DGM_NS}" xmlns:a="{A_NS}" uniqueId="{uid}" minVer="12.0"><dgm:title lang="" val=""/><dgm:desc lang="" val=""/><dgm:catLst><dgm:cat type="accent{n}" pri="11200"/></dgm:catLst>{body}</dgm:colorsDef>"#
    )
}

fn style_xml(kind: StyleKind) -> String {
    let uid = kind.unique_id();
    let labels = ["node0", "node1", "alignAcc", "asst0", "lnNode1"];
    let mut body = String::new();
    for name in labels {
        body.push_str(&format!(
            r#"<dgm:styleLbl name="{name}"><dgm:scene3d/><dgm:sp3d prstMaterial="warmMatte"/><dgm:txPr/><dgm:style><dgm:lnRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></dgm:lnRef><dgm:fillRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></dgm:fillRef><dgm:effectRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></dgm:effectRef><dgm:fontRef idx="minor"><a:schemeClr val="lt1"/></dgm:fontRef></dgm:style></dgm:styleLbl>"#
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><dgm:styleDef xmlns:dgm="{DGM_NS}" xmlns:a="{A_NS}" uniqueId="{uid}" minVer="12.0"><dgm:title lang="" val=""/><dgm:desc lang="" val=""/><dgm:catLst/><dgm:scene3d/>{body}</dgm:styleDef>"#
    )
}

fn next_numbered(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

fn load_part<'a>(
    pkg: &Package,
    docs: &'a mut HashMap<String, Document>,
    part: &str,
) -> Result<&'a mut Document, Error> {
    if !docs.contains_key(part) {
        let parsed = xml::parse(pkg.get(part)?)?;
        docs.insert(part.to_string(), parsed);
    }
    Ok(docs.get_mut(part).unwrap())
}

fn part_for_para(
    catalog: &[HfPart],
    doc_part: &str,
    para: &ParaTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    match para {
        ParaTarget::Body(_) => Ok(doc_part.to_string()),
        ParaTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::ParaNotFound { id: id.to_string() },
                    format!("ops[{i}].id"),
                )
            })?;
            Ok(item.part.clone())
        }
        _ => Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::InsertSmartArtNeedsPara,
            },
            format!("ops[{i}].id"),
        )),
    }
}

fn part_for_target(
    catalog: &[HfPart],
    doc_part: &str,
    target: &DiagramTarget,
    id: &str,
    path: &str,
) -> Result<String, Error> {
    match target {
        DiagramTarget::Body { .. } => Ok(doc_part.to_string()),
        DiagramTarget::HeaderFooter {
            kind, hf_type, n, ..
        } => {
            let container_id = format!("{}:{}:{n}", kind.as_str(), hf_type.as_str());
            let item = super::hf::find_by_id(catalog, &container_id).ok_or_else(|| {
                Error::with_path(
                    ErrorKind::SmartArtNotFound { id: id.to_string() },
                    path.to_string(),
                )
            })?;
            Ok(item.part.clone())
        }
    }
}

fn find_para_mut<'a>(root: &'a mut Element, target: &ParaTarget) -> Option<&'a mut Element> {
    match target {
        ParaTarget::Body(p) | ParaTarget::HeaderFooter { para: p, .. } => {
            para::find_paragraph_mut(root, p)
        }
        _ => None,
    }
}

fn nth_story_para_mut(root: &mut Element, index: usize) -> Option<&mut Element> {
    let parent = if root.local_name() == "document" {
        root.child_by_local_mut("body")?
    } else {
        root
    };
    let mut n = 0usize;
    for child in parent.children.iter_mut() {
        let Node::Element(e) = child else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if n == index {
            return Some(e);
        }
        n += 1;
    }
    None
}

fn missing_data(id: &str, i: usize) -> Error {
    Error::with_path(
        ErrorKind::PartMissing {
            name: format!("{id} data"),
        },
        format!("ops[{i}].id"),
    )
}

fn data_part_of(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    host: &str,
    target: &DiagramTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let doc = load_part(pkg, docs, host)?;
    let drawing = find_drawing(&doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = rel_ids(drawing).dm.ok_or_else(|| missing_data(id, i))?;
    crate::opc::resolve_id(pkg, host, &rid).map_err(|_| missing_data(id, i))
}

fn layout_unique_id(
    pkg: &Package,
    docs: &mut HashMap<String, Document>,
    host: &str,
    target: &DiagramTarget,
    id: &str,
    i: usize,
) -> Result<String, Error> {
    let doc = load_part(pkg, docs, host)?;
    let drawing = find_drawing(&doc.root, target).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtNotFound { id: id.to_string() },
            format!("ops[{i}].id"),
        )
    })?;
    let rid = rel_ids(drawing).lo.ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })?;
    unique_id_of(pkg, host, &rid).ok_or_else(|| {
        Error::with_path(
            ErrorKind::SmartArtLayoutNotConvertible,
            format!("ops[{i}].id"),
        )
    })
}

fn find_visible_pt<'a>(root: &'a Element, model_id: &str) -> Option<&'a Element> {
    visible_pts(root)
        .into_iter()
        .find(|p| p.attr_local("modelId") == Some(model_id))
}

fn find_visible_pt_mut<'a>(root: &'a mut Element, model_id: &str) -> Option<&'a mut Element> {
    let lst = find_local_mut(root, "ptLst")?;
    for child in lst.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() == "pt"
                && is_visible_pt(e)
                && e.attr_local("modelId") == Some(model_id)
            {
                return Some(e);
            }
        }
    }
    None
}

fn set_point_text(pt: &mut Element, text: &str) {
    if let Some(t) = pt.child_by_local_mut("t") {
        set_tx_text(t, text);
        return;
    }
    pt.push_element(minimal_t(text));
}

fn minimal_t(text: &str) -> Element {
    let mut t = Element::new("dgm:t");
    t.push_element(Element::new("a:bodyPr"));
    t.push_element(Element::new("a:lstStyle"));
    let mut p = Element::new("a:p");
    let mut r = Element::new("a:r");
    r.push_element(Element::new("a:rPr").with_attr("lang", "zh-CN").with_attr("sz", "1100"));
    let mut at = Element::new("a:t");
    apply_t_text(&mut at, text);
    r.push_element(at);
    p.push_element(r);
    t.push_element(p);
    t
}

fn set_tx_text(tx: &mut Element, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut filled = false;
    rewrite_paras(tx, &mut filled, &lines);
    if !filled {
        append_text_para(tx, &lines);
    }
    prune_empty_paras(tx);
}

fn rewrite_paras(el: &mut Element, filled: &mut bool, lines: &[&str]) {
    if el.local_name() == "p" {
        if !*filled && el.element_children().any(is_text_run) {
            fill_para(el, lines);
            *filled = true;
        } else {
            strip_para(el);
        }
        return;
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            rewrite_paras(e, filled, lines);
        }
    }
}

fn is_text_run(e: &Element) -> bool {
    e.local_name() == "r" && e.element_children().any(|c| c.local_name() == "t")
}

fn strip_para(p: &mut Element) {
    let mut i = p.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &mut p.children[i] else {
            continue;
        };
        match e.local_name() {
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                e.children.retain(|n| match n {
                    Node::Element(c) => c.local_name() != "t",
                    Node::Text(_) => true,
                });
                if !e.element_children().any(|c| c.local_name() != "rPr") {
                    p.children.remove(i);
                }
            }
            _ => {}
        }
    }
}

fn prune_empty_paras(el: &mut Element) {
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if e.local_name() != "p" {
                prune_empty_paras(e);
            }
        }
    }
    let mut remaining = el
        .children
        .iter()
        .filter(|n| matches!(n, Node::Element(e) if e.local_name() == "p"))
        .count();
    if remaining <= 1 {
        return;
    }
    let mut i = el.children.len();
    while i > 0 {
        i -= 1;
        let Node::Element(e) = &el.children[i] else {
            continue;
        };
        if e.local_name() != "p" {
            continue;
        }
        if remaining <= 1 {
            break;
        }
        if !e
            .element_children()
            .any(|c| !matches!(c.local_name(), "pPr" | "endParaRPr"))
        {
            el.children.remove(i);
            remaining -= 1;
        }
    }
}

fn fill_para(p: &mut Element, lines: &[&str]) {
    let mut seen = false;
    let mut i = 0;
    while i < p.children.len() {
        let Node::Element(e) = &mut p.children[i] else {
            i += 1;
            continue;
        };
        match e.local_name() {
            "r" if is_text_run(e) && !seen => {
                seen = true;
                i += 1;
            }
            "br" => {
                p.children.remove(i);
            }
            "r" => {
                e.children.retain(|n| match n {
                    Node::Element(c) => c.local_name() != "t",
                    Node::Text(_) => true,
                });
                if !e.element_children().any(|c| c.local_name() != "rPr") {
                    p.children.remove(i);
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    let Some(at) = p
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if is_text_run(e)))
    else {
        return;
    };
    let (rpr, t_name) = {
        let Node::Element(run) = &mut p.children[at] else {
            return;
        };
        let t_name = match run.element_children().find(|c| c.local_name() == "t") {
            Some(t) => t.name.clone(),
            None => return,
        };
        let rpr = fill_single_line(run, lines[0]);
        (rpr, t_name)
    };
    let proto = Element::new(t_name.clone());
    let mut ins = at + 1;
    for line in &lines[1..] {
        p.children
            .insert(ins, Node::Element(Element::new(proto.prefixed("br"))));
        ins += 1;
        let mut run = Element::new(proto.prefixed("r"));
        if let Some(pr) = &rpr {
            run.push_element(pr.clone());
        }
        let mut t = Element::new(t_name.clone());
        apply_t_text(&mut t, line);
        run.push_element(t);
        p.children.insert(ins, Node::Element(run));
        ins += 1;
    }
}

fn fill_single_line(run: &mut Element, line: &str) -> Option<Element> {
    let mut seen = false;
    let mut drop = Vec::new();
    for (i, child) in run.children.iter_mut().enumerate() {
        if let Node::Element(e) = child {
            if e.local_name() == "t" {
                if seen {
                    drop.push(i);
                } else {
                    apply_t_text(e, line);
                    seen = true;
                }
            }
        }
    }
    for i in drop.into_iter().rev() {
        run.children.remove(i);
    }
    run.element_children()
        .find(|e| e.local_name() == "rPr")
        .cloned()
}

fn append_text_para(tx: &mut Element, lines: &[&str]) {
    let sample = tx
        .element_children()
        .find(|e| e.local_name() == "p")
        .or_else(|| tx.element_children().next())
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "a:p".to_string());
    let proto = Element::new(sample);
    if let Some(p) = tx.child_by_local_mut("p") {
        append_lines_to_para(p, lines);
        return;
    }
    let mut p = Element::new(proto.prefixed("p"));
    append_lines_to_para(&mut p, lines);
    tx.push_element(p);
}

fn append_lines_to_para(p: &mut Element, lines: &[&str]) {
    let proto = Element::new("a:t");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            p.push_element(Element::new(proto.prefixed("br")));
        }
        let mut run = Element::new(proto.prefixed("r"));
        let mut t = Element::new(proto.name.clone());
        apply_t_text(&mut t, line);
        run.push_element(t);
        p.push_element(run);
    }
}

fn apply_t_text(t: &mut Element, text: &str) {
    t.set_text(text.to_string());
    if needs_xml_space(text) {
        t.set_attr("xml:space", "preserve");
    } else {
        t.remove_attr("xml:space");
    }
}

fn needs_xml_space(text: &str) -> bool {
    text.starts_with(' ') || text.starts_with('\t') || text.ends_with(' ') || text.ends_with('\t')
}

fn sync_drawing_text(
    pkg: &mut Package,
    host: &str,
    data_part: &str,
    model_id: &str,
    text: &str,
) -> Result<bool, Error> {
    let caches = collect_drawing_caches(pkg, host, data_part)?;
    if caches.is_empty() {
        return Ok(true);
    }
    let mut all_ok = true;
    for part in caches {
        let Ok(bytes) = pkg.get(&part) else {
            all_ok = false;
            continue;
        };
        let Ok(mut doc) = xml::parse(bytes) else {
            all_ok = false;
            continue;
        };
        if let Some(sp) = find_by_model_id_mut(&mut doc.root, model_id) {
            if let Some(tx) = find_local_mut(sp, "txBody") {
                set_tx_text(tx, text);
                pkg.set(&part, doc.to_bytes()?)?;
            } else {
                all_ok = false;
            }
        } else {
            all_ok = false;
        }
    }
    Ok(all_ok)
}

fn find_by_model_id_mut<'a>(el: &'a mut Element, model_id: &str) -> Option<&'a mut Element> {
    if el.attr_local("modelId") == Some(model_id) {
        return Some(el);
    }
    for child in el.children.as_mut_slice() {
        if let Node::Element(e) = child {
            if let Some(found) = find_by_model_id_mut(e, model_id) {
                return Some(found);
            }
        }
    }
    None
}

fn drop_drawing_caches(pkg: &mut Package, host: &str, data_part: &str) -> Result<(), Error> {
    let parts = collect_drawing_caches(pkg, host, data_part)?;
    for part in parts {
        drop_unused(pkg, &part)?;
        strip_rels_to(pkg, data_part, &part)?;
        strip_rels_to(pkg, host, &part)?;
    }
    Ok(())
}

fn collect_drawing_caches(
    pkg: &Package,
    host: &str,
    data_part: &str,
) -> Result<Vec<String>, Error> {
    let mut parts = drawing_cache_parts(pkg, data_part)?;
    parts.extend(host_drawing_cache_parts(pkg, host)?);
    if let Some(sib) = sibling_drawing_part(data_part) {
        if pkg.contains(&sib) {
            parts.push(sib);
        }
    }
    parts.sort();
    parts.dedup();
    Ok(parts)
}

/// Word 另存时常把 drawing 缓存写成与 data 同号的部件，却不写 diagramDrawing 关系。
fn sibling_drawing_part(data_part: &str) -> Option<String> {
    let (dir, file) = data_part.rsplit_once('/')?;
    let rest = file.strip_prefix("data")?;
    Some(format!("{dir}/drawing{rest}"))
}

fn drawing_cache_parts(pkg: &Package, data_part: &str) -> Result<Vec<String>, Error> {
    parts_by_rel_type(pkg, data_part, "diagramDrawing")
}

fn host_drawing_cache_parts(pkg: &Package, host: &str) -> Result<Vec<String>, Error> {
    parts_by_rel_type(pkg, host, "diagramDrawing")
}

fn parts_by_rel_type(pkg: &Package, source: &str, needle: &str) -> Result<Vec<String>, Error> {
    let rels = Relationships::from_package(pkg, source)?;
    let mut out = Vec::new();
    for r in &rels.rels {
        if r.is_external() || !r.rel_type.contains(needle) {
            continue;
        }
        if let Ok(p) = resolve_rel_target(source, &r.target) {
            out.push(p);
        }
    }
    Ok(out)
}

fn strip_rels_to(pkg: &mut Package, source: &str, target: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, source)?;
    let before = rels.rels.len();
    rels.remove_by_target_part(source, target);
    if rels.rels.len() != before {
        if rels.rels.is_empty() {
            pkg.remove(&rels_part_name(source));
        } else {
            rels.write_to(pkg, source)?;
        }
    }
    Ok(())
}

fn collect_blip_rids(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_blip_rids_walk(el, &mut out);
    out
}

fn collect_blip_rids_walk(el: &Element, out: &mut Vec<String>) {
    if el.local_name() == "blip" {
        if let Some(id) = el.attr_local("embed") {
            out.push(id.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_blip_rids_walk(e, out);
        }
    }
}

fn drop_unused(pkg: &mut Package, part: &str) -> Result<(), Error> {
    if part_still_used(pkg, part)? {
        return Ok(());
    }
    let rels = rels_part_name(part);
    pkg.remove(part);
    pkg.remove(&rels);
    let mut cts = ContentTypes::from_package(pkg)?;
    cts.remove_override(part);
    cts.write_to(pkg)?;
    Ok(())
}

fn part_still_used(pkg: &Package, target: &str) -> Result<bool, Error> {
    for name in pkg.part_names() {
        if !name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_from_rels(&name) else {
            continue;
        };
        let rels = Relationships::parse(pkg.get(&name)?)?;
        for r in &rels.rels {
            if r.is_external() {
                continue;
            }
            if let Ok(p) = resolve_rel_target(&source, &r.target) {
                if p == target {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn source_from_rels(rels_name: &str) -> Option<String> {
    let name = rels_name.strip_prefix('/')?;
    let (parent, file) = name.rsplit_once('/')?;
    let stem = file.strip_suffix(".rels")?;
    if parent.is_empty() || parent == "_rels" {
        Some(format!("/{stem}"))
    } else {
        let dir = parent.strip_suffix("/_rels")?;
        Some(format!("/{dir}/{stem}"))
    }
}

fn remove_rel_if_unused(
    pkg: &mut Package,
    part: &str,
    rid: &str,
    root: &Element,
) -> Result<(), Error> {
    let mut found = false;
    walk_outside_txbx(root, &mut |e| {
        if e.local_name() == "relIds" {
            if e.attr_local("dm") == Some(rid)
                || e.attr_local("lo") == Some(rid)
                || e.attr_local("qs") == Some(rid)
                || e.attr_local("cs") == Some(rid)
            {
                found = true;
            }
        }
    });
    if found {
        return Ok(());
    }
    let mut rels = Relationships::from_package(pkg, part)?;
    rels.remove_by_id(rid);
    if rels.rels.is_empty() {
        pkg.remove(&rels_part_name(part));
    } else {
        rels.write_to(pkg, part)?;
    }
    Ok(())
}

fn ensure_doc_pt(root: &mut Element) -> String {
    if let Some(lst) = find_local(root, "ptLst") {
        for pt in lst.element_children() {
            if pt.local_name() == "pt" && pt.attr_local("type") == Some("doc") {
                if let Some(id) = pt.attr_local("modelId") {
                    return id.to_string();
                }
            }
        }
    }
    let id = unused_model_id(root);
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.children.insert(
            0,
            Node::Element(Element::new("dgm:pt").with_attr("type", "doc").with_attr("modelId", &id)),
        );
    }
    id
}

fn all_model_ids(root: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_model_ids(root, &mut out);
    out
}

fn collect_model_ids(el: &Element, out: &mut Vec<String>) {
    if matches!(el.local_name(), "pt" | "cxn") {
        if let Some(id) = el.attr_local("modelId") {
            out.push(id.to_string());
        }
    }
    for n in &el.children {
        if let Node::Element(e) = n {
            collect_model_ids(e, out);
        }
    }
}

fn unused_model_id(root: &Element) -> String {
    next_model_ids(root).0
}

fn next_model_ids(root: &Element) -> (String, String) {
    let ids = all_model_ids(root);
    if ids.iter().all(|s| s.parse::<u64>().is_ok()) {
        let used: HashSet<u64> = ids.iter().filter_map(|s| s.parse().ok()).collect();
        let mut a = 1u64;
        while used.contains(&a) {
            a += 1;
        }
        let mut b = a + 1;
        while used.contains(&b) {
            b += 1;
        }
        (a.to_string(), b.to_string())
    } else {
        (new_guid(), new_guid())
    }
}

fn new_guid() -> String {
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let a = (t.as_secs() as u32).wrapping_mul(0x9E37_79B9) ^ (n as u32);
    let b = t.subsec_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-A{:03X}-{:012X}}}",
        a,
        (b >> 16) as u16,
        ((b as u16) & 0x0fff),
        ((n as u16) & 0x0fff),
        (n as u64).wrapping_shl(16) | (b as u64 & 0xffff)
    )
}

fn append_visible_pt(root: &mut Element, model_id: &str, role: Role, text: &str) {
    let mut pt = Element::new("dgm:pt").with_attr("modelId", model_id);
    if role == Role::Asst {
        pt.set_attr("type", "asst");
    }
    pt.push_element(Element::new("dgm:prSet"));
    pt.push_element(minimal_t(text));
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.push_element(pt);
    }
}

fn append_par_of(root: &mut Element, cxn_id: &str, src: &str, dest: &str) {
    let src_ord = count_children(root, src);
    let cxn = Element::new("dgm:cxn")
        .with_attr("modelId", cxn_id)
        .with_attr("srcId", src)
        .with_attr("destId", dest)
        .with_attr("srcOrd", src_ord.to_string())
        .with_attr("destOrd", "0");
    if let Some(lst) = find_local_mut(root, "cxnLst") {
        lst.push_element(cxn);
        return;
    }
    if let Some(model) = find_local_mut(root, "dataModel") {
        let mut lst = Element::new("dgm:cxnLst");
        lst.push_element(cxn);
        model.push_element(lst);
    }
}

fn count_children(root: &Element, src: &str) -> u32 {
    let Some(lst) = find_local(root, "cxnLst") else {
        return 0;
    };
    lst.element_children()
        .filter(|c| {
            c.local_name() == "cxn"
                && c.attr_local("srcId") == Some(src)
                && matches!(c.attr_local("type").unwrap_or("parOf"), "" | "parOf")
        })
        .count() as u32
}

fn visible_model_ids(root: &Element) -> HashSet<String> {
    visible_pts(root)
        .into_iter()
        .filter_map(|p| p.attr_local("modelId").map(str::to_string))
        .collect()
}

fn descendants_of(root: &Element, model_id: &str) -> HashSet<String> {
    let mut kids: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(lst) = find_local(root, "cxnLst") {
        for cxn in lst.element_children() {
            if cxn.local_name() != "cxn" {
                continue;
            }
            let ty = cxn.attr_local("type").unwrap_or("");
            if !ty.is_empty() && ty != "parOf" {
                continue;
            }
            if let (Some(src), Some(dest)) = (cxn.attr_local("srcId"), cxn.attr_local("destId")) {
                kids.entry(src.to_string())
                    .or_default()
                    .push(dest.to_string());
            }
        }
    }
    let mut out = HashSet::new();
    let mut stack = vec![model_id.to_string()];
    while let Some(id) = stack.pop() {
        if !out.insert(id.clone()) {
            continue;
        }
        if let Some(ch) = kids.get(&id) {
            stack.extend(ch.iter().cloned());
        }
    }
    out
}

fn delete_points_and_cxns(root: &mut Element, doomed: &HashSet<String>) {
    let mut doomed_cxn = HashSet::new();
    if let Some(lst) = find_local(root, "cxnLst") {
        for cxn in lst.element_children() {
            if cxn.local_name() != "cxn" {
                continue;
            }
            let src = cxn.attr_local("srcId").unwrap_or("");
            let dest = cxn.attr_local("destId").unwrap_or("");
            if doomed.contains(src) || doomed.contains(dest) {
                if let Some(id) = cxn.attr_local("modelId") {
                    doomed_cxn.insert(id.to_string());
                }
            }
        }
    }
    let mut trans = HashSet::new();
    if let Some(lst) = find_local(root, "ptLst") {
        for pt in lst.element_children() {
            if pt.local_name() != "pt" {
                continue;
            }
            let ty = pt.attr_local("type").unwrap_or("");
            if ty != "parTrans" && ty != "sibTrans" {
                continue;
            }
            if let Some(cxn_id) = pt.attr_local("cxnId") {
                if doomed_cxn.contains(cxn_id) {
                    if let Some(id) = pt.attr_local("modelId") {
                        trans.insert(id.to_string());
                    }
                }
            }
        }
    }
    if let Some(lst) = find_local_mut(root, "ptLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "pt" => {
                let Some(id) = e.attr_local("modelId") else {
                    return true;
                };
                if e.attr_local("type") == Some("doc") {
                    return true;
                }
                !doomed.contains(id) && !trans.contains(id)
            }
            _ => true,
        });
    }
    if let Some(lst) = find_local_mut(root, "cxnLst") {
        lst.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "cxn" => {
                let src = e.attr_local("srcId").unwrap_or("");
                let dest = e.attr_local("destId").unwrap_or("");
                let id = e.attr_local("modelId").unwrap_or("");
                !doomed.contains(src)
                    && !doomed.contains(dest)
                    && !trans.contains(src)
                    && !trans.contains(dest)
                    && !doomed_cxn.contains(id)
            }
            _ => true,
        });
    }
}

fn find_part_by_unique_id(pkg: &Package, unique_id: &str) -> Option<String> {
    for name in pkg.part_names() {
        if !name.starts_with("/word/diagrams/") {
            continue;
        }
        let Ok(bytes) = pkg.get(&name) else {
            continue;
        };
        let Ok(doc) = xml::parse(bytes) else {
            continue;
        };
        if doc.root.attr_local("uniqueId") == Some(unique_id) {
            return Some(name);
        }
    }
    None
}
