# F07 hue modulation shared foundation — g14, pending review

## Contract before implementation

- SDK DrawingML HueModulation / CT_PositivePercentage: 50% halves the angular hue, 200% doubles it. https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.huemodulation?view=openxml-3.0.1
- Transitional DrawingML schema: ST_Percentage is xsd:int; ST_PositivePercentage restricts it to >=0, unlike the fixed 0..100000 alpha percentage. https://schemas.liquid-technologies.com/OfficeOpenXML/2006/dml-basetypes_xsd.html

This baseline accepts strict integer-thousandth spelling in [0,i32MAX]. It shares the existing signed parser and checks nonnegativity, without changing that parser or old luminance/saturation/alpha paths. Other lexical forms, extra attrs/children, general transform sequences and new base color models remain unsupported.

## Discriminating native evidence

Source SHA256 `acf5e3d4c3486d8b0e4f21845f300b36263ebf26f2550988e57a241b84480c9c`. Same title,288×180pt, axes and data. One fresh owned readonly Excel16/DPI106 session exports32 fixtures:14 transforms +14 periodic-wrap mathematical RGB references +4 extra angular-clamp references. The two overflow models were declared before Office, not fitted after viewing results. Four genuine overflow chart/plot/theme controls support periodic angular wrap and all reject clamping overflow to red. Twelve wrap references have identical complete native PNG bytes; two differ at half-angle quantization / alpha readback. Mathematical reference fixtures are not fabricated gold.

Correct provenance: **OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE**. These are not untouched official originals, printed-page output or CopyPicture Bitmap gold. Native425×266 and library424×265 remain at their own dimensions: no stretching/registration, no whole-image acceptance claim.

## Actual general implementation

`frame_hue` uses the existing RGB↔HSL conversion, multiplies the hue by the supplied ratio, reduces complete turns modulo1, and converts back with saturation/light unchanged. `resolve_frame_solid` accepts only standalone hueMod or hueMod followed by alpha for existing sRGB/theme chart and plot solid fills. No guessed thresholds, angle/color tables, paths or fitted rounding constants.

All10 pre-existing tint/shade/lum/sat/alpha/strict-parser/HSL helper bodies retain their hashes. New renderer colors against14 native target probes: **12 exact, maximum RGB channel residual 2 byte**, alpha all exact. Record per-case values rather than hiding quantization/unpremultiplication behind a whole-plot AA error. Larger integer factors are supported by the general periodic contract, while native coverage here is bounded to0..300%; this is partial pending review, not blanket pixel equivalence.

14 previously NI inputs now render. All269 historical PNG controls (including the original253 and g13's16 new satMod controls) plus18 pre-existing explicit-RGB references retain exact PNG bytes:287 existing images unchanged, intentionally changed old PNGs0. Four scatter controls remain truly NI.

Eight new Rust tests cover numeric/structural parsing, angular multiplication and multiple turns, achromatic/identity, theme, straight alpha, chart/plot at96/106/144dpi and unsupported mixed sequences. Mandatory suite **2203 passed /190targets /0failed /0ignored**, Python15 and actual CLI build pass. Renderer replay records tested binary/request/input-source/output hashes for every case.

## Ownership, preservation and review

Existing MC preflight and all shared Office/binding/Bitmap guards remain unchanged. Only explicitly held readonly seed/copy are opened, with names/title/canvas, FullName/process/source hashes and file locks audited. Finally Close(false) verifies both closed, Workbooks0, unchanged files and no repair. No Save/Quit/kill or user workbook closure; empty-app disappearance is only an OS observation.

The unified desktop INDEX links unique generation `20261121-F07-hue-modulation-g14/F07-hue-modulation-foundation-待审.html`. Preserve prior69 review artifacts and2836 old protected files:2790 inside the archive,46 intentionally retained at their original desktop URLs. Preserve localStorage states, g8a, the g13 inventory-label correction and all original manifests/rollback. New totals70 review artifacts,64 NI evidence pages/41 IDs plus6 PR12; no automatic acceptance.

The prior3513-character PR body is preserved verbatim in [pr14-body-before-g14-20261121.md](pr14-body-before-g14-20261121.md), SHA256 `0318a8e2b656eba9dd94641f5f841f818d708a528cb719b3417368da470ef801`. The original62450-character audit archive remains untouched. Update only concise PR body, not title/base/state; commit/push receipt is authoritative.

49 groups remain **4accepted /5pending /26partial /10NI /1safety /3illegal**. Standalone alphaMod is a next independent prerequisite benefiting from the positive-percentage parser. sysClr/hueOff/hslClr base/arbitrary transform mixtures, auto-legend key metrics, square terminal gap, lim4 plot,180°retrace/scatter and other boundaries remain separate. No accepted criteria lowered and no claim that all NI is implemented.
