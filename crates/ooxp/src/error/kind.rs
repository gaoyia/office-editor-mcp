//! 错误身份。文案在 `messages/` 里按语言渲染，`code()` 由 kind 推导。

/// 字段期望的 JSON 类型，供 `WrongType` 使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeName {
    Object,
    Array,
    Integer,
    String,
    NonEmptyString,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeNeed {
    SheetAndRange,
    FromAndMaxParas,
    Slide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RangeReason {
    Empty,
    NotA1 { input: String },
    RowOutOfRange { row: u32 },
    ColOutOfRange { col: u32 },
    Reversed { input: String },
    InvalidColLetters { letters: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdReason {
    NotCell { id: String },
    ParaPrefix,
    /// 页眉/页脚容器 id，缺少 `/para:…`
    HdrFtrContainer,
    ParaIndexNotInt,
    ParaIndexFromOne,
    ParaIdNotHex,
    ShapePrefix,
    ShapeForm,
    SlideIndexNotInt,
    SlideIndexFromOne,
    ShapeEmpty,
    ShapeIndexNotInt,
    ShapeIndexFromOne,
    SectionPrefix,
    SectionIndexNotInt,
    SectionIndexFromOne,
    /// 图片 id 必须是 header:…/pic:… 或 footer:…/pic:…
    PicPrefix,
    /// 页眉/页脚容器 id，缺少 `/pic:…`
    PicContainer,
    PicIndexNotInt,
    PicIndexFromOne,
    /// 正文里的图本期不做
    PicBodyNotYet,
    /// insertPicture 必须用页眉页脚段落 id
    InsertNeedsPara,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SheetNameReason {
    Missing,
    Length,
    History,
    IllegalChars,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpcReason {
    EmptyPartName,
    InvalidPartName { name: String },
    MissingContentTypes,
    MissingRootRels,
    MissingOfficeDocument,
    RelTarget { target: String },
    Zip { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RelSource {
    Unspecified,
    Workbook,
    Part(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlIssue {
    UnbalancedEnd,
    NoRoot,
    TextDecode,
    Parse,
    AttrRead,
    AttrDecode,
    Write,
}

macro_rules! define_error_kinds {
    (
        $(
            $variant:ident $( { $($field:ident : $fty:ty),* $(,)? } )?
            => $sample:expr
        ),+ $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub(crate) enum ErrorKind {
            $( $variant $( { $($field: $fty),* } )? , )+
        }

        impl ErrorKind {
            /// 每种 kind 至少一条样本，新增 variant 必须在宏里补，否则编译不过。
            #[cfg(test)]
            pub(crate) fn all_samples() -> Vec<Self> {
                let mut out = Vec::new();
                $( out.extend($sample); )+
                out
            }
        }
    };
}

define_error_kinds! {
    InvalidJsonParse { detail: String } => vec![Self::InvalidJsonParse { detail: "x".into() }],
    InvalidJsonNotObject => vec![Self::InvalidJsonNotObject],
    MissingEnvelopeField { field: String } => vec![Self::MissingEnvelopeField { field: "action".into() }],
    UnsupportedSchema => vec![Self::UnsupportedSchema],
    UnsupportedAction { action: String } => vec![Self::UnsupportedAction { action: "foo".into() }],
    UnknownField { field: String } => vec![Self::UnknownField { field: "x".into() }],
    CannotInferFormat { path: String } => vec![Self::CannotInferFormat { path: "a.bin".into() }],
    UnsupportedFormat { format: String } => vec![Self::UnsupportedFormat { format: "xls".into() }],
    MissingField { field: String } => vec![Self::MissingField { field: "path".into() }],
    WrongType { field: String, expected: TypeName } => TypeName::all_samples()
        .into_iter()
        .map(|expected| Self::WrongType { field: "scope".into(), expected })
        .collect::<Vec<_>>(),
    EmptyArray { field: String } => vec![Self::EmptyArray { field: "ops".into() }],
    OutOfRange { field: String, min: i64, max: i64 } => vec![Self::OutOfRange { field: "maxCells".into(), min: 1, max: 20000 }],
    MinValue { field: String, min: i64 } => vec![Self::MinValue { field: "from".into(), min: 1 }],
    MutuallyExclusive { a: String, b: String } => vec![Self::MutuallyExclusive { a: "value".into(), b: "formula".into() }],
    ContentObjectsExclusive => vec![Self::ContentObjectsExclusive],
    CreateNeedsObject { format: String, field: String } => vec![Self::CreateNeedsObject { format: "xlsx".into(), field: "workbook".into() }],
    CreateMissingObject { field: String } => vec![Self::CreateMissingObject { field: "workbook".into() }],
    ScopeRequired { need: ScopeNeed } => ScopeNeed::all_samples()
        .into_iter()
        .map(|need| Self::ScopeRequired { need })
        .collect::<Vec<_>>(),
    UnsupportedOp { op: String } => vec![Self::UnsupportedOp { op: "merge".into() }],
    OpNotForFormat { format: String, op: String } => vec![Self::OpNotForFormat { format: "xlsx".into(), op: "setParaText".into() }],
    SetCellNeedsValueOrFormula => vec![Self::SetCellNeedsValueOrFormula],
    FormulaMustStartWithEquals => vec![Self::FormulaMustStartWithEquals],
    InvalidCellValue => vec![Self::InvalidCellValue],
    OpRequiresField { op: String, field: String } => vec![Self::OpRequiresField { op: "setParaText".into(), field: "text".into() }],
    InvalidRange { reason: RangeReason } => RangeReason::all_samples()
        .into_iter()
        .map(|reason| Self::InvalidRange { reason })
        .collect::<Vec<_>>(),
    InvalidId { reason: IdReason } => IdReason::all_samples()
        .into_iter()
        .map(|reason| Self::InvalidId { reason })
        .collect::<Vec<_>>(),
    SheetNotFound { name: String } => vec![Self::SheetNotFound { name: "Sheet9".into() }],
    ParaNotFound { id: String } => vec![Self::ParaNotFound { id: "para:1".into() }],
    SectionNotFound { id: String } => vec![Self::SectionNotFound { id: "section:9".into() }],
    SlideNotFound { n: u32 } => vec![Self::SlideNotFound { n: 3 }],
    ShapeNotFound { id: String } => vec![Self::ShapeNotFound { id: "slide:1/shape:2".into() }],
    ShapeHasNoText { id: String } => vec![Self::ShapeHasNoText { id: "slide:1/shape:2".into() }],
    SetCellWorksheetOnly => vec![Self::SetCellWorksheetOnly],
    InspectWorksheetOnly => vec![Self::InspectWorksheetOnly],
    SharedFormulaMaster { cell: String } => vec![Self::SharedFormulaMaster { cell: "A1".into() }],
    NeedAtLeastOneSheet => vec![Self::NeedAtLeastOneSheet],
    DuplicateSheetName { name: String } => vec![Self::DuplicateSheetName { name: "Sheet1".into() }],
    InvalidSheetName { reason: SheetNameReason } => SheetNameReason::all_samples()
        .into_iter()
        .map(|reason| Self::InvalidSheetName { reason })
        .collect::<Vec<_>>(),
    InvalidParaStyle => vec![Self::InvalidParaStyle],
    InvalidHfKind => vec![Self::InvalidHfKind],
    InvalidHfType => vec![Self::InvalidHfType],
    InvalidPicKind => vec![Self::InvalidPicKind],
    InvalidPicWrap => vec![Self::InvalidPicWrap],
    InvalidPicAlign => vec![Self::InvalidPicAlign],
    InvalidImageFile => vec![Self::InvalidImageFile],
    PictureNotFound { id: String } => vec![Self::PictureNotFound { id: "header:default:1/pic:1".into() }],
    CannotLinkFirstSection => vec![Self::CannotLinkFirstSection],
    HfTypeInactive { kind: String, hf_type: String } => vec![Self::HfTypeInactive {
        kind: "header".into(),
        hf_type: "even".into(),
    }],
    OpcOpen { reason: OpcReason } => OpcReason::all_samples()
        .into_iter()
        .map(|reason| Self::OpcOpen { reason })
        .collect::<Vec<_>>(),
    PartMissing { name: String } => vec![Self::PartMissing { name: "/xl/workbook.xml".into() }],
    RelNotFound { source: RelSource, rid: String } => RelSource::all_samples()
        .into_iter()
        .map(|source| Self::RelNotFound { source, rid: "rId1".into() })
        .collect::<Vec<_>>(),
    ExternalRel { rid: String } => vec![Self::ExternalRel { rid: "rId9".into() }],
    SlideRelMissing { index: usize } => vec![Self::SlideRelMissing { index: 1 }],
    XmlBroken { issue: XmlIssue, detail: Option<String> } => XmlIssue::all_samples()
        .into_iter()
        .map(|issue| Self::XmlBroken { issue, detail: Some("x".into()) })
        .collect::<Vec<_>>(),
    XmlMissing { parent: String, child: String } => vec![Self::XmlMissing { parent: "workbook.xml".into(), child: "sheets".into() }],
    SharedStringIndex { cell: String, index: String } => vec![Self::SharedStringIndex {
        cell: "Sheet1!A1".into(),
        index: "9".into(),
    }],
    SharedStringNotInTable { text: String } => {
        vec![Self::SharedStringNotInTable { text: "hello".into() }]
    },
}

impl TypeName {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::Object,
            Self::Array,
            Self::Integer,
            Self::String,
            Self::NonEmptyString,
            Self::Boolean,
        ]
    }
}

impl ScopeNeed {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![Self::SheetAndRange, Self::FromAndMaxParas, Self::Slide]
    }
}

impl RangeReason {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::Empty,
            Self::NotA1 { input: "??".into() },
            Self::RowOutOfRange { row: 0 },
            Self::ColOutOfRange { col: 0 },
            Self::Reversed { input: "D20:A1".into() },
            Self::InvalidColLetters { letters: "1A".into() },
        ]
    }
}

impl IdReason {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::NotCell { id: "x".into() },
            Self::ParaPrefix,
            Self::HdrFtrContainer,
            Self::ParaIndexNotInt,
            Self::ParaIndexFromOne,
            Self::ParaIdNotHex,
            Self::ShapePrefix,
            Self::ShapeForm,
            Self::SlideIndexNotInt,
            Self::SlideIndexFromOne,
            Self::ShapeEmpty,
            Self::ShapeIndexNotInt,
            Self::ShapeIndexFromOne,
            Self::SectionPrefix,
            Self::SectionIndexNotInt,
            Self::SectionIndexFromOne,
            Self::PicPrefix,
            Self::PicContainer,
            Self::PicIndexNotInt,
            Self::PicIndexFromOne,
            Self::PicBodyNotYet,
            Self::InsertNeedsPara,
        ]
    }
}

impl SheetNameReason {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![Self::Missing, Self::Length, Self::History, Self::IllegalChars]
    }
}

impl OpcReason {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::EmptyPartName,
            Self::InvalidPartName { name: "..".into() },
            Self::MissingContentTypes,
            Self::MissingRootRels,
            Self::MissingOfficeDocument,
            Self::RelTarget { target: "../x".into() },
            Self::Zip { detail: "bad".into() },
        ]
    }
}

