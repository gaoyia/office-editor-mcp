# OPC 层

状态：已实现。零脏保存由库测试覆盖三种格式。Office 打开 create / apply 产物见三份清单。领域 JSON 没有「只保存不改」的动作，清单 O1 已放弃。

OPC 指 Open Packaging Conventions：文件是一个 ZIP，里面有部件、内容类型和关系。ECMA-376 §9 规定 Office 文档用 ZIP 作物理包，且不使用 interleaving。

## 打开

打开时把整个 ZIP 读入内存，为每个条目记住部件名（OPC 规则下的路径）、未压缩字节，以及它在原 ZIP 里的下标。
原始字节整份留着，保存时未改条目直接按下标把原压缩流搬过去，压缩方法、时间戳、extra 字段自然跟着一起走，不必逐项抄录。

部件名按 OPC 规则规范化：以 `/` 开头，正斜杠，没有 `.` 或 `..` 段。ZIP 里写成 `xl/workbook.xml` 的，对内统一成 `/xl/workbook.xml`。

必须存在：

- `/[Content_Types].xml`
- `/_rels/.rels`

缺任何一个就返回 `OpcOpen`。其他部件按关系去找，不靠猜固定路径。xlsx 的入口是根关系里 `Type` 以 `/officeDocument` 结尾的那条，目标通常是 `/xl/workbook.xml`。Office 实际文件也允许入口不在这个常见路径，所以必须跟关系走。

## 内容类型

`[Content_Types].xml` 里有 `Default`（按扩展名）和 `Override`（按部件路径）。读部件时用这两张表决定 Content Type。写新部件时必须补上对应条目。没改过的 `[Content_Types].xml` 按原字节写回。

## 关系

关系文件在源部件同级的 `_rels` 目录，文件名为 `源文件名.rels`。包根的关系文件是 `/_rels/.rels`。

读关系时保留每条 `Id`、`Type`、`Target`、可选的 `TargetMode`，以及不认识的属性。apply 若删除某个部件，必须同时删掉指向它的关系。create 只写自己生成的那几条关系，不伪造 Office 专有扩展关系。

## 读与改

按规范化路径取出未压缩字节。调用方（xlsx / docx 模块）改完后把新字节交还给 opc，并标记该路径为脏。同一路径多次标记，以最后一次字节为准。

禁止在 opc 层解析 SpreadsheetML 语义。opc 不认识单元格、段落、形状。

## 保存

写回规则：

1. 未标记为脏的条目：原压缩流整块搬过去，压缩方法、未压缩字节、extra、时间戳都与打开时一致，字节级相同。
2. 脏条目：用新字节重写。压缩方法第一期固定为 deflate，级别不要求与原文件相同。
3. 新建条目：追加到中央目录末尾。
4. 删除条目：从包中去掉，并同步内容类型 Override 与相关关系。
5. 未改条目在中央目录中的相对顺序保持打开时的顺序。新建条目排在它们后面。

不要对未改 XML 做重新缩进、重排属性、改 XML 声明、改换行符。Office 生成的 XML 常带特定声明和空元素写法，重排容易造成「没改的区域看起来不一样」。

## 未知部件与未知关系

ECMA-376 §9.1.4 与 §9.1.7：未知部件和未知关系在消费时不得导致失败。本库打开时保留它们，保存时若未标记删除就原样写回。这是「手术刀」策略在包这一层的体现。

## create 时的最小包（xlsx）

xlsx create 生成 Transitional 包，命名空间用 Office 默认的 2006 那套，不用 Strict 的 `purl.oclc.org`。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Workbook，以及可选的核心属性 |
| `/xl/workbook.xml` | 工作表名单 |
| `/xl/_rels/workbook.xml.rels` | 指向各 worksheet / chartsheet 和 styles |
| `/xl/worksheets/sheetN.xml` | 每张普通工作表 |
| `/xl/chartsheets/sheetN.xml` | 仅当某张表 `kind` 为 `chartsheet` |
| `/xl/styles.xml` | 最小样式表。默认字体颜色写 `<color rgb="FF000000"/>`，不写 `theme`，也不生成 theme 部件 |

inspect 认 workbook `.rels` 里 Type 以 `/theme` 结尾的部件（Excel 16.0 另存常见 `/xl/theme/theme1.xml`）。
也认 Type 以 `/themeOverride` 结尾的部件。create 给了 `workbook.theme` 时写 `/xl/theme/theme1.xml`；省略则不写。
apply 的 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` 只改已有 theme，不新建。
`setThemeOverride` 缺则新建 `/xl/theme/themeOverride1.xml`。

可选但 create 要写：

- **`/docProps/core.xml`**
  - 核心文件属性，dc:title 可空
- **`/docProps/app.xml`**
  - Application 写成 `ooxp`，Sheet 名单与 workbook 一致
  - 有图表工作表时 HeadingPairs 拆成 Worksheets 与 Charts
- **`/docProps/custom.xml`**
  - 仅当 `workbook.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性
- **`/xl/drawings/drawingN.xml`**
  - 仅当某张表声明了 ，或 apply 插入了这些对象
    - `pictures`
    - `charts`
    - `textBoxes`
    - `shapes`
    - `connectors`
    - `smartArts`
    - `kind: chartsheet`
    - `headerFooters[].pictures`
  - 页眉图走 `drawingHF`，与工作表 drawing 不是同一部件
- **`/xl/media/imageN.ext`**
  - 仅当某张表声明了 `pictures` 或 `headerFooters[].pictures`
- **`/xl/charts/chartN.xml`**
  - 仅当某张表声明了 `charts`，或声明了 `kind: chartsheet`
- **`/xl/charts/chartExN.xml`**
  - 仅当某张表声明了 `type=chartex` 的图表
- **`/xl/diagrams/…`**
  - 仅当某张表声明了 `smartArts`，或 apply 插入了 SmartArt

Workbook 的 Content Type（`.xlsx`）：

`application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml`

create 到 `.xlsm` 时改成：

`application/vnd.ms-excel.sheet.macroEnabled.main+xml`

Worksheet：

`application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml`

根关系 `officeDocument` 的 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument`

worksheet 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet`

chartsheet 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet`

dialogsheet 关系 Type（inspect 认，create 不写）：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet`

