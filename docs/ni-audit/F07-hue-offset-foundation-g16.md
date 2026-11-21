# F07 signed hue offset foundation — g16, partial pending review

## Contract and discriminating evidence

SDK HueOffset changes hue while preserving saturation/light. Its Val is signed Int32 ST_Angle;600000 means10degrees (60000 per degree). IMPORTANT: the SDK Val remarks explicitly say shifts below0/above360 are clamped. This is not silently reinterpreted as a wrapping standard: https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hueoffset.val?view=openxml-3.0.1

Before Office,35 same-title/288×180pt/axes/data controls froze14 hueOff inputs,14 clamp references and7 additional periodic references. They span two sRGB bases,-360/-90/0/+90/+450degrees,plot-450degrees,theme+450,gray and optional trailing alpha. Source SHA256 `7e1ebfa6ab58bbaece6fe691dc16779146b27f49eeb84f7870d482e80ea612db`. All14 native transform PNGs match a predeclared mathematical reference; seven genuine overflow cases reject their clamp reference and match the periodic reference byte-for-byte. This is direct evidence of **Excel16 native chart behavior differing from the SDK clamp wording**, not confirmation that the documents say wrap. The implementation uses that proven Excel profile; no claims are made about other hosts or general OOXML compliance for overflow.

Provenance: OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE. Inputs are controlled derivatives, not untouched official originals; outputs are not printed-page or CopyPicture Bitmap gold.

## Actual general implementation

`frame_hue_offset` adds signed offset/(360*60000) to the existing RGB-to-HSL hue, reduces complete turns modulo1, then reuses the existing HSL-to-RGB helper. Saturation/light remain unchanged. Existing strict signed-i32 leaf parsing already supplies the prerequisite; its function body is unchanged and no guessed angle/offset/color table is added. Only standalone hueOff and hueOff followed by old alpha are accepted for existing sRGB/theme chart/Cartesian plot frame fills. The pre-existing render path reads these frames only for ChartHost::Excel, so Word/PPTX profiles are not expanded. New mixed/repeated/alphaMod sequences remain NI.

All17 old helper bodies, including all14 prior helpers plus alphaMod exact-product construction/conversion/initial-alpha validation, retain their hashes. No alphaMod precision, default, satMod/hueMod/lum/alpha path is altered. The old hueMod NI-negative list had one bare hueOff item which is now supported; that item now uses still-unsupported hueOff+hueMod. Other accepted tests/PNGs remain unchanged.

New actual renderer probes: 8/14 RGB exact, maximum RGB channel residual1byte, all alpha identical. The original HSL floating rounding and native alpha readback remain residuals, not fitted corrections. Initial new test expectations incorrectly assumed Python mathematical half rounding equaled the old Rust HSL result; the failure log is retained. Production helpers were NOT changed to make those expectations pass. Exact chart/plot integration compares to the existing helper's explicit RGB, while independent native probes retain their measured residual. Native425×266/library424×265 remain their own sizes, without stretch/registration or whole-image acceptance.

## Validation and lifecycle

14 formerly NI inputs now render.346 historical PNGs (including all45 g15 alphaMod controls/references) plus21 current pre-existing RGB references =367 existing PNGs exact byte-preserved; intentionally changed old PNGs0,4scatter controls stillNI. Eight new Rust tests cover signed bounds/strict structure,60000-unit shifts,negative/complete turns,Excel-vs-clamp discrimination,gray/identity/theme/trailing alpha,chart/plot96/106/144dpi and unsupported mixed sequences. Unchanged mandatory command:2220passed/190targets/0failed/0ignored;Python15 and actual CLI build pass. Actual binary/request/input/output replay receipts bind the evidence.

One readonly explicitly owned seed+copy session,MC namespace preflight,explicit names/title/canvas,FullName/PID/source hashes and locks are audited. Finally Close(false) proves both own books closed,Workbooks0 and unchanged source/no repair. No Save/Quit/kill/userbook close/second Office session or binding/Bitmap guard change. Later process counts are OS observations only, never active cleanup or substitutes for workbook closure proof.

## Review, publication and remaining work

Unique generation20261121-F07-hue-offset-g16/F07-hue-offset-foundation-待审.html is linked from the existing desktop INDEX. Old71review artifacts and3824 protected files (3778archive+46retained desktop) remain byte-for-byte, with URLs/localStorage states/g8a/g13 corrections/all manifests/rollback preserved. New72artifacts=66NI evidence pages/41IDs+6PR12,default unreviewed,not accepted. Exact current inventory/link counts are in the generation manifest and final receipt.

Prior3409-character PR body is archived verbatim in pr14-body-before-g16-20261121.md, SHA8aaaf9d244bd3f0b2f15cfb0ea283f8e7d82403d99836f25600d74d3183312df. Older audits remain untouched; concise PR updates must preserve title/base/state, and final commit/push receipt is authoritative.

49groups remain4accepted/5pending/26partial/10NI/1safety/3illegal. Next independent missing color prerequisite is alphaOff: establish its signed-fixed-percentage contract and native controls before enabling it. sysClr/hslClr,arbitrary mixtures,pie translucent plot,legend auto-key,square terminal-gap,lim4plot,180retrace/scatter and the documented native-vs-SDK overflow distinction remain explicit. No acceptance criteria lowered; not all NI implemented.
