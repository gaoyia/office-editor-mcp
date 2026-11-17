# xlsx 编辑路线

只记账顺序，不验收。Excel 全量指包里能表达的对象都能进 inspect、能 apply、create 能按同一套 JSON 生成。不是复刻 Excel 运行时。公式现按字符串读写。求值对照见已过 50。

docx 见 [`roadmap/docx.md`](docx.md)。pptx 见 [`roadmap/pptx.md`](pptx.md)。`ooxp mcp` 是命令行入口，不在本目录开切片。领域 JSON 原样交给库。

每份计划仍是：先改当期 `docs/`，确认后再动代码。挂机一次只开一份，从待做序号 0 起。当期切片写在 `.plan/`，不写进本目录。

## 永不做 🚫

- 不把出图的像素写回包，也不塞进 inspect。只读 PNG 见待做 82、83、84。金样见已过 C23
- 按列宽行高去反推 `twoCell` 的显示尺寸
- 做一套 VML 绘图引擎。未点名的 VML / `legacyDrawing` 原样写回。真源在 VML 的对象见已过 51
- 默认把图片字节或 chart 部件塞进 inspect 树
- xlsx 上的 `exportChartData`（数据在本工作簿单元格里）
- SmartArt 版式算法
- 执行宏、打印本身
- `.xls`

`vbaProject.bin` 未点名则原样写回。

## 不单开

- 工作表页眉页脚里的图：跟页眉页脚计划走。只有 `legacyDrawingHF` 的旧式页眉图见已过 65，不另造对象
- 图表工作表上那张 `xdr:graphicFrame`：id 仍是 `sheet:{name}/chart:{cNvPrId}`
- 组合 `xdr:grpSp` 里的图：不够用时补进 `.plan/2026年-9月-27日-✅-xlsx-工作表图.md`
- `exportMedia`：跟图走
- 复制已有文件再 apply：不必新动作
- 功能区艺术字：另存是无 `txBox` 的 `xdr:sp`。按现有规则不进 `textBoxes`，不扩大收集，不单开对象。字填充 / 轮廓见已过 C10

## 已过