常见路径 `/xl/dialogsheets/sheetN.xml`。CT：

`application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml`

macrosheet 关系 Type（inspect 认，create 不写；国际宏表不另开 `kind`）：

`http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet`

`http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet`

常见路径 `/xl/macrosheets/…`。CT：

`application/vnd.ms-excel.macrosheet+xml`

`application/vnd.ms-excel.intlmacrosheet+xml`

Excel 16.0 另存 `.xlsx` 会丢掉宏表，对话框表能留下。本仓库可打开 `.xlsm`，不执行宏。
未点名的 `vbaProject.bin` 原样写回。create 到 `.xlsm` 时 workbook Override 写成
`application/vnd.ms-excel.sheet.macroEnabled.main+xml`，不建空的 `vbaProject.bin`。
apply 不得改掉已有的 macroEnabled Override。

SpreadsheetML 主命名空间：

`http://schemas.openxmlformats.org/spreadsheetml/2006/main`

apply 的 `setCellStyle` 可能改 `/xl/styles.xml`（复用或追加 `cellXfs` 的 `xf`）。
apply 的 `insertCellStyle` / `deleteCellStyle` / `setCellNamedStyle` 改同一份 styles：insert 复用或追加 `cellStyleXfs` 并挂 `cellStyles`；delete 只去掉点名的 `cellStyle`；套用复用或追加引用该 `xfId` 的 `cellXfs`。
apply 的 `setColWidth` / `setRowHeight` / `mergeRange` / `unmergeRange` 只改目标工作表 XML 的 `cols`、`row/@ht` 与 `mergeCells`，不新建部件。
apply 的 `groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline` 只改目标工作表 XML 的 `sheetPr/outlinePr`、
`sheetFormatPr`（同步 `@outlineLevelRow` / `@outlineLevelCol`，
新建或补上必填的 `@defaultRowHeight`）、`cols`（`@outlineLevel` / `@hidden` / `@collapsed`；
新建 `<col>` 且无 `@width` 时补缺省列宽）与 `row/@outlineLevel` / `@hidden` / `@collapsed`，不新建部件。
apply 的数据验证 op 只改该表 `dataValidations`，不新建部件。
apply 的条件格式 op 改该表 `conditionalFormatting`，并可能复用或追加 `/xl/styles.xml` 的 `dxfs`，不新建其它部件。
apply 的工作表表格 op 可能新建或删除 `/xl/tables/tableN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml`）。
工作表 `.rels` 补一条 table 关系（Type 以 `/table` 结尾），工作表 XML 写 `<tableParts>`。
apply 的透视表 op 可能新建或删除 `/xl/pivotTables/pivotTableN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.pivotTable+xml`）、
`/xl/pivotCache/pivotCacheDefinitionN.xml`（CT `…pivotCacheDefinition+xml`）与 `/xl/pivotCache/pivotCacheRecordsN.xml`（CT
`…pivotCacheRecords+xml`）。
workbook 写 `<pivotCaches>`，关系 Type 以 `/pivotCacheDefinition` 结尾；缓存定义再指向记录（Type 以 `/pivotCacheRecords` 结尾）；
工作表 `.rels` 补一条透视表关系（Type 以 `/pivotTable` 结尾）；
透视表部件再写一条指向该缓存定义的关系（Type 同样以 `/pivotCacheDefinition` 结尾，ECMA-376 §12.3.11 / §12.3.12）。
不把合计写进 `sheetData`。apply 的定义名 op 只改 `/xl/workbook.xml` 的 `<definedNames>`（ECMA-376 §18.2.5），
按 `CT_Workbook` 插在 `externalReferences` 之后、`calcPr` 之前，不新建部件。
apply 的 `setPrintSetup` 改目标工作表或图表工作表 XML 的 `pageMargins` / `pageSetup` / `printOptions` / `sheetPr/pageSetUpPr`
 / `rowBreaks` / `colBreaks`（§18.3.1.62、§18.3.1.63、§18.3.1.70、§18.3.1.3），不新建部件；打印区域仍写 workbook 的表级 `_xlnm.Print_Area`。
apply 的 `setSheetView` 改目标表 `sheetViews` 里第一个 `sheetView`（§18.3.1.87、§18.3.1.66），不新建部件。
拆分窗格写 `<pane>` 且不写 `@state`（规范缺省 `split`）；`xSplit` / `ySplit` 为 1/20 磅。
apply 的 `insertCustomView` / `deleteCustomView`
 / `showCustomView` 改 `/xl/workbook.xml` 的 `<customWorkbookViews>`（§18.2.3 / §18.2.4，
