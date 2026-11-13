# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Excel 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Excel 16.0 已打开，用户确认通过。本机若装了 WPS，`Excel.Application` 会被抢走，测试仍直接启动 `EXCEL.EXE` 再挂 COM。
2026-10-30 补跑 `xml-attr-newline.ps1`。数据验证提示是两行，中间还有制表符，无 Repair。
2026-10-31 补跑 `xlsx-id-pin.ps1`。先删第一条数据验证再改原来第三条，先删第一条条件格式再改原来第二条。Excel 16.0 没有 Repair。
2026-10-31 补跑 `xlsx-chart-id-pin.ps1`。删掉第一个系列后给原来第二个系列加趋势线，趋势线挂在 Q2 上。Excel 16.0 没有 Repair。
2026-11-01 补跑 `xlsx-sheet-refs.ps1`。改名后公式和定义名换成 `New Name`，字符串里的表名不动。删表后公式和定义名变成 `#REF!`。Excel 16.0 没有 Repair。
2026-11-01 补跑 `xlsx-break-link.ps1`。断开第一条外部链接后，编辑链接只剩原来的第二条，单元格公式仍指向那个文件。没有缓存的格子是 0。Excel 16.0 没有 Repair。
2026-11-01 补跑 `xlsx-legacy-password.ps1`。遗留 XOR 口令 `secret` 改写成 SHA-512 后，Excel 16.0 用同一口令解开工作表保护。没有 Repair。
2026-11-01 补跑 `chartex-edit.ps1`。瀑布图改数据后 Excel 读到 42 和 7，加的第二条系列在 C2。漏斗和树状图 Word、PowerPoint 打开没有 Repair。
2026-11-01 重跑整套手测。公式还引用外部链接时 `deleteExternalLink` 断开链接，B1 留下 `0`，`LinkSources` 少一条，没有 Repair。`xlsx-svg` 与 `xlsx-emf` 曾因 Excel 的 Open 失败，清掉进程后通过。
2026-11-01 补跑 `chartex-cache.ps1`。Excel 把瀑布图 B2 改成 42 另存后，inspect 读到 42。再 `setChartSeries` 后 Excel 打开没有 Repair，B2、B3 是 7 和 8。
2026-11-02 补跑 `chartex-cache.ps1` 的 Cca3、Cca4。六种布局另存后 inspect 都在 `charts` 里，漏斗、直方、帕累托的第一个值是 42。再改这三张的系列，Excel 打开没有 Repair，源格是 7。
2026-11-13 补跑 `xlsx-background.ps1`，Xbg1–Xbg4 通过。Excel 16.0 打开没有 Repair，另存后背景 id 仍是 `sheet:Sheet1/background`。删除后 inspect 为 `null`。
2026-11-13 补跑 `xlsx-logic.ps1`，Xlg1–Xlg3 通过。Excel 16.0 打开没有 Repair。`IF` 短路与 `AND` / `OR` 的错误和库缓存一致。
2026-11-13 补跑 `xlsx-info.ps1`，Xinf1–Xinf3 通过。Excel 16.0 打开没有 Repair。`IS` 系列、`CELL` 和能稳定重现的 `INFO` 与库缓存一致。
2026-11-13 补跑 `xlsx-math.ps1`，Xmh1–Xmh3 通过。Excel 16.0 打开没有 Repair。舍入、`SUBTOTAL` 的隐藏行和 `MMULT` 与库缓存一致。
2026-11-13 补跑 `xlsx-text.ps1`，Xtx1–Xtx3 通过。Excel 16.0 打开没有 Repair。字符截取、字节截取和 `DOLLAR` / `BAHTTEXT` 与库缓存一致。
2026-11-13 补跑 `xlsx-date.ps1`，Xdt1–Xdt3 通过。Excel 16.0 打开没有 Repair。假闰日、工作日和 `YEARFRAC` 与库缓存一致。`TODAY` 与当天序列一致。
2026-11-13 补跑 `xlsx-stat.ps1`，Xst1–Xst3 通过。Excel 16.0 打开没有 Repair。计数、`PERCENTRANK` 和 `NORMDIST` 与库缓存一致。`ECMA.AVERAGE` 没有缓存。
2026-11-13 补跑 `xlsx-lookup.ps1`，Xlk1–Xlk3 通过。Excel 16.0 打开没有 Repair。精确与近似查找、`OFFSET` 和行字段求和透视与库缓存一致。`ECMA.VLOOKUP` 没有缓存。
2026-11-13 补跑 `xlsx-db.ps1`，Xdb1–Xdb3 通过。Excel 16.0 打开没有 Repair。`DSUM`、并且与或者、`DGET` 与库缓存一致。`ECMA.DSUM` 没有缓存。
2026-11-13 补跑 `xlsx-fin.ps1`，Xfn1–Xfn3 通过。Excel 16.0 打开没有 Repair。借贷、债券和 `DOLLARDE` 与库缓存一致。`ECMA.PV` 没有缓存。
2026-11-13 补跑 `xlsx-eng.ps1`，Xeg1–Xeg3 通过。Excel 16.0 打开没有 Repair。贝塞尔、进制补码和复数文本与库缓存一致。`ECMA.DEC2BIN` 没有缓存。
2026-11-13 补跑 `xlsx-cube.ps1`，Xcu1–Xcu2 通过。Excel 16.0 打开没有 Repair。没有连接时七个立方体函数是 `#N/A`。`ECMA.CUBEVALUE` 没有缓存。
2026-11-13 补跑 `xlsx-form-control.ps1`，Xfc1–Xfc5 通过。Excel 16.0 打开没有 Repair。插入的按钮、复选框、下拉能读到；删掉按钮后，复选框和下拉还在。

C、I、A、O、Xp、Xvg、Xem、X3d、Xom、Xtf、Xln、Xfl、Xc、Xh、Xfs、Xtl、Xef、Xns、Xdf 这几组现在由 `manual-check/office/cases/xlsx-*.ps1` 自动判定，
跑 `pwsh -File manual-check\office\run.ps1 -Case 'xlsx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。
2026-09-27 补跑 `xlsx-pictures.ps1`，Xp1–Xp7 通过。同日补跑 `xlsx-charts.ps1`，Xc1–Xc7 通过。
同日补跑 `xlsx-create-pictures.ps1`，C8–C10 通过。同日补跑 `xlsx-create-charts.ps1`，C11–C13 通过。
同日补跑 `xlsx-headerfooter.ps1`，Xh1–Xh5 通过。同日补跑 `xlsx-docprops.ps1`，Xd1–Xd5 通过。同日补跑 `xlsx-charts-remain.ps1`，Xr1–Xr4 通过。
同日补跑 `xlsx-sheets.ps1`，Xm1–Xm7 通过。同日补跑 `xlsx-cellstyle.ps1`，Xs1–Xs5 通过。同日补跑 `xlsx-layout.ps1`，Xl1–Xl5 通过。
同日补跑 `xlsx-formula.ps1`，Xf1–Xf5 通过。2026-09-28 补跑 `xlsx-validation.ps1`，Xv1–Xv5 通过。
同日补跑 `xlsx-conditional-format.ps1`，Xn1–Xn5 通过。同日补跑 `xlsx-pivot.ps1`，Xpt1–Xpt5 通过。
同日补跑 `xlsx-defined-name.ps1`，Xnm1–Xnm5 通过。同日补跑 `xlsx-print.ps1`，Xps1–Xps5 通过。同日补跑 `xlsx-view.ps1`，Xvw1–Xvw5 通过。
同日补跑 `xlsx-comments.ps1`，Xcm1–Xcm5 通过。同日补跑 `xlsx-shapes.ps1`，Xsa1–Xsa5 通过。同日补跑 `xlsx-hyperlinks.ps1`，Xhl1–Xhl5 通过。
同日补跑 `xlsx-sparklines.ps1`，Xsk1–Xsk5 通过。同日补跑 `xlsx-protect.ps1`，Xpr1–Xpr5 通过。
同日补跑 `xlsx-create-headerfooter.ps1`，C14–C16 通过。同日补跑 `xlsx-anchor.ps1`，Xa1–Xa5 通过。
2026-09-29 补跑 `xlsx-charts-rare.ps1`，Xq1–Xq5 通过。同日补跑 `xlsx-ole.ps1`，Xol1–Xol5 通过。同日补跑 `xlsx-slicers.ps1`，Xsl1–Xsl5 通过。
同日补跑 `xlsx-connections.ps1`，Xcn1–Xcn5 通过。2026-10-09 补跑 `xlsx-outline.ps1`，Xon1–Xon5 通过。
同日补跑 `xlsx-custom-view.ps1`，Xsv1–Xsv5 通过。同日补跑 `xlsx-create-drawing.ps1`，C17–C21 通过。
同日补跑 `xlsx-create-chartsheet.ps1`，C22–C24 通过。同日补跑 `xlsx-sort.ps1`，Xsr1–Xsr5 通过。
同日补跑 `xlsx-external-link.ps1`，Xel1–Xel5 通过。同日补跑 `xlsx-protect-hash.ps1`，Xph1–Xph5 通过。
同日补跑 `xlsx-dialog-macro.ps1`，Xdm1–Xdm5 通过。2026-09-28 补跑 `xlsx-chartsheet.ps1`，图表工作表 Xs1–Xs4 通过。同日补跑 `xlsx-table.ps1`，Xt1–Xt5 通过。
2026-10-09 补跑 `xlsx-svg.ps1`，Xvg1–Xvg6 通过。Excel COM `Type=28`，96×48 对应 914400×457200 EMU。
同日补跑 `xlsx-emf.ps1`，Xem1–Xem6 通过。Excel COM `Type=13`，无 Repair。
同日补跑 `xlsx-model3d.ps1`，X3d1–X3d6 通过。Excel COM `Type=30`，无 Repair。
同日补跑 `xlsx-omml.ps1`，Xom1–Xom6 通过。无 Repair。
同日补跑 `xlsx-shape-format.ps1`，Xtf1–Xtf5 通过。无 Repair。
2026-10-10 补跑 `xlsx-text-fill.ps1`，Xfl1–Xfl5 通过。无 Repair。
2026-10-10 补跑 `xlsx-charts-rare-remain.ps1`，Xcx1–Xcx5 通过。无 Repair。
2026-10-10 补跑 `xlsx-cf-spark-style.ps1`，Xfs1–Xfs5 通过。无 Repair。
2026-10-10 补跑 `xlsx-timelines.ps1`，Xtl1–Xtl5 通过。COM `SlicerCaches` 类型为 2，无 Repair。
2026-10-10 补跑 `xlsx-extpath-datatable.ps1`，Xef1–Xef5 通过。无 Repair。
2026-10-10 补跑 `xlsx-edit-range.ps1`，Xer1–Xer5 通过。无 Repair。COM `AllowEditRanges` 能看见区域，`.xlsm` 无 Repair。
2026-10-10 补跑 `xlsx-create-round4.ps1`，C25–C29 通过。无 Repair。COM 能见表、AllowEditRanges、OutlineLevel，组合 `Type=6`。
2026-10-10 补跑 `xlsx-named-style.ps1`，Xns1–Xns5 通过。无 Repair。COM `Styles` 能看见自定义名，套用后 `Style.Name` 对得上，黄底 `Interior.Color=65535`。
2026-10-10 补跑 `xlsx-create-round5.ps1`，C30–C32 通过。无 Repair。COM `Style.Name` 为 `OoxpNamed`，黄底 `Interior.Color=65535`，`CommentsThreaded` 能看见一层回复。
2026-10-13 补跑 `xlsx-model3d-geom.ps1`，X3g1–X3g6 通过。无 Repair。COM `Width` / `Height` 288×144 磅。`from=C3`。`twoCell` `to=D8`。`replaceModel3d` 不改几何。
2026-10-13 补跑 `xlsx-slicer-anchor.ps1`，Xsn1–Xsn6 通过。无 Repair。COM 288×144 磅。`from=G4`。`twoCell` `to=H12`。时间线 `from=I5`。
2026-10-14 补跑 `xlsx-slicer-caption.ps1`，Xsc1–Xsc5 通过。无 Repair。COM `Caption` 对得上。`columnCount=2`。时间线 `showHeader=false`。
2026-10-14 补跑 `xlsx-theme.ps1`，Xth1–Xth5 通过。无 Repair。空白另存有 theme。`accent1=FF0000`。`majorLatin=Arial`。create 可写。省略为 null。
2026-10-14 补跑 `xlsx-theme-defaults.ps1`，Xdf1–Xdf5 通过。无 Repair。create 后 `objectDefaults` 为 `null`。`lnW=38100`。`lnRef=1` `fontRef=minor`。`null` 清掉。
同日补跑 `xlsx-model3d-camera.ps1`，X3c1–X3c5 通过。无 Repair。缺省 `pos.z=66519324` `fov=2700000`，`rot` 三轴 `null`，`lightRig` `null`。改后 `pos.x=50400000` `fov=1892375` `rev=18000000`。`lightRig` 可清。对 3D id 发 `setPictureSize` 得 `InvalidId`。
2026-10-14 补跑 `xlsx-slicer-filter.ps1`，Xsi1–Xsi5 通过。无 Repair。表格 `selected=[]` 再 `乙`。透视勾选乙。时间线 `dateBetween` 再 `null`。
2026-10-14 补跑 `xlsx-math-geom.ps1`，Xmg1–Xmg5 通过。无 Repair。`oneCell` `cx=1828800` `cy=914400`。`from=C3`。`twoCell` `to=D8`。`setMath` 不改几何。
2026-10-14 补跑 `xlsx-shape-spacing.ps1`，Xln1–Xln5 通过。无 Repair。COM `SpaceBefore=12`，`Font.Spacing=2`。`textAnchor=just`。

- **C1** ✅
  - Excel 16.0 打开 `manual-check/office/out/xlsx-create.xlsx`，标题栏无 Repair
- **C3–C7** ✅
  - Excel：Sheet1、A1=1、A2=hello、A3=TRUE、B1=`=SUM(A1)` 且值为 1
- **I1–I6** ✅
  - 有范围的 inspect 含 A1/A2/A3/B1、无 E1
  - 无 scope 时没有 `cells`
- **A1** ✅
  - Excel 打开 `out/xlsx-applied.xlsx`，标题栏无 Repair
- **A2–A7** ✅
  - Excel：A1=42，B1=`=SUM(A1:A10)` 值为 47，C1 空且数字格式仍是 `0.00`，Sheet2 未动，E1 加粗/红字/黄底/列宽 20 还在，D1 重算为 84
- **O1** 🚫
  - 领域层不做原样保存：三个动作里没有「只保存」，`apply` 的 `ops` 必须非空
  - 零脏写回见 OPC 库测试
  - Excel 另存 create 产物见 `xlsx-O1-excel-resave`
- **O2** ✅
  - 未改表与未点名格子的样式还在
  - 未改部件 CRC 与打补丁前相同
- **Xp1–Xp7** ✅
  - Excel 16.0：无 `pictures` 时 `pictureCount=3`
  - 名单含 `oneCell` 与 `twoCell`
  - 换/插/删无修复
  - `oneCell` 尺寸 144 磅
  - 锚点改到 F12
  - export 与 `/xl/media/…` 字节一致
- **Xvg1–Xvg6** ✅
  - Excel 16.0：`xlsx-svg.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType=image/svg+xml`，COM `Type=28`，无 Repair
  - `exportMedia` 为 SVG 原字节
  - 换成 PNG 后没有 `contentType`、包内没有 `.svg`
  - 奇数页页眉图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`
  - 非图片 `file` 得 `InvalidField`
- **Xem1–Xem6** ✅
  - Excel 16.0：`xlsx-emf.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType` 为 `image/x-emf` / `image/x-wmf`，COM `Type=13`，无 Repair
  - `exportMedia` 为 EMF 原字节
  - 换成 PNG 后没有 `contentType`、包内没有 `.emf`
  - 奇数页页眉 TIFF 图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`
  - 非图片 `file` 得 `InvalidField`
- **X3d1–X3d6** ✅
  - Excel 16.0：`xlsx-model3d.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，COM `Type=30`，无 Repair
  - `exportMedia` 为 GLB 原字节
  - `replaceModel3d` 后 id 不变
  - `deleteModel3d` 后包内没有 `.glb`
  - 非 GLB `file` 得 `InvalidField`
