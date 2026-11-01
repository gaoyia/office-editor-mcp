# docx 容器内索引、题注按 Word 收口

状态：✅ 已完成。
日期：2026-11-02

对应 `roadmap/docx.md` 已过序号 **77**。前置：容器内域块（74）已过。缺陷记录见 `.bugfix-reports/2026年-11月-1日-✅-容器里的索引和题注更新失败.md`。

## 现状

74 放开了目录、题注、交叉引用、索引、索引项、引文、书目块写进页眉、页脚、文本框、脚注、尾注、批注。2026-11-01 补探测逐个 `Field.Update`，索引和题注在部分容器里报错。手测只看插入成功和没有 Repair。

## 目标

只写 Word 更新后有结果的组合。Word 报错的组合，insert 拒绝，契约写 Word 的原话。

## 不做

- 不删已有文件里这些组合的域：inspect 照常列出，`delete*` 照常能删。
- 不改交叉引用、索引项、引文、书目块。
- create 本来只把目录、题注、索引写进正文，没有改。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：目录在页眉、页脚、脚注、尾注、批注里更新；题注、目录在页眉文本框中更新 | ✅ |
| 2 | 文档：每种域可插入的容器，报错的写 Word 原话 | ✅ |
| 3 | 实现：insert 按第 2 步收口 | ✅ |
| 4 | 测试 | ✅ |
| 5 | 手测：页眉用例改成断言更新后的结果 | ✅ |

### 1. 探测

本库插入后 Word 16.0 `Field.Update`，没有 Repair：

- 目录 `TOC`：页眉、页脚、页眉文本框、脚注、尾注、批注的结果都是「第一章 1」。
- 题注 `SEQ`：页眉文本框、页脚、尾注都是「错误!仅主文档。」和页眉、脚注一致。正文文本框仍是 `1`。
- 索引 `INDEX`：页脚、尾注都是「错误!脚注、尾注、页眉、页脚或批注中不允许有索引。」和页眉、文本框、脚注、批注一致。

### 2. 文档

- `insertToc` 仍可写到正文、页眉页脚、文本框、脚注、尾注、批注，或 `docx:document`。
- `insertCaption` 只留正文、正文文本框和 `docx:document`。其余报 `InvalidId`，写上「错误!仅主文档。」
- `insertIndex` 只留正文和 `docx:document`。其余报 `InvalidId`，写上「错误!脚注、尾注、页眉、页脚或批注中不允许有索引。」
- 不再写「段落范围与 `insertToc` 相同」或「库仍接受插入」。

### 3. 实现

- 题注看宿主是不是正文或正文里的 `txbxContent`。页眉、页脚、注、批注，以及这些故事里的文本框，拒绝。
- 索引只收父元素是 `w:body` 的段落。`docx:document` 仍追加到正文末尾。
- 目录的范围没改。

### 4. 测试

- `index_in_a_header_or_text_box_is_rejected_but_an_old_one_can_be_deleted`：页眉和正文文本框插入报 `InvalidId`，源文件不变。手写的页眉 `INDEX` 仍列出为 `header:default:1/index:1`，`deleteIndex` 能删。
- `caption_in_a_body_text_box_is_kept_and_a_note_is_rejected`：脚注题注报 `InvalidId`。正文文本框题注的 id 带 `caption:1`。
- 页眉题注的旧正向断言改成 `InvalidId`。

### 5. 手测

- Ix6：页眉 `insertIndex` 为 `InvalidId`，不写文件。xlsx 仍是 `InvalidField`。
- Cp2：正文题注更新后 `SEQ` 是 `1`，没有 Repair。
- Cp6：页眉题注为 `InvalidId`。正文文本框题注更新后是 `1`，没有 Repair。
- Bi6：页眉书目更新后含 `Ecma` 与 `2006`，没有 Repair。

## 验收

- `cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 缺陷记录改成 ✅。路线图把 77 挪到已过。
