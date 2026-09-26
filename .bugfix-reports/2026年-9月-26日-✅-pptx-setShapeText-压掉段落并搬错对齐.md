# pptx setShapeText 压掉段落，还把别段的对齐搬到第一段

状态：✅ 已修复并有回归测试  
日期：2026-09-26  
影响面：对含多个 `a:p` 的形状执行 `setShapeText`，也就是绝大多数正文占位符

## 怎么发现的

审计 pptx 第一期时没有只看测试结果，而是拿 `docs/json-contract.md` 的替换规则逐条对着实现读。契约第 610 行要求「把完整新文本写入其中第一个 `a:t`……其余这类 `a:t` 删掉；因此变空、且里面再也没有别的 run 内容的 `a:r` 删掉」，而 `pptx/shape.rs` 的 `set_txbody_text` 做的是把 `txBody` 里所有 `a:p` 删掉、再塞一个新的进去。

**真正的线索是两个格式的做法不一致。** docx 面对同一类条款，`docx/para.rs` 的 `rewrite_runs` 老老实实原地改第一个 `w:t`、删多余的、删空 run，一个属性都不搬。同一份契约的同类条款，一个格式原地手术、另一个整段重建，其中必有一个是错的。

随后手工造了一个三段落形状（第一段没有 `a:pPr`、加粗；第二段居中；第三段斜体）实测，问题当场暴露。

当时 pptx 的 5 项保真度测试全绿。原因是所有样本的 `txBody` 都只有一个 `a:p`——这个形状正是 `create` 生成的样子。**这和 2026-09-25 那次记下的教训是同一条：拿自己的输出当测试输入，等于让实现给自己判卷。** 上次的结论没有落实到 pptx，于是同一个坑又摔了一次。

## 现象

源文件：

```xml
<a:p><a:r><a:rPr b="1"/><a:t>第一段</a:t></a:r></a:p>
<a:p><a:pPr algn="ctr"/><a:r><a:t>第二段</a:t></a:r></a:p>
<a:p><a:r><a:rPr i="1"/><a:t>第三段</a:t></a:r></a:p>
```

发一条 `setShapeText` 之后：

```xml
<a:p><a:pPr algn="ctr"/><a:r><a:rPr b="1"/><a:t>新文本</a:t></a:r></a:p>
```

三段并成一段，而且第一段凭空多出了 `algn="ctr"`。用户只想换一句文字，结果原本左对齐的文字变成居中，形状的行数也从三行掉到一行。文件本身是合法 XML，PowerPoint 不会报「需要修复」，所以这类缺陷只能靠对着契约读实现或者比对产物发现，跑一遍打开文件是看不出问题的。

## 根因

`crates/ooxp/src/pptx/shape.rs`：

```rust
let ppr = first_named_clone(tx, "pPr");
let rpr = first_named_clone(tx, "rPr");
tx.children.retain(|n| ... name != "p" && name != "AlternateContent");
tx.push_element(build_para(ppr, rpr, text));
```

两个错叠在一起。一是把段落当成可丢弃的容器，整片删掉重建，段落级信息只剩被单独捞出来的那一个 `pPr`。二是 `first_named_clone` 找的是**整个 `txBody` 里第一个** `pPr`，而不是第一段自己的。第一段恰好没有 `pPr` 时，它就去借了第二段的。这两个错单独看都容易漏过：只有当「第一段没有 `pPr`、后面某段有」时才显形，而这个组合在真实演示文稿里非常常见。

## 修法

照 docx 的 `rewrite_runs` 重写：递归找第一个含 `a:t` 的 `a:r`，把新文本写进它的第一个 `a:t`（`\n` 就地拆成 `a:t` 加 `a:br`），其余这类 `a:t` 删掉，因此变空的 `a:r` 删掉。段落、`a:pPr`、`a:bodyPr`、`a:lstStyle` 一概不动，`mc:AlternateContent` 仍然只进一支。一个含 `a:t` 的 run 都没有时才补，且优先挂在已有的第一段末尾，没有段落才新建一段，免得凭空多出一段。

修完的产物是三段仍在，第二段的 `algn="ctr"` 留在它自己身上，第一段只换了字：

