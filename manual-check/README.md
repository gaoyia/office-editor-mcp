# 人工验收

`docs/checklist-xlsx.md`、`docs/checklist-docx.md` 与 `docs/checklist-pptx.md` 要求在本机用 Microsoft Excel、Word、PowerPoint 打开文件。验收只认官方 Office，不再要求 WPS。这个目录放的就是那几轮验收的用例脚本。

## 目录

```text
manual-check/
├── lib/                  公共库，两轮共用
│   ├── bootstrap.ps1     用例只 dot-source 这一个
│   ├── common.ps1        路径、日志、结果表、调 ooxp
│   ├── office-com.ps1    绕开 WPS 抢注册，从窗口句柄挂 Office 的 COM
│   └── fixtures.ps1      用例要用的输入文件，谁缺谁造
├── office/               Microsoft Office 这一轮
│   ├── run.ps1           驱动
│   ├── cases/            一个文件对应清单里的一节
│   ├── requests/         手写的请求模板，只有三份
│   └── out/              产物、日志、生成的请求，都不入库
└── requests/             WPS 那一轮，还没拆
```

入库的是用例脚本、`requests/` 下手写的请求 JSON 和本文件。下面这些都不入库，删了再跑就会重新生成：

- `office/out/`：Office 这一轮的工作簿、文档、生成的请求、日志、结果表
- 根目录的 `*.docx`、`inspect-*.json`、`compare-*.csv`、`opc-crc-*.csv`、`wps-com-log.txt`、`ooxp-pic-*.png`：WPS 那一轮写在根下的产物
- `office/legacy/`、`_office-*.ps1`：拆分前的探测脚本

## 跑 Microsoft Office 这一轮

先在仓库根目录 `cargo build`，然后：

```powershell
pwsh -File manual-check\office\run.ps1                  # 全跑
pwsh -File manual-check\office\run.ps1 -Case docx-apply  # 只跑一个
pwsh -File manual-check\office\run.ps1 -Case 'pptx-*'    # 按通配符挑
pwsh -File manual-check\office\run.ps1 -List             # 只看有哪些用例
pwsh -File manual-check\office\run.ps1 -CloseOffice      # 跑完关掉 Word、Excel 和 PowerPoint
```

默认跑完把 `xlsx-applied.xlsx`、`docx-applied.docx` 和 `pptx-applied.pptx` 留在 Office 里，方便接着用肉眼过清单。看完想收拾干净，就加 `-CloseOffice`。

结果打在屏幕上，同时写进 `office/out/office-results.csv`，逐条日志在 `office/out/com-log.txt`。每个用例套独立的 try/catch，一个塌了不影响后面的，所以不必再为了跑完剩下的去复制脚本。

用例顺序有讲究：`xlsx-inspect` 和 `docx-inspect` 只调 ooxp、不开 Office，放最前面；`opc-resave` 里 Word 的 `SaveAs2` 曾经挂过，放最后并套了 60 秒超时。

几件踩过的坑，都写进注释了，改脚本前先看一眼：