- **X3g1–X3g6** ✅
  - Excel 16.0：`xlsx-model3d-geom.ps1`，6 PASS / 0 FAIL
  - `oneCell` 改尺寸、改锚点、`twoCell` 给 `to`
  - `replaceModel3d` 不改几何
  - 对 3D id 发 `setPictureSize` 得 `InvalidId`
  - `twoCell` 给 `cx`、xlsx 上 `setModel3dWrap` 得 `InvalidField`
  - 标题栏无 Repair
- **X3c1–X3c5** ✅
  - Excel 16.0：`xlsx-model3d-camera.ps1`，5 PASS / 0 FAIL
  - 缺省相机 / 旋转 / 光源对得上
  - `setModel3dCamera` 写位移、视野、三轴与 `lightRig.rev`
  - `lightRig` 为 `null` 后树上为 `null`
  - 对 3D id 发 `setPictureSize` 得 `InvalidId`
  - 空字段与负 `fov` 拒写
  - 标题栏无 Repair
- **Xom1–Xom6** ✅
  - Excel 16.0：`xlsx-omml.ps1`，6 PASS / 0 FAIL
  - `insertMath` 后树上 `text` 为 `E=mc^2`，不进 `textBoxes`，XML 含 `a14:m`，无 Repair
  - `setMath` 后 id 不变、`text` 换成新字
  - create `sheets[].maths` 同样写出
  - `deleteMath` 后 `mathCount` 为 0
  - 出现 `expr` / 空 `text` 得 `InvalidField`
- **Xmg1–Xmg5** ✅
  - Excel 16.0：`xlsx-math-geom.ps1`，5 PASS / 0 FAIL
  - `oneCell` `setMathSize` 后树上 `cx=1828800` `cy=914400`，无 Repair
  - `setMathAnchor` 后 `from=C3`
  - `twoCell` `setMathSize` 后 `to=D8`
  - `setMath` 不改几何。对公式框 id 发 `setTextBoxAnchor` 得 `InvalidId`
  - `twoCell` 给 `cx`、xlsx 上 `setMathPos` 得 `InvalidField`
- **Xln1–Xln5** ✅
  - Excel 16.0：`xlsx-shape-spacing.ps1`，5 PASS / 0 FAIL
  - `insertTextBox` `lnSpc` pts 2400，无 Repair
  - `spcBef` pts 1200、`spc=200` `kern=1200`
  - COM `SpaceBefore=12`，`Font.Spacing=2`
  - `textAnchor=just`。`lnSpc.type=lines` 得 `InvalidField`
- **Xc1–Xc7** ✅
  - Excel 16.0：无 `charts` 时 `chartCount=1`、没有名单
  - 名单 id 为 `sheet:Sheet1/chart:2`，系列 10/20/30 与单元格一致
  - 改系列为 100/200/300 无修复
  - `insertChart` 插 `oneCell` 且 `A10` 一带有数据表
  - 删图后数据格还在
  - 改折线 COM 认 line
  - `oneCell` 改到 F20、尺寸 144×72 磅
  - `exportChartData` 得 `InvalidField`
- **C8–C10** ✅
  - Excel 16.0：inspect 第一张 `sheet:Sheet1/pic:1`、`oneCell`、`from=B2`
  - 第二张 `twoCell`、`D2:F8`
  - 打开无 Repair，Shapes=2
- **C11–C13** ✅
  - Excel 16.0：inspect 第一张 `sheet:Sheet1/chart:1`、`type=col`、`from=E2`
  - 系列 1/2/3 与 `A10` 一带单元格一致
  - 打开无 Repair，Shapes=1 且 HasChart
- **C14–C16** ✅
  - Excel 16.0：`xlsx-create-headerfooter.ps1`，3 PASS / 0 FAIL
  - inspect 奇数页页眉含 `C14页眉`
  - PageSetup 中区页眉页脚对得上
  - 页眉图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`
  - 无 Repair
- **Xh1–Xh5** ✅
  - Excel 16.0：无 `headerFooter` 时有 `hasHeaderFooter`、没有名单
  - 展开六槽，奇数页页眉原文含 `Xh页眉`
  - `setHeaderFooterText` 后 PageSetup 为新字且无 Repair
  - 只开 `pictures` 时名单不含 `hf:`
  - 往 `hf:odd:header` 插图 id 为 `…/pic:2`，换图后无 Repair
- **Xd1–Xd5** ✅
  - Excel 16.0：`xlsx-docprops.ps1`，5 PASS / 0 FAIL
  - 根上 `props`、`creator=ooxp`
  - 只改标题后面板标题是新字、公司名还在
  - 自定义 `prop:部门`
  - 删后名单空
  - 单元格 id 发 `setProps` 为 `InvalidId`
  - 无 Repair
- **Xr1–Xr4** ✅
  - Excel 16.0：`xlsx-charts-remain.ps1`，4 PASS / 0 FAIL
  - 趋势线 COM Count=1
  - 次轴 `AxisGroup=2`
  - 命名区域改数 90/80/70 且 `c:f` 仍为 `ChartVals`
  - 并集改数成功、三维引用 `NotImplemented`
  - 无 Repair
- **Xq1–Xq5** ✅
  - Excel 16.0：`xlsx-charts-rare.ps1`，5 PASS / 0 FAIL
  - 插入股价 / 曲面 / 复合条无 Repair
  - 股价三条转柱形、复合饼转饼
  - 增系列与 `type=chartex`、两条股价均拒写
- **Xcx1–Xcx5** ✅
  - Excel 16.0：`xlsx-charts-rare-remain.ps1`，5 PASS / 0 FAIL
  - OHLC / 俯视线框 / 自定义误差线 / chartex funnel 无 Repair
  - 缺 `layoutId` 与两条股价 `InvalidField`，`regionMap` `NotImplemented`
- **Xm1–Xm7** ✅
  - Excel 16.0：`xlsx-sheets.ps1`，7 PASS / 0 FAIL
  - 无 scope 时每张表有 `hidden`
  - 插入 Sheet3、改名附录、挪到最前、隐藏 Sheet2、删除 Sheet2 均无 Repair
  - 隐藏或删除最后一张可见表得 `InvalidField` 且未写文件
- **Xs1–Xs5** ✅
  - Excel 16.0：`xlsx-cellstyle.ps1`，5 PASS / 0 FAIL
  - 默认 `numFmt=General`
  - A1 写成 `0.00` 与黄底后值还在、无 Repair
  - B1 仍是通用格式且无黄底
  - 缺字段得 `InvalidField` 且未写文件
- **Xns1–Xns5** ✅
  - Excel 16.0：`xlsx-named-style.ps1`，5 PASS / 0 FAIL
  - create 默认 `cellStyleCount=1`，名单含 `Normal`
  - 插入自定义样式后 COM 能看见、无 Repair
  - 套到 A1 后 `Style.Name` 对得上
  - 删除自定义样式后格子格式仍在；不能删 `Normal`
  - 缺字段得 `InvalidField` 且未写文件
- **Xl1–Xl5** ✅
  - Excel 16.0：`xlsx-layout.ps1`，5 PASS / 0 FAIL
  - 列宽 20、行高 30、合并 A1:B2 对得上
  - 取消合并无修复
  - 缺字段得 `InvalidField` 且未写文件
- **Xf1–Xf5** ✅
  - Excel 16.0：`xlsx-formula.ps1`，5 PASS / 0 FAIL
  - 共享主格改公式保住整组
  - 拆组后各格独立
  - 数组公式 Excel 认
  - 非主格 `NotImplemented`
- **Xv1–Xv5** ✅
  - Excel 16.0：`xlsx-validation.ps1`，5 PASS / 0 FAIL
  - A1 序列下拉能看见 1、2、3
  - 多区域 `A1`/`C1`
  - 删除后无列表校验、无 Repair
  - 缺字段得 `InvalidField` 且未写文件
- **Xn1–Xn5** ✅
  - Excel 16.0：`xlsx-conditional-format.ps1`，5 PASS / 0 FAIL
  - A1:A10 大于 0 填绿
  - 两色色阶
  - 删除后无条件格式、无 Repair
  - 缺字段得 `InvalidField` 且未写文件
- **Xpt1–Xpt5** ✅
  - Excel 16.0：`xlsx-pivot.ps1`，5 PASS / 0 FAIL
  - 建成 SalesPivot 无 Repair
  - 改源到 A1:C5 后仍认表
  - 删除后源数据还在
  - 缺字段得 `InvalidField` 且未写文件
- **Xnm1–Xnm5** ✅
  - Excel 16.0：`xlsx-defined-name.ps1`，5 PASS / 0 FAIL
  - 建成 Sales 无 Repair
  - COM Names 能看见
  - 改公式到 A1:A20 后仍认该名
  - 删除后计数为 0
  - 缺字段得 `InvalidField` 且未写文件
- **Xps1–Xps5** ✅
  - Excel 16.0：`xlsx-print.ps1`，5 PASS / 0 FAIL
  - 默认 `print` 左边距 0.7、竖向、无打印区域
  - 左边距 1 英寸与横向后 COM 对得上且页眉还在
  - 打印区域与网格线 COM 认
  - 缺字段得 `InvalidField` 且未写文件
  - 无 Repair
- **Xvw1–Xvw5** ✅
  - Excel 16.0：`xlsx-view.ps1`，5 PASS / 0 FAIL
  - 默认 `view` 缩放 100、无冻结
  - 冻结首行与缩放 80 后 COM 对得上
  - 工作表筛选范围为 `$A$1:$A$10`
  - 解冻后 `FreezePanes` 为假
  - 缺字段得 `InvalidField` 且未写文件
  - 无 Repair
- **Xcm1–Xcm5** ✅
  - Excel 16.0：`xlsx-comments.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `commentCount=0`
  - `insertComment` 后 COM `CommentsThreaded` 能看见正文且无 Repair
  - `replyComment` 后 COM `Replies` 与 inspect `replies` 对得上
  - 删除后计数为 0
  - 缺字段得 `InvalidField` 且未写文件
- **Xsa1–Xsa5** ✅
  - Excel 16.0：`xlsx-shapes.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `textBoxCount=0`、`smartArtCount=0`
  - `insertTextBox` 后 COM `Shapes` 能看见文本框且无 Repair
  - `setShapeText` 后可见字对得上
  - `insertSmartArt` `layout=list` 后 COM 能看见 SmartArt，inspect `layout` 对得上
  - 删除后计数为 0
  - 缺 `from` 得 `InvalidField` 且未写文件
- **Xtf1–Xtf5** ✅
  - Excel 16.0：`xlsx-shape-format.ps1`，5 PASS / 0 FAIL
  - `insertTextBox` 可带 `sz` / `color`
  - `setShapeFormat` 后 inspect 对得上
  - COM 字号与字色对得上
  - create `textBoxes[]` 认格式字段
  - 缺字段为 `InvalidField` 且未写文件
- **Xfl1–Xfl5** ✅
  - Excel 16.0：`xlsx-text-fill.ps1`，5 PASS / 0 FAIL
  - `setShapeFormat` 线性两停 `textFill` 后 COM Fill.Type=3
  - `textOutline` 宽 19050、COM Line.Visible 为真
  - `textFill.type=none` 后 `color` 为 null
  - 同一框两种字号 `runs` 为 2800 / 1200
  - `color` 与 `textFill` 同给为 `InvalidField` 且未写文件
  - 无修复
- **Xhl1–Xhl5** ✅
  - Excel 16.0：`xlsx-hyperlinks.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `hyperlinkCount=0`
  - `insertHyperlink` 后 COM `Hyperlinks` 能看见地址且无 Repair
  - 改地址后对得上
  - 删除后只留文字
  - 缺字段得 `InvalidField` 且未写文件
- **Xsk1–Xsk5** ✅
  - Excel 16.0：`xlsx-sparklines.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `sparklineCount=0`
  - `insertSparkline` 后 COM `SparklineGroups` 能看见折线且无 Repair
  - 改数据范围后对得上
  - 删除后计数为 0
  - 缺字段得 `InvalidField` 且未写文件
- **Xfs1–Xfs5** ✅
  - Excel 16.0：`xlsx-cf-spark-style.ps1`，5 PASS / 0 FAIL
  - `top10` `rank=3` 无 Repair
  - 低于平均与 `containsText` 生成公式、无 Repair
  - 迷你图标记与自定义轴 COM 能看见
  - 对角线与缩进 / 旋转无 Repair
  - 缺 `text`、`top10` 带 `operator`、自定义轴缺 `manualMin` 得 `InvalidField` 且未写文件
- **Xtl1–Xtl5** ✅
  - Excel 16.0：`xlsx-timelines.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `slicerCount=0`
  - `insertSlicer` `kind=timeline` 后 COM `SlicerCaches` 类型为 2 且无 Repair
  - 包内有 timelines / timelineCaches，无 slicers 部件
  - 删除后计数为 0
  - 表格源得 `InvalidField` 且未写文件
- **Xpr1–Xpr5** ✅
  - Excel 16.0：`xlsx-protect.ps1`，5 PASS / 0 FAIL
  - 无保护时根有 `workbookProtection`、工作表有 `sheetProtection` 且 `enforced=false`
  - 打开工作表保护后 COM `ProtectContents` 为真且无 Repair
  - 解开后能改格
  - 工作簿锁结构后 COM `ProtectStructure` 为真
  - 缺字段得 `InvalidField`，带哈希样例 `hasPassword=true` 且 apply 得 `NotImplemented`
- **Xph1–Xph5** ✅
  - Excel 16.0：`xlsx-protect-hash.ps1`，5 PASS / 0 FAIL
  - 带 `password` 后 `hasPassword=true`、树无哈希或明文
  - COM `ProtectContents` 为真、`Unprotect("secret")` 成功、错误口令失败、无 Repair
  - 工作簿 `ProtectStructure` 为真且能用同一口令解开
  - 对得上的口令 `enforced: false` 后无 Repair
  - 空口令 / 错口令为 `InvalidField`，不给口令改哈希为 `NotImplemented`
- **Xer1–Xer5** ✅
  - Excel 16.0：`xlsx-edit-range.ps1`，5 PASS / 0 FAIL
  - `insertProtectedRange` 后 COM `AllowEditRanges` 能看见区域且无 Repair
  - 带 `password` 后 `hasPassword=true`、树无哈希或明文，包内 SHA-512
  - 改名与区域后 inspect 对得上，删除后计数为 0
  - create 到 `.xlsm` 后 workbook Override 为 macroEnabled，inspect `format` 仍为 `xlsx`，注入的 `vbaProject.bin` 原字节留下
  - 对 chartsheet、空名、重名、只给 `op` 与 `id`、不给口令改哈希，分别得 `InvalidField` 或 `NotImplemented`
- **Xdm1–Xdm5** ✅
  - Excel 16.0：`xlsx-dialog-macro.ps1`，5 PASS / 0 FAIL
  - COM 对话框表 `kind=dialogsheet`
  - 打开无 Repair
  - 注入宏表 inspect `kind=macrosheet`
  - 拒删 / 改名 / 隐藏
  - create `kind: dialogsheet` 与点名单元格切片为 `InvalidField`
- **Xa1–Xa5** ✅
  - Excel 16.0：`xlsx-anchor.ps1`，5 PASS / 0 FAIL
  - twoCell 改 `to=G10` 无 Repair
  - oneCell 转到 absolute 后 `posX=914400`
  - 再改 `posX=2000000` 无 Repair
  - 转到 absolute 不给 pos、twoCell 给 `cx` 均得 `InvalidField` 且未写文件
- **Xsg1–Xsg5** ✅
  - Excel 16.0：`xlsx-shapes-geom.ps1`，5 PASS / 0 FAIL
  - 无名单时根与工作表有 `shapeCount=0`、`connectorCount=0`
  - `insertShape` 后 COM 能看见矩形且无 Repair
  - `insertConnector` 后名单 id 为 `sheet:…/cxn:…`
  - 改锚点与删后计数为 0
  - 缺 `from` 得 `InvalidField`
- **Xg1–Xg5** ✅
  - Excel 16.0：`xlsx-groups.ps1`，5 PASS / 0 FAIL
  - `groupShapes` 后 COM Type=6、子 id 仍在
  - `ungroup` 后组合解开
  - 形状与连接线可编成一组
  - 已组合再编、只给一个成员、`id=slide:1`、对未组合对象 `ungroup` 均拒写
  - 无 Repair