按 `CT_Workbook` 插在 `calcPr` 之后、`pivotCaches` 之前）以及各 `worksheet` / `chartsheet` 的 `<customSheetViews>`（§18.3.1.26），不新建部件。
apply 的 `setAutoFilter` 改工作表级 `<autoFilter>`（§18.3.1.2），不改 `/xl/tables/`，不新建部件。
apply 的 `setSortState` 改工作表级 `<sortState>`（§18.3.1.92），整段替换，不重排 `sheetData`，不新建部件。
apply 插入线程化批注时，可能新建 `/xl/threadedComments/threadedCommentN.xml`（CT `application/vnd.ms-excel.threadedcomments+xml`，
根 `ThreadedComments`，命名空间 `http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments`）、
`/xl/persons/person.xml`（CT `application/vnd.ms-excel.person+xml`，
根 `personList`）、占位 `/xl/commentsN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml`）以及
`/xl/drawings/vmlDrawingN.vml`。
工作表 `.rels` 的线程化关系 Type 必须是完整 URI `http://schemas.microsoft.com/office/2017/10/relationships/threadedComment`，不要用 `ends_with("/comments")`。
占位 comments 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments`。
VML 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing`，
`[Content_Types].xml` 补 Default `vml` 为 `application/vnd.openxmlformats-officedocument.vmlDrawing`。
workbook `.rels` 的 person 关系 Type 为 `http://schemas.microsoft.com/office/2017/10/relationships/person`。
工作表 XML 写 `<legacyDrawing r:id="…"/>`。同一张表多条批注共用一个 threadedComments 部件。`create` 给了 `sheets[].comments` 时按同一套规则写这些部件。
apply 插入工作表图时，可能新建 `/xl/drawings/drawingN.xml`（CT `application/vnd.openxmlformats-officedocument.drawing+xml`）、drawing 的 `.rels`，
以及 `/xl/media/imageN.*`。
工作表 `.rels` 补一条 drawing 关系（Type 以 `/drawing` 结尾），工作表 XML 写 `<drawing r:id="…"/>`。
往页眉槽位插图时写 `<drawingHF r:id="…"/>`（ECMA-376 §18.3.1.37），关系 Type 同样以 `/drawing` 结尾，drawing 内容类型相同；不改 `legacyDrawingHF`。
drawing 命名空间是 `http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing`（`xdr`）。
插入 SVG 时 drawing `.rels` 写两条 Type 以 `/image` 结尾的关系：一条指向 1×1 PNG 占位，一条指向 `/xl/media/imageN.svg`。
`[Content_Types].xml` 补 `Default Extension="svg"` 为 `image/svg+xml`。`a:blip/@r:embed` 指向 PNG，`asvg:svgBlip` 指向 SVG。
URI 为 `{96DAC541-7B7A-43D3-8B79-37D633B846F1}`，命名空间为 `http://schemas.microsoft.com/office/drawing/2016/SVG/main`。不栅格化。
插入 EMF / WMF / TIFF 时只写一条 image 关系，原字节写入 `/xl/media/imageN.emf` / `.wmf` / `.tif`。
`[Content_Types].xml` 补 Default：`emf` 为 `image/x-emf`，`wmf` 为 `image/x-wmf`，`tif` 为 `image/tiff`。
不要 PNG 占位，不要 `svgBlip`。
apply 或 create 插入工作表 3D 模型时，可能新建 `/xl/media/model3dN.glb`（Default `glb` → `model/gltf.binary`）以及海报 `/xl/media/imageN.png`。
关系写在 **drawing** `.rels`：一条 Type `http://schemas.microsoft.com/office/2017/06/relationships/model3d` 指向 GLB，一条 image 指向海报。
drawing 写 `mc:AlternateContent`，Choice `Requires="am3d"`，Choice 里是 `xdr:graphicFrame` 包 `am3d:model3d`，Fallback 是海报 `xdr:pic`。
`xdr:xfrm` 写 `0×0`。不写 `a16:creationId`。不把 GLB 当 `insertPicture`。create 给了 `sheets[].model3ds` 时同样写入。
apply 或 create 插入工作表公式框时，只改 worksheet drawing XML，不新建部件。
drawing 根补 `xmlns:a14` 与 `xmlns:m`。写 `mc:AlternateContent`，Choice `Requires="a14"`，Choice 里是 `xdr:sp`（`txBox="1"`）包 `a14:m` / `m:oMathPara`。
Fallback 是同一套纯文本框，字为调用方 `text`。不栅格化，不写海报。create 给了 `sheets[].maths` 时同样写入。
apply 插入工作表图表时，可能再新建 `/xl/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）及 chart 的 `.rels`；
drawing 关系 Type 以 `/chart` 结尾。
`type=chartex` 时写 `/xl/charts/chartExN.xml`（CT `application/vnd.ms-office.chartex+xml`），
关系 Type 为 `http://schemas.microsoft.com/office/2014/relationships/chartEx`。
同目录再写 `styleN.xml` / `colorsN.xml`（CT `application/vnd.ms-office.chartstyle+xml` /
`application/vnd.ms-office.chartcolorstyle+xml`），chartEx `.rels` 指向它们。
不写 embedding。apply 插入工作表超链接时，外部链在工作表 `.rels` 上补一条关系（Type
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink`，
`TargetMode` 为 `External`），工作表 XML 写 `<hyperlinks>/<hyperlink r:id="…"/>`（ECMA-376 §18.3.1.47 / §18.3.1.48）。
内部位置只写 `@location`，不写关系。删链或改成无 `r:id` 时，该关系无人引用则删除。不新建部件。
apply 插入工作表迷你图时，只改该表 XML 的 `extLst` / `x14:sparklineGroups`（MS-XLSX §2.4.8），不新建部件，不写关系。
apply 插入工作表 OLE 时，可能新建 `/xl/embeddings/…`（Office 文件 CT 与该格式主文档一致，
或 `oleObjectN.bin` 为 `application/vnd.openxmlformats-officedocument.oleObject`）以及一张预览 `/xl/media/imageN.png`。
工作表 `.rels` 补一条 package 或 oleObject 关系指向 embeddings，再补一条 image 关系指向预览。
工作表 XML 写 `<oleObjects>`（`mc:AlternateContent`，Choice 带 `objectPr`）。
隐藏 dummy `xdr:sp` 写在 worksheet drawing 里（`cNvPr/@id` 与 `shapeId` 相同）。VML Pict 占位写在 `legacyDrawing`，预览关系在 VML `.rels`。
`progId` 嵌入工作簿为 `Worksheet`。create 可写 OLE，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
apply 的 `setProtection` 改目标工作表或图表工作表 XML 的 `<sheetProtection>`（§18.3.1.85 / `CT_ChartsheetProtection`），
或 `/xl/workbook.xml` 的 `<workbookProtection>`（§18.2.29），按子元素顺序插入，不新建部件。
无密码时不写哈希。给了 `password` 则写 Excel 16.0 的 `algorithmName` / `hashValue` / `saltValue` / `spinCount`（工作簿为 `workbook*` 前缀），不写遗留 XOR `@password`。
apply 的允许编辑区域 op 改目标工作表 XML 的 `<protectedRanges>`（§18.3.1.71 / §18.3.1.72），
按 `CT_Worksheet` 插在 `sheetProtection` 之后，不新建部件。给了 `password` 则写与工作表保护相同的 SHA-512 属性。
apply 插入工作表文本框时，可能新建或复用 `/xl/drawings/drawingN.xml`；锚点写入 `xdr:sp`（`txBox`）。
apply 插入工作表 SmartArt 时，可能再新建 `/xl/diagrams/` 下 data / layout / colors
 / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、
`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`），
四条关系写在 **drawing** `.rels` 上（`diagramData` / `diagramLayout` / `diagramColors` / `diagramQuickStyle`），不要写到工作表 `.rels`。
改点时若对不上 diagram drawing 缓存，可能删除该缓存部件。`insertChartSheet` 再新建 `/xl/chartsheets/sheetN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml`），workbook 关系 Type 以 `/chartsheet` 结尾；
drawing 上写一张 `absoluteAnchor` 的 `xdr:graphicFrame`。
`insertSheet` 新建 `/xl/worksheets/sheetN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml`），workbook 关系 Type 以 `/worksheet` 结尾。
`setCellStyle` / `insertCellStyle` / `deleteCellStyle` / `setCellNamedStyle` 改 styles 部件（关系 Type 以 `/styles` 结尾，常见 `/xl/styles.xml`）；没有该部件则按 create 的最小样式表新建，并补 Override。不重排未点名的 `xf`。
create 声明了 `sheets[].pictures` 或 `sheets[].charts` 时按同一套规则写 drawing、media 与 chart 部件。
create 声明了 `sheets[].textBoxes` / `shapes` / `connectors`
 / `smartArts` 时按对应 insert 写同一份 worksheet drawing 与 `/xl/diagrams/`。
create 声明了 `sheets[].comments` 时按 `insertComment` 写线程化批注、占位 comments 与 VML。
create 声明了 `sheets[].hyperlinks` 时按 `insertHyperlink` 写工作表 `<hyperlinks>` 与外部链关系。
create 声明了 `sheets[].headerFooters` 时按 `setHeaderFooterText` / 页眉 `insertPicture` 写工作表 `headerFooter` 与可选 `drawingHF`，不写 `legacyDrawingHF`。
create 声明了 `kind: chartsheet` 时按 `insertChartSheet` 写 `/xl/chartsheets/sheetN.xml`、drawing 与 chart 部件。
create 声明了第四轮字段时按对应 apply 写，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
`setProps` / `setCustomProp` 可能新建 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`（三种格式相同）。
core 关系 Type 为 `http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties`，
CT `application/vnd.openxmlformats-package.core-properties+xml`。
app 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties`，
CT `application/vnd.openxmlformats-officedocument.extended-properties+xml`。
custom 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties`，
CT `application/vnd.openxmlformats-officedocument.custom-properties+xml`。
`deleteCustomProp` 删光后去掉 custom 部件、根关系和 Override。定位这些部件用根 `.rels` 的 Type 后缀，不要写死路径。
 apply 的切片器 op 可能新建或删除 `/xl/slicers/slicerN.xml`（CT `application/vnd.ms-excel.slicer+xml`）与
