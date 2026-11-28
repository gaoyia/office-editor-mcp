# F10 close-enough General decimals and bar inEnd (t218)

Office can `Chart.Export` horizontal clustered-bar value labels with General
decimals and with `dLblPos=inEnd`. The library used to reject both with
`ChartImageUnsupported` (decimals because `value_label_text` only accepted
integers; bar inEnd because only columns were in the position match). This
pass makes those Office-exportable cases produce an image.

These rules are documented close-enough renderers where a unique official
lock is not available. Residual pixels and canvas-size differences stay on
the review page. Do not auto-mark pass. The recorded zero-pixel standard is
not met and is not rewritten as if it were.

The already-exportable integer outEnd cases, the no-label control, and the
locked integer General subset (`0`, `-42`, no scientific notation) are
unchanged. Axis / title / plot-layout residuals already visible on the
control card were not chased as a unique lock.

## What now exports

- General decimal showVal (official F10 gold: 4.5, 6.3, 9.1999… rendered as
  9.2, plus the second series of integers 7/5/8), on the existing outEnd
  placement. The same formatter also lets line/column General decimals
  export instead of rejecting.
- Horizontal clustered-bar `dLblPos=inEnd` with complete finite General
  integers (official gold: 4/6/9 and 7/5/8). Placement is inward from the
  bar end with the same 6pt (3pt gap + 3pt inset) pad as the locked outEnd
  path, relative to the library's own bar geometry.

## Close-enough rules

- **General decimals.** The locked integer path (`fract() == 0`, `"0"` for
  zero, `{value:.0}` otherwise, reject non-finite and `|value| >= 1e11`) is
  unchanged. Other finite values format with 10 fraction digits and trailing
  zeros trimmed. Official 9.1999999999999993 becomes `"9.2"`. This is not a
  unique official precision, rounding, or scientific-notation lock.
- **Bar inEnd.** Positive values place the box to the left of the right end
  (`end - pad - width`); negative values to the right of the left end
  (`end + pad`). Zero or short bars that cannot hold the box, and labels
  that would overflow the canvas, still return `ChartImageUnsupported`
  rather than silently clipping, matching the column inEnd gate. Custom
  `numFmt` other than General, `txPr`, visible style, `bestFit` and other
  unlabelled positions stay rejected.

## Still rejected in this folder

- Custom number formats (`numFmt` other than General).
- Other positions (`bestFit`, `inBase`, `ctr`, …) — no official gold in the
  t169-f10 folder for those.
- Missing values / errors on labels, per-point visible overrides, and
  area/scatter/pie labels stay with later INDEX/TODO items (F11+ or still
  NI here). No official F10 Chart.Export gold was present for them, and
  Excel was not launched to invent new gold.
- Features Office itself cannot `Chart.Export` are skipped.

## Official gold (read-only)

Page-cited source `f10-bar-source.xlsx` SHA-256
`867c1cf75b8ac9e6438b3ca0ed9f412a2492ad84ad6c51c1692ca6104283291c`
(`C:\Users\AMD\AppData\Local\Temp\t169-f10-bar-labels-locked-20261121`).
Official Chart.Export SHAs match the sibling t169-f10-bar-labels-20261121
office-raw folder. Do not mutate these folders. Library rerenders use a
new temp only.