- **Xol1–Xol5** ✅
  - Excel 16.0：`xlsx-ole.ps1`，5 PASS / 0 FAIL
  - create 默认 `oleObjectCount=0`
  - `insertOle` 后 COM Type=7、id 为 `sheet:Sheet1/ole:1025`、`progId=Worksheet`、dummy 不进 `shapes`、无 Repair
  - `exportMedia` 与 embeddings 字节一致
  - 删除后计数为 0
  - 缺 `from` / `slide:1` / 形状 id 均拒写
- **Xsl1–Xsl5** ✅
  - Excel 16.0：`xlsx-slicers.ps1`，5 PASS / 0 FAIL
  - create 默认 `slicerCount=0`
  - 表格 `insertSlicer` 后 COM `SlicerCaches.Count=1`、id 为 `sheet:Sheet1/slicer:1`、`sourceKind=table`、无 Repair
  - 透视源 `sourceKind=pivot`
  - 删除后计数为 0
  - 缺 `from` / `slide:1` / 不存在的 `field` 均拒写
- **Xsn1–Xsn6** ✅
  - Excel 16.0：`xlsx-slicer-anchor.ps1`，6 PASS / 0 FAIL
  - `twoCell` 改 `from=G4`、给 `to=H12`
  - 改成 `oneCell` 后树上 `cx` / `cy` 对得上，COM 288×144 磅
  - 时间线改 `from=I5`
  - 对切片器 id 发 `setPictureSize` 得 `InvalidId`
  - `twoCell` 给 `cx` 得 `InvalidField`
  - 标题栏无 Repair
- **Xsc1–Xsc5** ✅
  - Excel 16.0：`xlsx-slicer-caption.ps1`，5 PASS / 0 FAIL
  - `insertSlicer` 带 `caption` 后树上对得上，缺省 `rowHeight=381000`、`columnCount=1`
  - `setSlicerView` 改标题、列数、`showCaption` 后 COM `Caption` 对得上
  - `style=SlicerStyleLight1` 再 `null` 后树上为 `null`
  - 时间线改 `caption` 与 `showHeader=false`
  - 空 op、对切片器给 `showHeader`、对图 id 均拒写
  - 标题栏无 Repair
- **Xsi1–Xsi5** ✅
  - Excel 16.0：`xlsx-slicer-filter.ps1`，5 PASS / 0 FAIL
  - 插入表格切片器后 `selected=[]`；透视切片器含源里各项；时间线 `filter=null`
  - 表格 `selected=["乙"]` 后 autoFilter 含乙，无 Repair
  - 透视切片器树上只有乙，无 Repair
  - 时间线 `dateBetween` 起止日对得上，再 `null` 则为 null，无 Repair
  - 空 op 与未知项得 `InvalidField`，图 id 得 `InvalidId`
- **Xcn1–Xcn5** ✅
  - Excel 16.0：`xlsx-connections.ps1`，5 PASS / 0 FAIL
  - create 默认 `connectionCount=0`
  - 网页 `insertConnection` 后 COM 认 `WebDemo`、inspect `kind=web`、无 Repair
  - 文本连接 `kind=text`、`sourceFile` 对得上
  - 删除后计数为 0
  - 缺 `type` / `id=slide:1` / `type=odbc` 均拒写
- **Xel1–Xel5** ✅
  - Excel 16.0：`xlsx-external-link.ps1`，5 PASS / 0 FAIL
  - create 默认 `externalLinkCount=0`
  - `insertExternalLink` 并写 `=[1]Sheet1!A1` 后 COM `LinkSources` 含源文件、inspect `kind=book`、无 Repair
  - 占用时删得 `NotImplemented`
  - 清公式后再删计数为 0
  - 缺 `path` / `id=slide:1` 拒写
- **Xef1–Xef5** ✅
  - Excel 16.0：`xlsx-extpath-datatable.ps1`，5 PASS / 0 FAIL
  - 列输入表主格 `=TABLE(,C1)`、`dataTableRef=B2:B4`，从属格 `dataTable` 且 `formula` 为 null
  - Excel 打开数据表无 Repair，COM `B2` 为 `=TABLE(,C1)`
  - `setExternalLink` 后 `LinkSources` 含新文件、单元格公式仍含 `[1]`
  - 主格改成 `=TABLE(,D1)` 后 inspect 与 COM 一致，XML `<f>` 无正文
  - 从属格 / 非 TABLE / 缺 `path` / 错 id 拒写
- **Xon1–Xon5** ✅
  - Excel 16.0：`xlsx-outline.ps1`，5 PASS / 0 FAIL
  - 默认 `outline.summaryBelow=true`、名单空
  - 分组行 2:4 与列 B:C 后 COM `OutlineLevel` 对得上、无 Repair
  - 折叠后细节行隐藏
  - 取消分组后级回落
  - 缺字段 / `id=slide:1` 拒写
- **Xsv1–Xsv5** ✅
  - Excel 16.0：`xlsx-custom-view.ps1`，5 PASS / 0 FAIL
  - 默认 `splitRows`/`splitCols` 为 0、`customViewCount` 为 0
  - 拆分后 COM `SplitRow=4` `SplitColumn=2`、无 Repair
  - `insertCustomView` 后 COM Count=1
  - 删除后计数 0
  - 冻结+拆分 / 空名 / `id=slide:1` 拒写
- **C17–C21** ✅
  - Excel 16.0：`xlsx-create-drawing.ps1`，5 PASS / 0 FAIL
  - inspect 批注 `ref=A1` 正文含 `C17批注`
  - 超链接 id 为 `sheet:Sheet1/link:1`
  - 形状 `from=B4` `prst=ellipse`
  - SmartArt `from=D6`
  - 打开无 Repair，COM 批注 / 超链接 / Shapes=4 / SmartArt=1
- **C22–C24** ✅
  - Excel 16.0：`xlsx-create-chartsheet.ps1`，3 PASS / 0 FAIL
  - inspect `kind=chartsheet`、id 为 `sheet:Chart1/chart:2`
  - 系列 1/2/3 与 `A10` 一带单元格一致
  - 打开无 Repair，COM 图表工作表系列值对得上
- **C25–C29** ✅
  - Excel 16.0：`xlsx-create-round4.ps1`，5 PASS / 0 FAIL
  - inspect 列宽 14、合并 `G1:G2`、数据验证、横向打印、G1 `numFmt=0.00`
  - 表 `SalesTbl`、定义名 `Sales`、迷你图 `D2`
  - 无密码保护，允许编辑区域 `Range1`
  - 大纲、冻结 1 行、drawing 含 `xdr:grpSp`
  - 打开无 Repair，COM 能见表、AllowEditRanges、OutlineLevel，组合 `Type=6`
- **C30–C32** ✅
  - Excel 16.0：`xlsx-create-round5.ps1`，3 PASS / 0 FAIL
  - inspect `cellStyleCount=2`，名单含 `OoxpNamed`，A1 黄底加粗
  - A1 批注正文与 `replies[0].text` 对得上
  - 打开无 Repair，COM `Style.Name=OoxpNamed`，`CommentsThreaded` 能看见回复
- **Xsr1–Xsr5** ✅
  - Excel 16.0：`xlsx-sort.ps1`，5 PASS / 0 FAIL
  - 默认 `sortState` 为 `null`
  - `setSortState` 后 inspect `ref=A1:A3`、`descending=true`、A1 仍为 3
  - COM `Sort.SortFields` 降序键 `$A$1:$A$3`、无 Repair
  - `ref: null` 后为 `null`
  - 缺 `ref` / 列越界得 `InvalidField`
- **Xhd1–Xhd5** ✅
  - Excel 16.0：`xlsx-hidden-sort.ps1`，5 PASS / 0 FAIL
  - `setRowHidden` 行 3、`setColHidden` 列 B 后 inspect `outline` 含 `level=0` 的隐藏行 / 列
  - COM 第 3 行与 B 列 Hidden，无 Repair
  - `customList=Low,Medium,High` 后 inspect 对上，A1 仍为 `Low`
  - `sortBy=cellColor` 且 `sortMethod=stroke` 后 inspect 对上，无 Repair
  - `cellColor` 不给 `color`、`icon` 带 `customList` 得 `InvalidField`；`id=slide:1` 得 `InvalidId`，均未写文件
- **Xdh1–Xdh5** ✅
  - Excel 16.0：`xlsx-drawing-hlink.ps1`，5 PASS / 0 FAIL
  - 图上 `insertHyperlink` 后 inspect `url` 对得上，COM Address 对得上，`hyperlinkCount=0`，无 Repair
  - `setHyperlink` 改 `url` 后 COM 更新；`deleteHyperlink` 后 `hyperlink` 为 `null`、图还在、无 Repair
  - chartsheet `insertShape` 带 `fromX` 四元组，inspect `anchor=relSize`，COM `Shapes.Count>=1`，无 Repair
  - chartsheet `insertPicture` 后 `pictureCount>=1`，无 Repair
  - chartsheet 单元格 `from`、图 id 带 `ref` 得 `InvalidField`，均未写文件
- **图表工作表 Xs1–Xs4** ✅
  - Excel 16.0：`xlsx-chartsheet.ps1`，4 PASS / 0 FAIL
  - `insertChartSheet` 后 `kind=chartsheet`、`chartCount=1`，无 Repair
  - 数据 1/2/3 与 COM 一致
  - 删除后表消失、数据格还在
  - 对 `worksheet` 发 `deleteChartSheet` 得 `InvalidId` 且未写文件
- **Xt1–Xt5** ✅
  - Excel 16.0：`xlsx-table.ps1`，5 PASS / 0 FAIL
  - create 默认 `tableCount=0`
  - 建成 Sales 后 COM 认表
  - 改范围到 `A1:C4` 无 Repair
  - 删除后数据还在、不再有表
  - 缺 `ref` 得 `InvalidField` 且未写文件


## create

用 `docs/examples/xlsx-create.json` 生成 `book.xlsx`。

- **C1**
  - 检查：Excel 打开
  - 通过标准：无修复提示、无「文件损坏」
- **C3**
  - 检查：工作表名
  - 通过标准：存在 `Sheet1`
- **C4**
  - 检查：A1
  - 通过标准：数字 `1`
- **C5**
  - 检查：A2
  - 通过标准：文本 `hello`
- **C6**
  - 检查：A3
  - 通过标准：布尔，显示为 `TRUE` / `真`
- **C7**
  - 检查：B1
  - 通过标准：公式栏为 `=SUM(A1)`，打开后值变成 `1`（允许先出现空白再重算）
- **C8**
  - 检查：带图 oneCell
  - 通过标准：用 `docs/examples/xlsx-create-pictures.json`（换成本机 PNG）生成
  - inspect `pictures: true` 时第一张 id 为 `sheet:Sheet1/pic:1`，`anchor` 为 `oneCell`，`from` 为 `B2`
- **C9**
  - 检查：带图 twoCell
  - 通过标准：同上，第二张 `anchor` 为 `twoCell`，`from` / `to` 对上
- **C10**
  - 检查：Excel 打开带图产物
  - 通过标准：无修复
  - 两张图都能看见
- **C11**
  - 检查：带图表
  - 通过标准：用 `docs/examples/xlsx-create-charts.json` 生成
  - inspect `charts: true` 时第一张 id 为 `sheet:Sheet1/chart:1`，`type` 为 `col`，`from` 为 `E2`
- **C12**
  - 检查：图表数据
  - 通过标准：同上，系列值与 `dataFrom` 一带单元格一致
- **C13**
  - 检查：Excel 打开带图表产物
  - 通过标准：无修复
  - 图能看见
- **C14**
  - 检查：带页眉文字
  - 通过标准：用 `docs/examples/xlsx-create-headerfooter.json`（图换成本机 PNG）生成
  - `headerFooter: true` 时奇数页页眉 `text` 含 `C14页眉`，`hasHeaderFooter` 为 `true`
- **C15**
  - 检查：Excel 打开带页眉文字
  - 通过标准：无修复
  - PageSetup 中区页眉对得上 `C14页眉`
- **C16**
  - 检查：带页眉图
  - 通过标准：同上
  - `headerFooter` 与 `pictures` 同时开时奇数页页眉图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`
  - Excel 无修复
- **C17**
  - 检查：带批注
  - 通过标准：用 `docs/examples/xlsx-create-drawing.json` 生成
  - `comments: true` 时第一条 `ref` 为 `A1`，`text` 含 `C17批注`
- **C18**
  - 检查：带超链接
  - 通过标准：同上
  - `hyperlinks: true` 时第一条 id 为 `sheet:Sheet1/link:1`，`url` 含 `example.com`
- **C19**
  - 检查：带形状
  - 通过标准：同上
  - `shapes: true` 时第一条 `from` 为 `B4`，`prst` 为 `ellipse`
- **C20**
  - 检查：带 SmartArt
  - 通过标准：同上
  - `smartArt: true` 时第一条 `from` 为 `D6`，`layout` 为 list 族 uniqueId
- **C21**
  - 检查：Excel 打开带批注形状产物
  - 通过标准：无修复
  - COM 线程化批注、超链接与 Shapes（含 SmartArt）都能看见
- **C22**
  - 检查：带图表工作表
  - 通过标准：用 `docs/examples/xlsx-create-chartsheet.json` 生成
  - `charts: true` 时存在 `name=Chart1` 且 `kind=chartsheet`，`chartCount` 为 1，图表 id 为 `sheet:Chart1/chart:2`
- **C23**
  - 检查：图表工作表数据
  - 通过标准：同上
  - Sheet1 `dataFrom` 一带单元格与系列值 1/2/3 一致
- **C24**
  - 检查：Excel 打开带图表工作表产物
  - 通过标准：无修复
  - COM 图表工作表系列值对得上
- **C25**
  - 检查：create 第四轮列宽、合并、数据验证、打印、单元格样式
  - 通过标准：用 `docs/examples/xlsx-create-round4.json` 生成
  - inspect `cols` 宽 14、`merges` 含 `G1:G2`、`dataValidations` 一条、`print.orientation=landscape`、G1 `numFmt` 为 `0.00`
- **C26**
  - 检查：表格、定义名、迷你图
  - 通过标准：同上
  - `tableCount=1` 名 `SalesTbl`，`definedNames` 含 `Sales`，`sparklineCount=1`
- **C27**
  - 检查：无密码保护与允许编辑区域
  - 通过标准：同上
  - `sheetProtection.enforced=true`、`hasPassword=false`，`protectedRanges` 名为 `Range1`
- **C28**
  - 检查：大纲、冻结、组合
  - 通过标准：同上
  - `outline.summaryBelow=true`、`view.frozenRows=1`，drawing 含 `xdr:grpSp`
- **C29**
  - 检查：Excel 打开第四轮产物
  - 通过标准：无修复
  - COM 能见表、AllowEditRanges、OutlineLevel；不要 COM 打开带哈希或外部链的产物
- **C30**
  - 检查：create 第五轮命名单元格样式
  - 通过标准：用 `docs/examples/xlsx-create-round5.json` 生成
  - inspect `cellStyleCount=2`，名单含 `OoxpNamed`，A1 有黄底加粗
- **C31**
  - 检查：一层批注回复
  - 通过标准：同上
  - `comments: true` 时 A1 批注正文对得上，`replies[0].text` 对得上
- **C32**
  - 检查：Excel 打开第五轮产物
  - 通过标准：无修复
  - COM `Style.Name` 为 `OoxpNamed`，`CommentsThreaded` 能看见回复

## inspect

对上面的文件发 `docs/examples/xlsx-inspect.json`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 表名单 | 含 `Sheet1`，id 为 `sheet:Sheet1` |
| I2 | 范围外 | `E1` 不出现在 `cells` |
| I3 | A1 | `type` 为 `number`，`value` 为 `1` |
| I4 | A2 | `type` 为 `text`，`value` 为 `hello` |
| I5 | B1 | `formula` 为 `=SUM(A1)` |
| I6 | 无 scope | 只发 path 时没有 `cells` 字段，仍有 `pictureCount`、`props` |

## apply

准备一份 Excel 亲手另存的样例（含两张表、若干样式、一张表里有公式）。仓库里那份是 `crates/ooxp/tests/fixtures/excel-source.xlsx`，由 Excel 16.0 `SaveAs` 写出，
重做跑 `manual-check/office/export-excel-fixture.ps1`。
对它执行 `docs/examples/xlsx-apply.json` 里的操作，必要时改 id 里的表名。库测试 `xlsx_excel_source.rs` 覆盖下面 A2–A7 的 XML 与 O2；A1 仍要 Office 打开。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft Excel 能打开，无修复提示 |
| A2 | 目标格 | `Sheet1!A1` 变成 `42` |
| A3 | 公式格 | `Sheet1!B1` 公式栏为 `=SUM(A1:A10)`，打开后会重算 |
| A4 | 清空 | `Sheet1!C1` 值为空，若原先有数字格式，格式还在 |
| A5 | 未改表 | 第二张表的格子与另存前一致 |
| A6 | 未改样式 | 未点名的格子字体、填充、列宽看起来不变 |
| A7 | 重算 | 依赖 A1 的其他公式在打开后得到新结果，而不是旧缓存 |

## 原样保存（opc 验收）

O1（用领域 JSON 打开再原样写出）第一期放弃。三个动作没有「只保存」，`apply` 不许空 `ops`。零脏写回在 OPC 层验收，见 `docs/opc.md`。

O2 仍按 apply 验收：没点名的表和样式看起来不变。

Office 另存 create 产物是打开冒烟，见 `manual-check/office/cases/opc-resave.ps1`，不顶替 O1。

## 工作表图

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少各有一张 `oneCell` 与一张 `twoCell` 的 DrawingML 图。不要用库 `create` 产物当带图样例。另备一张本地 PNG 供换图、插图。
手测脚本建议 `manual-check/office/cases/xlsx-pictures.ps1`。编号用 Xp，避免和 create 的 C 组、inspect 的 I 组重号。
Excel 16.0 的 `Shapes.AddPicture` 只写 `twoCellAnchor`（`Placement` 只改 `editAs`），
真 `oneCellAnchor` 由 ooxp 插入后再让 Excel `Save()`。

- **Xp1**
  - 检查：计数与名单
  - 通过标准：无 `pictures` 时工作簿与目标表有 `pictureCount`、没有 `pictures`
  - `pictures: true` 时 id 为 `sheet:…/pic:…`，`anchor` / `from` 与工作表一致
- **Xp2**
  - 检查：换图
  - 通过标准：`replacePicture` 后 Excel 无修复
  - 锚点不变
- **Xp3**
  - 检查：插图
  - 通过标准：`insertPicture` 往 `worksheet` 插 `oneCell`，Excel 能看见且无修复
- **Xp4**
  - 检查：删图
  - 通过标准：`deletePicture` 后该图消失，无修复
- **Xp5**
  - 检查：尺寸
  - 通过标准：对 `oneCell` `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差）
