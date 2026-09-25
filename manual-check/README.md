# 人工验收

`docs/checklist-xlsx.md` 与 `docs/checklist-docx.md` 要求在本机用 Excel、Word、WPS 亲手打开文件。这个目录放的是那几轮验收用的请求 JSON 和驱动脚本。

入库的只有三类：本文件、`requests/` 下的请求 JSON、`*.ps1` 脚本。Office 产物、素材图、COM 日志和 CSV 对比结果都不入库，跑一遍就会重新生成。素材图 `ooxp-pic-wide.png` 与 `ooxp-pic-square.png` 原先是两张外部大图，现在 `_wps-style-media.ps1` 会在缺图时自己画：一张 1200×300 的宽图、一张 600×600 的方图，各约 25KB，不必自备图片。

怎么复现：先在仓库根目录 `cargo build`，再用 `pwsh` 跑脚本。`office/requests/_office-checklist.ps1` 是 Microsoft Office 那一轮，`requests/_wps-*.ps1` 是 WPS 那一轮。脚本走 COM，本机得装了对应软件。两家的 `Excel.Application` 和 `Word.Application` 会互相抢注册，脚本的做法是直接启动 `EXCEL.EXE` 或 `WINWORD.EXE` 再挂上去。

有两处换机器会踩：一是 `*.ps1` 与请求 JSON 里的路径都是写死的 `C:/Users/GaoYi-AMD/Desktop/test/...`，得先改成自己的仓库位置；二是脚本吃的是 ooxp 的产物，所以顺序不能颠倒。以 07/08 这一组为例：先 `ooxp --input requests/07-create-media.json` 写出 `07-ooxp-media.docx`，再跑 `_wps-style-media.ps1` 让 WPS 铺样式和插图，最后 `ooxp --input requests/08-apply-media.json` 打那 15 条 `setParaText`。

## 07 / 08 这一组：改字之后样式和插图还在不在

请并排看这两份：左边是 WPS 加过样式和插图的源文件，右边是 ooxp 一次改了 15 段文字之后。

| 文件 | 从哪来 |
| --- | --- |
| `07-wps-media.docx` | ooxp 写出 17 段骨架；WPS 加上居中底纹、引用段、项目符号、楷体蓝字下划线、删除线黄底、一张行内宽图、一张带红框的绕排方图 |
| `08-media-applied.docx` | 对上一份一次打了 15 条 `setParaText`（第 15、16 段没改） |

第一期领域 JSON **不能**插入图片，也不能改段落样式或图片绕排。create 的 `style` 只认 `Normal` / `Heading1`–`Heading3`；apply 只认 `setParaText`。这次是先用 WPS 把样式和图片铺进文件，再看改字之后这些东西还在不在。

应当看到：标题 1/2/3 还在；第 2 段仍居中且有浅底；第 4 段仍像引用；第 6、7 段仍是项目符号；第 9 段仍是蓝字下划线；第 10 段仍有删除线和黄底；宽图仍在第 12 段里；方图仍绕排在第 13 段旁；第 15 段「这一段不要改，用来对照。」不动。

COM 读完 17 段：样式名、对齐、缩进、斜体、下划线、删除线、颜色、底纹、列表类型全部相同；行内图 1 张、浮动图 1 张，改前改后都是这个数。明细在 `compare-07-vs-08.csv`。
