# xlsx apply 的三处保真度缺陷

状态：✅ 已修复并有回归测试  
日期：2026-09-25  
影响面：`apply` 动作产出的所有 xlsx 文件

## 怎么发现的

代码审计时没有只看测试结果，而是照 Excel 真实存盘的写法手工造了一份样例：行上省略 `r`、单元格带样式索引 `s`、文本走共享字符串、有一组共享公式、部件里有注释、外加一个只靠 `Default` 认内容类型的未知部件。对它跑 `apply`，三个问题当场暴露。

当时仓库里的 11 项测试全绿。原因是所有测试都在操作库自己 `create` 出来的文件，而 `create` 生成的 XML 每行都带 `r`、没有共享公式、没有注释、没有样式索引，恰好把这三条路全绕开了。**这是本次最该记住的一条：拿自己的输出当测试输入，等于让实现给自己判卷。**

## 缺陷一：被改写的部件丢掉 XML 声明

现象：`apply` 之后 `sheet1.xml`、`workbook.xml` 都以 `<worksheet`、`<workbook` 直接开头，`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` 不见了。

根因在 `crates/ooxp/src/xml.rs` 的 `Document::to_bytes`。原来的写法是：

```rust
let mut writer = Writer::new(Cursor::new(Vec::new()));
writer.get_mut().get_mut().extend_from_slice(decl.as_bytes());
write_element(&mut writer, &self.root)?;
```

`extend_from_slice` 把声明追加进了底层 `Vec`，但 `Cursor` 的写指针还停在 0。紧接着 `write_element` 通过 `Write for Cursor` 从位置 0 开始写，正好把声明覆盖掉。如果某个部件的正文比声明还短，覆盖不全，尾巴上还会残留半截声明，直接变成非法 XML。

修法：声明拼在一个独立的 `Vec` 前面，元素写完再接上，不再依赖 `Cursor` 的位置。

教训：往一个带位置游标的写入器的底层缓冲里直接 `extend`，是在绕过它的状态。要么全程走写入器的 API，要么干脆别共用一个缓冲。

## 缺陷二：行上省略 `r` 时插出重复行

现象：样例第三行写成 `<row spans="1:3">`（合法，行号由出现顺序推出）。往 A3 写值之后，同一张表里出现了两个 A3：

```xml
<row spans="1:3"><c r="A3"><v>12</v></c></row><row r="3"><c r="A3"><v>999</v></c></row>
```

Excel 打开这种文件会报需要修复。更隐蔽的是清空操作：`clear_cell` 找不到目标就静默返回，命令行照样回报 `opsApplied: 4`，四条 op 实际只生效三条，程序一声不响。

根因在 `crates/ooxp/src/xlsx/apply.rs`。`find_row_insert_pos` 把缺 `r` 的行按 `unwrap_or(0)` 当成第 0 行，永远匹配不上目标行号；`find_cell_insert_pos` 对缺 `r` 的 `<c>` 直接 `continue`，插入位置同样会算错。

修法：新增 `make_refs_explicit`，在改这张表之前先按隐式编号规则（§18.3.1.73、§18.3.1.4）把省略的 `r` 算出来写成显式。语义完全等价，而且一次消灭了另一个隐患：往中间插新行会让后面那些隐式编号的行整体漂移，等于悄悄改了别的数据。

教训：OOXML 里大量属性是可选的，「可选」意味着两种写法都得能处理。用 `unwrap_or(0)` 给缺省值兜底，等于把「没写」和「写了 0」混为一谈。

## 缺陷三：改共享公式主格留下悬空引用

现象：样例里 B1 是共享公式主格（`<f t="shared" ref="B1:B2" si="0">A1*2</f>`），B2 是从属格（`<f t="shared" si="0"/>`）。对 B1 写新公式之后，B1 变成普通的 `<f>A1*3</f>`，而 B2 仍然引用 `si="0"`，这个 si 已经没有定义了。Excel 打开报需要修复。

根因：`fill_cell` 无条件把 `<f>` 整个换掉，没有考虑这个 `<f>` 可能正在给别的单元格当定义。清空操作同样会删掉 `<f>`，问题一样。

修法：新增 `reject_shared_formula_master`。目标格的 `<f>` 带 `t="shared"` 和 `ref`，并且表里确实还有别的单元格引用同一个 `si` 时，报 `NotImplemented` 并且不写文件。想改主格，先把同组从属格逐个改成独立公式。改从属格是允许的，主格不受影响。

之所以选择拒绝而不是自动展开：把主格公式平移到从属格的位置需要解析公式并偏移单元格引用，那是计算引擎的活，明确在本项目范围之外。宁可报错，不做半套引擎。

契约已同步更新，见 `docs/json-contract.md` 的「setCell」一节和 `docs/errors.md` 的 `NotImplemented`。

## 回归测试

新增 `crates/ooxp/tests/xlsx_fidelity.rs`，10 项，样例就是审计时那份仿 Excel 写法的包：

- XML 声明在改写后仍然存在
- 隐式行号的行不会被插成两行，值写得进去
- 清空隐式行号里的单元格确实生效
- 共享公式主格被拒绝，从属格能改
- 样式索引 `s`、`spans`、`ht`、`cols`、`pageMargins` 都保住
- 未改部件字节一致，改文本走 `inlineStr` 不动共享字符串表
- `calcChain.xml` 连同内容类型 Override 和关系一起清掉，`calcPr` 上原有属性保留
- ZIP 条目顺序保持
- inspect 能读共享字符串和共享公式

## 还没解决的

仿造的样例终究不是 Excel 亲手存的文件。真实样例已放进 `crates/ooxp/tests/fixtures/excel-source.xlsx`，清单 A2–A7 的 XML 与 O2 由 `xlsx_excel_source.rs` 回归。A1 仍靠 `manual-check/office/cases/xlsx-apply.ps1`。审计表已收口，记在 `.plan/2026年-9月-25日-✅-ooxp.md`。