impl RelSource {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::Unspecified,
            Self::Workbook,
            Self::Part("/xl/workbook.xml".into()),
        ]
    }
}

impl XmlIssue {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::UnbalancedEnd,
            Self::NoRoot,
            Self::TextDecode,
            Self::Parse,
            Self::AttrRead,
            Self::AttrDecode,
            Self::Write,
        ]
    }
}

impl ErrorKind {
    pub(crate) fn code(&self) -> super::ErrorCode {
        use super::ErrorCode::*;
        match self {
            Self::InvalidJsonParse { .. }
            | Self::InvalidJsonNotObject
            | Self::MissingEnvelopeField { .. } => InvalidJson,
            Self::UnsupportedSchema => UnsupportedSchema,
            Self::UnsupportedAction { .. } => UnsupportedAction,
            Self::CannotInferFormat { .. } | Self::UnsupportedFormat { .. } => UnsupportedFormat,
            Self::UnsupportedOp { .. } => UnsupportedOp,
            Self::UnknownField { .. }
            | Self::MissingField { .. }
            | Self::WrongType { .. }
            | Self::EmptyArray { .. }
            | Self::MinValue { .. }
            | Self::MutuallyExclusive { .. }
            | Self::ContentObjectsExclusive
            | Self::CreateNeedsObject { .. }
            | Self::CreateMissingObject { .. }
            | Self::OpNotForFormat { .. }
            | Self::SetCellNeedsValueOrFormula
            | Self::FormulaMustStartWithEquals
            | Self::InvalidCellValue
            | Self::OpRequiresField { .. }
            | Self::SetCellWorksheetOnly
            | Self::InspectWorksheetOnly
            | Self::NeedAtLeastOneSheet
            | Self::DuplicateSheetName { .. }
            | Self::InvalidSheetName { .. }
            | Self::InvalidParaStyle
            | Self::InvalidHfKind
            | Self::InvalidHfType
            | Self::InvalidPicKind
            | Self::InvalidPicWrap
            | Self::InvalidPicAlign
            | Self::InvalidImageFile
            | Self::CannotLinkFirstSection
            | Self::HfTypeInactive { .. } => InvalidField,
            Self::OutOfRange { .. } => TruncatedLimit,
            Self::InvalidRange { .. } => InvalidRange,
            Self::InvalidId { .. } => InvalidId,
            Self::SheetNotFound { .. }
            | Self::ParaNotFound { .. }
            | Self::SectionNotFound { .. }
            | Self::SlideNotFound { .. }
            | Self::ShapeNotFound { .. }
            | Self::ShapeHasNoText { .. }
            | Self::PictureNotFound { .. } => IdNotFound,
            Self::ScopeRequired { .. } => ScopeRequired,
            Self::SharedFormulaMaster { .. } => NotImplemented,
            Self::OpcOpen { .. } => OpcOpen,
            Self::PartMissing { .. }
            | Self::RelNotFound { .. }
            | Self::ExternalRel { .. }
            | Self::SlideRelMissing { .. } => OpcPartMissing,
            Self::XmlBroken { .. }
            | Self::XmlMissing { .. }
            | Self::SharedStringIndex { .. }
            | Self::SharedStringNotInTable { .. } => XmlBroken,
        }
    }
}
