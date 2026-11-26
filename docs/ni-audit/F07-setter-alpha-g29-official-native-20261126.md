# g29 official Chart.Export recertification (2026-11-26)

Status: official gold certificates now exist. Not auto-marked pass. Historical incomplete files were not mutated.

## Why a new folder

The 2026-11-26 19:00 combined session wrote 44 g29 rasters, then failed when opening g30 in the same Excel instance (`GetFullPath("")`). Those certificates are `NATIVE_EXPORT_INCOMPLETE_NOT_GOLD` and `OwnedCloseVerified=false`. They remain at:

`C:\Users\AMD\AppData\Local\Temp\t183-g29-g31-native-20261126\g29`

t203 still has the derived source (`d6bbfe93…1523c9`) and shared seed (`62d192a7…493e`) but no `office-raw` rasters. This recertification copied that source into a new evidence root and ran one official matrix session. t203/t204/t205 and the t183 incomplete files were not rewritten.

## Official session

Evidence root: `C:\Users\AMD\AppData\Local\Temp\t187-g29-setter-alpha-native-20261126`

- Idle-0 observed 2026-11-26 21:07:32 Asia/Shanghai, then one `export-controlled-chart-native.ps1` session.
- Excel 16.0, DPI 106, PID 60516, start 2026-11-26 21:07:33.5054576+08:00, `/x` seed.
- 44/44 `CLEAN_PUBLIC_OFFICE_EXPORT`. Provenance `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`.
- Close(false), FilesClean, repairs=0, source SHA unchanged, no Save, no Kill.
- Empty owned leftover Quit via GetActiveObject after Workbooks=0; process exited 2026-11-26 21:08:05.1689719+08:00. Post ExcelCount=0.

The 44 new official PNG SHA-256 values are identical to the 44 t183 incomplete rasters. Those earlier rasters were useful; the new folder supplies gold certificates and a verified Close without discarding or rewriting the incomplete set.

Official rasters remain 425x266. That size residual versus the library 424x265, and any +/-1 RGB translucent residuals, were not used to change the already-locked g29 setter+alpha contract.
