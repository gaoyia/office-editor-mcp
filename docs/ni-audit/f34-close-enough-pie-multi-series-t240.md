# F34 close-enough: official-like multi-series pies produce an image from the lowest-order series

Official F34 Chart.Export gold already existed for all five charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f34-pie-series-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=5,
chartExportReturned true). Source `f34-pie-series-source.xlsx` SHA-256
`67a54afd1ac8436187991483ba9bcb1308623100dfbd5c51c3fa7c0c3d069a76`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office standard `pieChart` matrix. Categories
are A–D, varyColors=1, firstSliceAng=0, legend r, overlay 0, chart-level
dLbls with every show flag off and `showLeaderLines=1` (the existing
accepted hidden-label encoding; the old showLeaderLines source gate is
not reopened). Series values:

- F34S1: 10,20,30,40
- F34S2: 25,15,40,20
- F34S3: 5,35,15,45

Official encodings:

- F34-1 pieControl: one series F34S1 (already-exported single-pie
  control, not an F34 lock)
- F34-2 pieTwoDirect: F34S1 then F34S2, both with strRef names
- F34-3 pieThreeDirect: F34S1, F34S2, F34S3
- F34-4 pieTwoViaColumn: same two series as F34-2 (built via PlotBy
  column)
- F34-5 pieNewSeries: F34S1 plus F34S2 with a literal `c:tx/c:v` name

All five official Chart.Export images use the same first-series slice
proportions (about 10/20/30/40 of the pie fill), not S2 or S3, and the
same single-pie layout (not concentric rings or side-by-side pies).
Pixel differences between the gold images are title, 1-px canvas, and
edge/legend residuals.

## What now exports

An official-like standard single-wrapper `pieChart` with more than one
official-like series must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for those Office-exportable
encodings.

The selection RULE from this official matrix is: paint one pie from the
series with the lowest `c:order` (then the lowest `c:idx`, then document
order) using the existing single-series pie path, including its
categories, point colors, and explosion. This is not an invented
first-or-last layout, and it is not a unique official-matrix lock of
every pie variant. A later series is not painted; doughnut, ofPie, and
3D pie wrappers keep their existing single-series gates.

pieControl already-exportable residuals stay on the review page and are
not chased as a unique lock.

## Leftover NI

These stay NotImplemented until a later official-matrix lock (or until
Office itself cannot Chart.Export them, in which case they are skipped):

- doughnut / ofPie / pie3D with more than one series
- an extra pie series that itself carries a still-NI encoding (visible
  dLbls, gradFill, and the other existing pie leftovers)
- non-default legend layout (F39)
- features Office itself cannot Chart.Export

The old showLeaderLines source gate is not reopened.

## Close-enough, not a unique official lock

Selecting the lowest-order series and painting it with the existing
single-pie layout is an authorized close-enough stroke so
Office-exportable official encodings produce an image. Residual title,
canvas, and edge pixels stay on the review page.
