//! 英文错误文案。

use super::super::kind::{
    ErrorKind, IdReason, OpcReason, RangeReason, RelSource, ScopeNeed, SheetNameReason, TypeName,
    XmlIssue,
};

pub(super) fn render(kind: &ErrorKind) -> String {
    match kind {
        ErrorKind::InvalidJsonParse { detail } => format!("Input is not JSON: {detail}"),
        ErrorKind::InvalidJsonNotObject => "Input must be a JSON object".into(),
        ErrorKind::MissingEnvelopeField { field } => format!("Missing {field}"),
        ErrorKind::UnsupportedSchema => "schemaVersion must be 1".into(),
        ErrorKind::UnsupportedAction { action } => format!("Unsupported action: {action}"),
        ErrorKind::UnknownField { field } => format!("Unknown field: {field}"),
        ErrorKind::CannotInferFormat { path } => format!("Cannot infer format from path: {path}"),
        ErrorKind::UnsupportedFormat { format } => format!("Unsupported format: {format}"),
        ErrorKind::MissingField { field } => format!("Missing {field}"),
        ErrorKind::WrongType { field, expected } => {
            format!("{field} must be {}", type_name(*expected))
        }
        ErrorKind::EmptyArray { field } => format!("{field} must not be empty"),
        ErrorKind::OutOfRange { field, min, max } => {
            format!("{field} must be between {min} and {max}")
        }
        ErrorKind::MinValue { field, min } => format!("{field} starts at {min}"),
        ErrorKind::MutuallyExclusive { a, b } => format!("{a} and {b} cannot both be present"),
        ErrorKind::ContentObjectsExclusive => {
            "workbook, document, and presentation cannot appear together".into()
        }
        ErrorKind::CreateNeedsObject { format, field } => {
            format!("{format} create requires {field}")
        }
        ErrorKind::CreateMissingObject { field } => format!("create requires {field}"),
        ErrorKind::ScopeRequired { need } => match need {
            ScopeNeed::SheetAndRange => {
                "Reading cells requires both sheet and range".into()
            }
            ScopeNeed::FromAndMaxParas => {
                "Reading paragraph text requires both from and maxParas".into()
            }
            ScopeNeed::Slide => "Reading shapes requires slide".into(),
        },
        ErrorKind::UnsupportedOp { op } => format!("Unsupported op: {op}"),
        ErrorKind::OpNotForFormat { format, op } => format!("{format} does not support {op}"),
        ErrorKind::SetCellNeedsValueOrFormula => "setCell requires value or formula".into(),
        ErrorKind::FormulaMustStartWithEquals => "formula must start with =".into(),
        ErrorKind::InvalidCellValue => {
            "Cell value must be a number, string, boolean, or null".into()
        }
        ErrorKind::OpRequiresField { op, field } => format!("{op} requires {field}"),
        ErrorKind::InvalidRange { reason } => match reason {
            RangeReason::Empty => "Empty cell address".into(),
            RangeReason::NotA1 { input } => format!("Not an A1 address: {input}"),
            RangeReason::RowOutOfRange { row } => format!("Row out of range: {row}"),
            RangeReason::ColOutOfRange { col } => format!("Column out of range: {col}"),
            RangeReason::Reversed { input } => format!("Range start and end are reversed: {input}"),
            RangeReason::InvalidColLetters { letters } => {
                format!("Invalid column letters: {letters}")
            }
        },
        ErrorKind::InvalidId { reason } => match reason {
            IdReason::NotCell { id } => format!("Not a cell id: {id}"),
            IdReason::ParaPrefix => "Paragraph id must start with para:".into(),
            IdReason::ParaIndexNotInt => "para:index must be followed by a positive integer".into(),
            IdReason::ParaIndexFromOne => "para:index starts at 1".into(),
            IdReason::ParaIdNotHex => "paraId must be hexadecimal".into(),
            IdReason::ShapePrefix => "Shape id must start with slide:".into(),
            IdReason::ShapeForm => "setShapeText id must be slide:N/shape:…".into(),
            IdReason::SlideIndexNotInt => "Slide index must be a positive integer".into(),
            IdReason::SlideIndexFromOne => "Slide index starts at 1".into(),
            IdReason::ShapeEmpty => "shape must not be empty".into(),
            IdReason::ShapeIndexNotInt => {
                "shape:index must be followed by a positive integer".into()
            }
            IdReason::ShapeIndexFromOne => "shape:index starts at 1".into(),
        },
        ErrorKind::SheetNotFound { name } => format!("Sheet not found: {name}"),
        ErrorKind::ParaNotFound { id } => format!("Paragraph not found: {id}"),
        ErrorKind::SlideNotFound { n } => format!("Slide not found: slide:{n}"),
        ErrorKind::ShapeNotFound { id } => format!("Shape not found: {id}"),
        ErrorKind::ShapeHasNoText { id } => format!("Shape has no text box: {id}"),
        ErrorKind::SetCellWorksheetOnly => "setCell only applies to worksheets".into(),
        ErrorKind::InspectWorksheetOnly => "Cells can only be read from worksheets".into(),
        ErrorKind::SharedFormulaMaster { cell } => format!(
            "{cell} is a shared formula master; changing it would leave dependents with the same si without a formula. Change those cells to independent formulas first"
        ),
        ErrorKind::NeedAtLeastOneSheet => "At least one worksheet is required".into(),
        ErrorKind::DuplicateSheetName { name } => format!("Duplicate sheet name: {name}"),
        ErrorKind::InvalidSheetName { reason } => match reason {
            SheetNameReason::Missing => "Missing sheet name".into(),
            SheetNameReason::Length => "Sheet name length must be 1 to 31 characters".into(),
            SheetNameReason::History => "A sheet cannot be named History".into(),
            SheetNameReason::IllegalChars => "Sheet name contains illegal characters".into(),
        },
        ErrorKind::InvalidParaStyle => {
            "style must be Normal, Heading1, Heading2, or Heading3".into()
        }
        ErrorKind::OpcOpen { reason } => match reason {
            OpcReason::EmptyPartName => "Part name must not be empty".into(),
            OpcReason::InvalidPartName { name } => format!("Invalid part name: {name}"),
            OpcReason::MissingContentTypes => "Package is missing [Content_Types].xml".into(),
            OpcReason::MissingRootRels => "Package is missing /_rels/.rels".into(),
            OpcReason::MissingOfficeDocument => {
                "Root relationships have no officeDocument".into()
            }
            OpcReason::RelTarget { target } => {
                format!("Cannot resolve relationship target: {target}")
            }
            OpcReason::Zip { detail } => format!("Cannot open as ZIP: {detail}"),
        },
        ErrorKind::PartMissing { name } => format!("Part not found: {name}"),
        ErrorKind::RelNotFound { source, rid } => match source {
            RelSource::Unspecified => format!("No relationship {rid}"),
            RelSource::Workbook => format!("Workbook has no relationship {rid}"),
            RelSource::Part(part) => format!("{part} has no relationship {rid}"),
        },
        ErrorKind::ExternalRel { rid } => format!("{rid} points to an external resource"),
        ErrorKind::SlideRelMissing { index } => {
            format!("p:sldId is missing a relationship id (slide {index})")
        }
        ErrorKind::XmlBroken { issue, detail } => match (issue, detail) {
            (XmlIssue::UnbalancedEnd, _) => "XML has more end tags than start tags".into(),
            (XmlIssue::NoRoot, _) => "XML has no root element".into(),
            (XmlIssue::TextDecode, Some(d)) => format!("Cannot decode XML text: {d}"),
            (XmlIssue::TextDecode, None) => "Cannot decode XML text".into(),
            (XmlIssue::Parse, Some(d)) => format!("XML parse failed: {d}"),
            (XmlIssue::Parse, None) => "XML parse failed".into(),
            (XmlIssue::AttrRead, Some(d)) => format!("Cannot read XML attribute: {d}"),
            (XmlIssue::AttrRead, None) => "Cannot read XML attribute".into(),
            (XmlIssue::AttrDecode, Some(d)) => format!("Cannot decode XML attribute: {d}"),
            (XmlIssue::AttrDecode, None) => "Cannot decode XML attribute".into(),
            (XmlIssue::Write, Some(d)) => format!("Failed to write XML: {d}"),
            (XmlIssue::Write, None) => "Failed to write XML".into(),
        },
        ErrorKind::XmlMissing { parent, child } => format!("{parent} is missing {child}"),
    }
}

fn type_name(t: TypeName) -> &'static str {
    match t {
        TypeName::Object => "an object",
        TypeName::Array => "an array",
        TypeName::Integer => "an integer",
        TypeName::String => "a string",
        TypeName::NonEmptyString => "a non-empty string",
    }
}
