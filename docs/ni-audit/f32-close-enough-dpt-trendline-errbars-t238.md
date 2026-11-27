# F32 close-enough: official-like dPt markers, linear trendlines, and fixed-value error bars produce an image

Official F32 Chart.Export gold already existed for all twelve charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f32-dpt-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=12,
chartExportReturned true). Source `f32-dpt-source.xlsx` SHA-256
`7fc9fb2cc1dc6b545bd8e8bbcea0c37a40a65b689e01628147d59ef297540b63`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office style-2 / style-102 matrix. Series
values are 20,40,60,40,20,50,30,70. Line charts use grouping=standard,
wrapper `c:smooth=0`, series marker `symbol=none`, valAx 0-80 / 20,
crossBetween=between, legend r. Scatter charts use scatterStyle=lineMarker,
numeric x=1..8, series `a:ln` noFill, omitted series marker (default auto
markers), valAx midCat 0-80. Area charts use standard grouping and valAx
midCat 0-80. Official decorations:

- F32-1 lineControl / F32-5 scatterControl / F32-9 areaControl: no dPt,
  trendline, or errBars (already-exported controls, not an F32 lock)
- F32-2 lineDpt / F32-6 scatterDpt: `c:dPt` idx=3, marker symbol=diamond
  size=10, bubble3D=0, c16 uniqueId extLst, no fill / spPr
- F32-3 lineTrend / F32-7 scatterTrend / F32-11 areaTrend:
  `c:trendline` trendlineType=linear, dispRSqr=0, dispEq=0, no spPr, no
  forward / backward / intercept
- F32-4 lineErr / F32-8 scatterErr / F32-12 areaErr: `c:errBars` errDir=y,
  errBarType=both, errValType=fixedVal, noEndCap=0, val=10, no spPr
- F32-10 areaDpt: `c:dPt` idx=3, bubble3D=0, spPr solidFill srgbClr=FF0000,
  c16 uniqueId extLst. Official Chart.Export of this case shows no
  distinct red point-region fill, so no category-band paint rule is
  invented from this matrix.

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- a single standard `lineChart` or `scatterChart` (scatterStyle
  marker / lineMarker / smoothMarker as already accepted) with one or
  more official-like `c:dPt` that carry an explicit marker symbol and
  size, `bubble3D` omitted or off, and an optional inert uniqueId
  extLst. When the dPt has no fill, the marker reuses the already
  resolved series color. An explicit dPt solidFill, if written, is
  used as the marker color. Omitted dPt markers (which would need an
  invented default symbol or color) stay NI.
- a single standard `areaChart` with official-like `c:dPt` that carry
  an opaque solidFill, `bubble3D` omitted or off, and an optional inert
  uniqueId extLst. The chart exports. The official F32 matrix does not
  discriminate a painted point region, so no region fill is invented.
- a single standard line, scatter, or area series with official-like
  `c:trendline` of type linear, dispRSqr/dispEq omitted or off, and no
  spPr, forward, backward, intercept, period, order, or equation label.
  The close-enough stroke is a least-squares line through finite points
  over the finite data domain (no extension), painted with existing
  axis INK at 1px. This is not a unique official fit, width, or style
  lock.
- a single standard line, scatter, or area series with official-like
  y-direction `c:errBars` of type both / plus / minus, errValType
  fixedVal, a finite non-negative val, and noEndCap omitted, off, or
  on, with no spPr. Close-enough vertical stems use existing axis INK
  at 1px; when caps are on, short horizontal ticks are painted at the
  error ends. Cap length is close-enough, not a unique official cap
  table.

This is not a unique official-matrix lock of default marker color, fit
coefficients, trendline style, or error-bar cap geometry.

Residuals stay on the review page. Do not auto-mark pass. Residual
counts against official Chart.Export gold are recorded on the live
review page after regeneration.

## Still NI

- dPt without an explicit marker symbol and size on line/scatter
  (invented default marker / color)
- dPt bubble3D on, unknown children, or non-inert extLst
- area dPt without an opaque solidFill, or with other spPr children
- trendline types other than linear, equation / R^2 display, forward /
  backward / intercept / period / order, or spPr
- errBars with errDir=x, errValType other than fixedVal, custom plus /
  minus lists, or spPr
- more than one trendline or errBars block on a series
- stacked / percentStacked or combo / secondary / 3D hosts plus these
  decorations
- features Office itself cannot Chart.Export
- terminal-gap, lim4, retrace, and legend residuals already listed as
  review leftovers