```xml
<a:p><a:r><a:rPr b="1"/><a:t>新文本</a:t></a:r></a:p>
<a:p><a:pPr algn="ctr"/></a:p>
<a:p/>
```

新建 run 时的前缀从 `txBody` 内部的元素借，不写死 `a:`。`txBody` 自己是 `p:` 前缀，拿它去拼会生成 `p:p`、`p:r` 这种不存在的元素。

## 第一版修法自己又踩了两个坑

上面那版把操作放在 run 这一级，当天就被指出还有洞，实测确认了两处，都与 `a:br` 有关。

**洞一：换行拆错了层级。** 第一版把 `\n` 拆成 `a:t` 加 `a:br` 一起塞进同一个 `a:r`，写入 `甲\n乙` 得到 `<a:r><a:rPr b="1"/><a:t>甲</a:t><a:br/><a:t>乙</a:t></a:r>`。查 ECMA-376 §A.4 才发现这是双重违规：

```xsd
<xsd:group name="EG_TextRun">
  <xsd:choice>
    <xsd:element name="r" type="CT_RegularTextRun"/>
    <xsd:element name="br" type="CT_TextLineBreak"/>
    <xsd:element name="fld" type="CT_TextField"/>
  </xsd:choice>
</xsd:group>

<xsd:complexType name="CT_RegularTextRun">
  <xsd:sequence>
    <xsd:element name="rPr" type="CT_TextCharacterProperties" minOccurs="0" maxOccurs="1"/>
    <xsd:element name="t" type="xsd:string" minOccurs="1" maxOccurs="1"/>
  </xsd:sequence>
</xsd:complexType>
```

`a:br` 与 `a:r` 平级、同为 `a:p` 的子元素，而 `a:r` 只允许一个可选 `a:rPr` 加**恰好一个** `a:t`。讽刺的是被替换掉的旧实现在这一点上是对的：它把 `a:br` 放在 `a:p` 下、每行一个独立的 `a:r`。修一个缺陷时把原本正确的部分改坏了。

**洞二：原有的 `a:br` 不清。** 第一版只删 `a:t`，段落里已有的 `a:br` 留在原处。源文件本来就有换行（`<a:r><a:t>上</a:t></a:r><a:br/><a:r><a:t>下</a:t></a:r>`，这正是 PowerPoint 的写法），改成单行 `新文本` 之后产物是 `<a:r>…<a:t>新文本</a:t></a:r><a:br/>`，inspect 读回来是 `新文本\n`，比写入值多一个换行，违反契约第 9 条。

两个洞的根子是同一个：**`a:br` 是可见文本的一部分**，inspect 早就把它读成 `\n`，所以它必须和 `a:t` 一样进入「写入」和「清理」两个环节，而且它的归属层级是段落、不是 run。第二版因此把整个操作提到段落级：第一个有文本 run 的段落收下全部新文本，换行在段落里插 `a:br` 加新 `a:r`（沿用第一个 run 的 `a:rPr`），其余段落只清可见内容。

**洞三：空的兄弟段留下，PowerPoint 多出空行。** 第二版把「不许整段重建、不许搬别段的 `a:pPr`」推成了「清空后的兄弟 `a:p` 也必须留下」。这是把 docx `setParaText`（id 是一段）套到了 pptx `setShapeText`（id 是整个形状）上。`\n` 对应的是段落里的 `a:br`，inspect 也不在 `a:p` 之间插入换行。ECMA-376 §21.1.2.2.6 把段落定为文本框里最高一层分隔，空的 `a:p` 仍是一段，PowerPoint 会画成空行。`CT_TextBody` 的 `a:p` 只需 `minOccurs="1"`。契约第 7 条已改成：写入段原地保留；清完后只剩 `a:pPr` / `a:endParaRPr` 的段删掉；含 `a:fld` 的段按第 8 条留下。计划记为 A15。

契约第 5 条原本只写「`text` 中的 `\n` 拆成 `a:t` 加 `a:br`」，既没说层级、也没提清理 `a:br`，我正是照着这句实现的。第 5 条与第 9 条（写进去能原样读回）因此自相矛盾。已把这条拆成第 5 到第 7 条，写明层级依据与 `a:br` 必须一并清掉的理由。