`/xl/slicerCaches/slicerCacheN.xml`（CT `application/vnd.ms-excel.slicerCache+xml`）。
 工作表 `.rels` 补 slicer 关系，workbook `.rels` 补 slicerCache 关系。
 还可在 workbook `<definedNames>` 写一条公式为 `#N/A` 的定义名（MS-XLSX 2.3.2.3）。
 透视源还在 `/xl/pivotCache/pivotCacheDefinitionN.xml` 的 `extLst` 写 `x14:pivotCacheDefinition/@pivotCacheId`（URI
`{725AE2AE-9491-48BE-B2B4-4EB974FC3084}`），与 `tabular/@pivotCacheId` 相同。
 drawing 宿主是 `mc:AlternateContent` 里的 slicer `graphicFrame`。create 可写切片器，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
apply 的外部数据连接 op 可能新建或删除 `/xl/connections.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml`，ECMA-376 §12.3.4 / §18.13）。
workbook `.rels` 补一条 connections 关系（Type
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/connections`，Target `connections.xml`）。
不写 `xr16:uid`，不写 `refreshOnLoad`。删光则去掉该部件、workbook 关系与 Override。不强制写 queryTable。create 可写连接，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
apply 的外部引用 op 可能新建或删除 `/xl/externalLinks/externalLinkN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml`，ECMA-376 §12.3.7 / §18.14）。
workbook `.rels` 补一条 externalLink 关系（Type
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink`）。
该部件 `.rels` 一条 externalLinkPath（Type `http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLinkPath`，
`TargetMode` 为 `External`）。
workbook.xml 写 `<externalReferences>`。不写 `xxl21` / 缓存值。删光则去掉 `<externalReferences>`、部件、关系与 Override。
`setExternalLink` 只改该链 `.rels` 的 `Target`，不新建部件，已有 `xxl21:alternateUrls` 原样留下。create 可写外部引用，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。

## create 时的最小包（docx）

docx create 同样生成 Transitional 包，命名空间用 2006 那套。

必有部件：

- **`/[Content_Types].xml`**
  - 内容类型
- **`/_rels/.rels`**
  - 指向 Main Document，以及可选的核心属性
- **`/word/document.xml`**
  - 主文档，至少一个 `w:p`
- **`/word/_rels/document.xml.rels`**
  - 指向 styles 与 settings
- **`/word/styles.xml`**
  - 至少含 Normal、Heading1、Heading2、Heading3
- **`/word/settings.xml`**
  - 最小文档设置
  - 必须含 `compatibilityMode`，`val` 为 `15`（MS-DOCX §2.3.5），否则 Word 按兼容模式打开（默认 12）
  - 给了 `document.protection` 或 apply `setProtection` 时还可含 `documentProtection` / `writeProtection`
  - `setProtection` 带 `password` 时 `documentProtection` 另写 `crypt*` 与 `hash` / `salt`
  - `setEastAsianSettings` 还可含 `characterSpacingControl` / `strictFirstAndLastChars` / `noLineBreaksBefore` / `noLineBreaksAfter` / `themeFontLang`

inspect 认主文档 `.rels` 里 Type 以 `/theme` 结尾的部件（Word 16.0 另存常见 `/word/theme/theme1.xml`）。
也认 Type 以 `/themeOverride` 结尾的部件。create 给了 `document.theme` 时写 theme；省略则不写。
apply 的 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` 只改已有 theme，不新建。
`setThemeOverride` 缺则新建 `/word/theme/themeOverride1.xml`。

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp` |
| `/docProps/custom.xml` | 仅当 `document.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性 |
| `/word/header1.xml` | 仅当 `document.headers` 含默认页眉 |
| `/word/footer1.xml` | 仅当 `document.footers` 含默认页脚 |
| `/word/media/imageN.ext` | 仅当段落或页眉页脚声明了 `pictures`；SVG 再写一份 `imageN.svg` |
| `/word/diagrams/…` | 仅当段落或页眉页脚声明了 `smartArts` |

apply 或 create 插入 SVG 时：

- 宿主 `.rels` 写两条 Type 以 `/image` 结尾的关系
- 一条指向 1×1 PNG 占位，一条指向 `/word/media/imageN.svg`
- `[Content_Types].xml` 补 `Default Extension="svg"` 为 `image/svg+xml`
- `a:blip/@r:embed` 指向 PNG
- `asvg:svgBlip` 指向 SVG
- URI 为 `{96DAC541-7B7A-43D3-8B79-37D633B846F1}`
- 命名空间为 `http://schemas.microsoft.com/office/drawing/2016/SVG/main`
- 不栅格化