- **Xp6**
  - 检查：锚点
  - 通过标准：`setPictureAnchor` 把 `from` 改到另一格后，图跟着走，无修复
- **Xp7**
  - 检查：导出
  - 通过标准：`exportMedia` 写出的文件与 `/xl/media/…` 字节一致
  - 不强制开 Excel

## 嵌入 SVG

工作表图与页眉图可带 Office 2016 `asvg:svgBlip`。不算渲染引擎，不把 SVG 原文放进 inspect 树。
手测脚本 `manual-check/office/cases/xlsx-svg.ps1`。编号用 Xvg，避免和自定义视图的 Xsv、Word 的 Sv 混号。
COM 认 `Type=28`（`msoGraphic`），不是普通图的 `13`。

- **Xvg1**
  - 检查：插入
  - 通过标准：`insertPicture` 把本地 SVG 写入工作表后，树上 `contentType` 为 `image/svg+xml`，Excel 无修复
  - COM 该形状 `Type=28`
- **Xvg2**
  - 检查：导出
  - 通过标准：对该图 `exportMedia` 得到 SVG 原字节，`contentType` 为 `image/svg+xml`
- **Xvg3**
  - 检查：create
  - 通过标准：`sheets[].pictures[].file` 指向 SVG 后，树上有 `contentType`，Excel 无修复
- **Xvg4**
  - 检查：换成位图
  - 通过标准：`replacePicture` 换成 PNG 后没有 `contentType`，包内没有 `.svg`，Excel 无修复
- **Xvg5**
  - 检查：页眉
  - 通过标准：往奇数页页眉插 SVG 后，页眉图名单末尾有 `contentType`，Excel 无修复
- **Xvg6**
  - 检查：拒错
  - 通过标准：`file` 不是图片得 `InvalidField`，未写文件
  - 不强制开 Excel

## 嵌入 EMF / WMF / TIFF

工作表图与页眉图按文件头认 EMF、WMF、TIFF，写入原字节。不要 PNG 占位，不要 `svgBlip`。不算渲染引擎。
手测脚本 `manual-check/office/cases/xlsx-emf.ps1`。编号用 Xem。
COM 认 `Type=13`（`msoPicture`），不是 SVG 的 `28`。

- **Xem1**
  - 检查：插入
  - 通过标准：`insertPicture` 把本地 EMF 写入工作表后，树上 `contentType` 为 `image/x-emf`，Excel 无修复
  - COM 该形状 `Type=13`，包内有 `.emf`、没有 `svgBlip`
- **Xem2**
  - 检查：导出
  - 通过标准：对该图 `exportMedia` 得到 EMF 原字节，`contentType` 为 `image/x-emf`
- **Xem3**
  - 检查：create
  - 通过标准：`sheets[].pictures[].file` 指向 WMF 后，树上 `contentType` 为 `image/x-wmf`，Excel 无修复
- **Xem4**
  - 检查：换成位图
  - 通过标准：`replacePicture` 换成 PNG 后没有 `contentType`，包内没有 `.emf`，Excel 无修复
- **Xem5**
  - 检查：页眉
  - 通过标准：往奇数页页眉插 TIFF 后，页眉图名单末尾 `contentType` 为 `image/tiff`，Excel 无修复
- **Xem6**
  - 检查：拒错
  - 通过标准：`file` 不是图片得 `InvalidField`，未写文件
  - 不强制开 Excel

## 工作表 3D 模型

工作表 drawing 里的 `am3d` AlternateContent。只认 GLB。不进 `pictures`。Fallback 海报不进 `pictures`。不算 3D 渲染引擎。
手测脚本 `manual-check/office/cases/xlsx-model3d.ps1`。编号用 X3d。
COM 认 `Type=30`（`mso3DModel`）。页眉槽本期不插。

- **X3d1**
  - 检查：插入
  - 通过标准：`insertModel3d` 把本地 GLB 写入工作表后，树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，Excel 无修复
  - COM 该形状 `Type=30`，包内有 `.glb`、XML 含 `am3d:model3d`
- **X3d2**
  - 检查：导出
  - 通过标准：对该 3D id `exportMedia` 得到 GLB 原字节，`contentType` 为 `model/gltf.binary`
- **X3d3**
  - 检查：create
  - 通过标准：`sheets[].model3ds[].file` 指向 GLB 后，树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，Excel 无修复
- **X3d4**
  - 检查：替换
  - 通过标准：`replaceModel3d` 换成另一份 GLB 后 id 不变，导出是新字节，Excel 无修复
- **X3d5**
  - 检查：删除
  - 通过标准：`deleteModel3d` 后 `model3dCount` 为 0，包内没有 `.glb`，Excel 无修复
- **X3d6**
  - 检查：拒错
  - 通过标准：`file` 不是 GLB 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 工作表墨迹

工作表 drawing 里 `Requires="xdr14"` 的 `xdr:contentPart`。一笔一个对象。只认 InkML。不进 `pictures`。Fallback 海报不进 `pictures`。不算手写识别。
库测试 `crates/ooxp/tests/xlsx_ink.rs`。Excel 16.0 打开桌面上的 `ooxp-工作表墨迹.xlsx`（两笔 `twoCell`），没有弹出修复。编号用 Xink。

- **Xink1**
  - 检查：插入
  - 通过标准：`insertInk` 把本地 InkML 写入工作表后，树上 `contentType` 为 `application/inkml+xml`，不进 `pictures`，Excel 无修复
- **Xink2**
  - 检查：导出
  - 通过标准：对该墨迹 id `exportMedia` 得到 InkML 原字节，`contentType` 为 `application/inkml+xml`
- **Xink3**
  - 检查：两笔
  - 通过标准：两次 `insertInk` 后 `inkCount` 为 2，两个 id 不同，Excel 无修复
- **Xink4**
  - 检查：替换与删除
  - 通过标准：`replaceInk` 后 id 不变、导出是新字节；`deleteInk` 后 `inkCount` 为 0，包内没有 `/xl/ink/`，Excel 无修复
- **Xink5**
  - 检查：锚点
  - 通过标准：`setInkAnchor` 把 `oneCell` 改成 `twoCell` 后树上 `to` 为给定单元格，Excel 无修复
- **Xink6**
  - 检查：拒错
  - 通过标准：`file` 不是 InkML 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 工作表 3D 事后几何

已有工作表 3D 改尺寸与锚点。不换 GLB。不算 3D 渲染。对 3D id 发 `setPictureSize` / `setPictureAnchor` 仍报 `InvalidId`。
手测脚本 `manual-check/office/cases/xlsx-model3d-geom.ps1`。编号用 X3g，不要和 X3d 撞。docx / pptx 见 D3g / P3g（C12）。

- **X3g1**
  - 检查：改尺寸
  - 通过标准：对 `oneCell` `setModel3dSize` 后树上 `cx` / `cy` 对得上。Excel 无修复。COM 尺寸对得上（允许 2 磅误差）
- **X3g2**
  - 检查：改锚点
  - 通过标准：`setModel3dAnchor` 把 `from` 改到另一格后，树上 `from` 对得上。Excel 无修复
- **X3g3**
  - 检查：twoCell
  - 通过标准：`anchor=twoCell` 且给 `to` 后，`setModel3dSize` 给 `to` 树上对得上。Excel 无修复
- **X3g4**
  - 检查：替换不改几何
  - 通过标准：改过尺寸后再 `replaceModel3d`，树上尺寸仍是新值。Excel 无修复
- **X3g5**
  - 检查：图 op
  - 通过标准：对 3D id 发 `setPictureSize` 得 `InvalidId`，未写文件
  - 不强制开 Excel
- **X3g6**
  - 检查：拒错
  - 通过标准：`twoCell` 给 `cx` 得 `InvalidField`。xlsx 上 `setModel3dWrap` 得 `InvalidField`。未写文件
  - 不强制开 Excel

## 工作表 3D 相机与灯光

已有工作表 3D 改 `am3d:camera`、`am3d:rot` 与 `spPr` 里的 `a:scene3d/a:lightRig`。不换 GLB。不算 3D 渲染。
手测脚本 `manual-check/office/cases/xlsx-model3d-camera.ps1`。编号用 X3c，不要和 X3d / X3g 撞。docx / pptx 见 D3c / P3k。

- **X3c1** ✅
  - 检查：缺省
  - 通过标准：`insertModel3d` 后树上 `camera.pos.z` 为 `66519324`，`fov` 为 `2700000`，`rot` 三轴 `null`，`lightRig` 为 `null`。Excel 无修复
- **X3c2** ✅
  - 检查：改相机 / 旋转 / 光源
  - 通过标准：`setModel3dCamera` 写 `pos.x=50400000`、`fov=1892375`、旋转三轴与 `lightRig.rev=18000000` 后树上对得上。Excel 无修复
- **X3c3** ✅
  - 检查：去掉光源
  - 通过标准：`lightRig` 为 `null` 后树上 `lightRig` 为 `null`。Excel 无修复
- **X3c4** ✅
  - 检查：图 op
  - 通过标准：对 3D id 发 `setPictureSize` 得 `InvalidId`，未写文件
  - 不强制开 Excel
- **X3c5** ✅
  - 检查：拒错
  - 通过标准：三个字段都不给报 `InvalidField`。`fov` 为负报 `InvalidField`。未写文件
  - 不强制开 Excel

## 工作表公式框

工作表 drawing 里的 `a14:m` AlternateContent。不是单元格 `f`。不进 `textBoxes`。不算公式排版引擎。
手测脚本 `manual-check/office/cases/xlsx-omml.ps1`。编号用 Xom。
页眉槽本期不插。

- **Xom1**
  - 检查：插入
  - 通过标准：`insertMath` 写入 `E=mc^2` 后，树上 `text` 对得上，不进 `textBoxes`，Excel 无修复
  - 包内 XML 含 `a14:m` 与 `Requires="a14"`
- **Xom2**
  - 检查：改字
  - 通过标准：`setMath` 换成 `a^2+b^2=c^2` 后 id 不变，`text` 为新字，Excel 无修复
- **Xom3**
  - 检查：create
  - 通过标准：`sheets[].maths[].text` 写入后树上 `text` 对得上，不进 `textBoxes`，Excel 无修复
- **Xom4**
  - 检查：删除
  - 通过标准：`deleteMath` 后 `mathCount` 为 0，Excel 无修复
- **Xom5**
  - 检查：备注无关
  - 通过标准：插入后 `textBoxCount` 仍为 0，公式框不进 `textBoxes`
- **Xom6**
  - 检查：拒错
  - 通过标准：出现 `expr` 或空 `text` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 工作表公式框事后几何

已有公式框能改锚点与尺寸。`setMath` 仍只改字。不进 `textBoxes`。不算公式排版引擎。
手测脚本 `manual-check/office/cases/xlsx-math-geom.ps1`。编号用 **Xmg**。不要和 Xm、Xom 撞。

- **Xmg1**
  - 检查：改尺寸
  - 通过标准：对 `oneCell` `setMathSize` 后树上 `cx` / `cy` 对得上。Excel 无修复
- **Xmg2**
  - 检查：改锚点
  - 通过标准：`setMathAnchor` 把 `from` 改到另一格后，树上 `from` 对得上。Excel 无修复
- **Xmg3**
  - 检查：twoCell
  - 通过标准：`anchor=twoCell` 且给 `to` 后，`setMathSize` 给 `to` 树上对得上。Excel 无修复
- **Xmg4**
  - 检查：改字不改几何；错 id
  - 通过标准：`setMath` 后位置尺寸仍是改过的值。对公式框 id 发 `setTextBoxAnchor` 得 `InvalidId`
- **Xmg5**
  - 检查：拒错
  - 通过标准：`twoCell` 上给 `cx` 得 `InvalidField`。对 xlsx 发 `setMathPos` 得 `InvalidField`。未写文件
  - 不强制开 Excel

## 工作表图表

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少有一张 DrawingML 柱形图，系列公式指向本表单元格。不要用库 `create` 产物当带图样例。
手测脚本建议 `manual-check/office/cases/xlsx-charts.ps1`。编号用 Xc。
Excel 16.0 插入的图通常是 `twoCellAnchor`；真 `oneCell` 由 `insertChart` 或 `setChartAnchor` 写入后再让 Excel 打开。

- **Xc1**
  - 检查：计数与名单
  - 通过标准：无 `charts` 时工作簿与目标表有 `chartCount`、没有 `charts`
  - `charts: true` 时 id 为 `sheet:…/chart:…`，`type` / 系列值与工作表一致
- **Xc2**
  - 检查：改系列
  - 通过标准：`setChartSeries` 后 Excel 无修复，单元格与图上的值都是新值
- **Xc3**
  - 检查：插图
  - 通过标准：`insertChart` 往 `worksheet` 插图，Excel 能看见且无修复
  - `dataFrom` 一带有数据表
- **Xc4**
  - 检查：删图
  - 通过标准：`deleteChart` 后该图消失，数据单元格还在，无修复
- **Xc5**
  - 检查：改类型
  - 通过标准：`setChartType` 改为折线后按折线显示（允许样式重置），无修复
- **Xc6**
  - 检查：尺寸或锚点
  - 通过标准：对 `oneCell` `setChartSize` 或 `setChartAnchor` 后位置或尺寸对上，无修复
- **Xc7**
  - 检查：导出
  - 通过标准：对 xlsx 发 `exportChartData` 得到 `InvalidField`
  - 不强制开 Excel

## 图表剩余

