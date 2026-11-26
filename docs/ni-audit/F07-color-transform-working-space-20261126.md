# Color-transform working space: comp / inv / gamma / invGamma / gray (2026-11-26)

Status: implemented subset (standalone `a:srgbClr` + one empty transform). Not auto-marked pass.

## Official evidence

New evidence root (does not mutate t203/t204/t205 or the t183 incomplete g29 files):

`C:\Users\AMD\AppData\Local\Temp\t187-frame-color-transforms-20261126`

- Source SHA: `9dc89f48b6172b1eaa80a672614d0d696a44505a6867ac5be68e2e65a65fb89d`
- Shared success-seed SHA: `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`
- 48 chart-area cases: 8 `a:srgbClr` bases times (none + comp + inv + gamma + invGamma + gray)
- Bases: 010101, 0A0A0A, 808080, A0A0A0, C0C0C0, FF0000, 00FF00, C04020
- Competing models were frozen in `logs/predeclared-models.json` before any Office session. Pure gamma 2.2 was a discriminant only, not a default.
- Official launch observed ExcelCount=0 at 2026-11-26 20:57:54 Asia/Shanghai, then one `export-controlled-chart-native.ps1` session.
- Excel PID 53764, start 2026-11-26 20:57:55.4892285+08:00, Excel 16.0, windowDPI=106.
- 48/48 `CLEAN_PUBLIC_OFFICE_EXPORT`. Provenance `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`. Close(false), no Save, no Kill, FilesClean, repairs=0, source SHA unchanged.
- Empty owned leftover was Quit via GetActiveObject after Workbooks=0; process exited 2026-11-26 20:58:27.1292316+08:00. Post ExcelCount=0.
- Official rasters are 425x266; library remains 424x265. That size residual was not compensated.
- Probe: official PNG pixel (5,5), same chart-area probe as the t200 frame-fill matrices.

## Unique locks (exact 8/8, maxAbsErr=0)

| Transform | Locked contract | Rejected by official pixels |
| --- | --- | --- |
| none | identity of the stored sRGB byte | - |
| inv | IEC 61966-2-1 linear invert then encode: `linear_to_srgb(1 - srgb_to_linear(c))` | encoded `255-c` (max err 120); pure 2.2 (max err 3) |
| gamma | IEC encode treating the stored byte as linear 0..1: `linear_to_srgb(c/255)` | identity (max err 73); decode (max err 133); pure 2.2 (max err 8) |
| invGamma | IEC decode scaled back to a byte: `round(srgb_to_linear(c)*255)` | identity (max err 73); encode (max err 133); pure 2.2 (max err 3) |
| gray | Rec.709 / sRGB luma on encoded bytes: `Y=round(0.2126 R + 0.7152 G + 0.0722 B)` | encoded Rec.601 (max err 32); linear-IEC Rec.709 (max err 73); 54/183/19 >> 8 (max err 1 on FF0000 and C04020, so not this integer shift) |
| comp | encoded mix-to-gray complement `(max+min)-channel` | encoded `255-c` and linear IEC invert (`inv`). Mid-tone grays stay put; FF0000 becomes 00FFFF (ECMA example); C04020 becomes 20A0C0, whose encoded average with the input is gray |

`inv` and `comp` are different operations. `comp` is the encoded HSL-style complement that mixes to gray; `inv` is the linear IEC invert.

## Implementation

- New helpers in `crates/ooxp/src/xlsx/chart_paint/color.rs`: `frame_comp`, `frame_inv`, `frame_gamma`, `frame_inv_gamma`, `frame_gray`.
- `resolve_frame_solid` accepts a bare `srgbClr` plus exactly one empty `comp` / `inv` / `gamma` / `invGamma` / `gray` child (xmlns-only attributes allowed; any `val`, nested children, or chains stay NI).
- `schemeClr`, `sysClr`, `prstClr`, `hslClr`, and `scrgbClr` plus these transforms stay unsupported. Transforms on a resolved sysClr were not in this official matrix.
- Slide host still does not paint an Excel chart-area frame.

## Tests

- `color_transform_working_space_matches_official_t187_probe_rgb`
- `color_transform_working_space_resolve_and_gates`
- `color_transform_working_space_chart_frame_matches_official_rgb_reference`

## Still NI (not uniquely locked here)

- Default title typography 10 vs 18 (do not invent a constant).
- Theme binding / hydration: serialized accent1 6D2A91 vs live 156082 (do not rewrite the file constant).
- F06 transparent-stop interpolation.
- Square terminal-gap / lim4 / 180 deg retrace / scatter.
- Official 425x266 vs library 424x265, and any +/-1 RGB translucent residuals. Not used as a library change.
- sysClr transforms, unmapped `@val` names, default `export_chart_image` without a caller palette, and series/pie `color_of_slot`. This matrix was srgbClr-only.
- Chains of these transforms with alpha or with each other.