插入 EMF / WMF / TIFF 时只写一条 image 关系，原字节写入 `/word/media/imageN.emf` / `.wmf` / `.tif`。
`[Content_Types].xml` 补 Default：`emf` 为 `image/x-emf`，`wmf` 为 `image/x-wmf`，`tif` 为 `image/tiff`。
不要 PNG 占位，不要 `svgBlip`。
apply 或 create 插入 3D 模型时，可能新建 `/word/media/model3dN.glb`（Default `glb` → `model/gltf.binary`）以及海报 `/word/media/imageN.png`。
宿主 `.rels` 补一条 Type `http://schemas.microsoft.com/office/2017/06/relationships/model3d` 指向 GLB，再补一条 image 指向海报。
宿主写 `mc:AlternateContent`，Choice `Requires="am3d"`，Choice 里是 `w:drawing` / `wp:inline` 包 `am3d:model3d`，Fallback 是海报 `pic`。
create 给了 `paragraphs[].model3ds` 时同样写入。不把 GLB 当 `insertPicture`。

apply 插入脚注或尾注时，可能新建 `/word/footnotes.xml` 或 `/word/endnotes.xml`（CT 分别为 `…footnotes+xml` / `…endnotes+xml`），
并在主文档 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。
apply 插入批注时，可能新建 `/word/comments.xml`（CT `…comments+xml`）、
`/word/commentsExtended.xml`（CT `application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml`）、
`/word/people.xml`（CT `application/vnd.openxmlformats-officedocument.wordprocessingml.people+xml`）。
`commentsExtended` 必须用这条 CT：Word 16.0 把旧的 `application/vnd.ms-word.commentsExtended+xml` 当成损坏，打开失败。
`commentsIds` / `commentsExtensible` 只在原包已有时补条目，create 与默认插入都不新建这两份。`replyComment` 还会在父批注范围上再套一套范围标记，因此可能脏主文档或页眉页脚、注部件。
apply 插入图表时，可能新建 `/word/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）、chart 的 `.rels`，
以及 `/word/embeddings/….xlsx`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`），
并在宿主 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。
`type=chartex` 时写 `/word/charts/chartExN.xml`（CT `application/vnd.ms-office.chartex+xml`），
宿主关系 Type 为 `http://schemas.microsoft.com/office/2014/relationships/chartEx`。
chartEx `.rels` 除 embedding 外再指向同目录 `styleN.xml` / `colorsN.xml`
（CT `application/vnd.ms-office.chartstyle+xml` / `application/vnd.ms-office.chartcolorstyle+xml`）。
apply 或 create 插入 SmartArt 时，可能新建 `/word/diagrams/` 下的 data / layout / colors
 / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、
`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`），
四条关系写在宿主 `.rels` 上（`diagramData` / `diagramLayout` / `diagramColors` / `diagramQuickStyle`）。
改点时若对不上 diagram drawing 缓存，可能删除该缓存部件。apply 或 create 插入 OLE 时，可能新建 `/word/embeddings/…`（Office 文件用对应格式的 Content Type；
Package 为 `application/vnd.openxmlformats-officedocument.oleObject`）以及占位预览图 `/word/media/imageN.png`。
Office 文件的关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`（与 Word 16.0 另存一致）；
Package 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject`。
apply 或 create 插入墨迹画布时，可能新建 `/word/ink/inkN.xml`（CT `application/inkml+xml`）。
宿主 `.rels` 补一条 `customXml` 关系（`http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml`），
`[Content_Types].xml` 补 Override。不写独立 `wordprocessingInk` 部件。
inspect 两种关系都认。create 给了 `footnote` / `endnote` / `comments` / `charts` / `oleObjects` / `inkCanvases` / `model3ds` 时，按对应 insert 同一套规则新建这些部件；也可写 SmartArt 四件套。
create 给了 `comments[].replies` 时按 `replyComment` 再写回复。
create 给了 `document.numberings` 时按 `insertNumbering` 新建 `/word/numbering.xml`（CT `…numbering+xml`）。
create 给了 `hyperlinks` 时按 `insertHyperlink` 在宿主 `.rels` 上补 External hyperlink 关系。
apply 给正文或页眉页脚里的图 / 文本框写 DrawingML 链时，同样在该故事部件 `.rels` 补 Type 以 `/hyperlink` 结尾的关系。
外部链 `TargetMode` 为 `External`。内部书签 `Target` 为 `#{name}`，不写 `TargetMode`。不新建部件。
删 DrawingML 链只摘 `a:hlinkClick`，孤儿关系删掉，不删图或框。
create 给了 `document.tocs` / `fields` / `maths` / `contentControls` / `bookmarks` 时只改宿主 XML（目录块会新插 `w:p`），不另建部件。
apply 的 `insertField` 给窗体 `code` 时在 begin 里写 `w:ffData`，仍只改宿主 XML，不新建部件。
apply 的 `insertMath` / `setMath` 无论 `text` 还是 `expr` 都只改宿主 XML 里的 `m:oMath`，不新建部件。
create 或 apply 给了水印时，可能新建 `/word/headerN.xml`，并把 DrawingML 写入该页眉；图片水印还可新建 `/word/media/…`。
apply 插入嵌入字体时，可能新建 `/word/fontTable.xml`（CT
`application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml`）、
`/word/_rels/fontTable.xml.rels`、`/word/fonts/fontN.odttf`（Default `odttf` →
`application/vnd.openxmlformats-officedocument.obfuscatedFont`）。
主文档关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable`。
fontTable 上每条嵌入字体关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/font`，Target 常见 `fonts/fontN.odttf`。
删掉某嵌入面时去掉对应 odttf 与 font 关系；fontTable 本身不因删光而拆掉。create 本期不写嵌入字体。
apply 写页面背景时只改主文档 `w:background` 与 `settings.xml` 的 `w:displayBackgroundShape`，
不新建 footnotes / theme / webSettings / fontTable。
apply 写页面边框时只改本节 `w:pgBorders`。apply 接受或拒绝修订只改已有主文档或页眉页脚 XML，不新建部件。
apply 插入构建基块时，可能新建 `/word/glossary/document.xml`（CT
`application/vnd.openxmlformats-officedocument.wordprocessingml.document.glossary+xml`）、
卫星 `styles.xml` / `settings.xml` / `webSettings.xml` / `fontTable.xml`（CT 与主文档同类部件相同）以及 glossary `.rels`。
主文档关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/glossaryDocument`，Target 常见 `glossary/document.xml`。
删光最后一条 `w:docPart` 时去掉这些部件、关系与 Override。create 本期不写 glossary。

Main Document 的 Content Type：

`application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml`

Style Definitions：

`application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml`

Document Settings：

`application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml`

Theme（inspect 认；create 给了 `document.theme` 时写）：

`application/vnd.openxmlformats-officedocument.theme+xml`

Theme Override：

`application/vnd.openxmlformats-officedocument.themeOverride+xml`

settings 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings`

