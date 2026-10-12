//! create 第四轮：文档级字段解析，复用已有 apply op。
//! 不写修订痕迹，不写 writePassword。

use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::biblio;
use super::east_asia::{self, KinsokuChars};
use super::field::{self, CrossRefKind};
use super::font_embed::FontStyle;
use super::glossary;
use super::request::{
    InsertDocPartOp, InsertEmbeddedFontOp, InsertSourceOp, InsertStyleOp, SetEastAsianSettingsOp,
    SetFontEmbedOp,
};
use super::styles;
use super::tbl::{
    self, DiagonalBorder, TableFloat, TableLookPatch, TableRowHeightSpec, TableWidthPatch,
};

pub const ROOT_KEYS: &[&str] = &[
    "styles",
    "sources",
    "bibliographyStyle",
    "bibliographies",
    "captions",
    "indexes",
    "docParts",
    "embeddedFonts",
    "fontEmbed",
    "eastAsianSettings",
];

pub struct AfterBlock {
    pub after: Option<u32>,
    pub instr: String,
}

pub struct CaptionBlock {
    pub after: Option<u32>,
    pub label: String,
    pub title: Option<String>,
    pub bookmark: Option<String>,
}

pub struct DocExtra {
    pub styles: Vec<InsertStyleOp>,
    pub sources: Vec<InsertSourceOp>,
    pub bibliography_style: Option<String>,
    pub bibliographies: Vec<AfterBlock>,
    pub captions: Vec<CaptionBlock>,
    pub indexes: Vec<AfterBlock>,
    pub doc_parts: Vec<InsertDocPartOp>,
    pub fonts: Vec<InsertEmbeddedFontOp>,
    pub font_embed: Option<SetFontEmbedOp>,
    pub east_asia: Option<SetEastAsianSettingsOp>,
}

pub fn parse(obj: &serde_json::Map<String, Value>, para_n: usize) -> Result<DocExtra, Error> {
    Ok(DocExtra {
        styles: parse_styles(obj.get("styles"))?,
        sources: parse_sources(obj.get("sources"))?,
        bibliography_style: parse_biblio_style(obj.get("bibliographyStyle"))?,
        bibliographies: parse_after_blocks(
            obj.get("bibliographies"),
            "document.bibliographies",
            para_n,
            field::default_bibliography_instr(),
            field::instr_is_bibliography,
        )?,
        captions: parse_captions(obj.get("captions"), para_n)?,
        indexes: parse_after_blocks(
            obj.get("indexes"),
            "document.indexes",
            para_n,
            field::default_index_instr(),
            field::instr_is_index,
        )?,
        doc_parts: parse_doc_parts(obj.get("docParts"))?,
        fonts: parse_fonts(obj.get("embeddedFonts"))?,
        font_embed: parse_font_embed(obj.get("fontEmbed"))?,
        east_asia: parse_east_asia(obj.get("eastAsianSettings"))?,
    })
}

pub fn is_empty(extra: &DocExtra) -> bool {
    extra.styles.is_empty()
        && extra.sources.is_empty()
        && extra.bibliography_style.is_none()
        && extra.bibliographies.is_empty()
        && extra.captions.is_empty()
        && extra.indexes.is_empty()
        && extra.doc_parts.is_empty()
        && extra.fonts.is_empty()
        && extra.font_embed.is_none()
        && extra.east_asia.is_none()
}

fn parse_styles(v: Option<&Value>) -> Result<Vec<InsertStyleOp>, Error> {
    let Some(arr) = as_opt_array(v, "document.styles")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let path = format!("document.styles[{i}]");
        let obj = as_object(item, &path)?;
        deny_keys(obj, &path, &["styleId", "type", "name", "basedOn"])?;
        let style_id = require_string(obj, &path, "styleId")?;
        styles::validate_style_id_token(&style_id, &format!("{path}.styleId"))?;
        let ty = match obj.get("type").and_then(Value::as_str) {
            Some(t @ ("paragraph" | "character" | "table")) => t.to_string(),
            Some(_) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidStyleType,
                    format!("{path}.type"),
                ));
            }
            None => {
                return Err(Error::with_path(
                    ErrorKind::NeedInsertStyleField,
                    format!("{path}.type"),
                ));
            }
        };
        let name = match obj.get("name") {
            None => style_id.clone(),
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::NeedInsertStyleField,
                    format!("{path}.name"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "name",
                    TypeName::String,
                    &format!("{path}.name"),
                ));
            }
        };
        let based_on = match obj.get("basedOn") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(styles::parse_style_ref(s, &format!("{path}.basedOn"))?),
            Some(_) => {
                return Err(wrong_type(
                    "basedOn",
                    TypeName::String,
                    &format!("{path}.basedOn"),
                ));
            }
        };
        out.push(InsertStyleOp {
            style_id,
            ty,
            name,
            based_on,
        });
    }
    Ok(out)
}

