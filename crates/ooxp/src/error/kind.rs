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
    NotCell {
        id: String,
    },
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
    /// 图片 id 必须是 {容器}/pic:… 或 para:…/pic:…
    PicPrefix,
    /// 段落或容器 id，缺少 `/pic:…`
    PicContainer,
    PicIndexNotInt,
    PicIndexFromOne,
    /// insertPicture 必须用段落 id
    InsertNeedsPara,
    /// xlsx 图 id 必须是 sheet:{name}/pic:…
    SheetPicPrefix,
    /// 工作表 id，缺少 /pic:…
    SheetPicContainer,
    /// xlsx insertPicture 必须用 sheet:{name} 或页眉槽位 id
    InsertNeedsSheet,
    /// pptx 图 id 必须是 slide:{n}/pic:…
    SlidePicPrefix,
    /// 页 id，缺少 /pic:…
    SlidePicContainer,
    /// 批注内图没有画布尺寸和位置
    CommentPicExportOnly,
    /// pptx 视频 / 音频 id 必须是 slide:N/video:… 或 slide:N/audio:…
    SlideMediaForm,
    /// pptx 的 insertVideo / insertAudio 必须用 slide:N
    InsertMediaNeedsSlide,
    /// pptx 连接线 id 必须是 slide:N/cxn:…
    SlideCxnForm,
    /// pptx 的 insertConnector / groupShapes 必须用 slide:N
    InsertConnectorNeedsSlide,
    /// pptx 的 insertPicture / insertShape 必须用 slide:N 或 slide:N/notes
    InsertNeedsSlide,
    /// pptx 的 insertShape 必须用 slide:N、slide:N/notes、master:N 或 master:N/layout:K
    InsertShapeHost,
    /// setNotesText 必须用 slide:N 或 slide:N/notes
    SetNotesTextNeedsSlide,
    /// pptx 批注 id 必须是 slide:N/comment:…
    PptxCommentForm,
    /// pptx insertComment 必须用 slide:N
    InsertCommentNeedsSlide,
    /// xlsx 批注 id 必须是 cell:表!A1/comment:…
    XlsxCommentForm,
    /// xlsx insertComment 必须用单元格 id
    InsertCommentNeedsCell,
    /// xlsx 页眉槽位 id 必须是 sheet:{name}/hf:{kind}:{side}
    HfSlotPrefix,
    /// 框 id 必须是 {段落}/txbx:…
    TxbxPrefix,
    /// 段落 id，缺少 /txbx:
    TxbxContainer,
    TxbxIndexNotInt,
    TxbxIndexFromOne,
    /// 对框 id 发 setParaText
    SetParaOnBox,
    /// insertTextBox / insertConnector 必须是宿主段落
    InsertNeedsHostPara,
    /// docx 连接线 id 必须是 {段落}/cxn:…
    DocxCxnPrefix,
    /// 段落或容器 id，缺少 /cxn:…
    DocxCxnContainer,
    /// docx 无框形状 id 必须是 {段落}/shape:…
    DocxShapePrefix,
    /// 段落或容器 id，缺少 /shape:…
    DocxShapeContainer,
    /// docx 墨迹画布 id 必须是 {段落}/ink:…
    DocxInkPrefix,
    /// 段落或容器 id，缺少 /ink:…
    DocxInkContainer,
    DocxInkIndexNotInt,
    DocxInkIndexFromOne,
    /// docx 3D 模型 id 必须是 {段落}/model3d:…
    DocxModel3dPrefix,
    /// 段落或容器 id，缺少 /model3d:…
    DocxModel3dContainer,
    DocxModel3dIndexNotInt,
    DocxModel3dIndexFromOne,
    /// docx 的 groupShapes 必须用 docx:document 或页眉/页脚容器
    GroupNeedsDocument,
    /// xlsx 文本框 id 必须是 sheet:{name}/sp:…
    SheetSpPrefix,
    /// 工作表 id，缺少 /sp:…
    SheetSpContainer,
    SpIndexNotInt,
    SpIndexFromOne,
    /// xlsx 的 insertTextBox 必须用 sheet:工作表名
    InsertTextBoxNeedsSheet,
    /// xlsx 形状 id 必须是 sheet:{name}/shape:…
    SheetShapePrefix,
    /// 工作表 id，缺少 /shape:…
    SheetShapeContainer,
    /// xlsx 的 insertShape 必须用 sheet:工作表名
    InsertShapeNeedsSheet,
    /// xlsx 连接线 id 必须是 sheet:{name}/cxn:…
    SheetCxnPrefix,
    /// 工作表 id，缺少 /cxn:…
    SheetCxnContainer,
    CxnIndexNotInt,
    CxnIndexFromOne,
    /// xlsx 的 insertConnector 必须用 sheet:工作表名
    InsertConnectorNeedsSheet,
    /// xlsx 的 groupShapes 必须用 sheet:工作表名
    GroupNeedsSheet,
    /// setShapeText 不能用连接线 id
    SetShapeTextNotCxn,
    /// xlsx 图示 id 必须是 sheet:{name}/smartdiagram:…
    SheetSmartArtPrefix,
    /// 工作表 id，缺少 /smartdiagram:…
    SheetSmartArtContainer,
    /// xlsx 的 insertSmartArt 必须用 sheet:工作表名
    InsertSmartArtNeedsSheet,
    /// deletePara 必须是段落 id，不能是容器
    InsertParaNeedsBox,
    /// 脚注/尾注 id 形态不对
    NotePrefix,
    /// 注容器 id，缺少 /para:…
    NoteContainer,
    /// 分隔符 id，禁止 apply
    NoteSep,
    /// 批注 id 形态不对
    CommentPrefix,
    /// 批注容器 id，缺少 /para:…
    CommentContainer,
    /// 图表 id 必须是 {段落}/chart:…
    ChartPrefix,
    /// 段落 id，缺少 /chart:…
    ChartContainer,
    ChartIndexNotInt,
    ChartIndexFromOne,
    /// 系列 id 必须是 {图表}/series:{n}
    SeriesPrefix,
    SeriesIndexNotInt,
    /// 趋势线 id 必须是 {系列}/trendline:{k}，k 从 1 起
    TrendlinePrefix,
    TrendlineIndexNotInt,
    TrendlineIndexFromOne,
    /// 误差线 id 必须是 {系列}/errorBars:{k}，k 从 1 起
    ErrorBarsPrefix,
    ErrorBarsIndexNotInt,
    ErrorBarsIndexFromOne,
    /// insertChart 必须用正文或页眉页脚段落 id
    InsertChartNeedsPara,
    /// xlsx 的 insertChart 必须用 sheet:工作表名
    InsertChartNeedsSheet,
    /// pptx 的 insertChart 必须用 slide:N
    InsertChartNeedsSlide,
    /// insertChartSheet 必须用 xlsx:workbook
    InsertChartSheetNeedsWorkbook,
    /// insertSheet 必须用 xlsx:workbook
    InsertSheetNeedsWorkbook,
    /// insertSlide 必须用 pptx:presentation
    InsertSlideNeedsPresentation,
    /// insertSlideSection 必须用 pptx:presentation
    InsertSlideSectionNeedsPresentation,
    /// insertCustomShow 必须用 pptx:presentation
    InsertCustomShowNeedsPresentation,
    /// setCustomShow / deleteCustomShow 必须用 show:N
    CustomShowPrefix,
    CustomShowIndexNotInt,
    CustomShowIndexFromOne,
    /// setSlideSection 必须用 slide:N
    SetSlideSectionNeedsSlide,
    /// deleteSlide / moveSlide / setSlideHidden / setSlideLayout 必须用 slide:N
    SlideOpNeedsSlide,
    /// setTransition 必须用 slide:N、master:N 或 master:N/layout:K
    SetTransitionNeedsHost,
    /// 母版或占位符 id 形态不对
    MasterForm,
    /// 母版 / 版式占位符请用 deletePlaceholder，不要用 deleteShape / setShapeFill / setShapeLine
    MasterPlaceholderLocked,
    /// setSlideLayout / deleteLayout 的 layout 必须是 master:N/layout:K
    LayoutForm,
    /// insertLayout 的 id 必须是 master:N
    InsertLayoutNeedsMaster,
    /// insertPlaceholder 的 id 必须是 master:N 或 master:N/layout:K
    InsertPlaceholderNeedsHost,
    /// insertMaster 必须用 pptx:presentation
    InsertMasterNeedsPresentation,
    /// deleteMaster 的 id 必须是 master:N
    DeleteMasterNeedsMaster,
    /// setSlideSize 必须用 pptx:presentation
    SetSlideSizeNeedsPresentation,
    /// setShowPr 必须用 pptx:presentation
    SetShowPrNeedsPresentation,
    /// pptx 的 setProtection 必须用 pptx:presentation
    SetProtectionNeedsPresentation,
    /// setThemeColors / setThemeFonts 必须用 master:N
    SetThemeNeedsMaster,
    /// setThemeColors / setThemeFonts 必须用 docx:document
    SetThemeNeedsDocument,
    /// setThemeColors / setThemeFonts 必须用 xlsx:workbook
    SetThemeNeedsWorkbook,
    /// setHandoutText 必须用 handout
    SetHandoutTextNeedsHandout,
    /// insertDefinedName 必须用 xlsx:workbook
    InsertDefinedNameNeedsWorkbook,
    /// insertConnection 必须用 xlsx:workbook
    InsertConnectionNeedsWorkbook,
    /// insertExternalLink 必须用 xlsx:workbook
    InsertExternalLinkNeedsWorkbook,
    /// insertCustomView 必须用 xlsx:workbook
    InsertCustomViewNeedsWorkbook,
    /// 自定义视图 id 必须是 customView:n
    CustomViewPrefix,
    CustomViewIndexNotInt,
    CustomViewIndexFromOne,
    /// insertCellStyle 必须用 xlsx:workbook
    InsertCellStyleNeedsWorkbook,
    /// 命名单元格样式 id 必须是 cellStyle:n
    CellStylePrefix,
    CellStyleIndexNotInt,
    CellStyleIndexFromOne,
    /// 连接 id 必须是 connection:{xmlId} 或 connection:index:{k}
    ConnectionPrefix,
    ConnectionIndexNotInt,
    ConnectionIndexFromOne,
    /// 外部引用 id 必须是 extlink:n
    ExternalLinkPrefix,
    ExternalLinkIndexNotInt,
    ExternalLinkIndexFromOne,
    /// setDefinedName / deleteDefinedName 必须用 name:n
    DefinedNamePrefix,
    NameIndexNotInt,
    NameIndexFromOne,
    /// deleteChartSheet 必须用图表工作表 sheet:工作表名
    DeleteChartSheetNeedsChartsheet,
    /// deleteSheet 必须用普通工作表 sheet:工作表名
    DeleteSheetNeedsWorksheet,
    /// renameSheet / moveSheet / setSheetHidden / insertDataValidation / insertConditionalFormat / insertSheetTable 必须用 sheet:工作表名
    SheetOpNeedsSheet,
    /// setProtection 必须用 sheet:工作表名 或 xlsx:workbook
    SetProtectionNeedsSheetOrWorkbook,
    /// insertProtectedRange 必须用 sheet:工作表名
    InsertProtectedRangeNeedsSheet,
    /// setProtectedRange / deleteProtectedRange 必须用 sheet:工作表名/editRange:n
    SheetEditRangePrefix,
    EditRangeIndexNotInt,
    EditRangeIndexFromOne,
    /// setDataValidation / deleteDataValidation 必须用 sheet:工作表名/dv:n
    SheetDvPrefix,
    DvIndexNotInt,
    DvIndexFromOne,
    /// setConditionalFormat / deleteConditionalFormat 必须用 sheet:工作表名/cf:n
    SheetCfPrefix,
    CfIndexNotInt,
    CfIndexFromOne,
    /// setSheetTable / deleteSheetTable 必须用 sheet:工作表名/table:n
    SheetTablePrefix,
    /// setPivotTable / deletePivotTable 必须用 sheet:工作表名/pivot:n
    SheetPivotPrefix,
    PivotIndexNotInt,
    PivotIndexFromOne,
    /// xlsx 图表 id 必须是 sheet:工作表名/chart:…
    SheetChartPrefix,
    /// 工作表 id，缺少 /chart:…
    SheetChartContainer,
    /// pptx 图表 id 必须是 slide:N/chart:…
    SlideChartPrefix,
    /// 页 id，缺少 /chart:…
    SlideChartContainer,
    /// SmartArt id 必须是 {段落}/smartdiagram:…
    SmartArtPrefix,
    /// 段落 id，缺少 /smartdiagram:…
    SmartArtContainer,
    SmartArtIndexNotInt,
    SmartArtIndexFromOne,
    /// 点 id 必须是 {图示}/pt:{modelId}
    SmartArtPtPrefix,
    /// insertSmartArt 必须用正文或页眉页脚段落 id
    InsertSmartArtNeedsPara,
    /// pptx 图示 id 必须是 slide:N/smartdiagram:…
    SlideSmartArtPrefix,
    /// 页 id，缺少 /smartdiagram:…
    SlideSmartArtContainer,
    /// pptx 的 insertSmartArt 必须用 slide:N
    InsertSmartArtNeedsSlide,
    /// pptx 表 id 必须是 slide:N/table:… 或 slide:N/notes/table:…
    SlideTablePrefix,
    /// 页 id，缺少 /table:…
    SlideTableContainer,
    SlideTableIndexNotInt,
    SlideTableIndexFromOne,
    /// 行 id 必须是 {表 id}/row:{r}
    SlideTableRowForm,
    /// 单元格 id 必须是 {表 id}/r:{r}/c:{c}
    SlideTableCellForm,
    /// pptx 的 insertTable 必须用 slide:N 或 slide:N/notes
    InsertTableNeedsSlide,
    /// setTableCellText 必须用单元格 id
    SetTableCellNeedsCell,
    /// OLE id 必须是 {段落}/ole:…
    OlePrefix,
    /// 段落 id，缺少 /ole:…
    OleContainer,
    OleIndexNotInt,
    OleIndexFromOne,
    /// insertOle 必须用正文或页眉页脚段落 id
    InsertOleNeedsPara,
    /// pptx OLE id 必须是 slide:N/ole:…
    SlideOlePrefix,
    /// 页 id，缺少 /ole:…
    #[allow(dead_code)]
    SlideOleContainer,
    /// pptx 的 insertOle 必须用 slide:N
    InsertOleNeedsSlide,
    /// xlsx 的 insertOle 必须用 sheet:工作表名
    InsertOleNeedsSheet,
    /// xlsx OLE id 必须是 sheet:{name}/ole:…
    SheetOlePrefix,
    /// xlsx 工作表 id，缺少 /ole:…
    SheetOleContainer,
    /// xlsx 的 insertModel3d 必须用 sheet:工作表名
    InsertModel3dNeedsSheet,
    /// xlsx 3D 模型 id 必须是 sheet:{name}/model3d:…
    SheetModel3dPrefix,
    /// 工作表 id，缺少 /model3d:…
    SheetModel3dContainer,
    /// pptx 的 insertModel3d 必须用 slide:N 或备注容器
    InsertModel3dNeedsSlide,
    /// pptx 3D 模型 id 必须是 slide:N/model3d:… 或备注
    SlideModel3dPrefix,
    /// 页 id，缺少 /model3d:…
    #[allow(dead_code)]
    SlideModel3dContainer,
    /// xlsx 的 insertMath 必须用 sheet:工作表名
    InsertMathNeedsSheet,
    /// xlsx 公式框 id 必须是 sheet:{name}/math:…
    SheetMathPrefix,
    /// 工作表 id，缺少 /math:…
    SheetMathContainer,
    /// pptx 的 insertMath 必须用 slide:N 或备注容器
    InsertMathNeedsSlide,
    /// pptx 公式框 id 必须是 slide:N/math:… 或备注
    SlideMathPrefix,
    /// 页 id，缺少 /math:…
    #[allow(dead_code)]
    SlideMathContainer,
    /// xlsx 的 insertSlicer 必须用 sheet:工作表名
    InsertSlicerNeedsSheet,
    /// xlsx 切片器 id 必须是 sheet:{name}/slicer:…
    SheetSlicerPrefix,
    /// 工作表 id，缺少 /slicer:…
    SheetSlicerContainer,
    SlicerIndexNotInt,
    SlicerIndexFromOne,
    /// insertSlicer 的 source 必须是本表表格 id 或透视表 id
    InsertSlicerNeedsTableOrPivot,
    /// pptx 的 insertZoom 必须用 slide:N
    InsertZoomNeedsSlide,
    /// insertZoom 的 type 为 slide 时 target 必须是 slide:N
    ZoomTargetSlide,
    /// pptx Zoom id 必须是 slide:N/zoom:…
    SlideZoomPrefix,
    /// 页 id，缺少 /zoom:…
    #[allow(dead_code)]
    SlideZoomContainer,
    ZoomIndexNotInt,
    ZoomIndexFromOne,
    /// insertWatermark 必须用 docx:document
    InsertWatermarkNeedsDocument,
    /// 水印 id 必须是 {页眉容器}/watermark:…
    WatermarkPrefix,
    /// 页眉容器 id，缺少 /watermark:…
    WatermarkContainer,
    WatermarkIndexNotInt,
    WatermarkIndexFromOne,
    /// setEvenAndOddHeaders 必须用 docx:document
    EvenOddNeedsDocument,
    /// setMirrorMargins / setGutterAtTop 必须用 docx:document
    SettingsNeedDocument,
    /// 表 id 必须是 table:N 或 {容器}/table:N
    TablePrefix,
    TableIndexNotInt,
    TableIndexFromOne,
    TableCellForm,
    InsertTableAnchor,
    DeleteTableNeedsTable,
    NeedTableId,
    InsertRowNeedsTableOrRow,
    DeleteRowNeedsRow,
    SetTableRowHeightNeedsRow,
    SplitNeedsCell,
    /// 修订 id 必须是 rev:N 或 {容器}/rev:N
    RevPrefix,
    RevIndexNotInt,
    RevIndexFromOne,
    /// 超链接 id 必须是 link:N 或 {容器}/link:N
    LinkPrefix,
    LinkIndexNotInt,
    LinkIndexFromOne,
    /// insertHyperlink 必须用正文或页眉页脚段落 id
    InsertLinkNeedsPara,
    /// xlsx 超链接 id 必须是 sheet:{name}/link:{n}
    SheetLinkPrefix,
    /// xlsx 的 insertHyperlink 必须用 sheet:工作表名
    InsertHyperlinkNeedsSheet,
    /// pptx 的 insertHyperlink 必须用文本形状、图、表、图表、SmartArt 或表单元格 id
    InsertHyperlinkNeedsShape,
    /// pptx 超链接 id 必须是 slide:N/link:K
    SlideLinkPrefix,
    /// pptx 内部跳转必须是 slide:N
    PptxJumpNeedsSlide,
    /// xlsx 迷你图组 id 必须是 sheet:{name}/spark:{n}
    SheetSparkPrefix,
    SparkIndexNotInt,
    SparkIndexFromOne,
    /// xlsx 的 insertSparkline 必须用 sheet:工作表名
    InsertSparklineNeedsSheet,
    /// 书签 id 必须是 bookmark:N 或 {容器}/bookmark:N
    BookmarkPrefix,
    BookmarkIndexNotInt,
    BookmarkIndexFromOne,
    /// insertBookmark 必须用正文或页眉页脚段落 id
    InsertBookmarkNeedsPara,
    /// 例外编辑区 id 必须是 perm:N 或 {容器}/perm:N
    PermPrefix,
    PermIndexNotInt,
    PermIndexFromOne,
    /// insertEditRange 必须用正文或页眉页脚段落 id
    InsertEditRangeNeedsPara,
    /// setBibliographyStyle 必须用 docx:document
    SetBibliographyStyleNeedsDocument,
    /// 域 id 必须是 field:N 或 {容器}/field:N
    FieldPrefix,
    FieldIndexNotInt,
    FieldIndexFromOne,
    /// insertField 必须用正文或页眉页脚段落 id
    InsertFieldNeedsPara,
    /// pptx 域 id 必须是 slide:N/fld:GUID 或 slide:N/fld:index:K
    PptxFieldForm,
    /// pptx 的 insertField 必须用幻灯片文本形状 id
    InsertFieldNeedsShape,
    /// insertNumbering 必须用 docx:document
    InsertNumberingNeedsDocument,
    /// insertStyle 必须用 docx:document
    InsertStyleNeedsDocument,
    /// 编号实例 id 必须是 num:N
    NumPrefix,
    NumIndexNotInt,
    NumIndexFromOne,
    /// 抽象编号 id 必须是 abstractNum:N
    AbstractNumPrefix,
    AbstractNumIndexNotInt,
    /// setParaNumbering 必须用正文或页眉页脚段落 id
    SetParaNumberingNeedsPara,
    /// 样式 id 必须是 style:{styleId}
    StylePrefix,
    /// setParaStyle 必须用正文或页眉页脚段落 id
    SetParaStyleNeedsPara,
    /// setRunFormat 必须用 {段落}/run:N
    SetRunFormatNeedsRun,
    /// setRunStyle 必须用正文或页眉页脚 run id
    SetRunStyleNeedsStoryRun,
    /// run id 必须是 {段落}/run:N
    RunPrefix,
    RunIndexNotInt,
    RunIndexFromOne,
    /// insertSection 必须是主文档 body 的直接子段落或 docx:document
    InsertSectionNeedsBodyPara,
    /// 目录 id 必须是 toc:N
    TocPrefix,
    TocIndexNotInt,
    TocIndexFromOne,
    /// insertToc 必须是主文档 body 的直接子段落或 docx:document
    InsertTocNeedsBodyPara,
    /// 题注 id 必须是 caption:N
    CaptionPrefix,
    CaptionIndexNotInt,
    CaptionIndexFromOne,
    /// insertCaption 必须是主文档 body 的直接子段落或 docx:document
    InsertCaptionNeedsBodyPara,
    /// 交叉引用 id 必须是 xref:N
    XrefPrefix,
    XrefIndexNotInt,
    XrefIndexFromOne,
    /// insertCrossRef 必须用正文或页眉页脚段落 id
    InsertCrossRefNeedsPara,
    /// 索引 id 必须是 index:N
    IndexPrefix,
    IndexIndexNotInt,
    IndexIndexFromOne,
    /// insertIndex 必须是主文档 body 的直接子段落或 docx:document
    InsertIndexNeedsBodyPara,
    /// 索引项 id 必须是 xe:N
    XePrefix,
    XeIndexNotInt,
    XeIndexFromOne,
    /// insertIndexEntry 必须用正文或页眉页脚段落 id
    InsertIndexEntryNeedsPara,
    /// 书目源 id 必须是 source:N
    SourcePrefix,
    SourceIndexNotInt,
    SourceIndexFromOne,
    /// insertSource 必须用 docx:document
    InsertSourceNeedsDocument,
    /// 引文 id 必须是 cite:N
    CitePrefix,
    CiteIndexNotInt,
    CiteIndexFromOne,
    /// insertCitation 必须用正文或页眉页脚段落 id
    InsertCitationNeedsPara,
    /// 书目块 id 必须是 biblio:N
    BiblioPrefix,
    BiblioIndexNotInt,
    BiblioIndexFromOne,
    /// insertBibliography 必须是主文档 body 的直接子段落或 docx:document
    InsertBibliographyNeedsBodyPara,
    /// 构建基块 id 必须是 docPart:N
    DocPartPrefix,
    DocPartIndexNotInt,
    DocPartIndexFromOne,
    /// insertDocPart 必须用 docx:document
    InsertDocPartNeedsDocument,
    /// insertDocPartContent 必须是主文档 body 的直接子段落或 docx:document
    InsertDocPartContentNeedsBodyPara,
    /// 嵌入字体 id 必须是 font:N
    FontPrefix,
    FontIndexNotInt,
    FontIndexFromOne,
    /// insertEmbeddedFont 必须用 docx:document
    InsertEmbeddedFontNeedsDocument,
    /// setFontEmbed 必须用 docx:document
    SetFontEmbedNeedsDocument,
    /// setPageBackground 必须用 docx:document
    SetPageBackgroundNeedsDocument,
    /// 公式 id 必须是 math:N 或 {容器}/math:N
    MathPrefix,
    MathIndexNotInt,
    MathIndexFromOne,
    /// insertMath 必须是正文或页眉页脚段落 id，或 docx:document
    InsertMathNeedsPara,
    /// 拼音 id 必须是 ruby:N 或 {容器}/ruby:N
    RubyPrefix,
    RubyIndexNotInt,
    RubyIndexFromOne,
    /// insertRuby 必须是正文或页眉页脚段落 id，或 docx:document
    InsertRubyNeedsPara,
    /// setEastAsianSettings 必须用 docx:document
    SetEastAsianSettingsNeedsDocument,
    /// 内容控件 id 必须是 sdt:N 或 {容器}/sdt:N
    SdtPrefix,
    SdtIndexNotInt,
    SdtIndexFromOne,
    /// insertContentControl 必须是正文或页眉页脚段落 id，或 docx:document
    InsertSdtNeedsPara,
    /// setProps / setCustomProp 必须用格式根
    SetPropsNeedsRoot,
    /// deleteCustomProp 必须用 prop:{name}
    CustomPropPrefix,
    /// 动画 id 必须是 slide:N/anim:K、备注或母版上的对应 anim
    AnimForm,
    AnimIndexNotInt,
    AnimIndexFromOne,
    /// insertAnim 必须用幻灯片、备注或母版占位符上的可绑对象
    InsertAnimNeedsHost,
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
    CreateIndexOutOfRange { field: String, max: i64 } => vec![Self::CreateIndexOutOfRange { field: "after".into(), max: 2 }],
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
    ScopeFieldRequires { field: String, required: String } => vec![Self::ScopeFieldRequires {
        field: "textBoxContent".into(),
        required: "textBoxes".into(),
    }],
    ActionNotForFormat { format: String, action: String } => vec![Self::ActionNotForFormat {
        format: "xlsx".into(),
        action: "exportMedia".into(),
    }],
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
    CustomShowNotFound { id: String } => vec![Self::CustomShowNotFound { id: "show:9".into() }],
    SlideNotFound { n: u32 } => vec![Self::SlideNotFound { n: 3 }],
    MasterNotFound { n: u32 } => vec![Self::MasterNotFound { n: 9 }],
    LayoutNotFound { master: u32, layout: u32 } => {
        vec![Self::LayoutNotFound { master: 1, layout: 9 }]
    },
    ShapeNotFound { id: String } => vec![Self::ShapeNotFound { id: "slide:1/shape:2".into() }],
    AnimNotFound { id: String } => vec![Self::AnimNotFound { id: "slide:1/anim:1".into() }],
    AnimHostNotFound { id: String } => {
        vec![Self::AnimHostNotFound { id: "slide:1/shape:2".into() }]
    },
    ShapeHasNoText { id: String } => vec![Self::ShapeHasNoText { id: "slide:1/shape:2".into() }],
    SetCellWorksheetOnly => vec![Self::SetCellWorksheetOnly],
    InspectWorksheetOnly => vec![Self::InspectWorksheetOnly],
    InsertPictureWorksheetOnly => vec![Self::InsertPictureWorksheetOnly],
    InsertChartWorksheetOnly => vec![Self::InsertChartWorksheetOnly],
    InsertTextBoxWorksheetOnly => vec![Self::InsertTextBoxWorksheetOnly],
    InsertModel3dWorksheetOnly => vec![Self::InsertModel3dWorksheetOnly],
    InsertMathWorksheetOnly => vec![Self::InsertMathWorksheetOnly],
    InsertSmartArtWorksheetOnly => vec![Self::InsertSmartArtWorksheetOnly],
    CommentsWorksheetOnly => vec![Self::CommentsWorksheetOnly],
    InsertHyperlinkWorksheetOnly => vec![Self::InsertHyperlinkWorksheetOnly],
    InsertSparklineWorksheetOnly => vec![Self::InsertSparklineWorksheetOnly],
    ProtectedRangeWorksheetOnly => vec![Self::ProtectedRangeWorksheetOnly],
    DataSheetMustBeWorksheet => vec![Self::DataSheetMustBeWorksheet],
    TwoCellPictureSize => vec![Self::TwoCellPictureSize],
    TwoCellChartSize => vec![Self::TwoCellChartSize],
    TwoCellTextBoxSize => vec![Self::TwoCellTextBoxSize],
    TwoCellSmartArtSize => vec![Self::TwoCellSmartArtSize],
    TwoCellModel3dSize => vec![Self::TwoCellModel3dSize],
    TwoCellSlicerSize => vec![Self::TwoCellSlicerSize],
    DeleteChartsheetLast => vec![Self::DeleteChartsheetLast],
    ChartFormulaNotA1 => vec![Self::ChartFormulaNotA1],
    InvalidXlsxAnchor => vec![Self::InvalidXlsxAnchor],
    InvalidXlsxEditAs => vec![Self::InvalidXlsxEditAs],
    NotesSlideImage => vec![Self::NotesSlideImage],
    GraphicInRestrictedStory => vec![Self::GraphicInRestrictedStory],
    DocxShapeNeedsAnchor => vec![Self::DocxShapeNeedsAnchor],
    NeedPropsField => vec![Self::NeedPropsField],
    NeedProtectionField => vec![Self::NeedProtectionField],
    InvalidProtectionEdit => vec![Self::InvalidProtectionEdit],
    InvalidSheetProtection { field: String } => {
        vec![Self::InvalidSheetProtection { field: "formatCells".into() }]
    },
    ProtectionPassword => vec![Self::ProtectionPassword],
    ProtectionHasPassword => vec![Self::ProtectionHasPassword],
    ProtectionPasswordBlank => vec![Self::ProtectionPasswordBlank],
    ProtectionPasswordWrong => vec![Self::ProtectionPasswordWrong],
    ProtectionPasswordNeedsDoc => vec![Self::ProtectionPasswordNeedsDoc],
    InvalidCustomPropType => vec![Self::InvalidCustomPropType],
    EmptyCustomPropName => vec![Self::EmptyCustomPropName],
    CustomPropNotFound { id: String } => vec![Self::CustomPropNotFound { id: "prop:Editor".into() }],
    SharedFormulaMaster { cell: String } => vec![Self::SharedFormulaMaster { cell: "A1".into() }],
    DataTableFormula { cell: String } => vec![Self::DataTableFormula { cell: "A1".into() }],
    InvalidDataTableFormula => vec![Self::InvalidDataTableFormula],
    ArrayFormulaPart { cell: String } => vec![Self::ArrayFormulaPart { cell: "A1".into() }],
    FormulaGroupOverlap { cell: String } => vec![Self::FormulaGroupOverlap { cell: "A1".into() }],
    FormulaRangeNotTopLeft => vec![Self::FormulaRangeNotTopLeft],
    SharedNeedsRef => vec![Self::SharedNeedsRef],
    NeedAtLeastOneSheet => vec![Self::NeedAtLeastOneSheet],
    LastSlide => vec![Self::LastSlide],
    LastLayout => vec![Self::LastLayout],
    LastMaster => vec![Self::LastMaster],
    LayoutInUse => vec![Self::LayoutInUse],
    MasterInUse => vec![Self::MasterInUse],
    InvalidPlaceholderType => vec![Self::InvalidPlaceholderType],
    InvalidSlideSizeType => vec![Self::InvalidSlideSizeType],
    SlideSizeOutOfRange => vec![Self::SlideSizeOutOfRange],
    ThemeNotFound { n: u32 } => vec![Self::ThemeNotFound { n: 9 }],
    DocumentThemeNotFound => vec![Self::DocumentThemeNotFound],
    LastVisibleSheet => vec![Self::LastVisibleSheet],
    MoveSheetAfterSelf => vec![Self::MoveSheetAfterSelf],
    MoveSlideAfterSelf => vec![Self::MoveSlideAfterSelf],
    SlideLayoutMissing => vec![Self::SlideLayoutMissing],
    SheetStillReferenced { name: String } => {
        vec![Self::SheetStillReferenced { name: "Sheet1".into() }]
    },
    DialogOrMacroSheet => vec![Self::DialogOrMacroSheet],
    DuplicateSheetName { name: String } => vec![Self::DuplicateSheetName { name: "Sheet1".into() }],
    InvalidSheetName { reason: SheetNameReason } => SheetNameReason::all_samples()
        .into_iter()
        .map(|reason| Self::InvalidSheetName { reason })
        .collect::<Vec<_>>(),
    InvalidParaStyle => vec![Self::InvalidParaStyle],
    InvalidHfKind => vec![Self::InvalidHfKind],
    InvalidHfType => vec![Self::InvalidHfType],
    CreateHfDuplicate => vec![Self::CreateHfDuplicate],
    InvalidXlsxHfKind => vec![Self::InvalidXlsxHfKind],
    InvalidXlsxHfSide => vec![Self::InvalidXlsxHfSide],
    CreateXlsxHfDuplicate => vec![Self::CreateXlsxHfDuplicate],
    NeedXlsxHfContent => vec![Self::NeedXlsxHfContent],
    InvalidPicKind => vec![Self::InvalidPicKind],
    InvalidPicWrap => vec![Self::InvalidPicWrap],
    InvalidPicAlign => vec![Self::InvalidPicAlign],
    InvalidImageFile => vec![Self::InvalidImageFile],
    InvalidInkMl => vec![Self::InvalidInkMl],
    InvalidMediaFile => vec![Self::InvalidMediaFile],
    NeedSetMediaField => vec![Self::NeedSetMediaField],
    InvalidMediaTime => vec![Self::InvalidMediaTime],
    DuplicateBookmarkName => vec![Self::DuplicateBookmarkName],
    InvalidTrack => vec![Self::InvalidTrack],
    NeedsPptxSectionList => vec![Self::NeedsPptxSectionList],
    SlideNotInPptxSection => vec![Self::SlideNotInPptxSection],
    PictureNotFound { id: String } => vec![Self::PictureNotFound { id: "header:default:1/pic:1".into() }],
    MediaNotFound { id: String } => vec![Self::MediaNotFound { id: "slide:1/video:3".into() }],
    TextBoxNotFound { id: String } => vec![Self::TextBoxNotFound { id: "para:3F2A1B00/txbx:8".into() }],
    LastTextBoxPara => vec![Self::LastTextBoxPara],
    LastNotePara => vec![Self::LastNotePara],
    NoteNotFound { id: String } => vec![Self::NoteNotFound { id: "footnote:2".into() }],
    NeedNoteNumberingField => vec![Self::NeedNoteNumberingField],
    InvalidNoteKind => vec![Self::InvalidNoteKind],
    InvalidNoteNumFmt => vec![Self::InvalidNoteNumFmt],
    InvalidNoteRestart => vec![Self::InvalidNoteRestart],
    LastCommentPara => vec![Self::LastCommentPara],
    LastStoryPara => vec![Self::LastStoryPara],
    CrossParaMarker => vec![Self::CrossParaMarker],
    ParaHasRevision => vec![Self::ParaHasRevision],
    SectionBreakPara => vec![Self::SectionBreakPara],
    CannotDeleteLastSection => vec![Self::CannotDeleteLastSection],
    InvalidSectionType => vec![Self::InvalidSectionType],
    SectionTypeOnLastPara => vec![Self::SectionTypeOnLastPara],
    CommentNotFound { id: String } => vec![Self::CommentNotFound { id: "comment:0".into() }],
    NeedCommentParaId => vec![Self::NeedCommentParaId],
    NeedCommentMetaField => vec![Self::NeedCommentMetaField],
    CommentRangeCrossPart => vec![Self::CommentRangeCrossPart],
    CommentRangeReversed => vec![Self::CommentRangeReversed],
    InkComment { id: String } => vec![Self::InkComment { id: "comment:0".into() }],
    ChartNotFound { id: String } => vec![Self::ChartNotFound { id: "para:1/chart:1".into() }],
    ChartSeriesNotFound { id: String } => {
        vec![Self::ChartSeriesNotFound { id: "para:1/chart:1/series:0".into() }]
    },
    TrendlineNotFound { id: String } => {
        vec![Self::TrendlineNotFound { id: "para:1/chart:1/series:0/trendline:1".into() }]
    },
    ErrorBarsNotFound { id: String } => {
        vec![Self::ErrorBarsNotFound { id: "para:1/chart:1/series:0/errorBars:1".into() }]
    },
    NeedChartSeriesField => vec![Self::NeedChartSeriesField],
    NeedTrendlineField => vec![Self::NeedTrendlineField],
    NeedErrorBarsField => vec![Self::NeedErrorBarsField],
    InvalidTrendlineType => vec![Self::InvalidTrendlineType],
    TrendlineOrderNotPoly => vec![Self::TrendlineOrderNotPoly],
    TrendlinePeriodNotMovingAvg => vec![Self::TrendlinePeriodNotMovingAvg],
    TrendlineNotSupported => vec![Self::TrendlineNotSupported],
    InvalidErrorBars => vec![Self::InvalidErrorBars],
    CustomErrorBars => vec![Self::CustomErrorBars],
    NeedErrorBarsValue => vec![Self::NeedErrorBarsValue],
    ErrorBarsValueOnStdErr => vec![Self::ErrorBarsValueOnStdErr],
    ErrorBarsNotSupported => vec![Self::ErrorBarsNotSupported],
    ErrorBarsDuplicateDir => vec![Self::ErrorBarsDuplicateDir],
    SecondaryAxisNotSupported => vec![Self::SecondaryAxisNotSupported],
    InvalidSeriesAxis => vec![Self::InvalidSeriesAxis],
    LastChartSeries => vec![Self::LastChartSeries],
    RestrictedChart { id: String } => vec![Self::RestrictedChart { id: "para:1/chart:1".into() }],
    InvalidChartType => vec![Self::InvalidChartType],
    InvalidTransitionType => vec![Self::InvalidTransitionType],
    InvalidAnimType => vec![Self::InvalidAnimType],
    InvalidAnimTrigger => vec![Self::InvalidAnimTrigger],
    InvalidAnimDelay => vec![Self::InvalidAnimDelay],
    InvalidAnimDur => vec![Self::InvalidAnimDur],
    InvalidAnimRepeat => vec![Self::InvalidAnimRepeat],
    InvalidAnimPara => vec![Self::InvalidAnimPara],
    InvalidAnimMoveIndex => vec![Self::InvalidAnimMoveIndex],
    NeedAnimSetField => vec![Self::NeedAnimSetField],
    AnimNeedEffectDur => vec![Self::AnimNeedEffectDur],
    InvalidTransitionDur => vec![Self::InvalidTransitionDur],
    InvalidTransitionAdvTm => vec![Self::InvalidTransitionAdvTm],
    InvalidTransitionDir => vec![Self::InvalidTransitionDir],
    InvalidTransitionOrient => vec![Self::InvalidTransitionOrient],
    InvalidTransitionSpokes => vec![Self::InvalidTransitionSpokes],
    InvalidShapePrst => vec![Self::InvalidShapePrst],
    InvalidGeomOp => vec![Self::InvalidGeomOp],
    ConnectorEndHost => vec![Self::ConnectorEndHost],
    NeedConnectorBind => vec![Self::NeedConnectorBind],
    InvalidConnectorPrst => vec![Self::InvalidConnectorPrst],
    GroupNeedsTwo => vec![Self::GroupNeedsTwo],
    GroupMemberNested => vec![Self::GroupMemberNested],
    NotGrouped => vec![Self::NotGrouped],
    ConnectorNotFound { id: String } => vec![Self::ConnectorNotFound { id: "slide:1/cxn:6".into() }],
    InvalidChartAxis => vec![Self::InvalidChartAxis],
    InvalidLegendPos => vec![Self::InvalidLegendPos],
    PieNeedsOneSeries => vec![Self::PieNeedsOneSeries],
    StockNeedsThreeSeries => vec![Self::StockNeedsThreeSeries],
    SurfaceNeedsThreeSeries => vec![Self::SurfaceNeedsThreeSeries],
    InvalidOfPieType => vec![Self::InvalidOfPieType],
    InvalidStockKind => vec![Self::InvalidStockKind],
    InvalidSurfaceKind => vec![Self::InvalidSurfaceKind],
    InvalidLayoutId => vec![Self::InvalidLayoutId],
    RegionMapChart => vec![Self::RegionMapChart],
    SizesOnNonBubble => vec![Self::SizesOnNonBubble],
    InvalidComboSeriesType => vec![Self::InvalidComboSeriesType],
    ChartTypeNotConvertible => vec![Self::ChartTypeNotConvertible],
    NeedChartCategories => vec![Self::NeedChartCategories],
    SmartArtNotFound { id: String } => vec![Self::SmartArtNotFound { id: "para:1/smartdiagram:1".into() }],
    SmartArtPointNotFound { id: String } => {
        vec![Self::SmartArtPointNotFound { id: "para:1/smartdiagram:1/pt:2".into() }]
    },
    LastSmartArtPoint => vec![Self::LastSmartArtPoint],
    InvalidSmartArtLayout => vec![Self::InvalidSmartArtLayout],
    InvalidSmartArtRole => vec![Self::InvalidSmartArtRole],
    InvalidSmartArtColors => vec![Self::InvalidSmartArtColors],
    InvalidSmartArtStyle => vec![Self::InvalidSmartArtStyle],
    SmartArtLayoutNotConvertible => vec![Self::SmartArtLayoutNotConvertible],
    SmartArtPointParent => vec![Self::SmartArtPointParent],
    SmartArtAsstOnList => vec![Self::SmartArtAsstOnList],
    SmartArtListDepth => vec![Self::SmartArtListDepth],
    OleNotFound { id: String } => vec![Self::OleNotFound { id: "para:1/ole:_1".into() }],
    InkNotFound { id: String } => vec![Self::InkNotFound { id: "para:1/ink:9".into() }],
    Model3dNotFound { id: String } => vec![Self::Model3dNotFound { id: "para:1/model3d:9".into() }],
    MathDrawingNotFound { id: String } => {
        vec![Self::MathDrawingNotFound { id: "sheet:Sheet1/math:2".into() }]
    },
    MathDrawingNoExpr => vec![Self::MathDrawingNoExpr],
    SlicerNotFound { id: String } => vec![Self::SlicerNotFound { id: "sheet:Sheet1/slicer:2".into() }],
    SlicerFieldNotFound { name: String } => vec![Self::SlicerFieldNotFound { name: "产品".into() }],
    InvalidSlicer { field: String } => vec![Self::InvalidSlicer { field: "kind".into() }],
    NeedSetSlicerViewField => vec![Self::NeedSetSlicerViewField],
    NeedSetSlicerFilterField => vec![Self::NeedSetSlicerFilterField],
    ConnectionNotFound { id: String } => vec![Self::ConnectionNotFound { id: "connection:1".into() }],
    InvalidConnection { field: String } => {
        vec![Self::InvalidConnection { field: "type".into() }]
    },
    ExternalLinkNotFound { id: String } => vec![Self::ExternalLinkNotFound { id: "extlink:1".into() }],
    InvalidExternalLink { field: String } => {
        vec![Self::InvalidExternalLink { field: "path".into() }]
    },
    ExternalLinkInUse => vec![Self::ExternalLinkInUse],
    CustomViewNotFound { id: String } => vec![Self::CustomViewNotFound { id: "customView:1".into() }],
    CellStyleNotFound { id: String } => vec![Self::CellStyleNotFound { id: "cellStyle:1".into() }],
    NeedInsertCellStyleField => vec![Self::NeedInsertCellStyleField],
    InvalidNamedCellStyle { field: String } => {
        vec![Self::InvalidNamedCellStyle { field: "name".into() }]
    },
    CannotDeleteBuiltinCellStyle => vec![Self::CannotDeleteBuiltinCellStyle],
    InvalidCustomView { field: String } => {
        vec![Self::InvalidCustomView { field: "name".into() }]
    },
    ZoomNotFound { id: String } => vec![Self::ZoomNotFound { id: "slide:1/zoom:3".into() }],
    InvalidZoomType => vec![Self::InvalidZoomType],
    InvalidShowPrType => vec![Self::InvalidShowPrType],
    InvalidFldType => vec![Self::InvalidFldType],
    ZoomTargetSelf => vec![Self::ZoomTargetSelf],
    NeedZoomSetField => vec![Self::NeedZoomSetField],
    InvalidZoomTargets => vec![Self::InvalidZoomTargets],
    WatermarkNotFound { id: String } => vec![Self::WatermarkNotFound { id: "header:default:1/watermark:1".into() }],
    WatermarkNeedTextOrFile => vec![Self::WatermarkNeedTextOrFile],
    TextBoxLinkCrossPart => vec![Self::TextBoxLinkCrossPart],
    InvalidHexColor => vec![Self::InvalidHexColor],
    NeedShapeFormatField => vec![Self::NeedShapeFormatField],
    InvalidTextFillType => vec![Self::InvalidTextFillType],
    NeedGradientStops => vec![Self::NeedGradientStops],
    DuplicateGradientPos => vec![Self::DuplicateGradientPos],
    NeedTextOutlineField => vec![Self::NeedTextOutlineField],
    InvalidTextAlign => vec![Self::InvalidTextAlign],
    InvalidTextAnchor => vec![Self::InvalidTextAnchor],
    InvalidTextUnderline => vec![Self::InvalidTextUnderline],
    EmptyShapeTypeface => vec![Self::EmptyShapeTypeface],
    InvalidTextBoxDirection => vec![Self::InvalidTextBoxDirection],
    InvalidTextBoxTextAnchor => vec![Self::InvalidTextBoxTextAnchor],
    CannotLinkFirstSection => vec![Self::CannotLinkFirstSection],
    HfTypeInactive { kind: String, hf_type: String } => vec![Self::HfTypeInactive {
        kind: "header".into(),
        hf_type: "even".into(),
    }],
    HfInsertWhileLinked => vec![Self::HfInsertWhileLinked],
    InvalidSectValue { field: String } => vec![Self::InvalidSectValue { field: "orient".into() }],
    TableNotFound { id: String } => vec![Self::TableNotFound { id: "table:1".into() }],
    LastTableRow => vec![Self::LastTableRow],
    LastTableCol => vec![Self::LastTableCol],
    SplitContinueCell => vec![Self::SplitContinueCell],
    NothingToSplit => vec![Self::NothingToSplit],
    TableHasRevision => vec![Self::TableHasRevision],
    TableGridMismatch => vec![Self::TableGridMismatch],
    MergeNotSimple => vec![Self::MergeNotSimple],
    NeedTableCellsMatch => vec![Self::NeedTableCellsMatch],
    RevisionNotFound { id: String } => vec![Self::RevisionNotFound { id: "rev:1".into() }],
    SetParaTextHasRevision => vec![Self::SetParaTextHasRevision],
    HyperlinkNotFound { id: String } => vec![Self::HyperlinkNotFound { id: "link:1".into() }],
    BookmarkNotFound { id: String } => vec![Self::BookmarkNotFound { id: "bookmark:1".into() }],
    EditRangeNotFound { id: String } => vec![Self::EditRangeNotFound { id: "perm:1".into() }],
    InvalidEdGrp => vec![Self::InvalidEdGrp],
    EmptyEditRangeEd => vec![Self::EmptyEditRangeEd],
    InvalidBibliographyStyle => vec![Self::InvalidBibliographyStyle],
    SetParaTextHasHyperlink => vec![Self::SetParaTextHasHyperlink],
    HyperlinkNeedTarget => vec![Self::HyperlinkNeedTarget],
    XlsxHyperlinkNeedTarget => vec![Self::XlsxHyperlinkNeedTarget],
    EmptyHyperlinkUrl => vec![Self::EmptyHyperlinkUrl],
    EmptyHyperlinkLocation => vec![Self::EmptyHyperlinkLocation],
    HyperlinkOverlap => vec![Self::HyperlinkOverlap],
    SparklineNotFound { id: String } => vec![Self::SparklineNotFound { id: "sheet:Sheet1/spark:1".into() }],
    SparklineOverlap => vec![Self::SparklineOverlap],
    SparklineHostNotCell => vec![Self::SparklineHostNotCell],
    SparklineFormulaNotLine => vec![Self::SparklineFormulaNotLine],
    SparklineGroupHasMany => vec![Self::SparklineGroupHasMany],
    EmptySparklineFormula => vec![Self::EmptySparklineFormula],
    InvalidSparklineType => vec![Self::InvalidSparklineType],
    InvalidSparkline { field: String } => {
        vec![Self::InvalidSparkline { field: "manualMin".into() }]
    },
    NeedSetSparklineField => vec![Self::NeedSetSparklineField],
    InvalidBookmarkName => vec![Self::InvalidBookmarkName],
    BookmarkNameTaken => vec![Self::BookmarkNameTaken],
    CustomShowNameTooLong => vec![Self::CustomShowNameTooLong],
    CustomShowNameTaken => vec![Self::CustomShowNameTaken],
    NeedCustomShowField => vec![Self::NeedCustomShowField],
    BookmarkRangeReversed => vec![Self::BookmarkRangeReversed],
    BookmarkRangeCrossStory => vec![Self::BookmarkRangeCrossStory],
    NeedSetHyperlinkField => vec![Self::NeedSetHyperlinkField],
    NeedXlsxSetHyperlinkField => vec![Self::NeedXlsxSetHyperlinkField],
    PptxHyperlinkNeedTarget => vec![Self::PptxHyperlinkNeedTarget],
    PptxNeedSetHyperlinkField => vec![Self::PptxNeedSetHyperlinkField],
    ShapeAlreadyHasHyperlink => vec![Self::ShapeAlreadyHasHyperlink],
    InvalidHyperlinkOn => vec![Self::InvalidHyperlinkOn],
    InvalidHyperlinkEvent => vec![Self::InvalidHyperlinkEvent],
    RunAlreadyHasHyperlink => vec![Self::RunAlreadyHasHyperlink],
    PptxRunLinkNeedsRun => vec![Self::PptxRunLinkNeedsRun],
    PptxHyperlinkSelfSlide => vec![Self::PptxHyperlinkSelfSlide],
    EmptyHyperlinkFile => vec![Self::EmptyHyperlinkFile],
    FieldNotFound { id: String } => vec![Self::FieldNotFound { id: "field:1".into() }],
    TocNotFound { id: String } => vec![Self::TocNotFound { id: "toc:1".into() }],
    CaptionNotFound { id: String } => vec![Self::CaptionNotFound { id: "caption:1".into() }],
    CrossRefNotFound { id: String } => vec![Self::CrossRefNotFound { id: "xref:1".into() }],
    IndexNotFound { id: String } => vec![Self::IndexNotFound { id: "index:1".into() }],
    IndexEntryNotFound { id: String } => vec![Self::IndexEntryNotFound { id: "xe:1".into() }],
    SourceNotFound { id: String } => vec![Self::SourceNotFound { id: "source:1".into() }],
    CitationNotFound { id: String } => vec![Self::CitationNotFound { id: "cite:1".into() }],
    BibliographyNotFound { id: String } => vec![Self::BibliographyNotFound { id: "biblio:1".into() }],
    DocPartNotFound { id: String } => vec![Self::DocPartNotFound { id: "docPart:1".into() }],
    EmptyDocPartName => vec![Self::EmptyDocPartName],
    DuplicateDocPartName => vec![Self::DuplicateDocPartName],
    FontNotFound { id: String } => vec![Self::FontNotFound { id: "font:1".into() }],
    EmptyFontName => vec![Self::EmptyFontName],
    DuplicateEmbeddedFont => vec![Self::DuplicateEmbeddedFont],
    InvalidFontStyle => vec![Self::InvalidFontStyle],
    InvalidFontFile => vec![Self::InvalidFontFile],
    NeedFontEmbedField => vec![Self::NeedFontEmbedField],
    InvalidPageBackgroundColor => vec![Self::InvalidPageBackgroundColor],
    InvalidPageBackgroundFill => vec![Self::InvalidPageBackgroundFill],
    InvalidThemeFmtIndex => vec![Self::InvalidThemeFmtIndex],
    InvalidThemeFmtKind => vec![Self::InvalidThemeFmtKind],
    NeedThemeOverrideField => vec![Self::NeedThemeOverrideField],
    NeedPageBordersField => vec![Self::NeedPageBordersField],
    InvalidPageBorderVal => vec![Self::InvalidPageBorderVal],
    InvalidPageBorderOffset => vec![Self::InvalidPageBorderOffset],
    InvalidPageBorderDisplay => vec![Self::InvalidPageBorderDisplay],
    InvalidPageBorderZOrder => vec![Self::InvalidPageBorderZOrder],
    InvalidPageBorderColor => vec![Self::InvalidPageBorderColor],
    InvalidDocPartGallery => vec![Self::InvalidDocPartGallery],
    InvalidDocPartBehavior => vec![Self::InvalidDocPartBehavior],
    EmptyDocPartText => vec![Self::EmptyDocPartText],
    MathNotFound { id: String } => vec![Self::MathNotFound { id: "math:1".into() }],
    RubyNotFound { id: String } => vec![Self::RubyNotFound { id: "ruby:1".into() }],
    ContentControlNotFound { id: String } => vec![Self::ContentControlNotFound { id: "sdt:1".into() }],
    InvalidTocKind => vec![Self::InvalidTocKind],
    InvalidSeqIdentifier => vec![Self::InvalidSeqIdentifier],
    InvalidCrossRefKind => vec![Self::InvalidCrossRefKind],
    InvalidIndexInstr => vec![Self::InvalidIndexInstr],
    InvalidIndexEntry => vec![Self::InvalidIndexEntry],
    InvalidSourceTag => vec![Self::InvalidSourceTag],
    DuplicateSourceTag => vec![Self::DuplicateSourceTag],
    InvalidSourceType => vec![Self::InvalidSourceType],
    EmptySourceTitle => vec![Self::EmptySourceTitle],
    InvalidCitationTag => vec![Self::InvalidCitationTag],
    InvalidBibliographyInstr => vec![Self::InvalidBibliographyInstr],
    InvalidMathKind => vec![Self::InvalidMathKind],
    InvalidMathJc => vec![Self::InvalidMathJc],
    MathJcOnInline => vec![Self::MathJcOnInline],
    EmptyMathText => vec![Self::EmptyMathText],
    MathNeedTextOrExpr => vec![Self::MathNeedTextOrExpr],
    EmptyMathExpr => vec![Self::EmptyMathExpr],
    InvalidMathExprType { got: String } => vec![Self::InvalidMathExprType { got: "foo".into() }],
    EmptyMathSlot { field: String } => vec![Self::EmptyMathSlot { field: "num".into() }],
    InvalidContentControlKind => vec![Self::InvalidContentControlKind],
    EmptyContentControlText => vec![Self::EmptyContentControlText],
    InvalidContentControlKeep => vec![Self::InvalidContentControlKeep],
    InvalidContentControlItems => vec![Self::InvalidContentControlItems],
    ContentControlFieldMismatch => vec![Self::ContentControlFieldMismatch],
    ContentControlBound => vec![Self::ContentControlBound],
    ContentControlRepeating => vec![Self::ContentControlRepeating],
    ContentControlGroup => vec![Self::ContentControlGroup],
    ContentControlHasTable => vec![Self::ContentControlHasTable],
    SetParaTextHasField => vec![Self::SetParaTextHasField],
    SetParaTextHasMath => vec![Self::SetParaTextHasMath],
    SetParaTextHasRuby => vec![Self::SetParaTextHasRuby],
    SetParaTextHasSdt => vec![Self::SetParaTextHasSdt],
    EmptyFieldInstr => vec![Self::EmptyFieldInstr],
    FieldNeedInstrOrCode => vec![Self::FieldNeedInstrOrCode],
    InvalidFieldCode { got: String } => vec![Self::InvalidFieldCode { got: "FOO".into() }],
    FieldNeedArg { code: String } => vec![Self::FieldNeedArg { code: "REF".into() }],
    InvalidFieldKeep => vec![Self::InvalidFieldKeep],
    NeedSetFieldInstr => vec![Self::NeedSetFieldInstr],
    NumberingNotFound { id: String } => vec![Self::NumberingNotFound { id: "num:1".into() }],
    AbstractNumNotFound { id: String } => vec![Self::AbstractNumNotFound { id: "abstractNum:1".into() }],
    InvalidNumberingKind => vec![Self::InvalidNumberingKind],
    InvalidListNumFmt => vec![Self::InvalidListNumFmt],
    NeedSetNumberingField => vec![Self::NeedSetNumberingField],
    InvalidIlvl => vec![Self::InvalidIlvl],
    NeedSetParaNumberingNum => vec![Self::NeedSetParaNumberingNum],
    StyleNotFound { id: String } => vec![Self::StyleNotFound { id: "style:Normal".into() }],
    SetStyleNeedsParagraph => vec![Self::SetStyleNeedsParagraph],
    InvalidStyleType => vec![Self::InvalidStyleType],
    StyleIdTaken => vec![Self::StyleIdTaken],
    InvalidStyleIdToken => vec![Self::InvalidStyleIdToken],
    StyleNotCustom => vec![Self::StyleNotCustom],
    NeedInsertStyleField => vec![Self::NeedInsertStyleField],
    SetStyleSpacingNeedsParagraph => vec![Self::SetStyleSpacingNeedsParagraph],
    SetRunStyleNeedsCharacter => vec![Self::SetRunStyleNeedsCharacter],
    SetTableStyleNeedsTable => vec![Self::SetTableStyleNeedsTable],
    NeedSetTableStyleField => vec![Self::NeedSetTableStyleField],
    NeedSetTableFloatField => vec![Self::NeedSetTableFloatField],
    InvalidTblAnchor => vec![Self::InvalidTblAnchor],
    NeedSetTableWidthField => vec![Self::NeedSetTableWidthField],
    NeedSetTableAlignField => vec![Self::NeedSetTableAlignField],
    NeedSetTableRowHeightField => vec![Self::NeedSetTableRowHeightField],
    InvalidTblWidthType => vec![Self::InvalidTblWidthType],
    InvalidTblAlign => vec![Self::InvalidTblAlign],
    InvalidRowHeightRule => vec![Self::InvalidRowHeightRule],
    TableColsCount => vec![Self::TableColsCount],
    TableRowHeightsCount => vec![Self::TableRowHeightsCount],
    NeedSetTableCellMarField => vec![Self::NeedSetTableCellMarField],
    NeedSetCellFormatField => vec![Self::NeedSetCellFormatField],
    NeedSetStyleField => vec![Self::NeedSetStyleField],
    EmptyStyleFont => vec![Self::EmptyStyleFont],
    InvalidStyleColor => vec![Self::InvalidStyleColor],
    NeedSetParaStyleStyle => vec![Self::NeedSetParaStyleStyle],
    NeedSetParaFormatField => vec![Self::NeedSetParaFormatField],
    NeedSetRunFormatField => vec![Self::NeedSetRunFormatField],
    NeedParaBordersField => vec![Self::NeedParaBordersField],
    InvalidHighlight => vec![Self::InvalidHighlight],
    InvalidVertAlign => vec![Self::InvalidVertAlign],
    InvalidTabVal => vec![Self::InvalidTabVal],
    InvalidEm => vec![Self::InvalidEm],
    NeedEastAsianLayoutField => vec![Self::NeedEastAsianLayoutField],
    InvalidCombineBrackets => vec![Self::InvalidCombineBrackets],
    EmptyRubyBase => vec![Self::EmptyRubyBase],
    EmptyRubyRt => vec![Self::EmptyRubyRt],
    EmptyRubyLid => vec![Self::EmptyRubyLid],
    InvalidRubyAlign => vec![Self::InvalidRubyAlign],
    NeedSetRubyField => vec![Self::NeedSetRubyField],
    InvalidCharacterSpacing => vec![Self::InvalidCharacterSpacing],
    NeedSetEastAsianSettingsField => vec![Self::NeedSetEastAsianSettingsField],
    EmptyKinsokuLang => vec![Self::EmptyKinsokuLang],
    EmptyKinsokuChars => vec![Self::EmptyKinsokuChars],
    NeedSetCellStyleField => vec![Self::NeedSetCellStyleField],
    InvalidCellStyle { field: String } => vec![Self::InvalidCellStyle { field: "numFmt".into() }],
    NeedSetColWidthField => vec![Self::NeedSetColWidthField],
    InvalidColWidth => vec![Self::InvalidColWidth],
    NeedSetRowHeightField => vec![Self::NeedSetRowHeightField],
    InvalidRowHeight => vec![Self::InvalidRowHeight],
    MergeNeedsMultipleCells => vec![Self::MergeNeedsMultipleCells],
    RowRangeReversed => vec![Self::RowRangeReversed],
    LayoutWorksheetOnly => vec![Self::LayoutWorksheetOnly],
    NeedSetDataValidationField => vec![Self::NeedSetDataValidationField],
    NeedDataValidationFormula { field: String } => {
        vec![Self::NeedDataValidationFormula { field: "formula1".into() }]
    },
    InvalidDataValidation { field: String } => {
        vec![Self::InvalidDataValidation { field: "type".into() }]
    },
    DataValidationNotFound { id: String } => {
        vec![Self::DataValidationNotFound { id: "sheet:Sheet1/dv:1".into() }]
    },
    NeedSetConditionalFormatField => vec![Self::NeedSetConditionalFormatField],
    NeedConditionalFormatFormula { field: String } => {
        vec![Self::NeedConditionalFormatFormula { field: "formulas".into() }]
    },
    InvalidConditionalFormat { field: String } => {
        vec![Self::InvalidConditionalFormat { field: "type".into() }]
    },
    ConditionalFormatNotFound { id: String } => {
        vec![Self::ConditionalFormatNotFound { id: "sheet:Sheet1/cf:1".into() }]
    },
    NeedSetSheetTableField => vec![Self::NeedSetSheetTableField],
    InvalidSheetTable { field: String } => {
        vec![Self::InvalidSheetTable { field: "ref".into() }]
    },
    SheetTableNotFound { id: String } => {
        vec![Self::SheetTableNotFound { id: "sheet:Sheet1/table:1".into() }]
    },
    NeedSetPivotTableField => vec![Self::NeedSetPivotTableField],
    InvalidPivotTable { field: String } => {
        vec![Self::InvalidPivotTable { field: "source".into() }]
    },
    PivotTableNotFound { id: String } => {
        vec![Self::PivotTableNotFound { id: "sheet:Sheet1/pivot:1".into() }]
    },
    NeedSetDefinedNameField => vec![Self::NeedSetDefinedNameField],
    InvalidDefinedName { field: String } => {
        vec![Self::InvalidDefinedName { field: "name".into() }]
    },
    DefinedNameNotFound { id: String } => {
        vec![Self::DefinedNameNotFound { id: "name:1".into() }]
    },
    NeedSetProtectedRangeField => vec![Self::NeedSetProtectedRangeField],
    InvalidProtectedRangeName => vec![Self::InvalidProtectedRangeName],
    DuplicateProtectedRangeName => vec![Self::DuplicateProtectedRangeName],
    ProtectedRangeNotFound { id: String } => {
        vec![Self::ProtectedRangeNotFound { id: "sheet:Sheet1/editRange:1".into() }]
    },
    NeedSetPrintSetupField => vec![Self::NeedSetPrintSetupField],
    InvalidPrintSetup { field: String } => {
        vec![Self::InvalidPrintSetup { field: "left".into() }]
    },
    NeedSetSheetViewField => vec![Self::NeedSetSheetViewField],
    InvalidSheetView { field: String } => {
        vec![Self::InvalidSheetView { field: "zoom".into() }]
    },
    NeedSetAutoFilterField => vec![Self::NeedSetAutoFilterField],
    InvalidAutoFilter { field: String } => {
        vec![Self::InvalidAutoFilter { field: "ref".into() }]
    },
    NeedSetSortStateField => vec![Self::NeedSetSortStateField],
    InvalidSortState { field: String } => {
        vec![Self::InvalidSortState { field: "ref".into() }]
    },
    NeedSetOutlineField => vec![Self::NeedSetOutlineField],
    InvalidOutline { field: String } => {
        vec![Self::InvalidOutline { field: "collapsed".into() }]
    },
    OutlineLevelMax => vec![Self::OutlineLevelMax],
    InvalidAlign => vec![Self::InvalidAlign],
    InvalidLineRule => vec![Self::InvalidLineRule],
    InvalidUnderline => vec![Self::InvalidUnderline],
    RunNotFound { id: String } => vec![Self::RunNotFound { id: "para:1/run:1".into() }],
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
            Self::Reversed {
                input: "D20:A1".into(),
            },
            Self::InvalidColLetters {
                letters: "1A".into(),
            },
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
            Self::InsertNeedsPara,
            Self::SheetPicPrefix,
            Self::SheetPicContainer,
            Self::InsertNeedsSheet,
            Self::SlidePicPrefix,
            Self::SlidePicContainer,
            Self::CommentPicExportOnly,
            Self::SlideMediaForm,
            Self::InsertMediaNeedsSlide,
            Self::InsertNeedsSlide,
            Self::InsertShapeHost,
            Self::InsertConnectorNeedsSlide,
            Self::SlideCxnForm,
            Self::SetNotesTextNeedsSlide,
            Self::PptxCommentForm,
            Self::InsertCommentNeedsSlide,
            Self::XlsxCommentForm,
            Self::InsertCommentNeedsCell,
            Self::HfSlotPrefix,
            Self::TxbxPrefix,
            Self::TxbxContainer,
            Self::TxbxIndexNotInt,
            Self::TxbxIndexFromOne,
            Self::SetParaOnBox,
            Self::InsertNeedsHostPara,
            Self::DocxCxnPrefix,
            Self::DocxCxnContainer,
            Self::DocxShapePrefix,
            Self::DocxShapeContainer,
            Self::DocxInkPrefix,
            Self::DocxInkContainer,
            Self::DocxInkIndexNotInt,
            Self::DocxInkIndexFromOne,
            Self::DocxModel3dPrefix,
            Self::DocxModel3dContainer,
            Self::DocxModel3dIndexNotInt,
            Self::DocxModel3dIndexFromOne,
            Self::GroupNeedsDocument,
            Self::SheetSpPrefix,
            Self::SheetSpContainer,
            Self::SpIndexNotInt,
            Self::SpIndexFromOne,
            Self::InsertTextBoxNeedsSheet,
            Self::SheetShapePrefix,
            Self::SheetShapeContainer,
            Self::InsertShapeNeedsSheet,
            Self::SheetCxnPrefix,
            Self::SheetCxnContainer,
            Self::CxnIndexNotInt,
            Self::CxnIndexFromOne,
            Self::InsertConnectorNeedsSheet,
            Self::GroupNeedsSheet,
            Self::SetShapeTextNotCxn,
            Self::SheetSmartArtPrefix,
            Self::SheetSmartArtContainer,
            Self::InsertSmartArtNeedsSheet,
            Self::InsertParaNeedsBox,
            Self::NotePrefix,
            Self::NoteContainer,
            Self::NoteSep,
            Self::CommentPrefix,
            Self::CommentContainer,
            Self::ChartPrefix,
            Self::ChartContainer,
            Self::ChartIndexNotInt,
            Self::ChartIndexFromOne,
            Self::SeriesPrefix,
            Self::SeriesIndexNotInt,
            Self::TrendlinePrefix,
            Self::TrendlineIndexNotInt,
            Self::TrendlineIndexFromOne,
            Self::ErrorBarsPrefix,
            Self::ErrorBarsIndexNotInt,
            Self::ErrorBarsIndexFromOne,
            Self::InsertChartNeedsPara,
            Self::InsertChartNeedsSheet,
            Self::InsertChartNeedsSlide,
            Self::InsertChartSheetNeedsWorkbook,
            Self::InsertSheetNeedsWorkbook,
            Self::InsertSlideNeedsPresentation,
            Self::InsertSlideSectionNeedsPresentation,
            Self::InsertCustomShowNeedsPresentation,
            Self::CustomShowPrefix,
            Self::CustomShowIndexNotInt,
            Self::CustomShowIndexFromOne,
            Self::InsertLayoutNeedsMaster,
            Self::InsertPlaceholderNeedsHost,
            Self::InsertMasterNeedsPresentation,
            Self::DeleteMasterNeedsMaster,
            Self::SetSlideSizeNeedsPresentation,
            Self::SetShowPrNeedsPresentation,
            Self::SetProtectionNeedsPresentation,
            Self::SetThemeNeedsMaster,
            Self::SetThemeNeedsDocument,
            Self::SetThemeNeedsWorkbook,
            Self::SetSlideSectionNeedsSlide,
            Self::SlideOpNeedsSlide,
            Self::SetTransitionNeedsHost,
            Self::MasterForm,
            Self::MasterPlaceholderLocked,
            Self::LayoutForm,
            Self::SetHandoutTextNeedsHandout,
            Self::InsertDefinedNameNeedsWorkbook,
            Self::InsertConnectionNeedsWorkbook,
            Self::InsertExternalLinkNeedsWorkbook,
            Self::InsertCustomViewNeedsWorkbook,
            Self::CustomViewPrefix,
            Self::CustomViewIndexNotInt,
            Self::CustomViewIndexFromOne,
            Self::InsertCellStyleNeedsWorkbook,
            Self::CellStylePrefix,
            Self::CellStyleIndexNotInt,
            Self::CellStyleIndexFromOne,
            Self::ConnectionPrefix,
            Self::ConnectionIndexNotInt,
            Self::ConnectionIndexFromOne,
            Self::ExternalLinkPrefix,
            Self::ExternalLinkIndexNotInt,
            Self::ExternalLinkIndexFromOne,
            Self::DefinedNamePrefix,
            Self::NameIndexNotInt,
            Self::NameIndexFromOne,
            Self::DeleteChartSheetNeedsChartsheet,
            Self::DeleteSheetNeedsWorksheet,
            Self::SheetOpNeedsSheet,
            Self::SetProtectionNeedsSheetOrWorkbook,
            Self::InsertProtectedRangeNeedsSheet,
            Self::SheetEditRangePrefix,
            Self::EditRangeIndexNotInt,
            Self::EditRangeIndexFromOne,
            Self::SheetDvPrefix,
            Self::DvIndexNotInt,
            Self::DvIndexFromOne,
            Self::SheetCfPrefix,
            Self::CfIndexNotInt,
            Self::CfIndexFromOne,
            Self::SheetTablePrefix,
            Self::SheetPivotPrefix,
            Self::PivotIndexNotInt,
            Self::PivotIndexFromOne,
            Self::SheetChartPrefix,
            Self::SheetChartContainer,
            Self::SlideChartPrefix,
            Self::SlideChartContainer,
            Self::SmartArtPrefix,
            Self::SmartArtContainer,
            Self::SmartArtIndexNotInt,
            Self::SmartArtIndexFromOne,
            Self::SmartArtPtPrefix,
            Self::InsertSmartArtNeedsPara,
            Self::SlideSmartArtPrefix,
            Self::SlideSmartArtContainer,
            Self::InsertSmartArtNeedsSlide,
            Self::SlideTablePrefix,
            Self::SlideTableContainer,
            Self::SlideTableIndexNotInt,
            Self::SlideTableIndexFromOne,
            Self::SlideTableRowForm,
            Self::SlideTableCellForm,
            Self::InsertTableNeedsSlide,
            Self::SetTableCellNeedsCell,
            Self::OlePrefix,
            Self::OleContainer,
            Self::OleIndexNotInt,
            Self::OleIndexFromOne,
            Self::InsertOleNeedsPara,
            Self::SlideOlePrefix,
            Self::SlideOleContainer,
            Self::InsertOleNeedsSlide,
            Self::InsertOleNeedsSheet,
            Self::SheetOlePrefix,
            Self::SheetOleContainer,
            Self::InsertModel3dNeedsSheet,
            Self::SheetModel3dPrefix,
            Self::SheetModel3dContainer,
            Self::InsertModel3dNeedsSlide,
            Self::SlideModel3dPrefix,
            Self::SlideModel3dContainer,
            Self::InsertMathNeedsSheet,
            Self::SheetMathPrefix,
            Self::SheetMathContainer,
            Self::InsertMathNeedsSlide,
            Self::SlideMathPrefix,
            Self::SlideMathContainer,
            Self::InsertSlicerNeedsSheet,
            Self::SheetSlicerPrefix,
            Self::SheetSlicerContainer,
            Self::SlicerIndexNotInt,
            Self::SlicerIndexFromOne,
            Self::InsertSlicerNeedsTableOrPivot,
            Self::InsertZoomNeedsSlide,
            Self::ZoomTargetSlide,
            Self::SlideZoomPrefix,
            Self::SlideZoomContainer,
            Self::ZoomIndexNotInt,
            Self::ZoomIndexFromOne,
            Self::InsertWatermarkNeedsDocument,
            Self::WatermarkPrefix,
            Self::WatermarkContainer,
            Self::WatermarkIndexNotInt,
            Self::WatermarkIndexFromOne,
            Self::EvenOddNeedsDocument,
            Self::SettingsNeedDocument,
            Self::TablePrefix,
            Self::TableIndexNotInt,
            Self::TableIndexFromOne,
            Self::TableCellForm,
            Self::InsertTableAnchor,
            Self::DeleteTableNeedsTable,
            Self::NeedTableId,
            Self::InsertRowNeedsTableOrRow,
            Self::DeleteRowNeedsRow,
            Self::SetTableRowHeightNeedsRow,
            Self::SplitNeedsCell,
            Self::RevPrefix,
            Self::RevIndexNotInt,
            Self::RevIndexFromOne,
            Self::LinkPrefix,
            Self::LinkIndexNotInt,
            Self::LinkIndexFromOne,
            Self::InsertLinkNeedsPara,
            Self::SheetLinkPrefix,
            Self::InsertHyperlinkNeedsSheet,
            Self::InsertHyperlinkNeedsShape,
            Self::SlideLinkPrefix,
            Self::PptxJumpNeedsSlide,
            Self::SheetSparkPrefix,
            Self::SparkIndexNotInt,
            Self::SparkIndexFromOne,
            Self::InsertSparklineNeedsSheet,
            Self::BookmarkPrefix,
            Self::BookmarkIndexNotInt,
            Self::BookmarkIndexFromOne,
            Self::InsertBookmarkNeedsPara,
            Self::PermPrefix,
            Self::PermIndexNotInt,
            Self::PermIndexFromOne,
            Self::InsertEditRangeNeedsPara,
            Self::SetBibliographyStyleNeedsDocument,
            Self::FieldPrefix,
            Self::FieldIndexNotInt,
            Self::FieldIndexFromOne,
            Self::InsertFieldNeedsPara,
            Self::PptxFieldForm,
            Self::InsertFieldNeedsShape,
            Self::InsertNumberingNeedsDocument,
            Self::InsertStyleNeedsDocument,
            Self::NumPrefix,
            Self::NumIndexNotInt,
            Self::NumIndexFromOne,
            Self::AbstractNumPrefix,
            Self::AbstractNumIndexNotInt,
            Self::SetParaNumberingNeedsPara,
            Self::StylePrefix,
            Self::SetParaStyleNeedsPara,
            Self::SetRunFormatNeedsRun,
            Self::SetRunStyleNeedsStoryRun,
            Self::RunPrefix,
            Self::RunIndexNotInt,
            Self::RunIndexFromOne,
            Self::InsertSectionNeedsBodyPara,
            Self::TocPrefix,
            Self::TocIndexNotInt,
            Self::TocIndexFromOne,
            Self::InsertTocNeedsBodyPara,
            Self::CaptionPrefix,
            Self::CaptionIndexNotInt,
            Self::CaptionIndexFromOne,
            Self::InsertCaptionNeedsBodyPara,
            Self::XrefPrefix,
            Self::XrefIndexNotInt,
            Self::XrefIndexFromOne,
            Self::InsertCrossRefNeedsPara,
            Self::IndexPrefix,
            Self::IndexIndexNotInt,
            Self::IndexIndexFromOne,
            Self::InsertIndexNeedsBodyPara,
            Self::XePrefix,
            Self::XeIndexNotInt,
            Self::XeIndexFromOne,
            Self::InsertIndexEntryNeedsPara,
            Self::SourcePrefix,
            Self::SourceIndexNotInt,
            Self::SourceIndexFromOne,
            Self::InsertSourceNeedsDocument,
            Self::CitePrefix,
            Self::CiteIndexNotInt,
            Self::CiteIndexFromOne,
            Self::InsertCitationNeedsPara,
            Self::BiblioPrefix,
            Self::BiblioIndexNotInt,
            Self::BiblioIndexFromOne,
            Self::InsertBibliographyNeedsBodyPara,
            Self::DocPartPrefix,
            Self::DocPartIndexNotInt,
            Self::DocPartIndexFromOne,
            Self::InsertDocPartNeedsDocument,
            Self::InsertDocPartContentNeedsBodyPara,
            Self::FontPrefix,
            Self::FontIndexNotInt,
            Self::FontIndexFromOne,
            Self::InsertEmbeddedFontNeedsDocument,
            Self::SetFontEmbedNeedsDocument,
            Self::SetPageBackgroundNeedsDocument,
            Self::MathPrefix,
            Self::MathIndexNotInt,
            Self::MathIndexFromOne,
            Self::InsertMathNeedsPara,
            Self::RubyPrefix,
            Self::RubyIndexNotInt,
            Self::RubyIndexFromOne,
            Self::InsertRubyNeedsPara,
            Self::SetEastAsianSettingsNeedsDocument,
            Self::SdtPrefix,
            Self::SdtIndexNotInt,
            Self::SdtIndexFromOne,
            Self::InsertSdtNeedsPara,
            Self::SetPropsNeedsRoot,
            Self::CustomPropPrefix,
            Self::AnimForm,
            Self::AnimIndexNotInt,
            Self::AnimIndexFromOne,
            Self::InsertAnimNeedsHost,
        ]
    }
}

