# 1 Introduction

<!-- source-pages 17-27 -->

<!-- page 17 -->

17 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
1
Introduction
The Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format specifies extensions to
the Office Open XML file formats described in [ISO/IEC29500-1:2016]. The extensions are specified
using conventions provided by the Office Open XML file formats described in [ISO/IEC29500-3:2015].
The extensions are a collection of structures and parts in a container that specify appropriate content,
which can include unstructured or semi-structured tables of numbers, text, or both numbers and text,
equations or functions, external data connections, charts, and images. Content in such a container is
typically organized in a grid-based layout, and often includes numeric data, structured data, and
formulas.
Sections 1.7 and 2 of this specification are normative. All other sections and examples in this
specification are informative.
1.1
Glossary
This document uses the following terms:
add-in: Supplemental functionality that is provided by an external application or macro to extend
the capabilities of an application.
adjacent cell: A cell that is in the same row as and adjoins the current cell in a worksheet.
aggregation function: A function, such as sum or average, that appears in the total row of a
table and is used to summarize data.
attribute: A characteristic of some object or entity, typically encoded as a name/value pair.
Augmented Backus-Naur Form (ABNF): A modified version of Backus-Naur Form (BNF),
commonly used by Internet specifications. ABNF notation balances compactness and simplicity
with reasonable representational power. ABNF differs from standard BNF in its definitions and
uses of naming rules, repetition, alternatives, order-independence, and value ranges. For more
information, see [RFC5234].
AutoComplete: A feature that suggests text automatically based on the first few characters that a
user types.
AutoFilter: A mechanism that can be used to filter tabular data based on user-defined criteria
such as values, strings, and formatting.
binary large image or picture (BLIP): A binary data structure that stores information about a
metafile image or bitmap picture.
Boolean: An operation or expression that can be evaluated only as either true or false.
bottom N filter: A filter that matches the smallest numerical values or oldest time and date values
in a specified range.
calculate: The process by which computations in a workbook are performed.
calculated column: A column in a table that contains a formula that is copied automatically to
each record in the column.
cell: A box that is formed by the intersection of a row and a column in a worksheet or a table. A
cell can contain numbers, strings, and formulas, and various formats can be applied to that
data.
Cell Control: A user interactable graphic representation of the data inside the specified cell.

<!-- page 18 -->

18 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
cell error value: Any of a number of special values that are returned as a result of an
unsuccessful formula calculation.
cell reference: A set of coordinates that a cell occupies on a worksheet. For example, "B3" is the
reference of a cell that appears at the intersection of column "B" and row "3".
cell value: The text or numeric content of a cell, or the results of a formula. A cell value does not
include a formula expression, cell formatting, or other metadata.
character set: A mapping between the characters of a written language and the values that are
used to represent those characters to a computer.
color scale: A specific range of colors that is used to give additional meaning to data by assigning
certain values to colors in the spectrum.
column: A single set of data that is displayed vertically in a worksheet or a table.
comment: An annotation that is associated with a cell, text, or other object to provide context-
specific information or reviewer feedback.
comment thread: A collection of annotations that is associated with a cell to provide context-
specific information, viewer feedback and collaborative discussion, which is made of a main
comment and its replies, if there are any.
condition: A logical expression comparing one or more properties in all incoming Message objects
against a set of clauses. This logical expression can evaluate to TRUE or FALSE.
conditional formatting: A mechanism that changes the appearance of a user interface element
based on the evaluation of a rule or expression.
control: A graphical user interface object that users interact with when working with applications,
forms, documents, webpages, and other types of files.
credential: Previously established, authentication data that is used by a security principal to
establish its own identity. When used in reference to the Netlogon Protocol, it is the data that is
stored in the NETLOGON_CREDENTIAL structure.
cube function: A function that is used to extract and display Online Analytical Processing (OLAP)
data sets and values.
custom filter: A filter that contains preconfigured expressions in which users can optionally enter
a string to filter data.
custom list: A user-defined list or enumeration that can be used to sort data in a worksheet.
data bar: A graphical representation of cell content as a bar graph.
data connection: A collection of information, such as the type and location, that defines how to
connect to an external data source, such as a database, web service, SharePoint list, or XML
file.
data marker: A customizable symbol or shape that identifies a data point on a line, scatter, or
radar chart. A data marker can be formatted with various sizes and colors.
data point: A representation of a PivotTable item in a PivotTable data field contained in a
PivotChart report.
data provider: A known data source that is specific to a target type and that provides data to a
collector type.

