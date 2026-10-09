# pptx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft PowerPoint 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。
本机若 `PowerPoint.Application` 被 WPS 抢走，直接启动 `POWERPNT.EXE` 再挂 COM，不要走被抢的 ProgID。Office 16 的对象模型在 `mdiClass` 上。

C、I、A、Pp、Pvg、Pem、P3d、Pc、Psa、Pn、Pcm、Pm、Ps、Sg、Pt、Tn、Tr、Am、Aw、Mm、Mw、Ts、Ho、Phl、Phr、Pcip、Pmd、Psc、Cs、Po、Cr、Pz、Pw、
Pf 各组由 `manual-check/office/cases/pptx-*.ps1` 自动判定，
跑 `pwsh -File manual-check\office\run.ps1 -Case 'pptx-*'` 即可复现。O1 已放弃，与 xlsx / docx 相同。

## 本机结果（2026-09-28）

Microsoft PowerPoint 16.0 已打开，C1、C3、C4、C5–C13、I1–I5、A1–A5、Pp1–Pp7、Pc1–Pc7、Psa1–Psa7、Pn1–Pn6、Pcm1–Pcm6、Pik1–Pik4、Pm1–Pm5、
Ps1–Ps5、Sg1–Sg5、Pt1–Pt6、Tn1–Tn5、Am1–Am5、Mm1–Mm5、Ho1–Ho5、Phl1–Phl5、Pmd1–Pmd5、Psc1–Psc5、Cs1–Cs5 均 PASS。
本机若装了 WPS，`PowerPoint.Application` 会被抢走，测试仍直接启动 `POWERPNT.EXE`，从 `mdiClass` 挂 COM。O1 已放弃。
2026-09-27 补跑 `pptx-pictures.ps1`，Pp1–Pp7 通过。同日补跑 `pptx-create-pictures.ps1`，C5–C7 通过。同日补跑 `pptx-charts.ps1`，Pc1–Pc7 通过。
同日补跑 `pptx-create-charts.ps1`，C8–C10 通过。同日补跑 `pptx-smartart.ps1`，Psa1–Psa7 通过。同日补跑 `pptx-notes.ps1`，Pn1–Pn4 通过。
同日补跑 `pptx-comments.ps1`，Pcm1–Pcm5、Pik1–Pik4 通过。同日补跑 `pptx-docprops.ps1`，Pd1–Pd5 通过。
同日补跑 `pptx-charts-remain.ps1`，Pr1–Pr3 通过。2026-09-28 补跑 `pptx-slides.ps1`，Pm1–Pm5 通过。同日补跑 `pptx-shapes.ps1`，Ps1–Ps5 通过。
同日补跑 `pptx-tables.ps1`，Pt1–Pt6 通过。同日补跑 `pptx-transition.ps1`，Tn1–Tn5 通过。同日补跑 `pptx-timing.ps1`，Am1–Am5 通过。
同日补跑 `pptx-masters.ps1`，Mm1–Mm5 通过。同日补跑 `pptx-handout.ps1`，Ho1–Ho5 通过。同日补跑 `pptx-hyperlinks.ps1`，Phl1–Phl5 通过。
同日补跑 `pptx-create-notes-comments.ps1`，C11–C13 通过。同日补跑 `pptx-notes.ps1`，Pn5–Pn6 通过。同日补跑 `pptx-comments.ps1`，Pcm6 通过。
同日补跑 `pptx-media.ps1`，Pmd1–Pmd5 通过。同日补跑 `pptx-sections.ps1`，Psc1–Psc5 通过。2026-09-29 补跑 `pptx-anim-write.ps1`，Aw1–Aw5 通过。
同日补跑 `pptx-transition-remain.ps1`，Tr1–Tr5 通过。同日补跑 `pptx-shape-geom.ps1`，Sg1–Sg5 通过。
同日补跑 `pptx-master-write.ps1`，Mw1–Mw5 通过。同日补跑 `pptx-theme-size.ps1`，Ts1–Ts5 通过。同日补跑 `pptx-custom-show.ps1`，Cs1–Cs5 通过。
2026-09-29 补跑 `pptx-create-remain.ps1`，Cr1–Cr5 通过。同日补跑 `pptx-notes.ps1`，Pn7–Pn11 通过。同日补跑 `pptx-zoom.ps1`，Pz1–Pz5 通过。
同日补跑 `pptx-protect.ps1`，Pw1–Pw5 通过。同日补跑 `pptx-fields.ps1`，Pf1–Pf5 通过。同日补跑 `pptx-run-links.ps1`，Phr1–Phr5 通过。
同日补跑 `pptx-comment-pics.ps1`，Pcip1–Pcip5 通过。2026-09-29 补跑 `pptx-charts-rare.ps1`，Pq1–Pq5 通过。
2026-10-09 补跑 `pptx-svg.ps1`，Pvg1–Pvg6 通过。PowerPoint COM `Type=28`，96×48 对应 914400×457200 EMU。
同日补跑 `pptx-emf.ps1`，Pem1–Pem6 通过。PowerPoint COM `Type=13`，无 Repair。
同日补跑 `pptx-model3d.ps1`，P3d1–P3d6 通过。PowerPoint COM `Type=30`，无 Repair。

- **C1** ✅
  - PowerPoint 16.0 能打开 `manual-check/office/out/pptx-create.pptx`，标题栏无 Repair
- **C3–C4** ✅
  - 第一页 `标题` / `hello`，第二页 `第二页`
- **C5–C7** ✅
  - PowerPoint 16.0：inspect 第一张 `slide:1/pic:3`
  - 同一页第二张 `slide:1/pic:4`
  - 打开无 Repair，Type=13 的图为 2
- **C8–C10** ✅
  - PowerPoint 16.0：inspect 第一张 `slide:1/chart:3`、`type=col`
  - 系列 1,2,3 与 embedding B2/B4 一致
  - 打开无 Repair，charts=1
- **C11–C13** ✅
  - PowerPoint 16.0：inspect 第一页 `hasNotes=true`、`body` 为「演讲者备注」
  - 根与第一页 `commentCount=1`、作者 Alice
  - 打开无 Repair，备注窗格可见「演讲者备注」
- **Cr1–Cr5** ✅
  - PowerPoint 16.0：`pptx-create-remain.ps1`，5 PASS / 0 FAIL
  - 两节 COM 名为「开场」「内容」、FirstSlide=2
  - 第一框单击地址对上
  - 第二页 Type=16 且 `audioCount=1`
  - 淡入 EntryEffect=3849
  - MainSequence.Count=1
  - 页级 `hyperlinks` / `oleObjects`、`type:null`、xlsx `sections` 拒写
  - 无 Repair
- **I1–I5** ✅
  - `pptx:presentation`、`slideCount=2`、无 scope 时没有 `shapes`、仍有 `pictureCount`、形状 id 为 `slide:1/shape:2` 且 `stable`
- **A1–A5** ✅
  - 目标框为「改过的正文」且仍加粗（`Bold=-1`），同页另一框与第二页未改
- **Pp1–Pp7** ✅
  - PowerPoint 16.0：无 `pictures` 时 `pictureCount=1`
  - 名单 id 为 `slide:1/pic:5`
  - 换/插/删无修复
  - 尺寸 144 磅
  - 位置 72×108 磅
  - export 与 `/ppt/media/…` 字节一致
- **Pvg1–Pvg6** ✅
  - PowerPoint 16.0：`pptx-svg.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType=image/svg+xml`，COM `Type=28`，无 Repair
  - `exportMedia` 为 SVG 原字节
  - 换成 PNG 后没有 `contentType`、包内没有 `.svg`
  - 备注图 id 为 `slide:1/notes/pic:5`
  - 非图片 `file` 得 `InvalidField`
- **Pem1–Pem6** ✅
  - PowerPoint 16.0：`pptx-emf.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType` 为 `image/x-emf` / `image/x-wmf`，COM `Type=13`，无 Repair
  - `exportMedia` 为 EMF 原字节
  - 换成 PNG 后没有 `contentType`、包内没有 `.emf`
  - 备注 TIFF 图 id 为 `slide:1/notes/pic:5`
  - 非图片 `file` 得 `InvalidField`
- **P3d1–P3d6** ✅
  - PowerPoint 16.0：`pptx-model3d.ps1`，6 PASS / 0 FAIL
  - 插入 / create 树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，COM `Type=30`，无 Repair
  - `exportMedia` 为 GLB 原字节
  - `replaceModel3d` 后 id 不变
  - 备注 `slide:1/notes` 插 3D 后根上 `model3dCount` 仍为 0
  - 非 GLB `file` 得 `InvalidField`
- **Pc1–Pc7** ✅
  - PowerPoint 16.0：无 `charts` 时 `chartCount=1`
  - 名单 id 为 `slide:1/chart:4`
  - 改系列 / 插 / 删无修复
  - 折线类型对上
  - 尺寸 144×72 磅、位置 72×108 磅
  - export 与 embedding 字节一致
