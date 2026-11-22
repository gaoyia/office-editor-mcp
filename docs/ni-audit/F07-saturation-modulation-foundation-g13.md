# F07 saturation modulation shared foundation — g13, pending review

## Contract and bounded assessment

The g12 with/without-legend pairs establish legend dependence for those controls, not a unique legend-key width or auto-layout formula. No guessed key formula or geometry compensation is introduced. This batch implements an independent missing color prerequisite.

- DrawingML `a:satMod`: input saturation is modulated by a percentage; 50% halves and 200% doubles. SDK `SaturationModulation` / CT_Percentage: https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.saturationmodulation?view=openxml-3.0.1
- Office reads percent or integer-thousandth percentage spelling, writes the latter: https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/157e5791-9c51-4251-ae90-7dd58ae69778
- This baseline intentionally implements strict signed i32 integer-thousandth spelling only. Percent-sign/fractional lexical forms and unverified transform orders remain unsupported.

At fixed HSL hue and light, RGB channels lie on the ray from the achromatic midpoint `(min+max)/2`. Scaling each channel's displacement by the saturation factor is the same HSL operation without hue conversion singularities. Conversion clips RGB at the end, not saturation before conversion. The latter hypothesis was explicitly contradicted by the frozen signed / 200% native controls. Existing tint/shade/luminance/default codepaths remain unchanged; lumOff retains its old ±100000 bounds after sharing strict numeric parsing.

## Actual implementation

`chart_paint.rs`: shared `frame_sat`, strict signed percentage parser, exact `satMod` and `satMod,alpha` sequences in `resolve_frame_solid` for existing sRGB/theme bases, reused by chart and plot solid fills. No preset/position tables, sample path dependencies, color lookup or per-fixture corrections. No new arbitrary transform pipeline, sysClr, hueMod, alphaMod or unsupported mixtures.

## Native discriminating controls, not print gold

Source SHA256: `a0e75448ee77b1fb9a8349073644b25b6d74ce0954c993ef83c759b849251808`.
32 same-title/canvas/data fixtures: 16 transforms + 16 mathematically derived explicit-RGB hypothesis references. Two chromatic bases, -50/0/50/100/200%, gray, theme, chart/plot and trailing alpha. All are **OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE**, not original untouched samples, CopyPicture Bitmap or printed-page gold. Namespace preflight passed before one owned readonly Excel session.

Early-saturation-clamp references: 9/16 native whole-PNG pairs equal, 7 different. This is a rejected model, not a pass claim. Implemented unclamped modulation: 11/16 target color probes exact and 5 differ by at most 1 channel byte, including alpha readback. Quantization/alpha residuals remain, without fitted rounding corrections. Native425×266 and library424×265 stay at their own dimensions: no stretching/registration or whole-image acceptance. Full common-region residuals are diagnostic only.

16 formerly NI variants now render. The 16 pre-existing explicit-RGB references and 237 historical controls retain whole-PNG bytes. Four scatter controls remain genuinely NI. Seven new Rust tests cover signed factors/over-saturation/gray/identity, lexical structure, theme, straight alpha, chart/plot at96/106/144dpi and unsupported mixed sequences. Full mandatory suite2195 tests/190 targets plus Python15 and actual CLI build must pass before commit.

Lifecycle record: both explicitly owned readonly seed/copy closed with finally Close(false), Workbooks0, hashes and exclusive read/owner lock verified, no repair/Save/Quit/kill/userbook closure. Empty application process/window is observation, not claimed cleanup. Shared binding, native exporter and Bitmap freshness guards are unchanged.

## Evidence and preserved review scope

Desktop unified INDEX links unique generation `20261121-F07-saturation-modulation-g13/F07-saturation-modulation-foundation-待审.html`, its source/native certificates, failed clamp-hypothesis references, new/baseline/historical PNGs, test logs, provenance/ownership audit and guarded INDEX-only rollback. Prior68 artifacts/2413 protected files (2367 in the archive and 46 intentionally retained at their desktop URLs), localStorage URLs/review states, all manifests/rollback and g8a correction remain byte-intact.

49 groups remain **4accepted /5pending /26partial /10NI /1safety /3illegal**. No acceptance, merge or widening of protected criteria. Independent next prerequisite candidates are standalone hue modulation or shared visible-frame stroke; auto-key layout, square terminal gap, lim4 plot, retracing and scatter remain explicit boundaries.

## PR audit preservation

The exact current PR14 body (62450 characters) is archived before body compression in [pr14-body-before-g13-20261121.md](pr14-body-before-g13-20261121.md), UTF8 SHA256 `cb8c0f1cee992b0690fb21305491a9535b0f194f8bdfc926cd57d9f77eb193c6`. Historical audit content stays versioned; a concise PR body can link this archive, the plan and this current implementation note. No comments/outreach or title/base/state change.

## Inventory-label correction (metadata only)

The first g13 page incorrectly called all2413 protected old files archive files. Exact inventory proves2367 old archive files plus46 retained desktop pages; no missing/unexpected file and every old SHA is intact. With g13 added417 files plus its manifest, archive file count is2785 and all-scope protected files2831. Preserve the published g13 page/manifest bytes and review state; an INDEX-linked supplemental JSON records this correction. This does not change renderer/tests/PNG residuals or acceptance.
