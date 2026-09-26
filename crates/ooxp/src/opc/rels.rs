//! 部件关系 `.rels`

use crate::error::{Error, ErrorCode, ErrorKind, OpcReason, RelSource};
use crate::xml;

use super::names::{normalize_part_name, rels_part_name, resolve_rel_target};
use super::Package;

pub const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

#[derive(Clone, Debug)]
pub struct Relationship {
    pub id: String,
    pub rel_type: String,
    pub target: String,
    pub target_mode: Option<String>,
    pub extra: Vec<(String, String)>,
}

impl Relationship {
    pub fn is_external(&self) -> bool {
        self.target_mode
            .as_deref()
            .is_some_and(|m| m.eq_ignore_ascii_case("External"))
    }
}

#[derive(Clone, Debug)]
pub struct Relationships {
    pub rels: Vec<Relationship>,
}

impl Relationships {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let doc = xml::parse(bytes)?;
        let mut rels = Vec::new();
        for child in doc.root.element_children() {
            if child.local_name() != "Relationship" {
                continue;
            }
            let id = child.attr("Id").unwrap_or("").to_string();
            let rel_type = child.attr("Type").unwrap_or("").to_string();
            let target = child.attr("Target").unwrap_or("").to_string();
            let target_mode = child.attr("TargetMode").map(str::to_string);
            let extra = child
                .attrs
                .iter()
                .filter(|(k, _)| {
                    let local = k.rsplit(':').next().unwrap_or(k);
                    !matches!(local, "Id" | "Type" | "Target" | "TargetMode")
                })
                .cloned()
                .collect();
            rels.push(Relationship {
                id,
                rel_type,
                target,
                target_mode,
                extra,
            });
        }
        Ok(Self { rels })
    }

    pub fn from_package(pkg: &Package, source_part: &str) -> Result<Self, Error> {
        let path = rels_part_name(source_part);
        match pkg.get(&path) {
            Ok(bytes) => Self::parse(bytes),
            Err(e) if e.code() == ErrorCode::OpcPartMissing => Ok(Self { rels: Vec::new() }),
            Err(e) => Err(e),
        }
    }

    pub fn find_by_id(&self, id: &str) -> Option<&Relationship> {
        self.rels.iter().find(|r| r.id == id)
    }

    pub fn find_office_document(&self) -> Option<&Relationship> {
        self.rels
            .iter()
            .find(|r| r.rel_type.ends_with("/officeDocument"))
    }

    pub fn find_by_type_suffix(&self, suffix: &str) -> Vec<&Relationship> {
        self.rels
            .iter()
            .filter(|r| r.rel_type.ends_with(suffix))
            .collect()
    }

    #[allow(dead_code)]
    pub fn remove_by_id(&mut self, id: &str) {
        self.rels.retain(|r| r.id != id);
    }

    pub fn remove_by_target_part(&mut self, source_part: &str, target_opc: &str) {
        self.rels.retain(|r| {
            if r.is_external() {
                return true;
            }
            match resolve_rel_target(source_part, &r.target) {
                Ok(p) => p != target_opc,
                Err(_) => true,
            }
        });
    }

    pub fn next_rid(&self) -> String {
        let mut max = 0u32;
        for r in &self.rels {
            if let Some(n) = r.id.strip_prefix("rId").and_then(|s| s.parse().ok()) {
                max = max.max(n);
            }
        }
        format!("rId{}", max + 1)
    }

    pub fn push(&mut self, rel: Relationship) {
        self.rels.push(rel);
    }

    pub fn to_xml(&self) -> String {
        let mut body = String::new();
        for r in &self.rels {
            let mut extra = String::new();
            if let Some(mode) = &r.target_mode {
                extra.push_str(&format!(r#" TargetMode="{}""#, xml::escape_attr(mode)));
            }
            for (k, v) in &r.extra {
                extra.push_str(&format!(r#" {}="{}""#, k, xml::escape_attr(v)));
            }
            body.push_str(&format!(
                r#"<Relationship Id="{}" Type="{}" Target="{}"{extra}/>"#,
                xml::escape_attr(&r.id),
                xml::escape_attr(&r.rel_type),
                xml::escape_attr(&r.target),
            ));
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS_NS}">{body}</Relationships>"#
        )
    }

    pub fn write_to(&self, pkg: &mut Package, source_part: &str) -> Result<(), Error> {
        let path = rels_part_name(source_part);
        pkg.set(&path, self.to_xml().into_bytes())
    }
}

pub fn resolve_id(pkg: &Package, source_part: &str, rid: &str) -> Result<String, Error> {
    let rels = Relationships::from_package(pkg, source_part)?;
    let rel = rels.find_by_id(rid).ok_or_else(|| {
        Error::new(ErrorKind::RelNotFound {
            source: RelSource::Part(source_part.to_string()),
            rid: rid.to_string(),
        })
    })?;
    if rel.is_external() {
        return Err(Error::new(ErrorKind::ExternalRel {
            rid: rid.to_string(),
        }));
    }
    resolve_rel_target(source_part, &rel.target)
}

pub fn office_document_part(pkg: &Package) -> Result<String, Error> {
    let rels = Relationships::from_package(pkg, "/")?;
    let rel = rels
        .find_office_document()
        .ok_or_else(|| {
            Error::new(ErrorKind::OpcOpen {
                reason: OpcReason::MissingOfficeDocument,
            })
        })?;
    resolve_rel_target("/", &rel.target)
}

/// 规范化源部件名，供 write 使用。
#[allow(dead_code)]
pub fn normalize_source(name: &str) -> Result<String, Error> {
    if name == "/" {
        Ok("/".into())
    } else {
        normalize_part_name(name)
    }
}