- **Psa1–Psa7** ✅
  - PowerPoint 16.0：`pptx-smartart.ps1`，7 PASS / 0 FAIL
  - 另存列表型 id 为 `slide:1/smartdiagram:4`
  - 改字 / 增删点 / 删整张无修复
  - create `list` 为 `slide:1/smartdiagram:3`
  - 列表改成组织图后文字还在、无修复
  - 库外 uniqueId 得 `NotImplemented` 且未写文件
- **Pn1–Pn6** ✅
  - PowerPoint 16.0：无 `notes` 时有 `hasNotes`
  - 展开后 `notes.id` 为 `slide:1/notes`
  - `setNotesText` 后备注窗格是新字且无修复
  - 改备注不改幻灯片字
  - 往备注插图无修复
  - 往备注插柱形图 id 为 `slide:1/notes/chart:5`，改系列后无修复且不并进幻灯片 `chartCount`
  - 往备注插 `list` id 为 `slide:1/notes/smartdiagram:5`，无修复且不并进幻灯片 `smartArtCount`
- **Pn7–Pn11** ✅
  - PowerPoint 16.0：`pptx-notes.ps1`，备注表组 PASS
  - id 为 `slide:1/notes/table:5`，不并进幻灯片 `tableCount`
  - COM 备注页表数为 1
  - 改格可见「改过的正文」
  - 删后 Count=0
  - 形状 id / 表 id 发 `insertTable` 为 `InvalidId`，xlsx 为 `InvalidField`
  - 无 Repair
- **Pcm1–Pcm6** ✅
  - PowerPoint 16.0：无 `comments` 时有 `commentCount`
  - 展开后 id 为 `slide:…/comment:{guid}`
  - 改字 / 插入 / 删除无修复
  - 回复与完成标记能看见
  - 注入 `a:blip` 后 `pictureCount=1`、`exportMedia` 字节一致
- **Pcip1–Pcip5** ✅
  - PowerPoint 16.0：`pptx-comment-pics.ps1`，5 PASS / 0 FAIL
  - `insertPicture` 后该条 `pictureCount=1`、id 为 `{批注 id}/pic:1`，不并进幻灯片 `pictureCount`
  - ZIP 里 `a:blip` 在 `txBody` 之后
  - 换图 `exportMedia` 字节一致
  - 删后文字还在、没有 `a:blip`
  - 带 `x`/`y` 得 `InvalidField`，`setPictureSize` 得 `InvalidId`，xlsx 得 `InvalidId`
  - 无 Repair
- **Pik1–Pik4** ✅
  - PowerPoint 16.0：`pptx-comments.ps1` 墨迹组，4 PASS / 0 FAIL
  - 现代批注注入 `ink` 后 `kind=ink`
  - `setCommentMeta` / `setCommentDone` 无修复
  - 无可见文本时 `setCommentText` 为 `NotImplemented` 且未写文件
  - `deleteComment` 无修复
- **Pd1–Pd5** ✅
  - PowerPoint 16.0：`pptx-docprops.ps1`，5 PASS / 0 FAIL
  - 根上 `props`、`creator=ooxp`
  - 只改标题后面板标题是新字、公司名还在
  - 自定义 `prop:部门`
  - 删后名单空
  - 形状 id 发 `setProps` 为 `InvalidId`
  - 无 Repair
- **Pm1–Pm5** ✅
  - PowerPoint 16.0：`pptx-slides.ps1`，5 PASS / 0 FAIL
  - 两页名单与 `hidden=false`
  - `insertSlide` 后第三页是原第二页、新页无文本框
  - `moveSlide` 到最前
  - `setSlideHidden` 后 COM Hidden 为真
  - 删最后一页 / 缺 `hidden` 为 `InvalidField` 且未写文件
  - 无 Repair
- **Ps1–Ps5** ✅
  - PowerPoint 16.0：`pptx-shapes.ps1`，5 PASS / 0 FAIL
  - `insertShape` 后 id 为 `slide:1/shape:4`
  - 位置 72×108 磅、尺寸 144×72 磅、填充红
  - 改字后旧框还在
  - 删除后 COM 形状数为 2
  - 另存样例插新框后旧框动画 Count 仍为 1
  - 错 id / 缺成对 `cx` / 缺 `color` 为 `InvalidField` 且未写文件
  - 无 Repair
- **Sg1–Sg5** ✅
  - PowerPoint 16.0：`pptx-shape-geom.ps1`，5 PASS / 0 FAIL
  - `insertShape` `prst=ellipse` 后 id 为 `slide:1/shape:4`、COM AutoShapeType=9
  - `insertConnector` 后 id 为 `slide:1/cxn:4`、COM Connector 为真
  - `groupShapes` 后 COM Type=6、子 id 仍在
  - `ungroup` 后组合解开、子 id 还在
  - 连接线几何名给 `insertShape`、只给一个成员 `groupShapes` 为 `InvalidField`，对形状 id 发 `insertConnector` 为 `InvalidId`，均未写文件
  - 无 Repair
- **Pt1–Pt6** ✅
  - PowerPoint 16.0：`pptx-tables.ps1`，7 PASS / 0 FAIL
  - 另存样例 id 为 `slide:1/table:4`
  - 改格 COM 可见「改过的正文」
  - 增行后 COM Rows=3
  - `insertTable` 后 id 为 `slide:1/table:4` 且 `stable`、COM 格可见「甲」
  - `deleteTable` 后 COM 表数为 0
  - 形状 id 得 `InvalidId`，缺 `rows` / 最后一行得 `InvalidField` 且未写文件
  - 无 Repair
- **Tn1–Tn5** ✅
  - PowerPoint 16.0：`pptx-transition.ps1`，5 PASS / 0 FAIL
  - 无切换页 `transition` 为 null
  - 另存淡入 `type=fade`、`dur=500`
  - `setTransition` 后 COM EntryEffect=3849、Duration=0.5
  - 只改字切换还在
  - `type: null` 后 COM 效果为 0
  - 形状 id 得 `InvalidId`，`type=other` 得 `InvalidField` 且未写文件
  - 无 Repair
- **Tr1–Tr5** ✅
  - PowerPoint 16.0：`pptx-transition-remain.ps1`，5 PASS / 0 FAIL
  - 定时进片 `advTm` 对上 COM AdvanceOnTime
  - `wipe` `dir=r` 对上 EntryEffect=2819
  - `fallOver` 不是 `other`
  - 只改字字段还在
  - 错 `dir` 为 `InvalidField` 且未写文件
  - 无 Repair
- **Am1–Am5** ✅
  - PowerPoint 16.0：`pptx-timing.ps1`，5 PASS / 0 FAIL
  - 无 `timing` 时根与各页有 `animCount`、没有名单
  - 另存进入淡入 `target=slide:1/shape:2`、`type=fade`、`trigger=click`
  - COM MainSequence.Count=1
  - 只改字名单还在
  - `setTiming` 为 `UnsupportedOp`
  - xlsx / docx 的 `scope.timing` 为 `InvalidField`
  - 无 Repair
- **Aw1–Aw5** ✅
  - PowerPoint 16.0：`pptx-anim-write.ps1`，5 PASS / 0 FAIL
  - create 后 `insertAnim` `type=fade`，名单一条且 COM Count=1
  - 另存页再插一条旧的还在、新的在末尾
  - `deleteAnim` 后 Count 少 1，删光没有 `<p:timing>`
  - 只改字名单还在
  - 错 id / `type` 为 `InvalidId` 或 `InvalidField` 且未写文件
  - 无 Repair
- **Mm1–Mm5** ✅
  - PowerPoint 16.0：`pptx-masters.ps1`，5 PASS / 0 FAIL
  - 无 `masters` 时只有 `masterCount`
  - create 版式名「空白」
  - 另存默认文稿列出「标题幻灯片」等
  - 标题占位符改字无修复
  - 插用户框后换成 `master:1/layout:7`（空白）用户框还在
  - 占位符 `setShapePos` 几何有变
  - xlsx / docx 拒 `scope.masters` 与 `setSlideLayout`
  - 无 Repair
- **Mw1–Mw5** ✅
  - PowerPoint 16.0：`pptx-master-write.ps1`，5 PASS / 0 FAIL
  - `insertLayout` 后 `masterCount=1`、版式 `kind=blank`、COM CustomLayouts 1→2
  - `insertPlaceholder` `type=body` 后 COM Type=14
  - `deletePlaceholder` 后占位符没了
  - `insertMaster` 后 `masterCount=2`、Designs=2
  - 占用时 `deleteLayout` / `deleteMaster` 为 `InvalidField`
  - 页 id / 坏 type / xlsx / docx 拒写
  - 无 Repair
- **Ts1–Ts5** ✅
  - PowerPoint 16.0：`pptx-theme-size.ps1`，5 PASS / 0 FAIL
  - `setSlideSize` 后 inspect `cx=9144000`、COM SlideWidth=720、无 Repair
  - 第一页文本框 `xfrm` 仍是 create 原值
  - `setThemeColors` `accent1=FF0000` 对上
  - `setThemeFonts` `majorLatin=Arial` 对上
  - 页 id 得 `InvalidId`，`cx` 越界 / `type=other` / xlsx / docx 得 `InvalidField`，未写文件
