# F23 close-enough: line/area stacked omitted min/max reuses scale_values

Official F23 Chart.Export gold already existed for all eight folder cases.
Seven already-exportable cases (explicit stacked / percentStacked line and
area, mixed-sign stacked, and the standard-line control) were regenerated
only. Residual pixels are left on the review page. The recorded zero-pixel
standard is not met and is not rewritten as if it were.

## What now exports

F23-7 `line-stacked-auto` (`sheet:Sheet1/chart:8` in the library id scheme,
official ChartObject `sheet:Sheet1/chart:7`) / chart7.xml was left 仍NI even
though official gold exists: three-series lineChart stacked, same all-positive
values as F23-1 (algebraic cumulative ends 50, 60, 60, 70), marker symbol=none,
smooth=0, linear autoZero minMax, crossBetween=between, but both valAx
min/max and majorUnit are omitted. Official 425×266.

Official auto draws 0–80 with major 10 (eight intervals). That is the
already-locked `scale_values` 1-2-5 / 0.95-pad rule applied to algebraic
cumulative extents, not to raw point values (max 40 would clip the
cumulative) and not to the F22 bar pos/neg packing extents. The same
general reuse is allowed on stacked area; official gold in this folder is
the line case only. Do not treat area auto as a uniquely locked case.
percentStacked auto stays NI (no official gold).

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- Visible markers or value labels on stacked lines
- Missing cache points on stacked line/area
- stacked area with crossBetween=between
- log, orientation=maxMin, delete, or custom crosses on stacked
- secondary valAx plus stacked
- percentStacked mixed signs, missing values, or omitted min/max
- Features Office itself cannot Chart.Export were skipped