<!-- page 19 -->

19 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
data source: A database, web service, disk, file, or other collection of information from which data
is queried or submitted. Supported data sources vary based on application and data provider.
data table: A range of cells that is designated to perform what-if analysis for formulas, based on
various input values.
data type: A property of a field that defines the kind of data that is stored in the field, or defines
the kind of data returned by an expression when the expression is evaluated.
data validation: The process of testing the accuracy of data; a set of rules that specify the type
and range of data that users can enter.
defined name: A word or string of characters in a formula that represents a cell, range of cells,
formula, or constant value.
descendant: A member that is below the current member in a hierarchy.
dimension: A structural attribute of a cube, which is an organized hierarchy of categories (levels)
that describe data in a fact table. These categories typically describe a similar set of members
upon which the user bases an analysis.
display folder: A folder into which attributes, measures, calculated members, and key
performance indicators can be organized to facilitate browsing.
drillthrough: A query that is used to retrieve individual records that were used to calculate an
aggregate value.
dynamic array: An array resulting from a formula whose dimensions can change when the
formula is evaluated.
email address: A string that identifies a user and enables the user to receive Internet messages.
English Metric Unit (EMU): A measurement in computer typography. There are 635 EMUs per
twip, 6,350 EMUs per half-point, 12,700 EMUs per point, and 914,400 EMUs per inch. These
units are used to translate on-screen layouts to printed layouts for specified printer hardware.
external code service: An external service responsible for executing code and returning result or
error.
feature property bag: A set of GLOSSARY: [key value pairs (KVP)]. A feature property bag can
reference other feature property bags.
field: An element or attribute in a data source that can contain data.
filter: A mechanism by which a set of data is scoped to display only those entries that meet
specified logical criteria.
formula: A logical equation or function that produces a result in a spreadsheet application.
future function: A function that can be written to but is not implemented in a file.
globally unique identifier (GUID): A term used interchangeably with universally unique
identifier (UUID) in Microsoft protocol technical documents (TDs). Interchanging the usage of
these terms does not imply or require a specific algorithm or mechanism to generate the value.
Specifically, the use of this term does not imply or require that the algorithms described in
[RFC4122] or [C706] have to be used for generating the GUID. See also universally unique
identifier (UUID).
hash: A fixed-size result that is obtained by applying a one-way mathematical function, which is
sometimes referred to as a hash algorithm, to an arbitrary amount of data. If the input data

<!-- page 20 -->

20 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
changes, the hash also changes. The hash can be used in many operations, including
authentication and digital signing.
Help topic identifier: A unique identifier for an article that contains Help content.
hidden: A condition of an object that prevents it from being displayed in rendered output.
hierarchy: A logical tree structure that organizes the members of a dimension such that each
member has one parent member and zero or more child members.
icon: A graphical image used to supplement alphanumeric text in the visual identification of an
object on a computer monitor. Icons are typically small, relative to the size of the area on
which they are displayed.
icon set: A collection of icons that can be used to comment and classify data into categories.
ink: A process of entering text in handwritten form. Instead of converting handwritten text to
typed text, ink is converted to an object and displayed exactly as it was written.
Input Method Editor (IME): An application that is used to enter characters in written Asian
languages by using a standard 101-key keyboard. An IME consists of both an engine that
converts keystrokes into phonetic and ideographic characters and a dictionary of commonly used
ideographic words.
key value pair (KVP): A set of two linked data items: a key that is an identifier for some data
item, and a value that is a value associated with the data item for the identifier represented by
the key.
left-to-right: A reading order in which characters in words are read from left to right, and words
are read from left to right in sentences.
locale: A collection of rules and data that are specific to a language and a geographical area. A
locale can include information about sorting rules, date and time formatting, numeric and
monetary conventions, and character classification.
localize: The process of adapting an application or documentation, including text and non-text
elements, to meet the language, cultural, and political expectations and requirements of a
specific geographic country or region.
locked: The condition of a cell, worksheet, or other object that restricts edits or modifications to it
by users.
MDX unique name: A unique identifier for a multidimensional expression (MDX) member or value
in a given Online Analytical Processing (OLAP) cube, for example "[Customer].[Customer
Geography].[Country].&[Australia]".
measure: In a cube, a set of values that are typically numeric and are based on a column in the
fact table of the cube. Measures are the central values that are aggregated and analyzed.
mention: A part of a threaded comment that provides a reference to a person's information.
Multidimensional Expressions (MDX): A syntax that is used for defining multidimensional
objects, and for querying and manipulating multidimensional data.
named sheet view: A named set of sort and/or filter conditions that can be easily restored onto a
given sheet.
nsvfilter: A set of sort and/or filter conditions associated with a single autofilter.  Part of a named
sheet view.

