# 图表标题 typography：显式 sz 已锁定，缺省字号仍 NI（2026-11-26）

## 授权与范围

原 USERt183u：从已合并 PR14 之后的最新 `main`（`8c5c9db4`）开新分支继续剩余 NI。标题字号与导出不一致是 2026-11-26 已用磁盘官方 Chart.Export 确认的缺口；本批只实现能唯一锁定的合同，不猜测 18/14 磅缺省常数，不降低零像素标准，不自动接受。

## 只读 COM 证据（fresh safe-idle0，未再导出）

本机观察 Excel 进程数为 0 后，按既有 owned-readonly 协议启动全新 Excel 16.0 / 窗口 DPI 106 会话：只读打开既有 t166 / t171 副本，读取 `ChartTitle.Font.Size/Name/Bold`，不 Save / Quit / kill，不 Chart.Export，finally `Close(false)`，Workbooks=0，源 SHA 与副本 SHA 均未变化，无 repair 日志。

结果（11 张图，状态均为 `CLEAN_PUBLIC_OFFICE_FONT_READ`）：

- t166（中文标题，空 `a:defRPr`，仅 `<c:style val="2"/>`）：Size=**10**，Name=**等线**，Bold=**True**（4/4）
- t171（英文 “Legend foundation”，空 `a:defRPr`，`c14:style val="102"` + fallback style 2）：Size=**18**，Name=**等线**，Bold=**True**（7/7）

两种 XML 都没有 `sz`。10 与 18 同时出现，因此**不能**把缺省标题写成单一 10 或 18 磅常数。工作簿语言（`c:lang=zh-CN`）相同；变量与脚本（CJK vs 拉丁）和 style 编码（裸 style 2 vs 102/2）混杂，当前样本不能唯一归因。空 Excel 窗口（PID 46528）按协议保留，未清理；因此本批不再开第二次 Office，g29/g30/g31 native 仍 pending。

证据根：`C:\Users\AMD\AppData\Local\Temp\t183-title-font-com-20261126\`。

## 唯一可实现的合同：显式 `sz`

ECMA-376 DrawingML `sz` 为百分之一磅。图例解析早已采用同一词法范围。本批只在标题 `defRPr`/`rPr` **显式给出 sz** 时启用新路径：

- `sz` 为有限小数，范围 100..=400000，且不超过 40900（与图例同一拒绝阈值），再除以 100 得到磅值
- 使用 `Face::open_dpi` / `open_cell_dpi` 与画布导出 DPI（含 106），不再把导出像素误当作 96 DPI 字号
- 同时解析显式 `b` / `i` / `latin|ea` typeface；`+` 主题字体占位仍 NI
- 下划线、删除线、无 sz 仅有粗体/字体等不完整 typography 保持 NI，不静默丢语义
- **空 defRPr / 仅 lang 的缺省标题仍走旧 `LABEL_PT=10` + `Face::open` 路径**，不把 10 或 18 写进缺省，以保持既有缺省标题光栅，直到缺省规则能被唯一锁定

未实现、也不声称：缺省标题与官方导出逐像素一致；theme major（Aptos Display / 等线 Light）绑定；`Face::open` 对轴标签的 96 DPI 假设。

## 验证

提交前重跑强制套件 cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1：2337 passed / 190 targets / 0 failed / 0 ignored（相对 g31 的 2332 增加本批 5 个新测试，此为当次新跑）。python -m unittest discover -s manual-check/lib -p test_controlled_chart_*.py -v：15 passed。cargo build --offline --locked -p ooxp-cli 成功。新增测试覆盖：缺省不发明磅值、百分之一磅、具名字体、不完整/装饰性 typography 保持 NI、10pt 与 18pt 在 106 DPI 下光栅不同。
