# sysClr frame/plot palette policy (2026-11-26)

Status: implemented subset (explicit caller palette). Not auto-marked pass.

## Contract locked from on-disk official Chart.Export (no new Office session)

Source package: `C:\Users\AMD\AppData\Local\Temp\t169-f07-scheme-20261121\source\f07-sys-lum-derived.xlsx`

Certificates: `office-export-sys-lum.log` — six charts, `CLEAN_PUBLIC_OFFICE_EXPORT`, windowDPI=106, Close unchangedCopy/unchangedMaster, lock=False, repair=0.

Discriminating official rasters (`office-raw\`):

| Chart | XML | lastClr | Official Chart.Export fill | lastClr pixels |
| --- | --- | --- | --- | --- |
| chart3 chart-area window | a:sysClr val="window" lastClr="00FF00" | green | white (255,255,255) dominant; mid-top white | green=0 |
| chart2 chart-area windowText | a:sysClr val="windowText" lastClr="FF0000" | red | black (0,0,0) dominant; mid-top black | red=0 |
| chart4 plot-area windowText | a:sysClr val="windowText" lastClr="000000" | black (coincides) | plot interior black | not discriminating |

Excel paints the live system color named by `@val` and ignores `lastClr`. The library must not treat `lastClr` as the current color, and must not query ambient OS colors (non-deterministic across machines).

## Implementation

- New `SystemColorPalette`: caller map of `sysClr/@val` to sRGB. Exact name match; no case folding or aliases.
- `resolve_frame_solid` accepts a bare `sysClr` only when the palette has that name. Children (alpha/tint/component/...) stay NI; they were not in the official matrix.
- Without a palette, or with an unmapped name, the fill stays `ChartImageUnsupported`. Existing `unsupported_frame_semantics_do_not_fall_back_to_white` still covers lastClr red/green.
- Chart area and plot area share the policy through `read_frame_fill`.
- Slide host still does not paint an Excel chart-area frame, even when a palette is supplied.
- Theme `clrScheme` dk1/lt1 still use serialized `lastClr` (different path). The palette is not auto-filled from the theme snapshot.
- Series/pie `color_of_slot` is unchanged and is not this contract. Those paths were not uniquely locked from this official matrix.

Render entry: `render_chart_with_sys_palette`. `render_chart_with_theme` and public `export_chart_image` still pass no palette, so sysClr remains unsupported on the default export path until a caller supplies a palette.

## Tests

- `chart_frame_sysclr_uses_caller_palette_and_ignores_last_clr`
- `chart_and_plot_sysclr_rasters_match_palette_srgb_not_last_clr`
- `theme_clrscheme_sysclr_lastclr_snapshot_is_unchanged`

## Still NI (investigated this batch, not uniquely lockable without new Office or further unique evidence)

- Default omitted title typography 10 vs 18 (Chinese 10 / English 18, confounded across workbooks).
- Theme binding / hydration: serialized accent1 6D2A91 vs live workbook / official pixels 156082 (WorkbookAccent1OleBgr 8544277 = RGB 156082). Must not rewrite the constant.
- F06 transparent-stop interpolation: even opaque multi-stop models in t176-f06-gradient-stops-20261121 did not uniquely match native (segment-existing-kernel still about 418/418 different, mean channel delta about 15). Do not invent straight vs premultiplied or a working space.
- Color-transform working space for comp / inv / gamma / invGamma / gray: no unique on-disk lock in this batch.
- Square terminal-gap / lim4 / 180 deg retrace / scatter: t181-square-terminal-20261121 already concluded the 30-case native matrix does not uniquely establish a safe general terminal cap rule. Do not invent a threshold.

## Mandatory suite

cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1: 2340 passed / 190 targets / 0 failed / 0 ignored (2337 + 3 new). python -m unittest discover -s manual-check/lib -p test_controlled_chart_*.py -v: 15 passed. cargo build --offline --locked -p ooxp-cli: ok.