fn parse_sources(v: Option<&Value>) -> Result<Vec<InsertSourceOp>, Error> {
    let Some(arr) = as_opt_array(v, "document.sources")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let path = format!("document.sources[{i}]");
        let obj = as_object(item, &path)?;
        deny_keys(
            obj,
            &path,
            &[
                "tag",
                "sourceType",
                "title",
                "year",
                "last",
                "first",
                "city",
                "publisher",
            ],
        )?;
        let tag = require_string(obj, &path, "tag")?;
        if biblio::tag_invalid(&tag) {
            return Err(Error::with_path(
                ErrorKind::InvalidSourceTag,
                format!("{path}.tag"),
            ));
        }
        let source_type = match obj.get("sourceType") {
            None => biblio::DEFAULT_SOURCE_TYPE.to_string(),
            Some(Value::String(s)) if biblio::source_type_ok(s) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidSourceType,
                    format!("{path}.sourceType"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "sourceType",
                    TypeName::String,
                    &format!("{path}.sourceType"),
                ));
            }
        };
        let title = require_string(obj, &path, "title")?;
        if title.is_empty() || title.chars().all(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::EmptySourceTitle,
                format!("{path}.title"),
            ));
        }
        out.push(InsertSourceOp {
            tag,
            source_type,
            title,
            year: opt_string(obj, &path, "year")?,
            last: opt_string(obj, &path, "last")?,
            first: opt_string(obj, &path, "first")?,
            city: opt_string(obj, &path, "city")?,
            publisher: opt_string(obj, &path, "publisher")?,
        });
    }
    Ok(out)
}

fn parse_biblio_style(v: Option<&Value>) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) if biblio::style_ok(s) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidBibliographyStyle,
            "document.bibliographyStyle",
        )),
        Some(_) => Err(wrong_type(
            "bibliographyStyle",
            TypeName::String,
            "document.bibliographyStyle",
        )),
    }
}

fn parse_after_blocks(
    v: Option<&Value>,
    path: &str,
    para_n: usize,
    default_instr: &str,
    instr_ok: fn(&str) -> bool,
) -> Result<Vec<AfterBlock>, Error> {
    let Some(arr) = as_opt_array(v, path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(obj, &item_path, &["after", "instr"])?;
        let after = parse_after(obj.get("after"), &format!("{item_path}.after"), para_n)?;
        let instr = match obj.get("instr") {
            None => default_instr.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() && instr_ok(s) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::EmptyFieldInstr,
                    format!("{item_path}.instr"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "instr",
                    TypeName::String,
                    &format!("{item_path}.instr"),
                ));
            }
        };
        out.push(AfterBlock { after, instr });
    }
    Ok(out)
}

fn parse_captions(v: Option<&Value>, para_n: usize) -> Result<Vec<CaptionBlock>, Error> {
    let Some(arr) = as_opt_array(v, "document.captions")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let path = format!("document.captions[{i}]");
        let obj = as_object(item, &path)?;
        deny_keys(obj, &path, &["after", "label", "title", "bookmark"])?;
        let after = parse_after(obj.get("after"), &format!("{path}.after"), para_n)?;
        let label = match obj.get("label") {
            None => "Figure".to_string(),
            Some(Value::String(s)) => {
                super::field::parse_seq_identifier(s)
                    .map_err(|kind| Error::with_path(kind, format!("{path}.label")))?;
                s.clone()
            }
            Some(_) => {
                return Err(wrong_type(
                    "label",
                    TypeName::String,
                    &format!("{path}.label"),
                ));
            }
        };
        let title = opt_string(obj, &path, "title")?;
        let bookmark = match obj.get("bookmark") {
            None => None,
            Some(Value::String(s)) => {
                if s.is_empty() || s.chars().any(char::is_whitespace) {
                    return Err(Error::with_path(
                        ErrorKind::InvalidBookmarkName,
                        format!("{path}.bookmark"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(wrong_type(
                    "bookmark",
                    TypeName::String,
                    &format!("{path}.bookmark"),
                ));
            }
        };
        out.push(CaptionBlock {
            after,
            label,
            title,
            bookmark,
        });
    }
    Ok(out)
}

fn parse_doc_parts(v: Option<&Value>) -> Result<Vec<InsertDocPartOp>, Error> {
    let Some(arr) = as_opt_array(v, "document.docParts")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let path = format!("document.docParts[{i}]");
        let obj = as_object(item, &path)?;
        deny_keys(
            obj,
            &path,
            &["name", "text", "gallery", "category", "behavior"],
        )?;
        let name = require_string(obj, &path, "name")?;
        if name.is_empty() || name.chars().all(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::EmptyDocPartName,
                format!("{path}.name"),
            ));
        }
        let text = require_string(obj, &path, "text")?;
        if text.is_empty() || text.chars().all(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::EmptyDocPartText,
                format!("{path}.text"),
            ));
        }
        let gallery = match obj.get("gallery") {
            None => glossary::DEFAULT_GALLERY.to_string(),
            Some(Value::String(s)) if glossary::gallery_ok(s) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartGallery,
                    format!("{path}.gallery"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "gallery",
                    TypeName::String,
                    &format!("{path}.gallery"),
                ));
            }
        };
        let category = match obj.get("category") {
            None => glossary::DEFAULT_CATEGORY.to_string(),
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::EmptyDocPartName,
                    format!("{path}.category"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "category",
                    TypeName::String,
                    &format!("{path}.category"),
                ));
            }
        };
        let behavior = match obj.get("behavior") {
            None => glossary::DEFAULT_BEHAVIOR.to_string(),
            Some(Value::String(s)) if glossary::behavior_ok(s) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidDocPartBehavior,
                    format!("{path}.behavior"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "behavior",
                    TypeName::String,
                    &format!("{path}.behavior"),
                ));
            }
        };
        out.push(InsertDocPartOp {
            name,
            gallery,
            category,
            behavior,
            text,
        });
    }
    Ok(out)
}

