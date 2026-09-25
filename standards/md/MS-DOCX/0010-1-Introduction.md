# 1 Introduction

<!-- source-pages 10-13 -->

<!-- page 10 -->

10 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
1
Introduction
This document specifies elements and attributes that extend the XML vocabulary for representing word
processing documents specified in the ISO/IEC-29500 specification. The new elements and attributes
are presented using the extensibility mechanisms provided by the ISO/IEC-29500 specification.
Sections 1.7 and 2 of this specification are normative. All other sections and examples in this
specification are informative.
1.1
Glossary
This document uses the following terms:
base64 encoding: A binary-to-text encoding scheme whereby an arbitrary sequence of bytes is
converted to a sequence of printable ASCII characters, as described in [RFC4648].
cyclic redundancy check (CRC): An algorithm used to produce a checksum (a small, fixed
number of bits) against a block of data, such as a packet of network traffic or a block of a
computer file. The CRC is a broad class of functions used to detect errors after transmission or
storage. A CRC is designed to catch random errors, as opposed to intentional errors. If errors
might be introduced by a motivated and intelligent adversary, a cryptographic hash function has
to be used instead.
entity: An instance of an EntityType element that has a unique identity and an independent
existence. An entity is an operational unit of consistency.
EntityInstance: A set of Field values that have a unique identity that represents a specific
instance of an Entity, and are stored in a line-of-business (LOB) system.
EntityInstanceId: A set of Field values of an EntityInstance that collectively and uniquely identify
an EntityInstance in a line-of-business (LOB) system.
external content type: A type of DataClass object that is stored in a line-of-business (LOB)
system and whose instances have a persistent EntityInstanceId. Also referred to as Entity.
field: An element or attribute in a data source that can contain data.
follow-up: A note that a user leaves for further review later. Follow-ups can be used by an app to
display in a special visual format, to provide navigation, or to provide special behaviors such as
reminders.
LobSystemInstance: A type of MetadataObject that represents a specific deployed instance of a
line-of-business (LOB) system, as represented by a LobSystem. LobSystemInstances are
contained by LobSystems. LobSystemInstance Properties describe how to connect to an instance
of the LobSystem that contains them by providing information such as the server name,
connection string, and authentication mode.
Unicode: A character encoding standard developed by the Unicode Consortium that represents
almost all of the written languages of the world. The Unicode standard [UNICODE5.0.0/2007]
provides three forms (UTF-8, UTF-16, and UTF-32) and seven schemes (UTF-8, UTF-16, UTF-16
BE, UTF-16 LE, UTF-32, UTF-32 LE, and UTF-32 BE).
MAY, SHOULD, MUST, SHOULD NOT, MUST NOT: These terms (in all caps) are used as defined
in [RFC2119]. All statements of optional behavior use either MAY, SHOULD, or SHOULD NOT.

<!-- page 11 -->

