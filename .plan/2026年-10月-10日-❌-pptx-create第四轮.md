# pptx create 第四轮

状态：❌
日期：2026-10-10

对应 `roadmap/pptx.md` 待做序号 **38**。前置：本轮 28–37 的 apply 已过后再开；可先写文档占位，代码等对应 apply 过完。

`docs/json-contract-pptx.md` 现状：create 不写 OLE、Zoom、域、run 级链、自定义放映、讲义、备注 / 母版动画、`showPr`、`modifyVerifier`。`anims[].type` 只认 `fade`。

## 目标

create 的 `presentation` JSON 能复用本轮已有 apply，写出 OLE、Zoom、域、run 级链、自定义放映、讲义、备注动画与放映 / 修改保护。不要另写一套 XML。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段与页内顺序 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 PowerPoint 16.0 手测 | ❌ |

## 已锁定

- 只复用已有 apply。对应 apply 还没过的字段不要先写进 create。
- 任意时间轴 JSON 仍拒绝。`slides[]` 上出现 `timing` 仍报未知字段。

## 不做

- 放映引擎
- 从已有文件当模板复制

## 确认后要改的文档

- `docs/json-contract-pptx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-pptx.md`
- `roadmap/pptx.md` 待做 38
