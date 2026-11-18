# 工作表斜线远边网格与 medium 笔刷

状态：✅
日期：2026-11-18
分类：绘制缺陷

用户 t36u 授权继续收已指出的旧问题。本份覆盖斜线端点与 medium 偏细（多样本位置/大小/方向，默认 96 与 106），不重复图片锚点或绝对远边，不把归档当成收完。PPT 渐变页底、chartsheet 画布、组合 AABB、图标 NI、虚线斜线仍不在本次范围。

## 错误历史归因（已纠正）

此前把官方细线端点写成「内缩」、把 medium≈3px 写成「不为单图改线宽」，未补多样本。新探针说明：

- 细线不是把两端各缩一个固定像素，而是 Bresenham 画到格子**远边网格** `(x1,y1)`（up 从右上到左下 `(x1,y0)→(x0,y1)`），含起点不含终点（LineTo）。画到 `(x1-1,y1-1)` 会错相位，看起来像端点内缩。
- 官方会落在远边网格（下一格的首像素）。先前「不得渗邻格」仍应禁止朝**左/上**渗进上一格或左邻格，这是真实保护，不得回退；远边 1px 是官方几何。
- 整表左/上外框 `x=0` 或 `y=0` 官方不落斜线墨。
- 孤立 medium 在多种宽高、位置、上/下及交叉上，列向众数厚度都是 3 像素，不是合同原来的 2 像素 L 形加粗。

## 锁定规则

- 细线：从格子上角网格 LineTo 到远边网格；允许 `x=x1` / `y=y1`；不含终点；不画 `x=0` 或 `y=0`；不画 `x<x0` 或 `y<y0`。
- medium：同一条中线**含终点**（盖住官方线帽），再按中心加四邻加粗（约 3 像素）；裁剪规则与细线相同。正交边框的 medium 仍是 2 像素，不改。
- 不为行高 29px 做特判。15 条细线里 12 条与官方逐像素一致；其余 3 条（A3、C3、L16）都是 29px 高，各差 1 个阶梯像素，不强改。medium 有界多样本下总对称差 84，尽力接近，不无限追逐最后阶梯差。

## 官方证据（专用非桌面 Temp，未写冻结金样）

新探针 `C:\Users\AMD\AppData\Local\Temp\pr11-diag-endpoint-20261118-142000\source\xlsx-diag-endpoint-probe.xlsx` SHA `9EBD66B1FEE3E7FFEB69B4E01F5BB3014F5C8AA0920C1F63587D5C03024107B5`。A1:L16，25 格唯一色 thin/medium、上/下/交叉，多位置与大小。工作表直接子元素顺序合法。Excel 16.0 pid 28048，窗口 DPI 106，2026-11-18 14:24:38–14:24:40 Asia/Shanghai，源前后不变、独占读、无锁、repair=0、`Close(false)`，两跑 raw 同 SHA `A6096F4146D6892E20D13455E80067D3CE7B0AE1E1B32EF3401421F64779862E`，两份证书均为 `CLEAN_PUBLIC_OFFICE_EXPORT`。官方与修前库 106 均为 1001×439，整图 differ=3256。本轮未再打开该包 Office。

原斜线探针 SHA `F3703CD0042E6FFD0F7850DED278640935DBFF930C2BD60F506A6AE69F9AF3B7` 复用 035703 合格官方 raw 256×110（SHA `0203FA8F39CE13DE692DFEED7AD5EE2F6E704A476EAE618415C8967FF33D9C45`），本轮未再打开该包 Office。修前 differ=567。

## 修复后对照（106，`exportSheetImage`）

CLI `ooxp.exe` SHA256 `7DCC72EC7C16907BEE73226C99BEB0ACD6FEC744D687063BEE34E44DD35A1272`（mtime 2026-11-18 14:36:30 Asia/Shanghai）。源 SHA 导出前后不变。左/上渗色均为 0；左/上外框无斜线墨。

- 新探针：1001×439，differ 3256→90（0.02%）。细线 12/15 差 0；A3/C3/L16 各差 2（29px 高的 1 个阶梯）。medium 十格总对称差 84。
- 原探针：256×110，differ 567→28（0.099%），全部在 A2 medium 交叉；A1/B1/C3 细线 differ=0。

## 测试

- 新增 `diagonal_thin_uses_far_grid_lineto_and_skips_leading_frame`、`medium_diagonal_does_not_spill_into_the_cell_to_the_left`；保留 `medium_diagonal_does_not_spill_into_the_cell_above`。
- `cargo fmt --all`、`cargo clippy -p ooxp --all-targets -- -D warnings` 通过。
- 相关回归：`xlsx_sheet_border`（17）、`xlsx_sheet_shape`（24）、`xlsx_sheet_picture`（8）、`xlsx_geometry_image`（4）通过。
- 完整 `cargo test -p ooxp -- --test-threads=1` 于 2026-11-18 14:36:56–14:41:45 Asia/Shanghai 通过，exit 0，耗时 289456ms（约 4 分 49 秒）。

## 不做

- 不按 29px 特判，不无限追逐 medium 最后阶梯差。不改正交 medium，不改 `setColWidth` 存盘语义，不改系统 DPI。
- 不扩大虚线斜线 / 仅有标志无 style / CF 斜线 NI。不新开渲染引擎。
- 不开始 PPT 渐变页底 / chartsheet 画布 / 组合 AABB / 图标 NI。
- 不写冻结金样。虚线首相位、文字位置/AA 仍待继续，不把本刀当成旧问题全部收完。
