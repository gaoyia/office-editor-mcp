# F33 close-enough: official-like area noFill, straight alpha, and pattern fills produce an image

Official F33 Chart.Export gold already existed for all five charts in
`C:\Users\AMD\AppData\Local\Temp\t174-f33-area-fill-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT, ChartObjects=5,
chartExportReturned true). Source `f33-area-fill-source.xlsx` SHA-256
`ffe8109fc31b654765fd80796b5cb67e9850446f0dbf5d50a1ab2e45144d0ece`
/ seed `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`.
Excel was not relaunched. Residual pixels stay on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it
were.

Official gold is a default Office standard `areaChart` matrix. Series
values are 20,40,60,40,20,50,30,70, cats A–H, varyColors=0, valAx midCat
0–80 / 20, chart-space fill EAF7F7, series teal 156082. Official
encodings:

- F33-1 areaControl: no series `spPr`, no marker (already-exported midCat
  control, not an F33 lock)
- F33-2 areaMarker: after normalizing uniqueId / axId / title, the XML is
  identical to F33-1. The COM marker was not serialized. This case
  already exports on the existing midCat area path. No default area
  marker is invented from this matrix.
- F33-3 areaNoFill: series `spPr` is `<a:noFill/>` plus an optional
  Office 2010 `a14:hiddenFill` solidFill srgb 156082 ext (URI
  `{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}`). Hidden solid is editing
  memory, not a visible fill.
- F33-4 areaTransparent: series `spPr` solidFill srgb 156082 with
  `<a:alpha val="50000"/>`
- F33-5 areaPattern: series `spPr` `<a:pattFill prst="pct10">` with
  opaque fg 156082 and bg FFFFFF

The historical midCat gate is already unlocked; the leftover
cannot-export cases were the official noFill / straight-alpha / pattern
encodings.

## What now exports

These official-like encodings must produce an image on an official-like
standard clustered single-wrapper non-3D non-secondary midCat area host.
NotImplemented / ChartImageUnsupported is not acceptable for them:

- series `a:noFill`, with or without an inert `a14:hiddenFill` ext. The
  area band is skipped; the remembered hidden solid is not painted.
- series solidFill with a straight `a:alpha` (thousandths of a percent;
  100000 = opaque). Alpha is mapped with the existing pie/frame rule
  `ooxml_alpha_to_u8` and composited over the already-painted plot /
  chart with `fill_polygon_alpha`. This is a close-enough straight-alpha
  composite, not a unique official-matrix lock. `alphaMod` / `alphaOff`
  stay NI.
- series `a:pattFill` with an explicit preset and opaque fg/bg colors.
  The existing 8×8 / request-DPI `PatternFill::color_at` RULE is sampled
  at canvas pixels inside the area polygon only. The area is not filled
  as a frame rectangle, and no per-fixture origin is invented. The
  `pctN` Bayer bitmap is the existing close-enough pattern sample, not a
  unique official-matrix lock. Phase residuals stay on the review page.

areaControl / areaMarker already-exportable residuals stay on the review
page and are not chased as a unique lock.

## Leftover NI

These stay NotImplemented until a later official-matrix lock (or until
Office itself cannot Chart.Export them, in which case they are skipped):

- a visible area marker (`c:marker` symbol other than none)
- stacked / combo / secondary / 3D hosts plus noFill, alpha, or pattern
- `a:gradFill` / `a:blipFill`
- `a:alphaMod` / `a:alphaOff`
- `a14:hiddenFill` without `a:noFill`
- multiple fills on one series `spPr`
- an unknown `pattFill` preset

## Close-enough, not a unique official lock

The noFill skip, straight-alpha composite, and polygon-sampled pattern
tile are authorized close-enough strokes so Office-exportable official
encodings produce an image. They are not presented as a unique
official-matrix lock, and residual pixels are left on the review page.
