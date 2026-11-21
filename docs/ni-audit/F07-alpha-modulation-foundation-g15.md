# F07 alpha modulation and exact opacity products — g15, pending review

## Contract and actual missing prerequisite

- AlphaModulation multiplies input opacity, never above100%;200% doubles it and50% halves it: https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.alphamodulation?view=openxml-3.0.1
- Alpha sets a specific opacity without changing RGB: https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.alpha?view=openxml-3.0.1
- CT_PositivePercentage is based on signed32-bit ST_Percentage restricted to>=0; CT_PositiveFixedPercentage has the additional100000 upper bound: https://schemas.liquid-technologies.com/OfficeOpenXML/2006/dml-basetypes_xsd.html

The existing integer-thousandth FrameFill opacity cannot represent products such as50000×33333/100000 exactly. Rounding that result to an integer percentage before final8-bit coverage yields43 instead of42. Unconditional flooring fails the neighboring50001×33333 case, whose correct coverage is43. This is a real precision prerequisite, not an AA/layout compensation.

The new private AlphaProduct retains the u64 numerator over100000², caps at1 and rounds only at final8-bit coverage. Integral products normalize to the old Translucent/Solid paths. All14 old lum/tint/shade/sat/hue/alpha/parser/HSL helper bodies remain byte-for-byte identical. No floating-point color model, guessed factor table, intermediate rounding or desktop path is introduced.

Supported for existing sRGB/theme chart frames and Cartesian plot solid fills: standalone alphaMod; explicit alpha then alphaMod; alphaMod then absolute alpha (the latter overwrites opacity, but earlier operands are still validated). Factor uses strict nonnegative integer-thousandths0..i32MAX. The new absolute-alpha operand is structurally strict and0..100000. Existing standalone-alpha helpers and all prior transform sequences are unchanged. Repeated/mixed color transforms and other base models/lexical forms remain unsupported rather than silently ignored; translucent pie plot support is not expanded.

## Predeclared controlled native evidence

Source SHA256 `41c50fed169a01020140b342a5ef70e89e12604b514a32a4abe0fb033ec6e7cc`. A single owned readonly Excel16/DPI106 session exports45 same-title/288×180pt/axes/data controls:21 transform inputs and24 pre-existing explicit-alpha references. Reference values, including both fractional floor/ceil alternatives, were frozen before Office. All21 transform inputs have an identical complete native PNG reference; three deliberately incorrect fractional references differ. Native chart probes discriminate42 versus43 for the adjacent product pair. Both modifier orders and saturation at opaque are independently exercised; theme/gray/two bases/plot are represented.

Provenance is **OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE**. These are not untouched official inputs, printed-page output or CopyPicture Bitmap gold. Native425×266 and library424×265 stay at their own dimensions: no stretching/registration or whole-image acceptance claim.

New actual renderer: all19 chart-background alpha probes match native exactly. RGB is intentionally unchanged by alpha, while native PNG readback/unpremultiplication has a maximum3byte chart-channel residual. Two opaque-composited plot probes have maximum1byte RGB residual; their output alpha255 alone does not prove local fill opacity. Per-case values/common-origin pixel differences remain recorded, not hidden or used as fitted corrections. All accepted zero-pixel requirements remain intact.

## Tests, regression and ownership

21 former NI inputs now render. All301 historical PNG controls (including all32 g14 hueMod controls/references) plus24 current pre-existing alpha references retain exact PNG bytes: **325 existing images unchanged**, intentionally changed old images0. Four scatter controls remain NI. One existing NI-negative input (alpha then alphaMod) is now genuinely supported; its obsolete expectation was replaced with a still-unsupported extra hueMod sequence. The first-suite failure log is retained, not erased or counted as a pass. Nine new Rust tests cover standalone/clamp, incoming-opacity multiplication, exact fractional precision, absolute override, strict structural/lexical rejection, theme, chart/plot96/106/144dpi, straight RGBA and unsupported mixed sequences. The unchanged mandatory command passes **2212tests/190targets/0failed/0ignored**; Python15 and actual CLI build pass. Resumable replay receipts bind actual executable/request/input/output hashes to every result.

All shared Office/MC/binding/Bitmap guards are unchanged. Explicit names/title/canvas, FullName/PID/source hashes and locks are audited. Finally Close(false) verifies only the two explicitly owned readonly seed/copy books closed, Workbooks0, unchanged files and no repair. No Save/Quit/kill or user workbook closure and no second Office session. Later empty-process disappearance is merely an OS observation, not active cleanup. At21:55 Asia/Shanghai one application process was still observed; at21:58 the count was0. Neither observation is the own-workbook closure proof, and neither involved COM/Quit/kill.

## Review preservation, publication and remaining work

Unique desktop generation: `20261121-F07-alpha-modulation-g15/F07-alpha-modulation-foundation-待审.html`, linked prominently from the existing INDEX. Preserve old70 review artifacts and3282 old protected files:3236 inside the archive,46 retained at their original desktop URLs. Preserve all review states/localStorage, g8a/g13 corrections and every previous manifest/rollback. New totals71 review artifacts=65NI evidence pages/41IDs+6PR12; default unreviewed, no acceptance.

Prior3638-character PR body is preserved verbatim in [pr14-body-before-g15-20261121.md](pr14-body-before-g15-20261121.md), SHA256 fad1af659e851040d33db2a2ff14ee5194b513722f84bda6c409c21f9a9fa204. Older62450/3513-character archives stay untouched. PR title/base/state are not changed; final ordinary commit/push receipt determines publication status.

49 groups remain **4accepted/5pending/26partial/10NI/1safety/3illegal**. A next independent actionable prerequisite is standalone hueOff, requiring its own signed-angle contract and controlled evidence; not another terminal/lim4 parameter hunt. sysClr/hslClr/arbitrary transform mixtures, pie translucent plots, legend auto-key, square terminal gap, lim4 plot,180°retrace/scatter and all other documented boundaries remain separate. No claim that all NI is implemented and no user action is currently required.
