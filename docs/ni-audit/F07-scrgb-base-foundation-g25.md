# F07 strict untransformed normal-domain scRGB base foundation — g25, partial pending review

## Original authorized prerequisites-first scope and authoritative contract

This continues the original t166–172 OOX-1 color-foundation work, not authority from a suggested new scope. No review wait or reopening accepted unrelated features.

ECMA-376 Part1 fifth edition2016 §20.1.2.3.30 states scrgbClr r/g/b each0..100%, LINEAR gamma1.0, and explicitly equates50%/50%/50% with srgbClr BCBCBC. CT_ScRgbColor requires r/g/b:ST_Percentage and permits zero-or-more EG_ColorTransform. This implementation is the strict integer NORMAL-DOMAIN LEAF SUBSET, not a claim that other schema-legal values/percent spellings/transforms are illegal. Values0..100000 mean thousandths of a percent; negative/above100000/overflow/percent/+ strings remain NI, not guessed clamp/extended-gamut support.

Original official ZIP SHA9d0bcad9cf06054785b03762fcfadbf6bab7e54a5f9d69434e34b7fd464d4129 and PDF SHA7cccfd0ad0e2ef89316acece15e44a8a088d0603ce4bba4e7b0361b50d13a37d. Retained official PDF page2783 excerpt, exact normal-domain/schema text, and receipt:
https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip

The ICC sRGB registry identifies IEC61966-2-1:1999; its published characterization A.8 gives the precise component transfer, approximately gamma2.2 overall but NOT the simple power1/2.2: for C<=0.0031308, C'=12.92*C; otherwise C'=1.055*C^(1/2.4)-0.055. Normal-domain linear scRGB uses the same primaries/white point, so only per-component transfer is needed here, no guessed matrix/adaptation. Scale by255 and quantize nearest in this explicit8bit library profile; actual native controls below corroborate selected quantized values, not all possible inputs. Out-of-domain clipping described elsewhere in ICC is not enabled. We use forward A.8, not the unrelated inverse-profile B.1 text (including its blue-divisor typographic inconsistency).
https://registry.color.org/rgb-registry/srgb
https://registry.color.org/rgb-registry/files/sRGB.pdf
ICC PDF SHAceed855c29808217a472954abb04cdc60a5bd3200387a93f5c5ffcef8ddf9f1f. Former lower-case www.color.org/srgb.pdf returned404; canonical public registry PDF opened normally, no credential/transport workaround. CSS Color4 was only corroborative research, not a palette or authority to guess an OOXML gamma.

## Actual implementation and protected boundaries

Added reusable linear_srgb_channel and strict parse_frame_scrgb_color. Exact three unqualified semantic attrs r/g/b required; namespace declarations ignored; duplicate/extra/qualified attrs, element children and non-whitespace text rejected. Reuse existing strict parse_frame_integer unchanged. resolve_frame_solid gets isolated early-return scrgbClr arm, so transforms/alpha remain NI and cannot accidentally use srgb transform dispatch on a new base. Existing Excel-only frame/Cartesian-plot host and single-fill guards unchanged; no other-host/series/legend/generic resolver widening.

All27 pre-existing helper bodies unchanged (the previous25 plus drawingml_preset_rgb and parse_frame_preset_color), including exact190 enum/RGB map and its distinct spellings, HSL/integer helpers, exactalpha/default/accepted semantics. Only2 superseded unaccepted g23/g24 plain-scRGB NI assertions were updated to transformed-scRGB alpha cases that remain NI. No accepted assertion, native/old PNG or prior review was changed.

8 new Rust tests: exact IEC/documented gray rather than raw128 or guessed gamma2.2=186; every100001 normal integer component monotonic/domain rejection; primaries/asymmetric/theme independence; strict attrs/leaf/range/lexical profile; duplicate/qualified attrs; chart/contrasting plot96/106/144dpi equivalence; Slide isolation; single-fill/other models/mixed gates. Exhaustive monotonicity is not an exhaustive native equivalence claim.

## Predeclared independent controls, genuine native evidence and lifecycle

