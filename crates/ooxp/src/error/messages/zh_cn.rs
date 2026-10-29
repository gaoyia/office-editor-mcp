//! 中文错误文案。现有句子照搬，只统一标点。

use super::super::kind::{
    ErrorKind, IdReason, OpcReason, RangeReason, RelSource, ScopeNeed, SheetNameReason, TypeName,
    XmlIssue,
};

pub(super) fn render(kind: &ErrorKind) -> String {
    match kind {
        ErrorKind::InvalidJsonParse { detail } => format!("入参不是 JSON：{detail}"),
        ErrorKind::IllegalXmlChar { scalar } => {
            format!("字符串含有 XML 不允许的字符 U+{scalar:04X}")
        }
        ErrorKind::InvalidJsonNotObject => "入参必须是 JSON 对象".into(),
        ErrorKind::MissingEnvelopeField { field } => format!("缺少 {field}"),
        ErrorKind::UnsupportedSchema => "schemaVersion 必须是 1".into(),
        ErrorKind::UnsupportedAction { action } => format!("不支持的 action：{action}"),
        ErrorKind::UnknownField { field } => format!("不认识的字段：{field}"),
        ErrorKind::CannotInferFormat { path } => format!("无法从路径推断格式：{path}"),
        ErrorKind::UnsupportedFormat { format } => format!("不支持的 format：{format}"),
        ErrorKind::MissingField { field } => format!("缺少 {field}"),
        ErrorKind::WrongType { field, expected } => {
            format!("{field} 必须是{}", type_name(*expected))
        }
        ErrorKind::EmptyArray { field } => format!("{field} 不能为空"),
        ErrorKind::OutOfRange { field, min, max } => {
            format!("{field} 允许范围是 {min} 到 {max}")
        }
        ErrorKind::MinValue { field, min } => format!("{field} 从 {min} 起"),
        ErrorKind::CreateIndexOutOfRange { field, max } => format!("{field} 不能超过 {max}"),
        ErrorKind::MutuallyExclusive { a, b } => format!("{a} 和 {b} 不能同时出现"),
        ErrorKind::ContentObjectsExclusive => {
            "workbook、document、presentation 不能同时出现".into()
        }
        ErrorKind::CreateNeedsObject { format, field } => {
            format!("{format} 的 create 必须用 {field}")
        }
        ErrorKind::CreateMissingObject { field } => format!("create 必须有 {field}"),
        ErrorKind::ScopeRequired { need } => match need {
            ScopeNeed::SheetAndRange => "看单元格时必须同时给 sheet 和 range".into(),
            ScopeNeed::FromAndMaxParas => "看段落正文时必须同时给 from 和 maxParas".into(),
            ScopeNeed::Slide => "看形状时必须给 slide".into(),
        },
        ErrorKind::UnsupportedOp { op } => format!("不支持的 op：{op}"),
        ErrorKind::OpNotForFormat { format, op } => format!("{format} 不支持 {op}"),
        ErrorKind::ScopeFieldRequires { field, required } => {
            format!("{field} 必须同时有 {required}")
        }
        ErrorKind::ActionNotForFormat { format, action } => {
            format!("{format} 不支持 {action}")
        }
        ErrorKind::SetCellNeedsValueOrFormula => "setCell 必须有 value 或 formula".into(),
        ErrorKind::FormulaMustStartWithEquals => "formula 必须以 = 开头".into(),
        ErrorKind::InvalidCellValue => "单元格值必须是数字、字符串、布尔或 null".into(),
        ErrorKind::OpRequiresField { op, field } => format!("{op} 必须有 {field}"),
        ErrorKind::InvalidRange { reason } => match reason {
            RangeReason::Empty => "空的单元格地址".into(),
            RangeReason::NotA1 { input } => format!("不是 A1 地址：{input}"),
            RangeReason::RowOutOfRange { row } => format!("行号超出范围：{row}"),
            RangeReason::ColOutOfRange { col } => format!("列号超出范围：{col}"),
            RangeReason::Reversed { input } => format!("区域起止颠倒：{input}"),
            RangeReason::InvalidColLetters { letters } => format!("非法列字母：{letters}"),
        },
        ErrorKind::InvalidId { reason } => match reason {
            IdReason::NotCell { id } => format!("不是单元格 id：{id}"),
            IdReason::ParaPrefix => {
                "段落 id 必须是 para:…、header:…/para:… 或 footer:…/para:…".into()
            }
            IdReason::HdrFtrContainer => {
                "页眉页脚 id 必须带 /para:…（容器 id 本身不是段落）".into()
            }
            IdReason::ParaIndexNotInt => "para:index 后面必须是正整数".into(),
            IdReason::ParaIndexFromOne => "para:index 从 1 起".into(),
            IdReason::ParaIdNotHex => "paraId 必须是十六进制".into(),
            IdReason::ShapePrefix => "形状 id 必须以 slide: 开头".into(),
            IdReason::ShapeForm => {
                "setShapeText 的 id 必须是 slide:N/shape:… 或 slide:N/notes/shape:…".into()
            }
            IdReason::SlideIndexNotInt => "slide 序号必须是正整数".into(),
            IdReason::SlideIndexFromOne => "slide 序号从 1 起".into(),
            IdReason::ShapeEmpty => "shape 后面不能为空".into(),
            IdReason::ShapeIndexNotInt => "shape:index 后面必须是正整数".into(),
            IdReason::ShapeIndexFromOne => "shape:index 从 1 起".into(),
            IdReason::SectionPrefix => "节 id 必须是 section:N".into(),
            IdReason::SectionIndexNotInt => "section 序号必须是正整数".into(),
            IdReason::SectionIndexFromOne => "section 序号从 1 起".into(),
            IdReason::PicPrefix => {
                "图片 id 必须是 header:…/pic:…、footer:…/pic:… 或 para:…/pic:…".into()
            }
            IdReason::PicContainer => {
                "段落或页眉页脚 id 必须带 /pic:…（该 id 本身不是图）".into()
            }
            IdReason::PicIndexNotInt => "pic:index 后面必须是正整数".into(),
            IdReason::PicIndexFromOne => "pic:index 从 1 起".into(),
            IdReason::InsertNeedsPara => "insertPicture 的 id 必须是段落 id".into(),
            IdReason::SheetPicPrefix => {
                "xlsx 图 id 必须是 sheet:工作表名/pic:… 或 sheet:工作表名/hf:…/pic:…".into()
            }
            IdReason::SheetPicContainer => {
                "工作表 id 必须带 /pic:…（该 id 本身不是图）".into()
            }
            IdReason::InsertNeedsSheet => {
                "xlsx 的 insertPicture 的 id 必须是 sheet:工作表名 或页眉槽位 id".into()
            }
            IdReason::SlidePicPrefix => {
                "pptx 图 id 必须是 slide:N/pic:…、slide:N/notes/pic:…、master:N/pic:…、master:N/layout:K/pic:… 或 slide:N/comment:GUID/pic:n".into()
            }
            IdReason::SlidePicContainer => {
                "页 id 必须带 /pic:…（该 id 本身不是图）".into()
            }
            IdReason::CommentPicExportOnly => {
                "批注内图没有画布尺寸和位置，不能 setPictureSize / setPicturePos".into()
            }
            IdReason::SlideMediaForm => {
                "pptx 视频 / 音频 id 必须是 slide:N/video:…、slide:N/audio:…、slide:N/notes/…、master:N/… 或 master:N/layout:K/…".into()
            }
            IdReason::InsertMediaNeedsSlide => {
                "pptx 的 insertVideo / insertAudio 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SlideCxnForm => {
                "pptx 连接线 id 必须是 slide:N/cxn:…、slide:N/notes/cxn:…、master:N/cxn:… 或 master:N/layout:K/cxn:…".into()
            }
            IdReason::InsertConnectorNeedsSlide => {
                "pptx 的 insertConnector 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::InsertNeedsSlide => {
                "pptx 的 insertPicture 的 id 必须是 slide:N、slide:N/notes、master:N、master:N/layout:K 或批注 id".into()
            }
            IdReason::InsertShapeHost => {
                "pptx 的 insertShape 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SetNotesTextNeedsSlide => {
                "setNotesText 的 id 必须是 slide:N 或 slide:N/notes".into()
            }
            IdReason::PptxCommentForm => {
                "pptx 批注 id 必须是 slide:N/comment:GUID 或 …/reply:GUID".into()
            }
            IdReason::InsertCommentNeedsSlide => {
                "pptx 的 insertComment 的 id 必须是 slide:N".into()
            }
            IdReason::XlsxCommentForm => {
                "xlsx 批注 id 必须是 cell:表!A1/comment:GUID 或 …/reply:GUID".into()
            }
            IdReason::InsertCommentNeedsCell => {
                "xlsx 的 insertComment 的 id 必须是单元格 id cell:表!A1".into()
            }
            IdReason::HfSlotPrefix => {
                "xlsx 页眉槽位 id 必须是 sheet:工作表名/hf:odd|even|first:header|footer".into()
            }
            IdReason::TxbxPrefix => "文本框 id 必须是段落 id 加上 /txbx:…".into(),
            IdReason::TxbxContainer => {
                "段落 id 必须带 /txbx:…（该 id 本身不是文本框）".into()
            }
            IdReason::TxbxIndexNotInt => "txbx:index 后面必须是正整数".into(),
            IdReason::TxbxIndexFromOne => "txbx:index 从 1 起".into(),
            IdReason::SetParaOnBox => "setParaText 的 id 必须是段落 id，不能是文本框 id".into(),
            IdReason::InsertNeedsHostPara => {
                "insertTextBox / insertConnector / insertShape / insertInkCanvas 的 id 必须是宿主段落 id".into()
            }
            IdReason::DocxCxnPrefix => "docx 连接线 id 必须是段落 id 加上 /cxn:…".into(),
            IdReason::DocxCxnContainer => {
                "段落或容器 id 必须带 /cxn:…（该 id 本身不是连接线）".into()
            }
            IdReason::DocxShapePrefix => "docx 形状 id 必须是段落 id 加上 /shape:…".into(),
            IdReason::DocxShapeContainer => {
                "段落或容器 id 必须带 /shape:…（该 id 本身不是形状）".into()
            }
            IdReason::DocxInkPrefix => "docx 墨迹画布 id 必须是段落 id 加上 /ink:…".into(),
            IdReason::DocxInkContainer => {
                "段落或容器 id 必须带 /ink:…（该 id 本身不是墨迹画布）".into()
            }
            IdReason::DocxInkIndexNotInt => "ink:index 后面必须是正整数".into(),
            IdReason::DocxInkIndexFromOne => "ink:index 从 1 起".into(),
            IdReason::DocxModel3dPrefix => "docx 3D 模型 id 必须是段落 id 加上 /model3d:…".into(),
            IdReason::DocxModel3dContainer => {
                "段落或容器 id 必须带 /model3d:…（该 id 本身不是 3D 模型）".into()
            }
            IdReason::DocxModel3dIndexNotInt => "model3d:index 后面必须是正整数".into(),
            IdReason::DocxModel3dIndexFromOne => "model3d:index 从 1 起".into(),
            IdReason::GroupNeedsDocument => {
                "docx 的 groupShapes 的 id 必须是 docx:document 或页眉/页脚容器".into()
            }
            IdReason::SheetSpPrefix => {
                "xlsx 文本框 id 必须是 sheet:工作表名/sp:…".into()
            }
            IdReason::SheetSpContainer => {
                "工作表 id 必须带 /sp:…（该 id 本身不是文本框）".into()
            }
            IdReason::SpIndexNotInt => "sp:index 后面必须是正整数".into(),
            IdReason::SpIndexFromOne => "sp:index 从 1 起".into(),
            IdReason::InsertTextBoxNeedsSheet => {
                "xlsx 的 insertTextBox 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetShapePrefix => {
                "xlsx 形状 id 必须是 sheet:工作表名/shape:…".into()
            }
            IdReason::SheetShapeContainer => {
                "工作表 id 必须带 /shape:…（该 id 本身不是形状）".into()
            }
            IdReason::InsertShapeNeedsSheet => {
                "xlsx 的 insertShape 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetCxnPrefix => {
                "xlsx 连接线 id 必须是 sheet:工作表名/cxn:…".into()
            }
            IdReason::SheetCxnContainer => {
                "工作表 id 必须带 /cxn:…（该 id 本身不是连接线）".into()
            }
            IdReason::CxnIndexNotInt => "cxn:index 后面必须是正整数".into(),
            IdReason::CxnIndexFromOne => "cxn:index 从 1 起".into(),
            IdReason::InsertConnectorNeedsSheet => {
                "xlsx 的 insertConnector 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::GroupNeedsSheet => {
                "xlsx 的 groupShapes 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SetShapeTextNotCxn => {
                "xlsx 的 setShapeText 的 id 必须是文本框或形状 id，不能是连接线".into()
            }
            IdReason::SheetSmartArtPrefix => {
                "xlsx SmartArt id 必须是 sheet:工作表名/smartdiagram:…".into()
            }
            IdReason::SheetSmartArtContainer => {
                "工作表 id 必须带 /smartdiagram:…（该 id 本身不是 SmartArt）".into()
            }
            IdReason::InsertSmartArtNeedsSheet => {
                "xlsx 的 insertSmartArt 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::InsertParaNeedsBox => {
                "deletePara 的 id 必须是段落 id（正文、页眉页脚、表格单元格、文本框、注或批注）".into()
            }
            IdReason::NotePrefix => "脚注、尾注 id 必须是 footnote:… 或 endnote:…".into(),
            IdReason::NoteContainer => {
                "注 id 必须带 /para:…（该 id 本身不是段落）".into()
            }
            IdReason::NoteSep => "分隔符 id 不能用于 apply".into(),
            IdReason::CommentPrefix => "批注 id 必须是 comment:…".into(),
            IdReason::CommentContainer => {
                "批注 id 必须带 /para:…（该 id 本身不是段落）".into()
            },
            IdReason::ChartPrefix => {
                "图表 id 必须是段落 id 加上 /chart:…".into()
            }
            IdReason::ChartContainer => {
                "段落 id 必须带 /chart:…（该 id 本身不是图表）".into()
            }
            IdReason::ChartIndexNotInt => "chart:index 后面必须是正整数".into(),
            IdReason::ChartIndexFromOne => "chart:index 从 1 起".into(),
            IdReason::SeriesPrefix => "系列 id 必须是图表 id 加上 /series:n".into(),
            IdReason::SeriesIndexNotInt => "series 后面必须是从 0 起的整数".into(),
            IdReason::TrendlinePrefix => "趋势线 id 必须是系列 id 加上 /trendline:k，k 从 1 起".into(),
            IdReason::TrendlineIndexNotInt => "trendline 后面必须是正整数".into(),
            IdReason::TrendlineIndexFromOne => "trendline 从 1 起".into(),
            IdReason::ErrorBarsPrefix => {
                "误差线 id 必须是系列 id 加上 /errorBars:k，k 从 1 起".into()
            }
            IdReason::ErrorBarsIndexNotInt => "errorBars 后面必须是正整数".into(),
            IdReason::ErrorBarsIndexFromOne => "errorBars 从 1 起".into(),
            IdReason::InsertChartNeedsPara => {
                "insertChart 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::InsertChartNeedsSheet => {
                "xlsx 的 insertChart 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::InsertChartNeedsSlide => {
                "pptx 的 insertChart 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::InsertChartSheetNeedsWorkbook => {
                "insertChartSheet 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::InsertSheetNeedsWorkbook => {
                "insertSheet 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::InsertSlideNeedsPresentation => {
                "insertSlide 的 id 必须是 pptx:presentation".into()
            }
            IdReason::InsertSlideSectionNeedsPresentation => {
                "insertSlideSection 的 id 必须是 pptx:presentation".into()
            }
            IdReason::InsertCustomShowNeedsPresentation => {
                "insertCustomShow 的 id 必须是 pptx:presentation".into()
            }
            IdReason::CustomShowPrefix => "自定义放映 id 必须是 show:N".into(),
            IdReason::CustomShowIndexNotInt => "show 序号必须是正整数".into(),
            IdReason::CustomShowIndexFromOne => "show 序号从 1 起".into(),
            IdReason::SetSlideSectionNeedsSlide => {
                "setSlideSection 的 id 必须是 slide:N".into()
            }
            IdReason::SlideOpNeedsSlide => {
                "id 必须是 slide:N".into()
            }
            IdReason::SetTransitionNeedsHost => {
                "setTransition 的 id 必须是 slide:N、master:N 或 master:N/layout:K".into()
            }
            IdReason::MasterForm => {
                "母版或占位符 id 必须是 master:N、master:N/layout:K、notesMaster/shape:…、handout/shape:… 或 …/shape:…".into()
            }
            IdReason::MasterPlaceholderLocked => {
                "母版 / 版式占位符请用 deletePlaceholder；装饰形才用 deleteShape / setShapeFill / setShapeLine".into()
            }
            IdReason::LayoutForm => {
                "版式 id 必须是 master:N/layout:K".into()
            }
            IdReason::InsertLayoutNeedsMaster => {
                "insertLayout 的 id 必须是 master:N".into()
            }
            IdReason::InsertPlaceholderNeedsHost => {
                "insertPlaceholder 的 id 必须是 master:N、master:N/layout:K、notesMaster 或 handout".into()
            }
            IdReason::InsertMasterNeedsPresentation => {
                "insertMaster 的 id 必须是 pptx:presentation".into()
            }
            IdReason::DeleteMasterNeedsMaster => {
                "deleteMaster 的 id 必须是 master:N".into()
            }
            IdReason::SetSlideSizeNeedsPresentation => {
                "setSlideSize 的 id 必须是 pptx:presentation".into()
            }
            IdReason::SetNotesSizeNeedsPresentation => {
                "setNotesSize 的 id 必须是 pptx:presentation".into()
            }
            IdReason::SetShowPrNeedsPresentation => {
                "setShowPr 的 id 必须是 pptx:presentation".into()
            }
            IdReason::SetProtectionNeedsPresentation => {
                "setProtection 的 id 必须是 pptx:presentation".into()
            }
            IdReason::SetThemeNeedsMaster => {
                "setThemeColors / setThemeFonts / setThemeFmt / setThemeExtraSchemes / setThemeObjectDefaults 的 id 必须是 master:N、notesMaster 或 handout".into()
            }
            IdReason::SetThemeNeedsDocument => {
                "setThemeColors / setThemeFonts / setThemeFmt / setThemeExtraSchemes / setThemeObjectDefaults / setThemeOverride 的 id 必须是 docx:document".into()
            }
            IdReason::SetThemeNeedsWorkbook => {
                "setThemeColors / setThemeFonts / setThemeFmt / setThemeExtraSchemes / setThemeObjectDefaults / setThemeOverride 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::SetClrMapNeedsHost => {
                "setClrMap 的 id 必须是 master:N、notesMaster 或 handout".into()
            }
            IdReason::SetClrMapOvrNeedsHost => {
                "setClrMapOvr 的 id 必须是 slide:N、slide:N/notes 或 master:N/layout:K".into()
            }
            IdReason::SetThemeOverrideNeedsSlide => {
                "setThemeOverride 的 id 必须是 slide:N 或 slide:N/notes".into()
            }
            IdReason::SetHandoutTextNeedsHandout => {
                "setHandoutText 的 id 必须是 handout".into()
            }
            IdReason::InsertDefinedNameNeedsWorkbook => {
                "insertDefinedName 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::InsertConnectionNeedsWorkbook => {
                "insertConnection 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::InsertExternalLinkNeedsWorkbook => {
                "insertExternalLink 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::InsertCustomViewNeedsWorkbook => {
                "insertCustomView 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::CustomViewPrefix => {
                "自定义视图 id 必须是 customView:n，n 从 1 起".into()
            }
            IdReason::CustomViewIndexNotInt => {
                "customView 后面必须是正整数".into()
            }
            IdReason::CustomViewIndexFromOne => "customView 从 1 起".into(),
            IdReason::InsertCellStyleNeedsWorkbook => {
                "insertCellStyle 的 id 必须是 xlsx:workbook".into()
            }
            IdReason::CellStylePrefix => {
                "命名单元格样式 id 必须是 cellStyle:n，n 从 1 起".into()
            }
            IdReason::CellStyleIndexNotInt => {
                "cellStyle 后面必须是正整数".into()
            }
            IdReason::CellStyleIndexFromOne => "cellStyle 从 1 起".into(),
            IdReason::ConnectionPrefix => {
                "连接 id 必须是 connection:ID 或 connection:index:N".into()
            }
            IdReason::ConnectionIndexNotInt => {
                "connection:index 后面必须是正整数".into()
            }
            IdReason::ConnectionIndexFromOne => "connection:index 从 1 起".into(),
            IdReason::ExternalLinkPrefix => {
                "外部引用 id 必须是 extlink:n，n 从 1 起".into()
            }
            IdReason::ExternalLinkIndexNotInt => {
                "extlink 后面必须是正整数".into()
            }
            IdReason::ExternalLinkIndexFromOne => "extlink 从 1 起".into(),
            IdReason::DefinedNamePrefix => {
                "定义名 id 必须是 name:n，n 从 1 起".into()
            }
            IdReason::NameIndexNotInt => "name 后面必须是正整数".into(),
            IdReason::NameIndexFromOne => "定义名序号从 1 起".into(),
            IdReason::DeleteChartSheetNeedsChartsheet => {
                "deleteChartSheet 的 id 必须是图表工作表 sheet:工作表名".into()
            }
            IdReason::DeleteSheetNeedsWorksheet => {
                "deleteSheet 的 id 必须是普通工作表 sheet:工作表名".into()
            }
            IdReason::SheetOpNeedsSheet => {
                "id 必须是 sheet:工作表名".into()
            }
            IdReason::SetProtectionNeedsSheetOrWorkbook => {
                "setProtection 的 id 必须是 sheet:工作表名 或 xlsx:workbook".into()
            }
            IdReason::InsertProtectedRangeNeedsSheet => {
                "insertProtectedRange 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetEditRangePrefix => {
                "允许编辑区域 id 必须是 sheet:工作表名/editRange:n，n 从 1 起".into()
            }
            IdReason::EditRangeIndexNotInt => "editRange 后面必须是正整数".into(),
            IdReason::EditRangeIndexFromOne => "editRange 序号从 1 起".into(),
            IdReason::SheetDvPrefix => {
                "数据验证 id 必须是 sheet:工作表名/dv:n，n 从 1 起".into()
            }
            IdReason::DvIndexNotInt => "dv 后面必须是正整数".into(),
            IdReason::DvIndexFromOne => "dv 序号从 1 起".into(),
            IdReason::SheetCfPrefix => {
                "条件格式 id 必须是 sheet:工作表名/cf:n，n 从 1 起".into()
            }
            IdReason::CfIndexNotInt => "cf 后面必须是正整数".into(),
            IdReason::CfIndexFromOne => "cf 序号从 1 起".into(),
            IdReason::SheetTablePrefix => {
                "工作表表格 id 必须是 sheet:工作表名/table:n，n 从 1 起".into()
            }
            IdReason::SheetPivotPrefix => {
                "透视表 id 必须是 sheet:工作表名/pivot:n，n 从 1 起".into()
            }
            IdReason::PivotIndexNotInt => "pivot 后面必须是正整数".into(),
            IdReason::PivotIndexFromOne => "pivot 序号从 1 起".into(),
            IdReason::SheetChartPrefix => {
                "xlsx 图表 id 必须是 sheet:工作表名/chart:…".into()
            }
            IdReason::SheetChartContainer => {
                "工作表 id 必须带 /chart:…（该 id 本身不是图表）".into()
            }
            IdReason::SlideChartPrefix => {
                "pptx 图表 id 必须是 slide:N/chart:…、slide:N/notes/chart:…、master:N/chart:… 或 master:N/layout:K/chart:…".into()
            }
            IdReason::SlideChartContainer => {
                "页 id 必须带 /chart:…（该 id 本身不是图表）".into()
            }
            IdReason::SmartArtPrefix => {
                "SmartArt id 必须是段落 id 加上 /smartdiagram:…".into()
            }
            IdReason::SmartArtContainer => {
                "段落 id 必须带 /smartdiagram:…（该 id 本身不是 SmartArt）".into()
            }
            IdReason::SmartArtIndexNotInt => "smartdiagram:index 后面必须是正整数".into(),
            IdReason::SmartArtIndexFromOne => "smartdiagram:index 从 1 起".into(),
            IdReason::SmartArtPtPrefix => {
                "SmartArt 点 id 必须是图示 id 加上 /pt:modelId".into()
            }
            IdReason::InsertSmartArtNeedsPara => {
                "insertSmartArt 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::SlideSmartArtPrefix => {
                "pptx SmartArt id 必须是 slide:N/smartdiagram:…、slide:N/notes/smartdiagram:…、master:N/smartdiagram:… 或 master:N/layout:K/smartdiagram:…".into()
            }
            IdReason::SlideSmartArtContainer => {
                "页 id 必须带 /smartdiagram:…（该 id 本身不是 SmartArt）".into()
            }
            IdReason::InsertSmartArtNeedsSlide => {
                "pptx insertSmartArt 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SlideTablePrefix => {
                "pptx 表 id 必须是 slide:N/table:…、slide:N/notes/table:…、master:N/table:… 或 master:N/layout:K/table:…".into()
            }
            IdReason::SlideTableContainer => {
                "页 id 必须带 /table:…（该 id 本身不是表）".into()
            }
            IdReason::SlideTableIndexNotInt => "table:index 后面必须是正整数".into(),
            IdReason::SlideTableIndexFromOne => "table:index 从 1 起".into(),
            IdReason::SlideTableRowForm => "行 id 必须是表 id 加上 /row:R，R 从 1 起".into(),
            IdReason::SlideTableCellForm => {
                "单元格 id 必须是表 id 加上 /r:R/c:C，R 与 C 从 1 起".into()
            }
            IdReason::InsertTableNeedsSlide => {
                "pptx insertTable 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SetTableCellNeedsCell => {
                "setTableCellText 的 id 必须是 DrawingML 表单元格 id".into()
            }
            IdReason::OlePrefix => {
                "OLE id 必须是段落 id 加上 /ole:…".into()
            }
            IdReason::OleContainer => {
                "段落 id 必须带 /ole:…（该 id 本身不是嵌入对象）".into()
            }
            IdReason::OleIndexNotInt => "ole:index 后面必须是正整数".into(),
            IdReason::OleIndexFromOne => "ole:index 从 1 起".into(),
            IdReason::InsertOleNeedsPara => {
                "insertOle 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::SlideOlePrefix => {
                "OLE id 必须是 slide:N/ole:…、slide:N/notes/ole:…、master:N/ole:… 或 master:N/layout:K/ole:…".into()
            }
            IdReason::SlideOleContainer => {
                "页 id 必须带 /ole:…（该 id 本身不是嵌入对象）".into()
            }
            IdReason::InsertOleNeedsSlide => {
                "pptx insertOle 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::InsertOleNeedsSheet => "xlsx insertOle 的 id 必须是 sheet:工作表名".into(),
            IdReason::SheetOlePrefix => "xlsx OLE id 必须是 sheet:工作表名/ole:…".into(),
            IdReason::SheetOleContainer => {
                "工作表 id 必须带 /ole:…（该 id 本身不是嵌入对象）".into()
            }
            IdReason::InsertInkNeedsSheet => {
                "xlsx 的 insertInk 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetInkPrefix => "xlsx 墨迹 id 必须是 sheet:工作表名/ink:…".into(),
            IdReason::SheetInkContainer => {
                "工作表 id 必须带 /ink:…（该 id 本身不是墨迹）".into()
            }
            IdReason::InsertModel3dNeedsSheet => {
                "xlsx 的 insertModel3d 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetModel3dPrefix => {
                "xlsx 3D 模型 id 必须是 sheet:工作表名/model3d:…".into()
            }
            IdReason::SheetModel3dContainer => {
                "工作表 id 必须带 /model3d:…（该 id 本身不是 3D 模型）".into()
            }
            IdReason::InsertModel3dNeedsSlide => {
                "pptx 的 insertModel3d 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SlideModel3dPrefix => {
                "pptx 3D 模型 id 必须是 slide:N/model3d:…、slide:N/notes/model3d:…、master:N/model3d:… 或 master:N/layout:K/model3d:…".into()
            }
            IdReason::SlideModel3dContainer => {
                "页 id 必须带 /model3d:…（该 id 本身不是 3D 模型）".into()
            }
            IdReason::InsertInkNeedsSlide => {
                "pptx 的 insertInk 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SlideInkPrefix => {
                "pptx 墨迹 id 必须是 slide:N/ink:…、slide:N/notes/ink:…、master:N/ink:… 或 master:N/layout:K/ink:…".into()
            }
            IdReason::SlideInkContainer => {
                "页 id 必须带 /ink:…（该 id 本身不是墨迹）".into()
            }
            IdReason::InsertMathNeedsSheet => {
                "xlsx 的 insertMath 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetMathPrefix => {
                "xlsx 公式框 id 必须是 sheet:工作表名/math:…".into()
            }
            IdReason::SheetMathContainer => {
                "工作表 id 必须带 /math:…（该 id 本身不是公式框）".into()
            }
            IdReason::InsertMathNeedsSlide => {
                "pptx 的 insertMath 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::SlideMathPrefix => {
                "pptx 公式框 id 必须是 slide:N/math:…、slide:N/notes/math:…、master:N/math:… 或 master:N/layout:K/math:…".into()
            }
            IdReason::SlideMathContainer => {
                "页 id 必须带 /math:…（该 id 本身不是公式框）".into()
            }
            IdReason::InsertSlicerNeedsSheet => {
                "xlsx insertSlicer 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::SheetSlicerPrefix => {
                "xlsx 切片器 id 必须是 sheet:工作表名/slicer:…".into()
            }
            IdReason::SheetSlicerContainer => {
                "工作表 id 必须带 /slicer:…（该 id 本身不是切片器）".into()
            }
            IdReason::SlicerIndexNotInt => "slicer:index 后面必须是正整数".into(),
            IdReason::SlicerIndexFromOne => "slicer:index 从 1 起".into(),
            IdReason::InsertSlicerNeedsTableOrPivot => {
                "insertSlicer 的 source 必须是本表表格 id 或透视表 id".into()
            }
            IdReason::InsertZoomNeedsSlide => {
                "insertZoom 的 id 必须是 slide:N、slide:N/notes、master:N 或 master:N/layout:K".into()
            }
            IdReason::ZoomTargetSlide => {
                "insertZoom 的 type 为 slide 时 target 必须是 slide:N".into()
            }
            IdReason::SlideZoomPrefix => {
                "Zoom id 必须是 slide:N/zoom:…、slide:N/notes/zoom:…、master:N/zoom:… 或 master:N/layout:K/zoom:…".into()
            }
            IdReason::SlideZoomContainer => {
                "页 id 必须带 /zoom:…（该 id 本身不是 Zoom）".into()
            }
            IdReason::ZoomIndexNotInt => "zoom:index 后面必须是正整数".into(),
            IdReason::ZoomIndexFromOne => "zoom:index 从 1 起".into(),
            IdReason::InsertWatermarkNeedsDocument => {
                "insertWatermark 的 id 必须是 docx:document".into()
            }
            IdReason::WatermarkPrefix => {
                "水印 id 必须是页眉容器 id 加上 /watermark:…".into()
            }
            IdReason::WatermarkContainer => {
                "页眉容器 id 必须带 /watermark:…（该 id 本身不是水印）".into()
            }
            IdReason::WatermarkIndexNotInt => "watermark:index 后面必须是正整数".into(),
            IdReason::WatermarkIndexFromOne => "watermark:index 从 1 起".into(),
            IdReason::EvenOddNeedsDocument => {
                "setEvenAndOddHeaders 的 id 必须是 docx:document".into()
            }
            IdReason::SettingsNeedDocument => {
                "setMirrorMargins / setGutterAtTop / setProtection 的 id 必须是 docx:document".into()
            }
            IdReason::TablePrefix => {
                "表 id 必须是 table:N，或页眉页脚容器 / 文本框 id 加上 /table:N".into()
            }
            IdReason::TableIndexNotInt => "table 序号必须是正整数".into(),
            IdReason::TableIndexFromOne => "table 序号从 1 起".into(),
            IdReason::TableCellForm => {
                "单元格 id 必须是 …/table:N/rRcC，行 id 必须是 …/table:N/row:R".into()
            }
            IdReason::InsertTableAnchor => {
                "insertTable 的 id 必须是段落、文档根、页眉页脚、文本框、表或单元格".into()
            }
            IdReason::DeleteTableNeedsTable => "deleteTable 的 id 必须是表 id".into(),
            IdReason::NeedTableId => "id 必须是表 id".into(),
            IdReason::InsertRowNeedsTableOrRow => {
                "insertRow 的 id 必须是表 id 或行 id".into()
            }
            IdReason::DeleteRowNeedsRow => "deleteRow 的 id 必须是行 id".into(),
            IdReason::SetTableRowHeightNeedsRow => {
                "setTableRowHeight 的 id 必须是行 id".into()
            }
            IdReason::SetTableTrHeightNeedsRow => {
                "setTableTrHeight 的 id 必须是行 id".into()
            }
            IdReason::SplitNeedsCell => "splitCells 的 id 必须是单元格 id".into(),
            IdReason::RevPrefix => {
                "修订 id 必须是 rev:N，或页眉页脚容器 id 加上 /rev:N".into()
            }
            IdReason::RevIndexNotInt => "修订序号必须是正整数".into(),
            IdReason::RevIndexFromOne => "修订序号从 1 起".into(),
            IdReason::LinkPrefix => {
                "超链接 id 必须是 link:N，或页眉页脚容器 id 加上 /link:N，或文本框 id 加上 /link:N".into()
            }
            IdReason::LinkIndexNotInt => "超链接序号必须是正整数".into(),
            IdReason::LinkIndexFromOne => "超链接序号从 1 起".into(),
            IdReason::InsertLinkNeedsPara => {
                "insertHyperlink 的 id 必须是正文、页眉页脚或框内段落 id，或图 / 文本框 / 连接线 / 无框形状 / SmartArt / 图表 / 墨迹画布 / 3D 的 id（含框内图、注内 / 批注内的图或框）".into()
            }
            IdReason::SheetLinkPrefix => {
                "xlsx 超链接 id 必须是 sheet:工作表名/link:n，n 从 1 起".into()
            }
            IdReason::InsertHyperlinkNeedsSheet => {
                "xlsx 的 insertHyperlink 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::InsertHyperlinkNeedsShape => {
                "pptx 的 insertHyperlink 的 id 必须是幻灯片、备注、母版或版式上的文本形状、图、表、图表、SmartArt 或表单元格 id".into()
            }
            IdReason::SlideLinkPrefix => {
                "pptx 超链接 id 必须是 slide:N/link:k、slide:N/notes/link:k、master:N/link:k 或 master:N/layout:K/link:k".into()
            }
            IdReason::PptxJumpNeedsSlide => {
                "pptx 内部跳转必须是 slide:n，n 从 1 起".into()
            }
            IdReason::SheetSparkPrefix => {
                "xlsx 迷你图组 id 必须是 sheet:工作表名/spark:n，n 从 1 起".into()
            }
            IdReason::SparkIndexNotInt => "迷你图序号必须是正整数".into(),
            IdReason::SparkIndexFromOne => "迷你图序号从 1 起".into(),
            IdReason::InsertSparklineNeedsSheet => {
                "xlsx 的 insertSparkline 的 id 必须是 sheet:工作表名".into()
            }
            IdReason::BookmarkPrefix => {
                "书签 id 必须是 bookmark:N，或页眉页脚容器 id 加上 /bookmark:N，或文本框 id 加上 /bookmark:N".into()
            }
            IdReason::BookmarkIndexNotInt => "书签序号必须是正整数".into(),
            IdReason::BookmarkIndexFromOne => "书签序号从 1 起".into(),
            IdReason::InsertBookmarkNeedsPara => {
                "insertBookmark 的 id 与 end 必须是正文、页眉页脚或框内段落 id".into()
            }
            IdReason::PermPrefix => {
                "例外编辑区 id 必须是 perm:N，或页眉页脚容器 id 加上 /perm:N".into()
            }
            IdReason::PermIndexNotInt => "例外编辑区序号必须是正整数".into(),
            IdReason::PermIndexFromOne => "例外编辑区序号从 1 起".into(),
            IdReason::InsertEditRangeNeedsPara => {
                "insertEditRange 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::SetBibliographyStyleNeedsDocument => {
                "setBibliographyStyle 的 id 必须是 docx:document".into()
            }
            IdReason::FieldPrefix => {
                "域 id 必须是 field:N，或页眉页脚容器 id 加上 /field:N，或文本框 id 加上 /field:N".into()
            }
            IdReason::FieldIndexNotInt => "域序号必须是正整数".into(),
            IdReason::FieldIndexFromOne => "域序号从 1 起".into(),
            IdReason::InsertFieldNeedsPara => {
                "insertField 的 id 必须是正文、页眉页脚或框内段落 id".into()
            }
            IdReason::PptxFieldForm => {
                "pptx 域 id 必须是 slide:N/fld:…、slide:N/notes/fld:…、master:N/fld:… 或 master:N/layout:K/fld:…".into()
            }
            IdReason::InsertFieldNeedsShape => {
                "pptx 的 insertField 的 id 必须是幻灯片、备注、母版或版式文本形状 id".into()
            }
            IdReason::InsertNumberingNeedsDocument => {
                "insertNumbering 的 id 必须是 docx:document".into()
            }
            IdReason::InsertStyleNeedsDocument => {
                "insertStyle 的 id 必须是 docx:document".into()
            }
            IdReason::NumPrefix => "编号实例 id 必须是 num:N".into(),
            IdReason::NumIndexNotInt => "num 后面必须是正整数".into(),
            IdReason::NumIndexFromOne => "num 序号从 1 起".into(),
            IdReason::AbstractNumPrefix => "抽象编号 id 必须是 abstractNum:N".into(),
            IdReason::AbstractNumIndexNotInt => "abstractNum 后面必须是整数".into(),
            IdReason::SetParaNumberingNeedsPara => {
                "setParaNumbering 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::StylePrefix => "样式 id 必须是 style: 加上非空的 styleId".into(),
            IdReason::SetParaStyleNeedsPara => {
                "setParaStyle 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::SetRunFormatNeedsRun => "setRunFormat 的 id 必须是段落 id 加上 /run:N".into(),
            IdReason::SetRunStyleNeedsStoryRun => {
                "setRunStyle 的 id 必须是正文或页眉页脚的 run id".into()
            }
            IdReason::RunPrefix => "run id 必须是段落 id 加上 /run:N".into(),
            IdReason::RunIndexNotInt => "run 序号必须是整数".into(),
            IdReason::RunIndexFromOne => "run 序号从 1 起".into(),
            IdReason::InsertSectionNeedsBodyPara => {
                "insertSection 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::TocPrefix => "目录 id 必须是 toc:N".into(),
            IdReason::TocIndexNotInt => "toc 序号必须是正整数".into(),
            IdReason::TocIndexFromOne => "toc 序号从 1 起".into(),
            IdReason::InsertTocNeedsBodyPara => {
                "insertToc 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::CaptionPrefix => "题注 id 必须是 caption:N".into(),
            IdReason::CaptionIndexNotInt => "caption 序号必须是正整数".into(),
            IdReason::CaptionIndexFromOne => "caption 序号从 1 起".into(),
            IdReason::InsertCaptionNeedsBodyPara => {
                "insertCaption 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::XrefPrefix => "交叉引用 id 必须是 xref:N".into(),
            IdReason::XrefIndexNotInt => "xref 序号必须是正整数".into(),
            IdReason::XrefIndexFromOne => "xref 序号从 1 起".into(),
            IdReason::InsertCrossRefNeedsPara => {
                "insertCrossRef 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::IndexPrefix => "索引 id 必须是 index:N".into(),
            IdReason::IndexIndexNotInt => "index 序号必须是正整数".into(),
            IdReason::IndexIndexFromOne => "index 序号从 1 起".into(),
            IdReason::InsertIndexNeedsBodyPara => {
                "insertIndex 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::XePrefix => "索引项 id 必须是 xe:N".into(),
            IdReason::XeIndexNotInt => "xe 序号必须是正整数".into(),
            IdReason::XeIndexFromOne => "xe 序号从 1 起".into(),
            IdReason::InsertIndexEntryNeedsPara => {
                "insertIndexEntry 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::SourcePrefix => "书目源 id 必须是 source:N".into(),
            IdReason::SourceIndexNotInt => "source 序号必须是正整数".into(),
            IdReason::SourceIndexFromOne => "source 序号从 1 起".into(),
            IdReason::InsertSourceNeedsDocument => "insertSource 的 id 必须是 docx:document".into(),
            IdReason::CitePrefix => "引文 id 必须是 cite:N".into(),
            IdReason::CiteIndexNotInt => "cite 序号必须是正整数".into(),
            IdReason::CiteIndexFromOne => "cite 序号从 1 起".into(),
            IdReason::InsertCitationNeedsPara => {
                "insertCitation 的 id 必须是正文或页眉页脚段落 id".into()
            }
            IdReason::BiblioPrefix => "书目块 id 必须是 biblio:N".into(),
            IdReason::BiblioIndexNotInt => "biblio 序号必须是正整数".into(),
            IdReason::BiblioIndexFromOne => "biblio 序号从 1 起".into(),
            IdReason::InsertBibliographyNeedsBodyPara => {
                "insertBibliography 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::DocPartPrefix => "构建基块 id 必须是 docPart:N".into(),
            IdReason::DocPartIndexNotInt => "docPart 序号必须是正整数".into(),
            IdReason::DocPartIndexFromOne => "docPart 序号从 1 起".into(),
            IdReason::InsertDocPartNeedsDocument => {
                "insertDocPart 的 id 必须是 docx:document".into()
            }
            IdReason::InsertDocPartContentNeedsBodyPara => {
                "insertDocPartContent 的 id 必须是 w:body 的直接子段落，或 docx:document".into()
            }
            IdReason::FontPrefix => "嵌入字体 id 必须是 font:N".into(),
            IdReason::FontIndexNotInt => "font 序号必须是正整数".into(),
            IdReason::FontIndexFromOne => "font 序号从 1 起".into(),
            IdReason::InsertEmbeddedFontNeedsDocument => {
                "insertEmbeddedFont 的 id 必须是 docx:document".into()
            }
            IdReason::SetFontEmbedNeedsDocument => {
                "setFontEmbed 的 id 必须是 docx:document".into()
            }
            IdReason::SetPageBackgroundNeedsDocument => {
                "setPageBackground 的 id 必须是 docx:document".into()
            }
            IdReason::MathPrefix => {
                "公式 id 必须是 math:N，或页眉页脚容器 id 加上 /math:N，或文本框 id 加上 /math:N".into()
            }
            IdReason::MathIndexNotInt => "公式序号必须是正整数".into(),
            IdReason::MathIndexFromOne => "公式序号从 1 起".into(),
            IdReason::InsertMathNeedsPara => {
                "insertMath 的 id 必须是正文、页眉页脚或框内段落 id，或 docx:document".into()
            }
            IdReason::RubyPrefix => {
                "拼音 id 必须是 ruby:N，或页眉页脚容器 id 加上 /ruby:N，或文本框 id 加上 /ruby:N".into()
            }
            IdReason::RubyIndexNotInt => "拼音序号必须是正整数".into(),
            IdReason::RubyIndexFromOne => "拼音序号从 1 起".into(),
            IdReason::InsertRubyNeedsPara => {
                "insertRuby 的 id 必须是正文、页眉页脚或框内段落 id，或 docx:document".into()
            }
            IdReason::SetEastAsianSettingsNeedsDocument => {
                "setEastAsianSettings 的 id 必须是 docx:document".into()
            }
            IdReason::SdtPrefix => {
                "内容控件 id 必须是 sdt:N，或页眉页脚容器 id 加上 /sdt:N，或文本框 id 加上 /sdt:N".into()
            }
            IdReason::SdtIndexNotInt => "内容控件序号必须是正整数".into(),
            IdReason::SdtIndexFromOne => "内容控件序号从 1 起".into(),
            IdReason::InsertSdtNeedsPara => {
                "insertContentControl 的 id 必须是正文、页眉页脚或框内段落 id，或 docx:document".into()
            }
            IdReason::SetPropsNeedsRoot => {
                "setProps / setCustomProp 的 id 必须是 xlsx:workbook、docx:document 或 pptx:presentation"
                    .into()
            }
            IdReason::CustomPropPrefix => "deleteCustomProp 的 id 必须是 prop:NAME".into(),
            IdReason::AnimForm => {
                "动画 id 必须是 slide:N/anim:K、slide:N/notes/anim:K、master:N/anim:K、master:N/layout:K/anim:M、notesMaster/anim:K 或 handout/anim:K；insertAnim 的 id 必须是可绑对象".into()
            }
            IdReason::AnimIndexNotInt => "anim 后面必须是正整数".into(),
            IdReason::AnimIndexFromOne => "动画序号从 1 起".into(),
            IdReason::InsertAnimNeedsHost => {
                "insertAnim 的 id 必须是幻灯片或备注上的形状、图、表、图表、SmartArt、视频或音频，或母版 / 版式、备注母版或讲义占位符".into()
            }
        },
        ErrorKind::SheetNotFound { name } => format!("找不到工作表：{name}"),
        ErrorKind::ParaNotFound { id } => format!("找不到段落：{id}"),
        ErrorKind::TableNotFound { id } => format!("找不到表：{id}"),
        ErrorKind::SectionNotFound { id } => format!("找不到节：{id}"),
        ErrorKind::CustomShowNotFound { id } => format!("找不到自定义放映：{id}"),
        ErrorKind::SlideNotFound { n } => format!("找不到幻灯片：slide:{n}"),
        ErrorKind::MasterNotFound { n } => format!("找不到幻灯片母版：master:{n}"),
        ErrorKind::ThemeNotFound { n } => format!("找不到该母版的 theme：master:{n}"),
        ErrorKind::DocumentThemeNotFound => "找不到文档 theme 部件".into(),
        ErrorKind::NotesThemeNotFound => "找不到备注母版的 theme".into(),
        ErrorKind::HandoutThemeNotFound => "找不到讲义母版的 theme".into(),
        ErrorKind::NotesSlideNotFound { n } => format!("找不到备注页：slide:{n}/notes"),
        ErrorKind::NotesMasterNotFound => "找不到备注母版".into(),
        ErrorKind::HandoutMasterNotFound => "找不到讲义母版".into(),
        ErrorKind::LayoutNotFound { master, layout } => {
            format!("找不到版式：master:{master}/layout:{layout}")
        }
        ErrorKind::ShapeNotFound { id } => format!("找不到形状：{id}"),
        ErrorKind::AnimNotFound { id } => format!("找不到动画：{id}"),
        ErrorKind::AnimHostNotFound { id } => format!("找不到可绑定动画的对象：{id}"),
        ErrorKind::ShapeHasNoText { id } => format!("形状没有文本框：{id}"),
        ErrorKind::PictureNotFound { id } => format!("找不到图：{id}"),
        ErrorKind::MediaNotFound { id } => format!("找不到视频或音频：{id}"),
        ErrorKind::TextBoxNotFound { id } => format!("找不到文本框：{id}"),
        ErrorKind::LastTextBoxPara => "文本框内至少要留一段，不能删掉最后一段".into(),
        ErrorKind::LastNotePara => "脚注、尾注内至少要留一段，不能删掉最后一段".into(),
        ErrorKind::NoteNotFound { id } => format!("找不到脚注或尾注：{id}"),
        ErrorKind::NeedNoteNumberingField => {
            "setNoteNumbering 至少要给 numFmt、numStart 或 numRestart 之一".into()
        }
        ErrorKind::InvalidNoteKind => "kind 必须是 footnote 或 endnote".into(),
        ErrorKind::InvalidNoteNumFmt => {
            "numFmt 必须是 decimal、upperRoman、lowerRoman、upperLetter、lowerLetter 或 chicago"
                .into()
        }
        ErrorKind::InvalidNoteRestart => {
            "numRestart 必须是 continuous、eachSect 或 eachPage".into()
        }
        ErrorKind::LastCommentPara => "批注内至少要留一段，不能删掉最后一段".into(),
        ErrorKind::LastStoryPara => {
            "正文、页眉、页脚或表格单元格至少要留一段，不能删掉最后一段".into()
        }
        ErrorKind::CrossParaMarker => {
            "不能删除书签、批注范围或许可范围跨出本段的段落".into()
        }
        ErrorKind::ParaHasRevision => "不能删除带修订标记的段落".into(),
        ErrorKind::SectionBreakPara => "不能删除带分节符的段落，先把节属性挪走".into(),
        ErrorKind::CannotDeleteLastSection => "不能删除最后一节，文档至少要留一节".into(),
        ErrorKind::InvalidSectionType => {
            "分节类型必须是 nextPage、continuous、evenPage、oddPage 或 nextColumn".into()
        }
        ErrorKind::SectionTypeOnLastPara => {
            "最后一段不能带 sectionType，末节的 sectPr 仍在 w:body".into()
        }
        ErrorKind::InvalidTocKind => {
            "kind 必须是 toc、tof 或 toa；instr 必须以 TOC 或 TOA 开头".into()
        }
        ErrorKind::InvalidSeqIdentifier => {
            "label 必须是 SEQ 序列名：拉丁字母开头，其后最多 39 个拉丁字母、数字或下划线".into()
        }
        ErrorKind::InvalidCrossRefKind => "kind 只能是 ref 或 page".into(),
        ErrorKind::InvalidIndexInstr => "instr 必须以 INDEX 开头".into(),
        ErrorKind::InvalidIndexEntry => {
            "entry 不能为空、不能只含空白、也不能含英文双引号".into()
        }
        ErrorKind::InvalidSourceTag => {
            "tag 不能为空、不能含空白、也不能含英文双引号".into()
        }
        ErrorKind::DuplicateSourceTag => "tag 不能与已有书目源重名".into(),
        ErrorKind::InvalidSourceType => "sourceType 不是本期认的 ST_SourceType".into(),
        ErrorKind::EmptySourceTitle => "title 不能为空".into(),
        ErrorKind::InvalidCitationTag => {
            "tag 不能为空、不能含空白、也不能含英文双引号".into()
        }
        ErrorKind::InvalidBibliographyInstr => "instr 必须以 BIBLIOGRAPHY 开头".into(),
        ErrorKind::EmptyDocPartName => "name 不能为空".into(),
        ErrorKind::DuplicateDocPartName => "name、gallery 与 category 不能与已有构建基块相同".into(),
        ErrorKind::InvalidDocPartGallery => "gallery 不是本期认的 ST_DocPartGallery".into(),
        ErrorKind::InvalidDocPartBehavior => "behavior 只能是 content、p 或 pg".into(),
        ErrorKind::EmptyDocPartText => "text 不能为空".into(),
        ErrorKind::CommentNotFound { id } => format!("找不到批注：{id}"),
        ErrorKind::NeedCommentParaId => {
            "批注最后一段必须有 w14:paraId，才能写 commentsExtended".into()
        }
        ErrorKind::NeedCommentMetaField => {
            "setCommentMeta 至少要给 author、initials 或 date 之一".into()
        }
        ErrorKind::CommentRangeCrossPart => {
            "insertComment 的 start 与 end 必须在同一部件".into()
        }
        ErrorKind::CommentRangeReversed => {
            "insertComment 的 start 不能晚于 end".into()
        }
        ErrorKind::InkComment { id } => format!("墨迹批注不能改字、回复或在其内插入 drawing：{id}"),
        ErrorKind::ChartNotFound { id } => format!("找不到图表：{id}"),
        ErrorKind::ChartSeriesNotFound { id } => format!("找不到图表系列：{id}"),
        ErrorKind::TrendlineNotFound { id } => format!("找不到趋势线：{id}"),
        ErrorKind::ErrorBarsNotFound { id } => format!("找不到误差线：{id}"),
        ErrorKind::NeedChartSeriesField => {
            "setChartSeries 至少要给 name、categories、values 或 sizes 之一".into()
        }
        ErrorKind::NeedTrendlineField => {
            "setTrendline 至少要给 type、order、period、displayRSquared、displayEquation 或 name 之一".into()
        }
        ErrorKind::NeedErrorBarsField => {
            "setErrorBars 至少要给 dir、barType、valType 或 value 之一".into()
        }
        ErrorKind::InvalidTrendlineType => {
            "type 必须是 linear、exp、log、poly、power 或 movingAvg".into()
        }
        ErrorKind::TrendlineOrderNotPoly => "order 只对 poly 有意义".into(),
        ErrorKind::TrendlinePeriodNotMovingAvg => "period 只对 movingAvg 有意义".into(),
        ErrorKind::TrendlineNotSupported => "这种图表类型不支持趋势线".into(),
        ErrorKind::InvalidErrorBars => {
            "dir 必须是 x 或 y；barType 必须是 both、plus 或 minus；valType 必须是 stdErr、stdDev、fixedVal、percentage 或 cust".into()
        }
        ErrorKind::CustomErrorBars => "valType 为 cust 时必须给 plus 与 minus 数组".into(),
        ErrorKind::NeedErrorBarsValue => {
            "valType 为 stdDev、fixedVal 或 percentage 时必须给 value".into()
        }
        ErrorKind::ErrorBarsValueOnStdErr => "valType 为 stdErr 或 cust 时不能给 value".into(),
        ErrorKind::ErrorBarsNotSupported => "这种图表类型不支持误差线".into(),
        ErrorKind::ErrorBarsDuplicateDir => "该系列这个方向已经有误差线".into(),
        ErrorKind::SecondaryAxisNotSupported => "这种图表类型不支持次轴".into(),
        ErrorKind::InvalidSeriesAxis => "axis 必须是 primary 或 secondary".into(),
        ErrorKind::LastChartSeries => "不能删光图表里的全部系列，整张去掉请用 deleteChart".into(),
        ErrorKind::RestrictedChart { id } => {
            format!("此图表不能按此操作编辑：{id}")
        }
        ErrorKind::InvalidChartType => {
            "type 必须是 bar、col、line、pie、area、scatter、doughnut、radar、bubble、stock、surface 或 ofPie；insertChart 还可为 combo 或 chartex".into()
        }
        ErrorKind::InvalidTransitionType => {
            "type 必须是可写的幻灯片切换名，或 null 表示清除".into()
        }
        ErrorKind::InvalidAnimType => {
            "type 必须是 appear、fade、flyIn、wipe、split、zoom、spin、growShrink、fadeOut、flyOut、wipeOut 或 pathRight".into()
        }
        ErrorKind::InvalidAnimTrigger => "trigger 必须是 click、with、after 或 clickOn".into(),
        ErrorKind::InvalidAnimDelay => "delay 必须是 0 到 60000 的整数".into(),
        ErrorKind::InvalidAnimDur => "dur 必须是 1 到 60000 的整数".into(),
        ErrorKind::InvalidAnimRepeat => "repeat 必须是 1 到 99 的整数，或 null".into(),
        ErrorKind::InvalidAnimPara => "para 必须是目标上从 1 计的段落序号".into(),
        ErrorKind::InvalidAnimMoveIndex => "index 必须是该宿主动画名单上从 1 计的位置".into(),
        ErrorKind::NeedAnimSetField => {
            "setAnim 至少要给一个可写字段".into()
        }
        ErrorKind::AnimNeedEffectDur => "dur 只能写在带时长行为的效果上".into(),
        ErrorKind::InvalidTransitionDur => "dur 必须是 1 到 60000 的整数".into(),
        ErrorKind::InvalidTransitionAdvTm => "advTm 必须是 1 到 86400000 的整数".into(),
        ErrorKind::InvalidTransitionDir => {
            "dir 必须是该切换允许的方向；invX / invY 只对 prstTrans 预设有效".into()
        }
        ErrorKind::InvalidTransitionOrient => {
            "orient 只对 split 有效，且必须是 horz 或 vert".into()
        }
        ErrorKind::InvalidTransitionSpokes => {
            "spokes 只对 wheel / wheelReverse 有效，且必须是 1 到 8 的整数".into()
        }
        ErrorKind::InvalidShapePrst => {
            "prst 必须是可写的预设几何名；连接线几何请用 insertConnector".into()
        }
        ErrorKind::InvalidGeomOp => {
            "路径命令只允许 moveTo、lnTo、close".into()
        }
        ErrorKind::ConnectorEndHost => {
            "start / end 必须与连接线同一宿主".into()
        }
        ErrorKind::NeedConnectorBind => {
            "setConnector 必须给 start 或 end".into()
        }
        ErrorKind::InvalidConnectorPrst => {
            "prst 必须是可写的连接线几何名".into()
        }
        ErrorKind::GroupNeedsTwo => "groupShapes 的 ids 至少两个".into(),
        ErrorKind::GroupMemberNested => {
            "只能组合尚未编组的直接子对象，不能把已在组合里的对象再编一层".into()
        }
        ErrorKind::NotGrouped => "该对象不在组合里".into(),
        ErrorKind::ConnectorNotFound { id } => format!("找不到连接线：{id}"),
        ErrorKind::InvalidChartAxis => "axis 必须是 cat 或 val，且图表里要有对应主轴".into(),
        ErrorKind::InvalidLegendPos => "pos 必须是 b、t、l、r 或 tr".into(),
        ErrorKind::PieNeedsOneSeries => "饼图、圆环图与复合饼必须恰好一条系列".into(),
        ErrorKind::StockNeedsThreeSeries => {
            "股价图系列条数必须与 stockKind 一致：hlc 三条，ohlc 或 vhlc 四条，vohlc 五条".into()
        }
        ErrorKind::SurfaceNeedsThreeSeries => "曲面图必须恰好三条系列".into(),
        ErrorKind::InvalidOfPieType => {
            "ofPieType 必须是 pie 或 bar，且只对 type 为 ofPie 有意义".into()
        }
        ErrorKind::InvalidStockKind => {
            "stockKind 必须是 hlc、ohlc、vhlc 或 vohlc，且只对 type 为 stock 有意义；setChartType 不能用 vhlc 或 vohlc".into()
        }
        ErrorKind::InvalidSurfaceKind => {
            "surfaceKind 必须是 surface3d、wireframe、contour 或 contourWire，且只对 type 为 surface 有意义".into()
        }
        ErrorKind::InvalidLayoutId => {
            "layoutId 必须是 waterfall、treemap、sunburst、funnel、clusteredColumn、boxWhisker 或 pareto，且只对 type 为 chartex 有意义".into()
        }
        ErrorKind::RegionMapChart => "regionMap 地图图本期不做".into(),
        ErrorKind::SizesOnNonBubble => "sizes 只对气泡系列有意义".into(),
        ErrorKind::InvalidComboSeriesType => {
            "系列 type 必须是 bar、col、line、area 或 scatter，且只有组合图才能带".into()
        }
        ErrorKind::ChartTypeNotConvertible => "这种图表类型无法无损转换".into(),
        ErrorKind::NeedChartCategories => {
            "addChartSeries 省略 categories 时图表里必须已有系列可抄".into()
        }
        ErrorKind::SmartArtNotFound { id } => format!("找不到 SmartArt：{id}"),
        ErrorKind::OleNotFound { id } => format!("找不到嵌入对象：{id}"),
        ErrorKind::InkNotFound { id } => format!("找不到墨迹：{id}"),
        ErrorKind::Model3dNotFound { id } => format!("找不到 3D 模型：{id}"),
        ErrorKind::MathDrawingNotFound { id } => format!("找不到公式框：{id}"),
        ErrorKind::MathDrawingNoExpr => {
            "xlsx / pptx 公式框只认非空 text，不要给 expr 或 kind".into()
        },
        ErrorKind::SlicerNotFound { id } => format!("找不到切片器：{id}"),
        ErrorKind::SlicerFieldNotFound { name } => {
            format!("切片器字段对不上源：{name}")
        }
        ErrorKind::InvalidSlicer { field } => format!("切片器字段不合法：{field}"),
        ErrorKind::NeedSetSlicerViewField => {
            "setSlicerView 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::NeedSetSlicerFilterField => {
            "setSlicerFilter 必须给 selected 或 filter".into()
        }
        ErrorKind::ConnectionNotFound { id } => format!("找不到连接：{id}"),
        ErrorKind::InvalidConnection { field } => {
            format!("连接的 {field} 不合法")
        }
        ErrorKind::ExternalLinkNotFound { id } => format!("找不到外部引用：{id}"),
        ErrorKind::InvalidExternalLink { field } => {
            format!("外部引用的 {field} 不合法")
        }
        ErrorKind::ExternalLinkInUse => "公式或定义名仍在使用该外部引用".into(),
        ErrorKind::CustomViewNotFound { id } => format!("找不到自定义视图：{id}"),
        ErrorKind::InvalidCustomView { field } => {
            format!("自定义视图的 {field} 不合法")
        }
        ErrorKind::CellStyleNotFound { id } => format!("找不到命名单元格样式：{id}"),
        ErrorKind::NeedInsertCellStyleField => {
            "insertCellStyle 至少要给 numFmt、font、fill、border 或 align 之一".into()
        }
        ErrorKind::InvalidNamedCellStyle { field } => {
            format!("命名单元格样式的 {field} 不合法")
        }
        ErrorKind::CannotDeleteBuiltinCellStyle => "不能删除内置单元格样式".into(),
        ErrorKind::ZoomNotFound { id } => format!("找不到 Zoom：{id}"),
        ErrorKind::InvalidZoomType => "type 必须是 slide、section 或 summary".into(),
        ErrorKind::InvalidShowPrType => "type 必须是 present、browse 或 kiosk".into(),
        ErrorKind::InvalidFldType => {
            "type 必须是 slidenum、datetime 或 datetime1–datetime13".into()
        }
        ErrorKind::ZoomTargetSelf => "insertZoom 的 target 不能是宿主自己".into(),
        ErrorKind::NeedZoomSetField => "setZoom 必须给 target、targets、file 或几何之一".into(),
        ErrorKind::InvalidZoomTargets => {
            "target 与 targets 必须与 Zoom 种类一致，且 targets 不能为空或重复".into()
        }
        ErrorKind::WatermarkNotFound { id } => format!("找不到水印：{id}"),
        ErrorKind::WatermarkNeedTextOrFile => {
            "insertWatermark 必须恰好给 text 或 file 之一".into()
        }
        ErrorKind::SmartArtPointNotFound { id } => format!("找不到 SmartArt 点：{id}"),
        ErrorKind::LastSmartArtPoint => {
            "不能删光 SmartArt 里的全部可见点，整张去掉请用 deleteSmartArt".into()
        }
        ErrorKind::InvalidSmartArtLayout => "layout 必须是 list、org 或 cycle".into(),
        ErrorKind::InvalidSmartArtRole => "role 必须是 node 或 asst".into(),
        ErrorKind::InvalidSmartArtColors => {
            "colors 必须是 accent1、accent2、accent3、accent4、accent5 或 accent6".into()
        }
        ErrorKind::InvalidSmartArtStyle => "style 必须是 simple、subtle 或 intense".into(),
        ErrorKind::SmartArtLayoutNotConvertible => "这种 SmartArt 版式无法无损转换".into(),
        ErrorKind::SmartArtPointParent => {
            "points[].parent 必须是本数组下标，且不能成环".into()
        }
        ErrorKind::SmartArtAsstOnList => "list / cycle 版式不能带 asst 点".into(),
        ErrorKind::SmartArtListDepth => "list / cycle 版式只接受顶层点（parent 必须省略或为 null）".into(),
        ErrorKind::TextBoxLinkCrossPart => "链接文本框必须在同一部件里".into(),
        ErrorKind::InvalidHexColor => "color 必须是六位 RGB 十六进制或 null".into(),
        ErrorKind::NeedShapeFormatField => {
            "setShapeFormat 至少要给 sz、color、textFill、textOutline、bold、italic、underline、font、eastAsiaFont、align、textAnchor、spcBef、spcAft、lnSpc、spc 或 kern 之一".into()
        }
        ErrorKind::InvalidTextFillType => "textFill.type 必须是 none 或 gradient".into(),
        ErrorKind::InvalidTableCellFillType => {
            "表单元格 fill.type 必须是 solid、grad、patt 或 blip".into()
        }
        ErrorKind::InvalidPresetPatternVal => "prst 必须是 ST_PresetPatternVal".into(),
        ErrorKind::NeedSetTableCellPrField => {
            "setTableCellPr 至少要给 vert、anchorCtr、horzOverflow、cell3D 或 headers 之一".into()
        }
        ErrorKind::InvalidTextVerticalType => "vert 必须是 ST_TextVerticalType".into(),
        ErrorKind::InvalidHorzOverflow => "horzOverflow 必须是 overflow 或 clip".into(),
        ErrorKind::InvalidBevelPreset => "bevel.prst 必须是 ST_BevelPresetType".into(),
        ErrorKind::InvalidPresetMaterial => "prstMaterial 必须是 ST_PresetMaterialType".into(),
        ErrorKind::NeedSetTableStyleRefField => {
            "setTableStyleRef 至少要给 tableStyleId 或 tblLook 之一".into()
        }
        ErrorKind::InvalidTableStyleId => "tableStyleId 必须是 ST_Guid".into(),
        ErrorKind::NeedSetThemeObjectDefaultsField => {
            "setThemeObjectDefaults 至少要给 sp、ln 或 tx 之一".into()
        }
        ErrorKind::NeedSetModel3dCameraField => {
            "setModel3dCamera 至少要给 camera、rot 或 lightRig 之一".into()
        }
        ErrorKind::InvalidFontRefIdx => "fontRef 必须是 major、minor 或 none".into(),
        ErrorKind::NeedGradientStops => "textFill.stops 至少要有两项".into(),
        ErrorKind::DuplicateGradientPos => "textFill.stops 的 pos 不能重复".into(),
        ErrorKind::NeedTextOutlineField => "textOutline 必须有 color 与 width".into(),
        ErrorKind::InvalidTextAlign => "align 必须是 l、ctr、r 或 just".into(),
        ErrorKind::InvalidTextAnchor => "textAnchor 必须是 t、ctr、b、just 或 dist".into(),
        ErrorKind::InvalidTextSpacing => "spcBef、spcAft 与 lnSpc 的 type 必须是 pts 或 pct".into(),
        ErrorKind::InvalidTextUnderline => "underline 必须是 sng、none 或 dbl".into(),
        ErrorKind::EmptyShapeTypeface => "font 与 eastAsiaFont 不能为空串".into(),
        ErrorKind::InvalidTextBoxDirection => "direction 必须是 horizontal 或 vertical".into(),
        ErrorKind::InvalidTextBoxTextAnchor => {
            "textAnchor 必须是 t、ctr、b、just 或 dist".into()
        }
        ErrorKind::InvalidPicKind => "kind 必须是 inline 或 anchor".into(),
        ErrorKind::InvalidPicWrap => {
            "wrap 必须是 square、tight、through、topAndBottom 或 none".into()
        }
        ErrorKind::InvalidPicAlign => "alignH 只允许 left/center/right，alignV 只允许 top/center/bottom".into(),
        ErrorKind::InvalidImageFile => {
            "file 不是本库认的图片（PNG、JPEG、GIF、BMP、SVG、EMF、WMF 或 TIFF）".into()
        }
        ErrorKind::InvalidInkMl => {
            "file 不是 InkML（根元素必须是 InkML 命名空间下的 ink）".into()
        }
        ErrorKind::InvalidMediaFile => {
            "file 不是本库认的媒体（insertVideo 认 MP4，insertAudio 认 WAV，字幕认 WebVTT）".into()
        }
        ErrorKind::NeedSetMediaField => "setMedia 至少要给 st、end、bookmarks 或 tracks".into(),
        ErrorKind::InvalidMediaTime => "st、end 与书签 time 必须是非负数字".into(),
        ErrorKind::DuplicateBookmarkName => "同一媒体上的书签名不能重复".into(),
        ErrorKind::InvalidTrack => {
            "字幕轨或书签的 name / label / id / trackDisplayLoc 不合法".into()
        }
        ErrorKind::NeedsPptxSectionList => {
            "文件还没有幻灯片节名单".into()
        }
        ErrorKind::SlideNotInPptxSection => {
            "该页不在任何幻灯片节里".into()
        }
        ErrorKind::SetCellWorksheetOnly => "只能对工作表 setCell".into(),
        ErrorKind::InspectWorksheetOnly => "只能对工作表读单元格".into(),
        ErrorKind::InsertPictureWorksheetOnly => "只能往工作表 insertPicture".into(),
        ErrorKind::InsertChartWorksheetOnly => "只能往工作表 insertChart".into(),
        ErrorKind::InsertTextBoxWorksheetOnly => "只能往工作表插入或改文本框、形状与连接线".into(),
        ErrorKind::InsertModel3dWorksheetOnly => "只能往工作表插入 3D 模型".into(),
        ErrorKind::InsertInkWorksheetOnly => "只能往工作表插入墨迹".into(),
        ErrorKind::InsertMathWorksheetOnly => "只能往工作表插入公式框".into(),
        ErrorKind::InsertSmartArtWorksheetOnly => "只能往工作表 insertSmartArt / 改图示".into(),
        ErrorKind::CommentsWorksheetOnly => "只能对工作表发批注 op".into(),
        ErrorKind::InsertHyperlinkWorksheetOnly => "只能对工作表发超链接 op".into(),
        ErrorKind::InsertSparklineWorksheetOnly => "只能对工作表发迷你图 op".into(),
        ErrorKind::ProtectedRangeWorksheetOnly => "只能对工作表发允许编辑区域 op".into(),
        ErrorKind::DataSheetMustBeWorksheet => {
            "图表工作表的数据必须写到普通工作表上".into()
        }
        ErrorKind::TwoCellPictureSize => {
            "twoCell 图请在 setPictureSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellTextBoxSize => {
            "twoCell 文本框、形状或连接线请在 setTextBoxSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellSmartArtSize => {
            "twoCell SmartArt 请在 setSmartArtSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellModel3dSize => {
            "twoCell 3D 请在 setModel3dSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellInkSize => {
            "twoCell 墨迹请在 setInkSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellOleSize => {
            "twoCell 嵌入对象请在 setOleSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellSlicerSize => {
            "twoCell 切片器请在 setSlicerSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellMathSize => {
            "twoCell 公式框请在 setMathSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::TwoCellChartSize => {
            "twoCell 图表请在 setChartSize 里给 to，不要给 cx / cy".into()
        }
        ErrorKind::DeleteChartsheetLast => {
            "不能删掉图表工作表上仅剩的图表，否则会留下空的图表工作表".into()
        }
        ErrorKind::ChartFormulaNotA1 => {
            "系列公式不是连续 A1 区域（命名区域、三维引用或不连续区域本期不写）".into()
        }
        ErrorKind::InvalidXlsxAnchor => "anchor 必须是 oneCell 或 twoCell".into(),
        ErrorKind::InvalidXlsxEditAs => "editAs 必须是 twoCell、oneCell 或 absolute".into(),
        ErrorKind::NotesSlideImage => "不能改 type 为 sldImg 的占位符".into(),
        ErrorKind::GraphicInRestrictedStory => {
            "不能把图形对象放到文本框、标注、批注、脚注或尾注中".into()
        }
        ErrorKind::DocxShapeNeedsAnchor => {
            "docx 的 setShapePos 只能改浮动形状（需要 wp:anchor）".into()
        }
        ErrorKind::NeedPropsField => {
            "setProps 至少要给 title、subject、creator、description、keywords、category、company、manager 之一"
                .into()
        }
        ErrorKind::NeedProtectionField => {
            "setProtection 至少要给一个可写字段".into()
        }
        ErrorKind::InvalidProtectionEdit => {
            "edit 必须是 readOnly、comments、trackedChanges 或 forms".into()
        }
        ErrorKind::InvalidSheetProtection { field } => {
            format!("图表工作表不认 {field}")
        }
        ErrorKind::ProtectionPassword => "setProtection 不写密码哈希".into(),
        ErrorKind::ProtectionHasPassword => {
            "这份文件带密码哈希，不能改保护字段".into()
        }
        ErrorKind::ProtectionPasswordBlank => {
            "password 与 writePassword 必须是非空字符串".into()
        }
        ErrorKind::ProtectionPasswordWrong => "口令与已有 SHA-512 哈希对不上".into(),
        ErrorKind::ProtectionPasswordNeedsDoc => {
            "password 只能配限制编辑，不能只配 readOnlyRecommended".into()
        }
        ErrorKind::InvalidCustomPropType => {
            "自定义属性 type 必须是 lpwstr、i4、r8 或 bool，且必须与 value 对得上".into()
        }
        ErrorKind::EmptyCustomPropName => "自定义属性 name 必须是非空字符串".into(),
        ErrorKind::CustomPropNotFound { id } => format!("找不到自定义属性：{id}"),
        ErrorKind::SharedFormulaMaster { cell } => format!(
            "{cell} 是仍有从属格的共享公式主格，mergeRange 不能盖住它"
        ),
        ErrorKind::DataTableFormula { cell } => {
            format!("{cell} 属于数据表公式，请改主格的 TABLE 公式")
        }
        ErrorKind::InvalidDataTableFormula => {
            "数据表主格公式必须是与现有行列方向一致的 TABLE".into()
        }
        ErrorKind::ArrayFormulaPart { cell } => {
            format!("{cell} 落在数组公式范围内，请改左上角主格")
        }
        ErrorKind::FormulaGroupOverlap { cell } => {
            format!("{cell} 已经属于另一组共享公式、数组公式或数据表公式")
        }
        ErrorKind::FormulaRangeNotTopLeft => {
            "id 必须是 sharedRef 或 arrayRef 的左上角".into()
        }
        ErrorKind::SharedNeedsRef => {
            "对还不是共享主格的格子写 shared: true 时必须给 sharedRef".into()
        }
        ErrorKind::NeedAtLeastOneSheet => "至少需要一张工作表".into(),
        ErrorKind::LastSlide => "至少需要一页幻灯片".into(),
        ErrorKind::LastLayout => "该母版至少需要一份版式".into(),
        ErrorKind::LastMaster => "至少需要一份幻灯片母版".into(),
        ErrorKind::LayoutInUse => "仍有幻灯片使用该版式".into(),
        ErrorKind::MasterInUse => "仍有幻灯片使用该母版下的版式".into(),
        ErrorKind::InvalidPlaceholderType => {
            "type 必须是可写的占位符类型".into()
        }
        ErrorKind::InvalidSlideSizeType => {
            "type 必须是可写的幻灯片尺寸类型".into()
        }
        ErrorKind::SlideSizeOutOfRange => {
            "cx / cy 必须是 914400 到 51206400 的整数".into()
        }
        ErrorKind::LastVisibleSheet => "不能隐藏或删除最后一张可见工作表".into(),
        ErrorKind::MoveSheetAfterSelf => "moveSheet 的 after 不能是自己".into(),
        ErrorKind::MoveSlideAfterSelf => "moveSlide 的 after 不能是自己".into(),
        ErrorKind::SlideLayoutMissing => "找不到可用的幻灯片版式".into(),
        ErrorKind::SheetStillReferenced { name } => {
            format!("定义名或公式仍引用工作表 {name}")
        }
        ErrorKind::DialogOrMacroSheet => {
            "对话表和宏表不能删除、改名、重排或隐藏".into()
        }
        ErrorKind::DuplicateSheetName { name } => format!("工作表名重复：{name}"),
        ErrorKind::InvalidSheetName { reason } => match reason {
            SheetNameReason::Missing => "缺少工作表名".into(),
            SheetNameReason::Length => "工作表名长度必须是 1 到 31 个字符".into(),
            SheetNameReason::History => "工作表不能叫 History".into(),
            SheetNameReason::IllegalChars => "工作表名含有非法字符".into(),
        },
        ErrorKind::InvalidParaStyle => "style 只允许 Normal、Heading1、Heading2、Heading3".into(),
        ErrorKind::InvalidHfKind => "kind 必须是 header 或 footer".into(),
        ErrorKind::InvalidHfType => "type 必须是 default、even 或 first".into(),
        ErrorKind::CreateHfDuplicate => "create 每一侧同一 type 只能出现一次".into(),
        ErrorKind::InvalidXlsxHfKind => "kind 必须是 odd、even 或 first".into(),
        ErrorKind::InvalidXlsxHfSide => "side 必须是 header 或 footer".into(),
        ErrorKind::CreateXlsxHfDuplicate => "create 同一 kind 与 side 只能出现一次".into(),
        ErrorKind::NeedXlsxHfContent => {
            "headerFooters 每一项至少要有非空 text 或一张图".into()
        }
        ErrorKind::CannotLinkFirstSection => "第一节不能链接到前一节".into(),
        ErrorKind::HfTypeInactive { kind, hf_type } => {
            format!("本节未启用这种 {kind}（{hf_type}）")
        }
        ErrorKind::HfInsertWhileLinked => {
            "本节该页眉页脚仍链接前节，请用 setHeaderFooterLink 断开".into()
        }
        ErrorKind::InvalidSectValue { field } => format!("{field} 不是允许的取值"),
        ErrorKind::LastTableRow => "表里至少要留一行，不能删掉最后一行".into(),
        ErrorKind::LastTableCol => "表里至少要留一列，不能删掉最后一列".into(),
        ErrorKind::SplitContinueCell => "splitCells 必须点名 restart 格，不能点 continue 格".into(),
        ErrorKind::NothingToSplit => "这一格没有合并".into(),
        ErrorKind::TableHasRevision => "表、行或单元格上有修订痕迹时不能改结构".into(),
        ErrorKind::TableGridMismatch => "这一行占的网格列数对不上 colCount，不能改结构".into(),
        ErrorKind::MergeNotSimple => "mergeCells 需要一块尚未合并、对齐网格的矩形".into(),
        ErrorKind::NeedTableCellsMatch => "cells 必须是 rows×cols 的二维数组".into(),
        ErrorKind::NeedTableColWidthsMatch => {
            "colWidths / cols 的长度必须等于 colCount".into()
        }
        ErrorKind::NeedTableRowHeightsMatch => "rowHeights 的长度必须等于行数".into(),
        ErrorKind::RevisionNotFound { id } => format!("找不到修订：{id}"),
        ErrorKind::SetParaTextHasRevision => "不能改带修订痕迹的段落的可见文本".into(),
        ErrorKind::ParaRunMismatch { text } => {
            format!("这段可见文字里对不上：{text}")
        }
        ErrorKind::ShapeRunMismatch { text } => {
            format!("这段形状文字里对不上：{text}")
        }
        ErrorKind::HyperlinkNotFound { id } => format!("找不到超链接：{id}"),
        ErrorKind::BookmarkNotFound { id } => format!("找不到书签：{id}"),
        ErrorKind::EditRangeNotFound { id } => format!("找不到例外编辑区：{id}"),
        ErrorKind::InvalidEdGrp => {
            "edGrp 必须是 everyone、administrators、contributors、editors、owners 或 current"
                .into()
        }
        ErrorKind::EmptyEditRangeEd => "ed 必须是非空字符串".into(),
        ErrorKind::InvalidBibliographyStyle => {
            "styleName 必须是 APA、MLA、Chicago、IEEE 或 GB7714".into()
        }
        ErrorKind::SetParaTextHasHyperlink => "不能改带超链接的段落的可见文本".into(),
        ErrorKind::HyperlinkNeedTarget => "必须恰好给 url 或 anchor 之一".into(),
        ErrorKind::XlsxHyperlinkNeedTarget => {
            "xlsx 的超链接必须至少给 url 或 location 之一".into()
        }
        ErrorKind::EmptyHyperlinkUrl => "url 不能为空".into(),
        ErrorKind::EmptyHyperlinkLocation => "location 不能为空".into(),
        ErrorKind::HyperlinkOverlap => "超链接区域与同一张表上已有链相交".into(),
        ErrorKind::SparklineNotFound { id } => format!("找不到迷你图组：{id}"),
        ErrorKind::SparklineOverlap => "迷你图宿主格与同一张表上已有迷你图相交".into(),
        ErrorKind::SparklineHostNotCell => "迷你图 ref 必须是单格".into(),
        ErrorKind::SparklineFormulaNotLine => "迷你图 formula 必须是单行或单列".into(),
        ErrorKind::SparklineGroupHasMany => {
            "组内有多条迷你图时 setSparkline 不能改 ref 或 formula".into()
        }
        ErrorKind::EmptySparklineFormula => "formula 不能为空".into(),
        ErrorKind::InvalidSparklineType => "type 只能是 line、column 或 stacked".into(),
        ErrorKind::InvalidSparkline { field } => format!("迷你图字段不合法：{field}"),
        ErrorKind::NeedSetSparklineField => {
            "setSparkline 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidBookmarkName => {
            "书签名不能为空、不能含空白、不能以下划线开头".into()
        }
        ErrorKind::BookmarkNameTaken => "书签名已被占用".into(),
        ErrorKind::CustomShowNameTooLong => "自定义放映名字不能超过 31 个字符".into(),
        ErrorKind::CustomShowNameTaken => "自定义放映名字已被占用".into(),
        ErrorKind::NeedCustomShowField => {
            "setCustomShow 至少要给 name 或 slides 之一".into()
        },
        ErrorKind::BookmarkRangeReversed => "insertBookmark 的 end 不能早于起始段".into(),
        ErrorKind::BookmarkRangeCrossStory => {
            "insertBookmark 的 end 必须与 id 在同一故事".into()
        }
        ErrorKind::NeedSetHyperlinkField => {
            "setHyperlink 至少要给 url、anchor、text 或 tooltip 之一".into()
        }
        ErrorKind::NeedXlsxSetHyperlinkField => {
            "xlsx 的 setHyperlink 至少要给 url、location、tooltip、display 或 ref 之一".into()
        }
        ErrorKind::PptxHyperlinkNeedTarget => {
            "pptx 的超链接必须恰好给 url、file 或 slide 之一".into()
        }
        ErrorKind::PptxNeedSetHyperlinkField => {
            "pptx 的 setHyperlink 至少要给 url、file、slide 或 tooltip 之一".into()
        }
        ErrorKind::ShapeAlreadyHasHyperlink => {
            "该对象已有同事件的框级超链接，请用 setHyperlink".into()
        }
        ErrorKind::InvalidHyperlinkOn => "on 必须与 id 种类一致".into(),
        ErrorKind::InvalidHyperlinkEvent => "event 必须是 click 或 hover".into(),
        ErrorKind::RunAlreadyHasHyperlink => "该 run 已有同事件超链接，请用 setHyperlink".into(),
        ErrorKind::PptxRunLinkNeedsRun => "on 为 run 时需要已有文本 run，或给 text".into(),
        ErrorKind::PptxHyperlinkSelfSlide => "不能把超链接跳到本页".into(),
        ErrorKind::EmptyHyperlinkFile => "file 不能为空".into(),
        ErrorKind::FieldNotFound { id } => format!("找不到域：{id}"),
        ErrorKind::TocNotFound { id } => format!("找不到目录：{id}"),
        ErrorKind::CaptionNotFound { id } => format!("找不到题注：{id}"),
        ErrorKind::CrossRefNotFound { id } => format!("找不到交叉引用：{id}"),
        ErrorKind::IndexNotFound { id } => format!("找不到索引：{id}"),
        ErrorKind::IndexEntryNotFound { id } => format!("找不到索引项：{id}"),
        ErrorKind::SourceNotFound { id } => format!("找不到书目源：{id}"),
        ErrorKind::CitationNotFound { id } => format!("找不到引文：{id}"),
        ErrorKind::BibliographyNotFound { id } => format!("找不到书目：{id}"),
        ErrorKind::DocPartNotFound { id } => format!("找不到构建基块：{id}"),
        ErrorKind::FontNotFound { id } => format!("找不到嵌入字体：{id}"),
        ErrorKind::EmptyFontName => "name 不能为空".into(),
        ErrorKind::DuplicateEmbeddedFont => "name 与 style 不能与已有嵌入面相同".into(),
        ErrorKind::InvalidFontStyle => {
            "style 只能是 regular、bold、italic 或 boldItalic".into()
        }
        ErrorKind::InvalidFontFile => {
            "file 不是本库认的 TrueType / OpenType 字体（TTF/OTF）；不接受字体集合".into()
        }
        ErrorKind::NeedFontEmbedField => {
            "setFontEmbed 至少要给 trueType、systemFonts 或 subset 之一".into()
        }
        ErrorKind::InvalidPageBackgroundColor => {
            "color 必须是六位 RGB 十六进制，或 null".into()
        }
        ErrorKind::InvalidPageBackgroundFill => {
            "fill.kind 只能是 gradient 或 picture；color 为 null 时不能带 fill；picture 必须给 file".into()
        }
        ErrorKind::InvalidThemeFmtIndex => {
            "index 必须是 0、1 或 2，且 fmtScheme 列表里要有这一槽".into()
        }
        ErrorKind::InvalidThemeFmtKind => "kind 只能是 solid 或 grad".into(),
        ErrorKind::InvalidColorSchemeIndex => {
            "clrMap 槽值必须是 dk1、lt1、dk2、lt2、accent1–accent6、hlink 或 folHlink".into()
        }
        ErrorKind::NeedThemeOverrideField => {
            "setThemeOverride 至少要给一个配色槽，或只给 enabled: false".into()
        }
        ErrorKind::NeedPageBordersField => {
            "setPageBorders 至少要给 enabled、offsetFrom、display、zOrder 或某一边".into()
        }
        ErrorKind::InvalidPageBorderVal => {
            "val 必须是 ST_Border 取值，例如 single 或 apples".into()
        }
        ErrorKind::InvalidPageBorderOffset => "offsetFrom 只能是 page 或 text".into(),
        ErrorKind::InvalidPageBorderDisplay => {
            "display 只能是 allPages、firstPage 或 notFirstPage".into()
        }
        ErrorKind::InvalidPageBorderZOrder => "zOrder 只能是 front 或 back".into(),
        ErrorKind::InvalidPageBorderColor => {
            "边框 color 只能是 auto 或六位 RGB 十六进制".into()
        }
        ErrorKind::MathNotFound { id } => format!("找不到公式：{id}"),
        ErrorKind::RubyNotFound { id } => format!("找不到拼音：{id}"),
        ErrorKind::ContentControlNotFound { id } => format!("找不到内容控件：{id}"),
        ErrorKind::SetParaTextHasField => "不能改带域的段落的可见文本".into(),
        ErrorKind::SetParaTextHasMath => "不能改带公式的段落的可见文本".into(),
        ErrorKind::SetParaTextHasRuby => "不能改带拼音的段落的可见文本".into(),
        ErrorKind::SetParaTextHasSdt => "不能改带内容控件的段落的可见文本".into(),
        ErrorKind::EmptyFieldInstr => "instr 不能为空".into(),
        ErrorKind::FieldNeedInstrOrCode => "必须恰好给 instr 或 code 其中一个".into(),
        ErrorKind::InvalidFieldCode { got } => format!("不认识的域 code：{got}"),
        ErrorKind::FieldNeedArg { code } => format!("{code} 必须给 arg"),
        ErrorKind::EmptyMathText => "text 不能为空".into(),
        ErrorKind::InvalidMathKind => "kind 只能是 inline 或 display".into(),
        ErrorKind::InvalidMathJc => "jc 必须是 center、centerGroup、left 或 right".into(),
        ErrorKind::MathJcOnInline => "行内公式不能带 jc".into(),
        ErrorKind::MathNeedTextOrExpr => "必须恰好给 text 或 expr 其中一个".into(),
        ErrorKind::EmptyMathExpr => "expr 不能为空".into(),
        ErrorKind::InvalidMathExprType { got } => format!("不认识的公式节点 type：{got}"),
        ErrorKind::EmptyMathSlot { field } => format!("{field} 不能为空"),
        ErrorKind::InvalidContentControlKind => {
            "kind 只能是 plainText、richText、dropDown、comboBox、date、checkbox、picture、group 或 equation".into()
        }
        ErrorKind::EmptyContentControlText => "text 不能为空".into(),
        ErrorKind::InvalidContentControlKeep => "keep 只能是 content 或 none".into(),
        ErrorKind::InvalidContentControlItems => "items 必须是非空字符串组成的非空数组".into(),
        ErrorKind::ContentControlFieldMismatch => {
            "items 只用于 dropDown/comboBox，fullDate 只用于 date，file 只用于 picture，checked 只用于 checkbox".into()
        }
        ErrorKind::ContentControlBound => "不能改绑定到自定义 XML 的内容控件".into(),
        ErrorKind::ContentControlRepeating => "不能改重复节内容控件的值".into(),
        ErrorKind::ContentControlGroup => "不能改组合内容控件的值".into(),
        ErrorKind::ContentControlHasTable => "不能摊平含表格的块级内容控件".into(),
        ErrorKind::InvalidFieldKeep => "keep 只能是 result 或 instr".into(),
        ErrorKind::NeedSetFieldInstr => "setField 必须有 instr".into(),
        ErrorKind::NumberingNotFound { id } => format!("找不到编号实例：{id}"),
        ErrorKind::AbstractNumNotFound { id } => format!("找不到抽象编号：{id}"),
        ErrorKind::InvalidNumberingKind => "kind 只能是 decimal 或 bullet".into(),
        ErrorKind::InvalidListNumFmt => {
            "numFmt 只能是 decimal、bullet、lowerLetter、upperLetter、lowerRoman 或 upperRoman"
                .into()
        }
        ErrorKind::NeedSetNumberingField => {
            "setNumbering 至少要给 numFmt、start 或 lvlText 之一".into()
        }
        ErrorKind::InvalidIlvl => "ilvl 必须是 0 到 8 的整数，且该抽象定义上要有这一级".into(),
        ErrorKind::NeedSetParaNumberingNum => {
            "setParaNumbering 必须有 num（num: 实例 id 或 null）".into()
        }
        ErrorKind::StyleNotFound { id } => format!("找不到样式：{id}"),
        ErrorKind::SetStyleNeedsParagraph => "setParaStyle 只接受段落样式".into(),
        ErrorKind::InvalidStyleType => "insertStyle 与 setStyle 不接受编号样式".into(),
        ErrorKind::StyleIdTaken => "styleId 已被占用".into(),
        ErrorKind::InvalidStyleIdToken => {
            "styleId 必须是字母或下划线开头，其余只能是字母、数字、下划线或连字符".into()
        }
        ErrorKind::StyleNotCustom => "deleteStyle 只能删带 customStyle 的自定义样式".into(),
        ErrorKind::NeedInsertStyleField => "insertStyle 必须有 styleId 与 type".into(),
        ErrorKind::SetStyleSpacingNeedsParagraph => {
            "字符样式上的 setStyle 不能写 before 或 after".into()
        }
        ErrorKind::SetRunStyleNeedsCharacter => "setRunStyle 只接受字符样式".into(),
        ErrorKind::SetTableStyleNeedsTable => "setTableStyle 只接受表格样式".into(),
        ErrorKind::NeedSetTableStyleField => "setTableStyle 至少要给 style 或 look 之一".into(),
        ErrorKind::NeedSetTableFloatField => "setTableFloat 必须有 pos".into(),
        ErrorKind::InvalidTblAnchor => {
            "horzAnchor 与 vertAnchor 只能是 margin、page 或 text".into()
        }
        ErrorKind::NeedSetTableWidthField => {
            "setTableWidth 至少要给 type、w 或 cols 之一".into()
        }
        ErrorKind::NeedSetTableAlignField => {
            "setTableAlign 至少要给 align 或 indent 之一".into()
        }
        ErrorKind::NeedSetTableRowHeightField => "setTableRowHeight 必须有 h".into(),
        ErrorKind::InvalidTblWidthType => {
            "type 只能是 auto、dxa 或 pct；auto 时不要给大于 0 的 w".into()
        }
        ErrorKind::InvalidTblAlign => "align 必须是 left、center 或 right".into(),
        ErrorKind::InvalidRowHeightRule => "rule 必须是 auto、atLeast 或 exact".into(),
        ErrorKind::TableColsCount => "cols 长度必须等于列数".into(),
        ErrorKind::TableRowHeightsCount => "rowHeights 长度必须等于行数".into(),
        ErrorKind::NeedSetTableCellMarField => {
            "setTableCellMar 至少要给 top、left、bottom 或 right 之一".into()
        }
        ErrorKind::NeedSetCellFormatField => {
            "setCellFormat 至少要给 fill、边距、斜线、vAlign、noWrap、textDirection、fitText、hideMark 或 headers 之一".into()
        }
        ErrorKind::NeedSetStyleField => {
            "setStyle 至少要给 font、eastAsiaFont、sz、bold、color、before 或 after 之一".into()
        }
        ErrorKind::EmptyStyleFont => "font 与 eastAsiaFont 不能为空".into(),
        ErrorKind::InvalidStyleColor => "color 必须是六位 RGB 十六进制".into(),
        ErrorKind::NeedSetParaStyleStyle => {
            "setParaStyle / setRunStyle 必须有 style（style: 样式 id 或 null）".into()
        }
        ErrorKind::NeedSetParaFormatField => {
            "setParaFormat 至少要给 align、left、right、firstLine、hanging、before、after、line、lineRule、kinsoku、overflowPunct、topLinePunct、autoSpaceDE、autoSpaceDN、borders、shd 或 tabs 之一"
                .into()
        }
        ErrorKind::NeedSetRunFormatField => {
            "setRunFormat 至少要给 bold、italic、underline、sz、color、font、eastAsiaFont、em、eastAsianLayout、highlight、strike、dstrike、vertAlign、spacing、kern 或 scale 之一"
                .into()
        }
        ErrorKind::NeedParaBordersField => {
            "borders 至少要给 top、left、bottom 或 right 之一".into()
        }
        ErrorKind::InvalidHighlight => {
            "highlight 不是认识的 ST_HighlightColor 值".into()
        }
        ErrorKind::InvalidVertAlign => {
            "vertAlign 必须是 superscript、subscript 或 baseline".into()
        }
        ErrorKind::InvalidTabVal => {
            "tabs[].val 必须是 left、center、right、decimal 或 bar".into()
        }
        ErrorKind::InvalidEm => {
            "em 必须是 none、dot、comma、circle 或 underDot".into()
        }
        ErrorKind::NeedEastAsianLayoutField => {
            "eastAsianLayout 必须给 combine 或 vert 之一".into()
        }
        ErrorKind::InvalidCombineBrackets => {
            "combineBrackets 只在 combine 为 true 时有效，且必须是 none、round、square、angle 或 curly"
                .into()
        }
        ErrorKind::EmptyRubyBase => "base 不能为空".into(),
        ErrorKind::EmptyRubyRt => "rt 不能为空".into(),
        ErrorKind::EmptyRubyLid => "lid 不能为空".into(),
        ErrorKind::InvalidRubyAlign => {
            "align 必须是 center、distributeLetter、distributeSpace、left、right 或 rightVertical"
                .into()
        }
        ErrorKind::NeedSetRubyField => {
            "setRuby 至少要给 base、rt、align、hps、hpsRaise、hpsBaseText 或 lid 之一".into()
        }
        ErrorKind::InvalidCharacterSpacing => {
            "characterSpacingControl 必须是 doNotCompress、compressPunctuation 或 compressPunctuationAndJapaneseKana"
                .into()
        }
        ErrorKind::NeedSetEastAsianSettingsField => {
            "setEastAsianSettings 至少要给 characterSpacingControl、strictFirstAndLastChars、noLineBreaksBefore、noLineBreaksAfter 或 themeFontLang 之一"
                .into()
        }
        ErrorKind::EmptyKinsokuLang => "lang 不能为空".into(),
        ErrorKind::EmptyKinsokuChars => "chars 不能为空".into(),
        ErrorKind::NeedSetCellStyleField => {
            "setCellStyle 至少要给 numFmt、font、fill、border 或 align 之一".into()
        }
        ErrorKind::InvalidCellStyle { field } => {
            format!("setCellStyle 的 {field} 不合法")
        }
        ErrorKind::NeedSetColWidthField => {
            "setColWidth 至少要给 width 或 bestFit 之一".into()
        }
        ErrorKind::InvalidColWidth => "width 必须大于 0 且不超过 255".into(),
        ErrorKind::NeedSetRowHeightField => "setRowHeight 必须有 height".into(),
        ErrorKind::InvalidRowHeight => "height 必须大于 0 且不超过 409".into(),
        ErrorKind::MergeNeedsMultipleCells => "mergeRange 至少要两格".into(),
        ErrorKind::RowRangeReversed => "toRow 不能小于 row".into(),
        ErrorKind::LayoutWorksheetOnly => {
            "只能对工作表改列宽、行高、合并、数据验证、条件格式、表格或透视表".into()
        }
        ErrorKind::NeedSetDataValidationField => {
            "setDataValidation 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::NeedDataValidationFormula { field } => {
            format!("数据验证必须有 {field}")
        }
        ErrorKind::InvalidDataValidation { field } => {
            format!("数据验证的 {field} 不合法")
        }
        ErrorKind::DataValidationNotFound { id } => format!("找不到数据验证：{id}"),
        ErrorKind::NeedSetConditionalFormatField => {
            "setConditionalFormat 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::NeedConditionalFormatFormula { field } => {
            format!("条件格式必须有 {field}")
        }
        ErrorKind::InvalidConditionalFormat { field } => {
            format!("条件格式的 {field} 不合法")
        }
        ErrorKind::ConditionalFormatNotFound { id } => format!("找不到条件格式：{id}"),
        ErrorKind::NeedSetSheetTableField => {
            "setSheetTable 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidSheetTable { field } => {
            format!("工作表表格的 {field} 不合法")
        }
        ErrorKind::SheetTableNotFound { id } => format!("找不到工作表表格：{id}"),
        ErrorKind::NeedSetPivotTableField => {
            "setPivotTable 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidPivotTable { field } => {
            format!("透视表的 {field} 不合法")
        }
        ErrorKind::PivotTableNotFound { id } => format!("找不到透视表：{id}"),
        ErrorKind::NeedSetDefinedNameField => {
            "setDefinedName 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidDefinedName { field } => {
            format!("定义名的 {field} 不合法")
        }
        ErrorKind::DefinedNameNotFound { id } => format!("找不到定义名：{id}"),
        ErrorKind::NeedSetProtectedRangeField => {
            "setProtectedRange 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidProtectedRangeName => {
            "允许编辑区域名去掉首尾空白后须为 1 到 255 个字符".into()
        }
        ErrorKind::DuplicateProtectedRangeName => "同一张表内允许编辑区域名不能重复".into(),
        ErrorKind::ProtectedRangeNotFound { id } => format!("找不到允许编辑区域：{id}"),
        ErrorKind::NeedSetPrintSetupField => {
            "setPrintSetup 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidPrintSetup { field } => {
            format!("打印设置的 {field} 不合法")
        }
        ErrorKind::NeedSetSheetViewField => {
            "setSheetView 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidSheetView { field } => {
            format!("视图的 {field} 不合法")
        }
        ErrorKind::NeedSetAutoFilterField => "setAutoFilter 必须给 ref".into(),
        ErrorKind::InvalidAutoFilter { field } => {
            format!("工作表筛选的 {field} 不合法")
        }
        ErrorKind::NeedSetSortStateField => "setSortState 必须给 ref".into(),
        ErrorKind::InvalidSortState { field } => {
            format!("工作表排序的 {field} 不合法")
        }
        ErrorKind::NeedSetOutlineField => {
            "setOutline 除 op 与 id 外至少要给一个字段".into()
        }
        ErrorKind::InvalidOutline { field } => {
            format!("大纲的 {field} 不合法")
        }
        ErrorKind::OutlineLevelMax => "大纲级不能超过 7".into(),
        ErrorKind::InvalidAlign => {
            "align 必须是 left、center、right、both、distribute、start 或 end".into()
        }
        ErrorKind::InvalidLineRule => "lineRule 必须是 auto、exact 或 atLeast".into(),
        ErrorKind::InvalidUnderline => "underline 不是认识的 ST_Underline 值".into(),
        ErrorKind::RunNotFound { id } => format!("找不到 run：{id}"),
        ErrorKind::OpcOpen { reason } => match reason {
            OpcReason::EmptyPartName => "部件名不能为空".into(),
            OpcReason::InvalidPartName { name } => format!("非法部件名：{name}"),
            OpcReason::MissingContentTypes => "包内缺少 [Content_Types].xml".into(),
            OpcReason::MissingRootRels => "包内缺少 /_rels/.rels".into(),
            OpcReason::MissingOfficeDocument => "根关系里没有 officeDocument".into(),
            OpcReason::RelTarget { target } => format!("无法解析关系目标：{target}"),
            OpcReason::Zip { detail } => format!("无法作为 ZIP 打开：{detail}"),
            OpcReason::DeclaredRatio => {
                "条目声明的未压缩大小达到压缩后大小的 4096 倍".into()
            }
            OpcReason::ExceedsDeclared => "解压结果超过中央目录声明的未压缩大小".into(),
            OpcReason::DuplicatePartName { name } => format!("部件名重复：{name}"),
        },
        ErrorKind::PartMissing { name } => format!("找不到部件 {name}"),
        ErrorKind::RelNotFound { source, rid } => match source {
            RelSource::Unspecified => format!("没有关系 {rid}"),
            RelSource::Workbook => format!("workbook 关系里没有 {rid}"),
            RelSource::Part(part) => format!("{part} 没有关系 {rid}"),
        },
        ErrorKind::ExternalRel { rid } => format!("{rid} 指向外部资源"),
        ErrorKind::SlideRelMissing { index } => {
            format!("p:sldId 缺少关系 id（第 {index} 页）")
        }
        ErrorKind::XmlBroken { issue, detail } => match (issue, detail) {
            (XmlIssue::UnbalancedEnd, _) => "XML 结束标签多于开始标签".into(),
            (XmlIssue::NoRoot, _) => "XML 没有根元素".into(),
            (XmlIssue::TextDecode, Some(d)) => format!("XML 文本无法解码：{d}"),
            (XmlIssue::TextDecode, None) => "XML 文本无法解码".into(),
            (XmlIssue::Parse, Some(d)) => format!("XML 解析失败：{d}"),
            (XmlIssue::Parse, None) => "XML 解析失败".into(),
            (XmlIssue::AttrRead, Some(d)) => format!("XML 属性无法读取：{d}"),
            (XmlIssue::AttrRead, None) => "XML 属性无法读取".into(),
            (XmlIssue::AttrDecode, Some(d)) => format!("XML 属性值无法解码：{d}"),
            (XmlIssue::AttrDecode, None) => "XML 属性值无法解码".into(),
            (XmlIssue::Write, Some(d)) => format!("写出 XML 失败：{d}"),
            (XmlIssue::Write, None) => "写出 XML 失败".into(),
        },
        ErrorKind::XmlMissing { parent, child } => format!("{parent} 缺少 {child}"),
        ErrorKind::SharedStringIndex { cell, index } => {
            format!("共享字符串索引 {index} 不存在（单元格 {cell}）")
        }
        ErrorKind::SharedStringNotInTable { text } => {
            format!("共享字符串表里没有 {text}")
        }
    }
}

fn type_name(t: TypeName) -> &'static str {
    match t {
        TypeName::Object => "对象",
        TypeName::Array => "数组",
        TypeName::Integer => "整数",
        TypeName::String => "字符串",
        TypeName::NonEmptyString => "非空字符串",
        TypeName::Boolean => "布尔值",
    }
}
