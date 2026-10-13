//! DrawingML theme 部件：`clrScheme` / `fontScheme` / `fmtScheme` 的 inspect 与写入。
//! 不算主题排版引擎，不把 `phClr` 展开成 RGB。
//! `inspect` 只报 `name` / `colors` / `fonts`；母版 / 工作簿 / 主文档用 `inspect_full`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::xml::{self, Element, Node};

pub const DML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

pub const COLOR_SLOTS: &[&str] = &[
    "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSlot {
    MajorLatin,
    MinorLatin,
    MajorEa,
    MinorEa,
    MajorCs,
    MinorCs,
}

impl FontSlot {
    fn group(self) -> &'static str {
        match self {
            Self::MajorLatin | Self::MajorEa | Self::MajorCs => "majorFont",
            Self::MinorLatin | Self::MinorEa | Self::MinorCs => "minorFont",
        }
    }

    fn face(self) -> &'static str {
        match self {
            Self::MajorLatin | Self::MinorLatin => "latin",
            Self::MajorEa | Self::MinorEa => "ea",
            Self::MajorCs | Self::MinorCs => "cs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtFillKind {
    Solid,
    Grad,
}

#[derive(Debug, Clone)]
pub struct ScriptFont {
    pub script: String,
    pub typeface: String,
}

#[derive(Debug, Clone)]
pub struct ExtraScheme {
    pub name: Option<String>,
    pub slots: Vec<(String, String)>,
}

const THEME_ROOT_ORDER: &[&str] = &[
    "themeElements",
    "objectDefaults",
    "extraClrSchemeLst",
    "custClrLst",
    "extLst",
];

pub fn inspect(root: &Element) -> Value {
    let name = root
        .attr_local("name")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    json!({
        "name": name,
        "colors": color_scheme_json(root),
        "fonts": font_scheme_json(root),
    })
}

/// 工作簿 / 主文档 / 幻灯片母版 theme：在 `inspect` 之上补 `fmt`、`extraSchemes` 与脚本字体。
pub fn inspect_full(root: &Element) -> Value {
    let mut v = inspect(root);
    if let Value::Object(m) = &mut v {
        m.insert("fmt".into(), fmt_from_theme(root));
        m.insert("extraSchemes".into(), extra_schemes_json(root));
        if let Some(Value::Object(fonts)) = m.get_mut("fonts") {
            let scheme = root
                .child_by_local("themeElements")
                .and_then(|e| e.child_by_local("fontScheme"));
            attach_script_fonts(fonts, scheme);
        }
    }
    v
}

/// `a:themeOverride` 全套字段，含脚本字体与补充色板。
pub fn inspect_override_full(root: &Element) -> Value {
    let mut v = inspect_override(root);
    if let Value::Object(m) = &mut v {
        m.insert("extraSchemes".into(), extra_schemes_json(root));
        if let Some(Value::Object(fonts)) = m.get_mut("fonts") {
            attach_script_fonts(fonts, root.child_by_local("fontScheme"));
        }
    }
    v
}

/// `a:themeOverride`：配色 / 字体 / fmt 在根下，不经 `themeElements`。
pub fn inspect_override(root: &Element) -> Value {
    let name = root
        .attr_local("name")
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    json!({
        "name": name,
        "colors": colors_of(root.child_by_local("clrScheme")),
        "fonts": fonts_of(root.child_by_local("fontScheme")),
        "fmt": fmt_of(root.child_by_local("fmtScheme")),
    })
}

pub fn fmt_from_theme(root: &Element) -> Value {
    fmt_of(
        root.child_by_local("themeElements")
            .and_then(|e| e.child_by_local("fmtScheme")),
    )
}

pub fn patch_colors(root: &mut Element, slots: &[(String, String)]) -> bool {
    ensure_xmlns(root, "a", DML_NS);
    let Some(scheme) = root
        .child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("clrScheme"))
    else {
        return false;
    };
    patch_scheme_colors(scheme, slots);
    true
}

pub fn patch_scheme_colors(scheme: &mut Element, slots: &[(String, String)]) {
    for (slot, hex) in slots {
        upsert_srgb_slot(scheme, slot, hex);
    }
}

