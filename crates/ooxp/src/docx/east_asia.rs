//! 整篇 `settings.xml` 上的东亚压缩与自定义禁则字符。不算排版引擎。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::error::Error;
use crate::opc::Package;
use crate::xml::{self, Document, Element};

use super::hf;
use super::notes;
use super::page_bg;

pub const CHARACTER_SPACING_SET: &[&str] = &[
    "doNotCompress",
    "compressPunctuation",
    "compressPunctuationAndJapaneseKana",
];

#[derive(Debug, Clone)]
pub struct KinsokuChars {
    pub lang: String,
    pub chars: String,
}

pub struct SettingsPatch<'a> {
    pub character_spacing: Option<Option<&'a str>>,
    pub strict: Option<Option<bool>>,
    pub no_before: Option<Option<&'a KinsokuChars>>,
    pub no_after: Option<Option<&'a KinsokuChars>>,
}

pub fn settings_json(pkg: &Package, doc_part: &str) -> Result<Value, Error> {
    let empty = json!({
        "characterSpacingControl": null,
        "strictFirstAndLastChars": null,
        "noLineBreaksBefore": null,
        "noLineBreaksAfter": null,
    });
    let Some(name) = notes::settings_part_name(pkg, doc_part)? else {
        return Ok(empty);
    };
    let Ok(bytes) = pkg.get(&name) else {
        return Ok(empty);
    };
    let doc = xml::parse(bytes)?;
    Ok(json!({
        "characterSpacingControl": child_val(&doc.root, "characterSpacingControl"),
        "strictFirstAndLastChars": hf::on_off_opt(&doc.root, "strictFirstAndLastChars"),
        "noLineBreaksBefore": kinsoku_json(&doc.root, "noLineBreaksBefore"),
        "noLineBreaksAfter": kinsoku_json(&doc.root, "noLineBreaksAfter"),
    }))
}

pub fn apply_settings(
    pkg: &mut Package,
    docs: &mut HashMap<String, Document>,
    dirty: &mut HashMap<String, ()>,
    doc_part: &str,
    patch: SettingsPatch<'_>,
) -> Result<(), Error> {
    let writes = patch
        .character_spacing
        .as_ref()
        .and_then(|v| v.as_ref())
        .is_some()
        || matches!(patch.strict, Some(Some(_)))
        || patch.no_before.as_ref().and_then(|v| v.as_ref()).is_some()
        || patch.no_after.as_ref().and_then(|v| v.as_ref()).is_some();
    let settings = match notes::settings_part_name(pkg, doc_part)? {
        Some(p) => p,
        None if writes => page_bg::ensure_settings(pkg, docs, dirty, doc_part)?,
        None => return Ok(()),
    };
    if !docs.contains_key(&settings) {
        let parsed = xml::parse(pkg.get(&settings)?)?;
        docs.insert(settings.clone(), parsed);
    }
    let doc = docs.get_mut(&settings).expect("已载入 settings");
    if let Some(v) = patch.character_spacing {
        match v {
            Some(s) => {
                let el = hf::ensure_child_in_order(
                    &mut doc.root,
                    "characterSpacingControl",
                    hf::SETTINGS_ORDER,
                );
                let key = el.prefixed("val");
                el.set_attr(&key, s);
            }
            None => hf::remove_local(&mut doc.root, "characterSpacingControl"),
        }
    }
    if let Some(v) = patch.strict {
        match v {
            Some(true) => {
                let el = hf::ensure_child_in_order(
                    &mut doc.root,
                    "strictFirstAndLastChars",
                    hf::SETTINGS_ORDER,
                );
                let key = el.prefixed("val");
                el.remove_attr(&key);
            }
            Some(false) => {
                let el = hf::ensure_child_in_order(
                    &mut doc.root,
                    "strictFirstAndLastChars",
                    hf::SETTINGS_ORDER,
                );
                let key = el.prefixed("val");
                el.set_attr(&key, "0");
            }
            None => hf::remove_local(&mut doc.root, "strictFirstAndLastChars"),
        }
    }
    if let Some(v) = patch.no_before {
        apply_kinsoku(&mut doc.root, "noLineBreaksBefore", v);
    }
    if let Some(v) = patch.no_after {
        apply_kinsoku(&mut doc.root, "noLineBreaksAfter", v);
    }
    dirty.insert(settings, ());
    Ok(())
}

fn apply_kinsoku(root: &mut Element, local: &str, val: Option<&KinsokuChars>) {
    match val {
        Some(k) => {
            let el = hf::ensure_child_in_order(root, local, hf::SETTINGS_ORDER);
            let lang = el.prefixed("lang");
            let v = el.prefixed("val");
            el.set_attr(&lang, &k.lang);
            el.set_attr(&v, &k.chars);
        }
        None => hf::remove_local(root, local),
    }
}

fn child_val(parent: &Element, local: &str) -> Value {
    parent
        .child_by_local(local)
        .and_then(|e| e.attr_local("val"))
        .filter(|s| !s.is_empty())
        .map(|s| json!(s))
        .unwrap_or(Value::Null)
}

fn kinsoku_json(parent: &Element, local: &str) -> Value {
    let Some(el) = parent.child_by_local(local) else {
        return Value::Null;
    };
    json!({
        "lang": el.attr_local("lang"),
        "chars": el.attr_local("val"),
    })
}
