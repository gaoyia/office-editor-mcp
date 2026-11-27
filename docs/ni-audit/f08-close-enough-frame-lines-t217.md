# F08 close-enough frame lines, rounded no-line, and outerShdw (t217)

Office can `Chart.Export` visible chart/plot `a:ln`, rounded no-line frames,
and a non-empty `a:effectLst`/`a:outerShdw`. The library used to reject those
with `ChartImageUnsupported`. This pass makes them produce an image.

These rules are documented close-enough renderers. They are **not** unique
official-matrix locks. Residual pixels and canvas-size differences stay on
the review page. Do not auto-mark pass. The recorded zero-pixel standard is
not met and is not rewritten as if it were.

## What now exports

- Visible solid chart-area `a:ln` (official gold: red 0.75pt, 1pt, 2pt).
- Visible solid plot-area `a:ln` (official gold: red 1pt, with the default
  1px gray chart border still painted).
- Rounded + `a:ln`/`a:noFill` (no official F08 gold in the t169 folder;
  still exports).
- Visible `a:outerShdw` (official gold: `blurRad=63500`, `dist=53881`,
  `dir=2700000`, opaque `scrgbClr` black).
- A bare `<a:ln w="..."/>` with no fill (close-enough opaque black).

The locked rectangular no-line path (`roundedCorners=0` + `a:ln`/`a:noFill`
+ hiddenLine) and the default 1px gray `stroke_border` (no `a:ln`) are
unchanged.

## Close-enough rules

- **Width.** Convert `a:ln/@w` with `emu_to_px_f` and paint a center-aligned
  solid stroke on the rectangle, inset 0.5 device pixels. The series-pen
  snap (`excel_chart_line_width_px`, almost-integer then round else ceil)
  is **not** used: at 106 dpi that snap would make official 0.75pt / 1pt
  (palette 1px) two pixels. Omitted `w` uses 12700 EMU (1pt). Official
  Chart.Export sometimes grows the bitmap (424×266 / 426×267 vs the
  requested 424×265). This renderer keeps the requested size and clips
  overflow. Size and AA residuals stay on the page. Official 0.75pt and
  1pt palette exports are not uniquely distinguishable; no fudge is
  invented to force a unique width lock.
- **Color.** `solidFill` goes through `resolve_frame_solid`. Omitted or
  unresolved fill is opaque black so the line still exports. Non-solid
  line fills (gradient/pattern/blip) also fall back to black. `a:noFill`
  stays invisible.
- **Rounded no-line.** Stop rejecting `rounded && a:ln`. After paint, if
  there is no default gray border, `clip_rounded_silhouette()` applies the
  already-locked `CORNER_R = 14` / `rad = CORNER_R - 0.5` outside-corner
  coverage from `stroke_quarter`, without painting that gray stroke.
- **outerShdw.** Parse `blurRad` / `dist` / `dir` / color. Missing values
  are zero blur/dist/dir and opaque black. `dir` is DrawingML 1/60000
  degree; 0 is +x, and this y-down buffer uses `dx = dist*cos(theta)`,
  `dy = dist*sin(theta)` so official 45° (2,700,000) is down-right. The
  canvas is expanded by `ceil(blur ± offset)` on each side (a general
  AABB, not a unique +15 lock: official 439×280 vs 424×265 is +15;
  `ceil(7.36+6.24) = 14`). Shadow is a silhouette splat plus two box
  blurs standing in for a Gaussian, composited under the painted chart.
  Unspecified color alpha is treated as opaque (the official scrgb has
  no alpha child); palette gold is not a unique alpha lock. Other
  `effectLst` children are ignored so the chart still exports.
- Unknown line attributes and join/dash children are ignored; dashes
  currently paint as solid.

## Still rejected (not this batch)

Empty `gradFill` and the remaining unmeasured solid-transform / `sysClr` /
scheme-without-palette fill cases stay errors (F07+). Features Office
itself cannot `Chart.Export` are skipped.

## Official gold (read-only)

`C:\Users\AMD\AppData\Local\Temp\t169-f08-lines-20261121`. Source
`f08-lines-source.xlsx` SHA-256
`0270a402dfb23fcd8d7837c22f08090992e71f035e45acc659167f6340fa28ef`.
Do not mutate this folder. Library rerenders use a new temp only.