| 序 | 计划 | 前置 |
| --- | --- | --- |
| 0 | `.plan/2026年-9月-27日-✅-xlsx-工作表图.md` | 无 |
| 1 | `.plan/2026年-9月-27日-✅-xlsx-工作表图表.md` | 工作表图；不要跟 `xdr:pic` 绑 |
| 2 | `.plan/2026年-9月-27日-✅-xlsx-create带图.md` | 工作表图 |
| 3 | `.plan/2026年-9月-27日-✅-xlsx-create带图表.md` | 工作表图表；create 带图 |
| 4 | `.plan/2026年-9月-27日-✅-xlsx-页眉页脚.md` | 无 |
| 5 | `.plan/2026年-9月-27日-✅-xlsx-图表工作表.md` | 工作表图表 |
| C1 | `.plan/2026年-9月-27日-✅-图表剩余.md` | 工作表图表已过 |
| C4 | `.plan/2026年-9月-29日-✅-图表稀有类型.md` | 图表剩余已过 |
| C5 | `.plan/2026年-10月-9日-✅-xlsx-pptx-嵌入SVG.md` | 工作表图；docx 44 已过 |
| C6 | `.plan/2026年-10月-9日-✅-三家图嗅探补EMF-WMF-TIFF.md` | 工作表图；C5 已过 |
| C7 | `.plan/2026年-10月-9日-✅-三家3D模型.md` | 工作表图 |
| C8 | `.plan/2026年-10月-9日-✅-xlsx-pptx-嵌入OMML.md` | 工作表图 / 文本框；不要改 Word 公式 |
| C9 | `.plan/2026年-10月-9日-✅-pptx-xlsx-形状文字格式.md` | 工作表文本框 / 形状；pptx 形状增删；不要改 Word `setRunFormat` |
| C10 | `.plan/2026年-10月-10日-✅-pptx-xlsx-字填充与轮廓.md` | 形状文字格式 C9；不要改 Word |
| C11 | `.plan/2026年-10月-10日-✅-图表稀有剩余.md` | 图表稀有类型 C4 |
| C12 | `.plan/2026年-10月-11日-✅-三家3D事后几何.md` | C7 已过 |
| C13 | `.plan/2026年-10月-11日-✅-xlsx-pptx-公式框事后几何.md` | C8 已过 |
| C14 | `.plan/2026年-10月-11日-✅-pptx-xlsx-形状行距与字距.md` | C9 已过；不改 Word |
| 6 | `.plan/2026年-9月-27日-✅-xlsx-工作表管理.md` | 无 |
| 7 | `.plan/2026年-9月-27日-✅-xlsx-单元格样式.md` | 无 |
| 8 | `.plan/2026年-9月-27日-✅-xlsx-列宽行高与合并.md` | 无 |
| 3 | `.plan/2026年-9月-27日-✅-xlsx-公式结构.md` | 无（不求值） |
| 4 | `.plan/2026年-9月-27日-✅-xlsx-数据验证.md` | 无 |
| 5 | `.plan/2026年-9月-27日-✅-xlsx-条件格式.md` | 单元格样式 |
| 6 | `.plan/2026年-9月-27日-✅-xlsx-表格.md` | 无 |
| 7 | `.plan/2026年-9月-27日-✅-xlsx-数据透视.md` | 无（只动缓存 XML，不刷新计算结果） |
| 8 | `.plan/2026年-9月-27日-✅-xlsx-命名区域.md` | 无 |
| 9 | `.plan/2026年-9月-27日-✅-xlsx-打印设置.md` | 页眉页脚已过 |
| 10 | `.plan/2026年-9月-27日-✅-xlsx-视图与筛选.md` | 表格（表上的筛选跟 `table` 走） |
| 11 | `.plan/2026年-9月-27日-✅-xlsx-批注.md` | 无（不要套 docx 的 `comment:{w:id}`） |
| 12 | `.plan/2026年-9月-27日-✅-xlsx-形状与SmartArt.md` | 工作表图已过 |
| 13 | `.plan/2026年-9月-27日-✅-xlsx-超链接.md` | 无 |
| 14 | `.plan/2026年-9月-27日-✅-xlsx-迷你图.md` | 无 |
| 15 | `.plan/2026年-9月-27日-✅-xlsx-保护.md` | 无 |
| 16 | `.plan/2026年-9月-27日-✅-xlsx-create页眉页脚.md` | 页眉页脚 apply 已过 |
| 17 | `.plan/2026年-9月-27日-✅-xlsx-锚点剩余.md` | 工作表图已过 |
| 18 | `.plan/2026年-9月-29日-✅-xlsx-无txBox形状与连接线.md` | 形状与 SmartArt |
| 19 | `.plan/2026年-9月-29日-✅-xlsx-组合.md` | 工作表图 |
| 20 | `.plan/2026年-9月-29日-✅-xlsx-OLE.md` | 工作表图 |
| 21 | `.plan/2026年-9月-29日-✅-xlsx-切片器.md` | 数据透视、表格 |
| 22 | `.plan/2026年-9月-29日-✅-xlsx-外部数据连接.md` | 无 |
| 23 | `.plan/2026年-10月-9日-✅-xlsx-分组分级显示.md` | 无 |
| 24 | `.plan/2026年-10月-9日-✅-xlsx-拆分窗格与自定义视图.md` | 视图与筛选 |
| 25 | `.plan/2026年-10月-9日-✅-xlsx-create带批注超链接形状SmartArt.md` | 批注、超链接、形状 |
| 26 | `.plan/2026年-10月-9日-✅-xlsx-create图表工作表.md` | 图表工作表 |
| 27 | `.plan/2026年-10月-9日-✅-xlsx-排序.md` | 视图与筛选 |
| 28 | `.plan/2026年-10月-9日-✅-xlsx-共享公式改主格.md` | 公式结构 |
| 29 | `.plan/2026年-10月-9日-✅-xlsx-外部引用.md` | 公式结构 |
| 30 | `.plan/2026年-10月-9日-✅-xlsx-保护密码哈希.md` | 保护 |
| 31 | `.plan/2026年-10月-9日-✅-xlsx-dialogsheet与macrosheet.md` | 工作表管理 |
| 32 | `.plan/2026年-10月-10日-✅-xlsx-条件格式与迷你图字段.md` | 条件格式、迷你图 |
| 33 | `.plan/2026年-10月-10日-✅-xlsx-时间线切片器.md` | 切片器 |
| 34 | `.plan/2026年-10月-10日-✅-xlsx-外部引用路径与数据表公式.md` | 外部引用、公式结构 |
| 35 | `.plan/2026年-10月-10日-✅-xlsx-隐藏行列与排序字段.md` | 分组、排序 |
| 36 | `.plan/2026年-10月-10日-✅-xlsx-页眉chartsheet与Drawing链.md` | 页眉、形状、OLE、超链接 |
| 37 | `.plan/2026年-10月-10日-✅-xlsx-允许编辑区域与xlsm.md` | 保护 |
| 38 | `.plan/2026年-10月-10日-✅-xlsx-create第四轮.md` | 32–37 对应 apply 已过 |
| 39 | `.plan/2026年-10月-10日-✅-xlsx-命名单元格样式.md` | 单元格样式 |
| 40 | `.plan/2026年-10月-10日-✅-xlsx-create第五轮.md` | 39 对应 apply 已过 |
| 41 | `.plan/2026年-10月-11日-✅-xlsx-切片器锚点.md` | 切片器、时间线已过 |
| 42 | `.plan/2026年-10月-11日-✅-xlsx-切片器标题.md` | 切片器已过 |
| 43 | `.plan/2026年-10月-11日-✅-xlsx-主题.md` | 无 |
| 44 | `.plan/2026年-10月-11日-✅-xlsx-切片器筛选项.md` | 切片器、时间线已过 |
| C16 | `.plan/2026年-10月-11日-✅-三家主题objectDefaults.md` | 主题已过 |
| C15 | `.plan/2026年-10月-11日-✅-三家3D相机与灯光.md` | C7、C12 已过 |
| 45 | `.plan/2026年-10月-18日-✅-xlsx-工作表墨迹.md` | 工作表图。Excel 16.0 打开无修复 |
| 46 | `.plan/2026年-10月-19日-✅-三家OLE事后改尺寸.md` | OLE 已过。与 docx 68、pptx 54 同一份计划。Office 16.0 打开无修复 |
| 47 | `.plan/2026年-10月-31日-✅-xlsx-工作表改名删除改写引用.md` | 工作表管理（6）、公式结构已过。Excel 16.0 改名后看到新表名，删表后看到 #REF! |
| 48 | `.plan/2026年-10月-31日-✅-xlsx-删外部引用时断开链接.md` | 外部引用（29）已过。Excel 16.0 断开后只剩另一条链接 |
| 52 | `.plan/2026年-11月-11日-✅-xlsx-新建数据表公式.md` | 公式结构已过。Excel 16.0 打开列输入数据表无修复，结果为 2、4、6 |
| 49 | `.plan/2026年-11月-10日-✅-xlsx-工作表背景图.md` | 工作表图已过。Excel 16.0 打开无修复，背景不计 `pictureCount` |
| 53 | `.plan/2026年-11月-11日-✅-xlsx-公式-逻辑.md` | 50 的求值器。Excel 16.0 对照过 `IF` 短路和 `AND` / `OR` 的错误 |
| 54 | `.plan/2026年-11月-11日-✅-xlsx-公式-信息.md` | 50 的求值器。Excel 16.0 对照过 `IS` 的隐式交集、`CELL` 和能稳定重现的 `INFO` |
| 55 | `.plan/2026年-11月-11日-✅-xlsx-公式-数学与三角.md` | 50 的求值器。Excel 16.0 对照过舍入、`SUBTOTAL` 隐藏行和 `MMULT` |
| 56 | `.plan/2026年-11月-11日-✅-xlsx-公式-文本.md` | 50 的求值器。Excel 16.0 对照过字节截取、`CODE` 和 `DOLLAR`。`JIS` 在这台中文 Excel 上是 `#NAME?`，不实现 |
| 57 | `.plan/2026年-11月-11日-✅-xlsx-公式-日期与时间.md` | 50 的求值器。Excel 16.0 对照过假闰日、工作日和 `YEARFRAC`。`TODAY` 与 `NOW` 写本地时间并带 `ca="1"` |
| 58 | `.plan/2026年-11月-11日-✅-xlsx-公式-统计.md` | 55。Excel 16.0 对照过计数、`PERCENTRANK`、`NORMDIST` 和数组公式里的 `LINEST`。`ECMA.AVERAGE` 不求值 |
| 59 | `.plan/2026年-11月-11日-✅-xlsx-公式-查找与引用.md` | 50。Excel 16.0 对照过精确与近似查找、`OFFSET` / `INDIRECT` 的 `ca="1"` 和行字段求和透视。`ECMA.VLOOKUP` 不求值 |
| 60 | `.plan/2026年-11月-11日-✅-xlsx-公式-数据库.md` | 58。Excel 16.0 对照过并且、或者、通配符和 `DGET`。`ECMA.DSUM` 不求值 |
| 61 | `.plan/2026年-11月-11日-✅-xlsx-公式-财务.md` | 57。Excel 16.0 对照过借贷、折旧、债券和国库券。`ECMA.PV` 不求值 |
| 62 | `.plan/2026年-11月-11日-✅-xlsx-公式-工程.md` | 55。Excel 16.0 对照过贝塞尔、进制补码和复数文本。`ECMA.DEC2BIN` 不求值 |
| 63 | `.plan/2026年-11月-11日-✅-xlsx-公式-多维数据集.md` | 50。不连接立方体。没有连接时七个函数写入 `#N/A`，不带 `ca`。`ECMA.CUBEVALUE` 不求值 |
| 64 | `.plan/2026年-11月-13日-✅-xlsx-窗体控件.md` | 按钮、复选框、下拉可查看、修改、插入、删除。Excel 16.0 打开无修复 |
| 65 | `.plan/2026年-11月-13日-✅-xlsx-旧式页眉图.md` | 页眉页脚已过。只有 `legacyDrawingHF` 的旧式页眉图可查看、换字节、删除。Excel 16.0 打开无修复 |
| 66 | `.plan/2026年-11月-14日-✅-xlsx-批注框.md` | 批注已过。按格子行列改 Note 的显隐和宽高。Excel 16.0 另存无修复 |
| 67 | `.plan/2026年-11月-14日-🚫-xlsx-GETTING_DATA.md` | 50。保存的工作簿留不住这种短暂错误，不凭空写 |
| 68 | `.plan/2026年-11月-14日-✅-xlsx-图表三维引用.md` | 50。系列公式 `表1:表2!区域` 的数值缓存按这些表上的格子重写 |
| 69 | `.plan/2026年-11月-14日-✅-xlsx-图表定义名.md` | 50。`OFFSET`、`INDEX` 这类定义名能求成格子时，重写系列数值缓存 |
| 70 | `.plan/2026年-11月-14日-🚫-xlsx-迷你图结果.md` | 50。包里没有折线坐标，不另造 Excel 不读的点缓存 |
| 71 | `.plan/2026年-11月-14日-✅-xlsx-条件格式命中.md` | 50。inspect 按已保存的值列出规则命中的格子，不把填充写进单元格 |
| 72 | `.plan/2026年-11月-14日-✅-xlsx-数据验证结果.md` | 50。inspect 展开列表并回答当前值是否通过，不写进文件 |
| 73 | `.plan/2026年-11月-14日-🚫-xlsx-计算链.md` | 50。完整链见已过 80。残缺链仍删除，不关整本重算 |
| 74 | `.plan/2026年-11月-14日-✅-xlsx-透视其余版式.md` | 50。多个行字段、一个列字段、筛选、计数、平均值和合计占比写入格子。Excel 16.0 打开无修复 |
| 75 | `.plan/2026年-11月-14日-✅-xlsx-透视多列版式.md` | 74。多个列字段、列字段加多个行字段、列字段加多个值写入格子。Excel 16.0 打开无修复 |
| 76 | `.plan/2026年-11月-14日-✅-xlsx-GETPIVOTDATA其余版式.md` | 75。按已写入的格子回答。空交叉是 0，平均值不把子项相加。Excel 16.0 打开无修复 |
| 77 | `.plan/2026年-11月-14日-✅-xlsx-条件格式公式命中.md` | 71。`expression` 和非常量 `cellIs` 在 inspect 列出命中格，不写填充。Excel 16.0 打开无修复 |
| 78 | `.plan/2026年-11月-14日-✅-xlsx-数据验证自定义.md` | 72。`custom` 能求值时在 inspect 回答是否通过，不写进文件。Excel 16.0 打开无修复 |
| 79 | `.plan/2026年-11月-14日-✅-xlsx-图表系列三维与定义名.md` | 68、69。`setChartSeries` 写入能落到格子上的三维引用和定义名。Excel 16.0 打开无修复 |
| 80 | `.plan/2026年-11月-14日-✅-xlsx-写出计算链.md` | 73。依赖排全时写出完整链。有环、数据表、`INDIRECT`、`OFFSET` 仍删除。不关 `fullCalcOnLoad`。Excel 16.0 打开无修复 |
| 81 | `.plan/2026年-11月-14日-✅-xlsx-迷你图取值.md` | 70。inspect 按范围顺序列出已有格子缓存。不新求值，不写回迷你图。Excel 16.0 打开无修复 |
| 50 | `.plan/2026年-11月-10日-✅-xlsx公式求值对照.md` | 11 类函数切片（53–63）已 ✅。父计划收口见 `.plan/2026年-11月-16日-✅-xlsx-公式求值对照收口.md`。有显示缓存的类已求值并写入。条件格式、数据验证不写进文件。`#GETTING_DATA` 见已过 67 🚫。计算链见已过 80，迷你图取值见已过 81 |
| 51 | `.plan/2026年-11月-11日-✅-三家-VML真源对象.md` | 另存已做并已收口，见 `.plan/2026年-11月-16日-✅-三家-VML真源对象收口.md`。窗体见已过 64，旧式页眉图见已过 65，批注框见已过 66。不新开 VML 绘图引擎。与 docx / pptx 的 C22 同一份 |

