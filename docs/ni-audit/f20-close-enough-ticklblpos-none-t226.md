# F20 close-enough: valAx tickLblPos=none keeps the axis line

Official F20 Chart.Export gold already existed for all eight folder cases.
Seven delete=true (or control) cases already exported and were regenerated
only; residual pixels are left on the review page. The recorded zero-pixel
standard is not met and is not rewritten as if it were.

## What now exports

F20-5 `col-mixed-ticklabels-none` (`sheet:Sheet1/chart:6`) / chart5.xml was
left 仍NI even though official gold exists: clustered column, values
-4/-1/3/6, explicit min=-8 max=8 major=2, valAx `tickLblPos=none` and
`delete=0`. Official 425×266. Official also releases the left value-label
gutter and omits tick labels, but keeps the value axis line (about x=16).
That official image differs from official delete=1 by 967 px, so this is
not the F20 delete=true rule.

The historical library export of this official file already returned an
image, but it ignored `tickLblPos=none` and still painted the labels in the
full left gutter. This pass reads `tickLblPos=none` as a distinct flag:
release the value-label gutter and omit tick labels, keep the axis line and
major tick marks. The same general rule is allowed on standard line and
clustered bar in the same linear autoZero minMax subset; official gold in
this folder is the column case only. Do not treat line/bar as uniquely
locked cases.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- Category-axis `delete` and category-axis `tickLblPos=none`
- `tickLblPos=none` plus log, orientation=maxMin, or custom crosses
- Scatter / pie / area `tickLblPos=none` (no official F20 gold)
- Features Office itself cannot Chart.Export were skipped
