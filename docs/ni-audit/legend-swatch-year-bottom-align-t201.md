# t201u legend swatch-to-year bottom alignment

Authorized by the user (t201u, 2026-11-27, Asia/Shanghai): after the t200u
horizontal gap approximation, the legend color swatch (图样) and the
series-name years (字) should share a bottom edge. This note records a
general vertical placement rule, not a typography rewrite and not a
per-fixture pixel table. The t200u horizontal gap rule is unchanged. Do not
auto-pass the review page.

## What was misaligned

On official 106 dpi Chart.Export, 2025 year ink shares a bottom edge with
the 8×8 swatch (dBot=0; ink starts 2 px above the swatch). Official 2026 on
F04/F05 hangs 1 px below the swatch (dBot=1), likely the digit’s ink, not a
different layout; t191 both series stay at dBot=0.

The library (HEAD 73d18262, after t200) placed default-right legend text at
`y - scale_px(2)`, a typographic-top fudge. At 106 dpi the swatch is 9 px
(`131..139`) and year ink sat at `131..141` (dTop=0, dBot=2): the GDI
baseline is `top + line_height()` (TEXTMETRIC ascent = 12), so the years
hung 2 px below the last swatch pixel. The same dBot=2 appeared on all F04
and F05 variants and on t191 replica-t166 / replica-t171. Horizontal first
ink was already the t200 result (F04 380 / t191 394) and was not the issue.

## Rule change (paint.rs)

`draw_legend` and `draw_pie_legend` now place the typographic top with
`legend_filled_label_top`: the GDI (or fontdue) baseline sits on the last
swatch pixel, `y + swatch_h - 1`. That is `top = (y + swatch_h - 1) -
line_height()`. At 106 dpi this is `y-4` instead of the old `y-2`, moving
year ink up 2 px. The typography-aware `legend.rs` Layout path is unchanged
(it already centers keys inside its own line box, and F04/F05/t191 default
right legends use `paint.rs` `draw_legend`).

96 dpi uses the same ascent rule, not a frozen `y-2`. With a 8 px swatch and
ascent 11, the typographic top is also `y-4` (2 px higher than the old
fudge). Horizontal t200 behavior is untouched: 96 dpi still uses
`LEGEND_GAP=6` for both draw and reservation; non-96 filled keys still draw
with a 2 device-pixel gap and reserve `scale_px(LEGEND_GAP)`.

This is a close-enough layout fix, not a uniquely locked Office metric, and
not a re-guess of default title typography.

## After-change measurements (official Chart.Export vs new library, dpi=106)

Swatch left edges and horizontal first-ink / empty columns are unchanged
from t200. Vertical residuals:

| Case | Official dBot (2025 / 2026) | Before library dBot | After library dBot |
| --- | --- | --- | --- |
| F04 four variants | 0 / 1 | 2 / 2 | 0 / 0 |
| F05 five official counterparts | 0 / 1 | 2 / 2 | 0 / 0 |
| t191 replica-t166 / replica-t171 | 0 / 0 | 2 / 2 | 0 / 0 |

Library 2025 now matches official bottoms and dTop=-2. Library 2026 is also
on the swatch bottom; official F04/F05 2026 remains 1 px lower (honest
residual, not auto-passed). t191 both series match official dBot=0.

New library renders: `C:\Users\AMD\AppData\Local\Temp\t201-library-rerender-20261127`.
Official gold was not mutated. No official Excel session was launched.
