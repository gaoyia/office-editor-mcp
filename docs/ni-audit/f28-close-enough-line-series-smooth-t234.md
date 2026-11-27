# F28 close-enough: official-like standard line series smooth produces an image

Official F28 Chart.Export gold already existed for all four charts in
`C:\Users\AMD\AppData\Local\Temp\t173-f28-smooth-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=4).
Source SHA-256
`0eb3ae935e517c6370a9c35b2a64c2f30ccd13097854c86b0d64b00e9633e54a`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is `lineChart` grouping=standard, wrapper `c:smooth=0`,
valAx explicit 0-80 / 20, crossBetween=between, categories A-D:

- F28-1: both series `c:smooth=1`, marker `symbol=none`
- F28-2: both series `c:smooth=1`, marker omitted (auto markers)
- F28-3: S1 `c:smooth=0`, S2 `c:smooth=1`, marker `symbol=none`
- F28-4: both series `c:smooth=0`, marker `symbol=none` (already exported
  on the locked standard-line path; control, not an F28 lock)

F28-3 shows smoothness is per series, not a wrapper-wide flag. Official
gold does not include scatter (already close-enough from F27), areaChart
smooth, wrapper `smooth=1`, stacked line smooth, combo smooth, or blank
points.

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- a single standard `lineChart` (wrapper `smooth=0`) with one or more
  series `c:smooth=1`, marker `symbol=none` or omitted
- mixed series on the same official-like wrapper: unsmoothed series stay
  polylines, smoothed series are curved (F28-3)

The connecting stroke reuses the same close-enough interpolating cubic
already used for official-like scatter (Catmull-Rom converted to cubic
Bezier, tension 0.5, duplicated endpoints, sampled into the existing
polyline stroke, in plot space). Official F28 gold supports this as a
general per-series rule on a standard lineChart. It is documented as
close-enough, not a unique official-matrix lock, and not a unique
endpoint / blank contract. Existing gap handling still breaks a run;
each finite run is curved independently, which is mechanical reuse, not
a unique blank rule.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- `lineChart` wrapper `c:smooth=1`
- `areaChart` series or wrapper `smooth=1`
- stacked or percentStacked line plus series `smooth=1`
- combo or radar or 3D line plus series `smooth=1`
- a dashed line series plus `smooth=1`
- features Office itself cannot Chart.Export
