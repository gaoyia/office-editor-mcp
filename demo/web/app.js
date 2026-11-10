import init, { Session } from "./pkg/ooxp_wasm.js?v=__OOXP_BUILD__";
import xlsxCreate from "./samples/xlsx.json?v=__OOXP_BUILD__" with { type: "json" };
import docxCreate from "./samples/docx.json?v=__OOXP_BUILD__" with { type: "json" };
import pptxCreate from "./samples/pptx.json?v=__OOXP_BUILD__" with { type: "json" };

const FORMATS = [
  {
    id: "xlsx",
    title: "工作簿",
    color: "var(--xlsx)",
    items: [
      {
        name: "销售经营看板",
        blurb: "KPI、月度公式、数据条、色阶、冻结窗格、筛选和柱状图，另有一张区域明细。写入「ooxp-example-销售经营看板.xlsx」。",
        request: xlsxCreate,
      },
      {
        name: "改看板数字",
        blurb: "把总览标题和 1 月的 2026 年收入改掉，仍写回「ooxp-example-销售经营看板.xlsx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-销售经营看板.xlsx",
          ops: [
            { op: "setCell", id: "cell:总览!A1", value: "2026 销售经营看板 · 已改" },
            { op: "setCell", id: "cell:总览!C10", value: 920 },
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
        name: "经营分析报告",
        blurb: "页眉页脚、水印、页边、标题、表格和柱状图，另有公式、图示、环形图、拼音、控件、脚注与批注。写入「ooxp-example-经营分析报告.docx」。",
        request: docxCreate,
      },
      {
        name: "改标题",
        blurb: "把标题换成新文字，仍写回「ooxp-example-经营分析报告.docx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-经营分析报告.docx",
          ops: [{ op: "setParaText", id: "para:00000001", text: "2026 年度产品经营分析报告 · 已改" }],
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
        name: "产品战略发布",
        blurb: "五页演示，带不同切换和逐项动画。目录单击展开，数据页单击出图，目标页单击出表。写入「ooxp-example-产品战略发布.pptx」。",
        request: pptxCreate,
      },
      {
        name: "改封面标题",
        blurb: "把封面标题改掉，仍写回「ooxp-example-产品战略发布.pptx」。请先写入上面那份。",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "ooxp-example-产品战略发布.pptx",
          ops: [{ op: "setShapeText", id: "slide:1/shape:7", text: "2026 产品战略发布 · 已改" }],
        },
      },
    ],
  },
];