11 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
1.2
References
Links to a document in the Microsoft Open Specifications library point to the correct section in the
most recently published version of the referenced document. However, because individual documents
in the library are not updated at the same time, the section numbers in the documents may not
match. You can confirm the correct section numbering by checking the Errata.
1.2.1 Normative References
We conduct frequent surveys of the normative references to assure their continued availability. If you
have any issue with finding a normative reference, please contact dochelp@microsoft.com. We will
assist you in finding the relevant information.
[ECMA-376] ECMA International, "Office Open XML File Formats", https://www.ecma-
international.org/publications-and-standards/standards/ecma-376/
[ISO/IEC-14496-22] International Organization for Standardization, "Information technology -- Coding
of audio-visual objects -- Part 22: Open Font Format", 2007,
http://www.iso.org/iso/iso_catalogue/catalogue_tc/catalogue_detail.htm?csnumber=43466
[ISO/IEC29500-1:2016] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 1: Fundamentals and Markup Language Reference",
ISO/IEC 29500-1:2016, https://www.iso.org/standard/71691.html
[ISO/IEC29500-3:2015] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 3: Markup Compatibility and Extensibility",
https://www.iso.org/standard/65533.html
[ISO/IEC29500-4:2016] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 4: Transitional Migration Features",
https://www.iso.org/standard/71692.html
[MS-DOC] Microsoft Corporation, "Word (.doc) Binary File Format".
[MS-DTYP] Microsoft Corporation, "Windows Data Types".
[MS-ODRAWXML] Microsoft Corporation, "Office Drawing Extensions to Office Open XML Structure".
[MS-OREACTXML] Microsoft Corporation, "Office Reaction Extensions to Office Open XML Structure".
[MS-OSHARED] Microsoft Corporation, "Office Common Data Types and Objects Structures".
[MS-OWEXML] Microsoft Corporation, "Office Web Extensibility Extensions to Office Open XML
Structure Specification".
[RFC2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC
2119, March 1997, https://www.rfc-editor.org/info/rfc2119
[RFC3986] Berners-Lee, T., Fielding, R., and Masinter, L., "Uniform Resource Identifier (URI): Generic
Syntax", STD 66, RFC 3986, January 2005, https://www.rfc-editor.org/info/rfc3986
[XMLSCHEMA1/2] Thompson, H., Beech, D., Maloney, M., and Mendelsohn, N., Eds., "XML Schema
Part 1: Structures Second Edition", W3C Recommendation, October 2004,
https://www.w3.org/TR/2004/REC-xmlschema-1-20041028/
[XMLSCHEMA2/2] Biron, P., and Malhotra, A., Eds., "XML Schema Part 2: Datatypes Second Edition",
W3C Recommendation, October 2004, https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/

<!-- page 12 -->

12 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
1.2.2 Informative References
None.
1.3
Structure Overview (Synopsis)
The structures specified in this format provide an extended XML vocabulary for a word processing
document. The extended elements and attributes allow the format to indicate further information
about a document, or to indicate content and formatting of parts of the document beyond the
elements and attributes specified in the Office Open XML File Formats specification. Because these
elements and attributes are meant as extensions, their intent and usage varies.
The new elements and attributes specified in this format come in six groups. The first group extends
the vocabulary for describing formatting properties of text by adding elements for specifying text
effects such as shadow, glow, reflection, and also by adding elements for specifying typographical
properties such as ligatures or how numeral spacing is displayed. See section 2.2.1 for more
information.
The second group extends the settings that are applied to a word processing document by adding two
settings to govern how images are saved, and two settings used when the document is authored by
multiple authors. See section 2.2.2 for more information.
The third group of extensions provides for specifying four more kinds of structured document tags and
altering the appearance of structured document tags. See section 2.2.3 for more information.
The fourth group of extensions specifies three additional attributes to appear on paragraph, section, or
table rows. These attributes provide for uniquely identifying paragraphs or table rows within a
document part, or provide information about presence of spelling mistakes within a paragraph, or
provide formatting information for the layout of footnotes in a section. See section 2.2.4 for more
information.
The fifth group specifies eight new elements that can be used when conflicting edits are present in a
document that is authored by multiple authors. See section 2.2.5 for more information.
The sixth group specifies a new attribute to appear on picture and embedded objects to provide an
identifier for those objects. See section 2.2.6 for more information.
Section 2.2.7 specifies a new value that extends the types of calendars that can be used in a word
processing document.
Section 2.2.8  specifies a new element for specifying formatting information for the layout of footnotes
in a document section.  See section 2.5.1.12 for more information.
Section 2.3 specifies an extension of the set of values to be used when specifying compatibility
settings of the document. Section 2.4 specifies an extension of the set of values to be used when
specifying numbering formats.
The extensions specified in this format are integrated into ISO/IEC-29500 by means of the Markup
Compatibility and Extensibility features as specified by ISO/IEC-29500. Specifically, the Ignorable
attribute and the AlternateContent element are used to maintain compatibility with ISO/IEC-29500
implementations when integrating the extensions from this format. Using these extensions as
specified in this document will result in a word processing document conformant to ISO/IEC-29500.
1.4
Relationship to Protocols and Other Structures
The structures described here are incorporated into word processing documents as described in
[ISO/IEC29500-1:2016] using the Markup Compatibility and Extensibility features as described in
[ISO/IEC29500-3:2015]. The global elements specified in this format appear as optional child

<!-- page 13 -->

13 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
elements of certain elements described in [ISO/IEC29500-1:2016] as described in section 2.2 of this
document.
The global attributes specified in this format appear as optional attributes on certain elements
described in [ISO/IEC29500-1:2016] as described in section 2.2 of this document.
Some of the elements, attributes, simple types and complex types here also refer to complex or
simple types described in [ISO/IEC29500-1:2016].
1.5
Applicability Statement
This document specifies a set of structures to extend the XML vocabulary of ISO/IEC-29500 word-
processing documents. The extensions specified in this document allow for expressing additional
document content and properties, and are not applicable as a stand-alone file format. Each structure
specified in this document is integrated with ISO/IEC-29500 word-processing documents in a
particular way, as specified at the description of that structure. All structures are integrated into word
processing documents in a way that maintains compatibility with ISO/IEC-29500 implementations.
The extensions specified in this document do not require any other extensions to be used, and do not
prohibit any other extensions from being used in the same document.
1.6
Versioning and Localization
None.
1.7
Vendor-Extensible Fields
None.
