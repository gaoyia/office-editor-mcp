# t196u automatic valAx major-grid density (approximation)

Authorized by the user (t196u, 2026-11-27, Asia/Shanghai): after the t194
auto-scale approximation already matched official min/max/majorUnit on all 41
auto cases of the t191 std180 gold, some horizontal major gridlines still looked
too dense. This note records a further general loosening of the interval
preference. It is still an approximation, not a unique Office lock for every
chart height or default title typography. Do not auto-pass the review page.

## What was too dense

No library case was denser than official (t194 already scored 41/41 auto).
The remaining tightness was official (and therefore then-current library)
9-interval leftovers, plus the unique official 10-interval signed +/-8 case
and 8-interval official scales.

Official 9-interval auto leftovers (10 tick labels): replica-t171, auto-max4,
auto-max8, auto-max16, auto-max39, auto-max40, auto-max42, auto-max80.

Official 8-interval auto cases kept: auto-max7, auto-max14 (protected),
auto-max15, auto-max35. Narrowing the preferred window to exclude 8 would
regress protected official max14 from 0-16/2 to the previously rejected
0-15/5, so those 8-interval leftovers stay.

Official 10-interval auto signed-m8-p8 (-10..10/2) is kept by the existing
both-ends 0.95-padding exception. Explicit exp-0-10-1 (0-10/1) is unchanged.

## Rule change (pick_auto_major / AutoCand::preferred in axis.rs)

Keep 1-2-5 candidates, 0.95 extra-major padding, and the interval budget cap.
Preferred set is now intervals 4..=8, or 10 only when padding fired at both
ends. Among that set, still minimize padded range, then step. Fallback is
unchanged: drop 10-interval candidates that were not both-ends padded, then
min-range min-step.

The only change from t194 is the preferred window 5..=9 -> 4..=8 (4 was
already used in fallback for official max18/19 and +/-9; including it lets
max16 loosen to official-unlike 0-20/5 instead of staying at 9-interval
0-18/2).

## Cases that change (all loosened; eight protected official alignments kept)

- auto-max4: official 0-4.5/0.5 -> 0-5/1
- auto-max8: official 0-9/1 -> 0-10/2
- auto-max16: official 0-18/2 -> 0-20/5
- auto-max39 / auto-max40 / auto-max42 / replica-t171: official 0-45/5 -> 0-50/10
- auto-max80: official 0-90/10 -> 0-100/20

Protected (unchanged, still official): auto-max9 / replica-t166 0-10/2;
auto-max14 0-16/2; auto-max18 / auto-max19 0-20/5; auto-max45 0-50/10;
auto-max90 0-100/20; signed-m9-p9 -10..10/5. Explicit XML scales unchanged.

Oracle rows height-240 / height-336 / height-480 (values 1,4,2 at budget 10)
and headroom samples 4 / 3.9 follow the same general rule as auto-max4
(0-5/1). old-95 stays 0-100/20.

Historical t171 large-title official 0-50/10 is still not a typography lock;
the same data with sz=1100 is now also 0-50/10 because of this density
preference, not because default title 10 vs 18 was re-guessed.