- **Ho1–Ho5** ✅
  - PowerPoint 16.0：`pptx-handout.ps1`，5 PASS / 0 FAIL
  - 无 `handout` 时只有 `hasHandout`
  - create 为 false、展开为 null
  - `setHandoutText` 写页脚 / 页眉后 COM 与 inspect 可见新字、无 Repair
  - xlsx / docx 拒 `scope.handout` 与 `setHandoutText`
- **Phl1–Phl5** ✅
  - PowerPoint 16.0：`pptx-hyperlinks.ps1`，5 PASS / 0 FAIL
  - 无名单时根与各页有 `hyperlinkCount=0`
  - `insertHyperlink` 后 COM 单击地址对上且无 Repair
  - 改地址后对得上
  - 删除后框上字还在
  - 对页 id 得 `InvalidId`，缺目标得 `InvalidField` 且未写文件
- **Phr1–Phr5** ✅
  - PowerPoint 16.0：`pptx-run-links.ps1`，5 PASS / 0 FAIL
  - `insertHyperlink` `on=run` 后 `host` 为第一框、`on=run`
  - ZIP 链在 `a:rPr` 不在 `cNvPr`
  - 改地址后 COM 文本范围对上
  - 删除后字还在、没有 `a:hlinkClick`
  - `on=other` / xlsx `on` 得 `InvalidField`，页 id 得 `InvalidId`
  - 无 Repair
- **Pmd1–Pmd5** ✅
  - PowerPoint 16.0：`pptx-media.ps1`，8 PASS / 0 FAIL
  - 无 `media` 时有 `videoCount` / `audioCount`、没有名单
  - 另存视频 id 为 `slide:1/video:3`，不并进 `pictures`
  - `exportMedia` 与 `/ppt/media/media1.mp4` 字节一致
  - `insertVideo` 后 Type=16、id 为 `slide:1/video:4`、无 Repair
  - `insertAudio` 后 `audioCount` 加 1、无 Repair
  - `deleteMedia` 后对象消失
  - 对 `slide:1/notes` 得 `InvalidId`，WAV 当视频得 `InvalidField` 且未写文件
- **Psc1–Psc5** ✅
  - PowerPoint 16.0：`pptx-sections.ps1`，5 PASS / 0 FAIL
  - 无 `sections` 时有 `sectionCount=2`、没有名单
  - 另存两节 id 为 `section:1`，`slides` 对得上页序
  - 连续 `insertSlideSection` 两节再划第二页后 COM 第二节名为「内容」、FirstSlide=2、无 Repair
  - 改名后左侧是「正文」
  - 删节后页还在、节名没了
  - 对页 id 得 `InvalidId`，`start` 与 `after` 同给得 `InvalidField` 且未写文件
- **Cs1–Cs5** ✅
  - PowerPoint 16.0：`pptx-custom-show.ps1`，5 PASS / 0 FAIL
  - 无 `customShows` 时有 `customShowCount=0`、没有名单
  - `insertCustomShow` 后 id 为 `show:1`、`name=Tour`、`slides` 为第 1、3 页
  - COM NamedSlideShows Count=1 且 `SlideShowName` 能设成 Tour、无 Repair
  - 改名 Guide 且页序为第 3、1 页后 COM 对上
  - `deleteCustomShow` 后名单空
  - `deleteSlide` 后对应 `r:id` 消失、空名单仍留那一条
  - 页 id / 空名 / 重名 / xlsx / docx 拒写
- **Po1–Po5** ✅
  - PowerPoint 16.0：`pptx-ole.ps1`，6 PASS / 0 FAIL
  - 无 `oleObjects` 时有 `oleObjectCount`、没有名单
  - 另存嵌入 xlsx id 为 `slide:1/ole:4`，不并进 `pictures`
  - `exportMedia` 与 embeddings 字节一致
  - `deleteOle` 后 COM Type=7 没了、无 Repair
  - `insertOle` 后 Type=7、`progId=Excel.Sheet.12`、无 Repair
  - 页 id 发 `deleteOle`、OLE id 发 `insertOle` 得 `InvalidId`，xlsx 得 `InvalidField`，未写文件
- **Pz1–Pz5** ✅
  - PowerPoint 16.0：`pptx-zoom.ps1`，6 PASS / 0 FAIL
  - 无名单时有 `zoomCount`
  - `insertZoom` 后 id 为 `slide:1/zoom:3`、`type=slide`、`target=slide:2`，不并进 `pictures`
  - COM Type=-2 且 Name 以 Zoom 开头
  - `deleteZoom` 后 Count=0
  - 页 id / Zoom id 错配、`type=summary`、目标是自己、xlsx 拒写
  - 无 Repair
- **Pw1–Pw5** ✅
  - PowerPoint 16.0：`pptx-protect.ps1`，5 PASS / 0 FAIL
  - create 根上 `showPr.type=present`、`showNarration=true`、`hasPassword=false`
  - kiosk+loop 的 COM ShowType=3、Loop 为真
  - browse 的 ShowType=2
  - 改回 present 后 ShowType=1
  - password / enforced true / 带哈希样例得 `NotImplemented`
  - 页 id 得 `InvalidId`
  - xlsx 拒 `setShowPr`
  - 无 Repair
- **Pf1–Pf5** ✅
  - PowerPoint 16.0：`pptx-fields.ps1`，5 PASS / 0 FAIL
  - 无名单时根与各页有 `fieldCount=0`
  - `insertField` `type=slidenum` 后 `host` 为形状 id、ZIP 有 `a:fld type="slidenum"`
  - `datetime1` 对上
  - `deleteField` 后没有 `a:fld`
  - 页 id / 备注形状 id 得 `InvalidId`，`type=other` / xlsx / `instr` 得 `InvalidField`
  - 无 Repair
- **Pr1–Pr3** ✅
  - PowerPoint 16.0：`pptx-charts-remain.ps1`，3 PASS / 0 FAIL
  - 趋势线 COM Count=1
  - 改删趋势线
  - 次轴 `AxisGroup=2`
  - 无 Repair
- **Pq1–Pq5** ✅
  - PowerPoint 16.0：`pptx-charts-rare.ps1`，5 PASS / 0 FAIL
  - 插入股价 / 曲面 / 复合条无 Repair
  - 股价三条转柱形、复合饼转饼
  - 增系列与 `type=chartex`、两条股价均拒写
- **O1** 🚫
  - 领域层不做原样保存，与 xlsx 清单相同
  - 零脏写回见 OPC 库测试
  - PowerPoint 另存 create 产物见 `pptx-O1-ppt-resave`

## create

用 `docs/examples/pptx-create.json` 生成 `deck.pptx`。

- **C1**
  - 检查：PowerPoint 打开
  - 通过标准：无修复提示、无「文件损坏」
- **C3**
  - 检查：第一页
  - 通过标准：两个文本框，可见文字分别为 `标题`、`hello`
- **C4**
  - 检查：第二页
  - 通过标准：一个文本框，可见文字为 `第二页`
- **C5**
  - 检查：带图 id
  - 通过标准：用 `docs/examples/pptx-create-pictures.json`（换成本机 PNG）生成
  - inspect `pictures: true` 时第一张 id 为 `slide:1/pic:3`
- **C6**
  - 检查：同一页第二张
  - 通过标准：同上，名单里有两张图
- **C7**
  - 检查：PowerPoint 打开带图产物
  - 通过标准：无修复
  - 图能看见
- **C8**
  - 检查：带图表 id
  - 通过标准：用 `docs/examples/pptx-create-charts.json` 生成
  - inspect `charts: true` 时第一张 id 为 `slide:1/chart:3`，`type` 为 `col`
- **C9**
  - 检查：系列与 embedding
  - 通过标准：系列值与 `exportChartData` 写出的表一致
- **C10**
  - 检查：PowerPoint 打开带图表产物
  - 通过标准：无修复
  - 图能看见
- **C11**
  - 检查：带备注
  - 通过标准：用 `docs/examples/pptx-create-notes-comments.json` 生成
  - inspect 无 `notes` 时第一页 `hasNotes` 为 `true`，第二页为 `false`
  - `scope.notes` 时第一页 `body` 为 `演讲者备注`
- **C12**
  - 检查：带批注
  - 通过标准：同上
  - 无名单时根与第一页 `commentCount` 为 `1`，第二页为 `0`
  - `scope.comments` 时第一条 `text` 为 `一条批注`、`author` 为 `Alice`
- **C13**
  - 检查：PowerPoint 打开带备注与批注产物
  - 通过标准：无修复
  - 备注窗格可见 `演讲者备注`

create 补齐节、形状级超链接、内嵌视频 / 音频、页切换和预设进入淡入。不算放映引擎。脚本：`manual-check/office/cases/pptx-create-remain.ps1`。编号用 Cr（不要跟 C1–C13、Cs、Po 撞）。

- **Cr1**
  - 检查：节
  - 通过标准：两页 create 带 `sections`：第一节名「开场」从第 1 页起，第二节名「内容」从第 2 页起
  - COM 节名对上
  - 无 Repair
