//! 图示数据模型与四件套部件。不含 Word 绕排、段落 id，也不含幻灯片 graphicFrame。

mod data;
mod parts;

use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind, TypeName};

pub use data::{
    append_par_of, append_visible_pt, delete_points_and_cxns, demote_asst, descendants_of,
    ensure_doc_pt, find_visible_pt, find_visible_pt_mut, next_model_ids, points_json,
    set_point_text, sync_drawing_text, unique_id_of, visible_model_ids, RelIds,
};
pub use parts::{
    collect_drawing_caches, drop_drawing_caches, drop_replaced_satellite, drop_unused, family_of,
    find_local, find_local_mut, rel_ids, retarget_satellite, write_four_parts,
};

pub const DGM_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
pub const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub const REL_DM: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData";
pub const REL_LO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramLayout";
pub const REL_CS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramColors";
pub const REL_QS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramQuickStyle";

pub const CT_DATA: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml";
pub const CT_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml";
pub const CT_COLORS: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml";
pub const CT_STYLE: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramQuickStyle+xml";

pub const DEFAULT_CX: i64 = 5_486_400;
pub const DEFAULT_CY: i64 = 3_200_400;

const UID_LIST: &str = "urn:microsoft.com/office/officeart/2005/8/layout/default";
const UID_ORG: &str = "urn:microsoft.com/office/officeart/2005/8/layout/orgChart1";
const UID_CYCLE: &str = "urn:microsoft.com/office/officeart/2005/8/layout/cycle2";

const LAYOUT_LIST: &str = include_str!("../../templates/smartart/layout-list.xml");
const LAYOUT_ORG: &str = include_str!("../../templates/smartart/layout-org.xml");
const LAYOUT_CYCLE: &str = include_str!("../../templates/smartart/layout-cycle.xml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    List,
    Org,
    Cycle,
}

impl LayoutKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "list" => Some(Self::List),
            "org" => Some(Self::Org),
            "cycle" => Some(Self::Cycle),
            _ => None,
        }
    }

    pub fn unique_id(self) -> &'static str {
        match self {
            Self::List => UID_LIST,
            Self::Org => UID_ORG,
            Self::Cycle => UID_CYCLE,
        }
    }

    pub fn layout_xml(self) -> &'static str {
        match self {
            Self::List => LAYOUT_LIST,
            Self::Org => LAYOUT_ORG,
            Self::Cycle => LAYOUT_CYCLE,
        }
    }

    /// 列表与循环都是一层子点，不要 asst，也不要更深的 parent。
    pub fn is_flat(self) -> bool {
        matches!(self, Self::List | Self::Cycle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Node,
    Asst,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
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
    pub fn parse(s: &str) -> Option<Self> {
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

    pub fn n(self) -> u8 {
        match self {
            Self::Accent1 => 1,
            Self::Accent2 => 2,
            Self::Accent3 => 3,
            Self::Accent4 => 4,
            Self::Accent5 => 5,
            Self::Accent6 => 6,
        }
    }

    pub fn unique_id(self) -> String {
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
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "simple" => Some(Self::Simple),
            "subtle" => Some(Self::Subtle),
            "intense" => Some(Self::Intense),
            _ => None,
        }
    }

    pub fn unique_id(self) -> &'static str {
        match self {
            Self::Simple => "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1",
            Self::Subtle => "urn:microsoft.com/office/officeart/2005/8/quickstyle/subtle1",
            Self::Intense => "urn:microsoft.com/office/officeart/2005/8/quickstyle/intense1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPoint {
    pub text: String,
    pub parent: Option<usize>,
    pub role: Role,
}

/// `a:graphicData/@uri` 是否为 diagram 命名空间（允许尾斜杠）。
pub fn is_diagram_uri(uri: &str) -> bool {
    uri.trim_end_matches('/') == DGM_NS
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

pub fn parse_points(
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

pub fn validate_points(layout: LayoutKind, points: &[NewPoint], path: &str) -> Result<(), Error> {
    for (i, p) in points.iter().enumerate() {
        if layout.is_flat() && p.role == Role::Asst {
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
            if layout.is_flat() {
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

pub fn parse_emu(v: Option<&Value>, path: &str, field: &str) -> Result<Option<i64>, Error> {
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

/// 给 Word 的 `parse_spec` 用：只认 layout / points，不管绕排。
pub fn parse_layout_and_points(
    obj: &Map<String, Value>,
    path: &str,
) -> Result<(LayoutKind, Option<Vec<NewPoint>>), Error> {
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
    Ok((layout, points))
}

pub fn colors_xml(kind: ColorsKind) -> String {
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

pub fn style_xml(kind: StyleKind) -> String {
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
