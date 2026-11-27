# F38 close-enough: official-like pie series/dPt linear gradients produce an image

Official F38 Chart.Export gold already existed for all eleven charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f38-pie-fill-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=11,
chartExportReturned true). Source `f38-pie-fill-source.xlsx` SHA-256
`6c0d60d86540d48cd7ae57350ec180dfb28269fc199748fd2edbd6f67a8916a0`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office standard `pieChart` matrix. Categories
are implied A–D, values 10,20,30,40, varyColors=1, firstSliceAng=0,
legend r, chart-level dLbls with every show flag off and
`showLeaderLines=1` (the existing accepted hidden-label encoding; the
old showLeaderLines source gate is not reopened). Official encodings
have no `pattFill` or `blipFill`.

Official encodings:

- F38-1 pieCtrl: no series `spPr` (accent varyColors control; already-exported)
- F38-2 pieSerSolid: series solid sRGB FF0000 (already-exported)
- F38-3 pieSerAlpha50: series solid FF0000 + `a:alpha` 50000 (already-exported)
- F38-4 piePtAlpha50: dPt idx=0 solid FF0000 + `a:alpha` 50000 (already-exported)
- F38-5 pieSerNoFill: series `a:noFill` plus inert `a14:hiddenFill` (already-exported)
- F38-6 piePtNoFill: dPt idx=0 `a:noFill` plus hiddenFill (already-exported)
- F38-7 pieSerLinH: series `<a:gradFill flip="none" rotWithShape="1">` with
  stops 0=FF0000 / 100000=0000FF and `<a:lin ang="5400000" scaled="1"/>`
- F38-8 pieSerLinV: same stops, `<a:lin ang="0" scaled="1"/>`
- F38-9 piePtSolid: dPt idx=0 solid 00FF00 (already-exported)
- F38-10 piePtLinH: dPt idx=0, same LinH gradient as pieSerLinH
- F38-11 pieMix: series solid FF0000, dPt idx=1 solid 0000FF, dPt idx=2
  solid 00FF00 + alpha 50000 (already-exported; no gradient)

Official Chart.Export of pieSerLinH is one linear field over the pie
(red toward the top, blue toward the bottom). pieSerLinV is one field
with red toward the left and blue toward the right. That is not a
per-wedge rotated gradient, and it is not the F06 chart-frame rectangle
contract.

## What now exports

An official-like standard single-wrapper `pieChart` series or dPt
linear `a:gradFill` with `a:lin` angle 0 or 5_400_000, `flip` omitted or
`none`, `rotWithShape` omitted or 1, `scaled` omitted or 1, empty
`tileRect`, no `a:path`, and two (or close-enough straight-alpha) sRGB
stops must produce an image. NotImplemented / ChartImageUnsupported is
not acceptable for those Office-exportable encodings. dPt still wins
over the series fill and over varyColors.

The fill is sampled with the existing `gradient_t` / stop-table /
`office_grad_weight` path at pixel centers inside each wedge, using the
existing pie plot square (the diameter box after title/legend reserve)
as the gradient rectangle. `rotWithShape=1` / `scaled=1` in this matrix
are accepted as that pie-square scope; they are not interpreted as a
per-slice rotated frame. Legend keys reuse `fill_linear_rect` on the
swatch's own rectangle as a close-enough stand-in.

Already-exportable solid, straight-alpha, noFill, mix, and control
cases are not re-locked. Their residual pixels stay on the review page
and are not chased as a unique lock.

## Leftover NI

These stay NotImplemented until a later official-matrix lock (or until
Office itself cannot Chart.Export them, in which case they are skipped):

- series/dPt `pattFill` and `blipFill` (no official F38 gold)
- path shade, flip other than none, `rotWithShape=0`, `scaled=0`, and
  linear angles other than 0° / 90°
- doughnut / ofPie / pie3D sector gradients
- scheme/system stop colors, `alphaMod` / `alphaOff`, and multiple fills
- non-default pie legend layout (F39)
- features Office itself cannot Chart.Export

The old showLeaderLines source gate is not reopened.

## Close-enough, not a unique official lock

Sampling official-like 0°/90° linear gradients in the pie plot square
is an authorized close-enough stroke so Office-exportable official
encodings produce an image. It is not a unique official-matrix lock of
every sector fill, and it is not the F06 rectangle gradient contract
applied to pie sectors. Residual title, canvas, edge, and legend-key
pixels stay on the review page.