pub fn patch_override_colors(root: &mut Element, slots: &[(String, String)]) {
    ensure_xmlns(root, "a", DML_NS);
    if root.child_by_local("clrScheme").is_none() {
        root.children.insert(0, Node::Element(default_clr_scheme()));
    }
    if let Some(scheme) = root.child_by_local_mut("clrScheme") {
        patch_scheme_colors(scheme, slots);
    }
}

pub fn patch_fonts(
    root: &mut Element,
    patches: &[(FontSlot, String)],
    json_path: &str,
) -> Result<(), Error> {
    ensure_xmlns(root, "a", DML_NS);
    if root.child_by_local("themeElements").is_none() {
        root.children
            .insert(0, Node::Element(Element::new("a:themeElements")));
    }
    let elements = root.child_by_local_mut("themeElements").ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "theme".into(),
                child: "themeElements".into(),
            },
            json_path.to_string(),
        )
    })?;
    if elements.child_by_local("fontScheme").is_none() {
        elements.push_element(Element::new("a:fontScheme").with_attr("name", "Office"));
    }
    let scheme = elements.child_by_local_mut("fontScheme").ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "themeElements".into(),
                child: "fontScheme".into(),
            },
            json_path.to_string(),
        )
    })?;
    for (slot, face) in patches {
        set_typeface(scheme, *slot, face);
    }
    Ok(())
}

pub fn patch_theme_name(root: &mut Element, name: Option<&str>) {
    patch_name_attr(root, name);
}

pub fn patch_clr_scheme_name(root: &mut Element, name: Option<&str>) -> bool {
    let Some(scheme) = root
        .child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("clrScheme"))
    else {
        return false;
    };
    patch_name_attr(scheme, name);
    true
}

pub fn patch_font_scheme_name(root: &mut Element, name: Option<&str>) -> bool {
    let Some(scheme) = root
        .child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("fontScheme"))
    else {
        return false;
    };
    patch_name_attr(scheme, name);
    true
}

pub fn patch_fmt_scheme_name(root: &mut Element, name: Option<&str>) -> bool {
    let Some(scheme) = fmt_scheme_mut(root) else {
        return false;
    };
    patch_name_attr(scheme, name);
    true
}

fn patch_name_attr(el: &mut Element, name: Option<&str>) {
    match name {
        Some(s) => el.set_attr("name", s),
        None => el.remove_attr("name"),
    }
}

pub fn replace_script_fonts(
    root: &mut Element,
    major: bool,
    fonts: &[ScriptFont],
    json_path: &str,
) -> Result<(), Error> {
    patch_fonts(root, &[], json_path)?;
    let group = if major { "majorFont" } else { "minorFont" };
    let elements = root.child_by_local_mut("themeElements").ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "theme".into(),
                child: "themeElements".into(),
            },
            json_path.to_string(),
        )
    })?;
    let scheme = elements.child_by_local_mut("fontScheme").ok_or_else(|| {
        Error::with_path(
            ErrorKind::XmlMissing {
                parent: "themeElements".into(),
                child: "fontScheme".into(),
            },
            json_path.to_string(),
        )
    })?;
    if scheme.child_by_local(group).is_none() {
        scheme.push_element(Element::new(format!("a:{group}")));
    }
    let Some(g) = scheme.child_by_local_mut(group) else {
        return Ok(());
    };
    g.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "font"));
    for font in fonts {
        g.push_element(
            Element::new("a:font")
                .with_attr("script", &font.script)
                .with_attr("typeface", &font.typeface),
        );
    }
    Ok(())
}

pub fn replace_extra_schemes(root: &mut Element, schemes: &[ExtraScheme]) {
    ensure_xmlns(root, "a", DML_NS);
    root.children
        .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "extraClrSchemeLst"));
    if schemes.is_empty() {
        return;
    }
    let mut lst = Element::new("a:extraClrSchemeLst");
    for spec in schemes {
        lst.push_element(extra_scheme_element(spec));
    }
    let dest = insert_index_by_order(root, "extraClrSchemeLst", THEME_ROOT_ORDER);
    root.children.insert(dest, Node::Element(lst));
}

fn extra_scheme_element(spec: &ExtraScheme) -> Element {
    let mut scheme = default_clr_scheme();
    patch_name_attr(&mut scheme, spec.name.as_deref());
    if !spec.slots.is_empty() {
        patch_scheme_colors(&mut scheme, &spec.slots);
    }
    let mut extra = Element::new("a:extraClrScheme");
    extra.push_element(scheme);
    extra.push_element(identity_clr_map());
    extra
}

