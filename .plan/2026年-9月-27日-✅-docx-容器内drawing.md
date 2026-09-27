# docx 容器内 drawing

状态：✅ 文档、实现与 Word 16.0 手测已过  
日期：2026-09-27

从 `roadmap/docx.md` 拆出待做序号 0。

## 目标

文本框、脚注、尾注、批注里的图表与 SmartArt 进树，op 与正文相同，id 带容器前缀。已有文件里的注内与批注内文本框同样进树。不要另写一套 chart / SmartArt 实现。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：收窄「本期不进树」那几句 | ✅ |
| 2 | 框内图表 / SmartArt | ✅ |
| 3 | 注内、批注内图表 / SmartArt；注内与批注内框只读进树 | ✅ |
| 4 | 库测试与 Word 16.0 手测 | ✅ |

## 已锁定

- 收集边界仍遇嵌套 `txbxContent` 停住，避免双计。
- `exportChartData` / SmartArt op 认带前缀的 id。
- 根上 `chartCount` / `smartArtCount` / `textBoxCount` 只计正文。
- 框内图表挂在框节点上。注与批注上的图表不含框内图。
- 对注内或批注内段落发 `insertTextBox` 报 `InvalidField`。Word 不能把图形对象放到文本框、标注、批注、脚注或尾注中。

## 不做

- 框内再套框
- create 往框内 / 注内 / 批注内写图表
- 往注内或批注内插入文本框

## 前置

文本框、脚注尾注、批注、嵌入图表、SmartArt 已过。

## 验收

批注里插一张柱形图，Word 无修复；对注内发 `insertTextBox` 报 `InvalidField`；正文 `chartCount` 不含批注内图。