fn parse_fonts(v: Option<&Value>) -> Result<Vec<InsertEmbeddedFontOp>, Error> {
    let Some(arr) = as_opt_array(v, "document.embeddedFonts")? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let path = format!("document.embeddedFonts[{i}]");
        let obj = as_object(item, &path)?;
        deny_keys(obj, &path, &["name", "file", "style"])?;
        let name = require_string(obj, &path, "name")?;
        if name.is_empty() || name.chars().all(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::EmptyFontName,
                format!("{path}.name"),
            ));
        }
        let file = require_string(obj, &path, "file")?;
        if file.is_empty() || file.chars().all(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::InvalidFontFile,
                format!("{path}.file"),
            ));
        }
        let style = match obj.get("style") {
            None => FontStyle::Regular,
            Some(Value::String(s)) => FontStyle::parse(s).ok_or_else(|| {
                Error::with_path(ErrorKind::InvalidFontStyle, format!("{path}.style"))
            })?,
            Some(_) => {
                return Err(wrong_type(
                    "style",
                    TypeName::String,
                    &format!("{path}.style"),
                ));
            }
        };
        out.push(InsertEmbeddedFontOp { name, style, file });
    }
    Ok(out)
}

fn parse_font_embed(v: Option<&Value>) -> Result<Option<SetFontEmbedOp>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = as_object(v, "document.fontEmbed")?;
    deny_keys(
        obj,
        "document.fontEmbed",
        &["trueType", "systemFonts", "subset"],
    )?;
    let true_type = opt_bool(obj, "document.fontEmbed", "trueType")?;
    let system_fonts = opt_bool(obj, "document.fontEmbed", "systemFonts")?;
    let subset = opt_bool(obj, "document.fontEmbed", "subset")?;
    if true_type.is_none() && system_fonts.is_none() && subset.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedFontEmbedField,
            "document.fontEmbed",
        ));
    }
    Ok(Some(SetFontEmbedOp {
        true_type,
        system_fonts,
        subset,
    }))
}

fn parse_east_asia(v: Option<&Value>) -> Result<Option<SetEastAsianSettingsOp>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = as_object(v, "document.eastAsianSettings")?;
    deny_keys(
        obj,
        "document.eastAsianSettings",
        &[
            "characterSpacingControl",
            "strictFirstAndLastChars",
            "noLineBreaksBefore",
            "noLineBreaksAfter",
        ],
    )?;
    let character_spacing = match obj.get("characterSpacingControl") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) if east_asia::CHARACTER_SPACING_SET.contains(&s.as_str()) => {
            Some(Some(s.clone()))
        }
        Some(Value::String(_)) => {
            return Err(Error::with_path(
                ErrorKind::InvalidCharacterSpacing,
                "document.eastAsianSettings.characterSpacingControl",
            ));
        }
        Some(_) => {
            return Err(wrong_type(
                "characterSpacingControl",
                TypeName::String,
                "document.eastAsianSettings.characterSpacingControl",
            ));
        }
    };
    let strict = opt_null_bool(obj, "document.eastAsianSettings", "strictFirstAndLastChars")?;
    let no_before = opt_kinsoku(obj, "document.eastAsianSettings", "noLineBreaksBefore")?;
    let no_after = opt_kinsoku(obj, "document.eastAsianSettings", "noLineBreaksAfter")?;
    if character_spacing.is_none() && strict.is_none() && no_before.is_none() && no_after.is_none()
    {
        return Err(Error::with_path(
            ErrorKind::NeedSetEastAsianSettingsField,
            "document.eastAsianSettings",
        ));
    }
    Ok(Some(SetEastAsianSettingsOp {
        character_spacing,
        strict,
        no_before,
        no_after,
    }))
}

fn opt_kinsoku(
    obj: &serde_json::Map<String, Value>,
    prefix: &str,
    field: &str,
) -> Result<Option<Option<KinsokuChars>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(inner)) => {
            deny_keys(inner, &format!("{prefix}.{field}"), &["lang", "chars"])?;
            let lang = match inner.get("lang") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuLang,
                        format!("{prefix}.{field}.lang"),
                    ));
                }
            };
            let chars = match inner.get("chars") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyKinsokuChars,
                        format!("{prefix}.{field}.chars"),
                    ));
                }
            };
            Ok(Some(Some(KinsokuChars { lang, chars })))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Object,
            &format!("{prefix}.{field}"),
        )),
    }
}

fn parse_after(v: Option<&Value>, path: &str, para_n: usize) -> Result<Option<u32>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(Error::with_path(
                    ErrorKind::WrongType {
                        field: "after".into(),
                        expected: TypeName::Integer,
                    },
                    path,
                ));
            };
            if u == 0 || u > para_n as u64 {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: "after".into(),
                        min: 1,
                        max: para_n as i64,
                    },
                    path,
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(wrong_type("after", TypeName::Integer, path)),
    }
}

