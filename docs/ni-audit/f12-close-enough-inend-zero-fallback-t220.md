# F12 close-enough inEnd zero fallback (t220)

This is a documented general rule, not a unique official-matrix lock.
Residual pixels on the official F12 Chart.Export cases stay on the review
page and are not auto-marked pass. The recorded zero-pixel standard is not
claimed as met.

## What now exports

Ordinary clustered-column (and the same rule on clustered-bar) series that
use default General labels and `dLblPos=inEnd`:

- Non-zero values whose label box still fits inside the mark keep the
  already-locked inEnd rule: 6pt pad (3pt mark-to-box gap + 3pt text
  inset). Positive labels sit inward from the mark end toward the axis;
  negative labels sit inward the other way.
- A value of `0` has no interior, so Office Chart.Export places that
  label on the outside of the axis. The library uses the already-locked
  OutsideEnd geometry for a non-negative value (column: `y - h - pad`;
  bar: `end + pad` on the unreversed axis). This is that same 6pt pad,
  not a newly invented offset and not a unique official-probe lock.
- Official F12 gold includes a 0/1/2 inEnd chart and a -4/0/9 mixed
  inEnd chart. Both now produce an image. Already-exportable non-zero
  inEnd and outEnd controls were regenerated only.

## What still errors, and why

- Short or narrow marks whose inEnd box would not fit still reject
  rather than auto-fit, clip, or stretch. There is no uniquely locked
  short-column auto-fit contract in this pass; official F12 1 and 2
  values are expected to fit under the existing inEnd rule.
- Missing/error labels, other positions (`bestFit`, `inBase`, `ctr`,
  …), visible custom style/`txPr`, and per-point position/style remain
  NI. Custom `numFmt` stays rejected.
- StandardLine `inEnd` stays NI.
- Features Office itself cannot Chart.Export were skipped. Excel was
  not launched; official F12 gold already existed and was left unmutated.

Rules stay on this page as close-enough. Do not rewrite them as a
zero-pixel identity with official Chart.Export.