theme 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme`

themeOverride 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/themeOverride`

`compatibilityMode` 的 `w:uri` 必须是 `http://schemas.microsoft.com/office/word`。

WordprocessingML 主命名空间：

`http://schemas.openxmlformats.org/wordprocessingml/2006/main`

`w14` 命名空间（paraId 与内容控件复选框）：

`http://schemas.microsoft.com/office/word/2010/wordml`

根元素须把 `w14` 列入 `mc:Ignorable`，依据 MS-DOCX §2.2.4。

## create 时的最小包（pptx）

pptx create 同样生成 Transitional 包，命名空间用 2006 那套。
不能只写 ECMA-376 §13.3.6 示例里那个仅含 `sldSz` / `notesSz` 的 presentation：PowerPoint 实际打开还要 theme、slideMaster、slideLayout。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Presentation，以及可选的核心属性 |
| `/ppt/presentation.xml` | 页名单、母版关系、幻灯片尺寸；可选 `p:modifyVerifier` |
| `/ppt/_rels/presentation.xml.rels` | 指向 slideMaster、各 slide、presProps |
| `/ppt/presProps.xml` | Presentation Properties（§13.3.7，包内应当恰好一份）。缺省只有 `clrMru`；给了 `showPr` 或 `setShowPr` 写 `p:showPr` |
| `/ppt/slides/slideN.xml` | 每一页 |
| `/ppt/slides/_rels/slideN.xml.rels` | 指向 slideLayout1；有图时再加 image 关系 |
| `/ppt/slideLayouts/slideLayout1.xml` | 一张空白版式 |
| `/ppt/slideLayouts/_rels/slideLayout1.xml.rels` | 指向 slideMaster1 |
| `/ppt/slideMasters/slideMaster1.xml` | 一张母版 |
| `/ppt/slideMasters/_rels/slideMaster1.xml.rels` | 指向 theme1 与 slideLayout1 |
| `/ppt/theme/theme1.xml` | 最小主题 |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp`，Slides 与页数一致 |
| `/docProps/custom.xml` | 仅当 `presentation.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性 |
| `/ppt/media/imageN.*` | 给了 `slides[].pictures` 时写入 |

create 不写 `viewProps`、`tableStyles`。
给了 `slides[].anims` 或 `notes.anims` 时写 `<p:timing>`。
给了 `slides[].oleObjects` 时写 embedding 与 `p:oleObj`。
给了 `slides[].zooms` 时写 Zoom。
给了 `presentation.handout` 时与 apply 的 `setHandoutText` 相同，可能新建 `/ppt/handoutMasters/handoutMaster1.xml`。
给了 `slides[].notes` 时与 apply 的 `setNotesText` 相同，
可能新建 `/ppt/notesMasters/notesMaster1.xml`、独立 theme 副本（常见为 `/ppt/theme/theme2.xml`）与 `/ppt/notesSlides/notesSlideN.xml`。
给了 `slides[].comments` 时与 apply 的 `insertComment` 相同，
可能新建 `/ppt/comments/modernComment_{sldId}_N.xml` 与 `/ppt/authors.xml`。
apply 的 `setHandoutText` 也可能新建讲义母版。

`presentation.xml` 里：`p:sldMasterIdLst` 一条、`p:sldIdLst` 按页顺序、`p:sldSz` 为 16:9（`cx="12192000"` `cy="6858000"`）、
`p:notesSz` 为 `cx="6858000"` `cy="9144000"`。
`p:sldId/@id` 从 256 起递增，与领域 id 的 `slide:{n}` 不是一回事。

Presentation 的 Content Type：

`application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml`

Slide：

`application/vnd.openxmlformats-officedocument.presentationml.slide+xml`

Slide Master：

`application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml`

Slide Layout：

`application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml`

Presentation Properties：

`application/vnd.openxmlformats-officedocument.presentationml.presProps+xml`

Theme：

`application/vnd.openxmlformats-officedocument.theme+xml`

根关系 `officeDocument` 的 Type 与 xlsx / docx 相同：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument`

幻灯片关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide`

幻灯片母版：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster`

版式：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout`

主题：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme`

Presentation Properties：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps`

PresentationML 主命名空间：

`http://schemas.openxmlformats.org/presentationml/2006/main`

DrawingML 主命名空间：

`http://schemas.openxmlformats.org/drawingml/2006/main`

