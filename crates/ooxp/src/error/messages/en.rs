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
        ErrorKind::CreateIndexOutOfRange { field, max } => format!("{field} must not exceed {max}"),
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
            IdReason::ShapeForm => {
                "setShapeText id must be slide:N/shape:… or slide:N/notes/shape:…".into()
            }
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
            IdReason::SheetPicPrefix => {
                "xlsx picture id must be sheet:name/pic:… or sheet:name/hf:…/pic:…".into()
            }
            IdReason::SheetPicContainer => {
                "Sheet id must include /pic:… (that id alone is not a picture)".into()
            }
            IdReason::InsertNeedsSheet => {
                "xlsx insertPicture id must be sheet:name or sheet:name/hf:kind:side".into()
            }
            IdReason::SlidePicPrefix => {
                "pptx picture id must be slide:N/pic:…, slide:N/notes/pic:…, master:N/pic:…, master:N/layout:K/pic:…, or slide:N/comment:GUID/pic:n".into()
            }
            IdReason::SlidePicContainer => {
                "Slide id must include /pic:… (that id alone is not a picture)".into()
            }
            IdReason::CommentPicExportOnly => {
                "comment pictures have no canvas size or position".into()
            }
            IdReason::SlideMediaForm => {
                "pptx video / audio id must be slide:N/video:…, slide:N/audio:…, slide:N/notes/…, master:N/… or master:N/layout:K/…".into()
            }
            IdReason::InsertMediaNeedsSlide => {
                "pptx insertVideo / insertAudio id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SlideCxnForm => {
                "pptx connector id must be slide:N/cxn:…, slide:N/notes/cxn:…, master:N/cxn:… or master:N/layout:K/cxn:…".into()
            }
            IdReason::InsertConnectorNeedsSlide => {
                "pptx insertConnector id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::InsertNeedsSlide => {
                "pptx insertPicture id must be slide:N, slide:N/notes, master:N, master:N/layout:K, or a comment id".into()
            }
            IdReason::InsertShapeHost => {
                "pptx insertShape id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SetNotesTextNeedsSlide => {
                "setNotesText id must be slide:N or slide:N/notes".into()
            }
            IdReason::PptxCommentForm => {
                "pptx comment id must be slide:N/comment:GUID or …/reply:GUID".into()
            }
            IdReason::InsertCommentNeedsSlide => {
                "pptx insertComment id must be slide:N".into()
            }
            IdReason::XlsxCommentForm => {
                "xlsx comment id must be cell:Sheet!A1/comment:GUID or …/reply:GUID".into()
            }
            IdReason::InsertCommentNeedsCell => {
                "xlsx insertComment id must be a cell id cell:Sheet!A1".into()
            }
            IdReason::HfSlotPrefix => {
                "xlsx header/footer id must be sheet:name/hf:odd|even|first:header|footer".into()
            }
            IdReason::TxbxPrefix => {
                "Text box id must be a paragraph id plus /txbx:…".into()
            }
            IdReason::TxbxContainer => {
                "Paragraph id must include /txbx:… (that id alone is not a text box)".into()
            }
            IdReason::TxbxIndexNotInt => "txbx:index must be followed by a positive integer".into(),
            IdReason::TxbxIndexFromOne => "txbx:index starts at 1".into(),
            IdReason::SetParaOnBox => "setParaText id must be a paragraph id, not a text box id".into(),
            IdReason::InsertNeedsHostPara => {
                "insertTextBox / insertConnector / insertInkCanvas id must be a host paragraph id".into()
            }
            IdReason::DocxCxnPrefix => {
                "docx connector id must be a paragraph id plus /cxn:…".into()
            }
            IdReason::DocxCxnContainer => {
                "paragraph or container id must include /cxn:… (that id alone is not a connector)".into()
            }
            IdReason::DocxInkPrefix => {
                "docx ink canvas id must be a paragraph id plus /ink:…".into()
            }
            IdReason::DocxInkContainer => {
                "paragraph or container id must include /ink:… (that id alone is not an ink canvas)".into()
            }
            IdReason::DocxInkIndexNotInt => "ink:index must be followed by a positive integer".into(),
            IdReason::DocxInkIndexFromOne => "ink:index starts at 1".into(),
            IdReason::DocxModel3dPrefix => {
                "docx 3D model id must be a paragraph id plus /model3d:…".into()
            }
            IdReason::DocxModel3dContainer => {
                "paragraph or container id must include /model3d:… (that id alone is not a 3D model)".into()
            }
            IdReason::DocxModel3dIndexNotInt => {
                "model3d:index must be followed by a positive integer".into()
            }
            IdReason::DocxModel3dIndexFromOne => "model3d:index starts at 1".into(),
            IdReason::GroupNeedsDocument => {
                "docx groupShapes id must be docx:document or a header/footer container".into()
            }
            IdReason::SheetSpPrefix => {
                "xlsx text box id must be sheet:name/sp:…".into()
            }
            IdReason::SheetSpContainer => {
                "Sheet id must include /sp:… (that id alone is not a text box)".into()
            }
            IdReason::SpIndexNotInt => "sp:index must be followed by a positive integer".into(),
            IdReason::SpIndexFromOne => "sp:index starts at 1".into(),
            IdReason::InsertTextBoxNeedsSheet => {
                "xlsx insertTextBox id must be sheet:name".into()
            }
            IdReason::SheetShapePrefix => {
                "xlsx shape id must be sheet:name/shape:…".into()
            }
            IdReason::SheetShapeContainer => {
                "Sheet id must include /shape:… (that id alone is not a shape)".into()
            }
            IdReason::InsertShapeNeedsSheet => {
                "xlsx insertShape id must be sheet:name".into()
            }
            IdReason::SheetCxnPrefix => {
                "xlsx connector id must be sheet:name/cxn:…".into()
            }
            IdReason::SheetCxnContainer => {
                "Sheet id must include /cxn:… (that id alone is not a connector)".into()
            }
            IdReason::CxnIndexNotInt => "cxn:index must be followed by a positive integer".into(),
            IdReason::CxnIndexFromOne => "cxn:index starts at 1".into(),
            IdReason::InsertConnectorNeedsSheet => {
                "xlsx insertConnector id must be sheet:name".into()
            }
            IdReason::GroupNeedsSheet => {
                "xlsx groupShapes id must be sheet:name".into()
            }
            IdReason::SetShapeTextNotCxn => {
                "xlsx setShapeText id must be a text box or shape id, not a connector".into()
            }
            IdReason::SheetSmartArtPrefix => {
                "xlsx SmartArt id must be sheet:name/smartdiagram:…".into()
            }
            IdReason::SheetSmartArtContainer => {
                "Sheet id must include /smartdiagram:… (that id alone is not a SmartArt)".into()
            }
            IdReason::InsertSmartArtNeedsSheet => {
                "xlsx insertSmartArt id must be sheet:name".into()
            }
            IdReason::InsertParaNeedsBox => {
                "deletePara id must be a paragraph id (body, header/footer, table cell, text box, note, or comment)".into()
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
            IdReason::TrendlinePrefix => {
                "Trendline id must be a series id plus /trendline:k starting at 1".into()
            }
            IdReason::TrendlineIndexNotInt => {
                "trendline must be followed by a positive integer".into()
            }
            IdReason::TrendlineIndexFromOne => "trendline index starts at 1".into(),
            IdReason::ErrorBarsPrefix => {
                "Error bars id must be a series id plus /errorBars:k starting at 1".into()
            }
            IdReason::ErrorBarsIndexNotInt => {
                "errorBars must be followed by a positive integer".into()
            }
            IdReason::ErrorBarsIndexFromOne => "errorBars index starts at 1".into(),
            IdReason::InsertChartNeedsPara => {
                "insertChart id must be a body or header/footer paragraph id".into()
            }
            IdReason::InsertChartNeedsSheet => {
                "xlsx insertChart id must be sheet:name".into()
            }
            IdReason::InsertChartNeedsSlide => {
                "pptx insertChart id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::InsertChartSheetNeedsWorkbook => {
                "insertChartSheet id must be xlsx:workbook".into()
            }
            IdReason::InsertSheetNeedsWorkbook => {
                "insertSheet id must be xlsx:workbook".into()
            }
            IdReason::InsertSlideNeedsPresentation => {
                "insertSlide id must be pptx:presentation".into()
            }
            IdReason::InsertSlideSectionNeedsPresentation => {
                "insertSlideSection id must be pptx:presentation".into()
            }
            IdReason::InsertCustomShowNeedsPresentation => {
                "insertCustomShow id must be pptx:presentation".into()
            }
            IdReason::CustomShowPrefix => "Custom show id must be show:N".into(),
            IdReason::CustomShowIndexNotInt => "show index must be a positive integer".into(),
            IdReason::CustomShowIndexFromOne => "show index starts at 1".into(),
            IdReason::SetSlideSectionNeedsSlide => {
                "setSlideSection id must be slide:N".into()
            }
            IdReason::SlideOpNeedsSlide => {
                "id must be slide:N".into()
            }
            IdReason::SetTransitionNeedsHost => {
                "setTransition id must be slide:N, master:N, or master:N/layout:K".into()
            }
            IdReason::MasterForm => {
                "master / placeholder id must be master:N, master:N/layout:K, notesMaster/shape:…, handout/shape:…, or …/shape:…".into()
            }
            IdReason::MasterPlaceholderLocked => {
                "master / layout placeholders use deletePlaceholder; decorative shapes use deleteShape / setShapeFill / setShapeLine".into()
            }
            IdReason::LayoutForm => {
                "layout id must be master:N/layout:K".into()
            }
            IdReason::InsertLayoutNeedsMaster => {
                "insertLayout id must be master:N".into()
            }
            IdReason::InsertPlaceholderNeedsHost => {
                "insertPlaceholder id must be master:N, master:N/layout:K, notesMaster, or handout".into()
            }
            IdReason::InsertMasterNeedsPresentation => {
                "insertMaster id must be pptx:presentation".into()
            }
            IdReason::DeleteMasterNeedsMaster => {
                "deleteMaster id must be master:N".into()
            }
            IdReason::SetSlideSizeNeedsPresentation => {
                "setSlideSize id must be pptx:presentation".into()
            }
            IdReason::SetShowPrNeedsPresentation => {
                "setShowPr id must be pptx:presentation".into()
            }
            IdReason::SetProtectionNeedsPresentation => {
                "setProtection id must be pptx:presentation".into()
            }
            IdReason::SetThemeNeedsMaster => {
                "setThemeColors / setThemeFonts id must be master:N".into()
            }
            IdReason::SetThemeNeedsDocument => {
                "setThemeColors / setThemeFonts id must be docx:document".into()
            }
            IdReason::SetHandoutTextNeedsHandout => {
                "setHandoutText id must be handout".into()
            }
            IdReason::InsertDefinedNameNeedsWorkbook => {
                "insertDefinedName id must be xlsx:workbook".into()
            }
            IdReason::InsertConnectionNeedsWorkbook => {
                "insertConnection id must be xlsx:workbook".into()
            }
            IdReason::InsertExternalLinkNeedsWorkbook => {
                "insertExternalLink id must be xlsx:workbook".into()
            }
            IdReason::InsertCustomViewNeedsWorkbook => {
                "insertCustomView id must be xlsx:workbook".into()
            }
            IdReason::CustomViewPrefix => {
                "custom view id must be customView:n, n starting at 1".into()
            }
            IdReason::CustomViewIndexNotInt => {
                "customView must be followed by a positive integer".into()
            }
            IdReason::CustomViewIndexFromOne => "customView starts at 1".into(),
            IdReason::ConnectionPrefix => {
                "connection id must be connection:ID or connection:index:N".into()
            }
            IdReason::ConnectionIndexNotInt => {
                "connection:index must be followed by a positive integer".into()
            }
            IdReason::ConnectionIndexFromOne => "connection:index starts at 1".into(),
            IdReason::ExternalLinkPrefix => {
                "external link id must be extlink:n, n starting at 1".into()
            }
            IdReason::ExternalLinkIndexNotInt => {
                "extlink must be followed by a positive integer".into()
            }
            IdReason::ExternalLinkIndexFromOne => "extlink starts at 1".into(),
            IdReason::DefinedNamePrefix => {
                "defined name id must be name:n, n starting at 1".into()
            }
            IdReason::NameIndexNotInt => "name must be followed by a positive integer".into(),
            IdReason::NameIndexFromOne => "defined name index starts at 1".into(),
            IdReason::DeleteChartSheetNeedsChartsheet => {
                "deleteChartSheet id must be a chartsheet sheet:name".into()
            }
            IdReason::DeleteSheetNeedsWorksheet => {
                "deleteSheet id must be a worksheet sheet:name".into()
            }
            IdReason::SheetOpNeedsSheet => {
                "id must be sheet:name".into()
            }
            IdReason::SetProtectionNeedsSheetOrWorkbook => {
                "setProtection id must be sheet:name or xlsx:workbook".into()
            }
            IdReason::SheetDvPrefix => {
                "data validation id must be sheet:name/dv:n, n starting at 1".into()
            }
            IdReason::DvIndexNotInt => "dv must be followed by a positive integer".into(),
            IdReason::DvIndexFromOne => "dv index starts at 1".into(),
            IdReason::SheetCfPrefix => {
                "conditional format id must be sheet:name/cf:n, n starting at 1".into()
            }
            IdReason::CfIndexNotInt => "cf must be followed by a positive integer".into(),
            IdReason::CfIndexFromOne => "cf index starts at 1".into(),
            IdReason::SheetTablePrefix => {
                "sheet table id must be sheet:name/table:n, n starting at 1".into()
            }
            IdReason::SheetPivotPrefix => {
                "pivot table id must be sheet:name/pivot:n, n starting at 1".into()
            }
            IdReason::PivotIndexNotInt => "pivot must be followed by a positive integer".into(),
            IdReason::PivotIndexFromOne => "pivot index starts at 1".into(),
            IdReason::SheetChartPrefix => {
                "xlsx chart id must be sheet:name/chart:…".into()
            }
            IdReason::SheetChartContainer => {
                "Sheet id must include /chart:… (that id alone is not a chart)".into()
            }
            IdReason::SlideChartPrefix => {
                "pptx chart id must be slide:N/chart:…, slide:N/notes/chart:…, master:N/chart:… or master:N/layout:K/chart:…".into()
            }
            IdReason::SlideChartContainer => {
                "Slide id must include /chart:… (that id alone is not a chart)".into()
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
            }
            IdReason::SlideSmartArtPrefix => {
                "pptx SmartArt id must be slide:N/smartdiagram:…, slide:N/notes/smartdiagram:…, master:N/smartdiagram:… or master:N/layout:K/smartdiagram:…".into()
            }
            IdReason::SlideSmartArtContainer => {
                "Slide id must include /smartdiagram:… (that id alone is not a SmartArt)".into()
            }
            IdReason::InsertSmartArtNeedsSlide => {
                "pptx insertSmartArt id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SlideTablePrefix => {
                "pptx table id must be slide:N/table:…, slide:N/notes/table:…, master:N/table:… or master:N/layout:K/table:…".into()
            }
            IdReason::SlideTableContainer => {
                "Slide id must include /table:… (that id alone is not a table)".into()
            }
            IdReason::SlideTableIndexNotInt => {
                "table:index must be followed by a positive integer".into()
            }
            IdReason::SlideTableIndexFromOne => "table:index starts at 1".into(),
            IdReason::SlideTableRowForm => "Row id must be a table id plus /row:R, R starting at 1".into(),
            IdReason::SlideTableCellForm => {
                "Cell id must be a table id plus /r:R/c:C, R and C starting at 1".into()
            }
            IdReason::InsertTableNeedsSlide => {
                "pptx insertTable id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SetTableCellNeedsCell => {
                "setTableCellText id must be a DrawingML table cell id".into()
            }
            IdReason::OlePrefix => {
                "OLE id must be a paragraph id plus /ole:…".into()
            }
            IdReason::OleContainer => {
                "Paragraph id must include /ole:… (that id alone is not an embedded object)".into()
            }
            IdReason::OleIndexNotInt => {
                "ole:index must be followed by a positive integer".into()
            }
            IdReason::OleIndexFromOne => "ole:index starts at 1".into(),
            IdReason::InsertOleNeedsPara => {
                "insertOle id must be a body or header/footer paragraph id".into()
            }
            IdReason::SlideOlePrefix => {
                "OLE id must be slide:N/ole:…, slide:N/notes/ole:…, master:N/ole:… or master:N/layout:K/ole:…".into()
            }
            IdReason::SlideOleContainer => {
                "Slide id must include /ole:… (that id alone is not an embedded object)".into()
            }
            IdReason::InsertOleNeedsSlide => {
                "pptx insertOle id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::InsertOleNeedsSheet => "xlsx insertOle id must be sheet:name".into(),
            IdReason::SheetOlePrefix => "xlsx OLE id must be sheet:name/ole:…".into(),
            IdReason::SheetOleContainer => {
                "Sheet id must include /ole:… (that id alone is not an embedded object)".into()
            }
            IdReason::InsertModel3dNeedsSheet => "xlsx insertModel3d id must be sheet:name".into(),
            IdReason::SheetModel3dPrefix => "xlsx 3D model id must be sheet:name/model3d:…".into(),
            IdReason::SheetModel3dContainer => {
                "Sheet id must include /model3d:… (that id alone is not a 3D model)".into()
            }
            IdReason::InsertModel3dNeedsSlide => {
                "pptx insertModel3d id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SlideModel3dPrefix => {
                "pptx 3D model id must be slide:N/model3d:…, slide:N/notes/model3d:…, master:N/model3d:… or master:N/layout:K/model3d:…".into()
            }
            IdReason::SlideModel3dContainer => {
                "Slide id must include /model3d:… (that id alone is not a 3D model)".into()
            }
            IdReason::InsertMathNeedsSheet => "xlsx insertMath id must be sheet:name".into(),
            IdReason::SheetMathPrefix => "xlsx math drawing id must be sheet:name/math:…".into(),
            IdReason::SheetMathContainer => {
                "Sheet id must include /math:… (that id alone is not a math drawing)".into()
            }
            IdReason::InsertMathNeedsSlide => {
                "pptx insertMath id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::SlideMathPrefix => {
                "pptx math drawing id must be slide:N/math:…, slide:N/notes/math:…, master:N/math:… or master:N/layout:K/math:…".into()
            }
            IdReason::SlideMathContainer => {
                "Slide id must include /math:… (that id alone is not a math drawing)".into()
            }
            IdReason::InsertSlicerNeedsSheet => "xlsx insertSlicer id must be sheet:name".into(),
            IdReason::SheetSlicerPrefix => "xlsx slicer id must be sheet:name/slicer:…".into(),
            IdReason::SheetSlicerContainer => {
                "Sheet id must include /slicer:… (that id alone is not a slicer)".into()
            }
            IdReason::SlicerIndexNotInt => {
                "slicer:index must be followed by a positive integer".into()
            }
            IdReason::SlicerIndexFromOne => "slicer:index starts at 1".into(),
            IdReason::InsertSlicerNeedsTableOrPivot => {
                "insertSlicer source must be a table or pivot id on the same sheet".into()
            }
            IdReason::InsertZoomNeedsSlide => {
                "insertZoom id must be slide:N, slide:N/notes, master:N or master:N/layout:K".into()
            }
            IdReason::ZoomTargetSlide => {
                "insertZoom target must be slide:N when type is slide".into()
            }
            IdReason::SlideZoomPrefix => {
                "Zoom id must be slide:N/zoom:…, slide:N/notes/zoom:…, master:N/zoom:… or master:N/layout:K/zoom:…".into()
            }
            IdReason::SlideZoomContainer => {
                "Slide id must include /zoom:… (that id alone is not a Zoom)".into()
            }
            IdReason::ZoomIndexNotInt => "zoom:index must be followed by a positive integer".into(),
            IdReason::ZoomIndexFromOne => "zoom:index starts at 1".into(),
            IdReason::InsertWatermarkNeedsDocument => {
                "insertWatermark id must be docx:document".into()
            }
            IdReason::WatermarkPrefix => {
                "Watermark id must be a header container id plus /watermark:…".into()
            }
            IdReason::WatermarkContainer => {
                "Header id must include /watermark:… (that id alone is not a watermark)".into()
            }
            IdReason::WatermarkIndexNotInt => {
                "watermark:index must be followed by a positive integer".into()
            }
            IdReason::WatermarkIndexFromOne => "watermark:index starts at 1".into(),
            IdReason::EvenOddNeedsDocument => {
                "setEvenAndOddHeaders id must be docx:document".into()
            }
            IdReason::SettingsNeedDocument => {
                "setMirrorMargins / setGutterAtTop / setProtection id must be docx:document".into()
            }
            IdReason::TablePrefix => {
                "Table id must be table:N or a header/footer container id plus /table:N".into()
            }
            IdReason::TableIndexNotInt => "table index must be a positive integer".into(),
            IdReason::TableIndexFromOne => "table index starts at 1".into(),
            IdReason::TableCellForm => {
                "Cell id must be table:N/rRcC; row id must be table:N/row:R".into()
            }
            IdReason::InsertTableAnchor => {
                "insertTable id must be a paragraph, document, header/footer, table, or cell".into()
            }
            IdReason::DeleteTableNeedsTable => "deleteTable id must be a table id".into(),
            IdReason::NeedTableId => "id must be a table id".into(),
            IdReason::InsertRowNeedsTableOrRow => {
                "insertRow id must be a table id or a row id".into()
            }
            IdReason::DeleteRowNeedsRow => "deleteRow id must be a row id".into(),
            IdReason::SplitNeedsCell => "splitCells id must be a cell id".into(),
            IdReason::RevPrefix => {
                "Revision id must be rev:N or a header/footer container id plus /rev:N".into()
            }
            IdReason::RevIndexNotInt => "revision index must be a positive integer".into(),
            IdReason::RevIndexFromOne => "revision index starts at 1".into(),
            IdReason::LinkPrefix => {
                "Hyperlink id must be link:N or a header/footer container id plus /link:N".into()
            }
            IdReason::LinkIndexNotInt => "hyperlink index must be a positive integer".into(),
            IdReason::LinkIndexFromOne => "hyperlink index starts at 1".into(),
            IdReason::InsertLinkNeedsPara => {
                "insertHyperlink id must be a body or header/footer paragraph id".into()
            }
            IdReason::SheetLinkPrefix => {
                "xlsx hyperlink id must be sheet:name/link:n, n starting at 1".into()
            }
            IdReason::InsertHyperlinkNeedsSheet => {
                "xlsx insertHyperlink id must be sheet:name".into()
            }
            IdReason::InsertHyperlinkNeedsShape => {
                "pptx insertHyperlink id must be a slide, notes, master or layout text shape, picture, table, chart, SmartArt or table cell id".into()
            }
            IdReason::SlideLinkPrefix => {
                "pptx hyperlink id must be slide:N/link:k, slide:N/notes/link:k, master:N/link:k or master:N/layout:K/link:k".into()
            }
            IdReason::PptxJumpNeedsSlide => {
                "pptx internal jump must be slide:n, n starting at 1".into()
            }
            IdReason::SheetSparkPrefix => {
                "xlsx sparkline id must be sheet:name/spark:n, n starting at 1".into()
            }
            IdReason::SparkIndexNotInt => "sparkline index must be a positive integer".into(),
            IdReason::SparkIndexFromOne => "sparkline index starts at 1".into(),
            IdReason::InsertSparklineNeedsSheet => {
                "xlsx insertSparkline id must be sheet:name".into()
            }
            IdReason::BookmarkPrefix => {
                "Bookmark id must be bookmark:N or a header/footer container id plus /bookmark:N"
                    .into()
            }
            IdReason::BookmarkIndexNotInt => "bookmark index must be a positive integer".into(),
            IdReason::BookmarkIndexFromOne => "bookmark index starts at 1".into(),
            IdReason::InsertBookmarkNeedsPara => {
                "insertBookmark id and end must be a body or header/footer paragraph id".into()
            }
            IdReason::FieldPrefix => {
                "Field id must be field:N or a header/footer container id plus /field:N".into()
            }
            IdReason::FieldIndexNotInt => "field index must be a positive integer".into(),
            IdReason::FieldIndexFromOne => "field index starts at 1".into(),
            IdReason::InsertFieldNeedsPara => {
                "insertField id must be a body or header/footer paragraph id".into()
            }
            IdReason::PptxFieldForm => {
                "pptx field id must be slide:N/fld:…, slide:N/notes/fld:…, master:N/fld:… or master:N/layout:K/fld:…".into()
            }
            IdReason::InsertFieldNeedsShape => {
                "pptx insertField id must be a slide, notes, master or layout text shape id".into()
            }
            IdReason::InsertNumberingNeedsDocument => {
                "insertNumbering id must be docx:document".into()
            }
            IdReason::NumPrefix => "Numbering instance id must be num:N".into(),
            IdReason::NumIndexNotInt => "num id must be a positive integer".into(),
            IdReason::NumIndexFromOne => "num id starts at 1".into(),
            IdReason::AbstractNumPrefix => "Abstract numbering id must be abstractNum:N".into(),
            IdReason::AbstractNumIndexNotInt => {
                "abstractNum id must be an integer".into()
            }
            IdReason::SetParaNumberingNeedsPara => {
                "setParaNumbering id must be a body or header/footer paragraph id".into()
            }
            IdReason::StylePrefix => "Style id must be style: plus a non-empty styleId".into(),
            IdReason::SetParaStyleNeedsPara => {
                "setParaStyle id must be a body or header/footer paragraph id".into()
            }
            IdReason::SetRunFormatNeedsRun => {
                "setRunFormat id must be a paragraph id plus /run:N".into()
            }
            IdReason::RunPrefix => "Run id must be a paragraph id plus /run:N".into(),
            IdReason::RunIndexNotInt => "Run index must be an integer".into(),
            IdReason::RunIndexFromOne => "Run index is 1-based".into(),
            IdReason::InsertSectionNeedsBodyPara => {
                "insertSection id must be a direct child paragraph of w:body, or docx:document"
                    .into()
            }
            IdReason::TocPrefix => "Table of contents id must be toc:N".into(),
            IdReason::TocIndexNotInt => "toc index must be a positive integer".into(),
            IdReason::TocIndexFromOne => "toc index starts at 1".into(),
            IdReason::InsertTocNeedsBodyPara => {
                "insertToc id must be a direct child paragraph of w:body, or docx:document".into()
            }
            IdReason::CaptionPrefix => "Caption id must be caption:N".into(),
            IdReason::CaptionIndexNotInt => "caption index must be a positive integer".into(),
            IdReason::CaptionIndexFromOne => "caption index starts at 1".into(),
            IdReason::InsertCaptionNeedsBodyPara => {
                "insertCaption id must be a direct child paragraph of w:body, or docx:document"
                    .into()
            }
            IdReason::XrefPrefix => "Cross-reference id must be xref:N".into(),
            IdReason::XrefIndexNotInt => "xref index must be a positive integer".into(),
            IdReason::XrefIndexFromOne => "xref index starts at 1".into(),
            IdReason::InsertCrossRefNeedsPara => {
                "insertCrossRef id must be a body or header/footer paragraph id".into()
            }
            IdReason::IndexPrefix => "Index id must be index:N".into(),
            IdReason::IndexIndexNotInt => "index index must be a positive integer".into(),
            IdReason::IndexIndexFromOne => "index index starts at 1".into(),
            IdReason::InsertIndexNeedsBodyPara => {
                "insertIndex id must be a direct child paragraph of w:body, or docx:document"
                    .into()
            }
            IdReason::XePrefix => "Index entry id must be xe:N".into(),
            IdReason::XeIndexNotInt => "xe index must be a positive integer".into(),
            IdReason::XeIndexFromOne => "xe index starts at 1".into(),
            IdReason::InsertIndexEntryNeedsPara => {
                "insertIndexEntry id must be a body or header/footer paragraph id".into()
            }
            IdReason::SourcePrefix => "Bibliography source id must be source:N".into(),
            IdReason::SourceIndexNotInt => "source index must be a positive integer".into(),
            IdReason::SourceIndexFromOne => "source index starts at 1".into(),
            IdReason::InsertSourceNeedsDocument => {
                "insertSource id must be docx:document".into()
            }
            IdReason::CitePrefix => "Citation id must be cite:N".into(),
            IdReason::CiteIndexNotInt => "cite index must be a positive integer".into(),
            IdReason::CiteIndexFromOne => "cite index starts at 1".into(),
            IdReason::InsertCitationNeedsPara => {
                "insertCitation id must be a body or header/footer paragraph id".into()
            }
            IdReason::BiblioPrefix => "Bibliography id must be biblio:N".into(),
            IdReason::BiblioIndexNotInt => "biblio index must be a positive integer".into(),
            IdReason::BiblioIndexFromOne => "biblio index starts at 1".into(),
            IdReason::InsertBibliographyNeedsBodyPara => {
                "insertBibliography id must be a direct child paragraph of w:body, or docx:document"
                    .into()
            }
            IdReason::DocPartPrefix => "Building block id must be docPart:N".into(),
            IdReason::DocPartIndexNotInt => "docPart index must be a positive integer".into(),
            IdReason::DocPartIndexFromOne => "docPart index starts at 1".into(),
            IdReason::InsertDocPartNeedsDocument => {
                "insertDocPart id must be docx:document".into()
            }
            IdReason::InsertDocPartContentNeedsBodyPara => {
                "insertDocPartContent id must be a direct child paragraph of w:body, or docx:document"
                    .into()
            }
            IdReason::FontPrefix => "Embedded font id must be font:N".into(),
            IdReason::FontIndexNotInt => "font index must be a positive integer".into(),
            IdReason::FontIndexFromOne => "font index starts at 1".into(),
            IdReason::InsertEmbeddedFontNeedsDocument => {
                "insertEmbeddedFont id must be docx:document".into()
            }
            IdReason::SetFontEmbedNeedsDocument => {
                "setFontEmbed id must be docx:document".into()
            }
            IdReason::SetPageBackgroundNeedsDocument => {
                "setPageBackground id must be docx:document".into()
            }
            IdReason::MathPrefix => {
                "Math id must be math:N or a header/footer container id plus /math:N".into()
            }
            IdReason::MathIndexNotInt => "math index must be a positive integer".into(),
            IdReason::MathIndexFromOne => "math index starts at 1".into(),
            IdReason::InsertMathNeedsPara => {
                "insertMath id must be a body or header/footer paragraph id, or docx:document"
                    .into()
            }
            IdReason::SdtPrefix => {
                "Content control id must be sdt:N or a header/footer container id plus /sdt:N"
                    .into()
            }
            IdReason::SdtIndexNotInt => "sdt index must be a positive integer".into(),
            IdReason::SdtIndexFromOne => "sdt index starts at 1".into(),
            IdReason::InsertSdtNeedsPara => {
                "insertContentControl id must be a body or header/footer paragraph id, or docx:document"
                    .into()
            }
            IdReason::SetPropsNeedsRoot => {
                "setProps / setCustomProp id must be xlsx:workbook, docx:document, or pptx:presentation"
                    .into()
            }
            IdReason::CustomPropPrefix => "deleteCustomProp id must be prop:NAME".into(),
            IdReason::AnimForm => {
                "anim id must be slide:N/anim:K, slide:N/notes/anim:K, master:N/anim:K, master:N/layout:K/anim:M, notesMaster/anim:K, or handout/anim:K; insertAnim id must be a bindable object".into()
            }
            IdReason::AnimIndexNotInt => "anim index must be a positive integer".into(),
            IdReason::AnimIndexFromOne => "anim index is 1-based".into(),
            IdReason::InsertAnimNeedsHost => {
                "insertAnim id must be a slide or notes shape, picture, table, chart, SmartArt, video, or audio, or a master/layout, notesMaster, or handout placeholder".into()
            }
        },
        ErrorKind::SheetNotFound { name } => format!("Sheet not found: {name}"),
        ErrorKind::ParaNotFound { id } => format!("Paragraph not found: {id}"),
        ErrorKind::TableNotFound { id } => format!("Table not found: {id}"),
        ErrorKind::SectionNotFound { id } => format!("Section not found: {id}"),
        ErrorKind::CustomShowNotFound { id } => format!("Custom show not found: {id}"),
        ErrorKind::SlideNotFound { n } => format!("Slide not found: slide:{n}"),
        ErrorKind::MasterNotFound { n } => format!("Slide master not found: master:{n}"),
        ErrorKind::ThemeNotFound { n } => format!("Theme not found for slide master: master:{n}"),
        ErrorKind::DocumentThemeNotFound => "Document theme part not found".into(),
        ErrorKind::LayoutNotFound { master, layout } => {
            format!("Slide layout not found: master:{master}/layout:{layout}")
        }
        ErrorKind::ShapeNotFound { id } => format!("Shape not found: {id}"),
        ErrorKind::AnimNotFound { id } => format!("Animation not found: {id}"),
        ErrorKind::AnimHostNotFound { id } => {
            format!("No animatable object found: {id}")
        }
        ErrorKind::ShapeHasNoText { id } => format!("Shape has no text box: {id}"),
        ErrorKind::PictureNotFound { id } => format!("Picture not found: {id}"),
        ErrorKind::MediaNotFound { id } => format!("Video or audio not found: {id}"),
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
        ErrorKind::LastStoryPara => {
            "Cannot delete the last paragraph in the document body, a header, a footer, or a table cell".into()
        }
        ErrorKind::CrossParaMarker => {
            "Cannot delete a paragraph whose bookmark, comment, or permission range continues outside it".into()
        }
        ErrorKind::ParaHasRevision => {
            "Cannot delete a paragraph that contains revision markup".into()
        }
        ErrorKind::SectionBreakPara => {
            "Cannot delete a paragraph that carries a section break; move section properties first".into()
        }
        ErrorKind::CannotDeleteLastSection => {
            "Cannot delete the last section; a document must keep at least one".into()
        }
        ErrorKind::InvalidSectionType => {
            "section type must be nextPage, continuous, evenPage, oddPage, or nextColumn".into()
        }
        ErrorKind::SectionTypeOnLastPara => {
            "the last paragraph must not have sectionType; the last section stays on w:body".into()
        }
        ErrorKind::InvalidTocKind => {
            "kind must be toc, tof, or toa; instr must start with TOC or TOA".into()
        }
        ErrorKind::InvalidSeqIdentifier => {
            "label must be a SEQ identifier: a Latin letter followed by up to 39 Latin letters, digits, or underscores".into()
        }
        ErrorKind::InvalidCrossRefKind => "kind must be ref or page".into(),
        ErrorKind::InvalidIndexInstr => "instr must start with INDEX".into(),
        ErrorKind::InvalidIndexEntry => {
            "entry must be non-empty, must not be only whitespace, and must not contain a double quote"
                .into()
        }
        ErrorKind::InvalidSourceTag => {
            "tag must be non-empty, must not contain whitespace, and must not contain a double quote"
                .into()
        }
        ErrorKind::DuplicateSourceTag => "tag must be unique among bibliography sources".into(),
        ErrorKind::InvalidSourceType => "sourceType is not a supported ST_SourceType value".into(),
        ErrorKind::EmptySourceTitle => "title must not be empty".into(),
        ErrorKind::InvalidCitationTag => {
            "tag must be non-empty, must not contain whitespace, and must not contain a double quote"
                .into()
        }
        ErrorKind::InvalidBibliographyInstr => "instr must start with BIBLIOGRAPHY".into(),
        ErrorKind::EmptyDocPartName => "name must not be empty".into(),
        ErrorKind::DuplicateDocPartName => {
            "name, gallery, and category must be unique among document parts".into()
        }
        ErrorKind::InvalidDocPartGallery => {
            "gallery is not a supported ST_DocPartGallery value".into()
        }
        ErrorKind::InvalidDocPartBehavior => "behavior must be content, p, or pg".into(),
        ErrorKind::EmptyDocPartText => "text must not be empty".into(),
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
            format!("Ink comments cannot set text, receive replies, or accept drawing inserts: {id}")
        }
        ErrorKind::ChartNotFound { id } => format!("Chart not found: {id}"),
        ErrorKind::ChartSeriesNotFound { id } => format!("Chart series not found: {id}"),
        ErrorKind::TrendlineNotFound { id } => format!("Trendline not found: {id}"),
        ErrorKind::ErrorBarsNotFound { id } => format!("Error bars not found: {id}"),
        ErrorKind::NeedChartSeriesField => {
            "setChartSeries needs at least one of name, categories, values, or sizes".into()
        }
        ErrorKind::NeedTrendlineField => {
            "setTrendline needs at least one of type, order, period, displayRSquared, displayEquation, or name".into()
        }
        ErrorKind::NeedErrorBarsField => {
            "setErrorBars needs at least one of dir, barType, valType, or value".into()
        }
        ErrorKind::InvalidTrendlineType => {
            "type must be linear, exp, log, poly, power, or movingAvg".into()
        }
        ErrorKind::TrendlineOrderNotPoly => "order is only valid when type is poly".into(),
        ErrorKind::TrendlinePeriodNotMovingAvg => {
            "period is only valid when type is movingAvg".into()
        }
        ErrorKind::TrendlineNotSupported => {
            "This chart type does not support trendlines".into()
        }
        ErrorKind::InvalidErrorBars => {
            "dir must be x or y; barType must be both, plus, or minus; valType must be stdErr, stdDev, fixedVal, percentage, or cust".into()
        }
        ErrorKind::CustomErrorBars => {
            "valType cust needs plus and minus arrays".into()
        }
        ErrorKind::NeedErrorBarsValue => {
            "value is required when valType is stdDev, fixedVal, or percentage".into()
        }
        ErrorKind::ErrorBarsValueOnStdErr => {
            "value is not allowed when valType is stdErr or cust".into()
        }
        ErrorKind::ErrorBarsNotSupported => {
            "This chart type does not support error bars".into()
        }
        ErrorKind::ErrorBarsDuplicateDir => {
            "This series already has error bars in that direction".into()
        }
        ErrorKind::SecondaryAxisNotSupported => {
            "This chart type does not support a secondary axis".into()
        }
        ErrorKind::InvalidSeriesAxis => "axis must be primary or secondary".into(),
        ErrorKind::LastChartSeries => {
            "Cannot delete the last series in a chart; use deleteChart to remove the chart".into()
        }
        ErrorKind::RestrictedChart { id } => {
            format!("This chart cannot be edited with this operation: {id}")
        }
        ErrorKind::InvalidChartType => {
            "type must be bar, col, line, pie, area, scatter, doughnut, radar, bubble, stock, surface, or ofPie; insertChart also allows combo or chartex".into()
        }
        ErrorKind::InvalidTransitionType => {
            "type must be a writable slide transition, or null to clear".into()
        }
        ErrorKind::InvalidAnimType => {
            "type must be appear, fade, flyIn, wipe, split, zoom, spin, growShrink, fadeOut, flyOut, wipeOut, or pathRight".into()
        }
        ErrorKind::InvalidAnimTrigger => "trigger must be click, with, after, or clickOn".into(),
        ErrorKind::InvalidAnimDelay => "delay must be an integer from 0 to 60000".into(),
        ErrorKind::InvalidAnimDur => "dur must be an integer from 1 to 60000".into(),
        ErrorKind::InvalidAnimRepeat => "repeat must be an integer from 1 to 99, or null".into(),
        ErrorKind::InvalidAnimPara => "para must be a 1-based paragraph index on the target".into(),
        ErrorKind::InvalidAnimMoveIndex => "index must be a 1-based position in the animation list".into(),
        ErrorKind::NeedAnimSetField => {
            "setAnim requires at least one writable field".into()
        }
        ErrorKind::AnimNeedEffectDur => "dur can only be set on an effect that has a timed behavior".into(),
        ErrorKind::InvalidTransitionDur => "dur must be an integer from 1 to 60000".into(),
        ErrorKind::InvalidTransitionAdvTm => {
            "advTm must be an integer from 1 to 86400000".into()
        }
        ErrorKind::InvalidTransitionDir => {
            "dir must be allowed for this transition; invX / invY are only valid on prstTrans presets".into()
        }
        ErrorKind::InvalidTransitionOrient => {
            "orient is only valid on split and must be horz or vert".into()
        }
        ErrorKind::InvalidTransitionSpokes => {
            "spokes is only valid on wheel / wheelReverse and must be an integer from 1 to 8".into()
        }
        ErrorKind::InvalidShapePrst => {
            "prst must be a writable preset geometry; use insertConnector for connector geometries".into()
        }
        ErrorKind::InvalidGeomOp => {
            "path commands may only be moveTo, lnTo, or close".into()
        }
        ErrorKind::ConnectorEndHost => {
            "start / end must be on the same host as the connector".into()
        }
        ErrorKind::NeedConnectorBind => {
            "setConnector requires start or end".into()
        }
        ErrorKind::InvalidConnectorPrst => {
            "prst must be a writable connector geometry".into()
        }
        ErrorKind::GroupNeedsTwo => "groupShapes ids must contain at least two members".into(),
        ErrorKind::GroupMemberNested => {
            "groupShapes members must be ungrouped siblings; nested groups cannot be grouped again".into()
        }
        ErrorKind::NotGrouped => "this object is not inside a group".into(),
        ErrorKind::ConnectorNotFound { id } => format!("connector not found: {id}"),
        ErrorKind::InvalidChartAxis => {
            "axis must be cat or val, and the chart must have that primary axis".into()
        }
        ErrorKind::InvalidLegendPos => "pos must be b, t, l, r, or tr".into(),
        ErrorKind::PieNeedsOneSeries => {
            "A pie, doughnut, or of-pie chart must have exactly one series".into()
        }
        ErrorKind::StockNeedsThreeSeries => {
            "A stock chart series count must match stockKind: 3 for hlc, 4 for ohlc or vhlc, 5 for vohlc".into()
        }
        ErrorKind::SurfaceNeedsThreeSeries => {
            "A surface chart must have exactly three series".into()
        }
        ErrorKind::InvalidOfPieType => {
            "ofPieType must be pie or bar, and is only valid when type is ofPie".into()
        }
        ErrorKind::InvalidStockKind => {
            "stockKind must be hlc, ohlc, vhlc, or vohlc, and is only valid when type is stock; setChartType cannot use vhlc or vohlc".into()
        }
        ErrorKind::InvalidSurfaceKind => {
            "surfaceKind must be surface3d, wireframe, contour, or contourWire, and is only valid when type is surface".into()
        }
        ErrorKind::InvalidLayoutId => {
            "layoutId must be waterfall, treemap, sunburst, funnel, clusteredColumn, boxWhisker, or pareto, and is only valid when type is chartex".into()
        }
        ErrorKind::RegionMapChart => "regionMap charts are not implemented".into(),
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
        ErrorKind::OleNotFound { id } => format!("Embedded object not found: {id}"),
        ErrorKind::InkNotFound { id } => format!("Ink canvas not found: {id}"),
        ErrorKind::Model3dNotFound { id } => format!("3D model not found: {id}"),
        ErrorKind::MathDrawingNotFound { id } => format!("math drawing not found: {id}"),
        ErrorKind::MathDrawingNoExpr => {
            "xlsx / pptx math drawings only accept non-empty text; do not give expr or kind"
                .into()
        }
        ErrorKind::SlicerNotFound { id } => format!("Slicer not found: {id}"),
        ErrorKind::SlicerFieldNotFound { name } => {
            format!("Slicer field does not match the source: {name}")
        }
        ErrorKind::InvalidSlicer { field } => format!("invalid slicer field: {field}"),
        ErrorKind::ConnectionNotFound { id } => format!("Connection not found: {id}"),
        ErrorKind::InvalidConnection { field } => {
            format!("{field} is not a valid connection value")
        }
        ErrorKind::ExternalLinkNotFound { id } => format!("External link not found: {id}"),
        ErrorKind::InvalidExternalLink { field } => {
            format!("{field} is not a valid external link value")
        }
        ErrorKind::ExternalLinkInUse => {
            "A formula or defined name still uses this external link".into()
        }
        ErrorKind::CustomViewNotFound { id } => format!("Custom view not found: {id}"),
        ErrorKind::InvalidCustomView { field } => {
            format!("{field} is not a valid custom view value")
        }
        ErrorKind::ZoomNotFound { id } => format!("Zoom not found: {id}"),
        ErrorKind::InvalidZoomType => "type must be slide, section, or summary".into(),
        ErrorKind::InvalidShowPrType => "type must be present, browse, or kiosk".into(),
        ErrorKind::InvalidFldType => {
            "type must be slidenum, datetime, or datetime1–datetime13".into()
        }
        ErrorKind::ZoomTargetSelf => "insertZoom target cannot be the host slide".into(),
        ErrorKind::NeedZoomSetField => {
            "setZoom requires at least one of target, targets, or file".into()
        }
        ErrorKind::InvalidZoomTargets => {
            "target and targets must match the Zoom kind, and targets cannot be empty or duplicated"
                .into()
        }
        ErrorKind::WatermarkNotFound { id } => format!("Watermark not found: {id}"),
        ErrorKind::WatermarkNeedTextOrFile => {
            "insertWatermark requires exactly one of text or file".into()
        }
        ErrorKind::SmartArtPointNotFound { id } => format!("SmartArt point not found: {id}"),
        ErrorKind::LastSmartArtPoint => {
            "Cannot delete the last visible SmartArt point; use deleteSmartArt to remove the diagram".into()
        }
        ErrorKind::InvalidSmartArtLayout => "layout must be list, org, or cycle".into(),
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
        ErrorKind::SmartArtAsstOnList => "list / cycle layout does not accept asst points".into(),
        ErrorKind::SmartArtListDepth => {
            "list / cycle layout only accepts top-level points (parent must be omitted or null)".into()
        }
        ErrorKind::TextBoxLinkCrossPart => {
            "Text boxes in a link chain must be in the same part".into()
        }
        ErrorKind::InvalidHexColor => "color must be a 6-digit RGB hex value or null".into(),
        ErrorKind::NeedShapeFormatField => {
            "setShapeFormat requires at least one of sz, color, textFill, textOutline, bold, italic, underline, font, eastAsiaFont, align, or textAnchor".into()
        }
        ErrorKind::InvalidTextFillType => "textFill.type must be none or gradient".into(),
        ErrorKind::NeedGradientStops => "textFill.stops must have at least two items".into(),
        ErrorKind::DuplicateGradientPos => "textFill.stops pos values must be unique".into(),
        ErrorKind::NeedTextOutlineField => "textOutline requires color and width".into(),
        ErrorKind::InvalidTextAlign => "align must be l, ctr, r, or just".into(),
        ErrorKind::InvalidTextAnchor => "textAnchor must be t, ctr, or b".into(),
        ErrorKind::InvalidTextUnderline => "underline must be sng, none, or dbl".into(),
        ErrorKind::EmptyShapeTypeface => "font and eastAsiaFont must not be empty".into(),
        ErrorKind::InvalidTextBoxDirection => "direction must be horizontal or vertical".into(),
        ErrorKind::InvalidPicKind => "kind must be inline or anchor".into(),
        ErrorKind::InvalidPicWrap => {
            "wrap must be square, tight, through, topAndBottom, or none".into()
        }
        ErrorKind::InvalidPicAlign => "alignH must be left/center/right; alignV must be top/center/bottom".into(),
        ErrorKind::InvalidImageFile => {
            "file is not a recognized image (PNG, JPEG, GIF, BMP, SVG, EMF, WMF, or TIFF)".into()
        }
        ErrorKind::InvalidInkMl => {
            "file is not InkML (root element must be ink in the InkML namespace)".into()
        }
        ErrorKind::InvalidMediaFile => {
            "file is not a recognized media clip (insertVideo accepts MP4, insertAudio accepts WAV, subtitle tracks accept WebVTT)"
                .into()
        }
        ErrorKind::NeedSetMediaField => {
            "setMedia requires at least one of st, end, bookmarks, or tracks".into()
        }
        ErrorKind::InvalidMediaTime => {
            "st, end, and bookmark time must be non-negative numbers".into()
        }
        ErrorKind::DuplicateBookmarkName => {
            "bookmark names on the same media object must be unique".into()
        }
        ErrorKind::InvalidTrack => {
            "media bookmark or subtitle track name / label / id / trackDisplayLoc is invalid".into()
        }
        ErrorKind::NeedsPptxSectionList => "the presentation has no slide section list".into(),
        ErrorKind::SlideNotInPptxSection => "that slide is not in any section".into(),
        ErrorKind::SetCellWorksheetOnly => "setCell only applies to worksheets".into(),
        ErrorKind::InspectWorksheetOnly => "Cells can only be read from worksheets".into(),
        ErrorKind::InsertPictureWorksheetOnly => "insertPicture only applies to worksheets".into(),
        ErrorKind::InsertChartWorksheetOnly => "insertChart only applies to worksheets".into(),
        ErrorKind::InsertTextBoxWorksheetOnly => {
            "insertTextBox, insertShape, insertConnector and related ops only apply to worksheets"
                .into()
        }
        ErrorKind::InsertModel3dWorksheetOnly => {
            "insertModel3d only applies to worksheets".into()
        }
        ErrorKind::InsertMathWorksheetOnly => "insertMath only applies to worksheets".into(),
        ErrorKind::InsertSmartArtWorksheetOnly => {
            "insertSmartArt and SmartArt ops only apply to worksheets".into()
        }
        ErrorKind::CommentsWorksheetOnly => "comment ops only apply to worksheets".into(),
        ErrorKind::InsertHyperlinkWorksheetOnly => {
            "hyperlink ops only apply to worksheets".into()
        }
        ErrorKind::InsertSparklineWorksheetOnly => {
            "sparkline ops only apply to worksheets".into()
        }
        ErrorKind::DataSheetMustBeWorksheet => {
            "chart sheet data must be written to a worksheet".into()
        }
        ErrorKind::TwoCellPictureSize => {
            "twoCell picture size is defined by the to cell; give to on setPictureSize, not cx / cy"
                .into()
        }
        ErrorKind::TwoCellTextBoxSize => {
            "twoCell text box, shape, or connector size is defined by the to cell; give to on setTextBoxSize, not cx / cy"
                .into()
        }
        ErrorKind::TwoCellSmartArtSize => {
            "twoCell SmartArt size is defined by the to cell; give to on setSmartArtSize, not cx / cy"
                .into()
        }
        ErrorKind::TwoCellChartSize => {
            "twoCell chart size is defined by the to cell; give to on setChartSize, not cx / cy"
                .into()
        }
        ErrorKind::DeleteChartsheetLast => {
            "cannot delete the last chart on a chartsheet; that would leave an empty chartsheet"
                .into()
        }
        ErrorKind::ChartFormulaNotA1 => {
            "series formula is not a contiguous A1 range (named ranges, 3-D references, and non-contiguous ranges are not written)"
                .into()
        }
        ErrorKind::InvalidXlsxAnchor => "anchor must be oneCell or twoCell".into(),
        ErrorKind::InvalidXlsxEditAs => "editAs must be twoCell, oneCell, or absolute".into(),
        ErrorKind::NotesSlideImage => {
            "Cannot edit a placeholder whose type is sldImg".into()
        }
        ErrorKind::GraphicInRestrictedStory => {
            "Cannot place a graphic object in a text box, callout, comment, footnote, or endnote"
                .into()
        }
        ErrorKind::NeedPropsField => {
            "setProps requires at least one of title, subject, creator, description, keywords, category, company, manager"
                .into()
        }
        ErrorKind::NeedProtectionField => {
            "setProtection requires at least one writable field".into()
        }
        ErrorKind::InvalidProtectionEdit => {
            "edit must be readOnly, comments, trackedChanges, or forms".into()
        }
        ErrorKind::InvalidSheetProtection { field } => {
            format!("chartsheet does not accept {field}")
        }
        ErrorKind::ProtectionPassword => {
            "setProtection does not write password hashes".into()
        }
        ErrorKind::ProtectionHasPassword => {
            "this document has a password hash; protection fields cannot be changed".into()
        }
        ErrorKind::ProtectionPasswordBlank => "password must be a non-empty string".into(),
        ErrorKind::ProtectionPasswordWrong => {
            "password does not match the existing SHA-512 hash".into()
        }
        ErrorKind::ProtectionPasswordNeedsDoc => {
            "password can only be used with document editing protection, not readOnlyRecommended alone"
                .into()
        }
        ErrorKind::InvalidCustomPropType => {
            "custom property type must be lpwstr, i4, r8, or bool, and must match value"
                .into()
        }
        ErrorKind::EmptyCustomPropName => "custom property name must be a non-empty string".into(),
        ErrorKind::CustomPropNotFound { id } => format!("custom property not found: {id}"),
        ErrorKind::SharedFormulaMaster { cell } => format!(
            "{cell} is a shared formula master with dependents; mergeRange cannot cover it"
        ),
        ErrorKind::DataTableFormula { cell } => format!(
            "{cell} is part of a data table formula, which this version does not change"
        ),
        ErrorKind::ArrayFormulaPart { cell } => format!(
            "{cell} is inside an array formula; change the top-left master cell instead"
        ),
        ErrorKind::FormulaGroupOverlap { cell } => format!(
            "{cell} already belongs to another shared, array, or data table formula"
        ),
        ErrorKind::FormulaRangeNotTopLeft => {
            "id must be the top-left cell of sharedRef or arrayRef".into()
        }
        ErrorKind::SharedNeedsRef => {
            "shared: true on a cell that is not already a shared master requires sharedRef".into()
        }
        ErrorKind::NeedAtLeastOneSheet => "At least one worksheet is required".into(),
        ErrorKind::LastSlide => "At least one slide is required".into(),
        ErrorKind::LastLayout => "The master must keep at least one layout".into(),
        ErrorKind::LastMaster => "At least one slide master is required".into(),
        ErrorKind::LayoutInUse => "A slide still uses this layout".into(),
        ErrorKind::MasterInUse => "A slide still uses a layout of this master".into(),
        ErrorKind::InvalidPlaceholderType => {
            "type must be a writable placeholder type".into()
        }
        ErrorKind::InvalidSlideSizeType => {
            "type must be a writable slide size type".into()
        }
        ErrorKind::SlideSizeOutOfRange => {
            "cx / cy must be an integer from 914400 to 51206400".into()
        }
        ErrorKind::LastVisibleSheet => {
            "Cannot hide or delete the last visible sheet".into()
        }
        ErrorKind::MoveSheetAfterSelf => {
            "moveSheet after cannot be the same sheet as id".into()
        }
        ErrorKind::MoveSlideAfterSelf => {
            "moveSlide after cannot be the same slide as id".into()
        }
        ErrorKind::SlideLayoutMissing => "No slide layout is available".into(),
        ErrorKind::SheetStillReferenced { name } => {
            format!("Defined names or formulas still refer to sheet {name}")
        }
        ErrorKind::DialogOrMacroSheet => {
            "dialogsheet and macrosheet cannot be deleted, renamed, moved, or hidden".into()
        }
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
        ErrorKind::CreateHfDuplicate => "create allows each header/footer type only once per side".into(),
        ErrorKind::InvalidXlsxHfKind => "kind must be odd, even, or first".into(),
        ErrorKind::InvalidXlsxHfSide => "side must be header or footer".into(),
        ErrorKind::CreateXlsxHfDuplicate => {
            "create allows each headerFooters kind and side only once".into()
        }
        ErrorKind::NeedXlsxHfContent => {
            "headerFooters item needs non-empty text or at least one picture".into()
        }
        ErrorKind::CannotLinkFirstSection => {
            "The first section cannot link to a previous section".into()
        }
        ErrorKind::HfTypeInactive { kind, hf_type } => {
            format!("{kind} type {hf_type} is not active on this section")
        }
        ErrorKind::HfInsertWhileLinked => {
            "This section inherits that header/footer; use setHeaderFooterLink to unlink".into()
        }
        ErrorKind::InvalidSectValue { field } => format!("{field} is not an allowed value"),
        ErrorKind::LastTableRow => "Cannot delete the last row in a table".into(),
        ErrorKind::LastTableCol => "Cannot delete the last column in a table".into(),
        ErrorKind::SplitContinueCell => {
            "splitCells must target the restart cell, not a continue cell".into()
        }
        ErrorKind::NothingToSplit => "This cell is not merged".into(),
        ErrorKind::TableHasRevision => {
            "Cannot change a table that has revision marks on the table, row, or cell".into()
        }
        ErrorKind::TableGridMismatch => {
            "Cannot change a table whose row grid spans do not match colCount".into()
        }
        ErrorKind::MergeNotSimple => {
            "mergeCells needs a rectangle of unmerged cells that line up on the grid".into()
        }
        ErrorKind::NeedTableCellsMatch => {
            "cells must be a rows×cols array of strings".into()
        },
        ErrorKind::RevisionNotFound { id } => format!("Revision not found: {id}"),
        ErrorKind::SetParaTextHasRevision => {
            "Cannot change visible text in a paragraph that contains tracked revisions".into()
        }
        ErrorKind::HyperlinkNotFound { id } => format!("Hyperlink not found: {id}"),
        ErrorKind::BookmarkNotFound { id } => format!("Bookmark not found: {id}"),
        ErrorKind::SetParaTextHasHyperlink => {
            "Cannot change visible text in a paragraph that contains a hyperlink".into()
        }
        ErrorKind::HyperlinkNeedTarget => "exactly one of url or anchor is required".into(),
        ErrorKind::XlsxHyperlinkNeedTarget => {
            "xlsx insertHyperlink and setHyperlink require at least one of url or location".into()
        }
        ErrorKind::EmptyHyperlinkUrl => "url must not be empty".into(),
        ErrorKind::EmptyHyperlinkLocation => "location must not be empty".into(),
        ErrorKind::HyperlinkOverlap => {
            "hyperlink ref overlaps another hyperlink on the same worksheet".into()
        }
        ErrorKind::SparklineNotFound { id } => format!("Sparkline group not found: {id}"),
        ErrorKind::SparklineOverlap => {
            "sparkline host cell overlaps another sparkline on the same worksheet".into()
        }
        ErrorKind::SparklineHostNotCell => "sparkline ref must be a single cell".into(),
        ErrorKind::SparklineFormulaNotLine => {
            "sparkline formula must be a single row or a single column".into()
        }
        ErrorKind::SparklineGroupHasMany => {
            "setSparkline ref and formula require a group with exactly one sparkline".into()
        }
        ErrorKind::EmptySparklineFormula => "formula must not be empty".into(),
        ErrorKind::InvalidSparklineType => "type must be line, column, or stacked".into(),
        ErrorKind::InvalidSparkline { field } => {
            format!("invalid sparkline field: {field}")
        }
        ErrorKind::NeedSetSparklineField => {
            "setSparkline requires at least one field besides op and id".into()
        }
        ErrorKind::InvalidBookmarkName => {
            "bookmark name must not be empty, contain whitespace, or start with an underscore"
                .into()
        }
        ErrorKind::BookmarkNameTaken => "bookmark name is already taken".into(),
        ErrorKind::CustomShowNameTooLong => {
            "custom show name must be at most 31 characters".into()
        }
        ErrorKind::CustomShowNameTaken => "custom show name is already taken".into(),
        ErrorKind::NeedCustomShowField => {
            "setCustomShow requires at least one of name or slides".into()
        }
        ErrorKind::BookmarkRangeReversed => {
            "insertBookmark end must not precede the start paragraph".into()
        }
        ErrorKind::BookmarkRangeCrossStory => {
            "insertBookmark end must be in the same story as id".into()
        }
        ErrorKind::NeedSetHyperlinkField => {
            "setHyperlink requires at least one of url, anchor, text, or tooltip".into()
        }
        ErrorKind::NeedXlsxSetHyperlinkField => {
            "xlsx setHyperlink requires at least one of url, location, tooltip, display, or ref"
                .into()
        }
        ErrorKind::PptxHyperlinkNeedTarget => {
            "pptx hyperlink requires exactly one of url, file, or slide".into()
        }
        ErrorKind::PptxNeedSetHyperlinkField => {
            "pptx setHyperlink requires at least one of url, file, slide, or tooltip".into()
        }
        ErrorKind::ShapeAlreadyHasHyperlink => {
            "this object already has a box-level hyperlink for this event; use setHyperlink".into()
        }
        ErrorKind::InvalidHyperlinkOn => "on must match the id kind".into(),
        ErrorKind::InvalidHyperlinkEvent => "event must be click or hover".into(),
        ErrorKind::RunAlreadyHasHyperlink => {
            "this run already has a hyperlink for this event; use setHyperlink".into()
        }
        ErrorKind::PptxRunLinkNeedsRun => {
            "on=run needs an existing text run or a text field".into()
        }
        ErrorKind::PptxHyperlinkSelfSlide => {
            "cannot hyperlink a shape to its own slide".into()
        }
        ErrorKind::EmptyHyperlinkFile => "file must not be empty".into(),
        ErrorKind::FieldNotFound { id } => format!("Field not found: {id}"),
        ErrorKind::TocNotFound { id } => format!("Table of contents not found: {id}"),
        ErrorKind::CaptionNotFound { id } => format!("Caption not found: {id}"),
        ErrorKind::CrossRefNotFound { id } => format!("Cross-reference not found: {id}"),
        ErrorKind::IndexNotFound { id } => format!("Index not found: {id}"),
        ErrorKind::IndexEntryNotFound { id } => format!("Index entry not found: {id}"),
        ErrorKind::SourceNotFound { id } => format!("Bibliography source not found: {id}"),
        ErrorKind::CitationNotFound { id } => format!("Citation not found: {id}"),
        ErrorKind::BibliographyNotFound { id } => format!("Bibliography not found: {id}"),
        ErrorKind::DocPartNotFound { id } => format!("Building block not found: {id}"),
        ErrorKind::FontNotFound { id } => format!("Embedded font not found: {id}"),
        ErrorKind::EmptyFontName => "name must not be empty".into(),
        ErrorKind::DuplicateEmbeddedFont => {
            "name and style already have an embedded face".into()
        }
        ErrorKind::InvalidFontStyle => {
            "style must be regular, bold, italic, or boldItalic".into()
        }
        ErrorKind::InvalidFontFile => {
            "file is not a recognized TrueType or OpenType font (TTF/OTF); font collections are not allowed".into()
        }
        ErrorKind::NeedFontEmbedField => {
            "setFontEmbed requires at least one of trueType, systemFonts, or subset".into()
        }
        ErrorKind::InvalidPageBackgroundColor => {
            "color must be a 6-digit RGB hex value, or null".into()
        }
        ErrorKind::NeedPageBordersField => {
            "setPageBorders requires enabled, offsetFrom, display, zOrder, or at least one side".into()
        }
        ErrorKind::InvalidPageBorderVal => {
            "val must be nil, none, single, dashed, dotted, double, thick, wave, dashSmallGap, dotDash, or dotDotDash".into()
        }
        ErrorKind::InvalidPageBorderOffset => "offsetFrom must be page or text".into(),
        ErrorKind::InvalidPageBorderDisplay => {
            "display must be allPages, firstPage, or notFirstPage".into()
        }
        ErrorKind::InvalidPageBorderZOrder => "zOrder must be front or back".into(),
        ErrorKind::InvalidPageBorderColor => {
            "border color must be auto or a 6-digit RGB hex value".into()
        }
        ErrorKind::MathNotFound { id } => format!("Equation not found: {id}"),
        ErrorKind::ContentControlNotFound { id } => format!("Content control not found: {id}"),
        ErrorKind::SetParaTextHasField => {
            "Cannot change visible text in a paragraph that contains a field".into()
        }
        ErrorKind::SetParaTextHasMath => {
            "Cannot change visible text in a paragraph that contains an equation".into()
        }
        ErrorKind::SetParaTextHasSdt => {
            "Cannot change visible text in a paragraph that contains a content control".into()
        }
        ErrorKind::EmptyFieldInstr => "instr must not be empty".into(),
        ErrorKind::FieldNeedInstrOrCode => "exactly one of instr or code is required".into(),
        ErrorKind::InvalidFieldCode { got } => format!("unknown field code: {got}"),
        ErrorKind::FieldNeedArg { code } => format!("{code} requires arg"),
        ErrorKind::EmptyMathText => "text must not be empty".into(),
        ErrorKind::InvalidMathKind => "kind must be inline or display".into(),
        ErrorKind::MathNeedTextOrExpr => "exactly one of text or expr is required".into(),
        ErrorKind::EmptyMathExpr => "expr must not be empty".into(),
        ErrorKind::InvalidMathExprType { got } => format!("unknown math node type: {got}"),
        ErrorKind::EmptyMathSlot { field } => format!("{field} must not be empty"),
        ErrorKind::InvalidContentControlKind => {
            "kind must be plainText, richText, dropDown, comboBox, date, checkbox, picture, group, or equation".into()
        }
        ErrorKind::EmptyContentControlText => "text must not be empty".into(),
        ErrorKind::InvalidContentControlKeep => "keep must be content or none".into(),
        ErrorKind::InvalidContentControlItems => {
            "items must be a non-empty array of non-empty strings".into()
        }
        ErrorKind::ContentControlFieldMismatch => {
            "items is only for dropDown/comboBox, fullDate is only for date, file is only for picture, and checked is only for checkbox".into()
        }
        ErrorKind::ContentControlBound => {
            "Cannot change a content control that is bound to custom XML".into()
        }
        ErrorKind::ContentControlRepeating => {
            "Cannot change the value of a repeating-section content control".into()
        }
        ErrorKind::ContentControlGroup => {
            "Cannot change the value of a group content control".into()
        }
        ErrorKind::ContentControlHasTable => {
            "Cannot flatten a block content control that contains a table".into()
        }
        ErrorKind::InvalidFieldKeep => "keep must be result or instr".into(),
        ErrorKind::NeedSetFieldInstr => "setField requires instr".into(),
        ErrorKind::NumberingNotFound { id } => format!("Numbering instance not found: {id}"),
        ErrorKind::AbstractNumNotFound { id } => format!("Abstract numbering not found: {id}"),
        ErrorKind::InvalidNumberingKind => "kind must be decimal or bullet".into(),
        ErrorKind::InvalidListNumFmt => {
            "numFmt must be decimal, bullet, lowerLetter, upperLetter, lowerRoman, or upperRoman"
                .into()
        }
        ErrorKind::NeedSetNumberingField => {
            "setNumbering requires at least one of numFmt, start, or lvlText".into()
        }
        ErrorKind::InvalidIlvl => "ilvl must be an integer from 0 to 8, and that level must exist"
            .into(),
        ErrorKind::NeedSetParaNumberingNum => {
            "setParaNumbering requires num (a num: id or null)".into()
        }
        ErrorKind::StyleNotFound { id } => format!("Style not found: {id}"),
        ErrorKind::SetStyleNeedsParagraph => "setStyle and setParaStyle require a paragraph style".into(),
        ErrorKind::NeedSetStyleField => {
            "setStyle requires at least one of font, eastAsiaFont, sz, bold, color, before, or after"
                .into()
        }
        ErrorKind::EmptyStyleFont => "font and eastAsiaFont must not be empty".into(),
        ErrorKind::InvalidStyleColor => "color must be six hexadecimal RGB digits".into(),
        ErrorKind::NeedSetParaStyleStyle => {
            "setParaStyle requires style (a style: id or null)".into()
        }
        ErrorKind::NeedSetParaFormatField => {
            "setParaFormat requires at least one of align, left, right, firstLine, hanging, before, after, line, or lineRule"
                .into()
        }
        ErrorKind::NeedSetRunFormatField => {
            "setRunFormat requires at least one of bold, italic, underline, sz, color, font, or eastAsiaFont"
                .into()
        }
        ErrorKind::NeedSetCellStyleField => {
            "setCellStyle requires at least one of numFmt, font, fill, border, or align".into()
        }
        ErrorKind::InvalidCellStyle { field } => {
            format!("{field} is not a valid setCellStyle value")
        }
        ErrorKind::NeedSetColWidthField => {
            "setColWidth requires at least one of width or bestFit".into()
        }
        ErrorKind::InvalidColWidth => {
            "width must be greater than 0 and at most 255".into()
        }
        ErrorKind::NeedSetRowHeightField => "setRowHeight requires height".into(),
        ErrorKind::InvalidRowHeight => {
            "height must be greater than 0 and at most 409".into()
        }
        ErrorKind::MergeNeedsMultipleCells => {
            "mergeRange needs at least two cells".into()
        }
        ErrorKind::RowRangeReversed => "toRow must not be less than row".into(),
        ErrorKind::LayoutWorksheetOnly => {
            "column width, row height, merge, data validation, conditional format, sheet table, and pivot table ops only apply to worksheets"
                .into()
        }
        ErrorKind::NeedSetDataValidationField => {
            "setDataValidation requires a field besides op and id".into()
        }
        ErrorKind::NeedDataValidationFormula { field } => {
            format!("data validation requires {field}")
        }
        ErrorKind::InvalidDataValidation { field } => {
            format!("{field} is not a valid data validation value")
        }
        ErrorKind::DataValidationNotFound { id } => {
            format!("Data validation not found: {id}")
        }
        ErrorKind::NeedSetConditionalFormatField => {
            "setConditionalFormat requires a field besides op and id".into()
        }
        ErrorKind::NeedConditionalFormatFormula { field } => {
            format!("conditional format requires {field}")
        }
        ErrorKind::InvalidConditionalFormat { field } => {
            format!("{field} is not a valid conditional format value")
        }
        ErrorKind::ConditionalFormatNotFound { id } => {
            format!("Conditional format not found: {id}")
        }
        ErrorKind::NeedSetSheetTableField => {
            "setSheetTable requires a field besides op and id".into()
        }
        ErrorKind::InvalidSheetTable { field } => {
            format!("{field} is not a valid sheet table value")
        }
        ErrorKind::SheetTableNotFound { id } => {
            format!("Sheet table not found: {id}")
        }
        ErrorKind::NeedSetPivotTableField => {
            "setPivotTable requires a field besides op and id".into()
        }
        ErrorKind::InvalidPivotTable { field } => {
            format!("{field} is not a valid pivot table value")
        }
        ErrorKind::PivotTableNotFound { id } => {
            format!("Pivot table not found: {id}")
        }
        ErrorKind::NeedSetDefinedNameField => {
            "setDefinedName requires a field besides op and id".into()
        }
        ErrorKind::InvalidDefinedName { field } => {
            format!("{field} is not a valid defined name value")
        }
        ErrorKind::DefinedNameNotFound { id } => {
            format!("Defined name not found: {id}")
        }
        ErrorKind::NeedSetPrintSetupField => {
            "setPrintSetup requires a field besides op and id".into()
        }
        ErrorKind::InvalidPrintSetup { field } => {
            format!("{field} is not a valid setPrintSetup value")
        }
        ErrorKind::NeedSetSheetViewField => {
            "setSheetView requires a field besides op and id".into()
        }
        ErrorKind::InvalidSheetView { field } => {
            format!("{field} is not a valid setSheetView value")
        }
        ErrorKind::NeedSetAutoFilterField => "setAutoFilter requires ref".into(),
        ErrorKind::InvalidAutoFilter { field } => {
            format!("{field} is not a valid setAutoFilter value")
        }
        ErrorKind::NeedSetSortStateField => "setSortState requires ref".into(),
        ErrorKind::InvalidSortState { field } => {
            format!("{field} is not a valid setSortState value")
        }
        ErrorKind::NeedSetOutlineField => {
            "setOutline requires a field besides op and id".into()
        }
        ErrorKind::InvalidOutline { field } => {
            format!("{field} is not a valid outline value")
        }
        ErrorKind::OutlineLevelMax => "outline level cannot exceed 7".into(),
        ErrorKind::InvalidAlign => {
            "align must be left, center, right, both, distribute, start, or end".into()
        }
        ErrorKind::InvalidLineRule => "lineRule must be auto, exact, or atLeast".into(),
        ErrorKind::InvalidUnderline => "underline is not a known ST_Underline value".into(),
        ErrorKind::RunNotFound { id } => format!("Run not found: {id}"),
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