<!-- page 21 -->

21 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Object Linking and Embedding (OLE): A technology for transferring and sharing information
between applications by inserting a file or part of a file into a compound document. The inserted
file can be either embedded or linked. See also embedded object and linked object.
object model: A collection of object-oriented APIs that represent data structures and are designed
to promote software interoperability.
OLAP All level: An optional level at the top of a hierarchy. It typically contains an OLAP All
member that represents an aggregation of all of the lower-level members of that hierarchy.
OLAP All member: A multidimensional expression (MDX) that evaluates a hierarchy and returns a
set that contains all of the members of the specified hierarchy.
OLAP allocation: An operation in which the values for members at lower levels in an OLAP
hierarchy are changed based on changes to values for members at higher levels in that
hierarchy.
OLAP hierarchy: An attribute hierarchy or a user-defined hierarchy in a data structure. By default,
each dimension attribute has an attribute hierarchy. A user-defined hierarchy is a set of related
attribute hierarchies that is used to facilitate browsing an OLAP cube.
OLAP level: Within an OLAP hierarchy, a set of data that is organized into a lower or higher level
of detail, such as Year, Quarter, Month, and Day levels in a Time hierarchy.
OLAP measure: A set of numeric values in an OLAP cube that is used in aggregation and analysis.
OLAP member: An item that is in an OLAP level. For example, a Canada member in a Country
level of a Geography hierarchy.
OLAP named set: A collection of OLAP tuples that have the same dimensionality. Also referred
to as OLAP set.
OLAP subselect: The ability to execute multiple SELECT commands in a FROM clause that is inside
a multidimensional expression (MDX) statement.
OLAP tuple: An ordered collection of members that are from different dimensions of an OLAP
cube. A single member is a special case of a tuple.
OLAP weight expression: A multidimensional expression (MDX) that is used to apply and allocate
modified values to an Online Analytical Processing (OLAP) provider. It typically returns a
decimal value between "0" and "1".
OLE DB: A set of interfaces that are based on the Component Object Model (COM) programming
model and expose data from a variety of sources. These interfaces support the amount of
Database Management System (DBMS) functionality that is appropriate for a data store and
they enable a data store to share data.
Online Analytical Processing (OLAP): A technology that uses multidimensional structures to
provide access to data for analysis. The source data for OLAP is stored in data warehouses in a
relational database. See also cube.
PivotTable: An interactive table that summarizes large amounts of data from various sources by
using format and calculation methods. Row and column headings can be rotated to view
different summaries of the source data, filter the data, or display detail data for specific areas.
PivotTable area: A logical zone that is used to organize data in a PivotTable report.
PivotTable data field: A PivotTable field that is contained in the PivotTable area where data
values are shown.

<!-- page 22 -->

