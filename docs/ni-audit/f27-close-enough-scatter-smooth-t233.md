# F27 close-enough: official-like scatter smooth styles produce an image

Official F27 Chart.Export gold already existed for all five charts in
`C:\Users\AMD\AppData\Local\Temp\t172-f27-scatter-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=5).
Source SHA-256
`c32b9bc3f2ad7ba4d3365cb3054fff3e33e33cba78df890d7fbe0c24cc8ba7a4`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

COM ChartType names are not the saved `c:scatterStyle` values. Official
Excel 16 writes:

- F27-1 (xlXYScatterLinesNoMarkers, ChartType 75): `scatterStyle=lineMarker`,
  marker `symbol=none`, `smooth=0`. This already exported on the locked
  line-without-markers path.
- F27-2 (xlXYScatterSmoothNoMarkers, ChartType 73): `scatterStyle=smoothMarker`,
  marker `symbol=none`, series `c:smooth=1`.
- F27-3 (xlXYScatterSmooth, ChartType 72): `scatterStyle=smoothMarker`,
  marker omitted, series `c:smooth=1`.
- F27-4 (xlXYScatter, ChartType -4169): `scatterStyle=lineMarker`, series
  `a:ln`/`a:noFill`, `smooth=0`. This already exported on the locked
  markers-only path.
- F27-5 (xlXYScatterLines, ChartType 74): `scatterStyle=lineMarker`,
  marker omitted, `smooth=0`. This already exported on the locked
  lineMarker path.

Official gold never writes `scatterStyle=line`, `scatterStyle=none`, or
`scatterStyle=smooth`. The leftover cannot-export work in this pass is
the official-like smooth encodings that Office can Chart.Export.

## What now exports

These official-like encodings must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- `scatterStyle=smoothMarker` (or `smooth`) with series `c:smooth=1` and
  marker `symbol=none` (F27-2)
- `scatterStyle=smoothMarker` (or `smooth`) with series `c:smooth=1` and
  omitted marker, using the already-locked auto marker cycle (F27-3)
- the same styles when the series `c:smooth` flag is omitted: the
  wrapper style supplies the default
- an explicit series `c:smooth=0` stays a polyline even under those
  wrappers

The connecting stroke is a close-enough interpolating cubic (Catmull-Rom
converted to cubic Bezier, tension 0.5, duplicated endpoints, sampled
into the existing polyline stroke). This is not a unique official-matrix
lock and is not the F28 reusable interpolation / endpoint / blank
contract. Already-locked marker / lineMarker / symbol=none / line
`noFill` paths are unchanged.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- explicit `scatterStyle=none`
- explicit `scatterStyle=line` (the unsaved COM name; official "line"
  charts are saved as `lineMarker` + `symbol=none`)
- `lineChart` / `areaChart` series or wrapper `smooth=1` (F28)
- dashed scatter pens, `varyColors`, `dispBlanksAs` zero/span, mixed
  midCat/between valAx, bubble charts, and other previously locked NI
  scatter inputs
- features Office itself cannot Chart.Export
