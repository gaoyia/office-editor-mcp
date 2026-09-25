# 3 Structure Examples

<!-- source-pages 95-96 -->

<!-- page 95 -->

95 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
3
Structure Examples
3.1
Glowing Text
This example shows a usage of an extended element to indicate glowing text. See section 2.2.1 for
more information. Consider the following XML, showing the complete contents of the Main Document
Part (see [ISO/IEC29500-1:2016] section 11.3.10) of a word processing document.
 <wd:document
  xmlns:wd="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
  xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
  mc:Ignorable="w14">
  <wd:body>
    <wd:p>
      <wd:r>
        <wd:rPr>
          <w14:glow w14:rad="228600">
            <w14:schemeClr w14:val="accent6">
              <w14:alpha w14:val="60000"/>
              <w14:satMod w14:val="175000"/>
            </w14:schemeClr>
          </w14:glow>
        </wd:rPr>
        <wd:t>glowing</wd:t>
      </wd:r>
      <wd:r>
        <wd:t xml:space="preserve"> text.</wd:t>
      </wd:r>
    </wd:p>
  </wd:body>
 </wd:document>
The glowing text is specified by the glow element, as a child of the rPr element (see [ISO/IEC29500-
1:2016] section 17.3.2.28). Also, the Ignorable attribute (see [ISO/IEC29500-3:2015] section 7.2)
is used to maintain compatibility with ISO/IEC-29500 implementations. The prefix w14 is specified in
the value of this attribute, which is the prefix used for the glow element.
3.2
Stylistic Sets
This example shows a usage of an extended element to indicate enabled stylistic sets and ligatures.
See section 2.2.1 for more information. Consider the following XML, showing the complete contents of
the Main Document Part (see [ISO/IEC29500-1:2016] section 11.3.10) of a word processing
document.
 <wd:document
  xmlns:wd="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
  xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
  xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
  mc:Ignorable="w14">
  <wd:body>
    <wd:p>
      <wd:r>
        <wd:rPr>
          <w14:ligatures w14:val="standardContextual"/>
          <w14:stylisticSets>
            <w14:styleSet w14:id="1"/>
            <w14:styleSet w14:id="4"/>
          </w14:stylisticSets>
        </wd:rPr>
        <wd:t>Office</wd:t>

<!-- page 96 -->

96 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
      </wd:r>
    </wd:p>
  </wd:body>
 </wd:document>
The ligatures child of the rPr element (see [ISO/IEC29500-1:2016] section 17.3.2.28) specifies that
the font makes use of standard and contextual ligatures if they are supported by the font.
Furthermore, the stylisticSets child specifies that stylistic sets 1 and 4 are enabled. Also, the
Ignorable attribute (see [ISO/IEC29500-3:2015] section 7.2) is used to maintain compatibility with
ISO/IEC-29500 implementations. The prefix w14 is specified in the value of this attribute, which is the
prefix used for the ligatures and stylisticSets elements.
