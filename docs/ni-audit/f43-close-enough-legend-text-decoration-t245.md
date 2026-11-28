# F43 close-enough legend underline, strike, and baseline shift

Official Excel 16 / DPI 106 Chart.Export gold at `t174-f43-legend-font-20261121` already exports seven clustered-column right legends. All seven certificates are `CLEAN_PUBLIC_OFFICE_EXPORT`. On the current tree, `ctrl` and same-family Arial 18pt bold already exported; `underline`, `strike`, `subscript`, and `doubleunderline` were still `NotImplemented`. The official `superscript` chart has no typography in the package XML (identical legend markup to `ctrl`), so it is not treated as a missing encoding.

## Official encodings

- `underline`: legend `txPr` `defRPr u="sng"`
- `strike`: `defRPr strike="sngStrike"`
- `subscript`: `defRPr baseline="-25000"` (DrawingML 100000ths of the font size, negative moves the run down)
- `doubleunderline`: `defRPr u="dbl"`
- `arial`: `sz="1800" b="1"` with matching latin/ea/cs Arial — already exportable from F41
- Other `u`/`strike` values, mismatched cs names, rotation, vertical text, and complex runs stay NI

Office can Chart.Export the four still-rejected decorations, so the library must produce an image. Close-enough is authorized. This is not a unique official-matrix lock of every typography decoration or residual pixel.

## What this change does

- Parses official-like `u=sng/dbl`, `strike=sngStrike`, and the XML `baseline` percentage onto the existing legend `TextStyle`. Other underline/strike values remain explicit `ChartImageUnsupported`.
- Applies the baseline shift as `top - (baseline/100000)*px` in y-down space. No invented superscript/subscript scale factor is applied; size residuals stay on the review page.
- Paints 1-device-pixel hairlines from GDI TEXTMETRIC ascent and descent: a single or double underline just below the baseline, and a mid-ascent strike. Not COM coordinates, not invented Office EMU constants.
- Already-exportable default and Arial family paths stay on the existing typography model.

## Residuals and leftover NI

Absolute position, font-size metrics, and AA residuals, including subscript size versus Office, stay on the shared F41/F43 review page. The recorded zero-pixel standard is not rewritten.

Still NI, with no official gold unlocked here: other underline/strike styles, theme/implicit families, rotation, vertical text, complex runs/paragraphs, and mismatched cs names.