## 教训

同一份契约的同类条款，如果两种格式的实现做法不一样，先别急着判断哪个对，但这个不一致本身就值得追。这次是 pptx 错、docx 对；反过来的情况同样可能。

段落不是可丢弃的容器。它承载对齐、缩进、列表级别这些用户看得见的东西，删段落重建的代价远大于省下的那几行代码。

从别处克隆属性来「保留格式」是个陷阱。要保留就原地别动，一旦开始跨节点搬运，就必须回答「搬的是谁的属性、搬到谁头上」，而这个问题在多段落场景下没有正确答案。

**改这类替换逻辑之前，先把 inspect 读了哪些元素列全，写入侧必须逐一对应。** 这次两轮返工都出在 `a:br`：inspect 把它读成 `\n`，写入侧却先是漏了清理、后又拆错层级。凡是 inspect 会读进 `text` 的元素（pptx 是 `a:t` 与 `a:br`，docx 是 `w:t`、`w:tab`、`w:br`），apply 都要回答两个问题——新文本怎么生成它、旧的怎么清掉——漏一个就破坏读写一致。

**动手插入元素之前先查 schema 的层级，别照着契约的散文措辞猜。** 契约那句「拆成 `a:t` 加 `a:br`」写在讲 run 的段落里，读起来像是放进 run，实际 `a:br` 属于段落级。查一次 §A.4 的 `EG_TextRun` 只要几分钟，比返工便宜。

**修 A 的时候顺手改掉了原本正确的 B。** 旧实现虽然压段落，但换行的层级是对的。重写时应该先把旧实现里哪些行为是对的列出来，作为新实现的验收项，而不是整块替换后只验证自己关心的那一条。

## 回归测试

`crates/ooxp/tests/pptx_fidelity.rs` 里与本缺陷相关的 apply 项现在是 6 项。样本包括审计时那个三段落形状、段落级 `a:br`、前段为空、以及带 `a:fld` 的兄弟段。

- `apply_keeps_written_para_and_drops_empty_siblings`：写入后只剩一段，精确匹配 `<a:p><a:r><a:rPr b="1"/><a:t>新文本</a:t></a:r></a:p>`（`b="1"` 还在，没有凭空多出 `a:pPr`，也没有第二段留下的 `algn="ctr"`），读回值等于写入值
- `apply_puts_br_between_runs_not_inside`：换行拆成 `<a:r>…<a:t>上</a:t></a:r><a:br/><a:r><a:rPr b="1"/><a:t>下</a:t></a:r>`，新 run 带上了 `rPr`；另有一条 `!contains("<a:br/><a:t>")` 专门盯住「`a:br` 跑进 run 里」这个形状；段落数是 1
- `apply_single_line_clears_existing_br`：源文件本来两行，改成单行之后产物里没有任何 `a:br`，读回值是单行
- `apply_multiline_then_single_line_roundtrips`：单行改三行、再改回单行，每一步的读回值都等于写入值
- `apply_drops_leading_empty_para`：第一段本来就是空的，写入落在第二段，空的第一段删掉
- `inspect_skips_field_cached_text`：带页码域的形状，inspect 的 `text` 只有用户字
- `apply_keeps_sibling_that_still_has_field`：兄弟段里还有 `a:fld`，这段留下，读回值等于写入值

覆盖关系：前两项在第一版实现下会失败（第一段是 `<a:p><a:pPr algn="ctr"/><a:r>…`，`a:br` 在 run 内）。清 `a:br` 的两项在第二版之前会失败。空段两项在第二版「留住兄弟段」下会失败（段落数仍是 3，或空的前段还在）。

## 还没解决的

样本仍是手工写的 XML，不是 PowerPoint 亲手存的文件。空的兄弟段已按 A15 删掉。域已按 A14 从 inspect 的 `text` 里排除，apply 仍不改 `a:fld`。审计的其他遗留项记在 `.plan/2026年-9月-25日-⚠️-ooxp.md` 的「审计遗留」表里。