apply 插入幻灯片图时，可能新建 `/ppt/media/imageN.*`，并在该页 `.rels` 补一条 image 关系（Type 以 `/image` 结尾）。
`p:pic` 写在该页 `p:spTree` 里，位置与尺寸在 `p:spPr/a:xfrm`。create 给了 `slides[].pictures` 时同样写入。
插入 SVG 时该页 `.rels` 写两条 image 关系：一条指向 1×1 PNG 占位，一条指向 `/ppt/media/imageN.svg`。
`[Content_Types].xml` 补 `Default Extension="svg"` 为 `image/svg+xml`。`a:blip/@r:embed` 指向 PNG，`asvg:svgBlip` 指向 SVG。
URI 与命名空间与 Word / Excel 相同。不栅格化。PowerPoint 另存可以没有 PNG 后备，库写入仍带占位 PNG。
插入 EMF / WMF / TIFF 时只写一条 image 关系，原字节写入 `/ppt/media/imageN.emf` / `.wmf` / `.tif`。
`[Content_Types].xml` 补 Default：`emf` 为 `image/x-emf`，`wmf` 为 `image/x-wmf`，`tif` 为 `image/tiff`。
不要 PNG 占位，不要 `svgBlip`。
apply 或 create 插入幻灯片或备注 3D 模型时，可能新建 `/ppt/media/model3dN.glb`（Default `glb` → `model/gltf.binary`）以及海报 `/ppt/media/imageN.png`。
该页或 notesSlide `.rels` 补一条 Type `http://schemas.microsoft.com/office/2017/06/relationships/model3d` 指向 GLB，再补一条 image 指向海报。
宿主是 `p:graphicFrame`，包在 `mc:AlternateContent` 的 Choice 里（`Requires="am3d"`）。Fallback 是海报 `p:pic`。
create 给了 `slides[].model3ds` 时同样写入。备注 apply 可往 `slide:{n}/notes` 插；create 本期不在 `slides[].notes` 上写 3D。不把 GLB 当 `insertPicture`。
apply 或 create 插入幻灯片或备注公式框时，只改该页或 notesSlide XML，不新建部件。
宿主根补 `xmlns:a14` 与 `xmlns:m`。写 `mc:AlternateContent`，Choice `Requires="a14"`，Choice 里是 `p:sp`（`txBox="1"`）包 `a14:m` / `m:oMathPara`。
Fallback 是同一套纯文本框，字为调用方 `text`。不栅格化，不写海报。create 给了 `slides[].maths` 时同样写入。
备注 apply 可往 `slide:{n}/notes` 插；create 本期不在 `slides[].notes` 上写公式。

apply 插入幻灯片视频或音频时，可能新建 `/ppt/media/mediaN.mp4` 或 `/ppt/media/mediaN.wav`，以及一张海报 `/ppt/media/imageN.png`。
该页 `.rels` 补三条关系：MS-PPTX media（Type `http://schemas.microsoft.com/office/2007/relationships/media`）、
ISO video 或 audio（Type 分别以 `/video`、`/audio` 结尾），两者指向同一媒体部件；再补一条 image 关系指向海报。
`p:pic` 的 `p:nvPr` 写 `a:videoFile` / `a:audioFile` 的 `r:link` 与 `p14:media` 的 `r:embed`。
`[Content_Types].xml` 补 Default：`mp4` 为 `video/mp4`，`wav` 为 `audio/x-wav`。不写 `<p:timing>`。
create 给了 `slides[].videos` / `slides[].audios` 时同样写入。

