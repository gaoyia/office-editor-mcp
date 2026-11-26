# F07 bare srgbClr transform chains (t189 official matrix)

Date: 2026-11-26 (Asia/Shanghai)

## Question

Can already-locked bare `srgbClr` transforms (`comp`, `inv`, `gamma`, `invGamma`, `gray`) be composed, and if so in what order?

schemeClr / theme binding and sysClr were out of scope. Theme accent1 `6D2A91` was not rewritten to live `156082`.

## Official session

- Evidence root: `C:\Users\AMD\AppData\Local\Temp\t189-srgb-transform-chains-20261126`
- Source SHA: `888ec47f6abd7ad2df9e8264f96829b8a0db6302601a3b8b013e3f43abe3535e`
- Seed SHA: `62d192a7a1f7706b1763a119b7fb5e2b4f8fab121ef4f9fce07c0f2eac8c493e`
- Cases: 50 (10 single-transform controls + 40 length-2 pairs; bases `808080` and `C04020`)
- Certificates: 50/50 `CLEAN_PUBLIC_OFFICE_EXPORT`
- Provenance: `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`
- Idle-0 observed: 2026-11-26 21:59:43.4557508+08:00 Asia/Shanghai
- Official launch: 21:59:43.7712679+08:00; exporter pid 68180
- Excel PID: 59852, start 21:59:44.8159338+08:00
- Exporter finished: 21:59:51.5867898+08:00 (ExitCode empty/null; summary+certs already gold)
- GetActiveObject Quit issued 21:59:52.1503066+08:00 after Workbooks=0
- Closer first wait ended 22:00:17.2792617+08:00 with `QUIT_DID_NOT_EXIT` (exit 2); no Save, no kill
- Process was gone by a later observation at 22:00:27.8917372+08:00; ExcelCount=0. No second official session was started from a non-idle state.

Models were frozen in `logs/predeclared-models.json` before Office: XML-order locked helpers, reverse order, identity.

## Result

- Controls (t187 singles re-checked): 10/10 exact at official (5,5)
- XML-order exact on 35/40 pairs; reverse 19/40; identity 11/40
- When order differs and official matches exactly one side: XML-order uniquely won 17 pixels; reverse uniquely won 0 exact non-residual pixels
- Five C04020 pairs are integer-roundtrip residuals against per-step rounded locked helpers (max abs 1..4). Official `inv+inv` on `C04020` restores the base exactly; the integer helpers do not (library `192,65,36`). Official `invGamma+gamma` also restores the base; library is `192,64,34`. These residuals are recorded, not fudged, and were not used to pick reverse order or lower the exact standard on the 35 exact cases.

The uniquely winning rule is: apply the already-locked empty transforms in XML document order on bare `srgbClr` only. Length 3+ stays NI. schemeClr / sysClr chains stay NI.

## Not done

No theme constant rewrite, no sysClr hard-code, no silent lastClr, no 425x266 size compensation, no plus/minus 1 translucent fudge.
