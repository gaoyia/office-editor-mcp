# F15 close-enough: log axis auto bounds and below-min clip

This is a documented general rule, not a unique official-matrix lock.
The recorded zero-pixel standard is not met and is not rewritten as if it were.

## What now exports

Official F15 Chart.Export gold already existed for all six folder cases. Two
already-exportable explicit log10 1–1000 controls (clustered column F15-2 and
standard line F15-4) were regenerated only; residual pixels are left on the
review page.

Four cases previously left 仍NI even though official gold exists now render:

- F15-1 `col-log10-auto` (`sheet:Sheet1/chart:2`): values 1/10/100/1000, XML
  has only `logBase=10`. Official runtime min=1 max=1000, same four decade
  grids as the explicit twin (425×266 vs 424×266 title/width residuals).
- F15-3 `col-log2-auto` (`sheet:Sheet1/chart:4`): values 4/8/16/32, XML has
  only `logBase=2`. Official runtime min=1 max=32 (six ticks 1,2,4,8,16,32),
  even though the data minimum is 4. This is the discriminating auto-min case.
- F15-5 `col-log10-auto-offgrid` (`sheet:Sheet1/chart:6`): values 3/30/300,
  XML has only `logBase=10`. Official runtime min=1 max=1000.
- F15-6 `col-log10-clip` (`sheet:Sheet1/chart:7`): explicit min=10 max=1000,
  values 1/10/100/1000. Official A=1 is a 1px-tall sliver; B=10 is zero-height
  at the min. Official 424×266.

## General rule

- When `logBase` is in [2, 1000], `majorUnit`/`minorUnit` are omitted, format
  is General, orientation is minMax, crosses is autoZero, and the kind is
  clustered column or standard line:
  - If both min and max are omitted and every finite value is >= 1: auto min
    is 1, auto max is the integer power of the base at or above the data max.
    Degenerate (computed max <= 1) and values < 1 or non-positive stay NI.
  - If both min and max are explicit >= 1 integer powers of the base: unchanged
    from the already-implemented F15 subset. A finite value above 0 but below
    the min is no longer a reject: columns paint a 1px-tall sliver at the log
    min axis (close-enough to official F15-6 “细条”). Values above the max, and
    non-positive values, stay NI.
- Area / bar / scatter / pie log, visible value labels, custom major/minor,
  reversed/deleted log axes, and a single explicit end stay NI.

Official F15-6 sliver sits about two pixels below the other bars’ bottoms; the
renderer uses a 1px sliver just above the axis rather than a uniquely locked
offset. Residuals stay on the review page. Do not auto-mark pass.
