import init, { Session } from "./pkg/ooxp_wasm.js";

function slidePictures(bg) {
  return [
    { file: bg, x: 0, y: 0, cx: 12192000, cy: 6858000 },
    { file: "deco-bar.png", x: 0, y: 0, cx: 120000, cy: 6858000 },
    { file: "deco-ring.png", x: 10650000, y: 0, cx: 2400000, cy: 2400000 },
    { file: "deco-dot.png", x: 11520000, y: 6320000, cx: 220000, cy: 220000 },
  ];
}

function coverCard(x, title, body, head) {
  const edge = { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } };
  return {
    rows: 2,
    cols: 1,
    x,
    y: 2720000,
    cx: 3358533,
    cy: 2360000,
    rowHeights: [760000, 1600000],
    tableStyleId: null,
    tblLook: null,
    cells: [
      [{ text: title, fill: head, borders: edge, cellMar: { top: 180000, left: 160000, right: 120000, bottom: 40000 } }],
      [{ text: body, fill: "F6F5F1", borders: edge, cellMar: { top: 100000, left: 160000, right: 140000, bottom: 80000 } }],
    ],
  };
}

function decoAnims(rest) {
  return [
    { type: "fade", shape: 1, trigger: "with", dur: 480 },
    { type: "fade", picture: 2, trigger: "with", delay: 40, dur: 560 },
    { type: "zoom", picture: 3, trigger: "with", delay: 120, dur: 680 },
    { type: "flyIn", picture: 4, trigger: "with", delay: 140, dur: 640 },
    ...rest,
    { type: "growShrink", picture: 4, trigger: "after", dur: 700, repeat: 2 },
  ];
}

