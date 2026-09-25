# Annex M.  (informative) Differences Between ECMA-376:2016 and ECMA-376:2006

<!-- source-pages 5030-5038 -->

<!-- page 5030 -->

ECMA-376 Part 1

5020

Annex M.
(informative)
Differences Between ECMA-376:2016 and
ECMA-376:2006
This annex is informative.
This annex highlights the differences between the versions of the Strict form of the Office Open XML
schemas, as defined in ECMA-376:2016 and the schemas as defined by ECMA-376:2006.
M.1
WordprocessingML
The following changes occurred to the WordprocessingML schema:

58 compatibility settings were removed.

The algorithmName, hashValue, saltValue, and spinCount attributes were added to
documentProtection (§17.15.1.29) and writeProtection (§17.15.1.93)

The allStyles, customStyles, latentStyles, stylesInUse, headingStyles, numberingStyles,
tableStyles, directFormattingOnRuns, directFormattingOnParagraphs,
directFormattingOnNumbering, directFormattingOnTables, clearFormatting,
top3HeadingStyles, and visibleStyles attributes were added to the stylePaneFormatFilter
element (§17.15.1.85)

The bdo element (§17.3.2.3) was added

The characterSet attribute was added to the charset element (§17.8.3.2)

The compatSetting element (§17.15.3.4) was added

The conformance attribute was added to document (§17.2.3)

The content model of ST_HpsMeasure (§17.18.42) was modified to allow
ST_PositiveUniversalMeasure (§22.9.2.12)

The content model of ST_OnOff (§22.9.2.7) was changed to an xsd:boolean, removing the
values on and off

The content model of ST_SignedHpsMeasure (§17.18.80) was modified to allow
ST_UniversalMeasure (§22.9.2.15)

The content model of ST_SignedTwipsMeasure (§17.18.81) was modified to allow
ST_UniversalMeasure (§22.9.2.15)

The contentPart element (§17.3.3.2) was added

The cryptProviderType, cryptAlgorithmClass, cryptAlgorithmType, cryptAlgorithmSid,
cryptSpinCount, cryptProvider, algIdExt, algIdExtSource, cryptProviderTypeExt,

<!-- page 5031 -->

Annex M

5021
cryptProviderTypeExtSource, hash, and salt attributes were removed from
documentProtection (§17.15.1.29) and writeProtection (§17.15.1.93)

The dir element (§17.3.2.8) was added

The end element (§17.4.10) was added

The end element (§17.4.11) was added

The end element (§17.4.12) was added

The end element (§17.4.13) was added

The firstRow, lastRow, firstColumn, lastColumn, noHBand, and noVBand attributes were
added to the tblLook element (§17.4.54; §17.4.55)

The firstRow, lastRow, firstColumn, lastColumn, oddVBand, evenVBand, oddHBand,
evenHBand, firstRowFirstColumn, firstRowLastColumn, lastRowFirstColumn, and
lastRowLastColumn attributes were added to the cnfStyle element (§17.3.1.8; §17.4.8;
§17.4.7)

The fldData element was removed

The following enumeration values were added to the ST_Border simple type (§17.18.2): earth3,
triangle1, triangle2, triangleCircle1, triangleCircle2, shapes1, shapes2, custom

The following enumeration values were added to the ST_CalendarType simple type (§22.9.2.1):
gregorianArabic, gregorianMeFrench, and gregorianUs.

The following enumeration values were added to the ST_Jc simple type (§17.18.44): start, end

The following enumeration values were added to the ST_NumberFormat simple type
(§17.18.59): bahtText, dollarText, custom

The following enumeration values were added to the ST_TabJc simple type (§17.18.84): start,
end

The following enumeration values were added to the ST_TextDirection simple type (§17.18.93):
tb, rl, lr, tbV, rlV, and lrV.

The following enumeration values were removed from the ST_Border simple type (§17.18.2):
tribal1, tribal2, tribal3, tribal4, tribal5, tribal6

The following enumeration values were removed from the ST_Jc simple type (§17.18.44): left,
right

The following enumeration values were removed from the ST_TabJc simple type (§17.18.84):
left, right

The following enumeration values were removed from the ST_TextDirection simple type
(§17.18.93): btLr, lrTb, lrTbV, tlLrV, tbLrV, tbRl, and tbRlV.

The fontSz attribute on the readModeInkLockDown element (§17.15.1.66) was modified to
use ST_DecimalNumberOrPercent (§17.18.11)

The format attribute was added to the numFmt element (§17.9.17)

