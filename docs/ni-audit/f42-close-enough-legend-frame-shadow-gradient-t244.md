# F42 close-enough legend frame outerShdw and linear gradFill

Official Excel 16 / DPI 106 Chart.Export gold at `t174-f42-legend-frame-20261121` already exports seven clustered-column right legends (ctrl, solid, alpha, nofill, line, shadow, gradient). All seven certificates are `CLEAN_PUBLIC_OFFICE_EXPORT`. The first five cases already exported from the library; `shadow` and `gradient` were still `NotImplemented`.

## Official encodings

- `shadow`: legend `spPr` has only `a:effectLst/a:outerShdw` with `blurRad="50800"`, `dist="71842"`, `dir="2700013"`, `rotWithShape="0"`, and `a:srgbClr val="000000"` plus `a:alpha val="60000"`. No legend fill or line.
- `gradient`: legend `spPr` has only `a:gradFill flip="none" rotWithShape="1"` with two sRGB stops (`FF0000` at 0, `FFFFFF` at 100000), `a:lin ang="5400000" scaled="1"`, and empty `a:tileRect`.

Office can Chart.Export both, so the library must produce an image. Close-enough is authorized. This is not a unique official-matrix lock of every effect, theme fill, or residual pixel.

## What this change does

- Reuses the existing legend `Layout.frame` rectangle. No COM origin, no invented Office constants, no threshold or coordinate fudge.
- Linear `gradFill` reuses `read_linear_frame_grad` and `Canvas::fill_linear_rect`, the same proven chart/plot/series path, painted into that frame before keys and text.
- `a:outerShdw` reuses `read_outer_shadow_effect` (same EMU, DrawingML `dir`, color, and alpha as chart-area shadows) and paints a box-blurred offset of the legend rectangle only. The export canvas is not grown. Zero dist and blur are a no-op, so a bare `outerShdw` still exports and stays bit-identical to the unstyled control.
- Already-exportable ctrl / solid / alpha / noFill / explicit opaque sRGB line paths are unchanged. `noFill` stays bit-identical to the default frame.

## Residuals and leftover NI

Frame position, padding, and corner/AA residuals on the already-exportable fills and border stay on the review page. They are not chased as a unique lock, and the recorded zero-pixel standard is not rewritten.

Still NI, with no official gold unlocked here:

- other `effectLst` children (`innerShdw`, glow, and any mixed list)
- theme `solidFill` on the legend frame
- complex or widthless lines (`prstDash` other than solid, missing `w`)
- empty `gradFill` without stops

Those remain explicit `ChartImageUnsupported`.
