# F17 close-enough: standard area midCat plus catAx crosses=min/max

This reuses two already-established general rules: standard-area midCat
category slots (first and last categories on the plot edges, from F16) and
catAx crosses=min/max (fill from the scale endpoint; crosses=max keeps a top
category-label gutter, unlike a clamped crossesAt). It is not a new unique
official-matrix lock. The recorded zero-pixel standard is not met and is not
rewritten as if it were.

## What now exports

Official F17 Chart.Export gold already existed for all eight folder cases.
Seven already-exportable column/line controls (F17-1 through F17-7) were
regenerated only; residual pixels are left on the review page.

One case previously left 仍NI even though official gold exists now renders,
because the F16 midCat gate and the existing StandardArea crosses=min/max
allowance already compose:

- F17-8 `area-mixed-min` (`sheet:Sheet1/chart:9`) / chart8.xml: standard area,
  values -4/-1/3/6, explicit min=-8 max=8 major=2, catAx crosses=min, valAx
  stays autoZero, `crossBetween=midCat`. Official 424×266. Official fill sits
  on the plot edges (x≈36–334) and runs from the data polyline down to the
  min axis (y≈84–227, just above the min grid at 228), with a bottom label
  gutter like official column crosses=min.

The historical library export of this official file returned NotImplemented
only because midCat standard area was still rejected. After F16 that encoding
exports; this pass documents the composition, adds regression tests, and
regenerates the F17 review page. No new unique clip or gutter formula was
invented.

`crosses=max` on a midCat standard area is allowed by the same general rule
(no official F17 gold for that twin). Do not treat it as a uniquely locked
case. Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- valAx-encoded crosses=min/max (official Excel 16 writes them on catAx)
- log plus crosses=min/max
- horizontal bar / scatter / pie crosses=min/max
- Features Office itself cannot Chart.Export were skipped
