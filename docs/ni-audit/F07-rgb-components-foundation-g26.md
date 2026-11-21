# F07 standalone RGB component replacement and sysClr contract boundary — g26, partial pending review

## Original authorized scope; sysClr is a genuine policy prerequisite, not a fabricated color

Continues original t166–172 prerequisites-first remaining NI, without review wait. First investigated sysClr from ECMA-376 Part1 fifth edition2016 §20.1.2.3.33 and §20.1.10.58, not a CSS assumption. The former binds a color to predefined OS elements and defines lastClr as the value last computed by the generating application; the latter explicitly says val is the color currently used on the viewing system and applications SHALL use lastClr if system colors are not supported. Those are two distinct contracts, not proof cached lastClr equals the current palette.

Existing renderer targets the Excel current-system-color profile and has contradictory-cache window/windowText NI tests; it has no explicit injected platform palette or agreed fallback-policy input. Enabling unconditional cached lastClr in the existing path would silently change that profile; querying ambient GetSysColor/COM would introduce nondeterministic environment dependence. Therefore sysClr remains NI, existing tests/comment/theme-slot behavior unchanged. A future explicit deterministic injected-palette or opt-in unsupported-system lastClr fallback contract is needed; this is not a claim the standard forbids fallback or a request for the user to close anything. Rather than an audit-only batch or idle loop, proceeded to independent missing F07 RGB component functionality in the same original color-transform scope.

Official original ZIP SHA9d0bcad9cf06054785b03762fcfadbf6bab7e54a5f9d69434e34b7fd464d4129, PDF SHA7cccfd0ad0e2ef89316acece15e44a8a088d0603ce4bba4e7b0361b50d13a37d; original text/schema excerpts and source receipt retained.
https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip

## Actual independent shared implementation; not a guessed formula

ECMA §20.1.2.3.4 blue / .10 green / .23 red specify substituting the requested component with the other two unchanged. EG_ColorTransform uses CT_Percentage. Implemented standalone strict integer normal-domain0..100000 subset on existing srgbClr/schemeClr Excel frame/Cartesian-plot paths. Other schema-legal ranges/percent spellings remain unsupported, not declared illegal. Native controlled evidence below discriminates component intensity from encoded-byte percentage in the Excel profile; it is not a universal claim about unspecified hosts.

One new shared frame_rgb_component helper: exact unqualified val, no extra/duplicate/qualified semantic attr, child element or non-whitespace text; namespace declarations ignored; existing strict parse_frame_integer and exact linear_srgb_channel reused unchanged. For selected channel C=v/100000, reuse ICC/IEC forward transfer C<=0.0031308:12.92C, otherwise1.055*C^(1/2.4)-0.055, then nearest8bit. Other two bytes retained exactly. No guessed power2.2, clamp, lookup thresholds, additive compensation or broader mixed dispatch. ICC original PDF SHAceed855c29808217a472954abb04cdc60a5bd3200387a93f5c5ffcef8ddf9f1f.
https://registry.color.org/rgb-registry/files/sRGB.pdf

Added only standalone red/green/blue match arms in resolve_frame_solid. All29 old helper bodies unchanged, including190preset map/distinct spelling values, HSL, precise alphaMod/default paths and g25 scRGB. New helper count30. sysClr, new-base alpha/transforms, redOff/redMod and green/blue counterparts, multi-component sequences, alpha/component mixtures and Slide/series/legend/other-host gates not widened. No old assertion/image was changed.

8 new Rust tests: channel replacement/untouched bytes over varied base colors; exact50%188 versus encoded128/power2.2=186; strict normal domain and lexical forms; leaf/namespace/duplicate attrs; srgb/theme resolution; contrasting plot/chart96/106/144dpi RGB equivalence; sysClr/new-base/mixed/offset/modulation NI; Slide isolation.

## Predeclared evidence and one safe owned-readonly native session

SourceSHA `1d9bedf9cc7d4c650fb207f7d616cb93db91bd203bf6f914a47e80f5d357fef2`.48 predeclared controls=24components+24explicit RGB references, frozen before renderer implementation/native. Independent80digit Decimal exact IEC references plus separately recorded encoded-percentage alternatives. Eighteen chart cases (red/green/blue times0,313,314,25000,50000,100000), three contrasting plot50% and three scheme50% pairs.313/314 are fixed published branch-adjacent integers, not a threshold scan; they do not uniquely distinguish all transfer equations after8bit quantization. Old binary24NI/24reference success. Initial Reference/NativeProof flags are historical pre-native labels, not rewritten after capture.