pub fn deny_keys(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    allowed: &[&str],
) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    Ok(())
}

fn as_opt_array<'a>(v: Option<&'a Value>, path: &str) -> Result<Option<&'a Vec<Value>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Array(a)) => Ok(Some(a)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: path.rsplit('.').next().unwrap_or(path).into(),
                expected: TypeName::Array,
            },
            path,
        )),
    }
}

fn as_object<'a>(v: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, Error> {
    v.as_object().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: path.into(),
                expected: TypeName::Object,
            },
            path,
        )
    })
}

fn require_string(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<String, Error> {
    match obj.get(field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{path}.{field}"),
        )),
    }
}

fn opt_string(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_bool(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<bool>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(wrong_type(
            field,
            TypeName::Boolean,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_null_bool(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<bool>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Bool(b)) => Ok(Some(Some(*b))),
        Some(_) => Err(wrong_type(
            field,
            TypeName::Boolean,
            &format!("{path}.{field}"),
        )),
    }
}

fn wrong_type(field: &str, expected: TypeName, path: &str) -> Error {
    Error::with_path(
        ErrorKind::WrongType {
            field: field.into(),
            expected,
        },
        path,
    )
}

#[derive(Clone)]
pub struct DrawingHlink {
    pub url: Option<String>,
    pub anchor: Option<String>,
    pub tooltip: Option<String>,
}

#[derive(Clone)]
pub struct ShapeSpec {
    pub prst: String,
    pub cx: Option<i64>,
    pub cy: Option<i64>,
}

#[derive(Clone)]
pub struct RubySpec {
    pub base: String,
    pub rt: String,
    pub align: String,
    pub hps: u32,
    pub hps_raise: u32,
    pub hps_base_text: u32,
    pub lid: String,
}

#[derive(Clone)]
pub struct CitationSpec {
    pub tag: String,
}

#[derive(Clone)]
pub struct IndexEntrySpec {
    pub entry: String,
}

#[derive(Clone)]
pub struct CrossRefSpec {
    pub bookmark: String,
    pub kind: CrossRefKind,
}

#[derive(Clone)]
pub struct EditRangeSpec {
    pub ed_grp: Option<String>,
    pub ed: Option<String>,
}

#[derive(Clone)]
pub struct CellMarSpec {
    pub top: Option<Option<u32>>,
    pub left: Option<Option<u32>>,
    pub bottom: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
}

#[derive(Clone)]
pub struct CellFormatSpec {
    pub r: u32,
    pub c: u32,
    pub fill: Option<Option<String>>,
    pub top: Option<Option<u32>>,
    pub left: Option<Option<u32>>,
    pub bottom: Option<Option<u32>>,
    pub right: Option<Option<u32>>,
    pub tl2br: Option<Option<DiagonalBorder>>,
    pub tr2bl: Option<Option<DiagonalBorder>>,
    pub v_align: Option<Option<String>>,
}

pub struct TableExtra {
    pub style: Option<String>,
    pub look: Option<TableLookPatch>,
    pub pos: Option<TableFloat>,
    pub align: Option<String>,
    pub indent: Option<u32>,
    pub width: Option<TableWidthPatch>,
    pub row_heights: Vec<TableRowHeightSpec>,
    pub cell_mar: Option<CellMarSpec>,
    pub cell_formats: Vec<CellFormatSpec>,
}

impl TableExtra {
    pub fn has_follow_ups(&self) -> bool {
        self.style.is_some()
            || self.look.is_some()
            || self.pos.is_some()
            || self.align.is_some()
            || self.indent.is_some()
            || self.width.is_some()
            || !self.row_heights.is_empty()
            || self.cell_mar.is_some()
            || !self.cell_formats.is_empty()
    }
}

pub fn parse_drawing_hlink(v: Option<&Value>, path: &str) -> Result<Option<DrawingHlink>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let obj = as_object(v, path)?;
    deny_keys(obj, path, &["url", "anchor", "tooltip"])?;
    let url = opt_string(obj, path, "url")?;
    let anchor = opt_string(obj, path, "anchor")?;
    match (&url, &anchor) {
        (Some(_), Some(_)) => {
            return Err(Error::with_path(
                ErrorKind::MutuallyExclusive {
                    a: "url".into(),
                    b: "anchor".into(),
                },
                path,
            ));
        }
        (None, None) => {
            return Err(Error::with_path(ErrorKind::HyperlinkNeedTarget, path));
        }
        _ => {}
    }
    if let Some(u) = url.as_deref() {
        if u.is_empty() {
            return Err(Error::with_path(
                ErrorKind::EmptyHyperlinkUrl,
                format!("{path}.url"),
            ));
        }
    }
    if let Some(a) = anchor.as_deref() {
        if a.is_empty() || a.starts_with('_') || a.chars().any(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::InvalidBookmarkName,
                format!("{path}.anchor"),
            ));
        }
    }
    Ok(Some(DrawingHlink {
        url,
        anchor,
        tooltip: opt_string(obj, path, "tooltip")?,
    }))
}

