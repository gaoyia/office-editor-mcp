# F06 transparent-stop interpolation (t189 official matrix)

Date: 2026-11-26 (Asia/Shanghai)

## Question

Which interpolation model (straight vs premultiplied, encoded vs IEC vs pure 2.2, skip-zero-alpha, ignore-alpha) uniquely matches official Chart.Export when gradient stops carry `a:alpha`?

Opaque 3+ stop interpolation was re-checked on the existing t176 official rasters (not mutated) with a clean chart-area band: piecewise encoded sRGB already uniquely wins, and the earlier ~15 mean channel delta was a sampling issue. That path was not changed. This matrix is only about transparent stops.

## Official session

- Evidence root: `C:\Users\AMD\AppData\Local\Temp\t189-f06-transparent-stops-20261126`
- Source SHA: `5978269db2c3d0bedd5a408cd16291ca2e257b998606cce2667f8f40d59f8e83`
- Seed SHA: `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`
- Cases: 30 (3 opaque controls, 2 fully-transparent backdrop controls, 7 two-stop alpha, 12 three-stop, 2 four-stop, 2 combo)
- Certificates: 30/30 `CLEAN_PUBLIC_OFFICE_EXPORT`
- Provenance: `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`
- Idle-0 observed: 2026-11-26 21:58:58.4548096+08:00 Asia/Shanghai
- Official launch: 21:58:58.7813274+08:00; exporter pid 43184
- Excel PID: 41820, start 21:58:59.8888466+08:00
- Exporter finished: 21:59:04.9644671+08:00 (ExitCode empty/null; summary+certs already gold)
- Empty owned Quit via GetActiveObject after Workbooks=0; Quit exited 21:59:30.7007524+08:00; POST ExcelCount=0

Models were frozen in `logs/predeclared-models.json` before Office.

## Result — not uniquely locked; left NI

Official natives carry real alpha (indexed+tRNS / RGBA). Fully-transparent controls are transparent `(0,0,0,0)`, not an opaque black backdrop. Scoring is official RGBA vs model RGBA; RGB is not compared when both alphas are 0.

Alpha: piecewise-linear interpolation of stop `a:alpha` units, then `round(units * 255 / 100000)`, has interior max abs error 4 and mean 0.48 across every predeclared model that lerps alpha. Skip-zero-alpha and ignore-alpha lose with interior max 255. Applying the locked two-stop office weight LUT to alpha is worse (interior max 31) and was exploratory only, not predeclared.

RGB: no unique official-matching model. Two-stop `FF0000` to `0000FF` with end alpha 0 has official mid `(140,0,112,127)` vs straight-encoded `(128,0,128,128)` (mid delta 16, interior max 126). Premultiplied and gamma models are worse. As end alpha rises, official mid approaches encoded mid, but the residual is systematic and was not closed by any predeclared candidate. 3-stop opaque controls still uniquely match encoded sRGB (interior max 2..3), as already implemented.

Best-count splits among predeclared models (straight-encoded 7, skip-zero 15, premul-encoded 4, straight-g22 1) confirm there is no unique RGB winner. Implementing encoded RGB plus linear alpha would be a guess, not a unique lock. The library continues to reject `a:alpha` children on gradient stops.

A first scoring pass that composited model RGB over black and compared it to official uncomposited RGB was invalid; it is kept as `logs/model-discrimination-v1-composited-rgb-wrong.json`. The corrected score is `logs/model-discrimination.json`.

## Not done

No invented interpolation constant, no lowered zero-pixel standard, no office-kernel-on-alpha implementation, no 425x266 size compensation.
