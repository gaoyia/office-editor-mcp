//! 中文错误文案。现有句子照搬，只统一标点。

use super::super::kind::{
    ErrorKind, IdReason, OpcReason, RangeReason, RelSource, ScopeNeed, SheetNameReason, TypeName,
    XmlIssue,
};

pub(super) fn render(kind: &ErrorKind) -> String {
    match kind {
        ErrorKind::InvalidJsonParse { detail } => format!("入参不是 JSON：{detail}"),
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
            IdReason::ShapeForm => "setShapeText 的 id 必须是 slide:N/shape:…".into(),
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
            IdReason::TxbxPrefix => "文本框 id 必须是段落 id 加上 /txbx:…".into(),
            IdReason::TxbxContainer => {
                "段落 id 必须带 /txbx:…（该 id 本身不是文本框）".into()
            }
            IdReason::TxbxIndexNotInt => "txbx:index 后面必须是正整数".into(),
            IdReason::TxbxIndexFromOne => "txbx:index 从 1 起".into(),
            IdReason::SetParaOnBox => "setParaText 的 id 必须是段落 id，不能是文本框 id".into(),
            IdReason::InsertNeedsHostPara => "insertTextBox 的 id 必须是宿主段落 id".into(),
            IdReason::InsertParaBody => {
                "insertPara 与 deletePara 只允许用在文本框、脚注、尾注或批注内".into()
            }
            IdReason::InsertParaNeedsBox => {
                "insertPara / deletePara 的 id 必须是文本框、框内段落、注、注内段落、批注或批注内段落".into()
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
            IdReason::InsertChartNeedsPara => {
                "insertChart 的 id 必须是正文或页眉页脚段落 id".into()
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
            },
        },
        ErrorKind::SheetNotFound { name } => format!("找不到工作表：{name}"),
        ErrorKind::ParaNotFound { id } => format!("找不到段落：{id}"),
        ErrorKind::SectionNotFound { id } => format!("找不到节：{id}"),
        ErrorKind::SlideNotFound { n } => format!("找不到幻灯片：slide:{n}"),
        ErrorKind::ShapeNotFound { id } => format!("找不到形状：{id}"),
        ErrorKind::ShapeHasNoText { id } => format!("形状没有文本框：{id}"),
        ErrorKind::PictureNotFound { id } => format!("找不到图：{id}"),
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
        ErrorKind::InkComment { id } => format!("墨迹批注除删除外不能编辑：{id}"),
        ErrorKind::ChartNotFound { id } => format!("找不到图表：{id}"),
        ErrorKind::ChartSeriesNotFound { id } => format!("找不到图表系列：{id}"),
        ErrorKind::NeedChartSeriesField => {
            "setChartSeries 至少要给 name、categories、values 或 sizes 之一".into()
        }
        ErrorKind::LastChartSeries => "不能删光图表里的全部系列，整张去掉请用 deleteChart".into(),
        ErrorKind::RestrictedChart { id } => {
            format!("此图表不能按此操作编辑：{id}")
        }
        ErrorKind::InvalidChartType => {
            "type 必须是 bar、col、line、pie、area、scatter、doughnut、radar 或 bubble；insertChart 还可为 combo".into()
        }
        ErrorKind::InvalidChartAxis => "axis 必须是 cat 或 val，且图表里要有对应主轴".into(),
        ErrorKind::InvalidLegendPos => "pos 必须是 b、t、l、r 或 tr".into(),
        ErrorKind::PieNeedsOneSeries => "饼图与圆环图必须恰好一条系列".into(),
        ErrorKind::SizesOnNonBubble => "sizes 只对气泡系列有意义".into(),
        ErrorKind::InvalidComboSeriesType => {
            "系列 type 必须是 bar、col、line、area 或 scatter，且只有组合图才能带".into()
        }
        ErrorKind::ChartTypeNotConvertible => "这种图表类型无法无损转换".into(),
        ErrorKind::NeedChartCategories => {
            "addChartSeries 省略 categories 时图表里必须已有系列可抄".into()
        }
        ErrorKind::SmartArtNotFound { id } => format!("找不到 SmartArt：{id}"),
        ErrorKind::SmartArtPointNotFound { id } => format!("找不到 SmartArt 点：{id}"),
        ErrorKind::LastSmartArtPoint => {
            "不能删光 SmartArt 里的全部可见点，整张去掉请用 deleteSmartArt".into()
        }
        ErrorKind::InvalidSmartArtLayout => "layout 必须是 list 或 org".into(),
        ErrorKind::InvalidSmartArtRole => "role 必须是 node 或 asst".into(),
        ErrorKind::InvalidSmartArtColors => {
            "colors 必须是 accent1、accent2、accent3、accent4、accent5 或 accent6".into()
        }
        ErrorKind::InvalidSmartArtStyle => "style 必须是 simple、subtle 或 intense".into(),
        ErrorKind::SmartArtLayoutNotConvertible => "这种 SmartArt 版式无法无损转换".into(),
        ErrorKind::SmartArtPointParent => {
            "points[].parent 必须是本数组下标，且不能成环".into()
        }
        ErrorKind::SmartArtAsstOnList => "list 版式不能带 asst 点".into(),
        ErrorKind::SmartArtListDepth => "list 版式只接受顶层点（parent 必须省略或为 null）".into(),
        ErrorKind::TextBoxLinkCrossPart => "链接文本框必须在同一部件里".into(),
        ErrorKind::InvalidHexColor => "color 必须是六位 RGB 十六进制或 null".into(),
        ErrorKind::InvalidTextBoxDirection => "direction 必须是 horizontal 或 vertical".into(),
        ErrorKind::InvalidPicKind => "kind 必须是 inline 或 anchor".into(),
        ErrorKind::InvalidPicWrap => {
            "wrap 必须是 square、tight、through、topAndBottom 或 none".into()
        }
        ErrorKind::InvalidPicAlign => "alignH 只允许 left/center/right，alignV 只允许 top/center/bottom".into(),
        ErrorKind::InvalidImageFile => "file 不是本库认的位图（PNG、JPEG、GIF 或 BMP）".into(),
        ErrorKind::SetCellWorksheetOnly => "只能对工作表 setCell".into(),
        ErrorKind::InspectWorksheetOnly => "只能对工作表读单元格".into(),
        ErrorKind::SharedFormulaMaster { cell } => format!(
            "{cell} 是共享公式的主格，改它会让引用同一 si 的单元格失去公式；请先把那些单元格改成各自独立的公式"
        ),
        ErrorKind::NeedAtLeastOneSheet => "至少需要一张工作表".into(),
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
        ErrorKind::CreateHfOnlyDefault => "create 的页眉页脚 type 只能是 default".into(),
        ErrorKind::CreateHfDuplicate => "create 每一侧只能有一个默认页眉或页脚".into(),
        ErrorKind::CannotLinkFirstSection => "第一节不能链接到前一节".into(),
        ErrorKind::HfTypeInactive { kind, hf_type } => {
            format!("本节未启用这种 {kind}（{hf_type}）")
        }
        ErrorKind::OpcOpen { reason } => match reason {
            OpcReason::EmptyPartName => "部件名不能为空".into(),
            OpcReason::InvalidPartName { name } => format!("非法部件名：{name}"),
            OpcReason::MissingContentTypes => "包内缺少 [Content_Types].xml".into(),
            OpcReason::MissingRootRels => "包内缺少 /_rels/.rels".into(),
            OpcReason::MissingOfficeDocument => "根关系里没有 officeDocument".into(),
            OpcReason::RelTarget { target } => format!("无法解析关系目标：{target}"),
            OpcReason::Zip { detail } => format!("无法作为 ZIP 打开：{detail}"),
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
