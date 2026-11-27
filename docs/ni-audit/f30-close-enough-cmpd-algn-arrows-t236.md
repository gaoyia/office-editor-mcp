# F30 close-enough: official-like standard line cmpd, algn=in, and arrows produce an image

Official F30 Chart.Export gold already existed for all ten charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f30-linecap-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=10,
chartExportReturned true). Source SHA-256
`d7d3a3f385ad69536dcbd1d11b4166fd07304d443db6f5b1e70375680e670ab3`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a single standard `lineChart` (grouping=standard,
wrapper `c:smooth=0`), one series, eight category points A-H all valued
40, valAx explicit 0-80 / 20, crossBetween=between, legend r, marker
`symbol=none`, series `c:smooth=0`. This public LineFormat matrix does
not write `cap`. Non-round cap evidence remains on F29 (`prstDash=dot`
with `cap=sq` or `cap=rnd`). All styled charts use explicit
`w=76200` (6pt):

- F30-1 controlDefault: no series `a:ln` (already-exported control, not
  an F30 lock)
- F30-2 controlThick: `w=76200` `cmpd=sng` (already-exported control,
  not an F30 lock)
- F30-3 insetPen: `w=76200` `algn=in`
- F30-4 thinThin: `w=76200` `cmpd=dbl`
- F30-5 thinThick: `w=76200` `cmpd=thinThick`
- F30-6 thickThin: `w=76200` `cmpd=thickThin`
- F30-7 thickBetweenThin: `w=76200` `cmpd=tri`
- F30-8 arrowsTriangle: `w=76200` `headEnd type=triangle` and
  `tailEnd type=triangle` (omitted `w`/`len`)
- F30-9 arrowsMixed: `headEnd type=arrow w=lg len=lg` and
  `tailEnd type=stealth w=sm len=sm`
- F30-10 arrowsOvalDiamond: `headEnd type=oval w=med len=med` and
  `tailEnd type=diamond w=lg len=lg`

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- a single standard `lineChart` series whose `a:ln` uses official
  `ST_CompoundLine` tokens `dbl`, `thinThick`, `thickThin`, or `tri`
- official `algn=in`
- official `headEnd` / `tailEnd` with types `triangle`, `arrow`,
  `stealth`, `oval`, or `diamond`, and optional `w`/`len` in
  `sm` / `med` / `lg`

Close-enough export keeps the existing single centered stroke of the
resolved explicit pen. Compound stripe ratios, inset path offsets,
arrowhead geometry, shaft shortening, and sm/med/lg size contracts are
not uniquely locked from this official one-width horizontal matrix and
are not invented here. This is not a unique official-matrix lock of
compound, inset, or arrow geometry.

Residuals stay on the review page. Do not auto-mark pass. Library
rerender sizes are 424x265; overlap residuals against official
Chart.Export gold (review-page RGBA overlap): controlDefault 13119,
controlThick 15724, insetPen 16200, thinThin 14917, thinThick 14674,
thickThin 14458, thickBetweenThin 15102, arrowsTriangle 15418,
arrowsMixed 16228, arrowsOvalDiamond 16447 (thickThin is the only
same-canvas pair at 424x265).

## Still NI

- scatter, area, combo, radar, or 3D hosts plus inset, compound, or
  arrow encodings
- unknown `cmpd` / `algn` / line-end tokens
- features Office itself cannot Chart.Export
- terminal-gap, lim4, retrace, and legend residuals already listed as
  review leftovers