const UI = {
  zh: {
    lead: "ooxp 的 WebAssembly 示例",
    slogan: "全球 Office 编辑最好的 MCP 之一",
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
    slogan: "One of the best MCPs for editing Office files.",
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
  "宽": "Wide",
  "密": "Tight",
  "正": "Regular",
  "窄": "Narrow",
  " 黄": " Mark",
  "1月": "Jan",
  "2月": "Feb",
  "3月": "Mar",
  "4月": "Apr",
  "5月": "May",
  "6月": "Jun",
  "交付": "Deliver",
  "其他": "Other",
  "区域": "Region",
  "华东": "East China",
  "华北": "North China",
  "华南": "South China",
  "合计": "Total",
  "同比": "YoY",
  "周明": "Zhou Ming",
  "备注": "Note",
  "季度": "Quarter",
  "实施": "Services",
  "开场": "Opening",
  "总览": "Overview",
  "指标": "Metric",
  "收入": "Revenue",
  "收尾": "Close",
  "月份": "Month",
  "林夕": "Lin Xi",
  "正文": "Body",
  "渠道": "Channel",
  "状态": "Status",
  "目标": "Target",
  "续费": "Renew",
  "苏禾": "Su He",
  "西南": "Southwest",
  "订阅": "Subscribe",
  "说明": "Notes",
  "陈可": "Chen Ke",
  "预测": "Forecast",
  "  疏": "  Loose",
  " 与 ": " and ",
  "1 月": "January",
  "2 月": "February",
  "3 月": "March",
  "4 月": "April",
  "5 月": "May",
  "6 月": "June",
  "毛利率": "Gross margin",
  "负责人": "Owner",
  "达成率": "Attainment",
  "企业订阅": "Enterprise subscriptions",
  "假期影响": "Holiday effect",
  "公式说明": "Formulas",
  "关键动作": "Key move",
  "区域明细": "Regions",
  "同比变化": "YoY change",
  "同比增速": "YoY growth",
  "复核项：": "Review: ",
  "收入构成": "Revenue mix",
  "数据版本": "Data version",
  "核心数据": "Core numbers",
  "活跃客户": "Active customers",
  "渠道下沉": "broader channels",
  "生成工具": "Tool",
  "生成方式": "How it was made",
  "目标拆解": "Targets",
  "示例文档": "Sample document",
  "示例演示": "Sample deck",
  "示例科技": "Sample Co.",
  "稳健增长": "steady growth",
  "第一季度": "Q1",
  "第三季度": "Q3",
  "第二季度": "Q2",
  "第四季度": "Q4",
  "经营回顾": "Review",
  "结论  ": "Conclusion  ",
  "营业收入": "Revenue",
  "落地节奏": "Cadence",
  "谢谢观看": "Thank you",
  "企业版上线": "Enterprise edition launched",
  "半年度收官": "Half-year close",
  "工作簿说明": "About this workbook",
  "示例工作簿": "Sample workbook",
  "16.9 万": "169,000",
  "2025 年": "2025",
  "2026 年": "2026",
  "一、执行摘要": "1. Executive summary",
  "三、季度趋势": "3. Quarterly trend",
  "二、核心指标": "2. Core metrics",
  "季度目标拆解": "Quarterly targets",
  "核心经营数据": "Core operating numbers",
  "渠道下沉启动": "Channel expansion started",
  "续费流程改造": "Renewal process redesign",
  "阅读提示  ": "How to read  ",
  "：营业收入 ": ": revenue ",
  "企业版订阅上线": "Enterprise subscription launch",
  "四、下一步行动": "4. Next steps",
  "增长主要来自 ": "Growth came mainly from ",
  "春节前交付集中": "Deliveries clustered before the holiday",
  "活跃客户(万)": "Active customers (10k)",
  "活跃客户（万）": "Active customers (10k)",
  "渠道下沉与培训": "Channel expansion and training",
  "净推荐值 NPS": "NPS",
  "华东交付中心投产": "East China delivery center opened",
  "工具与格式说明：": "Tools and format: ",
  "收入目标（万元）": "Revenue target (10k yuan)",
  "营业收入（万元）": "Revenue (10k yuan)",
  "10,760 万元": "107.6 million yuan",
  "ooxp 试用示例": "ooxp sample",
  "ooxp 项目主页": "ooxp project",
  "五、附注与资料来源": "5. Notes and sources",
  "同比增速写成公式：": "Year-over-year growth as a formula:",
  "图 2  经营闭环": "Figure 2  Operating loop",
  "月度收入对比（万元）": "Monthly revenue (10k yuan)",
  "2026 产品战略发布": "2026 product strategy",
  "2026 销售经营看板": "2026 sales board",
  "分季度营业收入（万元）": "Quarterly revenue (10k yuan)",
  "月度收入明细与达成情况": "Monthly revenue and attainment",
  "目录 / Agenda": "Agenda",
  "经营两个字可以带拼音。": "经营 can carry a ruby gloss.",
  "查看领域 JSON 契约": "Read the domain JSON contract",
  "改已有段落仍用 apply。": "Editing an existing paragraph still uses apply.",
  "2026 年度产品经营分析报告": "2026 annual business review",
  "六、这份 JSON 还能写什么": "6. What else this JSON can write",
  "2026 年前三季度，业务保持 ": "Through Q3 2026, the business stayed on ",
  "内部资料 · 仅供演示   第 ": "Internal · demo only   Page ",
  "区域销售明细（2026 上半年）": "Regional sales (2026 H1)",
  "尾注集中在文末，用来放资料来源。": "The endnote sits at the end and holds the source.",
  "把年度目标落到每个季度、每个负责人": "Break the annual target down by quarter and owner",
  "从数据到决策：一套可复制的增长方法论": "From numbers to decisions: a repeatable growth method",
  "四个议题，串起从复盘到落地的完整闭环": "Four topics, from the review through to delivery",
  "图 1  分季度营业收入对比（万元）": "Figure 1  Quarterly revenue (10k yuan)",
  "OOXML, pptx, 演示, 示例": "OOXML, pptx, slides, sample",
  "OOXML, xlsx, 看板, 示例": "OOXML, xlsx, board, sample",
  "图 3  2026 年收入构成（万元）": "Figure 3  2026 revenue mix (10k yuan)",
  "图表和图示都是行内的，不会浮到标题上。": "The chart and the diagram are inline, so they do not cover the heading.",
  "这一章只负责炫技，不改前面的经营结论。": "This section only shows what the file can hold. It does not change the findings above.",
  "Q & A   ·   数据与资料可下载": "Q & A   ·   The numbers and the files can be downloaded",
  "下面每一样都由 create 直接写出：": "Each of the following is written by create:",
  "脚注写在本页底部，和正文的批注不是一处。": "The footnote sits at the bottom of the page, separate from the comment.",
  "=IF(F10>=1,\"达标\",\"跟进\")": "=IF(F10>=1,\"Met\",\"Watch\")",
  "=IF(F11>=1,\"达标\",\"跟进\")": "=IF(F11>=1,\"Met\",\"Watch\")",
  "=IF(F12>=1,\"达标\",\"跟进\")": "=IF(F12>=1,\"Met\",\"Watch\")",
  "=IF(F13>=1,\"达标\",\"跟进\")": "=IF(F13>=1,\"Met\",\"Watch\")",
  "=IF(F14>=1,\"达标\",\"跟进\")": "=IF(F14>=1,\"Met\",\"Watch\")",
  "=IF(F15>=1,\"达标\",\"跟进\")": "=IF(F15>=1,\"Met\",\"Watch\")",
  "OOXML, docx, 示例, 经营分析": "OOXML, docx, sample, business review",
  "以上对象都在同一份 create 请求里。": "All of these objects are in the same create request.",
  "建立季度滚动预测机制，减少一次性预算偏差。": "Set up a rolling quarterly forecast so the budget is not fixed once a year.",
  "2026 年前三季度 · 收入与利润同步改善": "Through Q3 2026 · revenue and profit improved together",
  "建立滚动预测机制，让资源投入跟着结果动态调整。": "A rolling forecast lets spending follow the results.",
  "按季度拆分目标，明确负责人、里程碑与验收口径。": "Split the target by quarter, with an owner, a milestone, and a definition of done.",
  "OOXP  ·  2026 年度产品经营分析报告": "OOXP  ·  2026 annual business review",
  "ooxp-example-产品战略发布.pptx": "ooxp-example-strategy.pptx",
  "ooxp-example-经营分析报告.docx": "ooxp-example-report.docx",
  "ooxp-example-销售经营看板.xlsx": "ooxp-example-sales.xlsx",
  "数据来源：合并报表（模拟数据） · 03 / 05": "Source: consolidated figures (sample) · 03 / 05",
  "用一张图看清季度趋势，把 KPI 与业务动作对齐。": "One chart for the quarterly trend, with KPIs lined up to the work.",
  " 两条主线；成本侧通过供应链集中采购对冲了部分投入。": ". Central purchasing offset part of the extra cost.",
  "在华东与华南新增 3 个区域交付中心，缩短实施周期。": "Open three more delivery centers in East and South China to shorten implementation.",
  "按区域汇总的季度收入，含 SUM 公式与色阶条件格式": "Quarterly revenue by region, with SUM formulas and a color scale",
  "由 ooxp 领域 JSON 生成的 Excel 示例": "An Excel sample generated from ooxp domain JSON",
  "由 ooxp 领域 JSON 生成的 Word 文档示例": "A Word sample generated from ooxp domain JSON",
  "公式、循环图示、环形图、字宽字距、拼音、复选框和交叉引用。": "a formula, a cycle diagram, a doughnut, character width and spacing, ruby text, a checkbox, and a cross-reference.",
  "完成企业版订阅的续费流程改造，目标把续费率提升至 92%。": "Redesign enterprise-subscription renewal, aiming for a 92% renewal rate.",
  "把毛利率口径统一到财务新准则，并在 12 月完成回溯调整。": "Align gross margin with the new accounting policy and finish the look-back in December.",
  "注：Q4 为预测值，按渠道签约节奏折算 · 04 / 05": "Note: Q4 is a forecast from the channel pipeline · 04 / 05",
  "复盘 2025 年的收入结构与渠道变化，找出真正的增长来源。": "Review the 2025 revenue mix and channel shift, and name the real source of growth.",
  "由 ooxp 领域 JSON 生成的 PowerPoint 示例": "A PowerPoint sample generated from ooxp domain JSON",
  "下表对比 2025 与 2026 年的关键经营指标（单位见首列）。": "The table compares key 2025 and 2026 metrics. Units are in the first column.",
  "ooxp（领域 JSON → OOXML），未调用 Excel 生成": "ooxp (domain JSON to OOXML), written without Excel",
  "含合并单元格、KPI 卡片、公式、条件格式、冻结窗格、筛选与嵌入图表": "Merged cells, KPI cards, formulas, conditional formatting, a frozen pane, a filter, and an embedded chart",
  "说明：2026 年第四季度为预测值，按渠道签约节奏与历史季节性折算。": "Note: Q4 2026 is a forecast from the channel pipeline and the usual seasonal pattern.",
  "，同比增长 27.8%；毛利率提升 3.4 个百分点至 44.6%。": ", up 27.8% year over year. Gross margin rose 3.4 points to 44.6%.",
  "数据口径：合并报表 · 更新于 2026-11-10 · 金额单位 万元": "Basis: consolidated figures · updated 2026-11-10 · amounts in 10k yuan",
  "公式按字符串保存，本工具不计算；用 Excel 打开后会自动重算并显示结果": "Formulas are stored as text. This tool does not calculate them. Excel recalculates when the file opens.",
  "5 页 16:9 演示文稿：封面、议程、数据图表、目标表格、结束页，带切换和逐项动画": "A 5-slide 16:9 deck: cover, agenda, chart, target table, and close, with transitions and per-object animation",
  "OOXP 领域 JSON 生成示例   ·   2026.11   ·   内部资料": "OOXP domain-JSON sample   ·   2026.11   ·   Internal",
  "OOXP  ·  用领域 JSON 读写 Office 文件   ·   05 / 05": "OOXP  ·  Read and write Office files with domain JSON   ·   05 / 05",
  "KPI 卡片、月度明细、SUM/IF 公式、数据条与三色刻度、冻结窗格、自动筛选、嵌入柱状图": "KPI cards, a monthly table, SUM/IF formulas, data bars and a three-color scale, a frozen pane, an autofilter, and an embedded column chart",
  "在收入与利润双增的前提下，建议把资源继续向高毛利的订阅业务倾斜，并保持对交付产能的同步投入。": "With both revenue and profit up, keep shifting effort toward high-margin subscriptions and match delivery capacity to that shift.",
  "包含标题层级、混排文字、项目符号、带底纹表格、嵌入式图表，以及公式、图示、环形图、拼音、控件和水印的完整示例": "Headings, mixed runs, bullets, a shaded table, an embedded chart, plus a formula, a diagram, a doughnut, ruby text, content controls, and a watermark",
  "OOXP 领域 JSON 生成示例   |   数据截至 2026 年第三季度   |   编制日期 2026-11-10": "OOXP domain-JSON sample   |   Figures through Q3 2026   |   Prepared 2026-11-10",
  "本文档由一份 JSON 请求整包生成。前五章是经营分析，第六章集中演示公式、图示、环形图、字距、拼音、控件、脚注、批注和交叉引用。未调用 Office。": "This document is created from one JSON request. The first five sections are the business review. Section 6 shows a formula, a diagram, a doughnut, spacing, ruby text, controls, a footnote, a comment, and a cross-reference. Office was not called.",
  "工作簿": "Workbook",
  "文档": "Document",
  "演示文稿": "Slides",
  "销售经营看板": "Sales board",
  "改看板数字": "Edit the board",
  "KPI、月度公式、数据条、色阶、冻结窗格、筛选和柱状图，另有一张区域明细。写入「ooxp-example-销售经营看板.xlsx」。": "KPIs, monthly formulas, data bars, a color scale, a frozen pane, a filter, and a column chart, plus a regional sheet. Writes ooxp-example-sales.xlsx.",
  "把总览标题和 1 月的 2026 年收入改掉，仍写回「ooxp-example-销售经营看板.xlsx」。请先写入上面那份。": "Change the overview title and January 2026 revenue, then write back to ooxp-example-sales.xlsx. Create the file above first.",
  "2026 销售经营看板 · 已改": "2026 sales board · edited",
  "cell:总览!A1": "cell:Overview!A1",
  "cell:总览!C10": "cell:Overview!C10",
  "经营分析报告": "Business review",
  "改标题": "Edit the title",
  "页眉页脚、水印、页边、标题、表格和柱状图，另有公式、图示、环形图、拼音、控件、脚注与批注。写入「ooxp-example-经营分析报告.docx」。": "A header and footer, watermark, page border, headings, a table, and a column chart, plus a formula, a diagram, a doughnut, ruby text, controls, a footnote, and a comment. Writes ooxp-example-report.docx.",
  "把标题换成新文字，仍写回「ooxp-example-经营分析报告.docx」。请先写入上面那份。": "Replace the title and write back to ooxp-example-report.docx. Create the file above first.",
  "2026 年度产品经营分析报告 · 已改": "2026 annual business review · edited",
  "产品战略发布": "Product strategy",
  "改封面标题": "Edit the cover",
  "五页演示，带不同切换和逐项动画。目录单击展开，数据页单击出图，目标页单击出表。写入「ooxp-example-产品战略发布.pptx」。": "Five slides with different transitions and per-object animation. The agenda advances on click, the chart waits for a click, and so does the table. Writes ooxp-example-strategy.pptx.",
  "把封面标题改掉，仍写回「ooxp-example-产品战略发布.pptx」。请先写入上面那份。": "Change the cover title and write back to ooxp-example-strategy.pptx. Create the file above first.",
  "2026 产品战略发布 · 已改": "2026 product strategy · edited",
};

const SKIP_TEXT = new Set([
  "font", "eastAsiaFont", "majorLatin", "minorLatin", "majorEa", "minorEa",
  "author", "url", "instr", "file", "prst", "val", "rule", "type", "kind", "op",
  "action", "format", "layout", "ref", "sqref", "cell", "numFmt",
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
      link.href = "https://github.com/gaoyia/office-editor-mcp/releases/latest";
      link.setAttribute("aria-disabled", "true");
      continue;
    }
    link.removeAttribute("aria-disabled");
    if (!release) {
      link.querySelector(".file").textContent = t("unavailable");
      link.href = "https://github.com/gaoyia/office-editor-mcp/releases";
      continue;
    }
    if (!asset) {
      link.querySelector(".file").textContent = (name || plat.id) + " · " + t("missing");
      link.href = release.html_url || "https://github.com/gaoyia/office-editor-mcp/releases/latest";
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
      button.disabled = busy || !session;
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
    await init(new URL("./pkg/ooxp_wasm_bg.wasm?v=__OOXP_BUILD__", import.meta.url));
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

fetch("https://api.github.com/repos/gaoyia/office-editor-mcp/releases/latest", {
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