手测脚本 `manual-check/office/cases/xlsx-charts-remain.ps1`。编号用 Xr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xr1 | 趋势线 | ✅ `insertTrendline` 后 Excel 无修复，COM 能看见趋势线 |
| Xr2 | 次轴 | ✅ `setChartSeriesAxis` 后 `axis` 为 `secondary`，无修复 |
| Xr3 | 命名区域 | ✅ 系列 `c:f` 为定义名时 `setChartSeries` 写入对应格子，无修复 |
| Xr4 | 不连续 | ✅ 并集引用长度对得上时改数成功；三维引用仍 `NotImplemented` |

## 图表稀有类型

手测脚本 `manual-check/office/cases/xlsx-charts-rare.ps1`。编号用 Xq。

- **Xq1**
  - 检查：插入股价
  - 通过标准：✅ `insertChart` `type=stock` 三条系列后 Excel 无修复，`type` 为 `stock`
- **Xq2**
  - 检查：插入曲面
  - 通过标准：✅ `insertChart` `type=surface` 三条系列后 Excel 无修复，`type` 为 `surface`
- **Xq3**
  - 检查：插入复合饼
  - 通过标准：✅ `insertChart` `type=ofPie` 一条系列后 Excel 无修复
  - `ofPieType=bar` 为复合条
- **Xq4**
  - 检查：改类型
  - 通过标准：✅ 股价三条转 `col` 成功
  - 复合饼转 `pie` 成功
  - 无修复
- **Xq5**
  - 检查：仍拒绝
  - 通过标准：✅ 对股价 `addChartSeries` 得 `NotImplemented`
  - 源里已有 chartex 时 `setChartType` 得 `NotImplemented`
  - 源无 chartex 时 `insertChart type=chartex` 得 `InvalidField`
  - 两条系列的股价 `insertChart` 得 `InvalidField`
  - 均未写坏文件

## 图表稀有剩余

OHLC、线框 / 俯视曲面、自定义误差线、chartex 写入。脚本 `manual-check/office/cases/xlsx-charts-rare-remain.ps1`。编号用 Xcx。不要和 Xq、Xc 混。

- **Xcx1**
  - 检查：OHLC
  - 通过标准：✅ `insertChart` `type=stock` 四条系列后 Excel 无修复，`type` 为 `stock`，`stockKind` 为 `ohlc`
- **Xcx2**
  - 检查：俯视线框
  - 通过标准：✅ `insertChart` `type=surface` `surfaceKind=contourWire` 三条系列后无修复，`surfaceKind` 为 `contourWire`
- **Xcx3**
  - 检查：自定义误差线
  - 通过标准：✅ 柱形图 `insertErrorBars` `valType=cust` 后无修复，系列上 `plus` / `minus` 对得上
- **Xcx4**
  - 检查：chartex funnel
  - 通过标准：✅ `insertChart` `type=chartex` `layoutId=funnel` 后无修复，`type` 为 `chartex`，`layoutId` 为 `funnel`
- **Xcx5**
  - 检查：仍拒绝
  - 通过标准：✅ `type=chartex` 不带 `layoutId` 得 `InvalidField`
  - `layoutId=regionMap` 得 `NotImplemented`
  - 两条系列的股价仍 `InvalidField`
  - 均未写坏文件

## 页眉页脚

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少有奇数页页眉文字。不要用库 `create` 产物当唯一样例。
Excel 16.0 亲手插入的页眉图通常只有 VML `legacyDrawingHF`。DrawingML 页眉图由 `insertPicture` 往槽位写入后再让 Excel 打开。Xh5 仍只换 DrawingML。只有旧式 VML、没有 `drawingHF` 的查看、换图和删除见 Xh6 到 Xh11。
手测脚本建议 `manual-check/office/cases/xlsx-headerfooter.ps1`。编号用 Xh。

- **Xh1**
  - 检查：摘要与原文
  - 通过标准：无 `headerFooter` 时表上有 `hasHeaderFooter`、没有 `headerFooters`
  - `headerFooter: true` 时六个槽位都在，奇数页页眉 `text` 与 Excel 另存原文一致
- **Xh2**
  - 检查：改字
  - 通过标准：`setHeaderFooterText` 改奇数页页眉后 Excel 无修复，打印预览或 PageSetup 里是新字
- **Xh3**
  - 检查：工作表图不混
  - 通过标准：源文件若另有工作表图，`pictures: true` 且不开 `headerFooter` 时名单不含 `hf:`
- **Xh4**
  - 检查：插页眉图
  - 通过标准：对 `sheet:…/hf:odd:header` 发 `insertPicture` 后 Excel 无修复（允许页眉图在打印预览里才看见）
- **Xh5**
  - 检查：换页眉图
  - 通过标准：若 Xh4 写出了 DrawingML 页眉图，则 `replacePicture` 后无修复
  - 源文件只有 VML 页眉图时本条记跳过，不测换 VML
- **Xh6**
  - 检查：旧式页眉图名单
  - 通过标准：Excel 亲手插入、只有 `legacyDrawingHF` 的奇数页页眉图，`headerFooter` 与 `legacyPictures` 同时为真时，槽位上有 `legacyPic:CH`
  - 不进该槽 `pictures`，表上 `legacyPictureCount` 为 1
- **Xh7**
  - 检查：换旧式页眉图
  - 通过标准：`replaceLegacyPicture` 后 Excel 无修复，`exportMedia` 字节与新图一致，仍没有 `drawingHF`
- **Xh8**
  - 检查：删旧式页眉图
  - 通过标准：`deleteLegacyPicture` 后 Excel 无修复，页眉原文仍含 `&G`，图片文件名为空
- **Xh9**
  - 检查：奇数页左、中、右和页脚
  - 通过标准：Excel 亲手插入这六张旧式页眉图后，名单里有 `LH`、`CH`、`RH`、`LF`、`CF`、`RF`
  - `legacyPictureCount` 为 6，这些图不进 `pictures`
- **Xh10**
  - 检查：换一张后 Excel 另存
  - 通过标准：只换 `RH`。Excel 另存后无修复，没有 `drawingHF`
  - `RH` 的字节是新图，其余槽位仍是旧图
- **Xh11**
  - 检查：删页脚旧式图
  - 通过标准：`deleteLegacyPicture` 删 `LF` 后 Excel 无修复，左页脚原文仍含 `&G`，图片文件名为空

## 图表工作表

准备一份 Excel 亲手另存、至少有一张 `worksheet` 的工作簿；手测里再用 `insertChartSheet` 加一张。源文件不要用库 `create` 产物当唯一样例。
脚本建议 `manual-check/office/cases/xlsx-chartsheet.ps1`。编号用 Xs。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xs1 | 插入 | `insertChartSheet` 后表名单多一张 `kind: chartsheet`，`chartCount` 为 1，Excel 无修复 |
| Xs2 | 数据 | 点名的普通表 `dataFrom` 一带有数据表，图上的值对得上 |
| Xs3 | 删除 | `deleteChartSheet` 后该表消失，数据单元格还在，无修复 |
| Xs4 | 拒普通表 | 对 `worksheet` 发 `deleteChartSheet` 得到 `InvalidId`。不强制开 Excel |

## 文档属性

与 docx 同一套字段。脚本 `manual-check/office/cases/xlsx-docprops.ps1`。编号用 Xd（不要跟页眉图的 Xp 组混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xd1 | inspect | 无 scope 时根上有 `props`；create 产物 `creator` 为 `ooxp` |
| Xd2 | 改标题 | `setProps` 只改 `title` 后 Excel 文件属性标题是新字；未点名的公司名还在；无 Repair |
| Xd3 | 自定义 | `setCustomProp` 后树上 id 为 `prop:…`；Excel 自定义属性能看见 |
| Xd4 | 删除 | `deleteCustomProp` 后名单没有该项 |
| Xd5 | 拒错 id | 对单元格 id 发 `setProps` 得 `InvalidId`。不强制开 Excel |

## 工作表管理

准备一份 Excel 亲手另存、至少两张可见 `worksheet` 的工作簿。不要用库 `create` 产物当唯一样例。脚本 `manual-check/office/cases/xlsx-sheets.ps1`。编号用 Xm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xm1 | 隐藏字段 | 无 scope 时每张表有 `hidden`；可见表为 `false` |
| Xm2 | 插入 | `insertSheet` 后名单多一张空 `worksheet`，Excel 无修复 |
| Xm3 | 改名 | `renameSheet` 后旧 id 失效、新 id 能 `setCell`，Excel 无修复 |
| Xm4 | 重排 | `moveSheet` 且 `after` 省略后该表排第一，Excel 无修复 |
| Xm5 | 隐藏 | `setSheetHidden` 后 COM 对应表 Hidden，无修复 |
| Xm6 | 删除 | `deleteSheet` 后该表消失，无修复 |
| Xm7 | 最后一张可见表 | 隐藏或删除最后一张可见表得 `InvalidField`，未写文件。不强制开 Excel |

## 对话框表与宏表

inspect 只读进树。create 不生成。apply 不新开编辑。脚本 `manual-check/office/cases/xlsx-dialog-macro.ps1`。编号用 **Xdm**。不要和 Xm、Xd 撞。宏表用 ZIP 注入，不强制 Excel 打开注入样例。

- **Xdm1**
  - 检查：inspect 对话框
  - 通过标准：COM 另存 `.xlsx` 后，对应表 `kind=dialogsheet`
  - 字段为 `id`、`name`、`kind`、`hidden`、`hasHeaderFooter`
  - 没有 `pictureCount` / `print` / `sheetProtection`
- **Xdm2**
  - 检查：Excel 打开对话框
  - 通过标准：同一份文件 Excel 无 Repair
  - COM 能看见该对话框表
- **Xdm3**
  - 检查：inspect 宏表
  - 通过标准：注入 `/xlMacrosheet` 与 `/xlIntlMacrosheet` 后，两张表 `kind=macrosheet`
  - 字段形状与 Xdm1 相同
  - 不强制开 Excel
- **Xdm4**
  - 检查：拒写
  - 通过标准：对对话框表发 `deleteSheet` / `renameSheet` / `setSheetHidden` 得 `NotImplemented`，未写文件
  - 不强制开 Excel
- **Xdm5**
  - 检查：create 与单元格切片
  - 通过标准：create `kind: dialogsheet` 得 `InvalidField`
  - 点名对话框表做单元格切片得 `InvalidField`
  - 不强制开 Excel

## 单元格样式

准备一份 Excel 亲手另存或库 create 的工作簿，至少 A1 有数字、B1 有独立样式或未点名格子。脚本 `manual-check/office/cases/xlsx-cellstyle.ps1`。编号用 Xs。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xs1 | inspect | 范围内格子有 `numFmt`；create 默认格为 `General` |
| Xs2 | 数字格式与填充 | 给 A1 `setCellStyle` 写 `0.00` 与黄底后，Excel 无修复，`NumberFormat` 为 `0.00`，填充为黄，值还在 |
| Xs3 | 未点名 | B1 未出现在 ops 里，打开后样式与打补丁前观感一致 |
| Xs4 | 与 setCell 同批 | 同一 `ops` 先 `setCell` 再 `setCellStyle`（或反过来），值与格式都在，无修复 |
| Xs5 | 缺字段 | 只给 `op` 与 `id` 得 `InvalidField`，未写文件。不强制开 Excel |

## 命名单元格样式

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-named-style.ps1`。编号用 **Xns**。不要和 Xs、Xcs 撞。不改主题。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xns1 | inspect | 无 scope 时根上有 `cellStyleCount`；create 默认为 `1`。给了 `scope.cellStyles` 时名单含 `Normal`，`builtinId` 为 `0` |
| Xns2 | 插入 | `insertCellStyle` 写黄底加粗后，Excel 无修复，COM `Styles` 能看见该名，inspect `cellStyleCount` 为 `2` |
| Xns3 | 套用 | `setCellNamedStyle` 后 A1 的 COM `Style.Name` 对得上，填充为黄，值还在，无修复 |
| Xns4 | 删除 | `deleteCellStyle` 后名单不再含该自定义名；A1 格式仍在；对 `Normal` 发删除得 `InvalidField`。无修复 |
| Xns5 | 缺字段 | 只给 `op` 与 `id` 的 `insertCellStyle` 得 `InvalidField`，未写文件。不强制开 Excel |

## 列宽行高与合并

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-layout.ps1`。编号用 Xl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xl1 | inspect | 有单元格范围时表上有 `cols` / `rows` / `merges`；create 默认均为空数组 |
| Xl2 | 列宽 | `setColWidth` 把 A 列写成 20 后，Excel 无修复，COM `ColumnWidth` 约为 20 |
| Xl3 | 行高 | `setRowHeight` 把第 1 行写成 30 磅后，COM `RowHeight` 约为 30，无修复 |
| Xl4 | 合并与取消 | `mergeRange` `A1:B2` 后左上角值还在、其余格空、COM 认合并；再 `unmergeRange` 后不再合并，无修复 |
| Xl5 | 缺字段 | 只给 `op` 与 `id` 的 `setColWidth` 得 `InvalidField`，未写文件。不强制开 Excel |

## 公式结构

用库 create 产物再 apply 写出共享组与数组。脚本 `manual-check/office/cases/xlsx-formula.ps1`。编号用 Xf。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xf1 | inspect | 共享主格有 `formula`、`shared`、`sharedRef`；从属格 `formula` 为展开后的 A1，且 `shared` 为 `true` |
| Xf2 | 改主格公式 | 保住共享组，Excel 无修复，从属格打开后重算正确 |
| Xf3 | 拆组 | `shared: false` 后各格都是独立公式，Excel 无修复 |
| Xf4 | 数组公式 | `arrayRef` 写入后 Excel 认数组、无修复 |
| Xf5 | 数组从属格 | 对非主格 `setCell` 得 `NotImplemented`，未写文件。不强制开 Excel |

## 逻辑函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-logic.ps1`。编号用 Xlg。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xlg1 | 短路 | `IF(TRUE,1,1/0)` 为 1，`IF(FALSE,1/0,2)` 为 2，`IFERROR` 只在第一支出错时取第二支。Excel 无修复，与 inspect 缓存一致 |
| Xlg2 | 与或非 | `AND(FALSE,1/0)` 与 `OR(TRUE,1/0)` 为 `#DIV/0!`。文本和空格子按样本放下。Excel 无修复 |
| Xlg3 | 不认的名字 | `ECMA.IF(...)` 的 inspect 没有缓存。Excel 打开后显示 `#NAME?`，无修复 |

## 信息函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-info.ps1`。编号用 Xinf。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xinf1 | `IS` 与 `N` | 空单元格、隐式交集、`ISREF` 的三维引用、`ERROR.TYPE` 与库缓存一致。Excel 无修复 |
| Xinf2 | `CELL` | `address`、`col`、`row`、`contents`、`type` 与库缓存一致。Excel 无修复 |
| Xinf3 | `INFO` | `system`、`release`、`numfile`、`origin` 与库缓存一致。`recalc` 的缓存是 `Automatic`，中文 Excel 显示「自动」。`ECMA.ISBLANK` 没有缓存，Excel 显示 `#NAME?` |

## 数学与三角函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-math.ps1`。编号用 Xmh。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xmh1 | 舍入 | `CEILING`、`ISO.CEILING`、`FLOOR`、`ROUND`、`EVEN`、`MROUND` 与库缓存一致。Excel 无修复 |
| Xmh2 | 求和 | `SUMIF`、隐藏行上的 `SUBTOTAL`、嵌套 `SUBTOTAL`、`PRODUCT`、`ROMAN` 与库缓存一致。Excel 无修复 |
| Xmh3 | 其余 | `MMULT` 左上角、`ACOS`、`PI`、`MOD` 的除零与库缓存一致。`RAND` 两边都在 0 到 1 之间，不要求同一个数。`ECMA.SIN` 没有缓存，Excel 显示 `#NAME?` |

## 文本函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-text.ps1`。编号用 Xtx。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xtx1 | 截取 | `LEFT`、`RIGHT`、`MID` 与字节版和库缓存一致。拆开的汉字换成空格。Excel 无修复 |
| Xtx2 | 查找与变换 | `FIND`、`SEARCH`、`TRIM`、`CLEAN`、`CODE`、`CHAR` 与库缓存一致。`FIND` 大小写不配时是 `#VALUE!`。Excel 无修复 |
| Xtx3 | 格式 | `TEXT`、`DOLLAR`、`FIXED`、`BAHTTEXT`、`ASC`、`PHONETIC` 与库缓存一致。`ECMA.LEFT` 没有缓存，Excel 显示 `#NAME?` |