## 待做

Power Query 一等公民仍不挂。

第七轮。改写引用只按记号改公式文本，不求值。第七轮待做已经做完。

第八轮是 §18.17.7 的函数。按规范里的 11 类各开一份，序号 53 到 63。一类做完再开下一类。第五版类目表以外的函数名仍不求值。

| 序 | 计划 | 前置 |
| --- | --- | --- |
| 82 | `.plan/2026年-11月-14日-⚠️-xlsx-图表出图.md` | C23 已过。只读 PNG 已接上。宽高、柱色和缩放 100 的数值轴已对齐。字形灰度、圆角和个别柱边 1 像素仍差，容差保持 0 |
| 83 | `.plan/2026年-11月-14日-⚠️-xlsx-迷你图出图.md` | 81 已过。折线 PNG 已接上。64×20 对照差 86 像素，最大通道差 73 |
| 84 | `.plan/2026年-11月-14日-⚠️-xlsx-工作表出图.md`；描边子项 `.plan/2026年-11月-16日-⚠️-xlsx-工作表描边形状.md`；缩放裁剪 `.plan/2026年-11月-16日-⚠️-xlsx-工作表缩放裁剪.md` | 只读 PNG。按 Excel 16.0 的 `Range.CopyPicture` 画已用区域。不乘工作表缩放。字形灰度仍可能对不齐，容差保持 0 |