- **Cr2**
  - 检查：超链接
  - 通过标准：第一页第一框带 `url`
  - inspect `hyperlinkCount=1`
  - COM 单击地址对上
- **Cr3**
  - 检查：媒体
  - 通过标准：第二页 `videos` 本地 MP4 后 COM Type=16
  - `audios` 本地 WAV 后 `audioCount=1`
  - 无 Repair
- **Cr4**
  - 检查：切换
  - 通过标准：第一页 `transition.type=fade` 后 inspect 对上
  - COM EntryEffect 为淡入
- **Cr5**
  - 检查：动画与拒写
  - 通过标准：第一页 `anims` 绑第一框 fade 后 COM MainSequence.Count=1
  - `slides[]` 上 `hyperlinks` / `oleObjects`、`transition.type=null`、xlsx 上 `sections` 拒写

## inspect

对上面的文件发 `docs/examples/pptx-inspect-slides.json`（无 scope）和 `docs/examples/pptx-inspect.json`（`scope.slide` 为 `1`）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `pptx:presentation`，`slideCount` 为 `2` |
| I2 | 页名单 | 两页 id 为 `slide:1`、`slide:2`，`index` 为 `1`、`2` |
| I3 | 无 scope | 只发 path 时各页都没有 `shapes` 字段，仍有 `slideCount`、`pictureCount` 和 `slides` |
| I4 | 形状 id | 第一页两个形状 id 为 `slide:1/shape:{n}`，`stable` 为 `true` |
| I5 | 第一页文字 | 两个形状的 `text` 分别为 `标题`、`hello` |

## apply

准备一份 PowerPoint 亲手另存的样例（至少两页，第一页至少两个文本框，其中一个带加粗或别的 run 样式）。对它执行 `setShapeText`，改第一页某一个文本框的文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft PowerPoint 能打开，无修复提示 |
| A2 | 目标形状 | 被点名的那个文本框可见文字变成请求里的 `text`。源文件若有多段，改成不含换行的 `text` 之后，框里不应再多出空行 |
| A3 | 同页未改形状 | 第一页其余文本框文字和位置还在 |
| A4 | 样式 | 目标形状若原先有加粗等 run 样式，改完后看起来还在（允许整框变成单一 run，但加粗还在） |
| A5 | 其余页 | 第二页文字和版式看起来不变；有动画的样例里动画仍绑在原形状上 |

## 幻灯片图

源文件必须是 PowerPoint 亲手另存，至少一张 `p:pic`。另备一张本地 PNG。不要用库 `create` 产物当带图样例。脚本：`manual-check/office/cases/pptx-pictures.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pp1 | inspect | 无 `pictures` 时根与各页有 `pictureCount`、没有名单；`pictures: true` 时 id 为 `slide:…/pic:…` |
| Pp2 | 换图 | `replacePicture` 后无修复；位置尺寸不变 |
| Pp3 | 插图 | `insertPicture` 往一页插图，能看见且无修复 |
| Pp4 | 删图 | `deletePicture` 后该图消失，无修复 |
| Pp5 | 尺寸 | `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 误差） |
| Pp6 | 位置 | `setPicturePos` 后图跟着走，无修复 |
| Pp7 | 导出 | `exportMedia` 与 `/ppt/media/…` 字节一致。不强制开 PowerPoint |

## 嵌入 SVG

幻灯片图、备注图、批注内图可带 Office 2016 `asvg:svgBlip`。不算渲染引擎，不把 SVG 原文放进 inspect 树。
手测脚本 `manual-check/office/cases/pptx-svg.ps1`。编号用 Pvg，避免和 Word 的 Sv 混号。
COM 认 `Type=28`（`msoGraphic`），不是普通图的 `13`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pvg1 | 插入 | `insertPicture` 把本地 SVG 写入第一页后，树上 `contentType` 为 `image/svg+xml`，无修复。COM `Type=28` |
| Pvg2 | 导出 | 对该图 `exportMedia` 得到 SVG 原字节，`contentType` 为 `image/svg+xml` |
| Pvg3 | create | `slides[].pictures[].file` 指向 SVG 后，树上有 `contentType`，无修复 |
| Pvg4 | 换成位图 | `replacePicture` 换成 PNG 后没有 `contentType`，包内没有 `.svg`，无修复 |
| Pvg5 | 备注 | 往 `slide:1/notes` 插 SVG 后，备注图名单末尾有 `contentType`，无修复 |
| Pvg6 | 拒错 | `file` 不是图片得 `InvalidField`，未写文件。不强制开 PowerPoint |

## 嵌入 EMF / WMF / TIFF

幻灯片图、备注图、批注内图按文件头认 EMF、WMF、TIFF，写入原字节。不要 PNG 占位，不要 `svgBlip`。不算渲染引擎。
手测脚本 `manual-check/office/cases/pptx-emf.ps1`。编号用 Pem。
COM 认 `Type=13`（`msoPicture`），不是 SVG 的 `28`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pem1 | 插入 | `insertPicture` 把本地 EMF 写入第一页后，树上 `contentType` 为 `image/x-emf`，无修复。COM `Type=13` |
| Pem2 | 导出 | 对该图 `exportMedia` 得到 EMF 原字节，`contentType` 为 `image/x-emf` |
| Pem3 | create | `slides[].pictures[].file` 指向 WMF 后，树上 `contentType` 为 `image/x-wmf`，无修复 |
| Pem4 | 换成位图 | `replacePicture` 换成 PNG 后没有 `contentType`，包内没有 `.emf`，无修复 |
| Pem5 | 备注 | 往 `slide:1/notes` 插 TIFF 后，备注图名单末尾 `contentType` 为 `image/tiff`，无修复 |
| Pem6 | 拒错 | `file` 不是图片得 `InvalidField`，未写文件。不强制开 PowerPoint |

## 幻灯片 3D 模型

幻灯片与备注页上的 `am3d` AlternateContent。只认 GLB。不进 `pictures`。Fallback 海报不进 `pictures`。不算 3D 渲染引擎。
手测脚本 `manual-check/office/cases/pptx-model3d.ps1`。编号用 P3d。
COM 认 `Type=30`（`mso3DModel`）。批注内本期不插。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| P3d1 | 插入 | `insertModel3d` 把本地 GLB 写入第一页后，树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，无修复。COM `Type=30` |
| P3d2 | 导出 | 对该 3D id `exportMedia` 得到 GLB 原字节，`contentType` 为 `model/gltf.binary` |
| P3d3 | create | `slides[].model3ds[].file` 指向 GLB 后，树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，无修复 |
| P3d4 | 替换 | `replaceModel3d` 换成另一份 GLB 后 id 不变，导出是新字节，无修复 |
| P3d5 | 备注 | 往 `slide:1/notes` 插 GLB 后，备注 `model3ds` 末尾 `contentType` 为 `model/gltf.binary`，根上 `model3dCount` 仍为 0，无修复 |
| P3d6 | 拒错 | `file` 不是 GLB 得 `InvalidField`，未写文件。不强制开 PowerPoint |

## 幻灯片图表

源文件必须是 PowerPoint 亲手另存，至少一张柱形图。不要用库 `create` 产物当唯一样例。脚本：`manual-check/office/cases/pptx-charts.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pc1 | inspect | 无 `charts` 时根与各页有 `chartCount`、没有名单；`charts: true` 时 id 为 `slide:…/chart:…`，系列值与 embedding 一致 |
| Pc2 | 改系列 | `setChartSeries` 后无修复；数据表是新值 |
| Pc3 | 插图 | `insertChart` 后能看见且无修复 |
| Pc4 | 删图 | `deleteChart` 后该图消失，无修复 |
| Pc5 | 改类型 | 改折线后按折线显示（允许样式重置），无修复 |
| Pc6 | 尺寸或位置 | `setChartSize` 或 `setChartPos` 后位置或尺寸对上，无修复 |
| Pc7 | 导出 | `exportChartData` 与 embedding 字节一致。不强制开 PowerPoint |

## 图表剩余

手测脚本 `manual-check/office/cases/pptx-charts-remain.ps1`。编号用 Pr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pr1 | 趋势线 | ✅ `insertTrendline` 后 PowerPoint 无修复，COM 能看见趋势线 |
| Pr2 | 改删趋势线 | ✅ `setTrendline` / `deleteTrendline` 后无修复 |
| Pr3 | 次轴 | ✅ `setChartSeriesAxis` 后 `axis` 为 `secondary`，无修复 |

## 图表稀有类型

手测脚本 `manual-check/office/cases/pptx-charts-rare.ps1`。编号用 Pq。

- **Pq1**
  - 检查：插入股价
  - 通过标准：✅ `insertChart` `type=stock` 三条系列后 PowerPoint 无修复，`type` 为 `stock`
- **Pq2**
  - 检查：插入曲面
  - 通过标准：✅ `insertChart` `type=surface` 三条系列后 PowerPoint 无修复，`type` 为 `surface`
- **Pq3**
  - 检查：插入复合饼
  - 通过标准：✅ `insertChart` `type=ofPie` 一条系列后 PowerPoint 无修复
  - `ofPieType=bar` 为复合条
- **Pq4**
  - 检查：改类型
  - 通过标准：✅ 股价三条转 `col` 成功
  - 复合饼转 `pie` 成功
  - 无修复
- **Pq5**
  - 检查：仍拒绝
  - 通过标准：✅ 对股价 `addChartSeries` 得 `NotImplemented`
  - 源无 chartex 时 `insertChart type=chartex` 得 `InvalidField`
  - 两条系列的股价 `insertChart` 得 `InvalidField`
  - 均未写坏文件

## SmartArt

源文件必须是 PowerPoint 亲手另存，至少一张列表型。不要用库 `create` 产物当唯一样例。脚本：`manual-check/office/cases/pptx-smartart.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Psa1 | inspect | 另存列表型；inspect 的点文本与屏幕上能看见的字一致 |
| Psa2 | 改字 | `setSmartArtText` 后无修复，目标点的字变了 |
| Psa3 | 增删点 | 增删点后条数对，PowerPoint 重排 |
| Psa4 | 删整张 | 删除整张后 Fallback 也不在（有则必须一起少） |
| Psa5 | create | 带一张 `list`，打开无修复 |
| Psa6 | 跨家族 | 列表改成组织图后可见点文字还在，无修复 |
| Psa7 | 库外版式 | 当前 uniqueId 不在库内三族时 `setSmartArtLayout` 得 `NotImplemented`，文件未写坏 |