apply 插入幻灯片图表时，可能新建 `/ppt/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）、chart 的 `.rels`，
以及 `/ppt/embeddings/Microsoft_Excel_WorksheetN.xlsx`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`）。
该页 `.rels` 补一条 chart 关系（Type 以 `/chart` 结尾）。
`type=chartex` 时写 `/ppt/charts/chartExN.xml`（CT `application/vnd.ms-office.chartex+xml`），
关系 Type 为 `http://schemas.microsoft.com/office/2014/relationships/chartEx`。
chartEx `.rels` 除 embedding 外再指向同目录 `styleN.xml` / `colorsN.xml`
（CT `application/vnd.ms-office.chartstyle+xml` / `application/vnd.ms-office.chartcolorstyle+xml`）。
chart 部件写 `c:externalData r:id="rId1"`，关系 Type 为
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`。
宿主是 `p:graphicFrame`，位置与尺寸在 `p:xfrm`。create 给了 `slides[].charts` 时同样写入。

apply 插入幻灯片 SmartArt 时，可能新建 `/ppt/diagrams/` 下的 data / layout / colors
 / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、
`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`）。
该页 `.rels` 补四条 diagram 关系。宿主是 `p:graphicFrame`，`a:graphicData/@uri` 为 diagram URI，`dgm:relIds` 指向这四条。
create 给了 `slides[].smartArts` 时同样写入。不写 Fallback 位图，不写 diagram drawing 缓存。改点对不上缓存时删掉该缓存。

apply 新建备注页时，可能写入 `/ppt/notesSlides/notesSlideN.xml`（CT
`application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml`）和 `/ppt/notesMasters/notesMaster1.xml`（CT `…notesMaster+xml`）。
幻灯片 `.rels` 补一条 notesSlide 关系（Type 以 `/notesSlide` 结尾）。
备注页 `.rels` 指向 notesMaster 与对应幻灯片（Type 分别以 `/notesMaster`、`/slide` 结尾）。
若 presentation 还没有 notesMaster，补 presentation 关系，并在 `presentation.xml` 的 `p:sldMasterIdLst` 之后写入 `p:notesMasterIdLst`。
notesMaster 的 theme 关系指向新复制的 theme 部件（内容与包内已有 theme 相同，常见为 `/ppt/theme/theme2.xml`），不和 slideMaster 共用同一部件。
备注页结构对照 PowerPoint 16.0 另存样例：`sldImg`、`body`、`sldNum` 三个占位符。
往备注插的 `p:pic`、图表 `p:graphicFrame`、SmartArt `p:graphicFrame`、表 `p:graphicFrame` 与 3D 模型 `p:graphicFrame` 写在 notesSlide 的 `p:spTree` 里；
媒体仍在 `/ppt/media/…`，chart / embedding / diagram 部件仍在 `/ppt/charts/…`、`/ppt/embeddings/…`、`/ppt/diagrams/…`，关系写在 notesSlide `.rels`。表不新建部件。

apply 的 `setHandoutText` 可能写入 `/ppt/handoutMasters/handoutMaster1.xml`（CT
`application/vnd.openxmlformats-officedocument.presentationml.handoutMaster+xml`）。
presentation `.rels` 补一条 handoutMaster 关系（Type 以 `/handoutMaster` 结尾）。
若还没有 `p:handoutMasterIdLst`，写在 `notesMasterIdLst` 之后（没有则写在 `sldMasterIdLst` 之后）。
handoutMaster 的 theme 关系指向新复制的 theme 部件（内容与包内已有 theme 相同，常见为 `/ppt/theme/theme2.xml`），不和 slideMaster 共用同一部件。
新建部件含 `hdr`、`dt`、`ftr`、`sldNum` 四个占位符，不写 `sldImg`。

apply 插入现代批注时，可能写入 `/ppt/comments/modernComment_{sldId}_N.xml`（CT `application/vnd.ms-powerpoint.comments+xml`，
根 `p188:cmLst`，命名空间 `http://schemas.microsoft.com/office/powerpoint/2018/8/main`）和 `/ppt/authors.xml`（CT
`application/vnd.ms-powerpoint.authors+xml`，根 `p188:authorLst`）。
幻灯片 `.rels` 补一条 comments 关系，Type 必须是完整 URI `http://schemas.microsoft.com/office/2018/10/relationships/comments`，
不要用 `ends_with("/comments")`（会误伤 2006 旧式）。
presentation `.rels` 补一条 authors 关系，Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/authors`。
幻灯片 XML 在 `p:clrMapOvr` 之后写入 `p:extLst` / `p:ext`（uri `{6950BFC3-D8DA-4A85-94F7-54DA5524770B}`）/ `p188:commentRel`。
同一页多条批注共用一个 comments 部件。`p188:cm` 子元素顺序为锚点（`pc:sldMkLst`）、`p188:pos`、可选 `p188:replyLst`、`p188:txBody`。
`pc:sldMk/@sldId` 取 presentation 该页 `p:sldId` 的无前缀 `id`。
不写 ECMA-376 §19.4 的 `/ppt/comments/commentN.xml` 与 `commentAuthors`。
apply 往批注或回复插图时，媒体写入 `/ppt/media/imageN.*`，comments 部件 `.rels` 补一条 image 关系（Type 以 `/image` 结尾，
`Target` 形如 `../media/imageN.png`），并在该 `p188:cm` 或 `p188:reply` 的 `txBody` 之后、`extLst` 之前写入 `a:blip r:embed="…"`。
SVG 时再补一条 image 关系指向 `../media/imageN.svg`，并在该 `a:blip` 上写 `asvg:svgBlip`。
不写 `p:pic`。换图只改该关系 `Target` 与新媒体；换成 SVG 时保留 `a:blip` 后备。删图摘掉该 `a:blip`，关系与媒体无人引用则删。

apply 的超链接 op 只改该页幻灯片 XML 的 `p:cNvPr/a:hlinkClick` 与该页 `.rels`。
URL / 文件写 Type 以 `/hyperlink` 结尾、`TargetMode` 为 `External` 的关系；跳转到某页写 Type 以 `/slide` 结尾的内部关系。不新建幻灯片部件。删链只摘 `hlinkClick`，孤儿关系删掉。

apply 的 `setShowPr` 只改 `/ppt/presProps.xml` 的 `p:showPr`，插在 `prnPr` 之后、`clrMru` 之前。不新建 presProps 部件。
没有该部件时报 `OpcPartMissing`。不写 `penClr` 与激光笔扩展。改成 `browse` 时写入 `p14:browseMode`（MS-PPTX §2.3.1.2）。

apply 的 `setProtection` 只改 `presentation.xml` 的 `p:modifyVerifier`（`defaultTextStyle` 之后、`extLst` 之前）。
无密码时写空元素或删掉元素。给了 `password` 则写 PowerPoint 16.0 的 `crypt*` / `hashData` / `saltData` / `spinCount`。
已有可校验的 SHA-512 哈希时必须给对得上的口令。未知哈希不覆盖。

apply 插入幻灯片 OLE 时，可能新建 `/ppt/embeddings/Microsoft_Excel_WorksheetN.xlsx`（或其它 Office 扩展名，
CT 与该格式主文档一致）或 `/ppt/embeddings/oleObjectN.bin`（CT `application/vnd.openxmlformats-officedocument.oleObject`），以及一张预览 `/ppt/media/imageN.png`。
该页 `.rels` 补一条 package 或 oleObject 关系指向 embeddings，再补一条 image 关系指向预览。
宿主是 `p:graphicFrame`，`a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/presentationml/2006/ole`。
`mc:AlternateContent` 的 Choice 只有 `p:oleObj`，Fallback 带 `p:pic` 预览。位置与尺寸在 `p:xfrm`。不写 `<p:timing>`。
create 给了 `slides[].oleObjects` 时复用本写入。

apply 插入幻灯片 Zoom 时，可能新建一张预览 `/ppt/media/imageN.png`。该页 `.rels` 补一条 image 关系指向预览，再补一条 slide 关系指向目标页（节缩放则指向该节第一页；摘要缩放指向 `targets` 各节第一页）。
宿主是 `p:graphicFrame`，包在 `mc:AlternateContent` 的 Choice 里；
`a:graphicData/@uri` 为 `http://schemas.microsoft.com/office/powerpoint/2016/slidezoom`、`…/sectionzoom` 或 `…/summaryzoom`。
Fallback 是带 `ppaction://hlinksldjump` 的 `p:pic`。位置与尺寸在 graphicFrame 的 `p:xfrm`。不写 `<p:timing>`。
create 给了 `slides[].zooms` 时复用本写入。

## 验收（第 2 项）

零脏保存：`open` 之后不调用 `set` / `remove`，再 `save`。每个部件的未压缩字节、以及 ZIP 里该条目的 CRC，都与打开时相同。
库测试对 Excel 另存的 xlsx、以及 create 写出的 docx / pptx 各跑一遍。

领域 JSON 的三个动作不含原样保存。`apply` 的 `ops` 必须非空，见 [json-contract.md](json-contract.md)。命令行不能做「打开再原样写出」。这一条第一期放弃，原因写在三份清单的 O1。

Office 打开 create / apply 产物、以及 Office 自己另存 create 产物，
见 `docs/checklist-xlsx.md`、`docs/checklist-docx.md`、
`docs/checklist-pptx.md` 与 `manual-check/office/cases/opc-resave.ps1`。那是打开冒烟，不是零脏保存。
