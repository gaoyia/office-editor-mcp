# F26 close-enough: official-like combo wrappers produce an image

Official F26 Chart.Export gold already existed for all six charts in
`C:\Users\AMD\AppData\Local\Temp\t171-f26-combo-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT,
`chartExportReturned` true for clustered-column + standard-line on a
shared primary axis, two clustered columns + standard line on a shared
primary axis, clustered-column + standard-line on a secondary valAx,
standard area + clustered column on a shared between valAx, clustered
bar + standard line with Excel's automatic right valAx, and the
clustered-column control). Source SHA-256
`23bfc2db084aa527747112f1fae74fc38f75773f93b9637889c08e1b14b779f9`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

This is not a general F26 heterogeneous-combo lock. The earlier F21-8
column+line secondary case remains a close-enough composition for that
official F21 encoding; official F26-3 reuses that path and is not a
new unique lock.

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- `barChart` col clustered (1 series, gapWidth=150) plus `lineChart`
  standard (1 series, marker=none, smooth=0) sharing one catAx/valAx
  pair with crossBetween=between (F26-1)
- the same pairing with 2 column series in the barChart wrapper; cluster
  width uses only the column series, not the line (F26-2)
- clustered-column + standard-line on a secondary valAx axPos=r
  crosses=max, hidden secondary catAx (F26-3). This reuses the already
  landed F21-8 close-enough composition
- `areaChart` standard plus `barChart` col clustered, 1+1 series, sharing
  the same between valAx (not standalone area midCat). Area is painted
  behind the columns (F26-4)
- `barChart` bar clustered plus `lineChart` standard, where Office
  placed the line on a secondary valAx axPos=r / hidden catAx axPos=b
  while the bar keeps catAx l and valAx b (F26-5). The line is painted
  in column orientation on the right scale; this mixed layout is
  close-enough composition, not a general combo lock
- the two-dimensional clustered-column control (F26-6) already exported
  on the existing column path and is not an F26 combo

Each wrapper uses its own already-locked paint path. Shared-primary
area / column / line layers are painted back to front. These are
close-enough approximations, not unique official-matrix locks.

Residuals stay on the review page. Do not auto-mark pass. Official
F26-4 legend follows plotArea wrapper order (area then column); the
library keeps series-index order. That residual is authorized.

## Still NI

- three or more wrappers
- stacked or percentStacked combo wrappers
- line+area, bar+area, and other wrapper pairs without official gold
- a line combo with smooth=1 or a visible marker symbol
- more than two column series in a shared-primary col+line combo
- features Office itself cannot Chart.Export