pub fn parse_shapes(v: Option<&Value>, prefix: &str) -> Result<Vec<ShapeSpec>, Error> {
    parse_preset_shapes(
        v,
        &format!("{prefix}.shapes"),
        "rect",
        super::shape::is_writable_prst,
        ErrorKind::InvalidShapePrst,
        "insertShape",
    )
}

pub fn parse_connectors(v: Option<&Value>, prefix: &str) -> Result<Vec<ShapeSpec>, Error> {
    parse_preset_shapes(
        v,
        &format!("{prefix}.connectors"),
        "straightConnector1",
        super::connector::is_writable_prst,
        ErrorKind::InvalidConnectorPrst,
        "insertConnector",
    )
}

fn parse_preset_shapes(
    v: Option<&Value>,
    path: &str,
    default_prst: &str,
    prst_ok: fn(&str) -> bool,
    bad_prst: ErrorKind,
    op: &str,
) -> Result<Vec<ShapeSpec>, Error> {
    let Some(arr) = as_opt_array(v, path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(obj, &item_path, &["prst", "cx", "cy"])?;
        let cx = parse_emu(obj.get("cx"), &item_path, "cx")?;
        let cy = parse_emu(obj.get("cy"), &item_path, "cy")?;
        if cx.is_some() != cy.is_some() {
            return Err(Error::with_path(
                ErrorKind::OpRequiresField {
                    op: op.into(),
                    field: "cx and cy together".into(),
                },
                item_path.clone(),
            ));
        }
        let prst = match obj.get("prst") {
            None => default_prst.to_string(),
            Some(Value::String(s)) if prst_ok(s) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(bad_prst, format!("{item_path}.prst")));
            }
            Some(_) => {
                return Err(wrong_type(
                    "prst",
                    TypeName::String,
                    &format!("{item_path}.prst"),
                ));
            }
        };
        out.push(ShapeSpec { prst, cx, cy });
    }
    Ok(out)
}

pub fn parse_rubies(v: Option<&Value>, prefix: &str) -> Result<Vec<RubySpec>, Error> {
    let path = format!("{prefix}.rubies");
    let Some(arr) = as_opt_array(v, &path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(
            obj,
            &item_path,
            &[
                "base",
                "rt",
                "align",
                "hps",
                "hpsRaise",
                "hpsBaseText",
                "lid",
            ],
        )?;
        let base = require_nonempty(obj, &item_path, "base", ErrorKind::EmptyRubyBase)?;
        let rt = require_nonempty(obj, &item_path, "rt", ErrorKind::EmptyRubyRt)?;
        let align = match obj.get("align") {
            None => "distributeSpace".to_string(),
            Some(Value::String(s)) if super::ruby::ALIGN_SET.contains(&s.as_str()) => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::InvalidRubyAlign,
                    format!("{item_path}.align"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "align",
                    TypeName::String,
                    &format!("{item_path}.align"),
                ));
            }
        };
        let hps = parse_opt_u32_range(obj, &item_path, "hps", 2, 3276)?.unwrap_or(12);
        let hps_raise = parse_opt_u32_range(obj, &item_path, "hpsRaise", 2, 3276)?.unwrap_or(hps);
        let hps_base_text =
            parse_opt_u32_range(obj, &item_path, "hpsBaseText", 2, 3276)?.unwrap_or(24);
        let lid = match obj.get("lid") {
            None => "zh-CN".to_string(),
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::String(_)) => {
                return Err(Error::with_path(
                    ErrorKind::EmptyRubyLid,
                    format!("{item_path}.lid"),
                ));
            }
            Some(_) => {
                return Err(wrong_type(
                    "lid",
                    TypeName::String,
                    &format!("{item_path}.lid"),
                ));
            }
        };
        out.push(RubySpec {
            base,
            rt,
            align,
            hps,
            hps_raise,
            hps_base_text,
            lid,
        });
    }
    Ok(out)
}

pub fn parse_citations(v: Option<&Value>, prefix: &str) -> Result<Vec<CitationSpec>, Error> {
    let path = format!("{prefix}.citations");
    let Some(arr) = as_opt_array(v, &path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(obj, &item_path, &["tag"])?;
        let tag = require_string(obj, &item_path, "tag")?;
        if biblio::tag_invalid(&tag) {
            return Err(Error::with_path(
                ErrorKind::InvalidCitationTag,
                format!("{item_path}.tag"),
            ));
        }
        out.push(CitationSpec { tag });
    }
    Ok(out)
}

pub fn parse_index_entries(v: Option<&Value>, prefix: &str) -> Result<Vec<IndexEntrySpec>, Error> {
    let path = format!("{prefix}.indexEntries");
    let Some(arr) = as_opt_array(v, &path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(obj, &item_path, &["entry"])?;
        let entry = require_string(obj, &item_path, "entry")?;
        field::parse_index_entry(&entry)
            .map_err(|kind| Error::with_path(kind, format!("{item_path}.entry")))?;
        out.push(IndexEntrySpec { entry });
    }
    Ok(out)
}

