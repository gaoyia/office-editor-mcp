# F22 close-enough: stacked omitted min/max reuses scale_values

Official F22 Chart.Export gold already existed for all eight folder cases.
Seven already-exportable cases (explicit stacked / percentStacked column and
bar, mixed-sign stacked, omitted cache points, and the clustered control)
were regenerated only. Residual pixels are left on the review page. The
recorded zero-pixel standard is not met and is not rewritten as if it were.

## What now exports

F22-6 `col-stacked-auto` (`sheet:Sheet1/chart:7`) / chart6.xml was left 仍NI
even though official gold exists: three-series clustered-column stacked,
same values as F22-1 (category totals 50, 60, 60, 70), overlap=100,
invertIfNegative=0, linear autoZero minMax between, but both valAx min/max
and majorUnit are omitted. Official 424×266.

Official auto draws 0–80 with major 10 (eight intervals). That is the
already-locked `scale_values` 1-2-5 / 0.95-pad rule applied to the stacked
extents, not to raw point values (max 40 would clip the stack). The same
general reuse is allowed on clustered bar stacked; official gold in this
folder is the column case only. Do not treat bar as a uniquely locked case.
percentStacked auto and line/area stacked auto stay NI (no official F22 gold).

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- Other overlap (not 100), invertIfNegative=1, and value labels on stacked
- log, orientation=maxMin, delete, or custom crosses on stacked
- secondary valAx plus stacked
- percentStacked mixed signs, missing values, or omitted min/max
- line/area stacked omitted min/max (F23)
- Features Office itself cannot Chart.Export were skipped