SourceSHA `82b40d8ffe921fcadd100c5c1b014c49a7a5f5670b555d746c08276c04ce66d1`.40 predeclared MC-preflight controls=20scRGB+20RGB references, calculated BEFORE implementation and native capture with80-digit Python Decimal using the published exact transfer, not renderer output or sample fitting. Include endpoints/primaries/25–50–75% gray, low light, fixed normative branch-adjacent313/314 integer values, asymmetric channels and4 contrasting-background plot pairs. Those313/314 cases exercise the documented branch boundary, not a threshold search or proof that8bit output uniquely identifies the equation.

Old binary20NI/20reference success. One fresh OS gate `2026-11-22T00:58:47.3655068+08:00` Count0 allowed only this new source through original unmodified MC preflight and owned-readonly verifier. Excel16/DPI106 and explicit source/copy/name/title/sheet/geometry identity checked. Own success seed+explicit copy opened readonly and both finally Close(false), RemainingWorkbookCount0, source/seed/copy hashes unchanged, exclusive reads/no owner locks/no repair, Save/Quit/Kill=false/clipboardunused. Exported40 PNG once using ChartObject.Chart.Export, kindOOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE, NOT print/clipboard/Bitmap/unmodified-official gold. No prior completed plot-alpha/satOff/sat/lum matrix or pending hue/HSL/preset native was repeated or implicitly satisfied.

Lifecycle last observation `2026-11-22T00:58:53.5992872+08:00`: process count1, ProcessStillRunning=True, WindowStillPresent=True; empty owned process/window is historical observation, NOT permission to reuse/quit/kill or launch another session around it. No new Office session after this audit.

Native scRGB/native predeclared RGB reference whole-PNG equality20/20. Native/library RGB exact20/20, maxerror0byte; native/reference RGB maxerror0byte. New library20 inputs now render, reference whole-PNG equality20/20/RGB maxerror0byte. This establishes controlled basic semantics without tuning gamma/offsets.

Whole native/library file equality0/20, native425x266 vs library424x265 at106dpi. Raw common-origin changed-pixel range8566..18075 is diagnostic only, no stretch/registration/masks/compensation or global pixel pass. All old1byte/AA/layout/stroke residuals, HSL SDK blue/angle conflict and hue/HSL/preset native pending remain explicit. No lower accepted zero-pixel criteria or autoacceptance.

## Full verification and immutable generation/index receipt

Unchanged mandatory command cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1:2284passed/190targets/0failed/0ignored, Python15/CLI passed. Inherit ALL647 prior successful outputs=Cases56+Prior591 fromg24, not merely619old PNG count. All647 history+20current old RGB references=667 existing PNGs byte-identical, intentional old-image changes0; new20 outputs additional, next complete successful set687=Cases40+Prior647. Four scatter controls remain NI. BinarySHA `87a757a521cac11e33c8c7c4cdbc1c891dea3ccec763b92e5cab59c87139ab3a`. All protected helper/source/file hashes, accepted rectangular body `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`, opaque PNG `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`, page_paint/t131/main/Pages/shared Office guards unchanged.

Unique generation20261122-F07-scrgb-base-g25/F07-scrgb-base-foundation-待审.html joins existing INDEX only, default unreviewed. Original80artifacts/2291href/9545files (9499archive+46desktop), URLs/localStorage/review states/g8a/g13 corrections/all old manifests/rollback byte-preserved. New81artifacts=75NI evidence/41IDs+6PR12. Counts remain4accepted/5pending/26partial/10NI/1safety/3illegal=49, not all implemented. Prior PR2903bytes exact backup pr14-body-before-g25-20261122.md SHAe20015c53f21a837429dbcb2e80c2476f4f608b6c63a65ec66e66ee25c112460.

Correct-author ordinary commit/hooks; single batch-end bounded original SSH read probe then one normal push only if recovered, no retries/credential/transport changes, no merge. Final publication status comes from receipt. Parent gets proposal only, no PR body/comments/messages written by this executor.

Next independent color prerequisite: sysClr val/lastClr fallback/platform-dependence contract before enabling a base input; do not unconditionally substitute lastClr for a live system color. Newly supported bases' alpha/mixed transforms still require their own supported color-space/order contract/evidence, not automatic admission through old srgb helpers. No wait for review/native/network or endlessAA tuning.