pub fn parse_cross_refs(v: Option<&Value>, prefix: &str) -> Result<Vec<CrossRefSpec>, Error> {
    let path = format!("{prefix}.crossRefs");
    let Some(arr) = as_opt_array(v, &path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(obj, &item_path, &["bookmark", "kind"])?;
        let bookmark = require_string(obj, &item_path, "bookmark")?;
        if bookmark.is_empty() || bookmark.chars().any(char::is_whitespace) {
            return Err(Error::with_path(
                ErrorKind::InvalidBookmarkName,
                format!("{item_path}.bookmark"),
            ));
        }
        let kind = match obj.get("kind") {
            None => CrossRefKind::Ref,
            Some(Value::String(s)) => field::parse_cross_ref_kind(s)
                .map_err(|kind| Error::with_path(kind, format!("{item_path}.kind")))?,
            Some(_) => {
                return Err(wrong_type(
                    "kind",
                    TypeName::String,
                    &format!("{item_path}.kind"),
                ));
            }
        };
        out.push(CrossRefSpec { bookmark, kind });
    }
    Ok(out)
}

pub fn parse_edit_range(v: Option<&Value>, prefix: &str) -> Result<Option<EditRangeSpec>, Error> {
    let path = format!("{prefix}.editRange");
    match v {
        None => Ok(None),
        Some(Value::Bool(true)) => Ok(Some(EditRangeSpec {
            ed_grp: Some("everyone".into()),
            ed: None,
        })),
        Some(Value::Bool(false)) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "editRange".into(),
                expected: TypeName::Object,
            },
            path,
        )),
        Some(Value::Object(obj)) => {
            deny_keys(obj, &path, &["edGrp", "ed"])?;
            let ed_grp = match obj.get("edGrp") {
                None => None,
                Some(Value::String(s)) if super::perm::ed_grp_ok(s) => Some(s.clone()),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidEdGrp,
                        format!("{path}.edGrp"),
                    ));
                }
                Some(_) => {
                    return Err(wrong_type(
                        "edGrp",
                        TypeName::String,
                        &format!("{path}.edGrp"),
                    ));
                }
            };
            let ed = match obj.get("ed") {
                None => None,
                Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::EmptyEditRangeEd,
                        format!("{path}.ed"),
                    ));
                }
                Some(_) => {
                    return Err(wrong_type("ed", TypeName::String, &format!("{path}.ed")));
                }
            };
            if ed_grp.is_some() && ed.is_some() {
                return Err(Error::with_path(
                    ErrorKind::MutuallyExclusive {
                        a: "edGrp".into(),
                        b: "ed".into(),
                    },
                    path,
                ));
            }
            let ed_grp = if ed_grp.is_none() && ed.is_none() {
                Some("everyone".into())
            } else {
                ed_grp
            };
            Ok(Some(EditRangeSpec { ed_grp, ed }))
        }
        Some(_) => Err(wrong_type("editRange", TypeName::Object, &path)),
    }
}

