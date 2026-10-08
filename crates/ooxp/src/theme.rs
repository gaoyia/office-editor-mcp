//! DrawingML theme 部件：`clrScheme` / `fontScheme` 的 inspect 与写入。
//! 不算主题排版引擎，不重写 `fmtScheme`。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

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

pub fn patch_colors(root: &mut Element, slots: &[(String, String)]) -> bool {
    ensure_xmlns(root, "a", DML_NS);
    let Some(scheme) = root
        .child_by_local_mut("themeElements")
        .and_then(|e| e.child_by_local_mut("clrScheme"))
    else {
        return false;
    };
    for (slot, hex) in slots {
        upsert_srgb_slot(scheme, slot, hex);
    }
    true
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

fn color_scheme_json(root: &Element) -> Value {
    let mut obj = Map::new();
    let scheme = root
        .child_by_local("themeElements")
        .and_then(|e| e.child_by_local("clrScheme"));
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

fn font_scheme_json(root: &Element) -> Value {
    let scheme = root
        .child_by_local("themeElements")
        .and_then(|e| e.child_by_local("fontScheme"));
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

fn ensure_xmlns(el: &mut Element, prefix: &str, uri: &str) {
    let key = format!("xmlns:{prefix}");
    if el.attr(&key).is_none() {
        el.set_attr(&key, uri);
    }
}