## 备注

源文件必须是 PowerPoint 亲手另存，至少一页有演讲者备注文字。不要用库 `create` 产物当带备注的唯一样例。脚本：`manual-check/office/cases/pptx-notes.ps1`。

- **Pn1**
  - 检查：inspect
  - 通过标准：无 `notes` 时各页有 `hasNotes`、没有 `notes` 对象
  - `notes: true` 时有备注页的 `notes.id` 为 `slide:…/notes`，能看见演讲者备注文字
- **Pn2**
  - 检查：改字
  - 通过标准：`setNotesText` 后备注窗格是新字，无修复
- **Pn3**
  - 检查：互不影响
  - 通过标准：改备注后幻灯片上的 `setShapeText` 目标仍是原字
  - 改幻灯片形状不改备注
- **Pn4**
  - 检查：备注插图
  - 通过标准：往 `slide:{n}/notes` 插图后无修复，图在备注页上
- **Pn5**
  - 检查：备注图表
  - 通过标准：往 `slide:{n}/notes` 插柱形图后 id 为 `slide:…/notes/chart:…`
  - 改系列后 embedding 对得上
  - 无修复
  - 不并进幻灯片 `chartCount`
- **Pn6**
  - 检查：备注 SmartArt
  - 通过标准：往 `slide:{n}/notes` 插 `list` 后 id 为 `slide:…/notes/smartdiagram:…`
  - 无修复
  - 不并进幻灯片 `smartArtCount`
- **Pn7**
  - 检查：备注表名单
  - 通过标准：无 `notes` 时没有 `notes` 对象
  - `notes` 且 `tables` 时 id 为 `slide:…/notes/table:…`
  - 不并进幻灯片 `tableCount`
- **Pn8**
  - 检查：备注插表
  - 通过标准：往 `slide:{n}/notes` 插 2×2 表后 id 为 `slide:…/notes/table:…`
  - 备注页 COM 能看见表
  - 无修复
  - 不并进幻灯片 `tableCount`
- **Pn9**
  - 检查：改格
  - 通过标准：`setTableCellText` 后该格是新字
  - 无修复
- **Pn10**
  - 检查：删除
  - 通过标准：`deleteTable` 后备注 `tableCount` 为 0
  - 无修复
- **Pn11**
  - 检查：拒写
  - 通过标准：对形状 id 发 `insertTable` 得 `InvalidId`
  - 对备注表 id 发 `insertTable` 得 `InvalidId`
  - xlsx 得 `InvalidField`
  - 未写文件

## 批注

源文件必须是 PowerPoint 亲手另存，第一页至少两条线程化批注，其中一条带回复。不要用库 `create` 产物当带批注的唯一样例。脚本：`manual-check/office/cases/pptx-comments.ps1`。

- **Pcm1**
  - 检查：inspect
  - 通过标准：无 `comments` 时根与各页有 `commentCount`、没有名单
  - `comments: true` 时 id 为 `slide:…/comment:{guid}`，作者与文字与审阅窗格对得上，回复挂在 `replies`
- **Pcm2**
  - 检查：改字
  - 通过标准：`setCommentText` 后无修复，目标批注是新字
- **Pcm3**
  - 检查：插入
  - 通过标准：`insertComment` 后审阅窗格能看见新批注，无修复
- **Pcm4**
  - 检查：删除
  - 通过标准：`deleteComment` 后该条消失，无修复
- **Pcm5**
  - 检查：回复与完成
  - 通过标准：`replyComment` 后线程能看见回复
  - `setCommentDone` 后完成标记对上，无修复
- **Pcm6**
  - 检查：批注内图（只读）
  - 通过标准：注入 `a:blip` 后 `pictureCount` 为 1、id 为 `slide:…/comment:{guid}/pic:1`
  - `exportMedia` 字节与媒体部件一致

## 批注内插图

对批注 id 发 `insertPicture` 写 `a:blip`，不写 `p:pic`。不算放映引擎。脚本：`manual-check/office/cases/pptx-comment-pics.ps1`。编号用 Pcip（不要和 Pcm、Pik 撞）。

- **Pcip1**
  - 检查：插入
  - 通过标准：`insertPicture` 后该条 `pictureCount` 为 1、id 为 `{批注 id}/pic:1`
  - 不并进幻灯片 `pictureCount`
- **Pcip2**
  - 检查：打开
  - 通过标准：无 Repair
  - ZIP 里 `a:blip` 在 `txBody` 之后，comments `.rels` 指向 `/ppt/media/…`
- **Pcip3**
  - 检查：换图
  - 通过标准：`replacePicture` 后 `exportMedia` 与新图字节一致
  - 无 Repair
- **Pcip4**
  - 检查：删除
  - 通过标准：`deletePicture` 后 `pictureCount` 为 0，批注文字还在
  - 无 Repair
- **Pcip5**
  - 检查：拒写
  - 通过标准：批注 id 上带 `x`/`y` 得 `InvalidField`
  - 批注内图 id 发 `setPictureSize` 得 `InvalidId`
  - xlsx 上批注 id 得 `InvalidId`
  - 未写文件

## 批注墨迹

现代批注里带本地名为 `ink` 的标记。不把墨迹当位图列出。编号用 Pik。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pik1 | 名单 | ✅ `kind` 为 `ink` |
| Pik2 | 元数据 | ✅ `setCommentMeta` 后作者变了，无修复 |
| Pik3 | 完成 | ✅ `setCommentDone` 后 `done` 为 true，无修复 |
| Pik4 | 改字仍拒 | ✅ 没有可见文本时 `setCommentText` 得 `NotImplemented`，文件未写坏 |

## 文档属性

与 docx 同一套字段。脚本 `manual-check/office/cases/pptx-docprops.ps1`。编号用 Pd（不要跟图片的 Pp 组混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pd1 | inspect | 无 scope 时根上有 `props`；create 产物 `creator` 为 `ooxp` |
| Pd2 | 改标题 | `setProps` 只改 `title` 后 PowerPoint 文件属性标题是新字；未点名的公司名还在；无 Repair |
| Pd3 | 自定义 | `setCustomProp` 后树上 id 为 `prop:…`；PowerPoint 自定义属性能看见 |
| Pd4 | 删除 | `deleteCustomProp` 后名单没有该项 |
| Pd5 | 拒错 id | 对形状 id 发 `setProps` 得 `InvalidId`。不强制开 PowerPoint |

## 幻灯片管理

用库 create 产物再 apply。顺序跟 `sldIdLst`。脚本 `manual-check/office/cases/pptx-slides.ps1`。编号用 Pm。

- **Pm1**
  - 检查：inspect
  - 通过标准：create 两页
  - `slideCount=2`，两页 `hidden=false`，id 为 `slide:1`、`slide:2`
- **Pm2**
  - 检查：插入
  - 通过标准：`insertSlide` `after=slide:1` 后 `slideCount=3`
  - 新页是 `slide:2` 且无文本框
  - 原第二页变成 `slide:3` 文字还在
  - PowerPoint 无 Repair
- **Pm3**
  - 检查：重排
  - 通过标准：把 `slide:3` 移到最前
  - inspect 第一页是原先那页文字
  - 无 Repair
- **Pm4**
  - 检查：隐藏
  - 通过标准：`setSlideHidden` `hidden: true` 后 inspect `hidden=true`
  - COM `SlideShowTransition.Hidden` 为真
  - 无 Repair
- **Pm5**
  - 检查：删除
  - 通过标准：只剩一页时 `deleteSlide` 得 `InvalidField` 且未写文件
  - 三页里删一页后计数为 2、无 Repair
  - 缺 `hidden` 得 `InvalidField`

