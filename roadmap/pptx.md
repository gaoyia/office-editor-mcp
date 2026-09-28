# pptx 编辑路线（不做渲染）

只记账顺序，不验收。PowerPoint 全量指包里能表达的对象都能进 inspect、能 apply、create 能按同一套 JSON 生成。不是复刻 PowerPoint 运行时。

docx 见 [`roadmap/docx.md`](docx.md)。xlsx 见 [`roadmap/xlsx.md`](xlsx.md)。本仓库不实现 MCP；外部 MCP 用领域 JSON 调库，不在本目录开切片。

pptx 模块不要引用 `docx::` 或 `xlsx::`。`c:chart`（§21.2）用 `crates/ooxp/src/chart/`。每份计划仍是：先改当期 `docs/`，确认后再动代码。挂机一次只开一份，从待做序号 0 起。当期切片写在 `.plan/`，不写进本目录。

## 永不做 🚫

- 排版引擎、与 PowerPoint 像素级一致
- 完整写入 `<p:timing>` / 动画时间轴（只读清单是待做序号 4）
- 编辑 VML / `p:legacyDrawing`
- 默认把图片字节或 embedding 塞进 inspect 树
- 把 `p:pic` 当成 `slide:…/shape:…`（形状只收带 `p:txBody` 的 `p:sp`，已验收契约不拆）
- SmartArt 版式算法
- 执行宏、打印本身
- `.ppt`

旧式 `commentAuthors`（ECMA-376 §19.4）继续忽略。`vbaProject.bin` 未点名则原样写回。

## 不单开

- 组合 `p:grpSp` 里的图、图表、SmartArt：展开符合条件的子节点，组合本身不进树
- 备注页、批注里的图：跟容器走，复用图片 op
- `exportMedia`：跟幻灯片图走
- `exportChartData`：跟幻灯片图表走
- 复制已有文件再 apply：不必新动作
- `a:fld`：不收不改；独立节点不另开

## 已过

| 序 | 计划 | 前置 |
| --- | --- | --- |
| 0 | `.plan/2026年-9月-27日-✅-pptx-幻灯片图.md` | 无 |
| 1 | `.plan/2026年-9月-27日-✅-pptx-create带图.md` | 幻灯片图 |
| 2 | `.plan/2026年-9月-27日-✅-pptx-幻灯片图表.md` | 无（不要跟 `p:pic` 绑） |
| 3 | `.plan/2026年-9月-27日-✅-pptx-create带图表.md` | 幻灯片图表；create 带图 |
| 4 | `.plan/2026年-9月-27日-✅-pptx-SmartArt.md` | 无（不要跟 `pic` 绑） |
| 5 | `.plan/2026年-9月-27日-✅-pptx-备注.md` | 幻灯片图 |
| 6 | `.plan/2026年-9月-27日-✅-pptx-批注.md` | 幻灯片图 |
| C1 | `.plan/2026年-9月-27日-✅-图表剩余.md` | 幻灯片图表已过 |
| C2 | `.plan/2026年-9月-27日-✅-SmartArt跨家族.md` | SmartArt 已过 |
| C3 | `.plan/2026年-9月-27日-✅-批注墨迹.md` | 批注已过 |
| 0 | `.plan/2026年-9月-27日-✅-pptx-幻灯片管理.md` | 无 |
| 1 | `.plan/2026年-9月-27日-✅-pptx-形状增删与几何.md` | 无（`setShapeText` 已过；本期才插删形状） |
| 2 | `.plan/2026年-9月-27日-✅-pptx-表格.md` | 形状增删 |

0 与 2 不要合成一份。图是 `p:pic`，图表是 `p:graphicFrame`，位置是画布 `a:xfrm`。第一期页名单和 `setShapeText` 见总计划。

## 待做

| 序 | 计划 | 前置 |
| --- | --- | --- |
| 3 | `.plan/2026年-9月-27日-❌-pptx-切换.md` | 无（只动 `<p:transition>`，不写 timing） |
| 4 | `.plan/2026年-9月-27日-❌-pptx-动画清单.md` | 无（只读，禁止 apply 写 timing） |
| 5 | `.plan/2026年-9月-27日-❌-pptx-母版与版式.md` | 形状增删 |
| 6 | `.plan/2026年-9月-27日-❌-pptx-讲义.md` | 备注已过 |
| 7 | `.plan/2026年-9月-27日-❌-pptx-超链接.md` | 无 |
| 8 | `.plan/2026年-9月-27日-❌-pptx-create备注与批注.md` | 备注、批注 apply 已过 |
| 9 | `.plan/2026年-9月-27日-❌-pptx-备注批注内剩余.md` | 备注、批注、图表、SmartArt 已过 |
| 10 | `.plan/2026年-9月-27日-❌-pptx-媒体.md` | 幻灯片图已过 |
| 11 | `.plan/2026年-9月-27日-❌-pptx-幻灯片节.md` | 幻灯片管理已过 |

3、4、7 可并行。1 不要删形状重建已有页上的旧形状，以免动画丢绑。5 不要和 1 合成一份：母版是独立部件。

## 跨格式（本类型要跟）

本类型跨格式已无待做。xlsx 批注墨迹不套本文。

## 怎么算 PowerPoint 全量做完

已过表保持 ✅，待做 1–11 与 C1–C3 都 ✅，`docs/` 与实现一致，PowerPoint 16.0 手测过对应清单。永不做的条目不算欠账。完整写入 timing 保持 🚫。