## 先探测再进待做

探测 Office 16.0 另存后再挂序号。未探测的不算欠账。

2026-11-10 探测过。Power Query / `queryTables` 结果区仍不挂。Word 水印、内容控件、编号、修订、参考文献、`wpc` 画布不搬。工作表背景图已过，见序号 49。

## 跨格式（本类型要跟）

| 序 | 计划 | 前置 |
| --- | --- | --- |
| C2 | `.plan/2026年-9月-27日-✅-SmartArt跨家族.md` | Word / PowerPoint 已过；工作表图示已进树（序号 12） |
| C5 | `.plan/2026年-10月-9日-✅-xlsx-pptx-嵌入SVG.md` | 工作表图；docx 44 已过 |
| C6 | `.plan/2026年-10月-9日-✅-三家图嗅探补EMF-WMF-TIFF.md` | 工作表图；C5 已过 |
| C7 | `.plan/2026年-10月-9日-✅-三家3D模型.md` | 工作表图 |
| C8 | `.plan/2026年-10月-9日-✅-xlsx-pptx-嵌入OMML.md` | 工作表图 / 文本框；不要改 Word 公式 |
| C9 | `.plan/2026年-10月-9日-✅-pptx-xlsx-形状文字格式.md` | 工作表文本框 / 形状；pptx 形状增删；不要改 Word `setRunFormat` |
| C10 | `.plan/2026年-10月-10日-✅-pptx-xlsx-字填充与轮廓.md` | 形状文字格式 C9；不要改 Word |
| C11 | `.plan/2026年-10月-10日-✅-图表稀有剩余.md` | 图表稀有类型 C4 |
| C12 | `.plan/2026年-10月-11日-✅-三家3D事后几何.md` | C7 已过 |
| C13 | `.plan/2026年-10月-11日-✅-xlsx-pptx-公式框事后几何.md` | C8 已过 |
| C14 | `.plan/2026年-10月-11日-✅-pptx-xlsx-形状行距与字距.md` | C9 已过；不改 Word |
| C17 | `.plan/2026年-10月-31日-✅-三家-chartex改数据与地图图表.md` | C11 已过。地图图表 🚫：Excel 16.0 不另存地理缓存 |
| C18 | `.plan/2026年-10月-31日-✅-三家-旧版口令哈希.md` | 三家保护已过。Excel 16.0 用同一口令解开改写后的 SHA-512 |
| C20 | `.plan/2026年-11月-1日-✅-三家-chartex缓存缺失时按引用取值.md` | C17 已过。Excel 另存的隐藏定义名可以读回。Word / PowerPoint 不写定义名 |
| C21 | `.plan/2026年-11月-2日-✅-三家-chartex认cx2与cx7.md` | C20 已过。漏斗、直方、帕累托另存后回到 `charts`。Excel 16.0 改系列后没有 Repair |
| C23 | `.plan/2026年-11月-14日-✅-三家-出图金样容差.md` | 连续导出差 0。柱形图 481×294，幻灯片 1280×720。Word 没有页面 PNG |
| C24 | `.plan/2026年-11月-14日-✅-三家-用例出图对照.md` | 基线：通过 4，跳过 1104，失败 31，金样失败 2 |
| C25 | `.plan/2026年-11月-14日-❌-三家-出图补齐.md` | 第 0 段灰度差暂缓。第 1 段已开：没有图内缓存时读格子。第 2 段组合、整像素裁剪、母版形状和没有颜色变换的主题色矩形已画。第 10 段关掉网格和隐藏行列已画。从右到左没有文字时整图翻转，有文字仍待做。第 14 段没有颜色变换的主题色页底已画。渐变、图片页底和带合并单元格的表仍待做。第 9 段两种纯色的图案、虚线、双线、点划线和斜点划线已画。上面没有底边时竖线从上边界涂起。主题色 tint 已按 Excel 的 240 级 HLS 取亮度。斜线边框和渐变填充仍待做。没写 `defaultRowHeight` 的空行按 14.25 磅画。非负柱形迷你图已画。柱形的高点、低点、首点和末点换成纯色。折线仍差抗锯齿，盈亏和日期轴仍待做。第 12 段没有描边的纯色矩形已画。不透明 PNG 在不缩放时已画。整像素且不缩放的裁剪已画，要缩放的裁剪仍待做。同一单元格里的恒等组合已画。锚点框和组框按像素重合的组合可以跨格，也可以按整数比例缩放，或绕中心转 90 度的倍数并翻转。子矩形也可以绕自己的中心转 90 度的倍数。子矩形的水平或垂直翻转不改变这块纯色。组合可以再套一层。里层也可以绕中心转 90 度。里层和外层可以同时绕中心转 90 度。其它角度的组合仍待做。没有颜色变换的主题色矩形已画。没有描边的纯色矩形转 90 度的倍数，或做水平、垂直翻转时，按锚点外接矩形铺色。第 11 段公式规则的纯色和实线边框已画。高于平均、前 N 和包含文本的纯色已画。公式差分的点线、发丝、双线和点划线已画。重复值、唯一值、空白、非空白、错误值和非错误的纯色已画。时间段的纯色已按包里的公式画。图标集选择语义 ⚠️（出图仍 NI，尚无可复现精确绘制路径）；公式差分 `dashed`/`slantDashDot` ⚠️（节律已锁，整图像素门 DPI blocked）。负数数据条 x14 实心 middle 见 ⚠️ 切片（主题色-only 仍 NI）。第 7 段只有分号、两段到四段都空时不画数字，四段连文字和布尔一起不画。日期、时间、货币和其它多段格式仍待做。第 8 段和第 13 段仍待做。2026-11-16 起未完成项拆成小切片，见出图补齐「2026-11-16 后续切片」，不要把已有 ✅ 再写成待办 |