## 形状增删与几何

插文本框、改位置尺寸填色、删除。不删旧形状再重建。脚本 `manual-check/office/cases/pptx-shapes.ps1`。编号用 Ps（不要跟图的 Pp、幻灯片管理的 Pm 混）。

- **Ps1**
  - 检查：插入
  - 通过标准：✅ 对 create 产物 `insertShape` 后新框可见
  - inspect id 为 `slide:…/shape:…` 且 `stable`
  - PowerPoint 无 Repair
- **Ps2**
  - 检查：几何与填色
  - 通过标准：✅ `setShapePos` / `setShapeSize` 后位置尺寸对上
  - `setShapeFill` / `setShapeLine` 后填充与边框能看见
  - 无 Repair
- **Ps3**
  - 检查：改字
  - 通过标准：✅ 对新框 `setShapeText` 后文字是请求值
  - 旧框文字还在
- **Ps4**
  - 检查：删除
  - 通过标准：✅ `deleteShape` 后该框消失
  - 同页未删形状还在、`cNvPr/@id` 不变
  - 无 Repair
- **Ps5**
  - 检查：动画仍绑
  - 通过标准：✅ 对 PowerPoint 另存样例只插新框、不删带动画的旧框
  - 旧框动画还在
  - 无 Repair
  - 对形状 id 发 `insertShape`、缺成对 `cx` 或缺 `color` 得 `InvalidField` 且未写文件

## 形状几何剩余

非矩形预设几何、连接线、组合。脚本 `manual-check/office/cases/pptx-shape-geom.ps1`。编号用 Sg（不要跟 Ps、节的 Psc 混）。

- **Sg1**
  - 检查：椭圆
  - 通过标准：✅ 对 create 产物 `insertShape` `prst=ellipse` 后 inspect `prst` 为 `ellipse`，id 为 `slide:…/shape:…`
  - COM AutoShapeType=9
  - 无 Repair
- **Sg2**
  - 检查：连接线
  - 通过标准：✅ `insertConnector` 后 inspect `connectors: true` 时 id 为 `slide:…/cxn:…` 且 `stable`
  - COM Connector 为真
  - 无 Repair
- **Sg3**
  - 检查：组合
  - 通过标准：✅ `groupShapes` 两个文本框后 COM Type=6
  - inspect 子形状 id 仍在
  - 无 Repair
- **Sg4**
  - 检查：解开
  - 通过标准：✅ `ungroup` 后组合解开，子 id 还在
  - 无 Repair
- **Sg5**
  - 检查：拒错
  - 通过标准：✅ 连接线几何名给 `insertShape`、`groupShapes` 只给一个成员、对形状 id 发 `insertConnector` 得 `InvalidField` 或 `InvalidId`
  - 均未写文件

## 幻灯片表格

DrawingML 表（`a:tbl` 包在 `p:graphicFrame` 里）。不要把表编进 `shapes` / `charts` / `pictures`。源文件：PowerPoint 亲手另存，第一页至少一张表。
不要用库 `create` 产物当带表的唯一样例。脚本：`manual-check/office/cases/pptx-tables.ps1`。编号用 Pt（不要跟形状的 Ps、图的 Pp 混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pt1 | inspect | ✅ 无 `tables` 时根与各页有 `tableCount`、没有名单；`tables: true` 时 id 为 `slide:…/table:…`，能看见单元格 `text` |
| Pt2 | 改格 | ✅ `setTableCellText` 后目标格可见字是请求值，无 Repair |
| Pt3 | 增行 | ✅ `insertRow` 后行数加 1，无 Repair |
| Pt4 | 插入 | ✅ 对 create 产物 `insertTable` 后新表可见；inspect id 为 `slide:…/table:…` 且 `stable`；无 Repair |
| Pt5 | 删除 | ✅ `deleteTable` 后该表消失；同页未删形状还在；无 Repair |
| Pt6 | 拒错 | ✅ 对形状 id 发 `insertTable` 得 `InvalidId`；缺 `rows`、只剩一行时 `deleteRow` 得 `InvalidField`；均未写文件 |

## 幻灯片切换

只动 `<p:transition>`，不写 `<p:timing>`。源文件：PowerPoint 亲手另存一页带淡入切换。脚本：`manual-check/office/cases/pptx-transition.ps1`。编号用 Tn。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tn1 | inspect | ✅ 无切换的页 `transition` 为 `null`；另存淡入页 `type` 为 `fade`，有 `dur` |
| Tn2 | 设切换 | ✅ 对 create 产物 `setTransition` `type=fade` 后放映能看见淡入；inspect `type=fade`；无 Repair |
| Tn3 | 改字不丢 | ✅ 对另存带切换样例只 `setShapeText`，切换还在；无 Repair |
| Tn4 | 清除 | ✅ `type: null` 后 `transition` 为 `null`；无 Repair |
| Tn5 | 拒错 | ✅ 对形状 id 发 `setTransition` 得 `InvalidId`；`type` 为 `other` 得 `InvalidField`；均未写文件 |

## 切换剩余

自动进片、效果方向与 `p15:prstTrans`。仍只动 `<p:transition>`。脚本：`manual-check/office/cases/pptx-transition-remain.ps1`。编号用 Tr。

- **Tr1**
  - 检查：自动进片
  - 通过标准：✅ create 产物 `setTransition` `type=fade` `advTm=2000` 后 inspect `advTm` 为 2000、`advClick` 为 true
  - COM `AdvanceOnTime` 为真、`AdvanceTime` 为 2
  - 无 Repair
- **Tr2**
  - 检查：方向
  - 通过标准：✅ `type=wipe` `dir=r` 后 inspect `dir` 为 `r`
  - COM EntryEffect 为 2819
  - 无 Repair
- **Tr3**
  - 检查：prstTrans
  - 通过标准：✅ `type=fallOver` 后 inspect `type` 为 `fallOver`（不是 `other`）
  - COM EntryEffect 为 3934
  - 无 Repair
- **Tr4**
  - 检查：改字不丢
  - 通过标准：✅ 先写 `wipe` `dir=r` 且 `advTm=2000`，再只 `setShapeText`，`dir` 与 `advTm` 还在
  - 无 Repair
- **Tr5**
  - 检查：拒错
  - 通过标准：✅ 对 `fade` 发 `dir` 得 `InvalidField`
  - 对 `fade` 发 `invX` 得 `InvalidField`
  - 均未写文件

## 动画清单

只读 `<p:timing>` 清单。写入见 `.plan/2026年-9月-29日-✅-pptx-动画写入.md`。旧「禁止 apply 写 timing」作废；`setTiming` 仍报 `UnsupportedOp`。
源文件：PowerPoint 亲手另存一页带进入淡入动画。脚本：`manual-check/office/cases/pptx-timing.ps1`。编号用 Am。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Am1 | inspect | ✅ 无 `timing` 时根与各页有 `animCount`、没有名单；create 产物为 0；另存进入动画 `timing: true` 时 `target` 对上第一框 shape id |
| Am2 | COM | ✅ `TimeLine.MainSequence.Count` 与 `animCount` 一致；`trigger` 为 `click`；无 Repair |
| Am3 | 改字不丢 | ✅ 对另存带动画样例只 `setShapeText`，名单还在；无 Repair |
| Am4 | 无入口 | ✅ `setTiming` 得 `UnsupportedOp`，未写文件 |
| Am5 | 拒错 | ✅ xlsx / docx 的 `scope.timing` 得 `InvalidField` |

## 动画写入

按模板给对象加、删预设进入淡入。不实现放映引擎，也不接受整棵时间轴 JSON。脚本：`manual-check/office/cases/pptx-anim-write.ps1`。编号用 Aw。

- **Aw1**
  - 检查：插入
  - 通过标准：✅ create 产物 `insertAnim` `type=fade` 后 inspect 名单一条，`id` 为 `slide:1/anim:1`，`target` 对上第一框，COM `MainSequence.Count` 为 1，无 Repair
- **Aw2**
  - 检查：再插
  - 通过标准：✅ 另存已有动画的页再插一条，旧的还在，新的在末尾，COM Count 加 1，无 Repair
- **Aw3**
  - 检查：删除
  - 通过标准：✅ `deleteAnim` 后 Count 少 1
  - 删光则没有 `<p:timing>`，无 Repair
- **Aw4**
  - 检查：改字不丢
  - 通过标准：✅ 只 `setShapeText`，名单还在，无 Repair
- **Aw5**
  - 检查：拒错
  - 通过标准：✅ 错对象 id 得 `InvalidId`
  - `type` 不在白名单得 `InvalidField`
  - xlsx / docx 发 `insertAnim` 得 `InvalidField`
  - 均未写文件
  - `setTiming` 仍为 `UnsupportedOp`

## 母版与版式

