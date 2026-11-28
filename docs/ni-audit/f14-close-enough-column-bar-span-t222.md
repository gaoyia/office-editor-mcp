# F14 close-enough: column/bar `dispBlanksAs=span`

This is a documented general rule, not a unique official-matrix lock.
The recorded zero-pixel standard is not met and is not rewritten as if it were.

## What now exports

Official F14 Chart.Export gold already existed for unlabeled clustered column span
(`col-span`, `sheet:Sheet1/chart:6`, F14-5: values 4 / omitted idx 1 / 9, explicit
`dispBlanksAs=span`). The previous review page left this 仍NI even though official
gold exists and the library already wrote a PNG, because official has no mid column
and cannot uniquely be distinguished from gap.

The already-exportable area span / area span+#N/A / area zero / area gap / accepted
line-span controls are regenerated only; residual pixels are left on the review page.

## General rule

- Clustered column and clustered bar `dispBlanksAs=span` do not synthesize a mid
  value. A true blank or omitted cache point stays `None` and is skipped (not
  filled as 0; that fill is the `zero` rule from F13).
- Official unlabeled column span therefore looks like gap: no mid column. Document
  that honestly; do not claim a unique “span equals gap” lock.
- `#N/A` is not a blank and is also skipped, not filled as 0.
- Visible value labels on a remaining blank or error point stay NotImplemented.
- Area span (true blanks and `#N/A` as a V-to-baseline, not a 4-to-9 chord) and
  accepted line span stay as already implemented / accepted. Other errors, scatter
  / pie span with blanks, `dispNaAsBlank`, and unresolved sparse domain stay NI.
  Features Office itself cannot Chart.Export are skipped.

## Official gold notes

Official `col-span` is 424×266, palette Chart.Export. There is no official F14
column-gap twin in this folder, so a same-canvas official gap-vs-span pixel count
is not available here. The page records official vs current library residuals
(library 424×265, plus axis/title/font layout). Do not auto-mark pass.
