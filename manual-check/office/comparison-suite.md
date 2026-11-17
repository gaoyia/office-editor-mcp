# PR10 整套测试修正（进行中，非整套验收完成）

仅0914快照定义范围：主列表22个功能行、58个图片记录；附带2个历史阻塞行/12个参考图，共70条图片记录。图标集/超大切块2功能原来没有图片证据，不能凭空补PASS。没有纳入后续17/27/49等独立矩阵。

## 已完成的测试基础修复
- 70条归档图片全部清点；62条可恢复原请求，8条采用已检查同名源的新显式整表请求（不是伪造历史参数）。主58条均绑定source SHA、当前真实请求、HEAD、CLI SHA并实际重绘；44份独立源副本，旧快照/源不改。
- stroke-abs原手写空xdr:style违反CT_ShapeStyle的4个必需引用，已只从测试生成器省略该可选节点；修正版仅drawing1.xml内容变化。源副本SHA12ab9a4bb5b458e5106e30d1a2c1ec7bab07a787b1bb713c2ab1ad366979c68c。
- 修正版已实际Office打开/导出，无匹配恢复日志，源不变/自有wb0。raw 73×177，库65×161；不再使用原错误黑线217×45做gold。
- 16份整表源已在同一个Excel实例逐份打开/关闭，生成19条记录可复用的新同源同范围raw；剪贴板序号必须变化，禁止拿旧剪贴板非空当成功；源SHA前后相同、ownWorkbookCount0、exclusiveRead=true。没有重复打开savedcopy，没有Save/Quit/kill/删锁。
- 初次batch最后释放app时有Release-Com作用域错误：16份各自导出/Close/独占读均已成功，但wrapper退出1。原失败日志完整保留logs/excel-batch-initial-wrapper-error.log；脚本已改为直接标准Marshal释放，未为掩盖日志重开16份文件。

## 尺寸不能假装全相同
已核实当前Excel窗口106 DPI，库固定96。微软公开Range.CopyPicture只有Appearance/Format，Chart.Export只有FileName/FilterName/Interactive，没有DPI/宽高参数；没有修改系统DPI/注册表，也未修改生产输出迁就Office。PowerPoint的Slide.Export有原生ScaleWidth/ScaleHeight，后续可按源逻辑尺寸96 DPI导出。
当前Excel新raw仍尺寸不同。统一物理比例展示单独叫view-only-96dpi.png，明确使用96/106重采样并白底合成，仅方便查看，不是gold、不是像素通过证据；raw从未缩放裁边。字体MDW在96/106离散量化也可能导致归一后几何不完全重合，不自动称DPI-only通过。

## 尚未完成，不能让用户现在批量验收
余39条主列表图片尚未取得本轮新Office参考：含Word25、独立spark3、chart4（2个图在两个功能行重复）、PPT7。原参考仍只作为legacy展示，不标新gold/不自动勾PR。Word既有公开PDF路径曾有超时，不能无界重复25次；本阶段未重新开启Word或PPT。未批量改renderer/新功能；旧十项暂停不变。

## 入口与复现
index.html：70条同页折叠索引；comparison-status.csv：原生尺寸/新鲜性/哈希；suite-manifest.json：完整请求与来源；excel-native/*/office-export.json：原始范围、DPI、剪贴板、关闭证明。
仓库脚本：pr10_comparison_suite.py（清点/独立源/当前CLI），comparison-oracle.ps1及comparison-excel-batch.ps1（严格Excel导出），comparison-suite-report.py（只读raw+明确展示视图）。
本轮未改Rust，无renderer重建/全Rust重跑声明；真实CLI仍ce8d112d生成的52855d426c338327c8281b12130e2d562aea72b8315071e97fe1625a9eff9e56。

标准来源：用户t136修整套测试基础、t127已证明DPI-only可豁免；本表不把“来源一致/尺寸原因已知”自动提升为整功能通过。

参考公开规范/API：
- https://schemas.liquid-technologies.com/OfficeOpenXML/2006/ct_shapestyle.html
- https://learn.microsoft.com/en-us/office/vba/api/excel.range.copypicture
- https://learn.microsoft.com/en-us/office/vba/api/excel.chart.export
- https://learn.microsoft.com/en-us/office/vba/api/powerpoint.slide.export

本阶段实际验证：56 Python单测全过，3个PowerShell脚本语法检查、Python py_compile、cargo fmt --all -- --check、git diff --check通过；未重跑Rust单测/clippy/CLI构建（无Rust改动）。
