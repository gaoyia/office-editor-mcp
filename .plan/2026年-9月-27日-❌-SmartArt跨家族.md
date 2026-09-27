# SmartArt 跨家族

状态：❌ 文档未写，实现与手测未开始  
日期：2026-09-27

从 `roadmap/` 三份文件的跨格式表拆出 C2。同家族换 layout 已过。跨家族（例如列表改循环）目前 `NotImplemented`。本期只换库内模板并尽量保住可见点文字，不算版式。

## 目标

`setSmartArtLayout` 在库内已知家族之间可以跨族。点对不上就按契约写明：丢掉几何、保住 `dgm:t` 文字，或整表拒绝。xlsx 要等形状与 SmartArt 计划把图示送进树之后才验收表上的跨族。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：家族表、对不上时的行为、错误 | ❌ |
| 2 | 换 layout 部件、改 `r:lo`、必要时重建连接 | ❌ |
| 3 | 库测试与 Word / PowerPoint 手测；xlsx 有图示后再补 | ❌ |

## 已锁定

- 不算自动连线，不读 Fallback 位图。
- 当前 uniqueId 已是目标：空操作，仍计入 `opsApplied`。
- 不要写出要修复的包。

## 不做

- SmartArt 版式算法
- 任意 Office 在线下载的 layout

## 前置

docx / pptx SmartArt 已过。xlsx 见 `.plan/2026年-9月-27日-❌-xlsx-形状与SmartArt.md`。

## 验收

列表改成循环后文字还在、Office 无修复；不认识的 uniqueId 仍拒绝且不写文件。
