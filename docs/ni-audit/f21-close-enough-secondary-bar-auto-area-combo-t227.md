# F21 close-enough: bar / auto bounds / area / column+line secondary valAx

Official F21 Chart.Export gold already existed for all eight folder cases.
The first four (two-series primary control, same-type column secondary,
mixed-sign column secondary, same-type line secondary) already exported
and are regenerated only; residual pixels are left on the review page.
The recorded zero-pixel standard is not met and is not rewritten as if
it were.

## What now exports

Four official cases were still NI even though official gold exists:

- F21-5 `bar-s2-secondary` (`sheet:Sheet1/chart:6`) / chart5.xml: two
  clustered bar wrappers, S1 2,4,6,8 on explicit 0–10 major 2, S2
  20,40,80,60 on 0–100 major 20. Official Excel 16 writes primary catAx
  axPos=l, primary valAx axPos=b autoZero between, secondary valAx
  axPos=t crosses=max between, hidden secondary catAx axPos=l delete=1.
  Official 425×266. This is the already-locked secondary pairing composed
  with existing bar geometry: crosses=max still means the axis is on the
  opposite edge (top), and growth remains autoZero. Same-type bars overlap
  in the same category slots (one-series gap per wrapper). Close-enough,
  not a unique official-matrix lock.

- F21-6 `col-s2-secondary-auto` (`sheet:Sheet1/chart:7`) / chart6.xml: the
  same two clustered column wrappers and data as F21-2, but both valAxes
  omit min/max/majorUnit. Official 424×266, visually near F21-2 (Excel
  auto 0–10 / 0–100). Existing `scale_values` auto bounds are reused.
  Close-enough, not a unique official auto-scale lock.

- F21-7 `area-s2-secondary` (`sheet:Sheet1/chart:8`) / chart7.xml: two
  standard area wrappers, explicit 0–10 / 0–100, `crossBetween=midCat` on
  both valAxes, same l/r secondary pairing and hidden secondary cat.
  Official 425×266. F16 already opened standard-area midCat; this composes
  that rule with the locked pairing. Not a new unique midCat lock.

- F21-8 `combo-col-line-secondary` (`sheet:Sheet1/chart:9`) / chart8.xml:
  clustered column + standard line, explicit 0–10 / 0–100, between, same
  locked l/r pairing. Official 424×266. The column is painted on the
  primary scale and the line on the secondary scale using the existing
  paint paths. Close-enough for this official case only; not a general
  F26 heterogeneous-combo lock, and not a single-series overlay pretending
  to support both wrappers.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- log, orientation=maxMin, valAx delete, or value labels combined with a
  secondary valAx
- more than one series per wrapper
- secondary majorGridlines
- a visible (non-deleted) secondary category axis
- other heterogeneous combos beyond this official column+line case (F26)
- Features Office itself cannot Chart.Export were skipped
