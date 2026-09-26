# docx SmartArt

状态：❌ 未开始  
日期：2026-09-26

SmartArt 是 DrawingML 图示，不是图片。本期改数据模型里的字和点，不实现版式算法。Word 打开时自己重排。

规范线索：宿主 `a:graphicData` 的 diagram URI，`dgm:relIds` 指向 data / layout / colors / quickStyle 四个部件（ECMA-376 §21.4）。常见路径 `/word/diagrams/…`。常包在 `mc:AlternateContent` 里，Choice 是图示，Fallback 可能是位图；只进 Choice，不编辑 Fallback 图。

## 目标

SmartArt 能列、改各点文字、增删点、插入与删除整张图示、改配色与快速样式（只写部件引用，不画）。版式靠 Word 重算。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行 | ❌ |
| 2.1 | inspect：根上 `smartArtCount`；`scope.smartArt: true` 时列图示 | ❌ |
| 2.2 | 每张：版式类型（layout 部件或 uniqueId）、点列表（文本、层次） | ❌ |
| 3.1 | `setSmartArtText`：按点 id 改 `dgm:t` 可见文本 | ❌ |
| 3.2 | `addSmartArtPoint`：在父点下加一个点，写 data 部件；不自己算坐标 | ❌ |
| 3.3 | `deleteSmartArtPoint`：删点及其子点 | ❌ |
| 4.1 | `insertSmartArt`：按版式名生成一套最小 diagram 四件套 + 主文档 drawing | ❌ |
| 4.2 | `deleteSmartArt`：删 drawing、四件套、关系；Fallback 整段 `AlternateContent` 一起删 | ❌ |
| 4.3 | `setSmartArtSize` / `setSmartArtWrap`：宿主 drawing，对齐图片 | ❌ |
| 5.1 | `setSmartArtColors` / `setSmartArtStyle`：换成包内已有或库内最小内置的 colors/style 部件 | ❌ |
| 5.2 | `setSmartArtLayout`：换成另一种 layout；不能无损则 `NotImplemented` | ❌ |
| 6 | 页眉页脚里的 SmartArt：同一套前缀 | ❌ |
| 7 | 库测试 | ❌ |
| 8 | Word 16.0 手测，编号 Sa1…：改字后打开无修复，点上的字对；增删点后 Word 重排能看见 | ❌ |

建议提交顺序：`1 → 2.x → 3.1 → 4.2 → 4.1 → 3.2 → 3.3 → 4.3 → 5.x → 6 → 7 → 8`。

## 稳定 id

| 节点 | id | 例子 |
| --- | --- | --- |
| 图示 | `{段落 id}/smartdiagram:{docPrId}` | `para:3F2A1B00/smartdiagram:4` |
| 点 | `{图示 id}/pt:{modelId}` | `para:3F2A1B00/smartdiagram:4/pt:2` |

`{modelId}` 取 data 部件里点的 modelId 原文。没有 `docPr/@id` 时图示用 `smartdiagram:index:{k}`。只认 diagram URI。文本框、图表、普通图各走各的名单。

点的层次用 `parent`（父点 id 或 `null`）。文档点、样式点等非用户可见点不进树，或标 `visible: false` 且 apply 除删除整张外不改。写契约时用可执行判定（例如只收带 `dgm:t` 的点）。

## inspect

- 无 scope 时只有计数。
- 名单字段：`id`、`docPrId`、`name`、`layout`、`points`、`stable`。
- `points[]`：`id`、`modelId`、`text`、`parent`。
- 不读 Fallback 位图，不把图示画成图。

## apply

改字只动 data 部件对应 `dgm:t`。增删点只改 data（及必要的关系）。坐标、连接线、形状几何留给 Word。

插入：库内带一份最小「列表」或「组织结构」四件套模板（写在仓库 `crates/ooxp` 资源里，不是 Office 安装目录）。`create` 若要带 SmartArt，复用 `insertSmartArt` 的生成，字段写进本计划契约，不另开 create 计划。

删整张时整段 `AlternateContent` 删掉，避免留下旧位图。

## 不做

- 图示版式算法、自动连线、动画
- 把 SmartArt 拆成普通形状来编辑
- 编辑 VML / Fallback 预览图
- 当作 `replacePicture` 换一张静态图（那是图片计划；用户若只要换预览，应删图示再插图）

## 验收

- Word 另存一张列表型 SmartArt；inspect 的点文本与 Word 里能看见的字一致。
- `setSmartArtText` 后无修复，目标点的字变了。
- 增加一个点、删除一个点，Word 打开后重排，条数对。
- 删除整张后 Fallback 图也不在。
- 不支持的 layout 转换得到 `NotImplemented`，文件未写坏。
