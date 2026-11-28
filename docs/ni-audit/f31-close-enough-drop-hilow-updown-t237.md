# F31 close-enough: official-like empty dropLines, hiLowLines, and upDownBars produce an image

Official F31 Chart.Export gold already existed for all ten charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f31-droplines-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=10,
chartExportReturned true). Source SHA-256
`e5c142929cda1500cfb2133594b72fe18bc4f1afe777bc893e105ee5822ac9b5`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office style-2 / style-102 matrix. Line
charts use grouping=standard, wrapper `c:smooth=0`, marker `symbol=none`,
series `c:smooth=0`, valAx 0-80 / 20, crossBetween=between, legend r.
Area charts use standard grouping and valAx midCat 0-80. Decorations
are empty elements with no spPr:

- F31-1 lineControl: no drop/hiLow/upDown (already-exported control, not
  an F31 lock)
- F31-2 lineDrop: empty `c:dropLines` on one series
  (20,40,60,40,20,50,30,70)
- F31-3 lineHiLow: empty `c:hiLowLines` on two series (S2 =
  50,20,40,55,30,50,20,50)
- F31-4 lineUpDown: `c:upDownBars` with `gapWidth=150` and empty
  `upBars`/`downBars`
- F31-5 lineAll: empty dropLines + hiLowLines + the same upDownBars
- F31-6/7/8 scatterDrop/HiLow/UpDown: COM HasDropLines / HasHiLoLines /
  HasUpDownBars read True, but the saved package writes no
  dropLines/hiLowLines/upDownBars (already-exported scatter path, not
  an F31 scatter-decoration lock)
- F31-9 areaControl: no dropLines (already-exported standard area
  control, not an F31 lock)
- F31-10 areaDrop: empty `c:dropLines` on a standard midCat area

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- a single standard `lineChart` whose wrapper has an empty `c:dropLines`
  and/or empty `c:hiLowLines`, and/or official-like `c:upDownBars` with
  a finite non-negative `gapWidth` (default 150) plus empty `upBars` and
  `downBars`
- a single standard `areaChart` whose wrapper has an empty `c:dropLines`

Close-enough paint follows the OOXML meanings, not invented unique
geometry: drop lines go from each finite point to the value-axis
crossing; high-low lines connect the category min and max across
series; up/down bars span the first and last series (up when last >
first) and reuse the existing single-column `gapWidth` width formula.
Empty decorations reuse existing axis INK for strokes and a
close-enough light/dark fill for bars. Default stroke width, exact
official gray/off-white bar fills, and any unique style-matrix colors
are not locked from this one default-style group and are not invented
as a unique official-matrix table. This is not a unique official-matrix
lock of drop, high-low, or up-down geometry or style.

Residuals stay on the review page. Do not auto-mark pass. Library
rerender sizes are 424x265; overlap residuals against official
Chart.Export gold (review-page RGBA overlap): lineControl 14893,
lineDrop 16490, lineHiLow 18721, lineUpDown 20922, lineAll 21760,
scatterDrop 12110, scatterHiLow 15311, scatterUpDown 14950,
areaControl 13048, areaDrop 14730 (scatterDrop is the only same-canvas
pair at 424x265).

## Still NI

- dropLines / hiLowLines / upDownBars that carry spPr or other children
- scatter wrappers that actually write these elements in XML (official
  Excel 16 did not persist the COM switches in this gold)
- stacked / percentStacked or combo / secondary hosts plus these
  decorations
- features Office itself cannot Chart.Export
- terminal-gap, lim4, retrace, and legend residuals already listed as
  review leftovers