pub fn parse_table_extra(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    rows: usize,
    cols: usize,
) -> Result<TableExtra, Error> {
    let style = match obj.get("style") {
        None => None,
        Some(Value::String(s)) => Some(styles::parse_style_ref(s, &format!("{path}.style"))?),
        Some(_) => {
            return Err(wrong_type(
                "style",
                TypeName::String,
                &format!("{path}.style"),
            ));
        }
    };
    let look = match obj.get("look") {
        None => None,
        Some(Value::Object(m)) => {
            deny_keys(
                m,
                &format!("{path}.look"),
                &[
                    "firstRow",
                    "lastRow",
                    "firstColumn",
                    "lastColumn",
                    "bandedRows",
                    "bandedColumns",
                ],
            )?;
            Some(TableLookPatch {
                first_row: opt_bool(m, &format!("{path}.look"), "firstRow")?,
                last_row: opt_bool(m, &format!("{path}.look"), "lastRow")?,
                first_column: opt_bool(m, &format!("{path}.look"), "firstColumn")?,
                last_column: opt_bool(m, &format!("{path}.look"), "lastColumn")?,
                banded_rows: opt_bool(m, &format!("{path}.look"), "bandedRows")?,
                banded_columns: opt_bool(m, &format!("{path}.look"), "bandedColumns")?,
            })
        }
        Some(_) => {
            return Err(wrong_type(
                "look",
                TypeName::Object,
                &format!("{path}.look"),
            ));
        }
    };
    let pos = match obj.get("pos") {
        None => None,
        Some(Value::Object(m)) => {
            let p = format!("{path}.pos");
            deny_keys(
                m,
                &p,
                &[
                    "horzAnchor",
                    "vertAnchor",
                    "x",
                    "y",
                    "leftFromText",
                    "rightFromText",
                    "topFromText",
                    "bottomFromText",
                ],
            )?;
            Some(TableFloat {
                horz_anchor: opt_tbl_anchor(m, &p, "horzAnchor")?,
                vert_anchor: opt_tbl_anchor(m, &p, "vertAnchor")?,
                x: opt_i32(m, &p, "x")?,
                y: opt_i32(m, &p, "y")?,
                left_from_text: opt_u32_min(m, &p, "leftFromText", 0)?,
                right_from_text: opt_u32_min(m, &p, "rightFromText", 0)?,
                top_from_text: opt_u32_min(m, &p, "topFromText", 0)?,
                bottom_from_text: opt_u32_min(m, &p, "bottomFromText", 0)?,
            })
        }
        Some(_) => {
            return Err(wrong_type("pos", TypeName::Object, &format!("{path}.pos")));
        }
    };
    let align = match obj.get("align") {
        None => None,
        Some(Value::String(s)) => {
            if !tbl::TBL_ALIGN_VALS.contains(&s.as_str()) {
                return Err(Error::with_path(
                    ErrorKind::InvalidTblAlign,
                    format!("{path}.align"),
                ));
            }
            Some(s.clone())
        }
        Some(_) => {
            return Err(wrong_type(
                "align",
                TypeName::String,
                &format!("{path}.align"),
            ));
        }
    };
    let indent = match obj.get("indent") {
        None => None,
        Some(Value::Number(n)) => {
            let u = n.as_u64().ok_or_else(|| {
                wrong_type("indent", TypeName::Integer, &format!("{path}.indent"))
            })?;
            Some(u as u32)
        }
        Some(_) => {
            return Err(wrong_type(
                "indent",
                TypeName::Integer,
                &format!("{path}.indent"),
            ));
        }
    };
    let width_type = match obj.get("width") {
        None => None,
        Some(Value::Object(m)) => {
            let p = format!("{path}.width");
            deny_keys(m, &p, &["type", "w"])?;
            Some((
                tbl::parse_opt_width_type(m, &p)?,
                tbl::parse_opt_twip(m, &p, "w")?,
            ))
        }
        Some(_) => {
            return Err(wrong_type(
                "width",
                TypeName::Object,
                &format!("{path}.width"),
            ));
        }
    };
    let col_ws = tbl::parse_cols_array(obj.get("cols"), &format!("{path}.cols"))?;
    if let Some(c) = col_ws.as_ref() {
        if c.len() != cols {
            return Err(Error::with_path(
                ErrorKind::TableColsCount,
                format!("{path}.cols"),
            ));
        }
    }
    let width = match (width_type, col_ws) {
        (None, None) => None,
        (Some((ty, w)), col_ws) => Some(tbl::finish_width_patch(
            ty,
            w,
            col_ws,
            &format!("{path}.width"),
        )?),
        (None, Some(col_ws)) => Some(tbl::finish_width_patch(None, None, Some(col_ws), path)?),
    };
    let row_heights =
        tbl::parse_row_heights(obj.get("rowHeights"), &format!("{path}.rowHeights"), rows)?;
    let cell_mar = match obj.get("cellMar") {
        None => None,
        Some(Value::Object(m)) => {
            let p = format!("{path}.cellMar");
            deny_keys(m, &p, &["top", "left", "bottom", "right"])?;
            let top = opt_null_u32(m, &p, "top")?;
            let left = opt_null_u32(m, &p, "left")?;
            let bottom = opt_null_u32(m, &p, "bottom")?;
            let right = opt_null_u32(m, &p, "right")?;
            if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
                return Err(Error::with_path(ErrorKind::NeedSetTableCellMarField, p));
            }
            Some(CellMarSpec {
                top,
                left,
                bottom,
                right,
            })
        }
        Some(_) => {
            return Err(wrong_type(
                "cellMar",
                TypeName::Object,
                &format!("{path}.cellMar"),
            ));
        }
    };
    let cell_formats = parse_cell_formats(obj.get("cellFormats"), path, rows, cols)?;
    Ok(TableExtra {
        style,
        look,
        pos,
        align,
        indent,
        width,
        row_heights,
        cell_mar,
        cell_formats,
    })
}

fn parse_cell_formats(
    v: Option<&Value>,
    path: &str,
    rows: usize,
    cols: usize,
) -> Result<Vec<CellFormatSpec>, Error> {
    let field_path = format!("{path}.cellFormats");
    let Some(arr) = as_opt_array(v, &field_path)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let item_path = format!("{field_path}[{i}]");
        let obj = as_object(item, &item_path)?;
        deny_keys(
            obj,
            &item_path,
            &[
                "r", "c", "fill", "top", "left", "bottom", "right", "tl2br", "tr2bl", "vAlign",
            ],
        )?;
        let r = require_index1(obj, &item_path, "r", rows)?;
        let c = require_index1(obj, &item_path, "c", cols)?;
        let fill = opt_null_rgb(obj, &item_path, "fill")?;
        let top = opt_null_u32(obj, &item_path, "top")?;
        let left = opt_null_u32(obj, &item_path, "left")?;
        let bottom = opt_null_u32(obj, &item_path, "bottom")?;
        let right = opt_null_u32(obj, &item_path, "right")?;
        let tl2br = opt_null_diagonal(obj, &item_path, "tl2br")?;
        let tr2bl = opt_null_diagonal(obj, &item_path, "tr2bl")?;
        let v_align = opt_null_enum(
            obj,
            &item_path,
            "vAlign",
            tbl::CELL_VALIGN,
            ErrorKind::InvalidSectValue {
                field: "vAlign".into(),
            },
        )?;
        if fill.is_none()
            && top.is_none()
            && left.is_none()
            && bottom.is_none()
            && right.is_none()
            && tl2br.is_none()
            && tr2bl.is_none()
            && v_align.is_none()
        {
            return Err(Error::with_path(
                ErrorKind::NeedSetCellFormatField,
                item_path,
            ));
        }
        out.push(CellFormatSpec {
            r,
            c,
            fill,
            top,
            left,
            bottom,
            right,
            tl2br,
            tr2bl,
            v_align,
        });
    }
    Ok(out)
}

