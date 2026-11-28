# F40 close-enough: official-like legend manualLayout inner and edge produce an image

Official F40 Chart.Export gold already existed for all 24 charts (six
workbooks times four ChartObjects) in
`C:\Users\AMD\AppData\Local\Temp\t174-f40-legend-manual-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, chartExportReturned
true). Source `f40-legend-manual-source.xlsx` SHA-256
`bb810a0a4fa6227c9caf0666d86807f3f93fe9ce0a5af950464f5e4c75fe20fa`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office clustered `barChart` (col) matrix.
Categories A–D, values 10,20,30,40, titles and series names colAuto /
colManualA / colManualB / colManualC, legendPos=r, overlay=0. colAuto
has no manualLayout. The three official manuals use

`<c:xMode val="edge"/><c:yMode val="edge"/>` plus factor (absent)
wMode/hMode and no layoutTarget (outer). Derived workbooks inject
`layoutTarget=inner` and/or `wMode`/`hMode` `edge` onto those same
x/y/w/h values. Office Chart.Export succeeded for every derived
encoding.

Already-exportable colAuto (all six workbooks share one official image)
and official outer manuals are not re-locked. Their residual pixels
stay on the review page and are not chased as a unique lock.

Official evidence (not a unique lock):

- x, y, w, and h stay in chart space for both outer and inner.
  layoutTarget=inner does not re-originate the legend into the plot
  rectangle: inner keys keep the same chart-space placement as the
  matching outer manual. What changes is plot reservation: inner does
  not apply the automatic right-band reserve, so the plot grows to the
  default insets. That is not an outer no-op.
- wMode=edge / hMode=edge are the right / bottom edge in that same
  chart space, applied after left / top. Non-degenerate official A/C
  key shifts match a centered pack in the shortened box. Combined
  wMode+hMode composes; inner+edge keeps the edge-mode keys and the
  inner plot reserve.
- Degenerate hMode=edge on ManualB (bottom edge above the top edge)
  still Chart.Exports: no visible legend key, plot reservation stays
  the outer automatic band. This is not a factor-size fallback and not
  a guessed minimum box.

## What now exports

An official-like clustered column legend with default typography, no
legend frame, overlay=0, and manualLayout using xMode/yMode edge
(factor or omitted wMode/hMode, or wMode/hMode edge) and layoutTarget
omitted, outer, or inner must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for those Office-exportable
encodings.

## Leftover NI

These stay NotImplemented until a later official-matrix lock (or until
Office itself cannot Chart.Export them, in which case they are skipped):

- pie / doughnut / ofPie / pie3D legend manualLayout
- typography, deleted-entry, entry-override, and legend frame/border
  variants combined with manualLayout
- unknown layoutTarget or wMode/hMode values
- features Office itself cannot Chart.Export

The old showLeaderLines source gate is not reopened. Already-exportable
outer factor manuals and automatic right legends are not rewritten to
chase residuals.

## Close-enough, not a unique official lock

Keeping inner coordinates in chart space while dropping the automatic
pos reservation, and treating wMode/hMode=edge as the right/bottom
edge in that space (with degenerate non-positive sizes unpainted), is
an authorized close-enough stroke so Office-exportable official
encodings produce an image. It is not a unique official-matrix lock of
every manualLayout, not an outer no-op, and not a guessed
right-or-bottom formula invented without the official edge and
degenerate evidence. Residual title, canvas, key, and plot pixels stay
on the review page.