impl SheetNameReason {
    #[cfg(test)]
    fn all_samples() -> Vec<Self> {
        vec![
            Self::Missing,
            Self::Length,
            Self::History,
            Self::IllegalChars,
        ]
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
            Self::RelTarget {
                target: "../x".into(),
            },
            Self::Zip {
                detail: "bad".into(),
            },
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
            | Self::CreateIndexOutOfRange { .. }
            | Self::MutuallyExclusive { .. }
            | Self::ContentObjectsExclusive
            | Self::CreateNeedsObject { .. }
            | Self::CreateMissingObject { .. }
            | Self::OpNotForFormat { .. }
            | Self::ActionNotForFormat { .. }
            | Self::ScopeFieldRequires { .. }
            | Self::SetCellNeedsValueOrFormula
            | Self::FormulaRangeNotTopLeft
            | Self::SharedNeedsRef
            | Self::FormulaMustStartWithEquals
            | Self::InvalidDataTableFormula
            | Self::InvalidCellValue
            | Self::OpRequiresField { .. }
            | Self::SetCellWorksheetOnly
            | Self::InspectWorksheetOnly
            | Self::NeedAtLeastOneSheet
            | Self::LastSlide
            | Self::LastLayout
            | Self::LastMaster
            | Self::LayoutInUse
            | Self::MasterInUse
            | Self::InvalidPlaceholderType
            | Self::InvalidSlideSizeType
            | Self::SlideSizeOutOfRange
            | Self::LastVisibleSheet
            | Self::MoveSheetAfterSelf
            | Self::MoveSlideAfterSelf
            | Self::SlideLayoutMissing
            | Self::DuplicateSheetName { .. }
            | Self::InvalidSheetName { .. }
            | Self::InvalidParaStyle
            | Self::InvalidHfKind
            | Self::InvalidHfType
            | Self::InvalidSectionType
            | Self::SectionTypeOnLastPara
            | Self::CannotDeleteLastSection
            | Self::InvalidTocKind
            | Self::InvalidSeqIdentifier
            | Self::InvalidCrossRefKind
            | Self::InvalidIndexInstr
            | Self::InvalidIndexEntry
            | Self::InvalidSourceTag
            | Self::DuplicateSourceTag
            | Self::InvalidSourceType
            | Self::EmptySourceTitle
            | Self::InvalidCitationTag
            | Self::InvalidBibliographyInstr
            | Self::EmptyDocPartName
            | Self::DuplicateDocPartName
            | Self::EmptyFontName
            | Self::DuplicateEmbeddedFont
            | Self::InvalidFontStyle
            | Self::InvalidFontFile
            | Self::NeedFontEmbedField
            | Self::InvalidPageBackgroundColor
            | Self::InvalidPageBackgroundFill
            | Self::InvalidThemeFmtIndex
            | Self::InvalidThemeFmtKind
            | Self::NeedThemeOverrideField
            | Self::NeedPageBordersField
            | Self::InvalidPageBorderVal
            | Self::InvalidPageBorderOffset
            | Self::InvalidPageBorderDisplay
            | Self::InvalidPageBorderZOrder
            | Self::InvalidPageBorderColor
            | Self::InvalidDocPartGallery
            | Self::InvalidDocPartBehavior
            | Self::EmptyDocPartText
            | Self::InvalidMathKind
            | Self::InvalidMathJc
            | Self::MathJcOnInline
            | Self::InvalidEdGrp
            | Self::EmptyEditRangeEd
            | Self::InvalidBibliographyStyle
            | Self::EmptyMathText
            | Self::InvalidEm
            | Self::InvalidHighlight
            | Self::InvalidVertAlign
            | Self::InvalidTabVal
            | Self::NeedEastAsianLayoutField
            | Self::InvalidCombineBrackets
            | Self::EmptyRubyBase
            | Self::EmptyRubyRt
            | Self::EmptyRubyLid
            | Self::InvalidRubyAlign
            | Self::NeedSetRubyField
            | Self::InvalidCharacterSpacing
            | Self::NeedSetEastAsianSettingsField
            | Self::EmptyKinsokuLang
            | Self::EmptyKinsokuChars
            | Self::MathNeedTextOrExpr
            | Self::EmptyMathExpr
            | Self::InvalidMathExprType { .. }
            | Self::EmptyMathSlot { .. }
            | Self::InvalidContentControlKind
            | Self::EmptyContentControlText
            | Self::InvalidContentControlKeep
            | Self::InvalidContentControlItems
            | Self::ContentControlFieldMismatch
            | Self::CreateHfDuplicate
            | Self::InvalidXlsxHfKind
            | Self::InvalidXlsxHfSide
            | Self::CreateXlsxHfDuplicate
            | Self::NeedXlsxHfContent
            | Self::InvalidPicKind
            | Self::InvalidPicWrap
            | Self::InvalidPicAlign
            | Self::InvalidImageFile
            | Self::InvalidInkMl
            | Self::InvalidMediaFile
            | Self::NeedSetMediaField
            | Self::InvalidMediaTime
            | Self::DuplicateBookmarkName
            | Self::InvalidTrack
            | Self::NeedsPptxSectionList
            | Self::SlideNotInPptxSection
            | Self::CannotLinkFirstSection
            | Self::HfTypeInactive { .. }
            | Self::HfInsertWhileLinked
            | Self::InvalidSectValue { .. }
            | Self::LastTableRow
            | Self::LastTableCol
            | Self::SplitContinueCell
            | Self::NothingToSplit
            | Self::NeedTableCellsMatch
            | Self::HyperlinkNeedTarget
            | Self::EmptyHyperlinkUrl
            | Self::InvalidBookmarkName
            | Self::BookmarkNameTaken
            | Self::CustomShowNameTooLong
            | Self::CustomShowNameTaken
            | Self::NeedCustomShowField
            | Self::BookmarkRangeReversed
            | Self::BookmarkRangeCrossStory
            | Self::NeedSetHyperlinkField
            | Self::PptxHyperlinkNeedTarget
            | Self::PptxNeedSetHyperlinkField
            | Self::ShapeAlreadyHasHyperlink
            | Self::InvalidHyperlinkOn
            | Self::InvalidHyperlinkEvent
            | Self::RunAlreadyHasHyperlink
            | Self::PptxRunLinkNeedsRun
            | Self::PptxHyperlinkSelfSlide
            | Self::EmptyHyperlinkFile
            | Self::EmptyFieldInstr
            | Self::FieldNeedInstrOrCode
            | Self::InvalidFieldCode { .. }
            | Self::FieldNeedArg { .. }
            | Self::InvalidFieldKeep
            | Self::NeedSetFieldInstr
            | Self::InvalidNumberingKind
            | Self::InvalidListNumFmt
            | Self::NeedSetNumberingField
            | Self::InvalidIlvl
            | Self::NeedSetParaNumberingNum
            | Self::SetStyleNeedsParagraph
            | Self::InvalidStyleType
            | Self::StyleIdTaken
            | Self::InvalidStyleIdToken
            | Self::StyleNotCustom
            | Self::NeedInsertStyleField
            | Self::SetStyleSpacingNeedsParagraph
            | Self::SetRunStyleNeedsCharacter
            | Self::SetTableStyleNeedsTable
            | Self::NeedSetTableStyleField
            | Self::NeedSetTableFloatField
            | Self::InvalidTblAnchor
            | Self::NeedSetTableWidthField
            | Self::NeedSetTableAlignField
            | Self::NeedSetTableRowHeightField
            | Self::InvalidTblWidthType
            | Self::InvalidTblAlign
            | Self::InvalidRowHeightRule
            | Self::TableColsCount
            | Self::TableRowHeightsCount
            | Self::NeedSetTableCellMarField
            | Self::NeedSetCellFormatField
            | Self::NeedSetStyleField
            | Self::EmptyStyleFont
            | Self::InvalidStyleColor
            | Self::NeedSetParaStyleStyle
            | Self::NeedSetParaFormatField
            | Self::NeedSetRunFormatField
            | Self::NeedParaBordersField
            | Self::NeedSetCellStyleField
            | Self::InvalidCellStyle { .. }
            | Self::NeedInsertCellStyleField
            | Self::InvalidNamedCellStyle { .. }
            | Self::CannotDeleteBuiltinCellStyle
            | Self::NeedSetColWidthField
            | Self::InvalidColWidth
            | Self::NeedSetRowHeightField
            | Self::InvalidRowHeight
            | Self::MergeNeedsMultipleCells
            | Self::RowRangeReversed
            | Self::LayoutWorksheetOnly
            | Self::NeedSetDataValidationField
            | Self::NeedDataValidationFormula { .. }
            | Self::InvalidDataValidation { .. }
            | Self::NeedSetConditionalFormatField
            | Self::NeedConditionalFormatFormula { .. }
            | Self::InvalidConditionalFormat { .. }
            | Self::NeedSetSheetTableField
            | Self::InvalidSheetTable { .. }
            | Self::NeedSetPivotTableField
            | Self::InvalidPivotTable { .. }
            | Self::NeedSetDefinedNameField
            | Self::InvalidDefinedName { .. }
            | Self::NeedSetProtectedRangeField
            | Self::InvalidProtectedRangeName
            | Self::DuplicateProtectedRangeName
            | Self::NeedSetPrintSetupField
            | Self::InvalidPrintSetup { .. }
            | Self::NeedSetSheetViewField
            | Self::InvalidSheetView { .. }
            | Self::NeedSetAutoFilterField
            | Self::InvalidAutoFilter { .. }
            | Self::NeedSetSortStateField
            | Self::InvalidSortState { .. }
            | Self::NeedSetOutlineField
            | Self::InvalidOutline { .. }
            | Self::OutlineLevelMax
            | Self::SlicerFieldNotFound { .. }
            | Self::InvalidSlicer { .. }
            | Self::NeedSetSlicerViewField
            | Self::NeedSetSlicerFilterField
            | Self::InvalidConnection { .. }
            | Self::InvalidExternalLink { .. }
            | Self::InvalidCustomView { .. }
            | Self::InvalidAlign
            | Self::InvalidLineRule
            | Self::InvalidUnderline
            | Self::LastTextBoxPara
            | Self::LastNotePara
            | Self::NeedNoteNumberingField
            | Self::InvalidNoteKind
            | Self::InvalidNoteNumFmt
            | Self::InvalidNoteRestart
            | Self::LastCommentPara
            | Self::LastStoryPara
            | Self::NeedCommentParaId
            | Self::NeedCommentMetaField
            | Self::CommentRangeCrossPart
            | Self::CommentRangeReversed
            | Self::NeedChartSeriesField
            | Self::NeedTrendlineField
            | Self::NeedErrorBarsField
            | Self::InvalidTrendlineType
            | Self::TrendlineOrderNotPoly
            | Self::TrendlinePeriodNotMovingAvg
            | Self::TrendlineNotSupported
            | Self::InvalidErrorBars
            | Self::CustomErrorBars
            | Self::NeedErrorBarsValue
            | Self::ErrorBarsValueOnStdErr
            | Self::ErrorBarsNotSupported
            | Self::ErrorBarsDuplicateDir
            | Self::SecondaryAxisNotSupported
            | Self::InvalidSeriesAxis
            | Self::LastChartSeries
            | Self::InvalidChartType
            | Self::InvalidTransitionType
            | Self::InvalidAnimType
            | Self::InvalidZoomType
            | Self::InvalidShowPrType
            | Self::InvalidFldType
            | Self::ZoomTargetSelf
            | Self::NeedZoomSetField
            | Self::InvalidZoomTargets
            | Self::InvalidAnimTrigger
            | Self::InvalidAnimDelay
            | Self::InvalidAnimDur
            | Self::InvalidAnimRepeat
            | Self::InvalidAnimPara
            | Self::InvalidAnimMoveIndex
            | Self::NeedAnimSetField
            | Self::AnimNeedEffectDur
            | Self::InvalidTransitionDur
            | Self::InvalidTransitionAdvTm
            | Self::InvalidTransitionDir
            | Self::InvalidTransitionOrient
            | Self::InvalidTransitionSpokes
            | Self::InvalidShapePrst
            | Self::InvalidGeomOp
            | Self::ConnectorEndHost
            | Self::NeedConnectorBind
            | Self::InvalidConnectorPrst
            | Self::GroupNeedsTwo
            | Self::GroupMemberNested
            | Self::NotGrouped
            | Self::InvalidChartAxis
            | Self::InvalidLegendPos
            | Self::PieNeedsOneSeries
            | Self::StockNeedsThreeSeries
            | Self::SurfaceNeedsThreeSeries
            | Self::InvalidOfPieType
            | Self::InvalidStockKind
            | Self::InvalidSurfaceKind
            | Self::InvalidLayoutId
            | Self::SizesOnNonBubble
            | Self::InvalidComboSeriesType
            | Self::NeedChartCategories
            | Self::LastSmartArtPoint
            | Self::InvalidSmartArtLayout
            | Self::InvalidSmartArtRole
            | Self::InvalidSmartArtColors
            | Self::InvalidSmartArtStyle
            | Self::SmartArtPointParent
            | Self::SmartArtAsstOnList
            | Self::SmartArtListDepth
            | Self::TextBoxLinkCrossPart
            | Self::InvalidHexColor
            | Self::NeedShapeFormatField
            | Self::InvalidTextFillType
            | Self::NeedGradientStops
            | Self::DuplicateGradientPos
            | Self::NeedTextOutlineField
            | Self::InvalidTextAlign
            | Self::InvalidTextAnchor
            | Self::InvalidTextUnderline
            | Self::EmptyShapeTypeface
            | Self::InvalidTextBoxDirection
            | Self::InvalidTextBoxTextAnchor
            | Self::InsertPictureWorksheetOnly
            | Self::InsertChartWorksheetOnly
            | Self::InsertTextBoxWorksheetOnly
            | Self::InsertModel3dWorksheetOnly
            | Self::InsertMathWorksheetOnly
            | Self::MathDrawingNoExpr
            | Self::InsertSmartArtWorksheetOnly
            | Self::CommentsWorksheetOnly
            | Self::InsertHyperlinkWorksheetOnly
            | Self::InsertSparklineWorksheetOnly
            | Self::ProtectedRangeWorksheetOnly
            | Self::HyperlinkOverlap
            | Self::SparklineOverlap
            | Self::SparklineHostNotCell
            | Self::SparklineFormulaNotLine
            | Self::SparklineGroupHasMany
            | Self::EmptySparklineFormula
            | Self::InvalidSparklineType
            | Self::InvalidSparkline { .. }
            | Self::NeedSetSparklineField
            | Self::XlsxHyperlinkNeedTarget
            | Self::EmptyHyperlinkLocation
            | Self::NeedXlsxSetHyperlinkField
            | Self::DataSheetMustBeWorksheet
            | Self::InvalidXlsxAnchor
            | Self::InvalidXlsxEditAs
            | Self::TwoCellPictureSize
            | Self::TwoCellChartSize
            | Self::TwoCellTextBoxSize
            | Self::TwoCellSmartArtSize
            | Self::TwoCellModel3dSize
            | Self::TwoCellSlicerSize
            | Self::NotesSlideImage
            | Self::GraphicInRestrictedStory
            | Self::DocxShapeNeedsAnchor
            | Self::NeedPropsField
            | Self::NeedProtectionField
            | Self::WatermarkNeedTextOrFile
            | Self::InvalidProtectionEdit
            | Self::InvalidSheetProtection { .. }
            | Self::ProtectionPasswordBlank
            | Self::ProtectionPasswordWrong
            | Self::ProtectionPasswordNeedsDoc
            | Self::InvalidCustomPropType
            | Self::EmptyCustomPropName => InvalidField,
            Self::OutOfRange { .. } => TruncatedLimit,
            Self::InvalidRange { .. } => InvalidRange,
            Self::InvalidId { .. } => InvalidId,
            Self::SheetNotFound { .. }
            | Self::ParaNotFound { .. }
            | Self::SectionNotFound { .. }
            | Self::CustomShowNotFound { .. }
            | Self::SlideNotFound { .. }
            | Self::MasterNotFound { .. }
            | Self::ThemeNotFound { .. }
            | Self::DocumentThemeNotFound
            | Self::LayoutNotFound { .. }
            | Self::ShapeNotFound { .. }
            | Self::ConnectorNotFound { .. }
            | Self::AnimNotFound { .. }
            | Self::AnimHostNotFound { .. }
            | Self::ShapeHasNoText { .. }
            | Self::PictureNotFound { .. }
            | Self::MediaNotFound { .. }
            | Self::TextBoxNotFound { .. }
            | Self::NoteNotFound { .. }
            | Self::CommentNotFound { .. }
            | Self::ChartNotFound { .. }
            | Self::ChartSeriesNotFound { .. }
            | Self::TrendlineNotFound { .. }
            | Self::ErrorBarsNotFound { .. }
            | Self::SmartArtNotFound { .. }
            | Self::SmartArtPointNotFound { .. }
            | Self::OleNotFound { .. }
            | Self::InkNotFound { .. }
            | Self::Model3dNotFound { .. }
            | Self::MathDrawingNotFound { .. }
            | Self::SlicerNotFound { .. }
            | Self::ConnectionNotFound { .. }
            | Self::ExternalLinkNotFound { .. }
            | Self::CustomViewNotFound { .. }
            | Self::CellStyleNotFound { .. }
            | Self::ZoomNotFound { .. }
            | Self::WatermarkNotFound { .. }
            | Self::TableNotFound { .. }
            | Self::RevisionNotFound { .. }
            | Self::HyperlinkNotFound { .. }
            | Self::SparklineNotFound { .. }
            | Self::BookmarkNotFound { .. }
            | Self::EditRangeNotFound { .. }
            | Self::FieldNotFound { .. }
            | Self::TocNotFound { .. }
            | Self::CaptionNotFound { .. }
            | Self::CrossRefNotFound { .. }
            | Self::IndexNotFound { .. }
            | Self::IndexEntryNotFound { .. }
            | Self::SourceNotFound { .. }
            | Self::CitationNotFound { .. }
            | Self::BibliographyNotFound { .. }
            | Self::DocPartNotFound { .. }
            | Self::FontNotFound { .. }
            | Self::MathNotFound { .. }
            | Self::RubyNotFound { .. }
            | Self::ContentControlNotFound { .. }
            | Self::NumberingNotFound { .. }
            | Self::AbstractNumNotFound { .. }
            | Self::StyleNotFound { .. }
            | Self::RunNotFound { .. }
            | Self::CustomPropNotFound { .. }
            | Self::DataValidationNotFound { .. }
            | Self::ConditionalFormatNotFound { .. }
            | Self::SheetTableNotFound { .. }
            | Self::PivotTableNotFound { .. }
            | Self::DefinedNameNotFound { .. }
            | Self::ProtectedRangeNotFound { .. } => IdNotFound,
            Self::ScopeRequired { .. } => ScopeRequired,
            Self::SharedFormulaMaster { .. }
            | Self::DataTableFormula { .. }
            | Self::ArrayFormulaPart { .. }
            | Self::FormulaGroupOverlap { .. }
            | Self::InkComment { .. }
            | Self::SheetStillReferenced { .. }
            | Self::ExternalLinkInUse
            | Self::DialogOrMacroSheet
            | Self::CrossParaMarker
            | Self::ParaHasRevision
            | Self::SectionBreakPara
            | Self::RestrictedChart { .. }
            | Self::ChartTypeNotConvertible
            | Self::RegionMapChart
            | Self::SmartArtLayoutNotConvertible
            | Self::DeleteChartsheetLast
            | Self::ChartFormulaNotA1
            | Self::TableHasRevision
            | Self::TableGridMismatch
            | Self::MergeNotSimple
            | Self::SetParaTextHasRevision
            | Self::SetParaTextHasHyperlink
            | Self::SetParaTextHasField
            | Self::SetParaTextHasMath
            | Self::SetParaTextHasRuby
            | Self::SetParaTextHasSdt
            | Self::ContentControlBound
            | Self::ContentControlRepeating
            | Self::ContentControlGroup
            | Self::ContentControlHasTable
            | Self::ProtectionPassword
            | Self::ProtectionHasPassword => NotImplemented,
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
