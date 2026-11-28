# F39 close-enough: official-like pie automatic legend positions produce an image

Official F39 Chart.Export gold already existed for all ten charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f39-pie-legend-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=10,
chartExportReturned true). Source `f39-pie-legend-source.xlsx` SHA-256
`9b0f72afaf6f03d0ac4e576f634b65585f4d0c5de0ee6c94330279653f419df8`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office standard `pieChart` / clustered
`barChart` (col) matrix. Pie and column charts share the same categories
A–D, values 10,20,30,40, titles F39-1…, chart fill EAF7F7, and automatic
legend encodings `<c:legendPos val="r|l|t|b|tr"/><c:overlay val="0"/>`
with no manualLayout and no legend `spPr`. Pie charts also carry
chart-level dLbls with every show flag off and `showLeaderLines=1`
(the existing accepted hidden-label encoding; the old showLeaderLines
source gate is not reopened).

Official encodings:

- pieLegR / colLegR: `legendPos=r`, overlay 0 (already-exported)
- pieLegL / colLegL: `legendPos=l`, overlay 0 (pie was rejected)
- pieLegT / colLegT: `legendPos=t`, overlay 0 (pie was rejected)
- pieLegB / colLegB: `legendPos=b`, overlay 0 (pie was rejected)
- pieLegTR / colLegTR: `legendPos=tr`, overlay 0 (pie was rejected)

Column five-position charts already export. Their residual pixels stay
on the review page and are not chased as an F39 unique lock. The
existing right-legend pie path, including `pie_plot_bounds` and
`draw_pie_legend`, stays bit-identical and is not rewritten to chase
pieLegR residuals.

Official pie evidence (not a unique lock): left uses the same side
reserve width as right (same diameter, swapped side margins) with LTR
keys, not a mirrored key/text cluster. Top and bottom use a smaller
horizontally centered pie and a tight vertical seam of about
`LEGEND_PLOT_GAP` with the pie square. TopRight keeps a right-side
reserve (not a full-width top band) and top-aligns keys with the pie
square. Official TopRight is slightly smaller and shares the right
pie's bottom edge; that residual stays on the review page.

## What now exports

An official-like standard single-wrapper `pieChart` automatic legend
with `legendPos` r/l/t/b/tr, explicit `overlay=0`, default typography,
no manualLayout, no deleted entries or entry overrides, no legend
frame, and no border must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for those Office-exportable
encodings.

Layout reuses the already-locked pie diameter formula inside a newly
reserved rectangle (`pie_plot_bounds_reserved`). It does not invent a
new 0.875 scale, and it does not treat the old right-band
`pie_plot_bounds` contract or the Cartesian five-position reserve as
already locking these five pie positions.

- Left: the existing right-band width is applied as `left_band`. Keys
  stay LTR, inset with existing `LEGEND_PAD`.
- Top / bottom: a vertical band from existing `LEGEND_SWATCH` +
  `LEGEND_PLOT_GAP` + `LEGEND_PAD` tokens. The key row is centered
  horizontally; the shared pie–key seam is `LEGEND_PLOT_GAP`.
- TopRight: the same right band and the same pie square as right.
  Keys use the same right x as right and are top-aligned to the pie
  square (`plot.y`). No invented corner shrink and no full top band.

## Leftover NI

These stay NotImplemented until a later official-matrix lock (or until
Office itself cannot Chart.Export them, in which case they are skipped):

- overlay=1 on pie l/t/b/tr (absence of overlay equals overlay 1)
- pie legend `manualLayout` (F40)
- typography, deleted-entry, entry-override, and legend frame/border
  variants
- doughnut / ofPie / pie3D non-right automatic legends
- features Office itself cannot Chart.Export

Cartesian five-position layout is unchanged. The old showLeaderLines
source gate is not reopened.

## Close-enough, not a unique official lock

Reserving a band from existing pie tokens, then placing LTR keys
against the shared `LEGEND_PLOT_GAP` seam (and top-aligning TopRight
keys with the right-band pie square), is an authorized close-enough
stroke so Office-exportable official encodings produce an image. It is
not a unique official-matrix lock of every automatic pie legend, not a
mirrored layout, and not a copied Cartesian reserve. Residual title,
canvas, diameter, key-size, and inset pixels stay on the review page.