## 日期与时间函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-date.ps1`。编号用 Xdt。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xdt1 | 闰日 | `DATE(1900,2,28)` 是 59，假闰日是 60，`DATE(1900,3,1)` 是 61。`YEAR` 与库缓存一致。Excel 无修复 |
| Xdt2 | 推移 | `EDATE`、`WORKDAY`、假期、`DATEDIF` 的 `MD` 与库缓存一致。Excel 无修复 |
| Xdt3 | 间隔 | `DAYS360`、`YEARFRAC` 与库缓存一致。`TODAY` 与 Excel 当天的序列一致。`ECMA.YEAR` 没有缓存，Excel 显示 `#NAME?` |

## 统计函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-stat.ps1`。编号用 Xst。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xst1 | 计数 | `COUNT`、`COUNTA`、`COUNTBLANK`、`COUNTIF` 的通配符、`AVERAGE` 与 `AVERAGEA` 和库缓存一致。Excel 无修复 |
| Xst2 | 离散与分布 | `STDEV`、`PERCENTRANK`、`COVAR`、`NORMDIST`、`CHIDIST`、`MODE` 与库缓存一致。Excel 无修复 |
| Xst3 | 错误 | 没有重复值的 `MODE` 是 `#N/A`。`LARGE` 位次超出是 `#NUM!`。`ECMA.AVERAGE` 没有缓存，Excel 显示 `#NAME?` |

## 查找与引用函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-lookup.ps1`。编号用 Xlk。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xlk1 | 查找 | `VLOOKUP` 精确与近似、`MATCH`、`INDEX`、`CHOOSE` 与库缓存一致。Excel 无修复 |
| Xlk2 | 位置 | `ADDRESS`、`OFFSET` 的求和、`INDIRECT`、`COLUMN` 和数组公式里的 `TRANSPOSE` 与库缓存一致。Excel 无修复 |
| Xlk3 | 错误 | 找不到是 `#N/A`。移出工作表是 `#REF!`。`GETPIVOTDATA` 对上甲是 15，对不上是 `#REF!`。`RTD` 是 `#N/A`。`ECMA.VLOOKUP` 没有缓存，Excel 显示 `#NAME?` |

## 数据库函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-db.ps1`。编号用 Xdb。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xdb1 | 聚合 | `DSUM`、`DCOUNT`、`DAVERAGE`、`DMAX` 与库缓存一致。Excel 无修复 |
| Xdb2 | 条件 | 并且、或者、通配符和恰好一条的 `DGET` 与库缓存一致。Excel 无修复 |
| Xdb3 | 错误 | 多条 `DGET` 是 `#NUM!`。没有记录是 `#VALUE!`。没有数字的平均是 `#DIV/0!`。`ECMA.DSUM` 没有缓存，Excel 显示 `#NAME?` |

## 财务函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-fin.ps1`。编号用 Xfn。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xfn1 | 借贷 | `PV`、`PMT`、`RATE`、`IRR` 与库缓存一致。Excel 无修复 |
| Xfn2 | 债券 | `PRICE`、`SLN`、`DOLLARDE`、`COUPNUM` 与库缓存一致。Excel 无修复 |
| Xfn3 | 错误 | 期次颠倒的 `CUMIPMT` 是 `#NUM!`。寿命为 0 的 `SLN` 是 `#DIV/0!`。`ECMA.PV` 没有缓存，Excel 显示 `#NAME?` |

## 工程函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-eng.ps1`。编号用 Xeg。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xeg1 | 数值 | 贝塞尔、`ERF`、`CONVERT`、`DELTA`、`GESTEP` 与库缓存一致。Excel 无修复 |
| Xeg2 | 文本 | `COMPLEX`、补码、`IMDIV`、`IMSQRT`、`IMCOS` 与库缓存一致。Excel 无修复 |
| Xeg3 | 错误 | 溢出和非法数字是 `#NUM!`。未知单位是 `#N/A`。虚部后缀不一致是 `#VALUE!`。`ECMA.DEC2BIN` 没有缓存，Excel 显示 `#NAME?` |

## 多维数据集函数

用库 create 写出公式缓存，再交给 Excel 16.0 重算。脚本 `manual-check/office/cases/xlsx-cube.ps1`。编号用 Xcu。不连接立方体。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xcu1 | 无连接 | 七个函数按样本参数个数写入 `#N/A`，与 Excel 一致。Excel 无修复 |
| Xcu2 | 不求值 | `ECMA.CUBEVALUE` 没有缓存，Excel 显示 `#NAME?`。少一个参数的 `CUBEMEMBER` 不放进这本工作簿 |

## 窗体控件

用 Excel 16.0 查看、修改、插入和删除按钮、复选框、下拉。脚本 `manual-check/office/cases/xlsx-form-control.ps1`。编号用 Xfc。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xfc1 | 查看 | 三个控件都在 `formControls` 里。按钮和复选框有标题，下拉没有。链接和列表与放入时一致 |
| Xfc2 | 修改 | 改标题、链接、勾选和选中项后，Excel 16.0 打开没有 Repair，读到的是改后的值 |
| Xfc3 | 另存 | 另存后再 inspect，标题仍是改后的句子，drawing 与 VML 两处都有这句 |
| Xfc4 | 插入 | 库插入三种控件后，Excel 16.0 打开没有 Repair，读到标题、链接、勾选和选中项 |
| Xfc5 | 删除 | 删掉按钮后再打开，按钮没有了，复选框和下拉还在，没有 Repair |

## 共享公式改主格

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-formula-master.ps1`。编号用 Xsm。不要占用 Xf。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsm1 | 向上迁主格 | 把 `B2:B3` 迁到 `B1:B3` 后，inspect 上 `B1` 有 `sharedRef`，`B2` / `B3` 是从属格且公式为展开后的 A1 |
| Xsm2 | Excel 重算 | 同一份文件 Excel 无修复，COM `B1`/`B2`/`B3` 公式与打开后的值正确 |
| Xsm3 | 向下收主格 | 把 `B1:B3` 收到 `B2:B3` 后，`B2` 是主格，`B1` 是独立公式；Excel 无修复 |
| Xsm4 | 两组相交 | `sharedRef` 同时盖住两组已有共享组得 `NotImplemented`，未写文件。不强制开 Excel |
| Xsm5 | 从属格缺范围 | 对从属格只给 `shared: true` 得 `InvalidField`，未写文件。不强制开 Excel |

## 数据验证

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-validation.ps1`。编号用 Xv。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xv1 | inspect | 有单元格范围时表上有 `dataValidationCount` 与 `dataValidations`；create 默认为 `0` 与空数组 |
| Xv2 | 序列下拉 | `insertDataValidation` 给 A1 写 `type=list`、`formula1=1,2,3` 后，Excel 无修复，该格下拉能看见 1、2、3 |
| Xv3 | 多区域 | `sqref` 为 `A1 C1` 时 inspect 的 `sqref` 为 `["A1","C1"]`，两条都能看见下拉 |
| Xv4 | 删除 | `deleteDataValidation` 后该格不再有校验，Excel 无修复 |
| Xv5 | 缺字段 | 只给 `op` 与 `id` 的 `insertDataValidation` 得 `InvalidField`，未写文件。不强制开 Excel |

## 条件格式

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-conditional-format.ps1`。编号用 Xn。

- **Xn1**
  - 检查：inspect
  - 通过标准：有单元格范围时表上有 `conditionalFormatCount` 与 `conditionalFormats`
  - create 默认为 `0` 与空数组
- **Xn2**
  - 检查：单元格值填绿
  - 通过标准：`insertConditionalFormat` 给 `A1:A10` 写 `type=cellIs`、`greaterThan`、`formulas=["0"]`、绿底后，Excel 无修复
  - A1 为 1 时能看见绿色条件格式
- **Xn3**
  - 检查：色阶
  - 通过标准：写入两色 `colorScale` 后 Excel 认色阶、无修复
- **Xn4**
  - 检查：删除
  - 通过标准：`deleteConditionalFormat` 后该范围不再有条件格式，Excel 无修复
- **Xn5**
  - 检查：缺字段
  - 通过标准：只给 `op` 与 `id` 的 `insertConditionalFormat` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 条件格式与迷你图字段

`top10` / `aboveAverage` / 文本类规则，迷你图颜色与标记 / 轴，单元格对角线与缩进 / 旋转。脚本 `manual-check/office/cases/xlsx-cf-spark-style.ps1`。编号用 **Xfs**。不要和 Xn、Xsk、Xs 撞。不求值、不渲染色阶。

- **Xfs1**
  - 检查：top10
  - 通过标准：✅ `insertConditionalFormat` `type=top10` `rank=3` 后 Excel 无修复，inspect `rank` 为 3
- **Xfs2**
  - 检查：低于平均与包含文本
  - 通过标准：✅ `aboveAverage` 且 `aboveAverage: false` 后无修复；`containsText` `text=foo` 后无修复，树上有生成的 `formulas`
- **Xfs3**
  - 检查：迷你图标记与轴
  - 通过标准：✅ `insertSparkline` 带 `markers` 与 `minAxisType=custom` `manualMin=-2` 后无修复，COM 能看见迷你图
- **Xfs4**
  - 检查：对角线与缩进旋转
  - 通过标准：✅ `setCellStyle` 写对角线、`indent=2`、`rotation=45` 后无修复
- **Xfs5**
  - 检查：仍拒绝
  - 通过标准：✅ `containsText` 不带 `text` 得 `InvalidField`
  - `top10` 带 `operator` 得 `InvalidField`
  - `minAxisType=custom` 不带 `manualMin` 得 `InvalidField`
  - 均未写坏文件

## 工作表表格

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-table.ps1`。编号用 Xt。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xt1 | inspect | 有单元格范围时表上有 `tableCount` 与 `tables`；create 默认为 `0` 与空数组 |
| Xt2 | 建成表 | `insertSheetTable` 给 `A1:C3` 写列名后，Excel 无修复；COM `ListObjects` 能看见表，列名对 |
| Xt3 | 改范围 | `setSheetTable` 把 `ref` 改到 `A1:C4` 后 Excel 仍认该表、无修复 |
| Xt4 | 删除 | `deleteSheetTable` 后数据还在、不再有表，Excel 无修复 |
| Xt5 | 缺字段 | 只给 `op` 与 `id` 的 `insertSheetTable` 得 `InvalidField`，未写文件。不强制开 Excel |

## 数据透视

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-pivot.ps1`。编号用 Xpt（与工作表图的 Xp 分开）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xpt1 | inspect | 有单元格范围时表上有 `pivotCount` 与 `pivots`；create 默认为 `0` 与空数组 |
| Xpt2 | 建成表 | `insertPivotTable` 源 `A1:C4`、location `E3`、一行字段加一个值字段后，Excel 无修复；COM `PivotTables.Count>=1` |
| Xpt3 | 改源 | `setPivotTable` 把源改到含新数据行的范围后，Excel 仍认该表、无修复 |
| Xpt4 | 删除 | `deletePivotTable` 后 `pivotCount=0`、COM Count=0、源数据还在、无修复 |
| Xpt5 | 缺字段 | 只给 `op` 与 `id` 的 `insertPivotTable` 得 `InvalidField`，未写文件。不强制开 Excel |

## 命名区域

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-defined-name.ps1`。编号用 Xnm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xnm1 | inspect | 根上有 `definedNameCount` 与 `definedNames`；create 默认为 `0` 与空数组 |
| Xnm2 | 建成名 | `insertDefinedName` 名 `Sales`、公式 `Sheet1!$A$1:$A$10` 后，Excel 无修复；COM `Names` 能看见 `Sales` |
| Xnm3 | 改公式 | `setDefinedName` 把公式改到 `Sheet1!$A$1:$A$20` 后 Excel 仍认该名、无修复 |
| Xnm4 | 删除 | `deleteDefinedName` 后 `definedNameCount=0`、COM 找不到 `Sales`、无修复 |
| Xnm5 | 缺字段 | 只给 `op` 与 `id` 的 `insertDefinedName` 得 `InvalidField`，未写文件。不强制开 Excel |

## 打印设置

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-print.ps1`。编号用 Xps。

- **Xps1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `print`
  - create 默认为左边距 0.7、`orientation` 为 `portrait`、`printArea` 为 `null`
- **Xps2**
  - 检查：边距与纸向
  - 通过标准：先写奇数页页眉，再 `setPrintSetup` 左边距 1 英寸、`landscape` 后，Excel 无修复
  - COM `LeftMargin` 与 `InchesToPoints(1)` 一致，`Orientation` 为横向
  - 页眉字还在
- **Xps3**
  - 检查：打印区域
  - 通过标准：`printArea` 写成 `A1:A10` 后 COM `PrintArea` 能对上，无修复
- **Xps4**
  - 检查：网格线
  - 通过标准：`gridLines: true` 后 COM `PrintGridlines` 为真，无修复
- **Xps5**
  - 检查：缺字段
  - 通过标准：只给 `op` 与 `id` 的 `setPrintSetup` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 视图与筛选

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-view.ps1`。编号用 Xvw。

- **Xvw1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `view` 与 `autoFilter`
  - create 默认为 `zoom` 100、冻结 0、`activeCell` 为 `A1`、`autoFilter` 为 `null`
- **Xvw2**
  - 检查：冻结与缩放
  - 通过标准：`setSheetView` 冻结首行、`zoom` 80 后，Excel 无修复
  - COM `FreezePanes` 为真、`SplitRow` 为 1、`Zoom` 为 80
- **Xvw3**
  - 检查：工作表筛选
  - 通过标准：`setAutoFilter` 把 `ref` 写成 `A1:A10` 后 COM `AutoFilter.Range` 能对上，无修复
- **Xvw4**
  - 检查：解冻
  - 通过标准：`frozenRows` 与 `frozenCols` 都为 0 后 COM `FreezePanes` 为假，无修复
- **Xvw5**
  - 检查：缺字段
  - 通过标准：只给 `op` 与 `id` 的 `setSheetView` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 排序

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-sort.ps1`。编号用 **Xsr**。不要和 Xvw 撞。不重排格子，不算排序引擎。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsr1 | inspect | 每张 `worksheet` 始终有 `sortState`；create 默认为 `null` |
| Xsr2 | 写入 | `setSortState` `ref=A1:A3`、`columns[0].descending=true` 后 inspect 对上；Excel 无修复；A1 仍为 3（未重排） |
| Xsr3 | COM | COM `Sort.SortFields` 能看见对应列，降序；无修复 |
| Xsr4 | 删除 | `ref: null` 后 `sortState` 为 `null`，无修复 |
| Xsr5 | 拒写 | 缺 `ref`、列范围越出 `ref` 得 `InvalidField`，未写文件。不强制开 Excel |

## 隐藏行列与排序字段

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-hidden-sort.ps1`。编号用 **Xhd**。不要和 Xh、Xhl、Xsr、Xon 撞。不按值重排格子。不算排序引擎。

- **Xhd1**
  - 检查：inspect 隐藏
  - 通过标准：`setRowHidden` 行 3、`setColHidden` 列 B 后
  - `outline.rows` 含 `r=3`、`hidden=true`、`level=0`
  - `outline.cols` 含 `min=B`、`hidden=true`、`level=0`
- **Xhd2**
  - 检查：Excel 打开隐藏
  - 通过标准：无 Repair
  - COM 第 3 行与 B 列 `Hidden` 为真
- **Xhd3**
  - 检查：自定义序列
  - 通过标准：`customList=Low,Medium,High` 后 inspect 对上
  - A1 仍为 `Low`（未重排）
  - Excel 无 Repair
- **Xhd4**
  - 检查：颜色与笔画
  - 通过标准：`sortBy=cellColor` 且 `sortMethod=stroke` 后 inspect 对上
  - Excel 无 Repair
- **Xhd5**
  - 检查：拒写
  - 通过标准：`cellColor` 不给 `color`、`icon` 带 `customList`、`id` 为 `slide:1` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 批注

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-comments.ps1`。编号用 Xcm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xcm1 | inspect | 每张 `worksheet` 始终有 `commentCount`；create 默认为 `0`；给了 `scope.comments` 时名单为空数组，不依赖单元格切片 |
| Xcm2 | 插入 | `insertComment` 写到 `A1` 后 Excel 无修复；COM `CommentThreaded` 能看见正文 |
| Xcm3 | 回复 | `replyComment` 后 COM `Replies` 能看见；inspect 嵌在 `replies` |
| Xcm4 | 删除 | `deleteComment` 后 `commentCount=0`、COM 没有线程批注、无修复 |
| Xcm5 | 缺字段 | 只给 `op` 与 `id` 的 `setCommentMeta` 得 `InvalidField`，未写文件。不强制开 Excel |

## 形状与 SmartArt

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-shapes.ps1`。编号用 Xsa。