The hdrShapeDefaults element was removed

The header element (§17.4.18) was added

The headers element (§17.4.19) was added

The id attribute was added to the left element (§17.6.7) and right element (§17.6.15)

The id attribute was added to the tc element (§17.4.65)

<!-- page 5032 -->

ECMA-376 Part 1

5022


The id, bottomLeft, and bottomRight attributes were added to the bottom element (§17.6.2)

The id, topLeft, and topRight attributes were added to the top element (§17.6.21)

The jc element (§17.4.28) was modified to use the ST_JcTable simple type (§17.18.45)

The label element (§17.5.2.19) was added

The left element was removed

The left, leftChars, right, rightChars attributes were added to the ind element (§17.3.1.12)

The longDesc element (§17.15.2.23) was added

The numberingChange element was removed

The objectEmbed element (Part1, §17.3.3.20) was added

The objectLink element (§17.3.3.21) was added

The percent attribute on the zoom element (§17.15.1.94) was modified to use
ST_DecimalNumberOrPercent (§17.18.11)

The right element was removed

The shapeDefaults element was removed

The ST_ColorSchemeIndex simple type was renamed to ST_WmlColorSchemeIndex
(§17.18.103)

The ST_DecimalNumberOrPercent (§17.18.11) simple type was added

The ST_Direction simple type (§17.18.12) was added

The ST_DocType simple type (§17.18.19) was modified to allow any xsd:string

The ST_JcTable simple type (§17.18.45) was added

The ST_LangCode simple type was removed

The ST_MailMergeDataType simple type (§17.18.54) was modified to allow any xsd:string

The ST_ObjectDrawAspect simple type (§17.18.60) was added

The ST_ObjectUpdateMode simple type (§17.18.61) was added

The ST_StyleSort simple type (§17.18.82) was added

The ST_UnqualifiedPercentage simple type (Part 4, §9.10.8) was added

The start element (§17.4.33) was added

The start element (§17.4.34) was added

The start element (§17.4.35) was added

The start element (§17.4.36) was added

The start, startChars, end, endChars attributes were added to the ind element (§17.3.1.12)

The tabIndex element (§17.5.2.41) was added

The target attribute was added to the optimizeForBrowser element (§17.15.2.33)

The tblCaption element (§17.4.40) was added

The tblDescription element (§17.4.46) was added

The title element (§17.15.2.42) was added

The uiCompat97To2003 element was removed

The val attribute was removed from the charset element (§17.8.3.2)

The val attribute was removed from the cnfStyle element (§17.3.1.8; §17.4.8; §17.4.7)

The val attribute was removed from the stylePaneFormatFilter element (§17.15.1.85)

<!-- page 5033 -->

Annex M

5023

The val attribute was removed from the tblLook element (§17.4.54; §17.4.55)

The vendorID and dllVersions attributes on the activeWritingStyle element (§17.15.1.1) was
modified to use ST_String (§22.9.2.13)
M.2
SpreadsheetML
The following changes occurred to the SpreadsheetML schema:

The algorithmName, hashValue, saltValue, and spinCount attributes were added to
sheetProtection (§18.3.1.85; §18.3.1.84), protectedRange (§18.3.1.71), sheetProtection
(§18.3.1.85), and fileSharing (§18.2.12)

The anchor element (§18.3.1.1) was added

The characterSet attribute was added to the textPr element (§18.13.12) and the
webPublishing element (§18.2.24)

The codePage attribute was removed from the textPr element (§18.13.12)

The codePage attribute was removed from the webPublishing element (§18.2.24)

The commentPr element (§18.7.5) was added

The conformance attribute was added to the workbook element (§18.2.27)

The controlPr element (§18.3.1.20) was added

The drawingHF element (§18.3.1.37) was added

The end element (§18.8.16) was added

The left element was deleted

The leftLabels attribute was removed from the dataConsolidate element (§18.3.1.29)

The legacyDrawing element was deleted

The legacyDrawingHF element was deleted

The objectPr element (§18.3.1.56) was added

The paperHeight and paperWidth attributes were added to the pageSetup element
(§18.3.1.63)

The paperHeight and paperWidth attributes were added to the pageSetup element
(§18.3.1.64)

The password attribute was removed from sheetProtection (§18.3.1.85; §18.3.1.84),
protectedRange (§18.3.1.71), and the sheetProtection elements (§18.3.1.85)

The refreshedDate attribute was removed from the pivotCacheDefinition element
(§18.10.1.67)