- 本机装了 WPS，`New-Object -ComObject Word.Application` 会被 WPS 抢走。`lib/office-com.ps1` 的做法是直接启动 `WINWORD.EXE` / `EXCEL.EXE` / `POWERPNT.EXE`，枚举窗口，再用 `AccessibleObjectFromWindow` 从文档子窗口取原生对象模型。PowerPoint 顶层窗口类是 `PPTFrameClass`，子窗口类是 `mdiClass`（Office 16 没有 `paneClassDC`）。
- 启动时必须带一个文档。空的 Excel 进程没有 `EXCEL7` 子窗口，`AccessibleObjectFromWindow` 会返回 `hr=0x80004005`。
- 读 ooxp 的输出不能用 `cmd /c`。ooxp 吐 UTF-8，cmd 按控制台代码页解，中文会坏：偶数长度的悄悄判错，奇数长度的直接把 JSON 撕坏。`Invoke-Ooxp` 显式按 UTF-8 读，并检查退出码。
- 判断「有没有弹修复提示」要读被测文档自己的标题栏（`$doc.ActiveWindow.Caption`、`$wb.Windows.Item(1).Caption`），不能读 `$app.ActiveWindow`，那可能是别的文档的窗口。
- apply 的目标文件若正被 Office 开着，ooxp 写不进去。xlsx 用 `Close-AllWorkbooks`，docx 用 `Close-WordDocByPath`。
- 手动把 Excel 窗口关掉并不等于进程退出。它常常留成一个两百多兆、没有窗口的进程，下一轮 `Get-Process` 找得到它，可它没有 `EXCEL7` 子窗口，绑定就白等 25 秒然后报 `hr=0x80004005`。`run.ps1` 开头会把这种进程报出来，`-CloseOffice` 负责收拾。
- 收尾的顺序不能颠倒：先 `[GC]::Collect()` 回收用例留下的工作簿等 COM 包装对象，再关文档，最后才 `Quit`。反过来做，Excel 会当作还有客户端连着，直接无视 `Quit`。
- 有的实例连正确顺序的 `Quit` 也不理。`Stop-LingeringOffice` 只对这一轮绑过的那个 PID 兜底：等 8 秒，确认窗口已经没了才结束进程。要是 Excel 里还开着 `out/` 之外的工作簿，整个关闭动作直接跳过，不碰你自己的文件。

拆分前的旧脚本（`_office-checklist.ps1` 等七个）已经删掉，需要时从 git 历史取：`git show be936f2:manual-check/office/requests/_office-checklist.ps1`。

## WPS 那一轮

还是老样子，没拆：`requests/_wps-*.ps1`，路径写死成 `C:/Users/GaoYi-AMD/Desktop/test/...`，换机器要先改。脚本吃的是 ooxp 的产物，顺序不能颠倒。以 07/08 这一组为例：先 `ooxp --input requests/07-create-media.json` 写出骨架文档，再跑 `_wps-style-media.ps1` 让 WPS 铺样式和插图，最后 `ooxp --input requests/08-apply-media.json` 打那 15 条 `setParaText`。

素材图 `ooxp-pic-wide.png` 与 `ooxp-pic-square.png` 不入库，`_wps-style-media.ps1` 会在缺图时自己画：一张 1200×300 的宽图、一张 600×600 的方图，各约 25KB，不必自备图片。

### 07 / 08 这一组：改字之后样式和插图还在不在

请并排看这两份：左边是 WPS 加过样式和插图的源文件，右边是 ooxp 一次改了 15 段文字之后。

| 文件 | 从哪来 |
| --- | --- |
| `07-wps-media.docx` | ooxp 写出 17 段骨架；WPS 加上居中底纹、引用段、项目符号、楷体蓝字下划线、删除线黄底、一张行内宽图、一张带红框的绕排方图 |
| `08-media-applied.docx` | 对上一份一次打了 15 条 `setParaText`（第 15、16 段没改） |

第一期领域 JSON **不能**插入图片，也不能改段落样式或图片绕排。create 的 `style` 只认 `Normal` / `Heading1`–`Heading3`；apply 只认 `setParaText`。这次是先用 WPS 把样式和图片铺进文件，再看改字之后这些东西还在不在。

应当看到：标题 1/2/3 还在；第 2 段仍居中且有浅底；第 4 段仍像引用；第 6、7 段仍是项目符号；第 9 段仍是蓝字下划线；第 10 段仍有删除线和黄底；宽图仍在第 12 段里；方图仍绕排在第 13 段旁；第 15 段「这一段不要改，用来对照。」不动。

COM 读完 17 段：样式名、对齐、缩进、斜体、下划线、删除线、颜色、底纹、列表类型全部相同；行内图 1 张、浮动图 1 张，改前改后都是这个数。明细在 `compare-07-vs-08.csv`。
