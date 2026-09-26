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
        ErrorKind::ScopeFieldRequires { field, required } => {
            format!("{field} requires {required}")
        }
        ErrorKind::ActionNotForFormat { format, action } => {
            format!("{format} does not support {action}")
        }
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
            IdReason::ParaPrefix => {
                "Paragraph id must be para:… or header:…/para:… or footer:…/para:…".into()
            }
            IdReason::HdrFtrContainer => {
                "Header/footer id must include /para:… (container id alone is not a paragraph)".into()
            }
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
            IdReason::SectionPrefix => "Section id must be section:N".into(),
            IdReason::SectionIndexNotInt => "section index must be a positive integer".into(),
            IdReason::SectionIndexFromOne => "section index starts at 1".into(),
            IdReason::PicPrefix => {
                "Picture id must be header:…/pic:…, footer:…/pic:…, or para:…/pic:…".into()
            }
            IdReason::PicContainer => {
                "Paragraph or header/footer id must include /pic:… (that id alone is not a picture)"
                    .into()
            }
            IdReason::PicIndexNotInt => "pic:index must be followed by a positive integer".into(),
            IdReason::PicIndexFromOne => "pic:index starts at 1".into(),
            IdReason::InsertNeedsPara => "insertPicture id must be a paragraph id".into(),
            IdReason::SheetPicPrefix => "xlsx picture id must be sheet:name/pic:…".into(),
            IdReason::SheetPicContainer => {
                "Sheet id must include /pic:… (that id alone is not a picture)".into()
            }
            IdReason::InsertNeedsSheet => "xlsx insertPicture id must be sheet:name".into(),
            IdReason::TxbxPrefix => {
                "Text box id must be a paragraph id plus /txbx:…".into()
            }
            IdReason::TxbxContainer => {
                "Paragraph id must include /txbx:… (that id alone is not a text box)".into()
            }
            IdReason::TxbxIndexNotInt => "txbx:index must be followed by a positive integer".into(),
            IdReason::TxbxIndexFromOne => "txbx:index starts at 1".into(),
            IdReason::SetParaOnBox => "setParaText id must be a paragraph id, not a text box id".into(),
            IdReason::InsertNeedsHostPara => "insertTextBox id must be a host paragraph id".into(),
            IdReason::InsertParaBody => {
                "insertPara and deletePara are only allowed inside a text box, a note, or a comment".into()
            }
            IdReason::InsertParaNeedsBox => {
                "insertPara / deletePara id must be a text box, a paragraph inside a text box, a note, a paragraph inside a note, a comment, or a paragraph inside a comment"
                    .into()
            }
            IdReason::NotePrefix => {
                "Footnote/endnote id must be footnote:… or endnote:…".into()
            }
            IdReason::NoteContainer => {
                "Note id must include /para:… (that id alone is not a paragraph)".into()
            }
            IdReason::NoteSep => {
                "Separator ids cannot be used with apply".into()
            },
            IdReason::CommentPrefix => "Comment id must be comment:…".into(),
            IdReason::CommentContainer => {
                "Comment id must include /para:… (that id alone is not a paragraph)".into()
            },
            IdReason::ChartPrefix => {
                "Chart id must be a paragraph id plus /chart:…".into()
            }
            IdReason::ChartContainer => {
                "Paragraph id must include /chart:… (that id alone is not a chart)".into()
            }
            IdReason::ChartIndexNotInt => {
                "chart:index must be followed by a positive integer".into()
            }
            IdReason::ChartIndexFromOne => "chart:index starts at 1".into(),
            IdReason::SeriesPrefix => "Series id must be a chart id plus /series:n".into(),
            IdReason::SeriesIndexNotInt => "series must be followed by an integer starting at 0".into(),
            IdReason::InsertChartNeedsPara => {
                "insertChart id must be a body or header/footer paragraph id".into()
            }
            IdReason::SmartArtPrefix => {
                "SmartArt id must be a paragraph id plus /smartdiagram:…".into()
            }
            IdReason::SmartArtContainer => {
                "Paragraph id must include /smartdiagram:… (that id alone is not a SmartArt)".into()
            }
            IdReason::SmartArtIndexNotInt => {
                "smartdiagram:index must be followed by a positive integer".into()
            }
            IdReason::SmartArtIndexFromOne => "smartdiagram:index starts at 1".into(),
            IdReason::SmartArtPtPrefix => {
                "SmartArt point id must be a diagram id plus /pt:modelId".into()
            }
            IdReason::InsertSmartArtNeedsPara => {
                "insertSmartArt id must be a body or header/footer paragraph id".into()
            },
        },
        ErrorKind::SheetNotFound { name } => format!("Sheet not found: {name}"),
        ErrorKind::ParaNotFound { id } => format!("Paragraph not found: {id}"),
        ErrorKind::SectionNotFound { id } => format!("Section not found: {id}"),
        ErrorKind::SlideNotFound { n } => format!("Slide not found: slide:{n}"),
        ErrorKind::ShapeNotFound { id } => format!("Shape not found: {id}"),
        ErrorKind::ShapeHasNoText { id } => format!("Shape has no text box: {id}"),
        ErrorKind::PictureNotFound { id } => format!("Picture not found: {id}"),
        ErrorKind::TextBoxNotFound { id } => format!("Text box not found: {id}"),
        ErrorKind::LastTextBoxPara => {
            "Cannot delete the last paragraph in a text box".into()
        }
        ErrorKind::LastNotePara => {
            "Cannot delete the last paragraph in a footnote or endnote".into()
        }
        ErrorKind::NoteNotFound { id } => format!("Footnote or endnote not found: {id}"),
        ErrorKind::NeedNoteNumberingField => {
            "setNoteNumbering needs at least one of numFmt, numStart, or numRestart".into()
        }
        ErrorKind::InvalidNoteKind => "kind must be footnote or endnote".into(),
        ErrorKind::InvalidNoteNumFmt => {
            "numFmt must be decimal, upperRoman, lowerRoman, upperLetter, lowerLetter, or chicago"
                .into()
        }
        ErrorKind::InvalidNoteRestart => {
            "numRestart must be continuous, eachSect, or eachPage".into()
        }
        ErrorKind::LastCommentPara => {
            "Cannot delete the last paragraph in a comment".into()
        }
        ErrorKind::CommentNotFound { id } => format!("Comment not found: {id}"),
        ErrorKind::NeedCommentParaId => {
            "The last paragraph of the comment must have w14:paraId to write commentsExtended".into()
        }
        ErrorKind::NeedCommentMetaField => {
            "setCommentMeta needs at least one of author, initials, or date".into()
        }
        ErrorKind::CommentRangeCrossPart => {
            "insertComment start and end must be in the same part".into()
        }
        ErrorKind::CommentRangeReversed => {
            "insertComment start must not come after end".into()
        }
        ErrorKind::InkComment { id } => {
            format!("Ink comments cannot be edited except deleteComment: {id}")
        }
        ErrorKind::ChartNotFound { id } => format!("Chart not found: {id}"),
        ErrorKind::ChartSeriesNotFound { id } => format!("Chart series not found: {id}"),
        ErrorKind::NeedChartSeriesField => {
            "setChartSeries needs at least one of name, categories, values, or sizes".into()
        }
        ErrorKind::LastChartSeries => {
            "Cannot delete the last series in a chart; use deleteChart to remove the chart".into()
        }
        ErrorKind::RestrictedChart { id } => {
            format!("This chart cannot be edited with this operation: {id}")
        }
        ErrorKind::InvalidChartType => {
            "type must be bar, col, line, pie, area, scatter, doughnut, radar, or bubble; insertChart also allows combo".into()
        }
        ErrorKind::InvalidChartAxis => {
            "axis must be cat or val, and the chart must have that primary axis".into()
        }
        ErrorKind::InvalidLegendPos => "pos must be b, t, l, r, or tr".into(),
        ErrorKind::PieNeedsOneSeries => {
            "A pie or doughnut chart must have exactly one series".into()
        }
        ErrorKind::SizesOnNonBubble => "sizes is only valid on a bubble series".into(),
        ErrorKind::InvalidComboSeriesType => {
            "series type must be bar, col, line, area, or scatter, and only combo charts may use it".into()
        }
        ErrorKind::ChartTypeNotConvertible => {
            "This chart type cannot be converted without loss".into()
        }
        ErrorKind::NeedChartCategories => {
            "addChartSeries with omitted categories needs an existing series to copy from".into()
        }
        ErrorKind::SmartArtNotFound { id } => format!("SmartArt not found: {id}"),
        ErrorKind::SmartArtPointNotFound { id } => format!("SmartArt point not found: {id}"),
        ErrorKind::LastSmartArtPoint => {
            "Cannot delete the last visible SmartArt point; use deleteSmartArt to remove the diagram".into()
        }
        ErrorKind::InvalidSmartArtLayout => "layout must be list or org".into(),
        ErrorKind::InvalidSmartArtRole => "role must be node or asst".into(),
        ErrorKind::InvalidSmartArtColors => {
            "colors must be accent1, accent2, accent3, accent4, accent5, or accent6".into()
        }
        ErrorKind::InvalidSmartArtStyle => "style must be simple, subtle, or intense".into(),
        ErrorKind::SmartArtLayoutNotConvertible => {
            "This SmartArt layout cannot be converted without loss".into()
        }
        ErrorKind::SmartArtPointParent => {
            "points[].parent must be an index in the same array and must not form a cycle".into()
        }
        ErrorKind::SmartArtAsstOnList => "list layout does not accept asst points".into(),
        ErrorKind::SmartArtListDepth => {
            "list layout only accepts top-level points (parent must be omitted or null)".into()
        }
        ErrorKind::TextBoxLinkCrossPart => {
            "Text boxes in a link chain must be in the same part".into()
        }
        ErrorKind::InvalidHexColor => "color must be a 6-digit RGB hex value or null".into(),
        ErrorKind::InvalidTextBoxDirection => "direction must be horizontal or vertical".into(),
        ErrorKind::InvalidPicKind => "kind must be inline or anchor".into(),
        ErrorKind::InvalidPicWrap => {
            "wrap must be square, tight, through, topAndBottom, or none".into()
        }
        ErrorKind::InvalidPicAlign => "alignH must be left/center/right; alignV must be top/center/bottom".into(),
        ErrorKind::InvalidImageFile => "file is not a recognized bitmap (PNG, JPEG, GIF, or BMP)".into(),
        ErrorKind::SetCellWorksheetOnly => "setCell only applies to worksheets".into(),
        ErrorKind::InspectWorksheetOnly => "Cells can only be read from worksheets".into(),
        ErrorKind::InsertPictureWorksheetOnly => "insertPicture only applies to worksheets".into(),
        ErrorKind::TwoCellPictureSize => {
            "twoCell picture size is defined by the anchor cells; use setPictureAnchor to change the to cell"
                .into()
        }
        ErrorKind::PictureAnchorAbsolute => {
            "absolute anchors cannot convert to or from oneCell / twoCell; only posX / posY or size may change"
                .into()
        }
        ErrorKind::InvalidXlsxAnchor => "anchor must be oneCell or twoCell".into(),
        ErrorKind::InvalidXlsxEditAs => "editAs must be twoCell, oneCell, or absolute".into(),
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
        ErrorKind::InvalidHfKind => "kind must be header or footer".into(),
        ErrorKind::InvalidHfType => "type must be default, even, or first".into(),
        ErrorKind::CreateHfOnlyDefault => "create headers/footers type must be default".into(),
        ErrorKind::CreateHfDuplicate => "create allows only one default header or footer".into(),
        ErrorKind::CannotLinkFirstSection => {
            "The first section cannot link to a previous section".into()
        }
        ErrorKind::HfTypeInactive { kind, hf_type } => {
            format!("{kind} type {hf_type} is not active on this section")
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
        ErrorKind::SharedStringIndex { cell, index } => {
            format!("Shared string index {index} is missing at cell {cell}")
        }
        ErrorKind::SharedStringNotInTable { text } => {
            format!("Shared string table has no entry for {text}")
        }
    }
}

fn type_name(t: TypeName) -> &'static str {
    match t {
        TypeName::Object => "an object",
        TypeName::Array => "an array",
        TypeName::Integer => "an integer",
        TypeName::String => "a string",
        TypeName::NonEmptyString => "a non-empty string",
        TypeName::Boolean => "a boolean",
    }
}
