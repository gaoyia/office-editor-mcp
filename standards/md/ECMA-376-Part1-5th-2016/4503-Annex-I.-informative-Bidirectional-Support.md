# Annex I.  (informative) Bidirectional Support

<!-- source-pages 4503-4512 -->

<!-- page 4503 -->

Annex I

4493
Annex I.
(informative)
Bidirectional Support
This Annex is informative.
I.1
Introduction
Office Open XML was designed to work with multilingual documents. It is capable of fully representing
the information needed for different languages and scripts, including bidirectional (bidi) scripts (such as
Arabic or Hebrew). Those scripts are written from right-to-left (RTL), while numbers and segments of
text in other scripts are embedded from left-to-right (LTR). The Office Open XML standard allows users
to explicitly apply and define the bidirectional characteristics of objects and text properties.
This document describes the syntax used to define these properties, going from the highest level to the
most specific—that is from the document to the character (i.e., a run) level. Note that there are no
bidirectional settings which affect the whole document.  To accomplish this, styles can be used to allow
these settings to be changed in a single location, and used throughout the content.
I.2
Shared (WordprocessingML and DrawingML)
Office Open XML provides markup which is used to provide bidirectional support within its constituent
markup languages. This markup is used identically across its constituent languages, in order to ensure
that its functionality is clearly indicated as common.
Specifically, the following properties are shared between both WordprocessingML and DrawingML, in
order to provide identical bidirectional support:
At the paragraph level, the bidi element in WordprocessingML (§17.6.1) and  the rtl attribute in
Drawing ML (§21.1.2.2.7) specifies whether the base direction of the text within the paragraph is left-to-
right or right-to-left. This element overrides the paragraph level; rules P2 and P3 of the “Unicode
Bidirectional Algorithm” (see HL1 in the Unicode Standard Annex #9).
Element
Section
Description
cs
17.3.2.7
and
21.1.2.3.1
Treats the run as complex script run regardless of the content of the run.

<!-- page 4504 -->

ECMA-376 Part 1

4494

Element
Section
Description
lang
17.3.2.20
and
21.1.2.3.9
Since Arabic and Hebrew require different behavior for equation layout, the
“lang” attribute is used to determine the resolution of the following neutral
characters: the plus sign (+), the minus sign (-), and the solidus (/).

For Arabic, these characters behave just as any other neutral character and can
be impacted by the RTL property. Whereas for Hebrew, these characters are
treated as numeral separators. Therefore, they will be resolved as EN (European
Number) when surrounded by numerals.

For example:

<w:r>
    <w:rPr>
        <w:rFonts w:hint="cs" />
        <w:rtl />
        <w:lang w:bidi="he-IL" />
    </w:rPr>
    <w:t>25-12-2007 25.12.2007 425-123-4567</w:t>
</w:r>

This shall generate the following layout:

425-123-4567 25.12.2007 25-12-2007

If lang is set to ar-SA, the text would layout as:

4567-123-425 25.12.2007 2007-12-25
rtl
17.3.2.30
and
21.1.2.2.8
Specifies the reading order of the character run to be right-to-left, this controls
the visual layout of the run. It is important to note the following:

This element should not be used with strong LTR characters. The
behavior resulting from using this element with a strong LTR character is
unspecified.

Unlike Unicode’s embedding control characters, this element cannot be
nested, instead it behaves as a toggle between LTR and RTL character
runs. Therefore, it doesn’t increase the embedding level of the character
runs beyond 0 or 1. The only exception is when numerals exist within a
RTL character run in a LTR paragraph then these numerals will implicitly
be embedding level 2.

This element is particularly useful when applications need to support the
common scenario of users toggling the method of text entry from LTR to
RTL. For example, typing characters using the English keyboard, then
switching to Arabic or Hebrew and typing additional characters. Since
many directional-neutral characters are common between these
languages, this tag helps to disambiguate the reading order of these
neutral characters using the direction of the keyboard language rather
than of the surrounding characters.

<!-- page 4505 -->

Annex I

4495
Element
Section
Description

This element provides information used to resolve the classifications of
individual characters. Characters in RTL runs will be resolved as strong
right-to-left characters or as numerals, while characters in LTR runs will
be resolved as strong left-to-right characters or as numerals. Once this is
determined, the line is displayed subject to the recommendation of the
“Unicode Bidirectional Algorithm” (see HL3 in the Unicode Standard
Annex #9) in reordering the resolved levels.

For example, to control the layout of a quote in the middle of a left-to-right
paragraph (assume the lower-case text is English and the upper-case text is
Arabic or Hebrew).

<w:r>
    <w:t>he said: “</w:t>
</w:r>
<w:r>
    <w:rPr>
        <w:rtl />
    </w:rPr>
    <w:t>IT COSTS 12.95 LIRAS</w:t>
</w:r>
<w:r>
    <w:t>”.</w:t>
</w:r>

The RTL and LTR elements resolves the classification of this run as follows:

   he said: “IT COSTS 12.95 LIRAS”.
   LLLLLLLLLLRRRRRRRRREEEEERRRRRRLL
                      NNNNN

Which generates the following levels:

   he said: “IT COSTS 12.95 LIRAS”.
   00000000001111111112222211111100

Which generates the following text layout:

   he said: “SARIL 12.95 STSOC TI”.
I.3
WordprocessingML
Within the context of a word processing document, additional bidirectional support is specifically
applicable, and is described below.
At the section level in WordprocessingML, the bidi element (see §17.6.1) specifies that the section is
right-to-left. This affects the orientation of section level properties; the layout of the content within the

<!-- page 4506 -->

ECMA-376 Part 1

4496

section is directly controlled as described below. For example, if the section has multiple columns, the
flow of text within those columns is right-to-left.
The section also specifies the rtlGutter element (see §17.6.16), which places the page gutter margin on
the right side of a group of pages; this ensure that if bound, the booklet can be read from right-to-left.
At the table level, the bidiVisual element (see §17.4.1) specifies the order in which the cells in the table
are processed; either left-to-right or right-to-left (the syntactic model of tables stores the cells logically
in the WordprocessingML, irrespective of the desired display order). This does not affect the direction of
the text within the cells, but the visual order in which the cells are displayed. Individual table cell
properties (e.g. the start and end border are applied based on the display order set by this element.
At the run level, the following properties are used to control direction:
Element
Section
Description
dir
§17.3.2.8
Specifies an increase in the embedding level as described by the Bidirectional
Unicode Algorithm (See Unicode Technical Report #9). This element can be
nested and will cause the bidi levels to increase following the direction of the
embedding. The rtl element within the embedding continues to provide
classification resolution, but at the level reached by the dir embedding.

Example: A left-to-right paragraph that has a quote in a right-to-left language,
but the quote itself includes left-to-right text. In this case, the text layout within
the quote should be read from right-to-left. Adding the “dir” element around the
quote will result in the desired layout. (Assume the lower-case text is English and
the upper-case text is Arabic or Hebrew.)

<w:r>
    <w:t>he said: “</w:t>
</w:r>
<w:dir w:val=”rtl”>
    <w:r>
        <w:rPr>
            <w:rtl />
        </w:rPr>
        <w:t>I LEAVE FOR </w:t>
    </w:r>
    <w:r>
        <w:t>united states</w:t>
    </w:r>
    <w:r>
        <w:rPr>
            <w:rtl />
        </w:rPr>
        <w:t xml:space="preserve"> TOMORROW</w:t>
    </w:r>
</w:dir>

<!-- page 4507 -->

Annex I

4497
Element
Section
Description
<w:r>
    <w:t>”.</w:t>
</w:r>

The alternating RTL and LTR elements help in resolving the classification of
characters within this run as follows:

   he said: “I LEAVE FOR united states TOMORROW”.
   LLLLLLLLLLRRRRRRRRRRRRLLLLLLLLLLLLLRRRRRRRRRLL

But the fact that the quote is enclosed in an RTL embedding; the following bidi
levels will be generated:

   he said: “I LEAVE FOR united states TOMORROW”.
   0000000000111111111111222222222222211111111100

This generates the following text layout:

   he said: “WORROMOT united states ROF EVAEL I”.
bdo
17.3.2.3
This element forces a direction override for the enclosed text as described by the
Bidirectional Algorithm (see Unicode Technical Report #9). For example, the
right-to-left override can be used to force a product part number made of mixed
English, digits, and Hebrew text written from right-to-left. (Assume the lower-
case text is English and the upper-case text is Arabic or Hebrew.)

<w:r>
    <w:t>part number: “</w:t>
</w:r>
<w:bdo w:val=”rtl”>
    <w:r>
        <w:t>ad-326D-FG</w:t>
    </w:r>
</w:bdo>

This generates the following text layout:

part number: GF-D623-da
bCs
17.3.2.2
Specifies the bold property for a complex script run of characters, this is applied
when the “rtl” element is specified on a run. It is forced when the “cs” element is
specified (see the “cs” element later in this table).
iCs
17.3.2.17
Specifies the italic property for a complex script run of characters, this is applied
when the “rtl” element is specified on a run. It is forced when the “cs” element is
specified (see the “cs” element later in this table).
szCs
17.3.2.39
Specifies the font size property for a complex script run of characters, this is
applied when the “rtl” element is specified on a run. It is forced when the “cs”

<!-- page 4508 -->

ECMA-376 Part 1

4498

Element
Section
Description
element is specified (see the “cs” element later in this table).
rFonts
17.3.2.26
Specifies the font to be used for a run of characters. The complex script font is
used to render runs with strong bidirectional characters or weak and neutrals
marked with the rtl element.
I.4
SpreadsheetML
Within the context of a spreadsheet document, additional bidirectional support is specifically applicable,
and is described below.
At the sheet view level in SpreadsheetML, the rightToLeft attribute (see §18.3.1.87) sets the sheet
direction, i.e/ whether the columns progress from left-to-right, where column A is to the far left of the
worksheet, or right-to-left, where column A is to the far right of the worksheet. This attribute does not
affect the direction of the cells within the sheet (covered below). In addition, the location of objects
within the sheet will be mirrored, but their content will not.

A
B
C

C
B
A

1
LTR cell

LTR cell
1
2
RTL cell

RTL cell

2
3

3
<... rightToLeft = “0” …>
Left-to-right spreadsheet

<... rightToLeft = “1” …>
Right-to-left spreadsheet
Note:

Objects location in the sheet were mirrored

Cell direction and alignment are not affected by this element
At the cell level in SpreadsheetML, the readingOrder attribute (see §18.8.1) indicates the baseline
direction of all the paragraphs within the cell, which can be set to left-to-right, right-to-left, or context
dependent. The latter will scan the text for the first non-whitespace character, and if it is a strong right-
to-left character it sets a right-to-left direction for the cell; otherwise, the cell direction is set as left-to-
right.
Within cell text, which is optimized for use in calculation logic, the following rules should be followed:

The bidirectional algorithm, as recommended by Unicode UAX#9, should be used to display text.

Text copied from WordprocessingML or DrawingML should be processed by an application,
which should add Unicode markers to preserve the order of text.
object
object

<!-- page 4509 -->

Annex I

4499
I.5
PresentationML
Within the context of a presentation document, additional bidirectional support is specifically
applicable, and is described below.
At the presentation level in PresentationML, the rtl attribute (see §19.2.1.26) specifies the orientation
of the editing and slide show views of the user interface, either left-to-right or right-to-left. If this
attribute is omitted, the user interface is automatically left-to-right. This attribute does not affect any
other object within the presentation.
The following example shows the effect of the RTL attribute in the normal view:
Side
Content
Region
Slide Region

Slide Region
Side
Content
Region
Bottom Content Region

Bottom Content Region
<... rtl = “0” />
Left-to-right view

<... rtl = “1” />
Right-to-left view
I.6
DrawingML
At the body level in DrawingML, the rtlCol element (see §21.1.2.1.1) is used to indicate the order of text
flwo within multiple columns (either left-to-right or right-to-left). This setting does not affect the layout
of paragraphs or text runs within the body (described below).
At the table level, the rtl attribute on the tblPr element (see §21.1.3.15) specifies the same logical
display semantics as the WordprocessingML bidiVisual element described in §I.3 above.
I.7
The Unicode Bidirectional Algorithm and Office Open XML
Office Open XML provides explicit markup to specify the classification of character runs, as well as to
apply embedding and override settings to text.  Therefore, it is important to understand how to
represent the Unicode Bidirectional Algorithm in Open XML.
Before displaying text contained within WordprocessingML documents, a consumer must resolve the
classification of characters in each line using the Unicode Bidirectional Algorithm
(http://www.unicode.org/reports/tr9/).  Specifically, the sections that detail resolving weak types and
resolving neutral types must be followed.  After applying this algorithm, the higher-level protocol
specified by the rtl element is used to set the directionality of all characters that are resolved as R, AN
and EN. Note that AN and EN that remain after applying W7 in that document will be in the right-to-left
context.

<!-- page 4510 -->

ECMA-376 Part 1

4500

This table lists the recommended way to interpret Unicode control characters in Office Open XML:
Unicode
Equivalent markup
Comment
RLO (U+202E)
<w:bdo
w:val="rtl">
These Unicode characters have equivalent should be
represented by the equivalent markup within Office Open
XML documents.  The behavior resulting from directly
embedding these discouraged characters within Office
Open XML text is unspecified.
LRO (U+202D)
<w:bdo
w:val="ltr">
RLE (U+202B)
<w:dir
w:val="rtl">
LRE (U+202A)
<w:dir
w:val="ltr">
PDF (U+202C)
</w:bdo> or
</w:dir>
RLM (U+200F)
and LRM
(U+200E)
None
These characters affect resolving the surrounding neutral
and weak types while classifying text as described above.
Once the characters are resolved, R runs should be tagged
with <w:rtl w:val=”1”/>.  It is up to the implementer to
preserve these control characters in the text.
ZWJ
(U+200D) and
ZWNJ
(U+200C)
None
These characters should be preserved as they impact the
shaping process at text rendering time.

Example #1:
The following logical text fragment:
he said:“RLEI LEAVE FOR united states TOMORROWPDF”.
would be represented using the following Office Open XML markup:
<w:r>
    <w:t>he said: “</w:t>
</w:r>
<w:dir w:val="rtl">
    <w:r>
        <w:rPr>
            <w:rtl />
        </w:rPr>
        <w:t xml:space="preserve">I LEAVE FOR </w:t>
    </w:r>
    <w:r>
        <w:t>united states</w:t>
    </w:r>

<!-- page 4511 -->

Annex I

4501
    <w:r>
        <w:rPr>
            <w:rtl />
        </w:rPr>
        <w:t xml:space="preserve"> TOMORROW</w:t>
    </w:r>
</w:dir>
<w:r>
    <w:t>”.</w:t>
</w:r>
Example #2:
The following logical text fragment:
  product number: RLOad-326D-FGPDF
would be represented using the following Office Open XML markup:
<w:r>
    <w:t>product number: “</w:t>
</w:r>
<w:bdo w:val=”rtl”>
    <w:r>
        <w:t>ad-326D-FG</w:t>
    </w:r>
</w:bdo>
This generates the following text layout:
part number: GF-D623-da.
Example #3:
The following logical text fragment:
FIRSTLRM,SECONDLRM,THIRD
would be represented using the following Office Open XML markup:
<w:r>
    <w:rPr>
        <w:rtl />
    </w:rPr>
    <w:t>FIRST</w:t>
</w:r>
<w:r>

<!-- page 4512 -->

ECMA-376 Part 1

4502

    <w:t>,</w:t>
</w:r>
<w:r>
    <w:rPr>
        <w:rtl />
    </w:rPr>
    <w:t>SECOND</w:t>
</w:r>
<w:r>
    <w:t>,</w:t>
</w:r>
<w:r>
    <w:rPr>
        <w:rtl />
    </w:rPr>
    <w:t>THIRD</w:t>
</w:r>
End informative Annex.