fn identity_clr_map() -> Element {
    Element::new("a:clrMap")
        .with_attr("bg1", "lt1")
        .with_attr("tx1", "dk1")
        .with_attr("bg2", "lt2")
        .with_attr("tx2", "dk2")
        .with_attr("accent1", "accent1")
        .with_attr("accent2", "accent2")
        .with_attr("accent3", "accent3")
        .with_attr("accent4", "accent4")
        .with_attr("accent5", "accent5")
        .with_attr("accent6", "accent6")
        .with_attr("hlink", "hlink")
        .with_attr("folHlink", "folHlink")
}

fn insert_index_by_order(parent: &Element, local: &str, order: &[&str]) -> usize {
    let my = order
        .iter()
        .position(|&x| x == local)
        .unwrap_or(order.len());
    let mut after_lower = 0usize;
    for (i, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let Some(rank) = order.iter().position(|&x| x == e.local_name()) else {
            continue;
        };
        if rank < my {
            after_lower = i + 1;
        } else if rank > my {
            return i;
        }
    }
    after_lower
}

pub fn patch_line_width(root: &mut Element, index: usize, w: u32) -> bool {
    let Some(scheme) = fmt_scheme_mut(root) else {
        return false;
    };
    let Some(list) = scheme.child_by_local_mut("lnStyleLst") else {
        return false;
    };
    let Some(pos) = element_child_pos(list, index) else {
        return false;
    };
    if let Node::Element(el) = &mut list.children[pos] {
        el.set_attr("w", w.to_string());
        true
    } else {
        false
    }
}

pub fn patch_fill_slot(root: &mut Element, bg: bool, index: usize, kind: FmtFillKind) -> bool {
    let Some(scheme) = fmt_scheme_mut(root) else {
        return false;
    };
    let list_name = if bg { "bgFillStyleLst" } else { "fillStyleLst" };
    let Some(list) = scheme.child_by_local_mut(list_name) else {
        return false;
    };
    let Some(pos) = element_child_pos(list, index) else {
        return false;
    };
    list.children[pos] = Node::Element(fmt_fill_element(kind));
    true
}

pub fn default_theme_xml(name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="{DML_NS}" name="{name}"><a:themeElements><a:clrScheme name="Office">{slots}</a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Office">{fmt}</a:fmtScheme></a:themeElements></a:theme>"#,
        name = xml::escape_attr(name),
        slots = default_clr_scheme_inner(),
        fmt = default_fmt_scheme(),
    )
}

pub fn default_override_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:themeOverride xmlns:a="{DML_NS}"><a:clrScheme name="Office">{slots}</a:clrScheme></a:themeOverride>"#,
        slots = default_clr_scheme_inner(),
    )
}

pub fn clone_clr_scheme(theme_root: &Element) -> Option<Element> {
    theme_root
        .child_by_local("themeElements")
        .and_then(|e| e.child_by_local("clrScheme"))
        .cloned()
}

fn color_scheme_json(root: &Element) -> Value {
    colors_of(
        root.child_by_local("themeElements")
            .and_then(|e| e.child_by_local("clrScheme")),
    )
}

fn font_scheme_json(root: &Element) -> Value {
    fonts_of(
        root.child_by_local("themeElements")
            .and_then(|e| e.child_by_local("fontScheme")),
    )
}

fn colors_of(scheme: Option<&Element>) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "name".into(),
        json!(scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty())),
    );
    for slot in COLOR_SLOTS {
        obj.insert((*slot).into(), json!(slot_hex(scheme, slot)));
    }
    Value::Object(obj)
}

fn fonts_of(scheme: Option<&Element>) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "name".into(),
        json!(scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty())),
    );
    obj.insert(
        "majorLatin".into(),
        json!(typeface_of(scheme, "majorFont", "latin")),
    );
    obj.insert(
        "minorLatin".into(),
        json!(typeface_of(scheme, "minorFont", "latin")),
    );
    obj.insert(
        "majorEa".into(),
        json!(typeface_of(scheme, "majorFont", "ea")),
    );
    obj.insert(
        "minorEa".into(),
        json!(typeface_of(scheme, "minorFont", "ea")),
    );
    obj.insert(
        "majorCs".into(),
        json!(typeface_of(scheme, "majorFont", "cs")),
    );
    obj.insert(
        "minorCs".into(),
        json!(typeface_of(scheme, "minorFont", "cs")),
    );
    Value::Object(obj)
}

