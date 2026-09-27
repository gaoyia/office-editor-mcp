# pptx SmartArt

状态：✅ 文档、实现、库测试与 PowerPoint 16.0 手测 Psa1–Psa6 已过  
日期：2026-09-27

从 `roadmap/pptx.md` 拆出序号 4。SmartArt 不是图片。改数据模型里的字和点，不实现版式算法。PowerPoint 打开时自己重排。

规范线索：宿主 `p:graphicFrame` 的 diagram URI，`dgm:relIds` 指向 data / layout / colors / quickStyle（ECMA-376 §21.4）。常见路径 `/ppt/diagrams/…`。`mc:AlternateContent` 只进 Choice，不编辑 Fallback 图。

契约真源：`docs/json-contract.md`、`docs/json-schema/request.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-pptx.md`。

## 目标

幻灯片上的 SmartArt 能列、改各点文字、增删点、插入与删除整张、改配色与快速样式。版式靠 PowerPoint 重算。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行 | ✅ |
| 2.1 | inspect：根与每页始终有 `smartArtCount`；`scope.smartArt` 才列名单 | ✅ |
| 2.2 | 已有 SmartArt op 认 `slide:…/smartdiagram:…` 与点 id | ✅ |
| 2.3 | `setSmartArtSize` 写 `a:ext`；位置走 `setSmartArtPos`（pptx 新 op，docx / xlsx 拒绝） | ✅ |
| 3 | `create` 的 `slides[].smartArts` 复用 `insertSmartArt` | ✅ |
| 4 | 库测试 | ✅ |
| 5 | PowerPoint 16.0 手测，编号 Psa1… | ✅ |

建议提交顺序：`1 → 2.1 → 2.2 → 2.3 → 3 → 4 → 5`。先改 `docs/`，确认后再动代码。不要从 pptx 引用 `docx::`。

## 已锁定（写入契约后不要改）

- 图示 id：`slide:{n}/smartdiagram:{cNvPrId}`。点：`{图示 id}/pt:{modelId}`，规则与 docx 相同。
- 只认 diagram URI。四条关系写在该幻灯片 `.rels` 上。
- 根上 `smartArtCount` 等于各页之和。名单只在 `scope.smartArt: true` 时出现，挂在对应页上。
- `insertSmartArt` 的 `id` 必须是 `slide:{n}`。`layout` 只有 `list` 与 `org`。`kind` / `wrap` 报 `InvalidField`。可选 `x` / `y` / `cx` / `cy`。
- `setSmartArtWrap` 在 pptx 上报 `InvalidField`。
- `setSmartArtLayout` 只在同一家族内换库内模板；跨家族报 `NotImplemented`。
- 备注、批注里的 SmartArt 本期不进树。
- 不实现版式算法。改字对不上 drawing 缓存则删掉该缓存。

## 不做

- 图示版式算法、自动连线、动画
- 把 SmartArt 拆成普通形状
- 编辑 Fallback 预览图
- 母版 / 版式上的图示

## 验收

手测编号 Psa1–Psa6，写入 `docs/checklist-pptx.md`。脚本：`manual-check/office/cases/pptx-smartart.ps1`。源文件必须是 PowerPoint 亲手另存。

- 另存一张列表型；inspect 的点文本与屏幕上能看见的字一致。
- `setSmartArtText` 后无修复，目标点的字变了。
- 增删点后条数对，PowerPoint 重排。
- 删除整张后 Fallback 也不在（有则必须一起少）。
- create 带一张 `list`，打开无修复。
- 不支持的 layout 转换得 `NotImplemented`，文件未写坏。