inspect 列出 slideMaster / slideLayout 与占位符。apply 改占位符文字与几何，并用 `setSlideLayout` 只换页的版式关系。不算母版排版引擎。
脚本：`manual-check/office/cases/pptx-masters.ps1`。编号用 Mm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Mm1 | inspect | ✅ 无 `masters` 时根有 `masterCount`、没有名单；create 产物 `masterCount=1`、版式名「空白」；另存默认文稿列出「标题幻灯片」「标题和内容」「空白」等 |
| Mm2 | 改占位符字 | ✅ 对 `master:1/layout:2/shape:2` `setShapeText` 后文字为「改过的正文」；无 Repair |
| Mm3 | 换版式 | ✅ 插「用户框」后换成 `master:1/layout:7`（空白），用户框 id 为 `slide:1/shape:4`；无 Repair |
| Mm4 | 改几何 | ✅ 占位符 `setShapePos` 后 `x` 为 200000；无 Repair |
| Mm5 | 拒错 | ✅ xlsx / docx 的 `scope.masters` 与 `setSlideLayout` 得 `InvalidField`，未写文件 |

## 母版写入剩余

在已有母版上新建 / 删除空白版式，给版式或母版插删带 `p:ph` 的占位符，以及抄一份母版。不算母版排版引擎。脚本：`manual-check/office/cases/pptx-master-write.ps1`。编号用 Mw。

- **Mw1**
  - 检查：新建版式
  - 通过标准：create 产物 `insertLayout` 后 `masterCount` 仍为 1，版式名单多一条且 `kind=blank`
  - COM CustomLayouts.Count 加 1
  - 无 Repair
- **Mw2**
  - 检查：插占位符
  - 通过标准：对新版式 `insertPlaceholder` `type=body` 后 inspect 能看见 `type=body` 的占位符
  - COM Type=14
  - 无 Repair
- **Mw3**
  - 检查：删占位符
  - 通过标准：`deletePlaceholder` 后该占位符没了
  - 无 Repair
- **Mw4**
  - 检查：第二份母版
  - 通过标准：`insertMaster` 后 `masterCount` 为 2，无 Repair
  - 有页占用时 `deleteMaster` / `deleteLayout` 为 `InvalidField`，未写文件
- **Mw5**
  - 检查：拒错
  - 通过标准：对页 id 发 `insertLayout` 得 `InvalidId`
  - `type` 不在白名单得 `InvalidField`
  - xlsx / docx 发 `insertLayout` 得 `InvalidField`
  - 均未写文件

## 主题与幻灯片尺寸

inspect 根上始终带 `slideSize`。`scope.masters` 时每份母版带 `theme`。apply 改 `p:sldSz` 与 theme 的 `clrScheme` / `fontScheme`。
不按新画布重排，不重写 `fmtScheme`。脚本：`manual-check/office/cases/pptx-theme-size.ps1`。编号用 Ts。

- **Ts1**
  - 检查：改尺寸
  - 通过标准：create 产物 `setSlideSize` `cx=9144000` `cy=6858000` 后 inspect `slideSize.cx` 为 9144000
  - COM SlideWidth=720
  - 无 Repair
- **Ts2**
  - 检查：形状不缩放
  - 通过标准：同上一次 apply 后第一页文本框 `x` / `y` / `cx` / `cy` 仍是 create 原值
- **Ts3**
  - 检查：主题色
  - 通过标准：`setThemeColors` `accent1=FF0000` 后 inspect 该母版 `theme.colors.accent1` 为 `FF0000`
  - 无 Repair
- **Ts4**
  - 检查：字体
  - 通过标准：`setThemeFonts` `majorLatin=Arial` 后 inspect `theme.fonts.majorLatin` 为 `Arial`
  - 无 Repair
- **Ts5**
  - 检查：拒错
  - 通过标准：对页 id 发 `setSlideSize` 得 `InvalidId`
  - `cx` 越界或 `type=other` 得 `InvalidField`
  - xlsx / docx 发 `setSlideSize` 得 `InvalidField`
  - 均未写文件

## 讲义

inspect 列出讲义母版页眉、页脚、日期、页码槽位。apply 用 `setHandoutText` 改字，没有 handoutMaster 则新建最小部件。不算讲义打印布局。
脚本：`manual-check/office/cases/pptx-handout.ps1`。编号用 Ho。

- **Ho1**
  - 检查：inspect
  - 通过标准：无 `handout` 时根有 `hasHandout`、没有 `handout` 对象
  - create 产物 `hasHandout=false`
  - `scope.handout: true` 时 `handout` 为 `null`
- **Ho2**
  - 检查：改页脚
  - 通过标准：`setHandoutText` 写 `footer` 后 inspect 能看见新字
  - PowerPoint 无 Repair
- **Ho3**
  - 检查：改页眉
  - 通过标准：同一次或另一次 `setHandoutText` 写 `header` 后能看见新字
  - 无 Repair
- **Ho4**
  - 检查：新建
  - 通过标准：对 create 产物（原先没有讲义母版）发 `setHandoutText` 后 `hasHandout=true`
  - 无 Repair
- **Ho5**
  - 检查：拒错
  - 通过标准：xlsx / docx 的 `scope.handout` 与 `setHandoutText` 得 `InvalidField`，未写文件

## 超链接

形状级 `a:hlinkClick`。inspect 计 `hyperlinkCount`，`scope.hyperlinks` 列出链。apply 能改地址、插、删。内部跳转用 `slide:{n}`。不写放映宏动作。
脚本：`manual-check/office/cases/pptx-hyperlinks.ps1`。编号用 Phl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Phl1 | inspect | 无 `hyperlinks` 时根与各页有 `hyperlinkCount`、没有名单；create 产物为 0；`scope.hyperlinks: true` 时名单为空数组 |
| Phl2 | URL | `insertHyperlink` 给第一框 `url` 后 PowerPoint 无 Repair；该形状单击超链接地址对上；inspect `kind=url` |
| Phl3 | 改地址 | `setHyperlink` 改 URL 后 COM 地址是新值；无 Repair |
| Phl4 | 删链留字 | `deleteHyperlink` 后链没了，框上字还在；无 Repair |
| Phl5 | 拒错 | 对页 id 发 `insertHyperlink` 得 `InvalidId`；缺 `url`/`file`/`slide` 得 `InvalidField`，未写文件 |

## run 级超链接

run 的 `a:rPr/a:hlinkClick`。与形状级编进同一份 `hyperlinks`。`insertHyperlink` 的 `on` 为 `run`。不写 `a:hlinkHover`，不写下划线。
脚本：`manual-check/office/cases/pptx-run-links.ps1`。编号用 Phr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Phr1 | inspect | `insertHyperlink` `on=run` 后 `hyperlinkCount=1`，`on=run`，`host` 为第一框，`text` 对上 |
| Phr2 | XML | PowerPoint 无 Repair；ZIP 里链在 `a:rPr/a:hlinkClick`，`p:cNvPr` 上没有 |
| Phr3 | 改地址 | `setHyperlink` 改 URL 后 COM 文本范围地址是新值；无 Repair |
| Phr4 | 删链留字 | `deleteHyperlink` 后没有 `a:hlinkClick`，框上字还在；无 Repair |
| Phr5 | 拒错 | `on=other` 得 `InvalidField`；xlsx 出现 `on` 得 `InvalidField`；页 id 得 `InvalidId`，均未写文件 |

## 媒体

幻灯片上的视频 / 音频是带 `a:videoFile` / `a:audioFile` 的 `p:pic`，与普通图分家。inspect 计 `videoCount` / `audioCount`，`scope.media` 列名单。
apply 能插本地 MP4 / WAV、删除。`exportMedia` 写出媒体部件，不是海报。不写 timing。源文件必须是 PowerPoint 亲手另存，第一页至少一段视频。
脚本：`manual-check/office/cases/pptx-media.ps1`。编号用 Pmd。

- **Pmd1**
  - 检查：inspect
  - 通过标准：无 `media` 时根与各页有 `videoCount` / `audioCount`、没有名单
  - `scope.media: true` 时视频 id 为 `slide:1/video:…`，不并进 `pictureCount`
- **Pmd2**
  - 检查：导出
  - 通过标准：`exportMedia` 与 `/ppt/media/mediaN.*` 字节一致
  - 不强制开 PowerPoint
- **Pmd3**
  - 检查：插视频
  - 通过标准：对 create 产物 `insertVideo` 后 PowerPoint 无 Repair
  - COM Type=16
  - inspect id 为 `slide:1/video:…`
- **Pmd4**
  - 检查：插音频
  - 通过标准：`insertAudio` 一段 WAV 后无 Repair
  - `audioCount` 加 1
- **Pmd5**
  - 检查：删除
  - 通过标准：`deleteMedia` 后该对象消失，无 Repair
  - 对页 id 发 `insertVideo` 得 `InvalidId`，WAV 当视频得 `InvalidField`，未写文件

## 幻灯片节

PowerPoint 的节是 `p14:sectionLst` 分组元数据，页顺序仍以 `sldIdLst` 为准。inspect 计 `sectionCount`，`scope.sections` 列名单。
apply 能新建节、改名、删节（页还在）、把页划进某节。不写节的 UI 颜色。脚本：`manual-check/office/cases/pptx-sections.ps1`。编号用 Psc。