- **Xsa1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `textBoxCount` 与 `smartArtCount`
  - create 默认为 `0`
  - 给了 `scope.textBoxes` / `scope.smartArt` 时名单为空数组，不依赖单元格切片
- **Xsa2**
  - 检查：文本框
  - 通过标准：`insertTextBox` 写到 `B2` 后 Excel 无修复
  - COM `Shapes` 能看见文本框
  - `setShapeText` 后可见字对得上
- **Xsa3**
  - 检查：SmartArt
  - 通过标准：`insertSmartArt` `layout=list` 后 Excel 无修复
  - COM 能看见 SmartArt
  - inspect `layout` 对得上
- **Xsa4**
  - 检查：删除
  - 通过标准：`deleteTextBox` / `deleteSmartArt` 后对应计数为 0、无修复
- **Xsa5**
  - 检查：缺字段
  - 通过标准：只给 `op` 与 `id` 的 `insertTextBox`（缺 `from`）得 `InvalidField`，未写文件
  - 不强制开 Excel

## 形状文字格式

工作表文本框 / 带 `txBody` 的形状的 DrawingML 文字格式。脚本 `manual-check/office/cases/xlsx-shape-format.ps1`。编号用 **Xtf**。不要和 Xsa、Xsg 撞。xlsx 的 `anchor` 仍是单元格锚点，垂直对齐只用 `textAnchor`。

- **Xtf1**
  - 检查：插入带格式
  - 通过标准：✅ `insertTextBox` 带 `sz=2400` `color=0000FF` 后 inspect 对得上
  - Excel 无修复
- **Xtf2**
  - 检查：改格式
  - 通过标准：✅ `setShapeFormat` 后 `sz=1800`、`color=C41E3A`、`bold=true`
- **Xtf3**
  - 检查：COM
  - 通过标准：✅ COM `Font.Size` 为 18 磅，`Color.RGB` 为 OLE 色 `C41E3A`
  - 无修复
- **Xtf4**
  - 检查：create
  - 通过标准：✅ create `textBoxes[]` 带 `sz=2200` 后 inspect 对得上
  - 无修复
- **Xtf5**
  - 检查：缺字段
  - 通过标准：✅ 只给 `op` 与 `id` 的 `setShapeFormat` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 形状行距与字距

工作表文本框 / 带 `txBody` 的形状的段落间距、字符间距与 kern。`textAnchor` 补全 `just` / `dist`。不算排版引擎。不改 Word。脚本 `manual-check/office/cases/xlsx-shape-spacing.ps1`。编号用 **Xln**。不要和 Xtf、Xfl 撞。

- **Xln1**
  - 检查：插入带行距
  - 通过标准：✅ `insertTextBox` 带 `lnSpc` pts 2400 后 inspect 对得上
  - Excel 无修复
- **Xln2**
  - 检查：改段前与字距
  - 通过标准：✅ `setShapeFormat` 后 `spcBef` pts 1200、`spc` 200、`kern` 1200 对得上
- **Xln3**
  - 检查：COM
  - 通过标准：✅ COM `SpaceBefore` 约为 12 磅，`Font.Spacing` 约为 2 磅
  - 无修复
- **Xln4**
  - 检查：锚点
  - 通过标准：✅ `textAnchor` 为 `just` 后 inspect 对得上，无修复
- **Xln5**
  - 检查：拒错
  - 通过标准：✅ `lnSpc.type` 不是 `pts` / `pct` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 字填充与轮廓

工作表文本框的渐变字、字轮廓与混排 `runs`。不算排版引擎。不改 Word。脚本 `manual-check/office/cases/xlsx-text-fill.ps1`。编号用 **Xfl**。不要和 Xtf、Xsa 撞。

- **Xfl1**
  - 检查：渐变字
  - 通过标准：✅ `setShapeFormat` 给线性两停 `textFill` 后 inspect 对得上
  - ZIP 有 `gradFill flip="none" rotWithShape="1"`
  - Excel 无修复
- **Xfl2**
  - 检查：字轮廓
  - 通过标准：✅ `textOutline` `width=19050` 后 inspect 对得上
  - COM `Font.Line.Visible` 为真
  - 无修复
- **Xfl3**
  - 检查：无填充
  - 通过标准：✅ `textFill.type=none` 后 `color` 为 `null`
- **Xfl4**
  - 检查：混排 runs
  - 通过标准：✅ 同一框两种字号后 `runs` 两条 `sz` 为 2800 / 1200
- **Xfl5**
  - 检查：拒错
  - 通过标准：✅ 同时给 `color` 与 `textFill` 得 `InvalidField`，未写文件
  - 不强制开 Excel

## 无 txBox 的形状与连接线

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-shapes-geom.ps1`。编号用 Xsg。不要和 Xsa 撞。

- **Xsg1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `shapeCount` 与 `connectorCount`
  - create 默认为 `0`
  - 给了 `scope.shapes` / `scope.connectors` 时名单为空数组，不依赖单元格切片
- **Xsg2**
  - 检查：矩形
  - 通过标准：`insertShape` 写到 `B2` 后 Excel 无修复
  - inspect `prst` 为 `rect`，id 为 `sheet:…/shape:…`
  - COM `Shapes` 能看见
  - 无 `txBox`
- **Xsg3**
  - 检查：连接线
  - 通过标准：`insertConnector` 后 inspect `connectors` 名单 id 为 `sheet:…/cxn:…`，`prst` 为 `straightConnector1`
  - COM 能看见连接线
  - 无修复
- **Xsg4**
  - 检查：改锚点与删
  - 通过标准：`setTextBoxSize` / `setTextBoxAnchor` 认形状 id
  - `setShapeText` 后可见字对得上
  - `deleteShape` / `deleteConnector` 后对应计数为 0、无修复
- **Xsg5**
  - 检查：拒写
  - 通过标准：只给 `op` 与 `id` 的 `insertShape`（缺 `from`）得 `InvalidField`
  - 对 `/sp:` 发 `deleteShape`、对 `/cxn:` 发 `setShapeText` 得 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 组合

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-groups.ps1`。编号用 **Xg**。不要和 Xsg、Xsa 撞。不算按列宽反推已有 `twoCell` 图的显示尺寸。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xg1 | 组合 | `insertShape` 两个矩形后 `groupShapes`；Excel 无修复；COM Type=6；inspect 子形状 id 仍在 |
| Xg2 | 解开 | `ungroup` 后组合解开，子 id 还在；无 Repair |
| Xg3 | 形状与连接线 | 形状与连接线编成一组后 COM 认组合；inspect 两边名单都在；无 Repair |
| Xg4 | 再编拒绝 | 已组合的成员再 `groupShapes` 得 `InvalidField`，未写文件。不强制开 Excel |
| Xg5 | 拒写 | 只给一个成员、`id` 为 `slide:1`、对未组合对象发 `ungroup` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## OLE

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-ole.ps1`。编号用 **Xol**。不要和 Xg、Xsg、Xsa、Xa、Xs、Xq、O 撞。不激活 OLE 服务器。不把嵌入字节或预览图塞进 inspect 树。

- **Xol1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `oleObjectCount`
  - create 默认为 `0`
  - 给了 `scope.oleObjects` 时名单为空数组，不依赖单元格切片
- **Xol2**
  - 检查：插入
  - 通过标准：`insertOle` 嵌入一份 xlsx 后 Excel 无修复
  - COM Type=7
  - inspect id 为 `sheet:Sheet1/ole:1025`，`progId` 为 `Worksheet`，dummy 不进 `shapes`
- **Xol3**
  - 检查：导出
  - 通过标准：`exportMedia` 字节与 `/xl/embeddings/…` 部件一致
- **Xol4**
  - 检查：删除
  - 通过标准：`deleteOle` 后 `oleObjectCount=0`、无 Repair
- **Xol5**
  - 检查：拒写
  - 通过标准：缺 `from`、`id` 为 `slide:1`、对形状 id 发 `insertOle` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel
- **Xol6**
  - 检查：事后改尺寸
  - 通过标准：✅ `twoCell` 上 `setOleSize` 给 `to` 后树上右下角对得上。给 `cx` 得 `InvalidField`
  - Excel 16.0 打开无修复
  - 不启动 OLE 服务器，不重算嵌入工作簿

## 切片器

用库 create 产物再 apply 出表格，再插切片器。脚本 `manual-check/office/cases/xlsx-slicers.ps1`。编号用 **Xsl**。不要和 Xs、Xsa、Xsk、Xol 撞。不执行筛选，不刷新透视。

- **Xsl1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `slicerCount`
  - create 默认为 `0`
  - 给了 `scope.slicers` 时名单为空数组，不依赖单元格切片
- **Xsl2**
  - 检查：插入
  - 通过标准：对表格 `insertSlicer` 后 Excel 无修复
  - COM 能看见切片器
  - inspect id 为 `sheet:Sheet1/slicer:{cNvPrId}`，`sourceKind=table`
- **Xsl3**
  - 检查：透视源
  - 通过标准：对透视表 `insertSlicer` 后无修复
  - inspect `sourceKind=pivot`
- **Xsl4**
  - 检查：删除
  - 通过标准：`deleteSlicer` 后 `slicerCount=0`、无 Repair
- **Xsl5**
  - 检查：拒写
  - 通过标准：缺 `from`、`id` 为 `slide:1`、对不存在的 `field` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 切片器锚点

已有切片器（含时间线）改外层锚点与 `oneCell` / `absolute` 尺寸。不改 cache，不执行筛选。
对切片器 id 发 `setPictureSize` / `setPictureAnchor` 仍报 `InvalidId`。
手测脚本 `manual-check/office/cases/xlsx-slicer-anchor.ps1`。编号用 **Xsn**。不要和 Xsl、Xsa、Xtl、Xsk 撞。

- **Xsn1**
  - 检查：改锚点
  - 通过标准：`setSlicerAnchor` 把 `from` 改到另一格后，树上 `from` 对得上。Excel 无修复
- **Xsn2**
  - 检查：twoCell 尺寸
  - 通过标准：`setSlicerSize` 给 `to` 后树上 `to` 对得上。Excel 无修复
- **Xsn3**
  - 检查：oneCell
  - 通过标准：`anchor=oneCell` 且给 `cx` / `cy` 后，树上 `anchor` 与尺寸对得上。Excel 无修复。COM 尺寸对得上（允许 2 磅误差）
- **Xsn4**
  - 检查：时间线
  - 通过标准：时间线 `setSlicerAnchor` 改 `from` 后树上对得上。Excel 无修复
- **Xsn5**
  - 检查：图 op
  - 通过标准：对切片器 id 发 `setPictureSize` 得 `InvalidId`，未写文件
  - 不强制开 Excel
- **Xsn6**
  - 检查：拒错
  - 通过标准：`twoCell` 上给 `cx` 得 `InvalidField`。对 xlsx 发 `setSlicerSize` 给图 id 得 `InvalidId`。未写文件
  - 不强制开 Excel

## 切片器视图

已有切片器（含时间线）改视图部件上的标题、行高、列数、显示开关与样式名。不改 drawing 锚点，不改 cache，不执行筛选。
手测脚本 `manual-check/office/cases/xlsx-slicer-caption.ps1`。编号用 **Xsc**。不要和 Xsl、Xsn、Xtl 撞。

- **Xsc1**
  - 检查：insert 标题
  - 通过标准：`insertSlicer` 带 `caption` 后树上 `caption` 对得上。缺省视图字段与 schema 缺省一致。Excel 无修复
- **Xsc2**
  - 检查：改视图
  - 通过标准：`setSlicerView` 改 `caption`、`columnCount`、`showCaption` 后树上对得上。Excel 无修复。COM `Caption` 对得上
- **Xsc3**
  - 检查：样式名
  - 通过标准：`style` 写成内置名后树上对得上。再给 `null` 则树上为 `null`。Excel 无修复
- **Xsc4**
  - 检查：时间线
  - 通过标准：时间线 `setSlicerView` 改 `caption` 与 `showHeader` 后树上对得上。Excel 无修复
- **Xsc5**
  - 检查：拒写
  - 通过标准：只给 `op` 与 `id` 得 `InvalidField`。对切片器给 `showHeader` 得 `InvalidField`。对图 id 发 `setSlicerView` 得 `InvalidId`。未写文件
  - 不强制开 Excel

## 工作簿主题

工作簿 theme / themeOverride 能 inspect，能改配色、字体、fmt、名字、脚本字体与补充色板。create 可写。不把格子上的 theme 指数算成 RGB。
手测脚本 `manual-check/office/cases/xlsx-theme.ps1`。编号用 **Xth**。不要和 Th、Ts、Xtf 撞。

- **Xth1** ✅
  - 检查：inspect
  - 通过标准：Excel 空白另存后根上有 `theme`，`colors.accent1` 为六位十六进制。无 Repair
- **Xth2** ✅
  - 检查：改配色
  - 通过标准：`setThemeColors` 写 `accent1=FF0000` 后树上对得上。Excel 无修复
- **Xth3** ✅
  - 检查：改字体与补充色板
  - 通过标准：`setThemeFonts` 写 `majorLatin=Arial` 后树上对得上。`setThemeExtraSchemes` 写入一份后树上对得上。Excel 无修复
- **Xth4** ✅
  - 检查：override 与 create
  - 通过标准：`setThemeOverride` 写 `accent1` 后树上不是 null。create 给了 `workbook.theme` 后树上对得上；省略则为 null。Excel 无修复
- **Xth5** ✅
  - 检查：拒错
  - 通过标准：无 theme 发 `setThemeColors` 得 `IdNotFound`。错 id 得 `InvalidId`。只给 `op` 与 `id` 得 `InvalidField`。未写文件
  - 不强制开 Excel

## 主题 objectDefaults

工作簿 theme 的 `a:objectDefaults`。不算主题排版引擎。不改 Word。
手测脚本 `manual-check/office/cases/xlsx-theme-defaults.ps1`。编号用 **Xdf**。不要和 Xth、Xtf、Th 撞。

- **Xdf1** ✅
  - 检查：inspect
  - 通过标准：create 给了 `workbook.theme` 后 `objectDefaults` 为 `null`。无 Repair
- **Xdf2** ✅
  - 检查：线宽
  - 通过标准：`setThemeObjectDefaults` 写 `sp.lnW=38100` 后树上对得上。Excel 无修复
- **Xdf3** ✅
  - 检查：样式
  - 通过标准：写 `sp.style.lnRef=1` `fontRef=minor` 后树上对得上。无 Repair
- **Xdf4** ✅
  - 检查：清除
  - 通过标准：`objectDefaults: null` 后为 `null`。无 Repair
- **Xdf5** ✅
  - 检查：拒错
  - 通过标准：错 id 得 `InvalidId`。缺字段、非法 `fontRef`、无 theme 得 `InvalidField` / `IdNotFound`。未写文件
  - 不强制开 Excel

## 切片器筛选项

切片器 cache 已勾选项、时间线 cache 的筛选区间能 inspect、能 apply。打开后 Excel 自己按包内标记显示。库不算筛选结果，不刷新透视合计。
手测脚本 `manual-check/office/cases/xlsx-slicer-filter.ps1`。编号用 **Xsi**。不要和 Xsl、Xsn、Xsc、Xtl 撞。

- **Xsi1** ✅
  - 检查：inspect 缺省
  - 通过标准：插入表格切片器后树上 `selected` 为 `[]`。插入透视切片器后 `selected` 含源里各项。时间线 `filter` 为 `null`。Excel 无修复
- **Xsi2** ✅
  - 检查：表格勾选
  - 通过标准：`setSlicerFilter` `selected=["乙"]` 后树上对得上。Excel 无修复。表格 autoFilter 含 `乙`
- **Xsi3** ✅
  - 检查：透视勾选
  - 通过标准：透视切片器 `selected=["乙"]` 后树上只有乙。Excel 无修复
- **Xsi4** ✅
  - 检查：时间线
  - 通过标准：时间线 `filter` 写成 `dateBetween` 与起止日后树上对得上。再给 `null` 则树上为 `null`。Excel 无修复
- **Xsi5** ✅
  - 检查：拒写
  - 通过标准：只给 `op` 与 `id` 得 `InvalidField`。点名不存在的项得 `InvalidField`。对图 id 发此 op 得 `InvalidId`。未写文件
  - 不强制开 Excel

## 时间线切片器

用库 create 产物再 apply 出带日期列的透视表，再插时间线。脚本 `manual-check/office/cases/xlsx-timelines.ps1`。编号用 **Xtl**。不要和 Xsl、Xs、Xsa、Xsk 撞。不执行筛选，不刷新透视。OLAP 不测。

- **Xtl1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `slicerCount`
  - create 默认为 `0`
  - 给了 `scope.slicers` 时名单为空数组
- **Xtl2**
  - 检查：插入
  - 通过标准：`insertSlicer` `kind=timeline` 后 Excel 无修复
  - COM `SlicerCaches` 能看见，类型为时间线
  - inspect `sourceKind=timeline`，id 为 `sheet:Sheet1/slicer:{cNvPrId}`
- **Xtl3**
  - 检查：部件
  - 通过标准：包内有 `/xl/timelines/` 与 `/xl/timelineCaches/`
  - 没有 slicers / slicerCaches 部件
  - drawing 含 `tsle:timeslicer`
- **Xtl4**
  - 检查：删除
  - 通过标准：`deleteSlicer` 后 `slicerCount=0`、无 Repair
  - 时间线部件去掉
- **Xtl5**
  - 检查：拒写
  - 通过标准：`kind=timeline` 却给表格源、缺 `from`、`id` 为 `slide:1` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 外部数据连接

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-connections.ps1`。编号用 **Xcn**。不要和 Xc、Xcm 撞。不刷新连接。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xcn1 | inspect | 工作簿始终有 `connectionCount`；create 默认为 `0`；给了 `scope.connections` 时名单为空数组，不依赖单元格切片 |
| Xcn2 | 网页 | `insertConnection` `type=web` 后 Excel 无修复；COM `Connections` 能看见；inspect `kind=web`、`url` 对得上 |
| Xcn3 | 文本 | `insertConnection` `type=text` 后无修复；inspect `kind=text`、`sourceFile` 对得上 |
| Xcn4 | 删除 | `deleteConnection` 后 `connectionCount=0`、无 Repair |
| Xcn5 | 拒写 | 缺 `type`、`id` 为 `slide:1`、`type` 不是 `web`/`text` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 外部引用

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-external-link.ps1`。编号用 **Xel**。不要和 Xl、Xcn 撞。不打开目标文件求值。

- **Xel1**
  - 检查：inspect
  - 通过标准：工作簿始终有 `externalLinkCount`
  - create 默认为 `0`
  - 给了 `scope.externalLinks` 时名单为空数组，不依赖单元格切片
- **Xel2**
  - 检查：插入
  - 通过标准：`insertExternalLink` 并写 `=[1]Sheet1!A1` 后 Excel 无修复
  - COM `LinkSources` 或 Formula 含源文件名
  - inspect `kind=book`、`path` 含源文件名
  - Excel 16.0 对未引用的 `externalReferences` 不填 `LinkSources`
- **Xel3**
  - 检查：公式占用
  - 通过标准：写了 `=[1]Sheet1!A1` 后再 `deleteExternalLink`，B1 留下缓存值 `0`，`externalLinkCount` 为 0
  - Excel `LinkSources` 少一条，没有 Repair
  - 删之前的公式含 `Sheet1` 与 `A1`
- **Xel4**
  - 检查：删除
  - 通过标准：清掉公式后再删，`externalLinkCount=0`、无 Repair
- **Xel5**
  - 检查：拒写
  - 通过标准：缺 `path`、`id` 为 `slide:1` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 外部引用路径与数据表公式

用库 create 产物再 apply；数据表先由 Excel 16.0 `Range.Table` 写入再 inspect / 改主格。
脚本 `manual-check/office/cases/xlsx-extpath-datatable.ps1`。编号用 **Xef**。不要和 Xel、Xf 撞。不打开目标文件求值。不求 TABLE 结果。

- **Xef1**
  - 检查：inspect 数据表
  - 通过标准：列输入表主格 `dataTable` 为真，`dataTableRef` 为 `B2:B4`，`formula` 为 `=TABLE(,C1)`
  - 从属格 `B3` 有 `dataTable` 且 `formula` 为 `null`
- **Xef2**
  - 检查：Excel 打开数据表
  - 通过标准：无 Repair
  - COM `B2` 公式为 `=TABLE(,C1)`
- **Xef3**
  - 检查：改路径
  - 通过标准：`setExternalLink` 后 Excel 无 Repair
  - COM `LinkSources` 含新文件名
  - inspect `path` 含新文件名
  - 单元格公式仍含 `[1]`
- **Xef4**
  - 检查：改主格 TABLE
  - 通过标准：主格改成 `=TABLE(,D1)` 后 inspect 与 COM 均为 `=TABLE(,D1)`
  - 无 Repair
  - XML `<f>` 无正文
- **Xef5**
  - 检查：拒写
  - 通过标准：对从属格 `setCell`、对主格写 `=A1`、`setExternalLink` 缺 `path`、`id` 为 `slide:1` 得 `NotImplemented` 或 `InvalidField` / `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 分组 / 分级显示

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-outline.ps1`。编号用 **Xon**。不要和 Xol、Xg 撞。不实现放映引擎。不算筛选。

- **Xon1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `outline`
  - create 默认为 `summaryBelow` / `summaryRight` 为 `true`，`rows` / `cols` 为空数组
  - 不依赖单元格切片
- **Xon2**
  - 检查：分组
  - 通过标准：`groupRows` `2:4` 与 `groupCols` `B:C` 后 Excel 无修复
  - COM 行 2–4 与列 B–C 的 `OutlineLevel` 为 2
  - inspect `level` 为 1
- **Xon3**
  - 检查：折叠
  - 通过标准：`setOutline` `collapseRows=2:4` `collapsed=true` 后无修复
  - COM 行 2–4 `Hidden` 为真
- **Xon4**
  - 检查：取消
  - 通过标准：`ungroupRows` / `ungroupCols` 后 inspect 名单空，无 Repair
- **Xon5**
  - 检查：拒写
  - 通过标准：只给 `op` 与 `id` 的 `setOutline`、`id` 为 `slide:1` 的 `groupRows` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 拆分窗格与自定义视图

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-custom-view.ps1`。编号用 **Xsv**。不要和 Xvw 撞。不算放映引擎。不按自定义视图恢复隐藏行 / 列。

