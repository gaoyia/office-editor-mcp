# F29 close-enough: official-like standard line prstDash produces an image

Official F29 Chart.Export gold already existed for all six charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f29-dash-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=6,
chartExportReturned true). Source SHA-256
`4beb05944d53aed6975dd5c0fb07e1d1be3da31893878510b03096e99ca1d9ff`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a single standard `lineChart` (grouping=standard,
wrapper `c:smooth=0`), one series `F29S1`, eight category points A-H
all valued 40, valAx explicit 0-80 / 20, crossBetween=between, legend
r, marker `symbol=none`, series `c:smooth=0`. All six `a:ln` elements
omit `w` (COM Weight=3 is not in the package and is not inferred).
None write `custDash`:

- F29-1 solid: `prstDash=solid`, cap omitted (already-exported control,
  not an F29 lock)
- F29-2 dash: `prstDash=dash`, cap omitted
- F29-3 squareDot: official `msoLineSquareDot` writes `prstDash=dot`
  and `cap=sq` (not a distinct preset token)
- F29-4 roundDot: official `msoLineRoundDot` writes `prstDash=dot` and
  `cap=rnd`
- F29-5 dashDot: `prstDash=dashDot`, cap omitted
- F29-6 longDash: `prstDash=lgDash`, cap omitted

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- a single standard `lineChart` series whose `a:ln` uses a DrawingML
  `ST_PresetLineDashVal` token already mapped by
  `DashPattern::from_preset_width`, including omitted `w` and omitted
  `cap`
- the official square/round dot pair, which share `prstDash=dot` and
  are distinguished only by explicit `cap=sq` or `cap=rnd`

The period comes from the existing width-percent contract (each binary
run is one resolved pen width), not a new Excel period table. Omitted
`cap` keeps the accepted Round baseline used by solid pens. Omitted `w`
keeps the accepted default width (2.25pt / 4px at 106dpi); an explicit
positive width still uses the already-locked nearest-pixel explicit-pen
rule. Explicit caps already accepted for this pen (`flat` / `sq` /
`rnd`) reuse the same run/cap/join sampler as custom dashes. This is
close-enough export, not a unique official-matrix lock of period,
phase, or terminal-gap geometry, and not a unique omitted-width
contract.

Residuals stay on the review page. Do not auto-mark pass. Library
rerender sizes are 424x265; overlap residuals against official
Chart.Export gold (review-page RGBA overlap): solid 13113, dash 14006,
squareDot 13475, roundDot
13156, dashDot 13089, longDash 13392 (longDash is the only same-canvas
pair at 424x265).

## Still NI

- `custDash` without an explicit positive width and an explicit cap
- scatter, area, combo, radar, or 3D hosts plus a dashed pen
- a dashed line series plus `c:smooth=1`
- unknown `prstDash` tokens, invalid explicit widths, and compound or
  arrow line encodings
- square terminal-gap residuals, lim4 / 180-degree retrace residuals,
  and legend/layout residuals already listed as review leftovers
- features Office itself cannot Chart.Export
