# F11 close-enough point-label visibility (t219)

This is a documented general rule, not a unique official-matrix lock.
Residual pixels on the official F11 Chart.Export cases stay on the review
page and are not auto-marked pass. The recorded zero-pixel standard is not
claimed as met.

## What now exports

On ordinary clustered column, clustered bar, and standard line series
labels that already use the locked default General / outEnd or `r` path:

- `<c:dLbl>` with `idx` and `delete=true` (or omitted `val`, or `1`/`true`)
  still hides that point's label and keeps the data mark. Known inert Excel
  editing extensions (`{CE6537A1-...}` empty, `{C3380CC4-...}` uniqueId)
  remain accepted and have no paint semantics.
- `delete=false` (`0` / `false` / `off`) keeps that point's label. This is
  the OOXML meaning of the flag, not a fitted offset.
- Point `showVal=false` hides that label, using the same
  `deleted_label_points` paint skip as `delete=true`.
- Point `showVal=true`, when the series labels are already visible, is a
  no-op keep.

Consistent combinations are accepted (`delete=true` plus `showVal=false`,
or `delete=false` plus `showVal=true`). Conflicting combinations
(`delete=true` plus `showVal=true`, or `delete=false` plus `showVal=false`)
stay rejected. An `idx`-only `dLbl` with neither flag still stays NI.

## What still errors, and why

- Point `showVal=true` on a series whose labels are hidden stays NI: the
  library cannot yet paint a one-point show, and must not export a chart
  that silently drops those labels.
- Visible per-point position (`dLblPos`), custom text (`tx`), and style
  (`txPr`, visible `spPr`) stay NI. There is no official F11 Chart.Export
  gold for those cases; Excel was not launched to invent any.
- Unknown `extLst` children, wrapper-level `dLbl`, and other chart kinds
  stay NI.

Official F11 gold in `t169-f11-delete-label-locked-20261121` covers only
the already-exportable control and `idx+delete=true` cases (column, line,
and bar). Those were regenerated only; their axis/plot/size residuals were
not chased as a unique lock.