- **Xsv1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 的 `view` 始终有 `splitRows` / `splitCols`，create 默认为 0
  - 工作簿始终有 `customViewCount`，create 默认为 0
  - 给了 `scope.customViews` 时名单为空数组，不依赖单元格切片
- **Xsv2**
  - 检查：拆分
  - 通过标准：`setSheetView` `splitRows=4` `splitCols=2` 后 Excel 无修复
  - COM `Split` 为真、`SplitRow` 为 4、`SplitColumn` 为 2、`FreezePanes` 为假
- **Xsv3**
  - 检查：插入
  - 通过标准：`insertCustomView` `name=DemoView` 后无修复
  - COM `CustomViews.Count` 为 1
  - inspect 名单 `name` 对得上
- **Xsv4**
  - 检查：删除
  - 通过标准：`deleteCustomView` 后 `customViewCount=0`、无 Repair
- **Xsv5**
  - 检查：拒写
  - 通过标准：冻结与拆分同条 op、空 `name`、`id` 为 `slide:1` 的 `insertCustomView` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件
  - 不强制开 Excel

## 超链接

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-hyperlinks.ps1`。编号用 Xhl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xhl1 | inspect | 每张 `worksheet` 始终有 `hyperlinkCount`；create 默认为 `0`；给了 `scope.hyperlinks` 时名单为空数组，不依赖单元格切片 |
| Xhl2 | 插入 | `insertHyperlink` 写到 `A1` 后 Excel 无修复；COM `Hyperlinks` 能看见 `Address`；inspect `url` 对得上 |
| Xhl3 | 改地址 | `setHyperlink` 改 `url` 后 COM `Address` 对得上、无修复 |
| Xhl4 | 删除 | `deleteHyperlink` 后 `hyperlinkCount=0`、单元格文字还在、无修复 |
| Xhl5 | 缺字段 | 只给 `op` 与 `id` 的 `insertHyperlink`（缺 `ref`）得 `InvalidField`，未写文件。不强制开 Excel |

## 页眉、图表工作表 drawing 与 DrawingML 链

图表工作表叠加对象走 Chart Drawing。图 / 框 / 形状 / SmartArt 上的链走 `a:hlinkClick`。脚本 `manual-check/office/cases/xlsx-drawing-hlink.ps1`。编号用 **Xdh**。不要和 Xh、Xhl、Xsa、Xs、Xp 撞。不编辑 VML。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xdh1 | 图上链 | `insertPicture` 后再 `insertHyperlink` 点图 id，inspect `hyperlink.url` 对得上；Excel 无修复；COM 该形状 `Hyperlink.Address` 对得上 |
| Xdh2 | 改删链 | `setHyperlink` 改 `url` 后 COM 地址更新；`deleteHyperlink` 后 `hyperlink` 为 `null`、图还在、无修复 |
| Xdh3 | 图表工作表插形状 | `insertChartSheet` 后 `insertShape` 带 `fromX` 四元组，inspect `anchor=relSize`；Excel 无修复；COM `Charts.Item(1).Shapes.Count` 至少为 1 |
| Xdh4 | 图表工作表插图 | 对 `chartsheet` 发 `insertPicture` 带 `fromX` 四元组后 `pictureCount>=1`，无修复 |
| Xdh5 | 拒写 | 对 `chartsheet` 发带单元格 `from` 的 `insertShape`、对工作表图发带 `ref` 的 DrawingML `insertHyperlink` 得 `InvalidField`。未写文件。不强制开 Excel |

## 迷你图

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-sparklines.ps1`。编号用 Xsk。

- **Xsk1**
  - 检查：inspect
  - 通过标准：每张 `worksheet` 始终有 `sparklineCount`
  - create 默认为 `0`
  - 给了 `scope.sparklines` 时名单为空数组，不依赖单元格切片
- **Xsk2**
  - 检查：插入
  - 通过标准：`insertSparkline` 写到 `A1`、数据 `B1:F1` 后 Excel 无修复
  - COM `SparklineGroups` 能看见
  - inspect `type=line`、`formula` 含 `B1:F1`
- **Xsk3**
  - 检查：改范围
  - 通过标准：`setSparkline` 改 `formula` 后 COM `SourceData` 对得上、无修复
- **Xsk4**
  - 检查：删除
  - 通过标准：`deleteSparkline` 后 `sparklineCount=0`、无修复
- **Xsk5**
  - 检查：缺字段
  - 通过标准：只给 `op` 与 `id` 的 `insertSparkline`（缺 `ref`）得 `InvalidField`，未写文件
  - 不强制开 Excel

## 保护

用库 create 产物再 apply。带哈希样例由脚本在 ZIP 里写入 `sheetProtection` 属性，不靠 Excel 猜密码。脚本 `manual-check/office/cases/xlsx-protect.ps1`。编号用 Xpr。

- **Xpr1**
  - 检查：inspect
  - 通过标准：无 scope 时工作簿有 `workbookProtection`，每张 `worksheet` 有 `sheetProtection`
  - create 默认为 `enforced=false`、`hasPassword=false`、`lockStructure=false`
- **Xpr2**
  - 检查：保护工作表
  - 通过标准：`setProtection` `enforced: true` 后 Excel 无修复
  - COM `ProtectContents` 为真
- **Xpr3**
  - 检查：解开
  - 通过标准：`enforced: false` 后 COM `ProtectContents` 为假，单元格能写入，无修复
- **Xpr4**
  - 检查：工作簿结构
  - 通过标准：`lockStructure: true` 后 COM `ProtectStructure` 为真、无修复
- **Xpr5**
  - 检查：缺字段与哈希
  - 通过标准：只给 `op` 与 `id` 得 `InvalidField`，未写文件
  - 带 `hashValue` 的样例 inspect `hasPassword=true`，再 apply 得 `NotImplemented`
  - 不强制为缺字段开 Excel

## 保护密码哈希

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-protect-hash.ps1`。编号用 **Xph**。不要和 Xpr、Xp、Xh 撞。不破解密码。

- **Xph1**
  - 检查：inspect
  - 通过标准：`setProtection` 带 `password` 后 `hasPassword=true`，树里没有哈希或明文
- **Xph2**
  - 检查：工作表口令
  - 通过标准：Excel 无修复
  - COM `ProtectContents` 为真
  - `Unprotect("secret")` 成功，错误口令失败
- **Xph3**
  - 检查：工作簿口令
  - 通过标准：`lockStructure` 带 `password` 后 COM `ProtectStructure` 为真
  - `Unprotect("secret")` 成功
- **Xph4**
  - 检查：解开
  - 通过标准：给对得上的 `password` 且 `enforced: false` 后 `hasPassword=false`、无 Repair
- **Xph5**
  - 检查：拒写
  - 通过标准：空 `password`、错口令解开、不给 `password` 改带 SHA-512 的表，分别得 `InvalidField` 或 `NotImplemented`
  - 均未写文件
  - 不强制为拒写开 Excel

## 允许编辑区域与 xlsm

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-edit-range.ps1`。编号用 **Xer**。不要和 Xpr、Xph 撞。不执行宏，不破解密码。

- **Xer1** ✅
  - 检查：无密码插入
  - 通过标准：`insertProtectedRange` 后 inspect `protectedRangeCount=1`，名单 `name` / `sqref` / `hasPassword=false`
  - Excel 无 Repair
  - COM `AllowEditRanges` 能看到该区域
- **Xer2** ✅
  - 检查：带密码
  - 通过标准：带 `password` 后 `hasPassword=true`，树里没有哈希或明文
  - Excel 无 Repair
  - 包内 `algorithmName="SHA-512"`、`spinCount="100000"`
- **Xer3** ✅
  - 检查：改删
  - 通过标准：`setProtectedRange` 改名与区域后 inspect 对得上；`deleteProtectedRange` 后计数为 0
  - 无 Repair
- **Xer4** ✅
  - 检查：打开 `.xlsm`
  - 通过标准：create 到 `.xlsm` 后 workbook Override 为 macroEnabled，inspect `format` 仍为 `xlsx`
  - 注入 `vbaProject.bin` 再 apply 后该部件原字节留下，Override 仍是 macroEnabled
  - Excel 无 Repair
- **Xer5** ✅
  - 检查：拒写
  - 通过标准：对 `chartsheet`、空 `name`、表内重名、只给 `op` 与 `id`、不给口令改带 SHA-512 的区域，分别得 `InvalidField` 或 `NotImplemented`
  - 均未写文件
  - 不强制为拒写开 Excel

## 锚点剩余

用库 create 带图产物再 apply。不算列宽行高。脚本 `manual-check/office/cases/xlsx-anchor.ps1`。编号用 Xa。

- **Xa1**
  - 检查：twoCell 改 `to`
  - 通过标准：对 `twoCell` 图发 `setPictureSize` 且给 `to` 后，inspect `to` 对得上
  - Excel 无 Repair
- **Xa2**
  - 检查：转到 absolute
  - 通过标准：`oneCell` 图 `setPictureAnchor` `anchor=absolute` 且给齐 `posX` / `posY` 后，inspect `anchor=absolute`
  - Excel 无 Repair
- **Xa3**
  - 检查：改绝对坐标
  - 通过标准：已是 `absolute` 时只改 `posX`，inspect 对得上
  - Excel 无 Repair
- **Xa4**
  - 检查：互转缺字段
  - 通过标准：转到 `absolute` 却不给 `posX` / `posY` 得 `InvalidField`，未写文件
  - 不强制开 Excel
- **Xa5**
  - 检查：twoCell 拒 `cx`
  - 通过标准：对 `twoCell` 图发 `setPictureSize` 且给 `cx` 得 `InvalidField`，未写文件
  - 不强制开 Excel