fn require_index1(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    max: usize,
) -> Result<u32, Error> {
    match obj.get(field) {
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(wrong_type(
                    field,
                    TypeName::Integer,
                    &format!("{path}.{field}"),
                ));
            };
            if u == 0 || u > max as u64 {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: 1,
                        max: max as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            Ok(u as u32)
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Integer,
            &format!("{path}.{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{path}.{field}"),
        )),
    }
}

fn require_nonempty(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    empty: ErrorKind,
) -> Result<String, Error> {
    match obj.get(field) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(Error::with_path(empty, format!("{path}.{field}"))),
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
        None => Err(Error::with_path(
            ErrorKind::MissingField {
                field: field.into(),
            },
            format!("{path}.{field}"),
        )),
    }
}

fn parse_emu(v: Option<&Value>, prefix: &str, field: &str) -> Result<Option<i64>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let path = format!("{prefix}.{field}");
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                if u == 0 || u > i64::MAX as u64 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: field.into(),
                            min: 1,
                        },
                        path,
                    ));
                }
                return Ok(Some(u as i64));
            }
            Err(wrong_type(field, TypeName::Integer, &path))
        }
        _ => Err(wrong_type(field, TypeName::Integer, &path)),
    }
}

fn parse_opt_u32_range(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    min: u64,
    max: u64,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let u = n
                .as_u64()
                .ok_or_else(|| wrong_type(field, TypeName::Integer, &format!("{path}.{field}")))?;
            if u < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: min as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            if u > max {
                return Err(Error::with_path(
                    ErrorKind::OutOfRange {
                        field: field.into(),
                        min: min as i64,
                        max: max as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Integer,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_tbl_anchor(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<String>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::String(s)) if tbl::TBL_ANCHORS.contains(&s.as_str()) => Ok(Some(s.clone())),
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidTblAnchor,
            format!("{path}.{field}"),
        )),
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_i32(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<i32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let v = n
                .as_i64()
                .ok_or_else(|| wrong_type(field, TypeName::Integer, &format!("{path}.{field}")))?;
            i32::try_from(v)
                .map(Some)
                .map_err(|_| wrong_type(field, TypeName::Integer, &format!("{path}.{field}")))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Integer,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_u32_min(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    min: u64,
) -> Result<Option<u32>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Number(n)) => {
            let u = n
                .as_u64()
                .ok_or_else(|| wrong_type(field, TypeName::Integer, &format!("{path}.{field}")))?;
            if u < min {
                return Err(Error::with_path(
                    ErrorKind::MinValue {
                        field: field.into(),
                        min: min as i64,
                    },
                    format!("{path}.{field}"),
                ));
            }
            Ok(Some(u as u32))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Integer,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_null_u32(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<u32>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let u = n
                .as_u64()
                .ok_or_else(|| wrong_type(field, TypeName::Integer, &format!("{path}.{field}")))?;
            Ok(Some(Some(u as u32)))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Integer,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_null_enum(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
    allowed: &[&str],
    bad: ErrorKind,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => {
            if !allowed.contains(&s.as_str()) {
                return Err(Error::with_path(bad, format!("{path}.{field}")));
            }
            Ok(Some(Some(s.clone())))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_null_rgb(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<String>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Ok(Some(Some(s.to_ascii_uppercase())))
        }
        Some(Value::String(_)) => Err(Error::with_path(
            ErrorKind::InvalidStyleColor,
            format!("{path}.{field}"),
        )),
        Some(_) => Err(wrong_type(
            field,
            TypeName::String,
            &format!("{path}.{field}"),
        )),
    }
}

fn opt_null_diagonal(
    obj: &serde_json::Map<String, Value>,
    path: &str,
    field: &str,
) -> Result<Option<Option<DiagonalBorder>>, Error> {
    match obj.get(field) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(m)) => {
            let p = format!("{path}.{field}");
            deny_keys(m, &p, &["val", "sz", "color"])?;
            let val = match m.get("val") {
                None => "single".to_string(),
                Some(Value::String(s)) if tbl::is_diagonal_val(s) => s.clone(),
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderVal,
                        format!("{p}.val"),
                    ));
                }
                Some(_) => return Err(wrong_type("val", TypeName::String, &format!("{p}.val"))),
            };
            let sz = opt_u32_min(m, &p, "sz", 0)?;
            let color = match m.get("color") {
                None => None,
                Some(Value::String(s)) if s.eq_ignore_ascii_case("auto") => Some("auto".into()),
                Some(Value::String(s))
                    if s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    Some(s.to_ascii_uppercase())
                }
                Some(Value::String(_)) => {
                    return Err(Error::with_path(
                        ErrorKind::InvalidPageBorderColor,
                        format!("{p}.color"),
                    ));
                }
                Some(_) => {
                    return Err(wrong_type("color", TypeName::String, &format!("{p}.color")))
                }
            };
            Ok(Some(Some(DiagonalBorder { val, sz, color })))
        }
        Some(_) => Err(wrong_type(
            field,
            TypeName::Object,
            &format!("{path}.{field}"),
        )),
    }
}