22 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
PivotTable field: An element in a PivotTable report that represents a logical collection of data. A
PivotTable field can represent a dimension hierarchy or actual data values.
PivotTable field list: A user interface element that displays a list of all of the fields in a PivotTable
report. It can be used to populate a PivotTable report and to manipulate the fields.
pixel: A discrete unit of display on a computer display device.
plot area: A portion of a chart area that contains the plotted data and axes.
point: A unit of measurement for fonts and spacing. A point is equal to 1/72 of an inch.
protection: A mechanism that helps restrict users from making unwanted changes to the data or
structure of a workbook.
Python environment: A computational environment used to execute a Python script.
Python script: executable code written in the Python programming language.
query table: A two-dimensional table that presents data from an external data source.
range: An addressable region that is in a workbook. A range typically consists of zero or more cells
and represents a single, contiguous rectangle of cells on a single sheet.
rich array: A type of rich value data that is a two-dimensional array of values.
rich value: A complex representation of data that contains a rich value type, a rich value
fallback, and a collection of key value pairs (KVP). A rich value can exist as the value of a
cell or nested as the value in a key value pair.
rich value data: The value part of the key value pair (KVP).
rich value fallback: A value that will be used for any feature that does not know how to interpret
a given rich value type.
rich value key: The name of the key in the key value pair (KVP).
rich value key flag: A rich value flag assigned to a particular rich value key. Each flag is
assigned a Boolean value that indicates particular behaviors for the key.
rich value type: A collection of required key value pairs (KVP) and key flag definitions.
right-to-left: A reading and display order that is optimized for right-to-left languages.
root element: The top-level element in an XML document. It contains all other elements and is
not contained by any other element, as described in [XML].
row: A single set of data that is displayed horizontally in a worksheet or a table.
salt: An additional random quantity, specified as input to an encryption function that is used to
increase the strength of the encryption.
security descriptor: A data structure containing the security information associated with a
securable object. A security descriptor identifies an object's owner by its security identifier
(SID). If access control is configured for the object, its security descriptor contains a
discretionary access control list (DACL) with SIDs for the security principals who are allowed or
denied access. Applications use this structure to set and query an object's security status. The
security descriptor is used to guard access to an object as well as to control which type of
auditing takes place when the object is accessed. The security descriptor format is specified in
[MS-DTYP] section 2.4.6; a string representation of security descriptors, called SDDL, is
specified in [MS-DTYP] section 2.5.1.

<!-- page 23 -->

23 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
selected: The condition of a set of items that has focus in a workbook.
service: A process or agent that is available on the network, offering resources or services for
clients. Examples of services include file servers, web servers, and so on.
shape: A collection of qualifiers, such as names, and quantifiers, such as coordinates, that is used
to represent a geometric object. A shape can be contained in a document, file structure, run-
time structure, or other medium.
sheet: A part of an Excel workbook. There are four types of sheets: worksheet, macro sheet,
dialog sheet, and chart sheet. Multiple sheets are stored together within a workbook.
sort: A process that arranges cells in ascending or descending order, based on cell content.
sort condition: A condition that determines how to sort cells in a range.
sort order: A specific arrangement of cells that is based on cell content. The order can be
ascending or descending.
source data: The data that is used as the basis for charts, PivotTable reports, and other data
visualization features.
sparkline: A miniature chart that can be inserted into text or embedded in a cell on a worksheet to
illustrate highs, lows, and trends in data.
spreadsheet data model: A local Online Analytical Processing (OLAP) storage of data used
by a spreadsheet application.
style: A set of formatting options that is applied to text, tables, charts, and other objects in a
document.
supporting property bag: A set of GLOSSARY: [key value pairs (KVP)]. A supporting property
bag can exist as the value of a rich value or nested as the value in another supporting property
bag.
supporting property bag array: A type of supporting property bag data that is a one-
dimensional array of values.
supporting property bag data: The value part of the key value pair (KVP).
supporting property bag key: The name of the key in a key value pair (KVP).
table: A list that is defined in a workbook.
threaded comment: A comment in a comment thread. It can be a main comment or a reply.
top N filter: A filter that matches the top or bottom N items or N% of items in a specified column.
Unicode: A character encoding standard developed by the Unicode Consortium that represents
almost all of the written languages of the world. The Unicode standard [UNICODE5.0.0/2007]
provides three forms (UTF-8, UTF-16, and UTF-32) and seven schemes (UTF-8, UTF-16, UTF-16
BE, UTF-16 LE, UTF-32, UTF-32 LE, and UTF-32 BE).
Uniform Resource Locator (URL): A string of characters in a standardized format that identifies
a document or resource on the World Wide Web. The format is as specified in [RFC1738].
unique identifier (UID): A pair consisting of a GUID and a version sequence number to identify
each resource uniquely. The UID is used to track the object for its entire lifetime through any
number of times that the object is modified or renamed.
workbook: A container for a collection of sheets.