The refreshedDateIso attribute was added to the pivotCacheDefinition element (§18.10.1.67)

The reservationPassword attribute was removed from the fileSharing element (§18.2.12)

The right element was deleted

The Schema element (§18.16.4) now allows mixed content

The SchemaLanguage attribute was added to the schema element (§18.16.4)

The securityDescriptor attribute was removed from the protectedRange element (§18.3.1.71)

The securityDescriptor element (§18.3.1.77) was added

The shapeId attribute was added to the comment element (§18.7.3)

<!-- page 5034 -->

ECMA-376 Part 1

5024


The ST_CalendarType simple type (§22.9.2.1) now allows an enumeration value of saka

The ST_CellType simple type (§18.18.11) now allows an enumeration value of d

The ST_FileType simple type (§18.18.29) now allows enumeration values of lin and other

The ST_PivotAreaType simple type (§18.18.58) no longer allows an enumeration value of
topRight

The ST_PivotAreaType simple type (§18.18.58) now allows an enumeration value of topEnd

The ST_TextHAlign simple type (§18.18.80) was added

The ST_TextVAlign simple type (§18.18.81) was added

The ST_XmlDataType simple type (§18.18.93) was modified to allow any xsd:string

The start element (§18.8.37) was added

The startLabels attribute was added to the dataConsolidate element (§18.3.1.29)

The val and maxVal attributes were removed from the dynamicFilter element (§18.3.2.5)

The valIso and maxValIso attributes were added to the dynamicFilter element (§18.3.2.5)

The workbookPassword, workbookPasswordCharacterSet, revisionsPassword, and
revisionsPasswordCharacterSet attributes were removed from the workbookProtection
element (§18.2.29)

The workbookPasswordCharacterSet, revisionsPasswordCharacterSet,
revisionsAlgorithmName, revisionsHashValue, revisionsSaltValue, revisionsSpinCount,
workbookAlgorithmName, workbookHashValue, workbookSaltValue, and
workbookSpinCount attributes were added to the workbookProtection element (§18.2.29)
M.3
PresentationML
The following changes occurred to the PresentationML schema:

The algorithmName, hashValue, saltValue, and spinCount attributes were added to the
modifyVerifier  element (§19.2.1.19)

The cryptProviderType, cryptAlgorithmClass, cryptAlgorithmType, cryptAlgorithmSid,
cryptSpinCount, cryptProvider, algIdExt, algIdExtSource, cryptProviderTypeExt,
cryptProviderTypeExtSource, hash, and salt attributes were removed from the
modifyVerifier  element (§19.2.1.19)

The conformance attribute was added to the presentation element (§19.2.1.26)

The contentPart element was added (§19.3.1.14)

The pubBrowser attribute on the htmlPubPr element (Part 4, §11.2.1.1) was renamed target

The spid attribute was removed from the oleObj element (§19.3.2.4) and the control element
(§19.3.2.1)

The ST_HtmlPublishWebBrowserSupport simple type was removed and replaced by
xsd:string

<!-- page 5035 -->

Annex M

5025
M.4
DrawingML
M.4.1
DrawingML – Main
The following changes occurred to the DrawingML Main schema:

The builtIn attribute was removed from the snd element (§19.5.68)

The content model of ST_Coordinate (§20.1.10.16) was modified to allow
ST_UniversalMeasure (§22.9.2.15)

The content model of ST_Coordinate32 (§20.1.10.17) was modified to allow
ST_UniversalMeasure (§22.9.2.15)

The content model of ST_FixedPercentage (§20.1.10.24) was modified to allow
ST_FixedPercentage (§22.9.2.3) and remove ST_FixedPercentageDecimal

The content model of ST_Percentage (§20.1.10.40) was modified to allow ST_Percentage
(§22.9.2.9) and remove ST_PercentageDecimal

The content model of ST_PositiveFixedPercentage (§20.1.10.45) was modified to allow
ST_PositiveFixedPercentage (§22.9.2.10) and remove ST_PositiveFixedPercentageDecimal

The content model of ST_PositivePercentage (§20.1.10.46) was modified to allow
ST_PositivePercentage (§22.9.2.11) and remove ST_PositivePercentageDecimal

The contentType attribute was added to the videoFile (§20.1.3.6) and audioFile elements
(§20.1.3.2)

The header element (§21.1.3.3) was added

The headers element (§21.1.3.4) was added

The id attribute was added to the tc element (§21.1.3.16)

The rtl element (§21.1.2.2.8) was added

