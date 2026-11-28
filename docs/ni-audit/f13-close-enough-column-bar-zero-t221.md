# F13 close-enough: column/bar `dispBlanksAs=zero`

This is a documented general rule, not a unique official-matrix lock.
The recorded zero-pixel standard is not met and is not rewritten as if it were.

## What now exports

Official F13 Chart.Export gold already existed for unlabeled clustered columns:

- `col-neg-zero` (`sheet:Sheet1/chart:6`): values −4 / omitted idx 1 / −9, explicit `dispBlanksAs=zero`
- `col-na-zero` (`sheet:Sheet1/chart:7`): values 4 / `#N/A` / 9, explicit `dispBlanksAs=zero`

The already-exportable area omitted/zero/gap and `col-neg-gap` controls are regenerated only; residual pixels are left on the review page.

## General rule

- A true blank or omitted cache point plus `dispBlanksAs=zero` becomes value `0`, the same rule already used for StandardLine.
- A 0-height column or bar is not painted (`y_val == y_zero` / width `< 0.2`).
- `#N/A` is not a blank, so `dispBlanksAs` does not apply: the point stays `None` and is skipped, not filled as 0 (same accepted line subset).
- Other errors such as `#DIV/0!` plus `dispBlanksAs=zero` stay NotImplemented.
- Visible value labels on a remaining blank or error point stay NotImplemented, so a `showVal` cannot silently disappear. Official F13 column gold is unlabeled.
- Horizontal clustered bar uses the same general rule. `dispBlanksAs=span` on bar/column is unchanged this pass.
- Area error points, pie/scatter/other-type zero, `dispNaAsBlank`, and unresolved sparse domain stay NI. Features Office itself cannot Chart.Export are skipped.

## Official gold notes (do not treat as a unique lock)

Official `col-neg-gap` vs `col-neg-zero` is the same 424×266 canvas and differs by 78 pixels: the title glyphs F13-4 vs F13-5 (y=16–25, x=220–227) plus a 1-row vertical jitter of a left-side horizontal stroke at y=120 vs 121, x=39–46 (axis/grid ink, not a visible mid-slot zero column). There is no independently visible zero-height column and no axis-range change. That is consistent with painting 0 as a skipped 0-height mark, but it is not a unique “zero equals gap” lock.

Library vs official residuals (size 424×265 vs 424×266 on several official labeled/column frames, palette vs RGB, and remaining ink) stay on the review page. Do not auto-mark pass.
