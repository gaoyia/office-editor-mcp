# F16 close-enough: standard area midCat plus catAx crossesAt

This reuses the already-established midCat category-slot rule (first and last
categories on the plot edges) and the already-implemented catAx crossesAt
baseline fill. It is not presented as a new unique official-matrix lock.
The recorded zero-pixel standard is not met and is not rewritten as if it were.

## What now exports

Official F16 Chart.Export gold already existed for all eight folder cases.
Seven already-exportable column/line controls were regenerated only; residual
pixels are left on the review page.

One case previously left 仍NI even though official gold exists now renders:

- F16-8 `area-mixed-at-2` (`sheet:Sheet1/chart:9`) / chart8.xml: standard area,
  values -4/-1/3/6, explicit min=-8 max=8 major=2, catAx crossesAt=2, valAx
  stays autoZero, `crossBetween=midCat` (Excel's default area encoding).
  Official 424×266. Official fill sits on the plot edges (x≈36–334) and meets
  the crossing at y=132 (value 2), not the zero grid at y=155.

The library previously accepted only a restricted `crossBetween=between`
standard-area fixture, and the midCat gate was limited to stacked /
percentStacked area. Official F16-8 is non-stacked standard area with midCat,
so it returned NotImplemented.

## General rule

- Standard area with valAx `crossBetween=midCat` now exports. Category x uses
  the same `category_x_midcat` formula already locked for stacked areas:
  first/last categories on the plot edges. The between path is unchanged.
- catAx crossesAt on standard area still fills to the clamped crossing, not
  to 0. Official F16-8 uses both midCat and crossesAt=2 together.
- Stacked / percentStacked area still require midCat (between stays NI) and
  still reject custom crosses (unchanged F22/F23 gates).
- valAx crossesAt, log plus crossesAt, and horizontal bar / scatter / pie
  crossesAt stay NI. Features Office itself cannot Chart.Export were skipped.

Residuals (including official often 425×266 or 424×266 vs library 424×265,
plot-top / grid-color / font layout) stay on the review page. Do not auto-mark
pass.