本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-货币与多段格式出图.md` — 无 `[` 货币/多段 string 已与 Excel `Range.Text` 对齐并可 `exportSheetImage`；CopyPicture 金样 633×45@106 vs lib 555×41，整图 pxBlocked。

xlsx 没有 Word / PowerPoint 那种墨迹批注计划；批注已过（序号 11）。主题见已过 43，不要和 pptx 16 / 46 / 47 / 48 合成一份。C4、C5、C6、C7、C8、C9、C10、C11、C12、C13、C14、C16、C15 见已过表。C5 与 pptx 同一份计划，不要拆成两家各做一套 `svgBlip`。C6、C7、C12、C16 与 docx 同一份计划。C8、C13 与 pptx 同一份，不改 Word。C9、C10、C14 与 pptx 同一份，不改 Word。C11 三家同一份。

## 怎么算 Excel 全量做完

已过表保持 ✅。第五轮待做（40）已过。第六轮 41–46、C16、C15 已过。第七轮待做已过。49、53、54、55、56、57、58、59、60、61、62、63、64、65、66 已过。67 `#GETTING_DATA` 🚫。68 图表三维引用的数值缓存已过。69 图表定义名的数值缓存已过。70 迷你图结果 🚫。71 条件格式命中格子已过。72 数据验证是否通过已过。73 计算链 🚫。74 透视其余版式已过。75 透视多列版式已过。76 GETPIVOTDATA 其余版式已过。77 条件格式公式命中已过。78 数据验证自定义已过。79 图表系列三维引用和定义名已过。50、51 已过。83 未过。80 计算链已过：依赖排全时写出，排不全仍删除。81 迷你图取值已过：inspect 列出范围内已有格子缓存，不写回包。82 图表出图已接上，像素仍差，标 ⚠️。C23 出图金样已过，连续导出差 0。只读 PNG 已放开，像素不写回包。永不做的条目不算欠账。50 里有显示缓存的类已经写入。计算链在依赖排全时另写，见 80。迷你图只在查看时列出已有格子缓存，见已过 81，不另造点坐标。51 已另存：窗体和页眉图不是单份 VML，PowerPoint 旧式图形见 pptx 58，Word 水印旧版副本见 docx 79，批注框见已过 66。46 OLE 事后改尺寸已过，Office 16.0 打开无修复。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-日期时间格式出图.md` — 内置 14–22 + yyyy/mm/dd string 对齐 Range.Text；CopyPicture 721×22@106 vs lib 641×21 pxBlocked。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-错误值出图.md` — 点名七码 `t=e` string+paint；CopyPicture 809×30@106 vs lib 705×27 pxBlocked。
本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-换行缩进出图.md` — wrapText/indent paint；CopyPicture 373×221@106 vs lib 325×201 pxBlocked；Face::width 断点。
本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-从右到左文字出图.md` — sheetView RTL + 拉丁/CJK 正写；readingOrder=2/强 RTL NI；gold 145×43@106 vs lib 129×49 pxBlocked。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-斜线边框出图.md` — thin/medium 实线斜线；gold 256×110@106 vs lib 223×100 pxBlocked；等线行高 14.25 纠偏。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-渐变填充出图.md` — SpreadsheetML 两停点不透明 linear（sRGB mix，非 PPT gamma）；path/theme/alpha NI；CopyPicture 203×60@106 vs lib 177×54 pxBlocked；等线 11→14.25 COM 来源写入契约。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-负数数据条出图.md` — x14 实心 middle；gold 506×22@106 vs lib 441×96 pxBlocked；主题色-only/未知轴仍 NI。前刀：`.plan/2026年-11月-16日-⚠️-xlsx-盈亏迷你图.md` — stacked 盈亏半高柱；gold 73×22@106 vs lib 64×19 pxBlocked；日期轴有限核查仍 ❌。

本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-超大区域切块.md` — tilePx 切块；stitch differ=0；默认 4096 NI 不绕；MAX_TILE_COUNT/总像素闸。
本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-图标集出图.md` — 选择语义子 PASS；字形出图仍 NI。
本分支 follow-up：`.plan/2026年-11月-16日-⚠️-xlsx-差分虚线斜点划线.md` — CF dashed/slantDashDot；节律 oracle；整图 sizeDiffer。


本分支共享缺陷修复：`.plan/2026年-11月-17日-⚠️-xlsx-共享列几何与尺寸链路.md`。Normal 字体固定 96 MDW、直接 OOXML 列宽以及 sheet/tiles/spark/twoCell 入口已共用；固定 96 维度回归通过，官方整图仍未全部通过。其它未开工功能保持暂停。


2026-11-17 PR10既有图例缺陷修复：见`.plan/2026年-11月-17日-⚠️-xlsx-图例样式与位置修复.md`。有效普通txPr样式、五位置、overlay、普通manualLayout与entry删除不再成功返回却静默丢失，复杂有效配置诚实NI；styled-right纠正为合法手动框/叠放组合，不称Office坏图。新增10项回归、全套1849通过；旧缺省右图PNG SHA不变。7组同源严格比较仍636×371 vs576×336，字体/边距/灰阶等残差保留，不把布局修复标整图✅。100705逐图有限复核继续在本次待办，其余十项新功能暂停。


### 2026-11-17 P0：公平诊断语料修复（不是renderer验收）

- [x] 修geometry-matrix保留chart A20:B23实际源、同步used dimension；每次出图前核对actual/formula/cache与样式索引；8条独立回归。
- [x] 旧localized-normal明确为combined；新增四个单变量组。新分母17×3=51（50成功/1既有NI），Office17个chart实际值1/4/2及分类A/B/C全核验。
- [x] 报告透明展示白底合成，raw gold/严格RGBA比较不变。
- 100705的13×3仅历史尺寸诊断：其中13张chart缺实际公式源、缓存仍在，不能作内容验收；旧快照不改。
- 新公平矩阵固定96维度50/50，renderer未改、before/after宽度变化0；严格0 PASS / 50 sizeDiffer / 1 NI，不把内容正确说成像素通过。
- [x] P1：共同sheet/tiles复用已有spark painter，16个支持宿主恢复；隐藏源sheet诚实NI，详内嵌遗漏计划。P3圆角alpha仍未修。Normal-font-only的Office规范化残差仍记录，不称合法XF异常。其它10项未开工功能继续暂停。
- 验证：fmt、clippy all-targets/all-features -D warnings、全库1849 passed/0 failed/0 ignored、Python8 passed、实际CLI build；Office本轮自有wb=0、锁=0，无Quit。
- 新证据：桌面 `office-editor-PR10-公平对照与遗漏修复-20261117-131235`（index/README/metrics/source SHA/Office实际Series与字体/原始PNG）。


### 2026-11-17 P1：修复已有内嵌spark遗漏

- 共同宿主raster保留f32 coverage；单独PNG像素不改，sheet在真实底色上合成，tile只裁完整宿主，不重采样。
- 原13组12个支持sheet恢复/1隐藏源NI；扩展17组16恢复/1 NI。16个宿主对独立painter差0、宿主外变化0；16 spark/17 chart PNG字节不变。真实CLI非A1九块拼接差0。
- 51项当前49成功/2 NI，成功项49/49固定96维度；严格0 PASS/49 sizeDiffer/2 NI，widthChanged0/statusChanged1，不把诚实NI冒充尺寸回归。
- 可见复杂/未支持配置诚实NI；隐藏宿主不影响其它内容；RTL/merged/非空文字宿主不伪造支持，dateAxis等暂停功能不恢复。
- fmt、clippy all-targets/all-features -D warnings、全Rust1860 passed/0 failed/0 ignored、Python8、实际CLI build通过。P3/自动值轴/端点/Normal继承残差另核，不塞本片。
- 证据：桌面 `office-editor-PR10-迷你图遗漏修复-20261117-134723`；详情 `.plan/2026年-11月-17日-⚠️-xlsx-工作表内嵌迷你图遗漏.md`。gold复用P0同SHA原生图，P1未重开Office、owner lock0。


2026-11-17 P3（82圆角）：既有默认白底圆角PNG恢复coverage直出alpha，内部白/颜色及白底合成不变；合法但未支持的背景/描边/显式非圆角NI。整图及圆弧AA仍⚠️，不把17图四角透明恢复记全部通过；详圆角计划。自动值轴与暂停10项不在本片。

验证：fmt、clippy all-targets/all-features -D warnings、完整Rust1864 passed/0 failed/0 ignored、Python8 passed、实际CLI build通过。17 chart四角RGBA=0；不透明内侧RGB差0、白底合成差0；16 sheet/16 spark字节不变。51项严格0 PASS/49 sizeDiffer/2 NI，原始gold未改。证据：桌面 `office-editor-PR10-圆角透明修复-20261117-135846`。


2026-11-17 自动值轴切片（既有PR范围）：发现旧5..10区间忽略高度/完整font line box，旧显式min/max/major/minor未读取。有限27组Office COM标量/真实数据对照，26支持×4尺度值一致，窄范围非零auto minimum一项NI。大图4.5/.5由COM确认合法、保留旧测试，小图1/4/2改5/1。多值轴/log/反向/非默认交点/非General格式不扩功能；不宣称全域自动或整图通过。P3 AA、spark端点/Normal继承仍未解决，10个暂停新功能不恢复。详自动值轴计划与新证据。

最终验证：fmt --check、clippy all-targets/all-features -D warnings、完整Rust1868 passed/0 failed/0 ignored、Python8、实际CLI build通过。独立checker 27数据/cells/cache/公式/SHA验证，26支持×4标量=104一致/1窄范围NI。旧51项仍0 PASS/49 sizeDiffer/2 NI，新27项0 PASS/26 sizeDiffer/1 NI；16 sheet/16 spark字节不变、17 chart alpha不变，34原源SHA不变。证据：桌面 `office-editor-PR10-自动值轴修复-20261117-141413`。

默认autoZero交点同片补齐并核对26项，合计104尺度值+26交点=130逻辑值相同；原17/新27严格分母不变。最终全Rust1868/0/0、Python8及fmt/clippy/CLI重建与独立checker再次通过。


## 2026-11-17 t127：DPI差异分层验收（非新功能）

用户允许**已证实仅DPI引起的像素差**通过；raw严格结果保留但不再一票否决。库固定96 contract不变，原始gold不缩放、不改色、不放宽正常配置。验收必须绑定source SHA/实际exe，核实内容、尺度、字体及合法继承、位置/anchor后才可豁免命名子范围；sizeDiffer、同尺寸或逻辑标量通过不能直接推出整图通过。曲角AA、spark端点、默认边距或布局若未证因，写待确认，真实缺内容/样式/标签位置差仍须修。

当前原51几何子证书：**38通过（DPI豁免）/11待确认/2 NI**；11中6 Normal字体继承、5 chart导出1px边界舍入。轴26/27 case的130逻辑值、P1接线16/16、P3四角17/17分别是已通过子范围，不合并张数。完整图原51=49待确认/2NI；新27=26待确认/1NI；图例7待确认。raw严格49/26/7 sizeDiffer保留，不能当最终失败统计。

本轮只验收/文档，renderer基线a81c49e6；原1868 Rust和8 Python日志保留，新策略再加10 Python测试。证据：桌面 `office-editor-PR10-DPI验收复核-20261117-145216`。详 `manual-check/office/dpi_acceptance.py`、`dpi-acceptance-report.py` 和DPI验收复核计划。COM图例中心-4pt尚未校准原点，禁止直接硬补；explicit类别标签侧此前读图误报已撤回：COM=4 NextToAxis且双方原生bbox均在零轴下/绘图区底之上，不修改renderer。其它十项暂停新功能不恢复。


2026-11-17 类别标签反证复核：1ec6eac0中的explicit“Office标签在绘图区底”是读图错误，已明确撤回。当前1ec实际exe重渲染与a81图片SHA相同；COM公开TickLabelPosition=4（NextToAxis），原生bbox证实双方均跟随0轴，原有130逻辑值成果不动。修正验收器/CSV，不制造renderer补丁。38项几何DPI通过不变；曲角/端点/其它布局待确认不变。详类别标签误报复核计划及桌面150736新证据，旧快照不覆盖。


## 2026-11-17 Normal继承6路径有限核实（非renderer修复）

2个合法样式族×3路径=6记录、4份源，库按builtin0→styleXF→font的规范链96几何6/6验证；Office实际Styles.Normal与独立SaveCopy后的Normal均为Calibri11，而源声明Arial11，已证明不只是name/xf编号归一化。此探针族中导入改变字体语义，不能算仅DPI，也不证明库应改font0。17组有限控制、实际cell字体与Normal分开、列/Chart点数、原始/保存副本XML及SHA均保存；canonical从未重开、不当新gold。

DPI几何分子**38/49不变**，6 Normal路径仍需Office同语义可比证据，另5导出边界/2 NI不扩本片。6条规范链证书不是6功能/6整图通过；保留builtinId优先、非零xf及MDW96生产合同，不加7/8/font0回退。23 Python/语法/fmt/diff检查；无Rust改动，不冒称重跑1868。证据：桌面office-editor-PR10-Normal继承核实-20261117-151855；详Normal继承核实计划及normal-inheritance-report.py。


## 2026-11-17 Chart.Export宽度边界5条核实（仅验收器修复）

5个零off twoCell簇状柱图的旧ceil(ChartObject.Width*106/72)比原PNG多1，原因是将与Single ShapeRange相同的点数读回当作无限精确像素边界。公开Double列宽与同Worksheet左右端点通过IEEE754半ULP精确区间认证设备栅格，分别320/288/320/288/243px，与原PNG严格一致；不加epsilon、不容差，真多1px仍拒绝。

五个新frame-ext一致性单变量控制Office PNG字节不变；十个新原生PNG均与旧gold字节相同，十个库输出与旧after字节相同。公开DPI均106，pHYs4173/m是整数米量化，非新显示DPI。ChartArea/PlotArea内侧原点未当PNG外框。只更新宽度责任链，未宣称通用导出舍入算法，未改renderer/96/576336zoom/高度/alpha。

**最新几何DPI证书43/49（此前38历史值由此更新）**；本片5/5子范围通过，不代表5功能/43整图。Normal6仍待同语义oracle、2NI不变；其它AA/内容/样式残差仍分别保留，raw严格列仅诊断。29 Python/语法/fmt/diff验证，无Rust改动不冒称重跑全套。证据：桌面office-editor-PR10-导出边界核实-20261117-153629；chart-boundary-report.py/原始API/哈希/更新CSV。十个暂停新功能未恢复。
