# 3 Structure Examples

<!-- source-pages 364-367 -->

<!-- page 364 -->

364 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
3
Structure Examples
This section contains examples of some of the most commonly used data structures in Excel Binary
File Format files. The examples are meant to be a starting point for an implementer learning the file
format. They are not meant to cover all records in the file format.
In the following sections, the schema definition might differ from the processing rules imposed by the
application. The XSD in this specification provides a base description of the file format. The text that
introduces the XSD specifies additional restrictions that reflect actual Microsoft product behavior. For
example, the schema definition might allow for an element to be empty, null, or not present but the
behavior of the application as specified restricts the same elements to being non-empty, not null, and
present.
3.1
Slicer
This example shows a slicer attached to a native PivotTable and its associated slicer cache. The
PivotTable has the "State" and "City" fields added to the row area and the "Population" field added to
the data area. The slicer is based on the "State" field, and is currently filtering on "Washington".
The following figure shows a possible implementation of the slicer discussed in this example.

Figure 1: PivotTable and slicer
The following figure shows that the source data for the PivotTable that the slicer discussed in this
example is filtering, as depicted on a possible implementation of a worksheet.

<!-- page 365 -->

365 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026

Figure 2: Source data for the PivotTable
This example includes the entire slicer cache and slicer parts that are used in the specification of a
non-OLAP slicer.
The <i> elements inside the <items> element specify the cached non-OLAP slicer items. "Washington"
and "Oregon" are the cached non-OLAP slicer items in this example.
3.1.1 Slicer Cache Part
The following is an example slicer cache part, as specified in section 2.1.4.
 <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
 <slicerCacheDefinition xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main" name="Slicer_State"
sourceName="State">
    <pivotTables>
        <pivotTable tabId="1" name="PivotTable1"/>
    </pivotTables>
    <data>
        <tabular pivotCacheId="5">
            <items count="2">
                <i x="1"/>
                <i x="0" s="1"/>
            </items>
        </tabular>
    </data>
 </slicerCacheDefinition>
The following table describes the slicerCacheDefinition element used in this example.
Attribute name
Value
Notes
name
Slicer_State
This is the name of the slicer cache.
sourceName
State
This is the name of the associated slicer, as
specified in section 2.1.5.

The following table describes the pivotTable element used in this example.
Attribute name
Value
Notes
tabId
1
This indicates that the associated PivotTable, as
specified in [ISO/IEC29500-1:2016] section
18.10, is located on the first sheet.
name
PivotTable1
This is the name of the associated PivotTable.

<!-- page 366 -->

366 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
The following table describes the tabular element used in this example.
Attribute name
Value
Notes
pivotCacheId
5
This indicates that the identifier of the
associated PivotTable PivotCache is "5".

The following table describes the items element used in this example.
Attribute name
Value
Notes
count
2
This indicates that there are two items in the
slicer cache.

The following table describes the first i (item) element used in this example.
Attribute name
Value
Notes
x
1
This indicates that the first item in the slicer
cache is the second item in the PivotTable
PivotCache.

The following table describes the second i (item) element used in this example.
Attribute name
Value
Notes
x
0
This indicates that the second item in the slicer
cache is the first item in the PivotTable
PivotCache.
s
1
This indicates that this item is selected in the
slicer.

3.1.2 Slicer Part
The following is an example slicer part, as specified in section 2.1.5.
 <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
 <slicers xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <slicer name="State" cache="Slicer_State" caption="State" rowHeight="228600"/>
 </slicers>
The following table describes the slicer element used in this example.
Attribute name
Value
Notes
name
State
This is the name of the slicer.
cache
Slicer_State
This is the name of the slicer cache, as specified
in section 2.1.4.
caption
State
This is the caption displayed at the top of the
slicer.
rowHeight
228600
This is the height of a row in EMUs, equal to
one-fourth of an inch.

<!-- page 367 -->

367 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
