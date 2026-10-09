# pptx setAnim、进片自动播、备注与母版动画

状态：✅  
日期：2026-10-09

对应 `roadmap/pptx.md` 待做序号 **27**。前置：动画延迟已过。PowerPoint 16.0 手测 Aw7–Aw10 已过。

## 目标

`setAnim` 能改已有一条的 trigger / delay / dur。第一条 `with` / `after` 进片自动播。备注页与母版 / 版式占位符能写同一套 fade。`setTiming` 仍报 `UnsupportedOp`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：setAnim、进片自动播、备注 / 母版 | ✅ |
| 1 | 库：解析、写入、inspect | ✅ |
| 2 | 库测试与 PowerPoint 16.0 手测 Aw7–Aw10 | ✅ |

## 已锁定

- `setAnim` 的 id 是 inspect 名单上的 `…/anim:{k}`。可写 `trigger`、`delay`、`dur`，至少给一个。省略的字段不改。不改 `type`、不换绑定对象。
- `setTiming` 仍报 `UnsupportedOp`。不接受整棵时间轴树。
- 进片自动播：该宿主第一条效果 `trigger` 为 `with` 或 `after` 时，外层包装仍写 `delay="indefinite"`，并加 `evt="onBegin"` 指向 `mainSeq` 的 `cTn/@id`。第一条 `click` 不加这条。对照 Office 16.0 把第一条改成 With Previous 的另存。
- 备注：`insertAnim` 认 `slide:{n}/notes/` 下与幻灯片相同的可绑对象。名单在 `notes.timing`，id 为 `slide:{n}/notes/anim:{k}`。根上 `animCount` 仍只计幻灯片 XML。ECMA `CT_NotesSlide` 没有 `timing`；Office 16.0 另存不剥、无 Repair。讲义母版与备注母版仍不写。
- 母版：`insertAnim` 认占位符 `master:{n}/shape:…` 与 `master:{n}/layout:{k}/shape:…`。名单在该母版 / 版式的 `timing`，id 为 `master:{n}/anim:{k}` 或 `master:{n}/layout:{k}/anim:{m}`。只脏被点名的 slideMaster / slideLayout。不把母版动画抄进各页。slideMaster 的 `<p:timing>` 插在 `sldLayoutIdLst` 之后、`txStyles` 之前。
- create `slides[].anims` 复用同一套 insert 规则（第一条 `with` / `after` 即进片自动播）。create 不写备注动画、不写母版动画。

## 不做（本期）

- 重复、声音、运动路径、强调 / 退出、其它 `type`
- 放映引擎
- 备注母版 / 讲义母版动画
- 母版装饰形（没有 `p:ph`）
