# t200u legend swatch-to-year gap (approximation)

Authorized by the user (t200u, 2026-11-27, Asia/Shanghai): after t197u measured
a consistently larger library gap between the right-hand legend color swatch
and the series-name years (2025 / 2026, not category labels), and confirmed it
is spacing rather than left-vs-center alignment, the user said to move the
years a little closer — close enough. This note records a general
approximation, not a unique Office lock for every DPI or typography. Do not
auto-pass the review page. Do not lower the zero-pixel standard.

## What was too wide

On official 106 dpi Chart.Export, the filled-key swatch is 8×8 and there are
3–4 empty columns before the first year ink (F04 / F05 first ink ~379; t191
replica-t166 / replica-t171 first ink 395). The library used `LEGEND_SWATCH=8`
and `LEGEND_GAP=6` as 96 dpi pixels, then `scale_px` at 106 dpi (9 and 7).
Text was placed at swatch_left + 9 + 7, so first ink sat at 385 on F04 (8
empty columns) and 399 on the t191 replicas (8 empty columns). Swatch left
edges were already aligned (F04 both at x=368). Official CopyPicture
(~96 dpi plus a 1 px seam, not the acceptance canvas) does not uniquely
support changing the locked 96 dpi 8/6 constants. Official year glyphs also
look heavier; typography was not re-guessed. This was not caused by t196
valAx density.

## Rule change (paint.rs / types.rs)

96 dpi still uses the locked `LEGEND_GAP=6` for both the drawn swatch-to-name
gap and the plot-area reservation, so existing 96 dpi inset / oracle tests
stay on the same numbers.

At non-96 export DPI, filled-key (column, bar, area, pie) series-name text is
drawn with a 2 device-pixel gap (`LEGEND_GAP_SCALED_DPI`) instead of
`scale_px(LEGEND_GAP)`. The plot reservation still uses `scale_px(LEGEND_GAP)`
so the swatch left edge does not shift when the drawn gap tightens. Line and
scatter keys keep the existing 1.5 pt gap at every DPI.

This is an authorized close-enough move, not a uniquely locked Office metric.
The constant 2 was chosen because it brings official-export first ink near the
measured columns (F04 ~379, t191 ~395) without a per-fixture table and without
changing 96 dpi behavior.

## After-change measurements (official Chart.Export vs new library, dpi=106)

Before: F04 / F05 library first ink 385, empty 8; t191 replicas first ink 399,
empty 8. Swatches 9×9, left edges already aligned with official.

| Case | Official first ink / empty | New library first ink / empty | Residual |
| --- | --- | --- | --- |
| F04 four variants | 379 / 3 (8×8, x=368..375) | 380 / 3 (9×9, x=368..376) | first ink 1 px right; empty now matches |
| F05 five official Chart.Export counterparts | 379 / 3 (8×8, x=368..375) | 380 / 3 (9×9, x=368..376) | same 1 px first-ink residual |
| t191 replica-t166 / replica-t171 | 395 / 4 (8×8, x=383..390) | 394 / 3 (9×9, x=382..390) | first ink 1 px left; empty 3 vs 4; swatch left 382 vs 383 is pre-existing |

Honest residuals stay on the review page and are not auto-passed. Official
gold was not mutated. No official Excel session was launched for this change.

New library renders: `C:\Users\AMD\AppData\Local\Temp\t200-library-rerender-20261127`.
Review page (updated in place, not auto-passed):
`C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261127-F04-legend-year-gap\F04-legend-year-gap-待审.html`.
Superseded t197 images, if kept, live under `assets/history-t197-pre-approximation/`.
The original desktop F04 noFill page was left untouched.
