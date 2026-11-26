# t194u automatic valAx scale approximation (not a unique lock)

Authorized by the user (t194u, 2026-11-27, Asia/Shanghai): a best-effort general
auto-scale rule after official gold showed no predeclared model uniquely locked
41/41, with the instruction to move visible mismatches closer rather than invent
per-case constants. This is an approximation. Do not auto-pass the review page.
It is not a unique Office lock for every chart height or title typography.

## Rule (crates/ooxp/src/xlsx/chart_paint/axis.rs)

Keep the existing 1-2-5 candidates, 0.95 extra-major padding, and interval
budget cap. When XML does not specify majorUnit:

1. Prefer candidates with 5..=9 intervals, or 10 intervals only when the 0.95
   padding fired at both ends (official signed +/-8 is -10..10 step 2; signed
   +/-9 is -10..10 step 5). Among those, minimize padded range, then step.
2. If that set is empty, drop 10-interval candidates that were not both-ends
   padded, and again minimize padded range, then step (official 0-20/5 for
   data max 18/19; official -10..10/5 for +/-9).
3. Last resort: the previous min-range-then-min-step search over every
   budget-valid candidate.

Explicit XML min/max/majorUnit is unchanged. Major gridlines follow the chosen
major unit automatically. Gridline stroke color (official palette-quantized
gray vs library near-black) is out of scope.

## Official gold (reused, not re-exported)

C:\Users\AMD\AppData\Local\Temp\t191-valax-autoscale-std180-20261126
Source SHA 9b339d300382dd8914e734b96e2211a4fc52f07d7b75a865750460dd1b9cb35f.
Uniform 288x180 pt, titles explicit sz=1100, 44/44 CLEAN_PUBLIC_OFFICE_EXPORT.

Auto cases on that matrix: previous library 33/41, this rule 41/41, 0
regressions of already-aligned cases. The eight user-visible misses (max 9 /
replica-t166, max 14, max 18/19, max 45, max 90, signed +/-9) all move to the
official min/max/major. The three explicit-scale controls remain honored.

old-95 in chart_axis_oracle.json and axis_headroom_matches_excel_samples
were updated from 0-100/10 to 0-100/20 under the same general rule as official
auto-max90. Data 95 was not itself in the t191 matrix.

Historical t171 official 0-50/10 (default large title) is still not this rule's
job: the same data with sz=1100 is official 0-45/5 and stays matched. Default
title typography 10 vs 18 was not re-guessed.

Remaining visual residuals on the review page include library 424x265 vs
official 425x266 (at most 1 px gridline Y when the numbers already match) and
gridline stroke darkness. Those are not auto-passed.

## Tests (fresh, this turn)

- cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1: 2347 passed / 190 targets / 0 failed / 0 ignored (one new discriminant unit test).
- python -m unittest discover -s manual-check/lib -p test_controlled_chart_*.py -v: 15 tests, OK.
- cargo build --offline --locked -p ooxp-cli: OK.

Library re-render of the existing 44 gold cases at dpi 106 (new output under
C:/Users/AMD/AppData/Local/Temp/t194-library-rerender-20261127; official gold
not mutated): exactly the eight previously mismatched auto cases changed.
The other 36 PNGs (33 already-aligned auto cases plus the three explicit-scale
controls) are byte-identical to the previous library baseline. Tick labels and
major-gridline counts on those eight now match official min/max/majorUnit;
pairwise |dy| on matched mid-gridlines is 0 or 1 px (existing 424x265 vs
425x266 canvas). No new official Excel session.

## Later density loosening (t196u)

See valax-autoscale-t196-density.md. The preferred interval window was changed from 5..=9 to 4..=8; 10 both-ends-padded intervals remain allowed. Official 9-interval leftovers were loosened; the eight t194 official alignments were kept.
