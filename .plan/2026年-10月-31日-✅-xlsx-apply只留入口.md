# xlsx apply 只留入口

状态：✅ 已完成。
日期：2026-10-31

不在三种格式的待做表里，不占待做序号。这是 `.plan/2026年-10月-30日-✅-大文件按领域拆开.md` 文末「后续」表里的 `xlsx/apply.rs`。领域 JSON 不变，写出的 XML 不变。

## 为什么要做

`docs/architecture.md` 规定：`apply.rs` 只打开包、按 op 分发、写回脏部件；某个对象已经有模块时，op 函数体放在那个模块。docx 已经按此拆开，xlsx 还没有。

`crates/ooxp/src/xlsx/apply.rs` 现在 2715 行、50 个函数。入口和分发之外，还装着整套单元格、公式、`sheetData` 的读写：

- `apply_set_cell`、`write_array_formula`、`write_shared_formula`、`explode_shared_group`
- `ensure_sheet_data`、`ensure_row`、`ensure_cell`、`clear_cell`、`fill_cell`、`find_row_insert_pos` 等
- `make_refs_explicit`、`expand_dimension`、`mark_full_calc`、`remove_calc_chain`

xlsx 没有单独的单元格模块，这些代码只能放在入口里。

## 目标

- 新建 `xlsx/cell.rs`，单元格、公式、`sheetData` 的读写搬过去。
- `apply.rs` 只留：`apply`、`match`、多个对象共用的装载（`load_dirty`、`dirty_of`、`sheet_index` 一类）和按名找工作表的公共函数。
- `match` 分支里如果还有成段的实现，搬到对应的对象模块。

## 不做

- 不改任何函数在做什么，只换位置。参数和返回值保持原样。
- 不顺手做性能优化。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：模块清单写上 `xlsx/cell.rs` | ✅ |
| 2 | 列出要搬的函数与去向，写回本节 | ✅ |
| 3 | 搬到 `cell.rs` 与对应对象模块 | ✅ |
| 4 | 测试 | ✅ |

### 2. 去向

- `cell.rs`：`setCell` 的热缓存、单元格 id、隐式编号补全、公式写入、`sheetData` 查找和插入、重算标记、删计算链。
- `view.rs`：排序颜色编进 `dxf` 的 `intern_sort_dxfs`。
- `apply.rs` 留下入口、`match`，以及 `load_dirty`、`dirty_of`、`sheet_index` 和按名找工作表的函数。

### 4. 验收

- `cargo test --workspace` 全过，不改任何期望值。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- `xlsx/apply.rs` 不再装着某一类对象的整段实现。
- 写出的 XML 与改动前逐字节相同，不要求 Office。
