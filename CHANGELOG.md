# 变更记录

从 0.1.3 起，每次发布在这里写这一版改了什么。更早的版本不补。

## 0.1.3

- `help` 的 `name` 为 `inspectScope` 且带了 `format` 时，只返回该格式认的键。不带 `format` 时仍返回 `chooseFormat`，不给字段表。
- `apply` 失败时，错误可以带 `objectId` 和 `objectName`。`objectId` 是该条操作的 `id`。`objectName` 只在该条操作顶层有字符串 `name` 时出现。`path` 仍指向入参字段。
- `apply` 可以写 `dryRun: true`。各条操作仍在内存里执行，成功时返回 `opsApplied` 和标脏的部件名 `parts`，不写盘。
- 新增只读动作 `layoutCheck`。它检查 pptx 每页形状的包围盒、越界、重叠和文字溢出估算，不写文件，也不生成图片。
- pptx 的 `setPageBackground` 可以写本页背景，纯色、两停点渐变或图片。`color` 为 `null` 时删掉背景。
- pptx 的 `setShapeFill` 可以写透明度、渐变和图片填充。
- pptx 的 `insertShape` 和 `insertPicture` 可以用 `index`、`before` 或 `after` 指定图层。create 的图片可以用 `index`。
- pptx 形状的坐标可以为负。连接线和图片的坐标仍从 0 起。
- pptx 的 create 可以在 `shapes` 里直接写预设几何、坐标、尺寸、填充和线条。这些都省略时，缺省文本框与上一版字节相同。