const FORMATS = [
  {
    id: "xlsx",
    title: "工作簿",
    color: "var(--xlsx)",
    items: [
      {
        name: "业绩看板",
        blurb: "深色标题、数据条、色阶、图标集、迷你图、组合图、环形图和循环图示，另有一整页图表。写入「ooxp-example-业绩.xlsx」。",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "ooxp-example-业绩.xlsx",
          format: "xlsx",
          workbook: {
            theme: {
              name: "业绩",
              colors: { accent1: "0F6E6E", accent2: "1D4E89", accent3: "C2410C", accent4: "7C3AED" },
              fonts: { majorLatin: "Calibri", majorEa: "微软雅黑" },
            },
            sheets: [
              {
                name: "业绩",
                cells: [
                  {
                    ref: "A1",
                    value: "本季业绩",
                    font: { bold: true, sz: 18, color: { rgb: "F7F4EE" } },
                    fill: { pattern: "solid", fg: { rgb: "0F6E6E" } },
                    align: { horizontal: "center", vertical: "center" },
                  },
                  { ref: "A3", value: "产品", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "B3", value: "一月", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "C3", value: "二月", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "D3", value: "三月", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "E3", value: "合计", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "F3", value: "趋势", font: { bold: true, color: { rgb: "FFFFFF" } }, fill: { pattern: "solid", fg: { rgb: "1C1917" } } },
                  { ref: "A4", value: "甲" },
                  { ref: "B4", value: 42, numFmt: "#,##0" },
                  { ref: "C4", value: 38, numFmt: "#,##0" },
                  { ref: "D4", value: 55, numFmt: "#,##0" },
                  { ref: "E4", formula: "=SUM(B4:D4)", numFmt: "#,##0", font: { bold: true } },
                  { ref: "A5", value: "乙" },
                  { ref: "B5", value: 28, numFmt: "#,##0" },
                  { ref: "C5", value: 61, numFmt: "#,##0" },
                  { ref: "D5", value: 33, numFmt: "#,##0" },
                  { ref: "E5", formula: "=SUM(B5:D5)", numFmt: "#,##0", font: { bold: true } },
                  { ref: "A6", value: "丙" },
                  { ref: "B6", value: 51, numFmt: "#,##0" },
                  { ref: "C6", value: 22, numFmt: "#,##0" },
                  { ref: "D6", value: 47, numFmt: "#,##0" },
                  { ref: "E6", formula: "=SUM(B6:D6)", numFmt: "#,##0", font: { bold: true } },
                  { ref: "A7", value: "丁" },
                  { ref: "B7", value: 19, numFmt: "#,##0" },
                  { ref: "C7", value: 44, numFmt: "#,##0" },
                  { ref: "D7", value: 36, numFmt: "#,##0" },
                  { ref: "E7", formula: "=SUM(B7:D7)", numFmt: "#,##0", font: { bold: true } },
                ],
                cols: [
                  { col: "A", width: 14 },
                  { col: "B", width: 12 },
                  { col: "C", width: 12 },
                  { col: "D", width: 12 },
                  { col: "E", width: 12 },
                  { col: "F", width: 16 },
                  { col: "G", width: 3 },
                ],
                rows: [
                  { row: 1, height: 36 },
                  { row: 3, toRow: 7, height: 22 },
                ],
                merges: ["A1:F1"],
                tables: [{ ref: "A3:E7", name: "Quarter", columns: ["产品", "一月", "二月", "三月", "合计"] }],
                charts: [
                  {
                    type: "combo",
                    title: "三个月",
                    from: "H3",
                    dataFrom: "AA3",
                    cx: 5029200,
                    cy: 2286000,
                    series: [
                      { name: "一月", type: "col", categories: ["甲", "乙", "丙", "丁"], values: [42, 28, 51, 19] },
                      { name: "二月", type: "col", categories: ["甲", "乙", "丙", "丁"], values: [38, 61, 22, 44] },
                      { name: "三月", type: "line", categories: ["甲", "乙", "丙", "丁"], values: [55, 33, 47, 36] },
                    ],
                  },
                  {
                    type: "doughnut",
                    title: "合计占比",
                    from: "H17",
                    dataFrom: "AA12",
                    cx: 5029200,
                    cy: 2286000,
                    series: [{ name: "合计", categories: ["甲", "乙", "丙", "丁"], values: [135, 122, 120, 99] }],
                  },
                ],
                conditionalFormats: [
                  { sqref: "B4:B7", type: "dataBar", dataBar: { color: { rgb: "0F6E6E" } } },
                  {
                    sqref: "C4:C7",
                    type: "colorScale",
                    colorScale: { colors: [{ rgb: "F8696B" }, { rgb: "FFEB84" }, { rgb: "63BE7B" }] },
                  },
                  { sqref: "D4:D7", type: "iconSet" },
                ],
                sparklines: [
                  { ref: "F4", formula: "B4:D4", type: "column", high: true, low: true },
                  { ref: "F5", formula: "B5:D5", type: "column", high: true, low: true },
                  { ref: "F6", formula: "B6:D6", type: "column", high: true, low: true },
                  { ref: "F7", formula: "B7:D7", type: "column", high: true, low: true },
                ],
                smartArts: [
                  {
                    layout: "cycle",
                    from: "A10",
                    cx: 5029200,
                    cy: 2286000,
                    points: [{ text: "创建" }, { text: "图表" }, { text: "条件格式" }, { text: "改格" }],
                  },
                ],
                comments: [{ cell: "A1", text: "旁边有组合图和环形图，下一张表是整页图。", author: "ooxp" }],
                headerFooters: [
                  { kind: "odd", side: "header", text: "&C本季业绩" },
                  { kind: "odd", side: "footer", text: "&R第 &P 页" },
                ],
                view: { frozenRows: 3, zoom: 110 },
              },
              {
                name: "总览",
                kind: "chartsheet",
                type: "col",
                title: "季度对比",
                dataSheet: "业绩",
                dataFrom: "AA21",
                series: [
                  { name: "一月", categories: ["甲", "乙", "丙", "丁"], values: [42, 28, 51, 19] },
                  { name: "二月", categories: ["甲", "乙", "丙", "丁"], values: [38, 61, 22, 44] },
                  { name: "三月", categories: ["甲", "乙", "丙", "丁"], values: [55, 33, 47, 36] },
                ],
              },
            ],
          },
        },
      },
      {
        name: "改本季数字",
        blurb: "把标题和「甲」的二月改掉，仍写回「ooxp-example-业绩.xlsx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-业绩.xlsx",
          ops: [
            { op: "setCell", id: "cell:业绩!A1", value: "本季业绩 · 已改" },
            { op: "setCell", id: "cell:业绩!C4", value: 88 },
          ],
        },
      },
    ],
  },
  {
    id: "docx",
    title: "文档",
    color: "var(--docx)",
    items: [
      {
        name: "一页说明",
        blurb: "水印、页边、分色导语、字体宽窄、公式、图片、拼音、控件、行内图示和环形图，写入「ooxp-example-发布说明.docx」。",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "ooxp-example-发布说明.docx",
          format: "docx",
          document: {
            theme: {
              name: "说明",
              colors: { accent1: "0F6E6E", accent2: "C2410C", accent3: "1D4E89" },
              fonts: { majorLatin: "Calibri", minorLatin: "Cambria", majorEa: "微软雅黑", minorEa: "宋体" },
            },
            pageBackground: { color: "FBF7F1" },
            watermarks: [{ text: "OOXP" }],
            pageBorders: {
              offsetFrom: "page",
              top: { val: "single", sz: 18, space: 14, color: "0F6E6E" },
              left: { val: "single", sz: 18, space: 14, color: "0F6E6E" },
              bottom: { val: "single", sz: 18, space: 14, color: "0F6E6E" },
              right: { val: "single", sz: 18, space: 14, color: "0F6E6E" },
            },
            numberings: [{ kind: "bullet" }],
            headers: [{ type: "default", paragraphs: [{ text: "ooxp  ·  发布说明" }] }],
            footers: [{ type: "default", paragraphs: [{ text: "第 ", fields: [{ instr: " PAGE " }] }] }],
            paragraphs: [
              {
                style: "Heading1",
                text: "今晚发布",
                footnote: "脚注、页眉和页码写在各自分部里。",
                endnote: "尾注写在文末。",
                bookmarks: [{ name: "top" }],
                comments: [{ text: "批注", author: "ooxp", replies: [{ text: "回复" }] }],
              },
              {
                runs: [
                  { text: "三种格式", sz: 28, bold: true, color: "0F6E6E" },
                  { text: "，一次写进", sz: 28 },
                  { text: "同一个包", sz: 28, bold: true, color: "1D4E89" },
                  { text: "。", sz: 28 },
                ],
                shd: { fill: "E7F3F1" },
                hyperlinks: [{ text: "仓库", url: "https://github.com/gaoyia/ooxp" }],
              },
              { text: "工作簿写表和图", num: 1 },
              { text: "文档写版式和图示", num: 1 },
              { text: "演示文稿写页和切换", num: 1 },
              {
                runs: [
                  { text: "黑体", font: "SimHei", eastAsiaFont: "黑体", sz: 32, bold: true },
                  { text: "  ", sz: 32 },
                  { text: "楷体", font: "KaiTi", eastAsiaFont: "楷体", sz: 32 },
                  { text: "  ", sz: 32 },
                  { text: "Cambria", font: "Cambria", sz: 32, italic: true, color: "1D4E89" },
                  { text: "  ", sz: 32 },
                  { text: "等宽", font: "Consolas", eastAsiaFont: "Consolas", sz: 28, highlight: "yellow" },
                ],
              },
              {
                runs: [
                  { text: "窄", scale: 50, sz: 44, eastAsiaFont: "微软雅黑" },
                  { text: "正", scale: 100, sz: 44, eastAsiaFont: "微软雅黑" },
                  { text: "宽", scale: 200, sz: 44, color: "C2410C", eastAsiaFont: "微软雅黑" },
                  { text: " 疏", spacing: 80, sz: 32, underline: "wave", color: "0F6E6E" },
                  { text: "密", spacing: -12, sz: 32 },
                  { text: "字宽", eastAsianLayout: { combine: true, combineBrackets: "square" }, sz: 32 },
                ],
                borders: { bottom: { val: "single", sz: 12, color: "0F6E6E" } },
              },
              { text: "", maths: [{ text: "a^2+b^2=c^2", kind: "display" }] },
              { pictures: [{ file: "mark.png", cx: 2743200, cy: 1828800 }] },
              {
                text: "汉字",
                rubies: [{ base: "汉字", rt: "hànzì" }],
                crossRefs: [{ bookmark: "top" }],
              },
              {
                contentControls: [
                  { kind: "checkbox", checked: true },
                  { kind: "dropDown", text: "甲", items: ["甲", "乙", "丙"] },
                ],
                fields: [{ instr: " DATE ", result: "2026-11-08" }],
              },
              {
                text: "循环",
                smartArts: [
                  {
                    layout: "cycle",
                    kind: "inline",
                    cx: 5029200,
                    cy: 2286000,
                    points: [{ text: "创建" }, { text: "打开" }, { text: "修改" }, { text: "保存" }],
                  },
                ],
              },
              {
                text: "份额",
                charts: [
                  {
                    type: "doughnut",
                    kind: "inline",
                    cx: 4572000,
                    cy: 2743200,
                    title: "份额",
                    series: [{ name: "格式", categories: ["工作簿", "文档", "演示文稿"], values: [5, 3, 4] }],
                  },
                ],
              },
            ],
            tables: [
              {
                after: 13,
                align: "center",
                look: { firstRow: true, bandedRows: false, firstColumn: false },
                width: { type: "dxa", w: 9000 },
                cols: [3000, 3000, 3000],
                rowHeights: [
                  { h: 460, rule: "atLeast" },
                  { h: 720, rule: "atLeast" },
                ],
                cellMar: { top: 80, left: 120, bottom: 80, right: 120 },
                borders: {
                  top: { val: "single", sz: 12, color: "0F6E6E" },
                  left: { val: "single", sz: 12, color: "0F6E6E" },
                  bottom: { val: "single", sz: 12, color: "0F6E6E" },
                  right: { val: "single", sz: 12, color: "0F6E6E" },
                  insideH: { val: "single", sz: 4, color: "D6D3D1" },
                  insideV: { val: "single", sz: 8, color: "FFFFFF" },
                },
                cellFormats: [
                  { r: 1, c: 1, fill: "D8EFEA", vAlign: "center" },
                  { r: 1, c: 2, fill: "F4E4D7", vAlign: "center" },
                  { r: 1, c: 3, fill: "D6E4F5", vAlign: "center" },
                  { r: 2, c: 1, fill: "FBF7F1", vAlign: "center" },
                  { r: 2, c: 2, fill: "FBF7F1", vAlign: "center" },
                  { r: 2, c: 3, fill: "FBF7F1", vAlign: "center" },
                ],
                cells: [
                  ["工作簿", "文档", "演示文稿"],
                  ["表和图", "图示和公式", "形状和切换"],
                ],
              },
            ],
          },
        },
      },
      {
        name: "改标题",
        blurb: "把第一段换成新标题，仍写回「ooxp-example-发布说明.docx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-发布说明.docx",
          ops: [{ op: "setParaText", id: "para:00000001", text: "标题已经改过" }],
        },
      },
    ],
  },
  {
    id: "pptx",
    title: "演示文稿",
    color: "var(--pptx)",
    items: [
      {
        name: "六页发布会",
        blurb: "每页一张背景，角落三处装饰在进页时自动出现。内容是表格、图示和图表。写入「ooxp-example-发布会.pptx」。",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "ooxp-example-发布会.pptx",
          format: "pptx",
          presentation: {
            sections: [
              { name: "开场", start: 1 },
              { name: "图", start: 3 },
              { name: "数字", start: 5 },
            ],
            handout: { header: "ooxp", footer: "发布会" },
            slides: [
              {
                shapes: [
                  { text: "今晚发布", sz: 5400, bold: true, color: "0F6E6E", align: "ctr", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" },
                  { text: "六页排开", sz: 1800, color: "6F655C", align: "ctr", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" },
                ],
                pictures: slidePictures("bg-two.png"),
                tables: [
                  coverCard(838200, "工作簿", "表和图", "E7F2F0"),
                  coverCard(4416733, "文档", "图示和公式", "F3EEE6"),
                  coverCard(7995266, "演示文稿", "形状和切换", "E7EDF5"),
                ],
                comments: [{ text: "封面批注", author: "ooxp", x: 420000, y: 6000000 }],
                transition: { type: "fade", dur: 800 },
                anims: decoAnims([
                  { type: "flyIn", shape: 2, trigger: "with", delay: 60, dur: 480 },
                  { type: "fade", table: 1, trigger: "with", delay: 40, dur: 460 },
                  { type: "fade", table: 2, trigger: "with", delay: 70, dur: 460 },
                  { type: "fade", table: 3, trigger: "with", delay: 70, dur: 460 },
                ]),
                notes: "封面。第一个文本框是 slide:1/shape:2。",
              },
              {
                shapes: [{ text: "三件事", sz: 4000, bold: true, color: "1C1917", align: "l", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" }],
                pictures: slidePictures("bg-one.png"),
                tables: [
                  {
                    rows: 3,
                    cols: 2,
                    x: 838200,
                    y: 1900000,
                    cx: 9800000,
                    cy: 3600000,
                    colWidths: [1500000, 8300000],
                    rowHeights: [1200000, 1200000, 1200000],
                    tableStyleId: null,
                    tblLook: null,
                    cells: [
                      [
                        { text: "01", fill: "E7F2F0", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                        { text: "表、图、条件格式", fill: "F6F5F1", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                      ],
                      [
                        { text: "02", fill: "F3EEE6", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                        { text: "版式、图示、公式", fill: "F6F5F1", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                      ],
                      [
                        { text: "03", fill: "E7EDF5", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                        { text: "页、形状、切换", fill: "F6F5F1", borders: { left: { color: "FFFFFF", width: 6350 }, right: { color: "FFFFFF", width: 6350 }, top: { color: "FFFFFF", width: 6350 }, bottom: { color: "FFFFFF", width: 6350 } } },
                      ],
                    ],
                  },
                ],
                transition: { type: "push", dir: "l", dur: 700 },
                anims: decoAnims([{ type: "fade", table: 1, trigger: "with", delay: 80, dur: 500 }]),
                notes: "三行目录。",
              },
              {
                shapes: [{ text: "版式", sz: 4000, bold: true, color: "1C1917", align: "l", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" }],
                pictures: slidePictures("bg-one.png"),
                smartArts: [
                  {
                    layout: "list",
                    x: 2200000,
                    y: 1900000,
                    cx: 7800000,
                    cy: 4000000,
                    points: [{ text: "页边" }, { text: "页眉" }, { text: "公式" }, { text: "水印" }],
                  },
                ],
                transition: { type: "wipe", dir: "r", dur: 700 },
                anims: decoAnims([{ type: "fade", smartArt: 1, trigger: "with", delay: 100, dur: 600 }]),
                notes: "左图右清单。",
              },
              {
                shapes: [{ text: "四步", sz: 4000, bold: true, color: "1C1917", align: "l", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" }],
                pictures: slidePictures("bg-one.png"),
                smartArts: [
                  {
                    layout: "cycle",
                    x: 2600000,
                    y: 1750000,
                    cx: 7000000,
                    cy: 4400000,
                    points: [{ text: "创建" }, { text: "打开" }, { text: "修改" }, { text: "保存" }],
                  },
                ],
                transition: { type: "wheel", spokes: 4, dur: 700 },
                anims: decoAnims([{ type: "zoom", smartArt: 1, trigger: "with", delay: 80, dur: 640 }]),
                notes: "循环四步。",
              },
              {
                shapes: [{ text: "三个月", sz: 4000, bold: true, color: "1C1917", align: "l", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" }],
                pictures: slidePictures("bg-one.png"),
                charts: [
                  {
                    type: "col",
                    title: "销量",
                    x: 838200,
                    y: 1750000,
                    cx: 6000000,
                    cy: 4300000,
                    series: [
                      { name: "一月", categories: ["甲", "乙", "丙", "丁"], values: [42, 28, 51, 19] },
                      { name: "二月", categories: ["甲", "乙", "丙", "丁"], values: [38, 61, 22, 44] },
                      { name: "三月", categories: ["甲", "乙", "丙", "丁"], values: [55, 33, 47, 36] },
                    ],
                  },
                ],
                tables: [
                  {
                    rows: 5,
                    cols: 2,
                    x: 7400000,
                    y: 2100000,
                    cx: 3700000,
                    cy: 3200000,
                    tableStyleId: null,
                    tblLook: null,
                    cells: [
                      [
                        { text: "产品", fill: "D8EFEA" },
                        { text: "二月", fill: "D8EFEA" },
                      ],
                      [
                        { text: "甲", fill: "FBF7F1" },
                        { text: "38", fill: "FBF7F1" },
                      ],
                      [
                        { text: "乙", fill: "FBF7F1" },
                        { text: "61", fill: "FBF7F1" },
                      ],
                      [
                        { text: "丙", fill: "FBF7F1" },
                        { text: "22", fill: "FBF7F1" },
                      ],
                      [
                        { text: "丁", fill: "FBF7F1" },
                        { text: "44", fill: "FBF7F1" },
                      ],
                    ],
                  },
                ],
                transition: { type: "split", orient: "horz", dur: 700 },
                anims: decoAnims([
                  { type: "wipe", chart: 1, trigger: "with", delay: 80, dur: 700 },
                  { type: "fade", table: 1, trigger: "with", delay: 80, dur: 500 },
                ]),
                notes: "左图右表。",
              },
              {
                shapes: [
                  { text: "回到封面", sz: 4000, bold: true, color: "0F6E6E", align: "ctr", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑", slide: 1 },
                  { text: "写完了", sz: 1800, color: "6F655C", align: "ctr", textAnchor: "ctr", font: "Calibri", eastAsiaFont: "微软雅黑" },
                ],
                pictures: slidePictures("bg-two.png"),
                transition: { type: "doors", dur: 700 },
                anims: decoAnims([{ type: "flyIn", shape: 2, trigger: "with", delay: 100, dur: 520 }]),
                notes: "标题链回第一页。",
              },
            ],
          },
        },
      },
      {
        name: "改封面",
        blurb: "改第一页第一个文本框，仍写回「ooxp-example-发布会.pptx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-发布会.pptx",
          ops: [{ op: "setShapeText", id: "slide:1/shape:2", text: "ooxp · 已改" }],
        },
      },
    ],
  },
];

const UI = {
  zh: {
    lead: "ooxp 的 WebAssembly 示例",
    slogan: "全球网页 Office 编辑最好的 MCP 之一",
    aside: "为了不违反广告法加了之一",
    bannerStrong: "请更新浏览器。",
    bannerSpan: "这个浏览器不能改本地文件。请换成新版 Chrome 或 Edge，再用 https 或 localhost 打开。",
    heroTitle: "按稳定 id 改包里的 XML",
    heroBody: "ooxp 读写新版 Office 文件。已有文件只改目标节点，其余部件按原字节写回。只有新建才整包生成。这一页的示例写进你授权的目录。",
    tabXlsx: "工作簿",
    cardXlsx: "表格、图表、条件格式和迷你图。.xlsm 可以查看，不执行宏。",
    gotoXlsx: "看工作簿示例",
    tabDocx: "文档",
    cardDocx: "段落、表格、图片、公式、水印和批注。",
    gotoDocx: "看文档示例",
    tabPptx: "演示文稿",
    cardPptx: "幻灯片、形状、图示、图表和切换，支持写入动画。",
    gotoPptx: "看演示文稿示例",
    formatsTitle: "支持的格式",
    formatsBody: "只读写 .docx、.xlsx、.pptx。老的二进制 .doc、.xls、.ppt 不在范围内。库不排版、不算公式、也不放映。",
    landTitle: "文件怎么落地",
    landBody: "示例不从浏览器另存。先在右侧选择一个目录，再点「写入目录」，文件直接出现在那里。",
    mcpBody: "把命令行接到编辑器。command 也可以写成这个程序的完整路径。",
    downloads: "下载",
    latest: "最新版本",
    finding: "正在查找最新版本",
    missing: "这一版没有这个文件",
    unavailable: "打开发布页",
    dirTitle: "目录",
    pick: "选择目录",
    forget: "取消授权",
    restore: "恢复授权",
    home: "介绍",
    request: "请求",
    write: "写入目录",
    writing: "正在写入…",
    noBytes: "完成，没有文件可写。",
    badJson: "请求不是 JSON",
    needDir: "请先选择目录",
    libReady: "库已就绪。",
    libMissing: "库没有加载：",
    libFail: "库没有就绪",
    permFail: "目录权限没有授下来。",
    forgot: "已取消这个目录的授权。",
    pathRel: "路径必须相对授权目录",
    pathDots: "路径不能包含 . 或 ..",
    noName: "缺少文件名",
    missingFile: "目录里没有「{path}」。请先运行同一格式的创建。",
    busyFile: "「{path}」正被其他程序占用。请先关闭打开它的 Word、Excel 或 PowerPoint，再点写入。",
    fail: "执行失败",
    thisTime: "这次：",
    new: "新增",
    chg: "已改",
    written: "已写入",
    folder: "文件夹",
    emptyFiles: "选择目录后，这里列出其中的文件。",
    noFiles: "这个目录里还没有文件。",
    dirNone: "还没有授权目录。选择之后才能写入。",
    dirReady: "已授权「{name}」。示例会直接改这里的文件。",
    unsupported: "当前浏览器没有目录选择接口。请更新到新版 Chrome 或 Edge。",
    unsupportedEmpty: "更新浏览器之后，才能把示例写进本地目录。",
    licenseTitle: "许可证",
    license: "本软件以 Business Source License 1.0 发布。",
    donate: "捐赠",
    win: "Windows · 64 位",
    linux: "Linux · x64",
    linuxArm: "Linux · arm64",
    mac: "macOS · Apple 芯片",
  },
  en: {
    lead: "WebAssembly examples for ooxp",
    slogan: "One of the best MCPs for editing Office files on the web.",
    aside: "Added “one of” so this does not violate advertising law.",
    bannerStrong: "Update your browser.",
    bannerSpan: "This browser cannot edit local files. Use a current Chrome or Edge, and open the page over https or localhost.",
    heroTitle: "Patch the XML in the package by stable id",
    heroBody: "ooxp reads and writes the current Office formats. An existing file is patched at the target nodes, and every other part is written back as the original bytes. A whole package is generated only when creating a file. Examples on this page are written into the directory you authorize.",
    tabXlsx: "Workbook",
    cardXlsx: "Tables, charts, conditional formatting, and sparklines. .xlsm can be inspected. Macros are not run.",
    gotoXlsx: "Open the workbook examples",
    tabDocx: "Document",
    cardDocx: "Paragraphs, tables, pictures, formulas, a watermark, and comments.",
    gotoDocx: "Open the document examples",
    tabPptx: "Presentation",
    cardPptx: "Slides, shapes, diagrams, charts, and transitions. Animation can be written into the file.",
    gotoPptx: "Open the presentation examples",
    formatsTitle: "Formats",
    formatsBody: "The library reads and writes .docx, .xlsx, and .pptx. The older binary formats .doc, .xls, and .ppt are outside its scope. It does not paginate, calculate formulas, or run a slide show.",
    landTitle: "Where files go",
    landBody: "Examples are not saved through the browser download. Choose a directory on the right, then click Write. The file appears there.",
    mcpBody: "Connect the command-line program to an editor. command may also be the full path of that program.",
    downloads: "Download",
    latest: "Latest",
    finding: "Looking up the latest release",
    missing: "Not in this release",
    unavailable: "Open the releases page",
    dirTitle: "Directory",
    pick: "Choose a directory",
    forget: "Remove access",
    restore: "Restore access",
    home: "Intro",
    request: "Request",
    write: "Write",
    writing: "Writing…",
    noBytes: "Done. There is no file to write.",
    badJson: "The request is not JSON",
    needDir: "Choose a directory first",
    libReady: "The library is ready.",
    libMissing: "The library did not load: ",
    libFail: "The library is not ready",
    permFail: "Directory permission was not granted.",
    forgot: "Access to this directory has been removed.",
    pathRel: "The path must stay inside the authorized directory",
    pathDots: "The path cannot contain . or ..",
    noName: "The file name is missing",
    missingFile: "The directory does not contain “{path}”. Run the create example for this format first.",
    busyFile: "“{path}” is open in another program. Close Word, Excel, or PowerPoint, then click Write again.",
    fail: "The request failed",
    thisTime: "This time: ",
    new: "New",
    chg: "Edited",
    written: "Written",
    folder: "Folder",
    emptyFiles: "After you choose a directory, its files are listed here.",
    noFiles: "This directory has no files yet.",
    dirNone: "No directory is authorized yet. Choose one before writing.",
    dirReady: "“{name}” is authorized. Examples write into this directory.",
    unsupported: "This browser has no directory picker. Update to a current Chrome or Edge.",
    unsupportedEmpty: "After the browser is updated, examples can be written into a local directory.",
    licenseTitle: "License",
    license: "This software is released under the Business Source License 1.0.",
    donate: "Donate",
    win: "Windows · 64-bit",
    linux: "Linux · x64",
    linuxArm: "Linux · arm64",
    mac: "macOS · Apple silicon",
  },
};

const EN_TEXT = {
  "工作簿": "Workbook",
  "业绩看板": "Results board",
  "深色标题、数据条、色阶、图标集、迷你图、组合图、环形图和循环图示，另有一整页图表。写入「ooxp-example-业绩.xlsx」。": "A dark title, data bars, a color scale, an icon set, sparklines, a combo chart, a doughnut, and a cycle diagram, plus a full-page chart. Writes ooxp-example-results.xlsx.",
  "ooxp-example-业绩.xlsx": "ooxp-example-results.xlsx",
  "业绩": "Results",
  "本季业绩": "This quarter",
  "产品": "Product",
  "一月": "January",
  "二月": "February",
  "三月": "March",
  "合计": "Total",
  "趋势": "Trend",
  "甲": "A",
  "乙": "B",
  "丙": "C",
  "丁": "D",
  "三个月": "Three months",
  "合计占比": "Share",
  "创建": "Create",
  "图表": "a chart",
  "条件格式": "Formats",
  "改格": "Edit",
  "旁边有组合图和环形图，下一张表是整页图。": "The combo chart and the doughnut sit beside this table. The next sheet is a full-page chart.",
  "&C本季业绩": "&CThis quarter",
  "&R第 &P 页": "&RPage &P",
  "总览": "Overview",
  "季度对比": "Quarter",
  "改本季数字": "Edit this quarter",
  "把标题和「甲」的二月改掉，仍写回「ooxp-example-业绩.xlsx」。请先写入上面那份。": "Change the title and A's February, then write back to ooxp-example-results.xlsx. Create the file above first.",
  "cell:业绩!A1": "cell:Results!A1",
  "cell:业绩!C4": "cell:Results!C4",
  "本季业绩 · 已改": "This quarter · edited",
  "文档": "Document",
  "一页说明": "Release notes",
  "水印、页边、分色导语、字体宽窄、公式、图片、拼音、控件、行内图示和环形图，写入「ooxp-example-发布说明.docx」。": "A watermark, page border, colored lead, font width, a formula, a picture, ruby text, content controls, an inline diagram, and a doughnut. Writes ooxp-example-notes.docx.",
  "三种格式": "Three formats",
  "，一次写进": ", into ",
  "同一个包": "one package",
  "工作簿写表和图": "Workbook: tables and charts",
  "文档写版式和图示": "Document: layout and a diagram",
  "演示文稿写页和切换": "Slides: pages and transitions",
  "表和图": "Tables and charts",
  "图示和公式": "A diagram and formulas",
  "形状和切换": "Shapes and transitions",
  "ooxp-example-发布说明.docx": "ooxp-example-notes.docx",
  "说明": "Notes",
  "ooxp  ·  发布说明": "ooxp  ·  Notes",
  "第 ": "Page ",
  "今晚发布": "Tonight",
  "脚注、页眉和页码写在各自分部里。": "The footnote, header, and page number each live in their own part.",
  "尾注写在文末。": "The endnote sits at the end.",
  "批注": "Comment",
  "回复": "Reply",
  "这一页同时有": "This page has ",
  "分色正文": "colored text",
  "、": ", ",
  "图示": "a diagram",
  "和": " and ",
  "。": ".",
  "仓库": "Repository",
  "工作簿：数据条、色阶、图标集": "Workbook: data bars, color scale, icon set",
  "文档：页边、图示、环形图": "Document: page border, diagram, doughnut",
  "演示文稿：四页切换": "Slides: four transitions",
  "黑体": "Hei",
  "楷体": "Kai",
  "等宽": "Mono",
  "窄": "Narrow",
  "正": "Regular",
  "宽": "Wide",
  " 疏": " Loose",
  "密": "Tight",
  "编号": "Item",
  "左\t中": "Left\tCenter",
  "循环": "Cycle",
  "打开": "Open",
  "修改": "Change",
  "保存": "Save",
  "份额": "Share",
  "格式": "Format",
  "看到什么": "What you see",
  "写在哪": "Where",
  "数据条和色阶": "Data bars and color scale",
  "业绩表": "Results sheet",
  "整页柱状图": "Full-page columns",
  "四页切换": "Four transitions",
  "发布会": "Briefing",
  "中": "Medium",
  "压进窄列": "Packed",
  "中等宽度": "Medium width",
  "这一列最宽，用来对照列宽": "This column is the widest",
  "改标题": "Edit the title",
  "把第一段换成新标题，仍写回「ooxp-example-发布说明.docx」。请先写入上面那份。": "Change the first paragraph and write back to ooxp-example-notes.docx. Create the file above first.",
  "标题已经改过": "Title updated",
  "演示文稿": "Slides",
  "六页发布会": "Six-slide briefing",
  "每页一张背景，角落三处装饰在进页时自动出现。内容是表格、图示和图表。写入「ooxp-example-发布会.pptx」。": "Each slide has a background. Three corner ornaments appear as the slide opens. The content is a table, a diagram, or a chart. Writes ooxp-example-briefing.pptx.",
  "页边": "Page edge",
  "页眉": "Header",
  "公式": "Formula",
  "水印": "Watermark",
  "左图右表。": "Chart on the left, table on the right.",
  "版式": "Layout",
  "写完了": "Done",
  "六页排开": "Six slides",
  "三件事": "Three things",
  "表、图、条件格式": "Tables, charts, formats",
  "版式、图示、公式": "Layout, diagrams, formulas",
  "页、形状、切换": "Pages, shapes, transitions",
  "一边图一边清单": "Picture and a list",
  "封面": "Cover",
  "色卡": "Cards",
  "插图": "Picture",
  "四步": "Four steps",
  "回到封面": "Back to the cover",
  "点这一行回到第一页": "This line returns to the first slide",
  "封面批注": "Cover comment",
  "三张色卡。": "Three color cards.",
  "三行目录。": "Three rows.",
  "左图右清单。": "Picture on the left, list on the right.",
  "循环四步。": "A four-step cycle.",
  "标题链回第一页。": "The title links back to the first slide.",
  "ooxp-example-发布会.pptx": "ooxp-example-briefing.pptx",
  "开场": "Open",
  "路线": "Route",
  "数字": "Numbers",
  "四页，四种切换": "Four slides, four transitions",
  "封面。第一个文本框是 slide:1/shape:2。": "Cover. The first text box is slide:1/shape:2.",
  "组织图示。": "Organization diagram.",
  "结构": "Structure",
  "环形图铺在标题下面。": "The doughnut sits under the title.",
  "销量": "Sales",
  "上面三个合计，下面是柱状图。": "Three totals above, a column chart below.",
  "改封面": "Edit the cover",
  "改第一页第一个文本框，仍写回「ooxp-example-发布会.pptx」。请先写入上面那份。": "Change the first text box on the first slide and write back to ooxp-example-briefing.pptx. Create the file above first.",
  "ooxp · 已改": "ooxp · edited",
};

const SKIP_TEXT = new Set([
  "font", "eastAsiaFont", "majorLatin", "minorLatin", "majorEa", "minorEa",
  "author", "url", "instr", "file", "prst", "val", "rule", "type", "kind", "op",
  "action", "format", "layout", "ref", "formula", "sqref", "cell", "numFmt",
  "align", "underline", "highlight", "combineBrackets", "pattern", "rgb", "color",
  "side", "offsetFrom", "dir", "wrap",
]);

const PLATS = [
  { id: "win32-x64", ext: "zip", label: "win" },
  { id: "linux-x64", ext: "tar.gz", label: "linux" },
  { id: "linux-arm64", ext: "tar.gz", label: "linuxArm" },
  { id: "darwin-arm64", ext: "tar.gz", label: "mac" },
  { id: "wasm", ext: "tar.gz" },
];

let lang = localStorage.getItem("ooxp-demo-lang") === "en" ? "en" : "zh";
let release = null;
let releaseLoaded = false;
let marksCache = null;

function t(key) {
  return UI[lang][key];
}

function fill(key, vars) {
  let text = t(key);
  for (const [name, value] of Object.entries(vars)) text = text.replaceAll("{" + name + "}", value);
  return text;
}

function localize(node, key) {
  if (lang === "zh") return node;
  if (Array.isArray(node)) return node.map((item) => localize(item, key));
  if (node && typeof node === "object") {
    const out = {};
    for (const [name, value] of Object.entries(node)) out[name] = localize(value, name);
    return out;
  }
  if (typeof node !== "string" || SKIP_TEXT.has(key)) return node;
  return Object.prototype.hasOwnProperty.call(EN_TEXT, node) ? EN_TEXT[node] : node;
}

function localizedFormats() {
  if (lang === "zh") return FORMATS;
  return localize(JSON.parse(JSON.stringify(FORMATS)), "");
}

function renderMcp() {
  const args = lang === "zh" ? ["mcp", "--lang", "zh-CN"] : ["mcp", "--lang", "en"];
  document.querySelector("#mcp-json").textContent = JSON.stringify({
    mcpServers: { ooxp: { command: "ooxp", args } },
  }, null, 2);
}

function renderDownloads() {
  const version = release && release.tag_name ? release.tag_name.replace(/^v/, "") : "";
  const meta = document.querySelector("#release-meta");
  meta.textContent = !releaseLoaded ? t("finding") : version ? t("latest") + " " + version : "";
  for (const plat of PLATS) {
    const link = document.querySelector('.choice[data-plat="' + plat.id + '"]');
    const name = version ? "ooxp-v" + version + "-" + plat.id + "." + plat.ext : "";
    const asset = release && release.assets && name
      ? release.assets.find((item) => item.name === name)
      : null;
    link.querySelector(".kind").textContent = plat.label ? t(plat.label) : plat.id;
    if (!releaseLoaded) {
      link.querySelector(".file").textContent = t("finding");
      link.href = "https://github.com/gaoyia/ooxp/releases/latest";
      link.setAttribute("aria-disabled", "true");
      continue;
    }
    link.removeAttribute("aria-disabled");
    if (!release) {
      link.querySelector(".file").textContent = t("unavailable");
      link.href = "https://github.com/gaoyia/ooxp/releases";
      continue;
    }
    if (!asset) {
      link.querySelector(".file").textContent = (name || plat.id) + " · " + t("missing");
      link.href = release.html_url || "https://github.com/gaoyia/ooxp/releases/latest";
      continue;
    }
    link.querySelector(".file").textContent = asset.name;
    link.href = asset.browser_download_url;
  }
}

function applyPage() {
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  document.title = "Office Open XML Patch";
  document.querySelector("#lead").textContent = t("lead");
  for (const el of document.querySelectorAll("[data-i18n]")) {
    const value = t(el.dataset.i18n);
    if (value) el.textContent = value;
  }
  document.querySelector("#lang-zh").setAttribute("aria-pressed", lang === "zh" ? "true" : "false");
  document.querySelector("#lang-en").setAttribute("aria-pressed", lang === "en" ? "true" : "false");
  document.querySelector(".dir").setAttribute("aria-label", t("dirTitle"));
  renderMcp();
  renderDownloads();
  renderExamples();
  showDir();
  renderFiles(marksCache);
}

function setLang(next) {
  lang = next;
  localStorage.setItem("ooxp-demo-lang", next);
  applyPage();
}

const logEl = document.querySelector("#log");
const dirName = document.querySelector("#dir-name");
const filesEl = document.querySelector("#files");
const changeEl = document.querySelector("#change");
const pickButton = document.querySelector("#pick");
let session = null;
let root = null;
let listing = [];
let busy = false;

function append(text) {
  const p = document.createElement("p");
  p.textContent = text;
  logEl.append(p);
  logEl.scrollTop = logEl.scrollHeight;
}

function partsOf(path) {
  const text = String(path ?? "").trim().replace(/\\/g, "/");
  if (!text || text.startsWith("/") || /^[A-Za-z]:/.test(text)) {
    throw new Error(t("pathRel"));
  }
  const parts = text.split("/").filter(Boolean);
  if (parts.some((part) => part === "." || part === "..")) {
    throw new Error(t("pathDots"));
  }
  return parts;
}

async function dirHandle(parts, create) {
  let dir = root;
  for (const name of parts) {
    dir = await dir.getDirectoryHandle(name, { create });
  }
  return dir;
}

function isBusyFile(err) {
  const name = err && err.name;
  const message = err && err.message ? String(err.message) : "";
  return (
    name === "NoModificationAllowedError" ||
    message.includes("state had changed since it was read from disk") ||
    message.includes("being used by another process")
  );
}

function busyError(path) {
  const err = new Error(fill("busyFile", { path }));
  err.busyFile = true;
  return err;
}

async function readBytes(path) {
  try {
    const parts = partsOf(path);
    const name = parts.pop();
    const dir = await dirHandle(parts, false);
    const handle = await dir.getFileHandle(name);
    const file = await handle.getFile();
    return new Uint8Array(await file.arrayBuffer());
  } catch (err) {
    if (isBusyFile(err)) throw busyError(path);
    throw err;
  }
}

async function writeBytes(path, bytes) {
  const parts = partsOf(path);
  const name = parts.pop();
  if (!name) throw new Error(t("noName"));
  try {
    const dir = await dirHandle(parts, true);
    const handle = await dir.getFileHandle(name, { create: true });
    const writable = await handle.createWritable();
    await writable.write(bytes);
    await writable.close();
  } catch (err) {
    if (isBusyFile(err)) throw busyError(path);
    throw err;
  }
}

function needsPackage(action) {
  return action === "inspect" || action === "apply" || action === "exportMedia" || action === "exportChartData";
}

function formatSize(size) {
  if (size < 1024) return size + " B";
  if (size < 1024 * 1024) return (size / 1024).toFixed(size < 10240 ? 1 : 0) + " KB";
  return (size / 1024 / 1024).toFixed(1) + " MB";
}

function formatTime(ms) {
  return new Date(ms).toLocaleString("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

async function snapshot() {
  const items = [];
  for await (const [name, handle] of root.entries()) {
    if (handle.kind === "file") {
      const file = await handle.getFile();
      items.push({ name, kind: "file", size: file.size, mtime: file.lastModified });
    } else {
      items.push({ name, kind: "dir" });
    }
  }
  items.sort((a, b) => a.name.localeCompare(b.name, lang === "zh" ? "zh" : "en"));
  return items;
}

function marksOf(before, after) {
  const prev = new Map(before.map((item) => [item.name, item]));
  const marks = new Map();
  for (const item of after) {
    const old = prev.get(item.name);
    if (!old) marks.set(item.name, "new");
    else if (item.kind === "file" && (old.size !== item.size || old.mtime !== item.mtime)) {
      marks.set(item.name, "chg");
    }
  }
  return marks;
}

function renderFiles(marks) {
  filesEl.replaceChildren();
  if (!root) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = t("emptyFiles");
    filesEl.append(li);
    return;
  }
  if (listing.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = t("noFiles");
    filesEl.append(li);
    return;
  }
  for (const item of listing) {
    const li = document.createElement("li");
    const name = document.createElement("span");
    name.className = "file-name";
    name.textContent = item.name;
    li.append(name);
    const mark = marks && marks.get(item.name);
    if (mark) {
      const badge = document.createElement("span");
      badge.className = mark === "new" ? "badge new" : "badge chg";
      badge.textContent = mark === "new" ? t("new") : t("chg");
      li.append(badge);
    }
    const meta = document.createElement("span");
    meta.className = "file-meta";
    meta.textContent = item.kind === "dir" ? t("folder") : formatSize(item.size) + " · " + formatTime(item.mtime);
    li.append(meta);
    filesEl.append(li);
  }
}

async function refreshFiles(before) {
  listing = await snapshot();
  marksCache = before ? marksOf(before, listing) : null;
  renderFiles(marksCache);
  return marksCache;
}

function showDir() {
  dirName.textContent = root ? fill("dirReady", { name: root.name }) : t("dirNone");
  document.querySelector("#forget").hidden = !root;
}

function setBusy(next) {
  busy = next;
  for (const button of document.querySelectorAll(".write")) button.disabled = next || !session;
}

let markBytes = null;
let deckCache = null;

function drawPng(width, height, paint) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx.textBaseline = "top";
  paint(ctx, width, height);
  return new Promise((resolvePromise, reject) => {
    canvas.toBlob(async (blob) => {
      if (!blob) {
        reject(new Error(lang === "zh" ? "页面没有画出插图" : "The page could not draw a picture"));
        return;
      }
      resolvePromise(new Uint8Array(await blob.arrayBuffer()));
    }, "image/png");
  });
}

function paintBg(ctx, w, h, holes, lineY) {
  ctx.fillStyle = "#FFFFFF";
  ctx.fillRect(0, 0, w, h);
  ctx.fillStyle = "rgba(14, 92, 86, 0.07)";
  ctx.beginPath();
  ctx.arc(60, h + 40, 460, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#0E5C56";
  ctx.fillRect(132, lineY, 1656, 3);
  for (const hole of holes) ctx.clearRect(hole.x, hole.y, hole.w, hole.h);
}

function paintDot(ctx, w, h) {
  ctx.fillStyle = "#0E5C56";
  ctx.beginPath();
  ctx.arc(w / 2, h / 2, w / 2 - 2, 0, Math.PI * 2);
  ctx.fill();
}

function paintRing(ctx, w, h) {
  ctx.strokeStyle = "#C2410C";
  ctx.lineWidth = 28;
  ctx.beginPath();
  ctx.arc(w / 2, h / 2, w / 2 - 24, 0, Math.PI * 2);
  ctx.stroke();
}

function paintBar(ctx, w, h) {
  ctx.fillStyle = "#0E5C56";
  ctx.fillRect(0, 0, w, h);
}

async function demoMark() {
  if (markBytes) return markBytes;
  markBytes = await drawPng(240, 160, (ctx) => {
    ctx.fillStyle = "#0E5C56";
    ctx.fillRect(0, 0, 240, 160);
    ctx.fillStyle = "#FFFDF8";
    ctx.font = "600 64px Segoe UI, Microsoft YaHei, sans-serif";
    ctx.textBaseline = "middle";
    ctx.fillText("ooxp", 48, 80);
  });
  return markBytes;
}

async function deckPictures() {
  if (deckCache) return deckCache;
  const one = [{ x: 116, y: 92, w: 1688, h: 158 }];
  const two = [{ x: 116, y: 92, w: 1688, h: 302 }];
  const files = {
    "mark.png": await demoMark(),
    "bg-one.png": await drawPng(1920, 1080, (ctx, w, h) => paintBg(ctx, w, h, one, 276)),
    "bg-two.png": await drawPng(1920, 1080, (ctx, w, h) => paintBg(ctx, w, h, two, 418)),
    "deco-dot.png": await drawPng(160, 160, paintDot),
    "deco-ring.png": await drawPng(800, 800, paintRing),
    "deco-bar.png": await drawPng(32, 400, paintBar),
  };
  deckCache = files;
  return files;
}

async function runRequest(raw, statusEl) {
  if (!session) throw new Error(t("libFail"));
  if (!root) throw new Error(t("needDir"));
  let request;
  try {
    request = JSON.parse(raw);
  } catch {
    throw new Error(t("badJson"));
  }
  const action = request.action || "";
  let pkg = new Uint8Array(0);
  if (needsPackage(action)) {
    try {
      pkg = await readBytes(request.path);
    } catch (err) {
      if (err && err.busyFile) throw err;
      throw new Error(fill("missingFile", { path: request.path }));
    }
  }
  session.clear_files();
  const pictures = await deckPictures();
  for (const [name, bytes] of Object.entries(pictures)) session.put_file(name, bytes);
  const before = listing.slice();
  const out = session.run(JSON.stringify(request), pkg);
  if (!out.ok) {
    const message = out.error && out.error.message ? out.error.message : t("fail");
    throw new Error(message);
  }
  if (out.kind !== "bytes") {
    statusEl.textContent = t("noBytes");
    statusEl.className = "status";
    return;
  }
  const path = out.path || request.output || request.path;
  await writeBytes(path, out.bytes);
  const marks = await refreshFiles(before);
  const code = marks && marks.get(path.split(/[/\\]/).pop());
  const mark = code === "new" ? t("new") : code === "chg" ? t("chg") : t("written");
  const line = mark + " " + path;
  changeEl.hidden = false;
  changeEl.textContent = t("thisTime") + line;
  statusEl.textContent = line;
  statusEl.className = "status ok";
  append(line);
}

function renderExamples() {
  const current = document.body.dataset.format || "home";
  const tabs = document.querySelector("#tabs");
  const panels = document.querySelector("#panels");
  tabs.replaceChildren();
  for (const panel of [...panels.querySelectorAll(".panel")]) {
    if (panel.dataset.format !== "home") panel.remove();
  }
  const home = document.createElement("button");
  home.type = "button";
  home.setAttribute("role", "tab");
  home.setAttribute("aria-selected", "true");
  home.dataset.format = "home";
  const homeDot = document.createElement("span");
  homeDot.className = "dot";
  homeDot.style.color = "var(--accent)";
  home.append(homeDot, document.createTextNode(t("home")));
  home.addEventListener("click", () => selectFormat("home"));
  tabs.append(home);
  document.body.dataset.format = "home";

  localizedFormats().forEach((format) => {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-selected", "false");
    tab.dataset.format = format.id;
    const dot = document.createElement("span");
    dot.className = "dot";
    dot.style.color = format.color;
    tab.append(dot, document.createTextNode(format.title));
    tab.addEventListener("click", () => selectFormat(format.id));
    tabs.append(tab);

    const panel = document.createElement("div");
    panel.className = "panel";
    panel.dataset.format = format.id;
    panel.hidden = true;
    for (const item of format.items) {
      const card = document.createElement("article");
      card.className = "card";
      const title = document.createElement("h3");
      title.textContent = item.name;
      const blurb = document.createElement("p");
      blurb.className = "blurb";
      blurb.textContent = item.blurb;
      const details = document.createElement("details");
      const summary = document.createElement("summary");
      summary.textContent = t("request");
      const box = document.createElement("textarea");
      box.spellcheck = false;
      box.value = JSON.stringify(item.request, null, 2);
      details.append(summary, box);
      const actions = document.createElement("div");
      actions.className = "card-actions";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "write";
      button.textContent = t("write");
      button.disabled = true;
      const status = document.createElement("p");
      status.className = "status";
      button.addEventListener("click", () => {
        status.textContent = t("writing");
        status.className = "status";
        setBusy(true);
        runRequest(box.value, status)
          .catch((err) => {
            const message = err.message || String(err);
            status.textContent = message;
            status.className = "status bad";
            append(message);
          })
          .finally(() => setBusy(false));
      });
      actions.append(button, status);
      card.append(title, blurb, details, actions);
      panel.append(card);
    }
    panels.append(panel);
  });
  selectFormat(current);
}

function selectFormat(id) {
  document.body.dataset.format = id;
  for (const tab of document.querySelectorAll("#tabs button")) {
    tab.setAttribute("aria-selected", tab.dataset.format === id ? "true" : "false");
  }
  for (const panel of document.querySelectorAll(".panel")) {
    panel.hidden = panel.dataset.format !== id;
  }
}

function bindIntro() {
  for (const button of document.querySelectorAll("[data-goto]")) {
    button.addEventListener("click", () => selectFormat(button.dataset.goto));
  }
}

function openDb() {
  return new Promise((resolvePromise, reject) => {
    const request = indexedDB.open("ooxp-demo", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("kv");
    request.onsuccess = () => resolvePromise(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function clearHandle() {
  const db = await openDb();
  await new Promise((resolvePromise, reject) => {
    const tx = db.transaction("kv", "readwrite");
    tx.objectStore("kv").delete("dir");
    tx.oncomplete = () => resolvePromise();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

async function saveHandle(handle) {
  const db = await openDb();
  await new Promise((resolvePromise, reject) => {
    const tx = db.transaction("kv", "readwrite");
    tx.objectStore("kv").put(handle, "dir");
    tx.oncomplete = () => resolvePromise();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

async function loadHandle() {
  const db = await openDb();
  const handle = await new Promise((resolvePromise, reject) => {
    const tx = db.transaction("kv", "readonly");
    const request = tx.objectStore("kv").get("dir");
    request.onsuccess = () => resolvePromise(request.result || null);
    request.onerror = () => reject(request.error);
  });
  db.close();
  return handle;
}

async function useDir(handle) {
  root = handle;
  showDir();
  changeEl.hidden = true;
  await refreshFiles(null);
}

document.querySelector("#forget").addEventListener("click", async () => {
  if (!root || busy) return;
  try {
    await clearHandle();
    root = null;
    listing = [];
    changeEl.hidden = true;
    document.querySelector("#restore").hidden = true;
    showDir();
    marksCache = null;
    renderFiles(null);
    append(t("forgot"));
  } catch (err) {
    append(err.message || String(err));
  }
});

pickButton.addEventListener("click", async () => {
  if (!window.showDirectoryPicker) return;
  try {
    const handle = await window.showDirectoryPicker({ mode: "readwrite", id: "ooxp" });
    await saveHandle(handle);
    document.querySelector("#restore").hidden = true;
    await useDir(handle);
  } catch (err) {
    if (err && err.name === "AbortError") return;
    append(err.message || String(err));
  }
});

function showUnsupported() {
  document.querySelector("#unsupported").hidden = false;
  pickButton.disabled = true;
  dirName.textContent = t("unsupported");
  filesEl.replaceChildren();
  const li = document.createElement("li");
  li.className = "empty";
  li.textContent = t("unsupportedEmpty");
  filesEl.append(li);
}

async function boot() {
  document.querySelector("#lang-zh").addEventListener("click", () => setLang("zh"));
  document.querySelector("#lang-en").addEventListener("click", () => setLang("en"));
  applyPage();
  bindIntro();
  const supported = typeof window.showDirectoryPicker === "function";
  if (!supported) showUnsupported();
  else {
    showDir();
    renderFiles(null);
  }
  try {
    await init();
    session = new Session();
    const ping = session.run(JSON.stringify({ action: "help", locale: lang === "zh" ? "zh" : "en" }), new Uint8Array(0));
    if (!ping.ok) throw new Error(ping.error && ping.error.message ? ping.error.message : t("libFail"));
    if (supported) setBusy(false);
    append(t("libReady"));
  } catch (err) {
    append(t("libMissing") + (err.message || String(err)));
    return;
  }
  if (!supported) return;
  try {
    const handle = await loadHandle();
    if (!handle) return;
    const perm = await handle.queryPermission({ mode: "readwrite" });
    if (perm === "granted") {
      await useDir(handle);
      return;
    }
    const button = document.querySelector("#restore");
    button.hidden = false;
    button.classList.add("ghost");
    button.addEventListener("click", async () => {
      const next = await handle.requestPermission({ mode: "readwrite" });
      if (next !== "granted") {
        append(t("permFail"));
        return;
      }
      button.hidden = true;
      await useDir(handle);
    });
  } catch (err) {
    append(err.message || String(err));
  }
}

fetch("https://api.github.com/repos/gaoyia/ooxp/releases/latest", {
  headers: { Accept: "application/vnd.github+json" },
})
  .then((response) => (response.ok ? response.json() : null))
  .then((data) => {
    release = data;
    releaseLoaded = true;
    renderDownloads();
  })
  .catch(() => {
    release = null;
    releaseLoaded = true;
    renderDownloads();
  });

boot();