Fresh OS gate `2026-11-22T01:20:28.4578343+08:00` Count0, original MC preflight and owned-readonly guard unchanged. Only this new fixed source exported once:48 derived ChartObject.Chart.Export native PNG. Excel16/DPI106, explicit source/copy/name/title/sheet/dimensions/path/SHA preflight; own success-seed+explicit copy readonly, both finally Close(false), RemainingWorkbookCount0, source/seed/copy unchanged/exclusive reads/no locks/no repair, Save/Quit/Kill=false/clipboardunused. SourceKind=OOXML_DERIVED_REEXPORTED_BY_EXCEL, ExportKind=EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE; NOT print/clipboard/bitmap/unmodified-official gold.

Lifecycle last observation `2026-11-22T01:20:35.9145717+08:00` process1, ProcessStillRunning=True, WindowStillPresent=True. Owned PID68396 empty window is historical observation only, not reuse/cleanup permission. No later Office session or busy idle loop; no prior completed plot/satOff/sat/lum/scRGB matrices repeated. Hue/HSL/preset native and SDK HSL blue/angle conflict still pending, not implicitly resolved.

Native component/native predeclared RGB reference whole-PNG equality24/24; native/predeclared RGB exact24/24/maxerror0byte. Encoded-percentage alternative matches only6/24 (the6 nondiscriminating endpoints); all18 nonendpoint discriminating cases favor predeclared linear-IEC references. This corroborates the general selected intensity contract, not unique causation or exhaustive native equivalence for every integer.

New library24 formerlyNI render; library/predeclared RGB whole-PNG equality24/24/maxerror0byte. Native/library RGB exact24/24/maxerror0byte. Whole native/library PNG equality0/24, native425x266/library424x265; raw common-origin changed pixels18074..18075 diagnostic only, no stretch/registration/compensation or whole-image acceptance. Existing1byte/AA/layout/stroke/terminal-gap/miter/retrace/scatter residuals unchanged, no threshold/AA hunt.

## Mandatory verification and immutable review generation

Unchanged cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1:2292passed/190targets/0failed/0ignored;8newRust/Python15/CLI passed. Inherit all687 successful g25 outputs=Cases40+Prior647, not just667old regression count. All687history+24current old RGB references=711 existing PNG byte-identical; intentional old-image changes0. New24 additional successful outputs, next complete set735=Cases48+Prior687. Four scatter stillNI. Tested binary `a5c7386c139a7296248ac3bc41228884df6568ed64c582c9b3d47650c0672f74`.

All29old helpers, ALPHA_PRODUCT_DENOM, protected source/Office guards/page_paint/t131/main/Pages unchanged. Accepted rectangle body `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978` and opaque fixture PNG `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d` unchanged. No accepted tests/PNGs edited or autoaccepted.

Unique generations/20261122-F07-rgb-components-g26/F07-rgb-components-foundation-待审.html added through existing INDEX, default unreviewed. Original81artifacts/2413href/10411files (10365archive+46desktop) bytes/URLs/localStorage/review states/g8a/g13 corrections/all old manifests/rollback retained. New82artifacts=76NI evidence/41IDs+6PR12. Counts4accepted/5pending/26partial/10NI/1safety/3illegal=49 remain incomplete. Original rollback scope unchanged, guarded INDEX-only rollback retains all evidence. Old PR3057bytes versioned verbatim pr14-body-before-g26-20261122.md SHA721f9ba849605ffd7a31d0fa42c04261eba90744353ec64e5f6fbeb8b974486a; executor performs no PR body writes/comments/messages.

Correct-author ordinary commit/hooks only, no merge/accept/forced operation. One batch-end bounded existing SSH read probe then one normal push only if recovered; otherwise retain local commit and exact pending state. No retry/credential/transport changes. Final publication determined by final receipt.

Next original-scope prerequisite: standalone RGB component modulation/offset contract and linear-light arithmetic/evidence (not encoded-byte math or guessed out-of-range clamp); sysClr needs explicit platform/fallback policy first. All unverified mixed/new-base transformations remain NI. Do not wait for review/native/network or repeatedly audit the same completed matrices.