pub fn attach_script_fonts(fonts: &mut Map<String, Value>, scheme: Option<&Element>) {
    fonts.insert("majorScripts".into(), script_fonts_of(scheme, "majorFont"));
    fonts.insert("minorScripts".into(), script_fonts_of(scheme, "minorFont"));
}

pub fn extra_schemes_json(root: &Element) -> Value {
    let Some(lst) = root.child_by_local("extraClrSchemeLst") else {
        return json!([]);
    };
    Value::Array(
        lst.element_children()
            .filter(|e| e.local_name() == "extraClrScheme")
            .map(extra_scheme_item)
            .collect(),
    )
}

fn extra_scheme_item(el: &Element) -> Value {
    let scheme = el.child_by_local("clrScheme");
    json!({
        "name": scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty()),
        "colors": colors_of(scheme),
    })
}

fn script_fonts_of(scheme: Option<&Element>, group: &str) -> Value {
    let Some(g) = scheme.and_then(|s| s.child_by_local(group)) else {
        return json!([]);
    };
    Value::Array(
        g.element_children()
            .filter(|e| e.local_name() == "font")
            .map(|e| {
                json!({
                    "script": e.attr_local("script"),
                    "typeface": e.attr_local("typeface"),
                })
            })
            .collect(),
    )
}

fn fmt_of(scheme: Option<&Element>) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "name".into(),
        json!(scheme
            .and_then(|s| s.attr_local("name"))
            .filter(|s| !s.is_empty())),
    );
    let mut fills = Vec::new();
    let mut lines = Vec::new();
    let mut effects = Vec::new();
    let mut bg_fills = Vec::new();
    if let Some(scheme) = scheme {
        if let Some(list) = scheme.child_by_local("fillStyleLst") {
            for el in list.element_children() {
                fills.push(json!({ "kind": fill_kind(el.local_name()) }));
            }
        }
        if let Some(list) = scheme.child_by_local("lnStyleLst") {
            for el in list.element_children() {
                let w = el
                    .attr_local("w")
                    .and_then(s_to_u32)
                    .map_or(Value::Null, |n| json!(n));
                lines.push(json!({ "w": w }));
            }
        }
        if let Some(list) = scheme.child_by_local("effectStyleLst") {
            for el in list.element_children() {
                effects.push(json!({ "kind": effect_kind(el) }));
            }
        }
        if let Some(list) = scheme.child_by_local("bgFillStyleLst") {
            for el in list.element_children() {
                bg_fills.push(json!({ "kind": fill_kind(el.local_name()) }));
            }
        }
    }
    obj.insert("fills".into(), Value::Array(fills));
    obj.insert("lines".into(), Value::Array(lines));
    obj.insert("effects".into(), Value::Array(effects));
    obj.insert("bgFills".into(), Value::Array(bg_fills));
    Value::Object(obj)
}

fn fill_kind(local: &str) -> String {
    match local {
        "solidFill" => "solid".into(),
        "gradFill" => "grad".into(),
        "blipFill" => "blip".into(),
        "noFill" => "none".into(),
        "pattFill" => "patt".into(),
        other => other.to_string(),
    }
}

fn effect_kind(style: &Element) -> String {
    let Some(lst) = style.child_by_local("effectLst") else {
        return "none".into();
    };
    lst.element_children()
        .next()
        .map(|e| e.local_name().to_string())
        .unwrap_or_else(|| "none".into())
}

fn s_to_u32(s: &str) -> Option<u32> {
    s.parse().ok()
}

