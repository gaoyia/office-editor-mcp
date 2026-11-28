# 江城黑体 400W（嵌入用字形子集）

`JiangChengHeiTi-400W-subset.ttf` 是江城黑体 Version 2.1 的直立 Regular 400W 字形子集，只用于在本库非 Windows 路径（WASM / macOS / Linux）里嵌入绘制单元格文字。不是单独重新发布的一款新字体，名称表沿用官方源文件，没有改族名、全名或 PostScript 名。

本文件按 **SIL Open Font License 1.1** 随库分发。完整条款见同目录 `SILOFL1.1.txt`（与用户提供的 2.1 包内同名文件逐字节相同，未改写版权行）。该许可证文本的版权行是 `Copyright (c) 2020, Celestial Phineas`，版权声明后没有 Reserved Font Name。源字体名称表 name ID 9 的作者是 `LiuPeng`。

## 来源

- 作者：LiuPeng（name ID 9）
- 用户提供的 2.1 包路径：`D:/BaiduNetdiskDownload/江城黑体/JiangChengHeiTi 400W.ttf`
- 使用的源文件：`JiangChengHeiTi 400W.ttf`（正文字重 Regular / 400，直立）。同包还有 100W–700W 以及对应的 Italic 字重，均未嵌入。
- 版本：源字体 name ID 5 为 `Version 2.1  2021/10/28`（head 表修改日期 2021-10-28）。
- 源文件大小：8,050,972 字节
- 源文件 SHA-256：`c96b71d3931925e6bd3dd4f4f08adba5958a1d138638ea43e0e1fec95d2f29fd`
- 源文件字形数：30,394

## 名称表（与官方源文件记录一致，含原始字节）

- name ID 0：`© 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.` 子集未改动这条记录。
- name ID 1 / 4（英文）：JiangChengHeiTi 400W
- name ID 1 / 4（简体中文）：江城黑体 400W
- name ID 2：Regular
- name ID 3：JiangChengHeiTi-400W;Version 2.1
- name ID 5：Version 2.1  2021/10/28
- name ID 6：JiangChengHeiTi-400W
- name ID 7：Source is a trademark of Adobe in the United States and/or other countries.
- name ID 9：LiuPeng
- name ID 10：源文件保留的 Source 制作人员记录；子集未改动。
- name ID 13 / 14：SIL Open Font License 1.1 文本与 `http://scripts.sil.org/OFL`
- 此 2.1 源文件没有 typographic family（ID 16）记录。

主族名仍是江城黑体 / JiangChengHeiTi。只做字形子集、不重新发布新字体，名称表保持官方原文。

## 覆盖范围

- 基本拉丁、Latin-1、Latin Extended-A
- 通用标点、货币、部分箭头
- CJK 标点、假名、半角/全角
- GB2312 汉字

缺字画 `.notdef`，不因此让整表失败。打开单元格字时先按字体名找本机字体，找不到再用这份子集。不与 Windows GDI / 微软雅黑 / 等线逐像素一致。

本子集通过 `include_bytes!` 编进 `crates/ooxp/src/xlsx/face_soft.rs`；替换后需重新编译。系统已安装对应字体时，非 WASM 构建会优先走系统字体查找。

再加工可用的附加文件：同目录 `subset.py`。

## 重新生成

需要源文件 `JiangChengHeiTi 400W.ttf` 和 `fonttools`：

```
python crates/ooxp/fonts/subset.py --src "path/to/JiangChengHeiTi 400W.ttf"
```

未指定 `--src` 时，脚本默认尝试用户本机已提供的 2.1 包路径。脚本只做字形子集，不改名称表。