<!-- page 24 -->

24 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
worksheet: A single logical container for a set of tabular data and other objects in a workbook.
XML: The Extensible Markup Language, as described in [XML1.0].
XML namespace: A collection of names that is used to identify elements, types, and attributes in
XML documents identified in a URI reference [RFC3986]. A combination of XML namespace and
local name allows XML documents to use elements, types, and attributes that have the same
names but come from different sources. For more information, see [XMLNS-2ED].
XML schema definition (XSD): The World Wide Web Consortium (W3C) standard language that
is used in defining XML schemas. Schemas are useful for enforcing structure and constraining
the types of data that can be used validly within other XML documents. XML schema definition
refers to the fully specified and currently recommended standard for use in authoring XML
schemas.
XOR obfuscation: A type of file encryption that helps protect private data by using an exclusive or
bitwise operation. This is done by adding a mathematical expression that prevents a simple
reverse-engineering process.
zoom level: The degree to which a portion of an image, document, or other screen object is made
to appear closer or farther away relative to its default appearance. This value is usually
expressed as a percentage of the default appearance.
MAY, SHOULD, MUST, SHOULD NOT, MUST NOT: These terms (in all caps) are used as defined
in [RFC2119]. All statements of optional behavior use either MAY, SHOULD, or SHOULD NOT.
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
[CSS-Level2-2009] World Wide Web Consortium, "Cascading Style Sheets Level 2 Revision 1 (CSS
2.1) Specification", W3C Candidate Recommendation 08 September 2009,
http://www.w3.org/TR/2009/CR-CSS2-20090908/
[ISO/IEC29500-1:2016] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 1: Fundamentals and Markup Language Reference",
ISO/IEC 29500-1:2016, https://www.iso.org/standard/71691.html
[ISO/IEC29500-3:2015] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 3: Markup Compatibility and Extensibility",
https://www.iso.org/standard/65533.html
[ISO/IEC29500-4:2016] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 4: Transitional Migration Features",
https://www.iso.org/standard/71692.html
[MS-DTYP] Microsoft Corporation, "Windows Data Types".
[MS-ODRAWXML] Microsoft Corporation, "Office Drawing Extensions to Office Open XML Structure".

<!-- page 25 -->

25 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
[MS-OFFCRYPTO] Microsoft Corporation, "Office Document Cryptography Structure".
[MS-OSHARED] Microsoft Corporation, "Office Common Data Types and Objects Structures".
[MS-OWEXML] Microsoft Corporation, "Office Web Extensibility Extensions to Office Open XML
Structure Specification".
[MS-XLDM] Microsoft Corporation, "Spreadsheet Data Model File Format".
[MS-XLSB] Microsoft Corporation, "Excel (.xlsb) Binary File Format".
[RFC2119] Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC
2119, March 1997, https://www.rfc-editor.org/info/rfc2119
[RFC3066] Alvestrand, H., "Tags for the Identification of Languages", BCP 47, RFC 3066, January
2001, https://www.rfc-editor.org/info/rfc3066
[RFC5234] Crocker, D., Ed., and Overell, P., "Augmented BNF for Syntax Specifications: ABNF", STD
68, RFC 5234, January 2008, https://www.rfc-editor.org/info/rfc5234
[RFC5646] Phillips, A, and Davis, M., "Tags for Identifying Languages", BCP 47, RFC 4646, September
2006, https://www.rfc-editor.org/info/rfc5646
[UNICODE5.1] The Unicode Consortium, "The Unicode Standard, Version 5.1.0, defined by: The
Unicode Standard, Version 5.0 (Boston, MA, Addison-Wesley, 2007. ISBN 0-321-48091-0), as
amended by Unicode 5.1.0", http://www.unicode.org/versions/Unicode5.1.0/
[W3C-XML] Bray, T., Paoli, J., Sperberg-McQueen, C.M., Maler, E., Yergeau, F., Eds., "Extensible
Markup Language (XML) 1.1 (Second Edition)", W3C Recommendation, August 2006,
http://www.w3.org/TR/2006/REC-xml11-20060816/
[XMLNS] Bray, T., Hollander, D., Layman, A., et al., Eds., "Namespaces in XML 1.0 (Third Edition)",
W3C Recommendation, December 2009, https://www.w3.org/TR/2009/REC-xml-names-20091208/
[XMLSCHEMA1/2] Thompson, H., Beech, D., Maloney, M., and Mendelsohn, N., Eds., "XML Schema
Part 1: Structures Second Edition", W3C Recommendation, October 2004,
https://www.w3.org/TR/2004/REC-xmlschema-1-20041028/
[XMLSCHEMA2/2] Biron, P., and Malhotra, A., Eds., "XML Schema Part 2: Datatypes Second Edition",
W3C Recommendation, October 2004, https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/
1.2.2 Informative References
[ISO/IEC29500-2:2012] ISO/IEC, "Information technology -- Document description and processing
languages -- Office Open XML File Formats -- Part 2: Open Packaging Conventions", ISO/IEC 29500-
2:2012, http://www.iso.org/iso/home/store/catalogue_ics/catalogue_detail_ics.htm?csnumber=61796
[MS-AZOD] Microsoft Corporation, "Authorization Protocols Overview".
1.3
Structure Overview (Synopsis)
This document specifies spreadsheet-related extensions to Office Open XML file formats as described
in [ISO/IEC29500-1:2016]. Workbook data is contained in a ZIP package conforming to the Open
Packaging Conventions as described in [ISO/IEC29500-2:2012]. Individual files stored in the ZIP
package, called parts [ISO/IEC29500-1:2016], contain information about the content and structure of
a workbook, including workbook data, such as worksheet definitions. Some parts [ISO/IEC29500-
1:2016]) store information by using XML and other parts [ISO/IEC29500-1:2016] store information
by using binary data. The extensions specified in this document use the conventions specified by

