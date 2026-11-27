# F25 close-enough: other chart types produce an image

Official F25 Chart.Export gold already existed for all six default
wrappers in
`C:\\Users\\AMD\\AppData\\Local\\Temp\\t170-f25-other-types-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT,
`chartExportReturned` true for doughnut, pie-of-pie, radar marker,
bubble, stock HLC, and surface top-down). Excel was not relaunched.
Residual pixels stay on the review page. The recorded zero-pixel
standard is not met and is not rewritten as if it were.

## What now exports

These official-like wrappers must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- single-series `doughnutChart` with holeSize in 10..=90 (ECMA / official
  default 50), firstSliceAng 0, varyColors=1, no axes
- single-series `ofPieChart` ofPieType=pie with omitted
  splitType/splitPos/custSplit (close-enough auto split = last three
  finite non-zero points, keeping at least one on the main pie),
  gapWidth default 150, secondPieSize default 75, optional serLines
- `radarChart` radarStyle=marker, varyColors=0, lines plus auto/default
  markers, first category at 12 o'clock clockwise
- `bubbleChart` bubble3D=0 (or omitted), showNegBubbles=0 (or omitted),
  bubbleScale default 100. Numeric strings in an xVal strCache are
  parsed as numbers; non-numeric headers and non-positive sizes are
  skipped. Official Chart.Export of this gold parks bubbles in a
  vertical stack near x=0; the library places them on the numeric x,
  and that residual is authorized
- `stockChart` with exactly three series (High/Low/Close), hiLowLines,
  and no upDownBars. dateAx category serials (1900 system) are formatted
  as yyyy-mm-dd for labels; this is close-enough, not a unique
  official date-axis scale lock
- `surfaceChart` (not surface3DChart) with wireframe=0 (or omitted) and
  top-down view3D rotX=90, rotY=0, rAngAx=0, perspective=0. Painted as
  a triangulated category-by-series contour with close-enough
  major-unit value bands

These are close-enough approximations, not unique official-matrix
locks. Title and legend stay in 2D screen space. Official doughnut and
ofPie write hidden dLbls plus showLeaderLines=1; with no visible labels
that flag is already inert (F24). Deleting the chart-wrapper whitelist
is not an implementation; the wrappers were already listed there.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- ofPieChart ofPieType=bar, and any explicit splitType / splitPos /
  custSplit
- radarChart radarStyle=filled or standard
- bubbleChart bubble3D=1 or showNegBubbles=1
- stockChart OHLC / volume (not exactly three series) or upDownBars
- surfaceChart wireframe=1, or any view3D other than the official
  top-down encoding
- doughnut holeSize outside 10..=90
- features Office itself cannot Chart.Export