The ST_PresetColorVal simple type (§20.1.10.48) now allows enumeration values of: darkBlue,
darkCyan, darkGoldenrod, darkGray, darkGrey, darkGreen, darkKhaki, darkMagenta,
darkOliveGreen, darkOrange, darkOrchid, darkRed, darkSalmon, darkSeaGreen,
darkSlateBlue, darkSlateGray, darkSlateGrey, darkTurquoise, darkViolet, dkGrey,
dkSlateGrey, dimGrey, grey, lightBlue, lightCoral, lightCyan, lightGoldenrodYellow,
lightGray, lightGrey, lightGreen, lightPink, lightSalmon, lightSeaGreen,
lightSkyBlue, lightSlateGray, lightSlateGrey, lightSteelBlue, lightYellow, ltGrey,
ltSlateGrey, mediumAquamarine, mediumBlue, mediumOrchid, mediumPurple,
mediumSeaGreen, mediumSlateBlue, mediumSpringGreen, mediumTurquoise,
mediumVioletRed, slateGrey

The ST_TextFontScalePercent simple type was renamed to
ST_TextFontScalePercentOrPercentString (§20.1.10.67) and modified to allow ST_Percentage
(§22.9.2.9) and remove ST_TextFontScalePercent

The ST_TextPoint simple type (§20.1.10.74) was modified to allow ST_UniversalMeasure
(§22.9.2.15)

<!-- page 5036 -->

ECMA-376 Part 1

5026


The ST_TextSpacingPercent simple type was renamed to
ST_TextSpacingPercentOrPercentString (§20.1.10.77) and modified to allow ST_Percentage
(§22.9.2.9) and remove ST_TextSpacingPercent

The title attribute was added to the cNvPr element (§20.1.2.2.8)
M.4.2
DrawingML – Chart
The following changes occurred to the Chart schema:

The legacyDrawingHF element was removed

The paperHeight and paperWidth attributes were added to the pageSetup element
(§21.2.2.134)
M.4.3
DrawingML – Diagrams
The following changes occurred to the Diagram schema:

The ST_HorizontalAlignment simple type was renamed to ST_DiagramHorizontalAlignment
(§21.4.7.24)

The ST_TextAlignment simple type was renamed to ST_DiagramTextAlignment (§21.4.7.25)
M.4.4
DrawingML – Spreadsheet Drawing
The following changes occurred to the Spreadsheet Drawing schema:

The contentPart element (§20.5.2.12) was added
M.5
VML
All VML schemas have been removed.
M.6
Shared
M.6.1
Shared – Bibliography
The following changes occurred to the Bibliography schema:

The ST_String255 simple type was removed and replaced by ST_String (§22.9.2.13)
M.6.2
Shared – Custom Properties Variant Types
The following changes occurred to the Custom Properties Variant Types schema:

The cf element was removed

The ST_Cf simple type was removed
M.6.3
Shared – Math
The following changes occurred to the Math schema:

<!-- page 5037 -->

Annex M

5027

The ST_YAlign simple type (§22.9.2.20) now uses an enumeration value of bottom in place of
bot, and allows values inside and outside

The ST_XAlign simple type (§22.9.2.18) now allows values inside and outside

The content model of ST_OnOff (§22.9.2.7) was changed to an xsd:boolean, removing the
values on and off
M.6.4
Shared Simple Types
The following changes occurred to shared simple types:

The ST_UniversalMeasure simple type (§22.9.2.15) was added

The ST_AlgClass simple type (Part 4, §15.1.2.1) was removed

The ST_AlgType simple type (Part 4, §15.1.2.2) was removed

The ST_CryptProv simple type (Part 4, §15.1.2.4) was removed

The content model of ST_OnOff (§22.9.2.7) was changed to an xsd:boolean, removing the
values on and off

The content model of ST_TwipsMeasure (§22.9.2.14) was modified to allow
ST_PositiveUniversalMeasure (§22.9.2.12)

The ST_PositiveUniversalMeasure simple type (§22.9.2.12) was added

The ST_Percentage simple type (§22.9.2.9) was added

The ST_FixedPercentage simple type (§22.9.2.3) was added

The ST_PositivePercentage simple type (§22.9.2.11) was added

The ST_PositiveFixedPercentage simple type (§22.9.2.10) was added
M.7
Custom XML Schema References
The following changes occurred to the Custom XML Schema References schema:

The schemaLanguage attribute was added to the schema element (§23.2.1)
End informative annex.

<!-- page 5038 (empty) -->