fn slot_hex(scheme: Option<&Element>, slot: &str) -> Option<String> {
    let el = scheme?.child_by_local(slot)?;
    if let Some(v) = el
        .child_by_local("srgbClr")
        .and_then(|e| e.attr_local("val"))
        .filter(|s| !s.is_empty())
    {
        return Some(v.to_string());
    }
    el.child_by_local("sysClr")
        .and_then(|e| e.attr_local("lastClr"))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn typeface_of(scheme: Option<&Element>, group: &str, face: &str) -> Option<String> {
    scheme?
        .child_by_local(group)
        .and_then(|g| g.child_by_local(face))
        .and_then(|e| e.attr_local("typeface"))
        .map(str::to_string)
}

fn upsert_srgb_slot(scheme: &mut Element, slot: &str, hex: &str) {
    if let Some(el) = scheme.child_by_local_mut(slot) {
        el.children.clear();
        el.push_element(Element::new("a:srgbClr").with_attr("val", hex));
        return;
    }
    let mut el = Element::new(format!("a:{slot}"));
    el.push_element(Element::new("a:srgbClr").with_attr("val", hex));
    scheme.push_element(el);
}

fn set_typeface(scheme: &mut Element, slot: FontSlot, face: &str) {
    if scheme.child_by_local(slot.group()).is_none() {
        scheme.push_element(Element::new(format!("a:{}", slot.group())));
    }
    let Some(group) = scheme.child_by_local_mut(slot.group()) else {
        return;
    };
    if let Some(el) = group.child_by_local_mut(slot.face()) {
        el.set_attr("typeface", face);
        return;
    }
    group.push_element(Element::new(format!("a:{}", slot.face())).with_attr("typeface", face));
}

fn fmt_scheme_mut(root: &mut Element) -> Option<&mut Element> {
    root.child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("fmtScheme"))
}

fn element_child_pos(parent: &Element, index: usize) -> Option<usize> {
    parent
        .children
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, Node::Element(_)))
        .nth(index)
        .map(|(i, _)| i)
}

fn fmt_fill_element(kind: FmtFillKind) -> Element {
    match kind {
        FmtFillKind::Solid => {
            let mut el = Element::new("a:solidFill");
            el.push_element(Element::new("a:schemeClr").with_attr("val", "phClr"));
            el
        }
        FmtFillKind::Grad => {
            let stop = |pos: &str| {
                let mut gs = Element::new("a:gs").with_attr("pos", pos);
                gs.push_element(Element::new("a:schemeClr").with_attr("val", "phClr"));
                gs
            };
            let mut gs_lst = Element::new("a:gsLst");
            gs_lst.push_element(stop("0"));
            gs_lst.push_element(stop("100000"));
            let mut el = Element::new("a:gradFill").with_attr("rotWithShape", "1");
            el.push_element(gs_lst);
            el.push_element(
                Element::new("a:lin")
                    .with_attr("ang", "16200000")
                    .with_attr("scaled", "1"),
            );
            el
        }
    }
}

fn default_clr_scheme() -> Element {
    xml::parse(
        format!(
            r#"<a:clrScheme xmlns:a="{DML_NS}" name="Office">{}</a:clrScheme>"#,
            default_clr_scheme_inner()
        )
        .as_bytes(),
    )
    .expect("默认 clrScheme")
    .root
}

fn default_clr_scheme_inner() -> String {
    r#"<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F497D"/></a:dk2><a:lt2><a:srgbClr val="EEECE1"/></a:lt2><a:accent1><a:srgbClr val="4F81BD"/></a:accent1><a:accent2><a:srgbClr val="C0504D"/></a:accent2><a:accent3><a:srgbClr val="9BBB59"/></a:accent3><a:accent4><a:srgbClr val="8064A2"/></a:accent4><a:accent5><a:srgbClr val="4BACC6"/></a:accent5><a:accent6><a:srgbClr val="F79646"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink>"#
        .into()
}

fn default_fmt_scheme() -> String {
    // Word 16.0 空白另存：fill 为 solid/grad/grad，线宽 12700/19050/25400，bg 为 solid/solid/grad。
    r#"<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="50000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="35000"><a:schemeClr val="phClr"><a:tint val="37000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="15000"/><a:satMod val="350000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="1"/></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="100000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="80000"><a:schemeClr val="phClr"><a:shade val="30000"/><a:satMod val="200000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="200000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="12700" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="19050" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln><a:ln w="25400" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="38000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="40000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="40000"><a:schemeClr val="phClr"><a:tint val="45000"/><a:shade val="99000"/><a:satMod val="350000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="20000"/><a:satMod val="255000"/></a:schemeClr></a:gs></a:gsLst><a:path path="circle"><a:fillToRect l="50000" t="-80000" r="50000" b="180000"/></a:path></a:gradFill></a:bgFillStyleLst>"#
        .into()
}

pub(crate) fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}
