# F07 strict untransformed preset-color base foundation — g24, partial pending review

## Authoritative contract, no CSS guesswork

Downloaded directly from Ecma International, ECMA-376 Part1 fifth edition2016, section20.1.10.48 ST_PresetColorVal: all190 exact enum names and RGB triples independently checked against its190-member XSD A.4.1. Original ZIP SHA `9d0bcad9cf06054785b03762fcfadbf6bab7e54a5f9d69434e34b7fd464d4129`, PDF SHA `7cccfd0ad0e2ef89316acece15e44a8a088d0603ce4bba4e7b0361b50d13a37d`. Original published URL:
https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip

Retained seven-page original-derived PDF excerpt, extracted table, CT_PresetColor contract, enum/RGB JSON and hash receipt. The source PDF generated xref-reconstruction warnings during extraction; all190 rows in the extracted PDF were independently checked identical to the full source text. No palette values were changed to silence a warning or match samples.

Section20.1.2.3.22 and CT_PresetColor require val:ST_PresetColorVal and allow zero or more EG_ColorTransform children. We implement the strict untransformed LEAF SUBSET, not a claim that transforms are invalid OOXML. Exactly one unqualified val semantic attribute, namespace declarations ignored, no element children or non-whitespace text; unknown/case-folded/CSS-only names or whitespace-normalized token spellings outside this strict profile remain NI. Val matching is exact, no dark/dk, light/lt, medium/med or gray/grey normalization. All accepted alternate names are independently present in the normative190 table.

Important normative distinctions deliberately NOT silently aliased: darkSeaGreen=8FBC8F versus dkSeaGreen=8FBC8B; lightGoldenrodYellow=FAFAD2 versus ltGoldenrodYellow=FAFA78. These are separate published RGB entries. Library normative-model equality does NOT prove that installed Excel uses those differing values. They are explicit discriminating native controls, native remains pending below.

Opened current MS-OI29500 note documents an Office extension transparent=FFFFFF; it is NOT in this strict190 enum profile and NOT CSS alpha0. Opened MS-OE376 note documents RGB deviations only for Office2007/SP1/SP2, including ltGoldenrodYellow and a shifted tail turquoise..yellowGreen. We do not apply that historical table to modern Excel or call unsupported extensions invalid OOXML. Sources:
https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/cc8be648-0286-48f8-8f8e-66b878d23432
https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/237b3230-4341-4f1f-94b7-0e85d2dbb278
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetcolor?view=openxml-3.0.1
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetcolorvalues?view=openxml-3.0.1

## Actual implementation and intentional scope

Added reusable drawingml_preset_rgb exact190 lookup and strict parse_frame_preset_color; resolve_frame_solid gets an early-return prstClr arm. Existing Excel-only frame/Cartesian plot upstream host/single-fill guards unchanged. No series/legend/Slide host or generic color resolver expansion. Transforms/alpha on this base remain NI rather than entering generic transform dispatch. Existing25 helpers (including both g23 HSL helpers), exactalpha/default/accepted/HSL outputs are unchanged. Only one superseded unaccepted g23 NI assertion changed from plain prstClr red to transformed prstClr red/alpha, because the new base is now implemented; no accepted assertion or image changed.

56 predeclared MC-preflight source controls=28 preset+28 existing RGB references, fixed BEFORE implementation. Chart keys cover primary/color-meaning cases, exact documented synonyms, both non-equivalent pairs, Office2007-tail differentiators; four plot controls discriminate both non-equivalent pairs over a contrasting background. SourceSHA `2fe6aa474fd0ce9b866f28fb3bfd75bf5fb38773ba2b7272bdf4712c7de99790`. Old binary28NI/28success; new28 formerly NI now render. Normative-reference whole PNG equal 28/28; RGB maxerror0byte. References derive from the published table, NOT native gold.

## Native lifecycle and honest boundaries

Initial fresh machine/OS observation2026-11-22T00:34:14.4067985+08:00 Asia/Shanghai: Excel1/PID43732/titleExcel, not stale permission to cleanup. Later single fresh gate `2026-11-22T00:51:59.5189520+08:00` Count=1 PIDs=[43732]. This batch uses NO Office session, no COM binding/reuse/Save/Quit/kill/workbook operations or guard changes. No idle loops or repeated completed plot-alpha/satOff/sat/lum matrices. NativeCount0/pending; SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED, ExportKind=null. Table/reference equality is not native Chart.Export/print/clipboard/unmodified official gold. Hue/HSL native, SDK HSL blue/angle conflict and prior1byte/geometry/AA/stroke boundaries remain unresolved, no compensation/acceptance-threshold change.

## Full validation and immutable review history

Eight new Rust tests: canonical values/unknown spellings, no-alias discriminants, all190 exact enum names/theme independence, strict val leaf/namespace/transforms, duplicate attributes, chart/contrasting plot96/106/144dpi RGB equivalence, Slide host isolation, duplicate fills/other models/mixed gates. Exhaustive separate Python source-map check verifies all190 RGB triples against the independently frozen official table.

Exact mandatory cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1:2276passed/190targets/0failed/0ignored; Python15 and CLI passed. All591 historical outputs (g23 Cases40 plus its prior551, including newly supported g23 HSL cases) plus28 current old RGB references=619 existing PNGs byte-identical, intentional old-image changes0. Four scatter controls remainNI. Test binarySHA `491b90173936fe49bc8171f7836706b00d46bda53ef5cad4242b216b3cfe2b20`. Accepted rectangular function SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978` and opaque fixtureSHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d` unchanged; page_paint/t131/main/Pages/Office guards/protected SHA verified. This expands replay accounting rather than dropping the20 g23 new outputs from the prior reported571 baseline.

Unique review generation20261122-F07-preset-base-g24/F07-preset-base-foundation-待审.html integrated into existing INDEX, default unreviewed; no duplicate desktop page. Original79 artifacts/8786 protected files (8740archive+46desktop), URL/localStorage/review states/g8a/g13 corrections/all old manifests/rollback remain byte-identical. New80 artifacts=74NI evidence/41IDs+6PR12. Counts unchanged4accepted/5pending/26partial/10NI/1safety/3illegal=49, not all implemented. Prior PR3207bytes exact backup pr14-body-before-g24-20261122.md, SHA f1990a158a2e71b1c3fc1f0130ee46fef8c7b8c035c0844ec7c4f350aaff6ce1.

Ordinary correct-author commit and batch-end single bounded original SSH readonly probe; one ordinary push only if recovered. Publication is confirmed by final receipt, not assumed. No merge/automatic acceptance.

Next concrete independent prerequisite: scrgbClr normal-domain base/linear-to-sRGB transfer contract before any implementation; no guessed gamma or byte compensation. Preset alpha/mixed transforms and native discriminants remain explicit pending work rather than implicitly enabled by the existing srgb path.
