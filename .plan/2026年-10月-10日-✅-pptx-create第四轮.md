# pptx create 第四轮

状态：✅
日期：2026-10-10

对应 `roadmap/pptx.md` 待做序号 **38**。前置：本轮 28–37 的 apply 已过。

## 目标

create 的 `presentation` JSON 能复用本轮已有 apply，写出 OLE、Zoom、域、run 级链、自定义几何、连接线、自定义放映、讲义、备注动画与放映 / 修改保护。不要另写一套 XML。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段与页内顺序 | ✅ |
| 1 | 库：create 复用 apply | ✅ |
| 2 | 库测试与 PowerPoint 16.0 手测 | ✅ |

## 已锁定

- 只复用已有 apply。任意时间轴 JSON 仍拒绝。`slides[]` 上出现 `timing` 仍报未知字段。
- `shapes[].geom` 与 `insertShape` 的 `geom` 相同。不要给矩形补 `geom`。
- `shapes[].fields` 复用 `insertField`。不要在 `slides[]` 上开 `fields` 名单。
- `slides[].oleObjects` 复用 `insertOle`，只要 `file`。
- `slides[].connectors` 复用 `insertConnector`。`start` / `end` 用 `{ "shape": 1, "site": 1 }`（该页 `shapes[]` 的 1 起下标）。
- `slides[].zooms` 复用 `insertZoom`。`target` / `targets` 是整数下标，不要 `slide:{n}`。
- `slides[].notes` 仍接受字符串；也可对象 `{ "text", "anims" }`。备注页 `shape: 1` 是第一个带 `txBody` 的形状。
- `presentation.customShows` / `handout` / `showPr` / `modifyProtection` 分别复用 `insertCustomShow`、`setHandoutText`、`setShowPr`、`setProtection`。
- 给了口令则写 SHA-512。不要 COM 打开带哈希产物。
- 手测编号 **Cq**。Office 16.0 手测 Cq1–Cq5 已过。

## 不做

- 放映引擎
- 从已有文件当模板复制
- 图 / 表 / 图表 / SmartArt 上的链