<!-- page 26 -->

26 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
[ISO/IEC29500-3:2015], and they are both structures within parts specified by [ISO/IEC29500-
1:2016] and extension parts within the package described by [ISO/IEC29500-2:2012].
1.4
Relationship to Protocols and Other Structures
This file format is a specified set of extensions to Office Open XML SpreadsheetML, specified in
[ISO/IEC29500-1:2016]. This specification is dependent on the structures and concepts defined in the
following references:

[ISO/IEC29500-1:2016] for baseline SpreadsheetML persistence format.

[ISO/IEC29500-2:2012] for open packaging conventions.

[ISO/IEC29500-3:2015] for markup compatibility and extensibility.

[ISO/IEC29500-4:2016] for backward compatibility considerations.

[MS-OFFCRYPTO] for the persistence format for document signing, information rights
management, document encryption, and obfuscation.

[MS-ODRAWXML] for the persistence format for shapes.
1.5
Applicability Statement
This document specifies a persistence format for extensions to Office Open XML file formats as
described in [ISO/IEC29500-1:2016] SpreadsheetML documents. The extensions specified in this
document allow for expressing additional content and properties, and are not applicable as a stand-
alone file format. Each structure specified in this document is integrated with SpreadsheetML
documents as described in [ISO/IEC29500-1:2016] in a particular way, as specified in the section for
that structure. All structures are integrated into SpreadsheetML documents in such a way that
maintains compatibility with implementations of the Office Open XML file formats as described in
[ISO/IEC29500-1:2016].
The extensions specified in this document do not require any other extensions to be used, and do not
prohibit any other extensions from being used in the same document.
This persistence format provides interoperability with applications that create or read documents
conforming to this structure.
1.6
Versioning and Localization
This document covers versioning issues in the following areas:

Structure Versions: There is only one version of Excel Extensions to the Office Open XML
SpreadsheetML File Format.

Localization: The CT_ProtectedRange structure contains fields that specify locale-dependent
meaning.
The Security Considerations section also specifies processes and data that are locale-dependent. See
each structure and section description for more information.
1.7
Vendor-Extensible Fields
Specified in this document are extensions, using conventions described in [ISO/IEC29500-3:2015],
both as structures within parts described in [ISO/IEC29500-1:2016] and as extension parts within the
package described in [ISO/IEC29500-2:2012]. Implementations are not required to preserve or

<!-- page 27 -->

27 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
remove additional parts when modifying an existing document. See [ISO/IEC29500-3:2015] for more
information.
