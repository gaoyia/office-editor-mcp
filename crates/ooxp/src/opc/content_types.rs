//! `[Content_Types].xml`

use crate::error::Error;
use crate::xml;

use super::names::normalize_part_name;
use super::Package;

pub const CONTENT_TYPES_PART: &str = "/[Content_Types].xml";
pub const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

#[derive(Clone, Debug)]
pub struct ContentTypes {
    pub defaults: Vec<(String, String)>,
    pub overrides: Vec<(String, String)>,
}

impl ContentTypes {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let doc = xml::parse(bytes)?;
        let mut defaults = Vec::new();
        let mut overrides = Vec::new();
        for child in doc.root.element_children() {
            match child.local_name() {
                "Default" => {
                    let ext = child.attr("Extension").unwrap_or("").to_string();
                    let ct = child.attr("ContentType").unwrap_or("").to_string();
                    defaults.push((ext, ct));
                }
                "Override" => {
                    let pn = child.attr("PartName").unwrap_or("").to_string();
                    let ct = child.attr("ContentType").unwrap_or("").to_string();
                    overrides.push((pn, ct));
                }
                _ => {}
            }
        }
        Ok(Self {
            defaults,
            overrides,
        })
    }

    pub fn from_package(pkg: &Package) -> Result<Self, Error> {
        Self::parse(pkg.get(CONTENT_TYPES_PART)?)
    }

    #[allow(dead_code)]
    pub fn content_type_of(&self, opc_name: &str) -> Option<&str> {
        let name = if opc_name.starts_with('/') {
            opc_name
        } else {
            return None;
        };
        if let Some((_, ct)) = self
            .overrides
            .iter()
            .find(|(p, _)| p.eq_ignore_ascii_case(name))
        {
            return Some(ct);
        }
        let ext = name.rsplit('.').next()?;
        self.defaults
            .iter()
            .find(|(e, _)| e.eq_ignore_ascii_case(ext))
            .map(|(_, ct)| ct.as_str())
    }

    pub fn set_override(&mut self, part_name: &str, content_type: &str) -> Result<(), Error> {
        let part = normalize_part_name(part_name)?;
        if let Some((_, ct)) = self.overrides.iter_mut().find(|(p, _)| p == &part) {
            *ct = content_type.to_string();
            return Ok(());
        }
        self.overrides.push((part, content_type.to_string()));
        Ok(())
    }

    pub fn remove_override(&mut self, part_name: &str) {
        self.overrides.retain(|(p, _)| p != part_name);
    }

    #[allow(dead_code)]
    pub fn ensure_default(&mut self, ext: &str, content_type: &str) {
        if !self.defaults.iter().any(|(e, _)| e == ext) {
            self.defaults
                .push((ext.to_string(), content_type.to_string()));
        }
    }

    pub fn to_xml(&self) -> String {
        let mut defaults = String::new();
        for (ext, ct) in &self.defaults {
            defaults.push_str(&format!(
                r#"<Default Extension="{}" ContentType="{}"/>"#,
                xml::escape_attr(ext),
                xml::escape_attr(ct)
            ));
        }
        let mut overrides = String::new();
        for (pn, ct) in &self.overrides {
            overrides.push_str(&format!(
                r#"<Override PartName="{}" ContentType="{}"/>"#,
                xml::escape_attr(pn),
                xml::escape_attr(ct)
            ));
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="{CONTENT_TYPES_NS}">{defaults}{overrides}</Types>"#
        )
    }

    pub fn write_to(&self, pkg: &mut Package) -> Result<(), Error> {
        pkg.set(CONTENT_TYPES_PART, self.to_xml().into_bytes())
    }
}