- **Psc1**
  - 检查：inspect
  - 通过标准：无 `sections` 时根上有 `sectionCount`、没有名单
  - 另存两节以上时 `scope.sections: true` 的 id 为 `section:1`，`slides` 对得上页序
- **Psc2**
  - 检查：建节划页
  - 通过标准：两页 create 产物连续 `insertSlideSection` 两节，再把第二页划进第二节后 PowerPoint 无 Repair
  - COM 第二节名为「内容」、从第 2 页起
- **Psc3**
  - 检查：改名
  - 通过标准：`setSlideSectionName` 后左侧节名是新字
  - 无 Repair
- **Psc4**
  - 检查：删节留页
  - 通过标准：`deleteSlideSection` 后页还在、对应节名没了
  - 无 Repair
- **Psc5**
  - 检查：拒错
  - 通过标准：对页 id 发 `insertSlideSection` 得 `InvalidId`
  - `start` 与 `after` 同给得 `InvalidField`，未写文件

## 自定义放映

inspect 计 `customShowCount`，`scope.customShows` 列名单。apply 能新建、改名、改页序、删除自定义放映。不算放映引擎，不改 `showPr`。
删页时按 presentation 关系 `r:id` 清名单。脚本：`manual-check/office/cases/pptx-custom-show.ps1`。编号用 Cs。

- **Cs1**
  - 检查：inspect
  - 通过标准：无 `customShows` 时根上有 `customShowCount`、没有名单
  - `insertCustomShow` 后 `scope.customShows: true` 的 id 为 `show:1`，`name` 与 `slides` 对得上
- **Cs2**
  - 检查：新建
  - 通过标准：三页 create 产物 `insertCustomShow` `name=Tour`、`slides` 为第 1、3 页后 PowerPoint 无 Repair
  - COM `SlideShowSettings.SlideShowName` 能设成 Tour
- **Cs3**
  - 检查：改名改页序
  - 通过标准：`setCustomShow` 改名并换成第 3、1 页后 inspect 对上
  - 无 Repair
- **Cs4**
  - 检查：删除与删页
  - 通过标准：`deleteCustomShow` 后名单少一条、无 Repair
  - 另建含第一页的放映再 `deleteSlide` 后对应 `r:id` 从名单消失，空名单仍留那一条
- **Cs5**
  - 检查：拒错
  - 通过标准：对页 id 发 `insertCustomShow` 得 `InvalidId`
  - 空名 / 重名 / xlsx / docx 得 `InvalidField`，未写文件

## OLE

inspect 计 `oleObjectCount`，`scope.oleObjects` 列名单。`exportMedia` 按 OLE id 写出 embeddings 原字节，不是预览图。apply 能从本地文件插入、能删除。
不算放映引擎，不激活 OLE 服务器。预览 `p:pic` 不进 `pictures`。源文件必须是 PowerPoint 亲手另存，第一页至少一个嵌入对象。
脚本：`manual-check/office/cases/pptx-ole.ps1`。编号用 Po（不要用 Ol，会和 docx 撞）。

- **Po1**
  - 检查：inspect
  - 通过标准：无 `oleObjects` 时根与各页有 `oleObjectCount`、没有名单
  - 另存嵌入 xlsx 后 `scope.oleObjects: true` 的 id 为 `slide:1/ole:…`，不并进 `pictures`
- **Po2**
  - 检查：导出
  - 通过标准：`exportMedia` 与 ZIP 内 `/ppt/embeddings/…` 字节一致
  - 不强制开 PowerPoint
- **Po3**
  - 检查：删除
  - 通过标准：`deleteOle` 后计数为 0、COM Type=7 的形状没了
  - 无 Repair
- **Po4**
  - 检查：插入
  - 通过标准：对 create 产物 `insertOle` 本地 xlsx 后无 Repair
  - COM Type=7
  - inspect id 为 `slide:1/ole:…`、`progId` 含 Excel
- **Po5**
  - 检查：拒错
  - 通过标准：对页 id 发 `deleteOle` 得 `InvalidId`
  - 对 `slide:1/ole:…` 发 `insertOle` 得 `InvalidId`
  - xlsx 发 `insertOle` 得 `InvalidField`，未写文件

## 幻灯片缩放

inspect 计 `zoomCount`，`scope.zooms` 列名单。apply 能插入幻灯片缩放 / 节缩放、能删除。不算放映引擎，不写 `<p:timing>`。
Fallback 预览 `p:pic` 不进 `pictures`。插入走 ooxp，不要 `ExecuteMso`。COM 认 `Type=-2` 且 `Name` 以 `Zoom` 开头。
脚本：`manual-check/office/cases/pptx-zoom.ps1`。编号用 Pz。

- **Pz1**
  - 检查：inspect
  - 通过标准：无 `zooms` 时根与各页有 `zoomCount`、没有名单
  - `insertZoom` 后 `scope.zooms: true` 的 id 为 `slide:1/zoom:…`，`type=slide`，`target` 为 `slide:2`，不并进 `pictures`
- **Pz2**
  - 检查：插入
  - 通过标准：对 create 三页产物 `insertZoom` 后 PowerPoint 无 Repair
  - COM `Type=-2` 且 `Name` 以 `Zoom` 开头
- **Pz3**
  - 检查：不进图
  - 通过标准：插入后 `pictureCount` 仍为 0，`scope.pictures` 为空数组
- **Pz4**
  - 检查：删除
  - 通过标准：`deleteZoom` 后计数为 0、COM 不再有 `Type=-2` 且名以 Zoom 开头的形状
  - 无 Repair
- **Pz5**
  - 检查：拒错
  - 通过标准：对页 id 发 `deleteZoom` 得 `InvalidId`
  - 对 Zoom id 发 `insertZoom` 得 `InvalidId`
  - `type=summary` 或目标是自己得 `InvalidField`
  - xlsx 发 `insertZoom` 得 `InvalidField`，未写文件

## 修改密码与放映保护

inspect 根上始终给出 `showPr` 与 `modifyProtection`，不要另给 scope 键。
apply 能改放映类型与循环（`setShowPr`），能在没有哈希时删掉 `p:modifyVerifier`（`setProtection` `enforced: false`）。不算放映引擎，不写哈希。
带哈希样例只 ZIP 注入，不要用 COM 打开。脚本：`manual-check/office/cases/pptx-protect.ps1`。编号用 Pw（不要跟 Pz、Po、Pd、Pr、Pp 撞）。

- **Pw1**
  - 检查：inspect
  - 通过标准：create 产物根上 `showPr.type=present`、`loop=false`、`showNarration=true`、`showAnimation=true`、`useTimings=true`
  - `modifyProtection.hasPassword=false`
  - 无 `scope` 也有这两项
- **Pw2**
  - 检查：展台
  - 通过标准：`setShowPr` `type=kiosk` `loop=true` 后 PowerPoint 无 Repair
  - COM `ShowType=3`、`LoopUntilStopped` 为真
- **Pw3**
  - 检查：窗口
  - 通过标准：`type=browse` 后 COM `ShowType=2`
  - 无 Repair
- **Pw4**
  - 检查：演讲者
  - 通过标准：改回 `present` / `loop=false` 后 COM `ShowType=1`
  - 无 Repair
- **Pw5**
  - 检查：拒错
  - 通过标准：`password` / `enforced: true` / 带哈希样例 `enforced: false` 得 `NotImplemented`
  - xlsx / docx 发 `setShowPr` 得 `InvalidField`
  - 页 id 发 `setShowPr` 得 `InvalidId`
  - 未写文件
  - 带哈希样例不要开 PowerPoint

## 字段

inspect 计 `fieldCount`，`scope.fields` 列名单。apply 能往幻灯片文本形状插入、改类型、删除 `a:fld`。不算放映引擎，不刷新缓存。create 本期不写域。备注 / 母版上的域不进本期。
脚本：`manual-check/office/cases/pptx-fields.ps1`。编号用 **Pf**（不要跟 Pw、Pz、Po、Pd 撞）。

- **Pf1**
  - 检查：inspect
  - 通过标准：create 无 `fields` 时根与各页有 `fieldCount=0`、没有名单
  - `insertField` `type=slidenum` 后 `scope.fields` 有一条，`type=slidenum`，`host` 为形状 id
- **Pf2**
  - 检查：页码
  - 通过标准：打开无 Repair
  - ZIP 里 `a:fld type="slidenum"`
- **Pf3**
  - 检查：日期
  - 通过标准：`type=datetime1` 后 XML 对上
  - 无 Repair
- **Pf4**
  - 检查：删除
  - 通过标准：`deleteField` `keep=result` 后没有 `a:fld`，缓存变成 `a:r`
  - 无 Repair
- **Pf5**
  - 检查：拒错
  - 通过标准：页 id / 备注形状 id 得 `InvalidId`
  - `type=other` 得 `InvalidField`
  - xlsx 拒 `insertField`
  - docx 式 `instr` 在 pptx 上未知字段
  - 未写文件

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改页。
