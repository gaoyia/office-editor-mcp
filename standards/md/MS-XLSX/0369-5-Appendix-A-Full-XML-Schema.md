# 5 Appendix A: Full XML Schema

<!-- source-pages 369-417 -->

<!-- page 369 -->

369 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
5
Appendix A: Full XML Schema
For ease of implementation, this section provides the full W3C XML schemas for the new elements,
attributes, complex types, and simple types specified in the earlier sections. Any schema references to
namespaces included in Office Open XML file formats as described in [ISO/IEC29500-1:2016] refer
specifically to the transitional schemas as described in [ISO/IEC29500-4:2016].
For ease of implementation, the following sections provide the full XML schema for this protocol.
Schema name
Prefix
Section
http://schemas.microsoft.com/office/excel/2006/main

section 5.1
http://schemas.microsoft.com/office/drawing/2010/slicer

section 5.2
http://schemas.microsoft.com/office/spreadsheetml/2010/11/main

section 5.3
http://schemas.microsoft.com/office/spreadsheetml/2009/9/main

section 5.4
http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac

section 5.5
http://schemas.microsoft.com/office/spreadsheetml/2011/1/ac

section 5.6
http://schemas.microsoft.com/office/drawing/2012/timeslicer

section 5.7
http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing

section 5.8
http://schemas.microsoft.com/office/spreadsheetml/2010/11/ac

section 5.9
http://schemas.microsoft.com/office/spreadsheetml/2014/11/main

section
5.10
http://schemas.microsoft.com/office/spreadsheetml/2015/02/main

section
5.11
http://schemas.microsoft.com/office/spreadsheetml/2016/revision10

section
5.12
http://schemas.microsoft.com/office/spreadsheetml/2016/revision9

section
5.13
http://schemas.microsoft.com/office/spreadsheetml/2016/revision6

section
5.14
http://schemas.microsoft.com/office/spreadsheetml/2014/revision

section
5.15
http://schemas.microsoft.com/office/spreadsheetml/2015/revision2

section
5.16
http://schemas.microsoft.com/office/spreadsheetml/2016/revision3

section
5.17
http://schemas.microsoft.com/office/spreadsheetml/2016/revision5

section
5.18
http://schemas.microsoft.com/office/spreadsheetml/2016/pivotdefaultlayout

section
5.19
http://schemas.microsoft.com/office/spreadsheetml/2017/richdata2

section
5.20
http://schemas.microsoft.com/office/spreadsheetml/2017/richdata

section

<!-- page 370 -->

370 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Schema name
Prefix
Section
5.21
http://schemas.microsoft.com/office/spreadsheetml/2018/calcfeatures

section
5.22
http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments

section
5.23
http://schemas.microsoft.com/office/spreadsheetml/2018/08/main

section
5.24
http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray

section
5.25
http://schemas.microsoft.com/office/spreadsheetml/2019/namedsheetviews

section
5.26
http://schemas.microsoft.com/office/spreadsheetml/2019/extlinksprops

section
5.27
http://schemas.microsoft.com/office/spreadsheetml/2020/richdatawebimage

section
5.28
http://schemas.microsoft.com/office/spreadsheetml/2020/pivotNov2020

section
5.29
http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2

section
5.30
http://schemas.microsoft.com/office/spreadsheetml/2020/richvaluerefresh

section
5.31
http://schemas.microsoft.com/office/spreadsheetml/2022/pivotVersionInfo

section
5.32
http://schemas.microsoft.com/office/spreadsheetml/2022/pivotRichData

section
5.33
http://schemas.microsoft.com/office/spreadsheetml/2021/extlinks2021

section
5.34
http://schemas.microsoft.com/office/spreadsheetml/2022/richvaluerel

section
5.35
http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag

section
5.36
http://schemas.microsoft.com/office/spreadsheetml/2023/msForms

section
5.37
http://schemas.microsoft.com/office/spreadsheetml/2023/externalCodeService

section
5.38
http://schemas.microsoft.com/office/spreadsheetml/2023/python

section
5.39
http://schemas.microsoft.com/office/spreadsheetml/2023/pivot2023Calculation

section
5.40
http://schemas.microsoft.com/office/spreadsheetml/2024/pivotAutoRefresh

section
5.41
http://schemas.microsoft.com/office/spreadsheetml/2024/workbookCompatibilityVersion

section

<!-- page 371 -->

371 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Schema name
Prefix
Section
5.42
http://schemas.microsoft.com/office/spreadsheetml/2023/showDataTypeIcons

section
5.43
http://schemas.microsoft.com/office/spreadsheetml/2025/externalCodeService2
Schema

section
5.44
http://schemas.microsoft.com/office/spreadsheetml/2025/pivotDataSource Schema

section
5.45

5.1
http://schemas.microsoft.com/office/excel/2006/main Schema
 <xsd:schema xmlns="http://schemas.microsoft.com/office/excel/2006/main"
elementFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/excel/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:simpleType name="ST_Ref">
    <xsd:restriction base="xsd:string"/>
  </xsd:simpleType>
  <xsd:complexType name="CT_Ref">
    <xsd:simpleContent>
      <xsd:extension base="ST_Ref">
        <xsd:attribute name="edited" type="xsd:boolean" use="optional"/>
        <xsd:attribute name="adjusted" type="xsd:boolean" use="optional"/>
        <xsd:attribute name="adjust" type="xsd:boolean" use="optional"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_Sqref">
    <xsd:list itemType="ST_Ref"/>
  </xsd:simpleType>
  <xsd:complexType name="CT_Sqref">
    <xsd:simpleContent>
      <xsd:extension base="ST_Sqref">
        <xsd:attribute name="edited" type="xsd:boolean" use="optional"/>
        <xsd:attribute name="split" type="xsd:boolean" use="optional"/>
        <xsd:attribute name="adjusted" type="xsd:boolean" use="optional"/>
        <xsd:attribute name="adjust" type="xsd:boolean" use="optional"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:element name="f" type="x:ST_Formula"/>
  <xsd:element name="ref" type="CT_Ref"/>
  <xsd:element name="sqref" type="CT_Sqref"/>
 </xsd:schema>
5.2
http://schemas.microsoft.com/office/drawing/2010/slicer Schema
 <xsd:schema elementFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/drawing/2010/slicer"
xmlns="http://schemas.microsoft.com/office/drawing/2010/slicer"
xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import schemaLocation="oart14docprop.xsd"
namespace="http://schemas.microsoft.com/office/drawing/2010/main"/>

<!-- page 372 -->

372 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import schemaLocation="oartdocprop.xsd"
namespace="http://schemas.openxmlformats.org/drawingml/2006/main"/>
  <xsd:import schemaLocation="orel.xsd"
namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
  <xsd:complexType name="CT_Slicer">
    <xsd:sequence>
      <xsd:element name="extLst" type="a:CT_OfficeArtExtensionList" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:element name="slicer" type="CT_Slicer"/>
 </xsd:schema>
5.3
http://schemas.microsoft.com/office/spreadsheetml/2010/11/main Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main"
elementFormDefault="qualified"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"
xmlns:xr10="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
schemaLocation="xlslicercache.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlworkbook.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/excel/2006/main"
schemaLocation="xlmbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsheet.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlpivot.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlextconns.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2014/11/main"
schemaLocation="xlpivot16.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
schemaLocation="xlrevexignore10.xsd"/>
  <xsd:element name="slicerCaches" type="x14:CT_SlicerCaches"/>
  <xsd:element name="tableSlicerCache" type="CT_TableSlicerCache"/>
  <xsd:complexType name="CT_TableSlicerCache">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="tableId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="column" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="sortOrder" type="x14:ST_TabularSlicerCacheSortOrder" use="optional"
default="ascending"/>
    <xsd:attribute name="customListSort" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="crossFilter" type="x14:ST_SlicerCacheCrossFilter" use="optional"
default="showItemsWithDataAtTop"/>
  </xsd:complexType>
  <xsd:element name="slicerCacheHideItemsWithNoData" type="CT_SlicerCacheHideNoData"/>
  <xsd:complexType name="CT_SlicerCacheHideNoData">
    <xsd:sequence>
      <xsd:element name="slicerCacheOlapLevelName" type="CT_SlicerCacheOlapLevelName"
minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional" default="0"/>
  </xsd:complexType>

<!-- page 373 -->

373 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_SlicerCacheOlapLevelName">
    <xsd:attribute name="uniqueName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:element name="slicerCachePivotTables" type="x14:CT_SlicerCachePivotTables"/>
  <xsd:element name="pivotCaches" type="x:CT_PivotCaches"/>
  <xsd:element name="pivotTableReferences" type="CT_PivotTableReferences"/>
  <xsd:complexType name="CT_PivotTableReferences">
    <xsd:sequence>
      <xsd:element name="pivotTableReference" type="CT_PivotTableReference" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotTableReference">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:element name="queryTable" type="CT_QueryTable"/>
  <xsd:complexType name="CT_QueryTable">
    <xsd:attribute name="clipped" use="optional" default="false" type="xsd:boolean"/>
    <xsd:attribute name="sourceDataName" type="xsd:string" use="optional"/>
    <xsd:attribute name="drillThrough" use="optional" default="false" type="xsd:boolean"/>
  </xsd:complexType>
  <xsd:element name="webExtensions" type="CT_WebExtensions"/>
  <xsd:complexType name="CT_WebExtensions">
    <xsd:sequence>
      <xsd:element name="webExtension" type="CT_WebExtension" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_WebExtension">
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="appRef" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:element name="timelineCacheRefs" type="CT_TimelineCacheRefs"/>
  <xsd:complexType name="CT_TimelineCacheRefs">
    <xsd:sequence>
      <xsd:element name="timelineCacheRef" type="CT_TimelineCacheRef" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineCacheRef">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:element name="timelineRefs" type="CT_TimelineRefs"/>
  <xsd:complexType name="CT_TimelineRefs">
    <xsd:sequence>
      <xsd:element name="timelineRef" type="CT_TimelineRef" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineRef">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:element name="timelineCachePivotCaches" type="x:CT_PivotCaches"/>
  <xsd:complexType name="CT_WorkbookPr">
    <xsd:attribute name="chartTrackingRefBase" type="xsd:boolean" default="false"/>
  </xsd:complexType>
  <xsd:element name="workbookPr" type="CT_WorkbookPr"/>
  <xsd:element name="timelineStyles" type="CT_TimelineStyles"/>
  <xsd:complexType name="CT_TimelineStyles">
    <xsd:sequence>
      <xsd:element name="timelineStyle" type="CT_TimelineStyle" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="defaultTimelineStyle" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineStyleElements">

<!-- page 374 -->

374 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:sequence>
      <xsd:element name="timelineStyleElement" type="CT_TimelineStyleElement" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineStyle">
    <xsd:sequence>
      <xsd:element name="timelineStyleElements" type="CT_TimelineStyleElements" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineStyleElement">
    <xsd:attribute name="type" type="ST_TimelineStyleType" use="required"/>
    <xsd:attribute name="dxfId" type="x:ST_DxfId" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_TimelineStyleType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="selectionLabel"/>
      <xsd:enumeration value="timeLevel"/>
      <xsd:enumeration value="periodLabel1"/>
      <xsd:enumeration value="periodLabel2"/>
      <xsd:enumeration value="selectedTimeBlock"/>
      <xsd:enumeration value="unselectedTimeBlock"/>
      <xsd:enumeration value="selectedTimeBlockSpace"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="dxfs" type="x:CT_Dxfs"/>
  <xsd:element name="connection" type="CT_Connection"/>
  <xsd:complexType name="CT_ModelTextPr">
    <xsd:attribute name="headers" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_RangePr">
    <xsd:attribute name="sourceName" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DbTable">
    <xsd:attribute name="name" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DbTables">
    <xsd:sequence>
      <xsd:element name="dbTable" minOccurs="1" maxOccurs="unbounded" type="CT_DbTable"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_DbCommand">
    <xsd:attribute name="text" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OledbPr">
    <xsd:choice minOccurs="1" maxOccurs="1">
      <xsd:element name="dbTables" type="CT_DbTables"/>
      <xsd:element name="dbCommand" type="CT_DbCommand"/>
    </xsd:choice>
    <xsd:attribute name="connection" use="optional" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DataFeedPr">
    <xsd:sequence>
      <xsd:element name="dbTables" type="CT_DbTables" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="connection" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Connection">
    <xsd:sequence>
      <xsd:element name="textPr" minOccurs="0" maxOccurs="1" type="x:CT_TextPr"/>
      <xsd:element name="modelTextPr" minOccurs="0" maxOccurs="1" type="CT_ModelTextPr"/>
      <xsd:element name="rangePr" minOccurs="0" maxOccurs="1" type="CT_RangePr"/>
      <xsd:element name="oledbPr" minOccurs="0" maxOccurs="1" type="CT_OledbPr"/>
      <xsd:element name="dataFeedPr" minOccurs="0" maxOccurs="1" type="CT_DataFeedPr"/>
    </xsd:sequence>
    <xsd:attribute name="id" use="required" type="x:ST_Xstring"/>
    <xsd:attribute name="model" type="xsd:boolean" default="false" use="optional"/>

<!-- page 375 -->

375 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="excludeFromRefreshAll" type="xsd:boolean" default="false"
use="optional"/>
    <xsd:attribute name="autoDelete" type="xsd:boolean" default="false" use="optional"/>
    <xsd:attribute name="usedByAddin" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_CalcMemNumberFormat">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="default"/>
      <xsd:enumeration value="number"/>
      <xsd:enumeration value="percent"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_CalculatedMemberExt">
    <xsd:sequence>
      <xsd:element ref="calculatedMember" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="calculatedMember" type="CT_CalculatedMember"/>
  <xsd:complexType name="CT_CalculatedMember">
    <xsd:attribute name="measureGroup" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="numberFormat" type="ST_CalcMemNumberFormat" use="optional"
default="default"/>
    <xsd:attribute name="measure" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="pivotTableUISettings" type="CT_PivotTableUISettings"/>
  <xsd:complexType name="CT_PivotTableUISettings">
    <xsd:sequence>
      <xsd:element name="activeTabTopLevelEntity" type="CT_FieldListActiveTabTopLevelEntity"
minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="sourceDataName" type="xsd:string" use="optional"/>
    <xsd:attribute name="relNeededHidden" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_FieldListActiveTabTopLevelEntity">
    <xsd:attribute name="name" use="required" type="xsd:string"/>
    <xsd:attribute name="type" use="optional" default="0" type="xsd:unsignedInt"/>
  </xsd:complexType>
  <xsd:element name="pivotFilter" type="CT_PivotFilter"/>
  <xsd:complexType name="CT_PivotFilter">
    <xsd:attribute name="useWholeDay" type="xsd:boolean" use="required"/>
  </xsd:complexType>
  <xsd:element name="cachedUniqueNames" type="CT_CachedUniqueNames"/>
  <xsd:complexType name="CT_CachedUniqueNames">
    <xsd:sequence>
      <xsd:element name="cachedUniqueName" minOccurs="1" maxOccurs="unbounded"
type="CT_CachedUniqueName"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CachedUniqueName">
    <xsd:attribute name="index" use="required" type="xsd:unsignedInt"/>
    <xsd:attribute name="name" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CacheHierarchy">
    <xsd:attribute name="aggregatedColumn" use="required" type="xsd:int"/>
  </xsd:complexType>
  <xsd:element name="cacheHierarchy" type="CT_CacheHierarchy"/>
  <xsd:element name="timelinePivotCacheDefinition" type="CT_TimelinePivotCacheDefinition"/>
  <xsd:complexType name="CT_TimelinePivotCacheDefinition">
    <xsd:attribute name="timelineData" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="pivotCacheIdVersion" type="CT_PivotCacheIdVersion"/>
  <xsd:complexType name="CT_PivotCacheIdVersion">
    <xsd:attribute name="cacheIdSupportedVersion" type="xsd:unsignedByte" use="required"/>
    <xsd:attribute name="cacheIdCreatedVersion" type="xsd:unsignedByte" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ModelTable">
    <xsd:attribute name="id" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>

<!-- page 376 -->

376 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="connection" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ModelTables">
    <xsd:sequence>
      <xsd:element name="modelTable" minOccurs="1" maxOccurs="unbounded"
type="CT_ModelTable"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_ModelRelationship">
    <xsd:attribute name="fromTable" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="fromColumn" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="toTable" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="toColumn" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ModelRelationships">
    <xsd:sequence>
      <xsd:element name="modelRelationship" minOccurs="1" maxOccurs="unbounded"
type="CT_ModelRelationship"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_DataModel">
    <xsd:sequence>
      <xsd:element name="modelTables" minOccurs="0" maxOccurs="1" type="CT_ModelTables"/>
      <xsd:element name="modelRelationships" minOccurs="0" maxOccurs="1"
type="CT_ModelRelationships"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="minVersionLoad" type="xsd:unsignedByte" use="optional" default="5"/>
  </xsd:complexType>
  <xsd:element name="dataModel" type="CT_DataModel"/>
  <xsd:element name="pivotTableData" type="CT_PivotTableData"/>
  <xsd:complexType name="CT_PivotTableData">
    <xsd:sequence>
      <xsd:element name="pivotRow" type="CT_PivotRow" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="rowCount" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="columnCount" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="cacheId" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotRow">
    <xsd:sequence>
      <xsd:element name="c" type="CT_PivotValueCell" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="r" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotValueCell">
    <xsd:sequence>
      <xsd:element name="v" type="x:ST_Xstring" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="x" type="CT_PivotValueCellExtra" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="i" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="t" type="ST_SXVCellType" use="optional" default="n"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotValueCellExtra">
    <xsd:attribute name="in" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="bc" type="x:ST_UnsignedIntHex" use="optional"/>
    <xsd:attribute name="fc" type="x:ST_UnsignedIntHex" use="optional"/>
    <xsd:attribute name="i" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="un" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="st" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="b" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_SXVCellType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="n"/>
      <xsd:enumeration value="e"/>
      <xsd:enumeration value="str"/>

<!-- page 377 -->

377 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:enumeration value="d"/>
      <xsd:enumeration value="bl"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_PivotTableServerFormats">
    <xsd:sequence>
      <xsd:element name="serverFormat" type="x:CT_ServerFormat" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="count" use="required" type="xsd:unsignedInt"/>
  </xsd:complexType>
  <xsd:element name="dataField" type="CT_DataField"/>
  <xsd:complexType name="CT_DataField">
    <xsd:attribute name="isCountDistinct" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="survey" type="CT_Survey"/>
  <xsd:complexType name="CT_Survey">
    <xsd:sequence>
      <xsd:element name="surveyPr" type="CT_SurveyElementPr" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="titlePr" type="CT_SurveyElementPr" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="descriptionPr" type="CT_SurveyElementPr" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="questions" type="CT_SurveyQuestions" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="id" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="guid" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="title" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="description" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SurveyQuestions">
    <xsd:sequence>
      <xsd:element name="questionsPr" type="CT_SurveyElementPr" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="question" type="CT_SurveyQuestion" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SurveyQuestion">
    <xsd:sequence>
      <xsd:element name="questionPr" type="CT_SurveyElementPr" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="binding" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="text" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="type" type="ST_QuestionType" use="optional"/>
    <xsd:attribute name="format" type="ST_QuestionFormat" use="optional"/>
    <xsd:attribute name="helpText" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="required" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="defaultValue" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="decimalPlaces" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="rowSource" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SurveyElementPr">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="cssClass" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="bottom" type="xsd:int" use="optional"/>
    <xsd:attribute name="top" type="xsd:int" use="optional"/>
    <xsd:attribute name="left" type="xsd:int" use="optional"/>
    <xsd:attribute name="right" type="xsd:int" use="optional"/>
    <xsd:attribute name="width" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="height" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="position" type="ST_SurveyPosition" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_QuestionType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="checkBox"/>
      <xsd:enumeration value="choice"/>

<!-- page 378 -->

378 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:enumeration value="date"/>
      <xsd:enumeration value="time"/>
      <xsd:enumeration value="multipleLinesOfText"/>
      <xsd:enumeration value="number"/>
      <xsd:enumeration value="singleLineOfText"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_QuestionFormat">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="generalDate"/>
      <xsd:enumeration value="longDate"/>
      <xsd:enumeration value="shortDate"/>
      <xsd:enumeration value="longTime"/>
      <xsd:enumeration value="shortTime"/>
      <xsd:enumeration value="generalNumber"/>
      <xsd:enumeration value="standard"/>
      <xsd:enumeration value="fixed"/>
      <xsd:enumeration value="percent"/>
      <xsd:enumeration value="currency"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_SurveyPosition">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="absolute"/>
      <xsd:enumeration value="fixed"/>
      <xsd:enumeration value="relative"/>
      <xsd:enumeration value="static"/>
      <xsd:enumeration value="inherit"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="timelines" type="CT_Timelines"/>
  <xsd:complexType name="CT_Timelines">
    <xsd:sequence>
      <xsd:element name="timeline" type="CT_Timeline" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Timeline">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute ref="xr10:uid" use="optional"/>
    <xsd:attribute name="cache" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="caption" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="showHeader" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="showSelectionLabel" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="showTimeLevel" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="showHorizontalScrollbar" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="level" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="selectionLevel" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="scrollPosition" type="xsd:dateTime" use="optional"/>
    <xsd:attribute name="style" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:element name="timelineCacheDefinition" type="CT_TimelineCacheDefinition"/>
  <xsd:complexType name="CT_TimelineCacheDefinition">
    <xsd:sequence>
      <xsd:element name="pivotTables" type="CT_TimelineCachePivotTables" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="state" type="CT_TimelineState" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="timelinePivotFilter" minOccurs="0" maxOccurs="1"
type="CT_TimelinePivotFilter"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute ref="xr10:uid" use="optional"/>
    <xsd:attribute name="sourceName" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>

<!-- page 379 -->

379 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_TimelineCachePivotTables">
    <xsd:sequence>
      <xsd:element name="pivotTable" type="CT_TimelineCachePivotTable" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineCachePivotTable">
    <xsd:attribute name="tabId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineRange">
    <xsd:attribute name="startDate" type="xsd:dateTime" use="required"/>
    <xsd:attribute name="endDate" type="xsd:dateTime" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelineState">
    <xsd:sequence>
      <xsd:element name="selection" type="CT_TimelineRange" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="bounds" type="CT_TimelineRange" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="singleRangeFilterState" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="minimalRefreshVersion" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="lastRefreshVersion" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="pivotCacheId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="filterType" use="required" type="x:ST_PivotFilterType"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TimelinePivotFilter">
    <xsd:sequence>
      <xsd:element name="autoFilter" minOccurs="0" maxOccurs="1" type="x:CT_AutoFilter"/>
    </xsd:sequence>
    <xsd:attribute name="useWholeDay" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="fld" use="required" type="xsd:unsignedInt"/>
    <xsd:attribute name="id" use="required" type="xsd:unsignedInt"/>
    <xsd:attribute name="name" use="optional" type="x:ST_Xstring"/>
    <xsd:attribute name="description" use="optional" type="x:ST_Xstring"/>
  </xsd:complexType>
 </xsd:schema>
5.4
http://schemas.microsoft.com/office/spreadsheetml/2009/9/main Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xr10="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
elementFormDefault="qualified" xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"
xmlns:xr="http://schemas.microsoft.com/office/spreadsheetml/2014/revision"
xmlns:xr2="http://schemas.microsoft.com/office/spreadsheetml/2015/revision2">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsheet.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main"
schemaLocation="xl15.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main"
schemaLocation="xlslicercache15.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
schemaLocation="xlrevexignore10.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlpivot.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlworkbook.xsd"/>

<!-- page 380 -->

380 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsupbook.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/excel/2006/main"
schemaLocation="xlmbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2014/revision"
schemaLocation="xlrevexignore.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2015/revision2"
schemaLocation="xlrevexignore2.xsd"/>
  <xsd:import schemaLocation="xlsst.xsd"
namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>
  <xsd:element name="slicerCacheDefinition" type="CT_SlicerCacheDefinition"/>
  <xsd:complexType name="CT_SlicerCacheDefinition">
    <xsd:sequence>
      <xsd:element name="pivotTables" type="CT_SlicerCachePivotTables" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="data" type="CT_SlicerCacheData" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute ref="xr10:uid" use="optional"/>
    <xsd:attribute name="sourceName" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_OlapSlicerCacheSortOrder">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="natural"/>
      <xsd:enumeration value="ascending"/>
      <xsd:enumeration value="descending"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_TabularSlicerCacheSortOrder">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="ascending"/>
      <xsd:enumeration value="descending"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_SlicerCacheCrossFilter">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="none"/>
      <xsd:enumeration value="showItemsWithDataAtTop"/>
      <xsd:enumeration value="showItemsWithNoData"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_SlicerCacheData">
    <xsd:choice minOccurs="1" maxOccurs="1">
      <xsd:element name="olap" type="CT_OlapSlicerCache" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="tabular" type="CT_TabularSlicerCache" minOccurs="1" maxOccurs="1"/>
    </xsd:choice>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerCachePivotTables">
    <xsd:sequence>
      <xsd:element name="pivotTable" type="CT_SlicerCachePivotTable" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerCachePivotTable">
    <xsd:attribute name="tabId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheItem">
    <xsd:sequence>
      <xsd:element name="p" type="CT_OlapSlicerCacheItemParent" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="c" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="nd" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheItemParent">
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>

<!-- page 381 -->

381 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheRange">
    <xsd:sequence>
      <xsd:element name="i" type="CT_OlapSlicerCacheItem" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="startItem" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheRanges">
    <xsd:sequence>
      <xsd:element name="range" type="CT_OlapSlicerCacheRange" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheLevelData">
    <xsd:sequence>
      <xsd:element name="ranges" type="CT_OlapSlicerCacheRanges" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="uniqueName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="sourceCaption" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="sortOrder" type="ST_OlapSlicerCacheSortOrder" use="optional"
default="natural"/>
    <xsd:attribute name="crossFilter" type="ST_SlicerCacheCrossFilter" use="optional"
default="showItemsWithDataAtTop"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheLevelsData">
    <xsd:sequence>
      <xsd:element name="level" type="CT_OlapSlicerCacheLevelData" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCache">
    <xsd:sequence>
      <xsd:element name="levels" type="CT_OlapSlicerCacheLevelsData" minOccurs="1"
maxOccurs="1"/>
      <xsd:element name="selections" type="CT_OlapSlicerCacheSelections" minOccurs="1"
maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="pivotCacheId" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheSelections">
    <xsd:sequence>
      <xsd:element name="selection" type="CT_OlapSlicerCacheSelection" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_OlapSlicerCacheSelection">
    <xsd:sequence>
      <xsd:element name="p" type="CT_OlapSlicerCacheItemParent" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TabularSlicerCache">
    <xsd:sequence>
      <xsd:element name="items" type="CT_TabularSlicerCacheItems" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="pivotCacheId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="sortOrder" type="ST_TabularSlicerCacheSortOrder" use="optional"
default="ascending"/>
    <xsd:attribute name="customListSort" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="showMissing" type="xsd:boolean" use="optional" default="true"/>

<!-- page 382 -->

382 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="crossFilter" type="ST_SlicerCacheCrossFilter" use="optional"
default="showItemsWithDataAtTop"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TabularSlicerCacheItems">
    <xsd:sequence>
      <xsd:element name="i" type="CT_TabularSlicerCacheItem" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TabularSlicerCacheItem">
    <xsd:attribute name="x" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="s" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="nd" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="conditionalFormattings" type="CT_ConditionalFormattings"/>
  <xsd:element name="dataValidations" type="CT_DataValidations"/>
  <xsd:element name="sparklineGroups" type="CT_SparklineGroups"/>
  <xsd:element name="slicerList" type="CT_SlicerRefs"/>
  <xsd:element name="protectedRanges" type="CT_ProtectedRanges"/>
  <xsd:element name="ignoredErrors" type="CT_IgnoredErrors"/>
  <xsd:complexType name="CT_ConditionalFormattings">
    <xsd:sequence>
      <xsd:element name="conditionalFormatting" type="CT_ConditionalFormatting" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_ConditionalFormatting">
    <xsd:sequence>
      <xsd:element name="cfRule" type="CT_CfRule" minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element ref="xm:sqref" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" minOccurs="0" type="x:CT_ExtensionList"/>
    </xsd:sequence>
    <xsd:attribute name="pivot" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DataValidations">
    <xsd:sequence>
      <xsd:element name="dataValidation" type="CT_DataValidation" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="disablePrompts" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="xWindow" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="yWindow" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DataValidationFormula">
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_DataValidation">
    <xsd:sequence>
      <xsd:element name="formula1" type="CT_DataValidationFormula" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="formula2" type="CT_DataValidationFormula" minOccurs="0"
maxOccurs="1"/>
      <xsd:element ref="xm:sqref" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="type" type="x:ST_DataValidationType" use="optional" default="none"/>
    <xsd:attribute name="errorStyle" type="x:ST_DataValidationErrorStyle" use="optional"
default="stop"/>
    <xsd:attribute name="imeMode" type="x:ST_DataValidationImeMode" use="optional"
default="noControl"/>
    <xsd:attribute name="operator" type="x:ST_DataValidationOperator" use="optional"
default="between"/>
    <xsd:attribute name="allowBlank" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="showDropDown" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="showInputMessage" type="xsd:boolean" use="optional"
default="false"/>

<!-- page 383 -->

383 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="showErrorMessage" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="errorTitle" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="error" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="promptTitle" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="prompt" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute ref="xr:uid"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_DispBlanksAs">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="span"/>
      <xsd:enumeration value="gap"/>
      <xsd:enumeration value="zero"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_SparklineAxisMinMax">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="individual"/>
      <xsd:enumeration value="group"/>
      <xsd:enumeration value="custom"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_SparklineGroups">
    <xsd:sequence>
      <xsd:element name="sparklineGroup" type="CT_SparklineGroup" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SparklineGroup">
    <xsd:sequence>
      <xsd:element name="colorSeries" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorNegative" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorAxis" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorMarkers" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorFirst" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorLast" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorHigh" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element name="colorLow" minOccurs="0" maxOccurs="1" type="x:CT_Color"/>
      <xsd:element ref="xm:f" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="sparklines" type="CT_Sparklines" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="manualMax" type="xsd:double" use="optional"/>
    <xsd:attribute name="manualMin" type="xsd:double" use="optional"/>
    <xsd:attribute name="lineWeight" type="xsd:double" use="optional" default="0.75"/>
    <xsd:attribute name="type" type="ST_SparklineType" use="optional" default="line"/>
    <xsd:attribute name="dateAxis" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="displayEmptyCellsAs" type="ST_DispBlanksAs" use="optional"
default="zero"/>
    <xsd:attribute name="markers" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="high" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="low" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="first" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="last" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="negative" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="displayXAxis" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="displayHidden" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="minAxisType" type="ST_SparklineAxisMinMax" use="optional"
default="individual"/>
    <xsd:attribute name="maxAxisType" type="ST_SparklineAxisMinMax" use="optional"
default="individual"/>
    <xsd:attribute name="rightToLeft" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute ref="xr2:uid"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_SparklineType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="line"/>
      <xsd:enumeration value="column"/>
      <xsd:enumeration value="stacked"/>
    </xsd:restriction>

<!-- page 384 -->

384 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  </xsd:simpleType>
  <xsd:complexType name="CT_Sparklines">
    <xsd:sequence>
      <xsd:element name="sparkline" type="CT_Sparkline" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Sparkline">
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="0" maxOccurs="1"/>
      <xsd:element ref="xm:sqref" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="pivotCaches" type="x:CT_PivotCaches"/>
  <xsd:element name="slicerCaches" type="CT_SlicerCaches"/>
  <xsd:element name="workbookPr" type="CT_WorkbookPr"/>
  <xsd:complexType name="CT_WorkbookPr">
    <xsd:attribute name="defaultImageDpi" type="xsd:unsignedInt" default="220"/>
    <xsd:attribute name="discardImageEditData" type="xsd:boolean" default="false"/>
    <xsd:attribute name="accuracyVersion" type="xsd:unsignedInt" default="0"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerRefs">
    <xsd:sequence>
      <xsd:element name="slicer" type="CT_SlicerRef" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerRef">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerCaches">
    <xsd:sequence>
      <xsd:element name="slicerCache" type="CT_SlicerCache" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerCache">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:element name="calculatedMember" type="CT_CalculatedMember"/>
  <xsd:complexType name="CT_CalculatedMember">
    <xsd:sequence>
      <xsd:element name="tupleSet" minOccurs="0" maxOccurs="1" type="CT_TupleSet"/>
    </xsd:sequence>
    <xsd:attribute name="displayFolder" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="flattenHierarchies" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="dynamicSet" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="hierarchizeDistinct" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="mdxLong" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSet">
    <xsd:sequence>
      <xsd:element name="headers" type="CT_TupleSetHeaders" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="rows" type="CT_TupleSetRows" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="rowCount" type="xsd:unsignedInt" use="optional" default="1"/>
    <xsd:attribute name="columnCount" type="xsd:unsignedInt" use="optional" default="1"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSetHeaders">
    <xsd:sequence>
      <xsd:element name="header" type="CT_TupleSetHeader" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSetHeader">
    <xsd:attribute name="uniqueName" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="hierarchyName" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSetRows">

<!-- page 385 -->

385 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:sequence>
      <xsd:element name="row" type="CT_TupleSetRow" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSetRow">
    <xsd:sequence>
      <xsd:element name="rowItem" type="CT_TupleSetRowItem" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TupleSetRowItem">
    <xsd:attribute name="u" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="d" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:element name="cacheHierarchy" type="CT_CacheHierarchy"/>
  <xsd:complexType name="CT_SetLevels">
    <xsd:sequence>
      <xsd:element name="setLevel" minOccurs="1" maxOccurs="unbounded" type="CT_SetLevel"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SetLevel">
    <xsd:attribute name="hierarchy" use="required" type="xsd:int"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CacheHierarchy">
    <xsd:sequence>
      <xsd:element name="setLevels" minOccurs="0" maxOccurs="1" type="CT_SetLevels"/>
    </xsd:sequence>
    <xsd:attribute name="flattenHierarchies" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="measuresSet" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="hierarchizeDistinct" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="ignore" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:element name="dataField" type="CT_DataField"/>
  <xsd:complexType name="CT_DataField">
    <xsd:attribute name="pivotShowAs" type="ST_PivotShowAs" use="optional"/>
    <xsd:attribute name="sourceField" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="uniqueName" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_PivotShowAs">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="percentOfParent"/>
      <xsd:enumeration value="percentOfParentRow"/>
      <xsd:enumeration value="percentOfParentCol"/>
      <xsd:enumeration value="percentOfRunningTotal"/>
      <xsd:enumeration value="rankAscending"/>
      <xsd:enumeration value="rankDescending"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_DataBarDirection">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="context"/>
      <xsd:enumeration value="leftToRight"/>
      <xsd:enumeration value="rightToLeft"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_DataBarAxisPosition">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="automatic"/>
      <xsd:enumeration value="middle"/>
      <xsd:enumeration value="none"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_CfvoType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="num"/>
      <xsd:enumeration value="percent"/>

<!-- page 386 -->

386 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:enumeration value="max"/>
      <xsd:enumeration value="min"/>
      <xsd:enumeration value="formula"/>
      <xsd:enumeration value="percentile"/>
      <xsd:enumeration value="autoMin"/>
      <xsd:enumeration value="autoMax"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Cfvo">
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="type" type="ST_CfvoType" use="required"/>
    <xsd:attribute name="gte" type="xsd:boolean" use="optional" default="true"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CfRule">
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="0" maxOccurs="3"/>
      <xsd:element name="colorScale" type="CT_ColorScale" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="dataBar" type="CT_DataBar" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="iconSet" type="CT_IconSet" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="dxf" type="x:CT_Dxf" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="type" type="x:ST_CfType" use="optional"/>
    <xsd:attribute name="priority" type="xsd:int" use="optional"/>
    <xsd:attribute name="stopIfTrue" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="aboveAverage" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="percent" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="bottom" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="operator" type="x:ST_ConditionalFormattingOperator" use="optional"/>
    <xsd:attribute name="text" type="xsd:string" use="optional"/>
    <xsd:attribute name="timePeriod" type="x:ST_TimePeriod" use="optional"/>
    <xsd:attribute name="rank" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="stdDev" type="xsd:int" use="optional"/>
    <xsd:attribute name="equalAverage" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="activePresent" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_IconSet">
    <xsd:sequence>
      <xsd:element name="cfvo" type="CT_Cfvo" minOccurs="2" maxOccurs="unbounded"/>
      <xsd:element name="cfIcon" type="CT_CfIcon" minOccurs="0" maxOccurs="5"/>
    </xsd:sequence>
    <xsd:attribute name="iconSet" type="ST_IconSetType" use="optional"
default="3TrafficLights1"/>
    <xsd:attribute name="showValue" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="percent" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="reverse" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="custom" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ColorScale">
    <xsd:sequence>
      <xsd:element name="cfvo" type="CT_Cfvo" minOccurs="2" maxOccurs="unbounded"/>
      <xsd:element name="color" type="x:CT_Color" minOccurs="2" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_DataBar">
    <xsd:sequence>
      <xsd:element name="cfvo" type="CT_Cfvo" minOccurs="2" maxOccurs="2"/>
      <xsd:element name="fillColor" type="x:CT_Color" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="borderColor" type="x:CT_Color" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="negativeFillColor" type="x:CT_Color" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="negativeBorderColor" type="x:CT_Color" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="axisColor" type="x:CT_Color" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="minLength" type="xsd:unsignedInt" use="optional" default="10"/>
    <xsd:attribute name="maxLength" type="xsd:unsignedInt" use="optional" default="90"/>

<!-- page 387 -->

387 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="showValue" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="border" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="gradient" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="direction" type="ST_DataBarDirection" use="optional"
default="context"/>
    <xsd:attribute name="negativeBarColorSameAsPositive" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="negativeBarBorderColorSameAsPositive" type="xsd:boolean"
use="optional" default="true"/>
    <xsd:attribute name="axisPosition" type="ST_DataBarAxisPosition" use="optional"
default="automatic"/>
  </xsd:complexType>
  <xsd:element name="pivotField" type="CT_PivotField"/>
  <xsd:complexType name="CT_PivotField">
    <xsd:attribute name="fillDownLabels" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="ignore" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:element name="pivotTableDefinition" type="CT_PivotTableDefinition"/>
  <xsd:complexType name="CT_PivotTableDefinition">
    <xsd:sequence>
      <xsd:element name="pivotEdits" type="CT_PivotEdits" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="pivotChanges" type="CT_PivotChanges" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="conditionalFormats" type="CT_ConditionalFormats" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="fillDownLabelsDefault" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="visualTotalsForSets" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="calculatedMembersInFilters" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="altText" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="altTextSummary" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="enableEdit" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="autoApply" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="allocationMethod" type="ST_AllocationMethod" use="optional"
default="equalAllocation"/>
    <xsd:attribute name="weightExpression" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="hideValuesRow" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="pivotCacheDefinition" type="CT_PivotCacheDefinition"/>
  <xsd:complexType name="CT_PivotCacheDefinition">
    <xsd:attribute name="slicerData" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="pivotCacheId" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="supportSubqueryNonVisual" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="supportSubqueryCalcMem" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="supportAddCalcMems" type="xsd:boolean" use="optional"
default="false"/>
  </xsd:complexType>
  <xsd:element name="connection" type="CT_Connection"/>
  <xsd:complexType name="CT_Connection">
    <xsd:sequence>
      <xsd:element name="calculatedMembers" type="x:CT_CalculatedMembers" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="culture" use="optional" type="x:ST_Xstring"/>
    <xsd:attribute name="embeddedDataId" use="optional" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:element name="table" type="CT_Table"/>
  <xsd:complexType name="CT_Table">
    <xsd:attribute name="altText" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="altTextSummary" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CfIcon">
    <xsd:attribute name="iconSet" type="ST_IconSetType" use="required"/>
    <xsd:attribute name="iconId" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_IconSetType">

<!-- page 388 -->

388 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="3Arrows"/>
      <xsd:enumeration value="3ArrowsGray"/>
      <xsd:enumeration value="3Flags"/>
      <xsd:enumeration value="3TrafficLights1"/>
      <xsd:enumeration value="3TrafficLights2"/>
      <xsd:enumeration value="3Signs"/>
      <xsd:enumeration value="3Symbols"/>
      <xsd:enumeration value="3Symbols2"/>
      <xsd:enumeration value="4Arrows"/>
      <xsd:enumeration value="4ArrowsGray"/>
      <xsd:enumeration value="4RedToBlack"/>
      <xsd:enumeration value="4Rating"/>
      <xsd:enumeration value="4TrafficLights"/>
      <xsd:enumeration value="5Arrows"/>
      <xsd:enumeration value="5ArrowsGray"/>
      <xsd:enumeration value="5Rating"/>
      <xsd:enumeration value="5Quarters"/>
      <xsd:enumeration value="3Stars"/>
      <xsd:enumeration value="3Triangles"/>
      <xsd:enumeration value="5Boxes"/>
      <xsd:enumeration value="NoIcons"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_PivotEdits">
    <xsd:sequence>
      <xsd:element name="pivotEdit" minOccurs="1" maxOccurs="unbounded" type="CT_PivotEdit"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotEdit">
    <xsd:sequence>
      <xsd:element name="userEdit" type="CT_PivotUserEdit" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="tupleItems" type="CT_TupleItems" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="pivotArea" type="x:CT_PivotArea" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotChanges">
    <xsd:sequence>
      <xsd:element name="pivotChange" minOccurs="1" maxOccurs="unbounded"
type="CT_PivotChange"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotChange">
    <xsd:sequence>
      <xsd:element name="editValue" type="CT_PivotEditValue" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="tupleItems" type="CT_TupleItems" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="allocationMethod" type="ST_AllocationMethod"
default="equalAllocation"/>
    <xsd:attribute name="weightExpression" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotUserEdit">
    <xsd:choice minOccurs="1" maxOccurs="1">
      <xsd:element ref="xm:f" minOccurs="1" maxOccurs="1"/>
      <xsd:element name="editValue" type="CT_PivotEditValue" minOccurs="1" maxOccurs="1"/>
    </xsd:choice>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotEditValue">
    <xsd:simpleContent>
      <xsd:extension base="x:ST_Xstring">
        <xsd:attribute name="valueType" use="required" type="ST_PivotEditValueType"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_PivotEditValueType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="number"/>

<!-- page 389 -->

389 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:enumeration value="dateTime"/>
      <xsd:enumeration value="string"/>
      <xsd:enumeration value="boolean"/>
      <xsd:enumeration value="error"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_TupleItems">
    <xsd:sequence>
      <xsd:element name="tupleItem" type="x:ST_Xstring" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:simpleType name="ST_AllocationMethod">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="equalAllocation"/>
      <xsd:enumeration value="equalIncrement"/>
      <xsd:enumeration value="weightedAllocation"/>
      <xsd:enumeration value="weightedIncrement"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_SlicerStyle">
    <xsd:sequence>
      <xsd:element name="slicerStyleElements" type="CT_SlicerStyleElements" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerStyleElement">
    <xsd:attribute name="type" type="ST_SlicerStyleType" use="required"/>
    <xsd:attribute name="dxfId" type="x:ST_DxfId" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_SlicerStyleType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="unselectedItemWithData"/>
      <xsd:enumeration value="selectedItemWithData"/>
      <xsd:enumeration value="unselectedItemWithNoData"/>
      <xsd:enumeration value="selectedItemWithNoData"/>
      <xsd:enumeration value="hoveredUnselectedItemWithData"/>
      <xsd:enumeration value="hoveredSelectedItemWithData"/>
      <xsd:enumeration value="hoveredUnselectedItemWithNoData"/>
      <xsd:enumeration value="hoveredSelectedItemWithNoData"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="slicerStyles" type="CT_SlicerStyles"/>
  <xsd:element name="dxfs" type="x:CT_Dxfs"/>
  <xsd:complexType name="CT_OleItem">
    <xsd:sequence>
      <xsd:element name="values" type="x:CT_DdeValues" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="icon" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="advise" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="preferPic" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:element name="oleItem" type="CT_OleItem"/>
  <xsd:element name="pivotHierarchy" type="CT_PivotHierarchy"/>
  <xsd:complexType name="CT_PivotHierarchy">
    <xsd:attribute name="ignore" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:element name="cacheField" type="CT_CacheField"/>
  <xsd:complexType name="CT_CacheField">
    <xsd:attribute name="ignore" type="xsd:boolean" default="false" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ConditionalFormats">
    <xsd:sequence>
      <xsd:element name="conditionalFormat" minOccurs="1" maxOccurs="unbounded"
type="CT_ConditionalFormat"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>

<!-- page 390 -->

390 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_ConditionalFormat">
    <xsd:sequence>
      <xsd:element name="pivotAreas" type="x:CT_PivotAreas" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" minOccurs="0" maxOccurs="1" type="x:CT_ExtensionList"/>
    </xsd:sequence>
    <xsd:attribute name="scope" type="x:ST_Scope" default="selection" use="optional"/>
    <xsd:attribute name="type" type="x:ST_Type" default="none" use="optional"/>
    <xsd:attribute name="priority" use="optional" type="xsd:unsignedInt"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerStyles">
    <xsd:sequence>
      <xsd:element name="slicerStyle" type="CT_SlicerStyle" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="defaultSlicerStyle" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SlicerStyleElements">
    <xsd:sequence>
      <xsd:element name="slicerStyleElement" type="CT_SlicerStyleElement" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="id" type="x:ST_Guid"/>
  <xsd:complexType name="CT_IgnoredErrors">
    <xsd:sequence>
      <xsd:element name="ignoredError" type="CT_IgnoredError" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_IgnoredError">
    <xsd:sequence>
      <xsd:element ref="xm:sqref" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="evalError" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="twoDigitTextYear" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="numberStoredAsText" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="formula" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="formulaRange" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="unlockedFormula" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="emptyCellReference" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="listDataValidation" type="xsd:boolean" use="optional"
default="false"/>
    <xsd:attribute name="calculatedColumn" type="xsd:boolean" use="optional"
default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ProtectedRanges">
    <xsd:sequence>
      <xsd:element name="protectedRange" type="CT_ProtectedRange" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_ProtectedRange">
    <xsd:sequence maxOccurs="1">
      <xsd:element ref="xm:sqref" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="password" type="x:ST_UnsignedShortHex" use="optional"/>
    <xsd:attribute name="algorithmName" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="hashValue" type="xsd:base64Binary" use="optional"/>
    <xsd:attribute name="saltValue" type="xsd:base64Binary" use="optional"/>
    <xsd:attribute name="spinCount" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="securityDescriptor" type="xsd:string" use="optional"/>
  </xsd:complexType>
  <xsd:element name="iconFilter" type="CT_IconFilter"/>

<!-- page 391 -->

391 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_IconFilter">
    <xsd:attribute name="iconSet" type="ST_IconSetType" use="required"/>
    <xsd:attribute name="iconId" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:element name="filter" type="CT_Filter"/>
  <xsd:complexType name="CT_Filter">
    <xsd:attribute name="val" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:element name="customFilters" type="CT_CustomFilters"/>
  <xsd:complexType name="CT_CustomFilters">
    <xsd:sequence>
      <xsd:element name="customFilter" type="CT_CustomFilter" minOccurs="1" maxOccurs="2"/>
    </xsd:sequence>
    <xsd:attribute name="and" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CustomFilter">
    <xsd:attribute name="operator" type="x:ST_FilterOperator" default="equal"
use="optional"/>
    <xsd:attribute name="val" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SortCondition">
    <xsd:attribute name="descending" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="sortBy" type="x:ST_SortBy" use="optional" default="value"/>
    <xsd:attribute name="ref" type="x:ST_Ref" use="required"/>
    <xsd:attribute name="customList" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="dxfId" type="x:ST_DxfId" use="optional"/>
    <xsd:attribute name="iconSet" type="ST_IconSetType" use="optional" default="3Arrows"/>
    <xsd:attribute name="iconId" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:element name="sortCondition" type="CT_SortCondition"/>
  <xsd:complexType name="CT_CacheSourceExt">
    <xsd:sequence>
      <xsd:element ref="sourceConnection" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="sourceConnection" type="CT_SourceConnection"/>
  <xsd:complexType name="CT_SourceConnection">
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:element name="datastoreItem" type="CT_DatastoreItem"/>
  <xsd:complexType name="CT_DatastoreItem">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="id" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:element name="formControlPr" type="CT_FormControlPr"/>
  <xsd:complexType name="CT_ListItem">
    <xsd:attribute name="val" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ListItems">
    <xsd:sequence>
      <xsd:element name="item" type="CT_ListItem" minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_FormControlPr">
    <xsd:sequence>
      <xsd:element name="itemLst" type="CT_ListItems" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="objectType" type="ST_ObjectType" use="optional"/>
    <xsd:attribute name="checked" type="ST_Checked" use="optional"/>
    <xsd:attribute name="colored" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="dropLines" type="xsd:unsignedInt" use="optional" default="8"/>
    <xsd:attribute name="dropStyle" type="ST_DropStyle" use="optional"/>
    <xsd:attribute name="dx" type="xsd:unsignedInt" use="optional" default="80"/>
    <xsd:attribute name="firstButton" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="fmlaGroup" type="x:ST_Formula" use="optional"/>

<!-- page 392 -->

392 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:attribute name="fmlaLink" type="x:ST_Formula" use="optional"/>
    <xsd:attribute name="fmlaRange" type="x:ST_Formula" use="optional"/>
    <xsd:attribute name="fmlaTxbx" type="x:ST_Formula" use="optional"/>
    <xsd:attribute name="horiz" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="inc" type="xsd:unsignedInt" use="optional" default="1"/>
    <xsd:attribute name="justLastX" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="lockText" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="max" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="min" type="xsd:unsignedInt" use="optional" default="0"/>
    <xsd:attribute name="multiSel" type="xsd:string" use="optional"/>
    <xsd:attribute name="noThreeD" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="noThreeD2" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="page" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="sel" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="seltype" type="ST_SelType" use="optional" default="single"/>
    <xsd:attribute name="textHAlign" type="ST_TextHAlign" use="optional" default="left"/>
    <xsd:attribute name="textVAlign" type="ST_TextVAlign" use="optional" default="top"/>
    <xsd:attribute name="val" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="widthMin" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="editVal" type="ST_EditValidation" use="optional"/>
    <xsd:attribute name="multiLine" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="verticalBar" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="passwordEdit" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_ObjectType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="Button"/>
      <xsd:enumeration value="CheckBox"/>
      <xsd:enumeration value="Drop"/>
      <xsd:enumeration value="GBox"/>
      <xsd:enumeration value="Label"/>
      <xsd:enumeration value="List"/>
      <xsd:enumeration value="Radio"/>
      <xsd:enumeration value="Scroll"/>
      <xsd:enumeration value="Spin"/>
      <xsd:enumeration value="EditBox"/>
      <xsd:enumeration value="Dialog"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_Checked">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="Unchecked"/>
      <xsd:enumeration value="Checked"/>
      <xsd:enumeration value="Mixed"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_DropStyle">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="combo"/>
      <xsd:enumeration value="comboedit"/>
      <xsd:enumeration value="simple"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_SelType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="single"/>
      <xsd:enumeration value="multi"/>
      <xsd:enumeration value="extended"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_TextHAlign">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="left"/>
      <xsd:enumeration value="center"/>
      <xsd:enumeration value="right"/>
      <xsd:enumeration value="justify"/>
      <xsd:enumeration value="distributed"/>
    </xsd:restriction>
  </xsd:simpleType>

<!-- page 393 -->

393 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:simpleType name="ST_TextVAlign">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="top"/>
      <xsd:enumeration value="center"/>
      <xsd:enumeration value="bottom"/>
      <xsd:enumeration value="justify"/>
      <xsd:enumeration value="distributed"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_EditValidation">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="text"/>
      <xsd:enumeration value="integer"/>
      <xsd:enumeration value="number"/>
      <xsd:enumeration value="reference"/>
      <xsd:enumeration value="formula"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="slicers" type="CT_Slicers"/>
  <xsd:complexType name="CT_Slicers">
    <xsd:sequence>
      <xsd:element name="slicer" type="CT_Slicer" minOccurs="1" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Slicer">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="x:ST_Xstring" use="required"/>
    <xsd:attribute ref="xr10:uid" use="optional"/>
    <xsd:attribute name="cache" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="caption" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="startItem" type="xsd:unsignedInt" use="optional" default="0"/>
    <xsd:attribute name="columnCount" type="xsd:unsignedInt" use="optional" default="1"/>
    <xsd:attribute name="showCaption" type="xsd:boolean" use="optional" default="true"/>
    <xsd:attribute name="level" type="xsd:unsignedInt" use="optional" default="0"/>
    <xsd:attribute name="style" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="lockedPosition" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="rowHeight" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.5
http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="dyDescent" type="xsd:double"/>
  <xsd:attribute name="knownFonts" type="xsd:boolean"/>
 </xsd:schema>
5.6
http://schemas.microsoft.com/office/spreadsheetml/2011/1/ac Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2011/1/ac"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns="xl12AcSchemaUri">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>

<!-- page 394 -->

394 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:element name="list" type="x:ST_Xstring"/>
 </xsd:schema>
5.7
http://schemas.microsoft.com/office/drawing/2012/timeslicer Schema
 <xsd:schema elementFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/drawing/2012/timeslicer"
xmlns="http://schemas.microsoft.com/office/drawing/2012/timeslicer"
xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/drawingml/2006/main"
schemaLocation="oartdocprop.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:complexType name="CT_Timeline">
    <xsd:sequence>
      <xsd:element name="extLst" type="a:CT_OfficeArtExtensionList" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:element name="timeslicer" type="CT_Timeline"/>
 </xsd:schema>
5.8
http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing Schema
 <xsd:schema elementFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing"
xmlns="http://schemas.microsoft.com/office/excel/2010/spreadsheetDrawing"
xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"
xmlns:xdr14="http://xldr14SchemaUri"
xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import schemaLocation="oart14docprop.xsd"
namespace="http://schemas.microsoft.com/office/drawing/2010/main"/>
  <xsd:import schemaLocation="oartdocprop.xsd"
namespace="http://schemas.openxmlformats.org/drawingml/2006/main"/>
  <xsd:import schemaLocation="orel.xsd"
namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
  <xsd:complexType name="CT_ApplicationNonVisualDrawingProps">
    <xsd:attribute name="macro" type="xsd:string" use="optional"/>
    <xsd:attribute name="fPublished" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ContentPartNonVisual">
    <xsd:sequence>
      <xsd:element name="cNvPr" type="a:CT_NonVisualDrawingProps" minOccurs="1"
maxOccurs="1"/>
      <xsd:element name="cNvContentPartPr" type="a14:CT_NonVisualInkContentPartProperties"
minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_ContentPart">
    <xsd:sequence>
      <xsd:element name="nvContentPartPr" type="CT_ContentPartNonVisual" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="nvPr" type="CT_ApplicationNonVisualDrawingProps" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="xfrm" type="a:CT_Transform2D" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="a:CT_OfficeArtExtensionList" minOccurs="0"
maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute ref="r:id" use="required"/>
    <xsd:attribute name="bwMode" type="a:ST_BlackWhiteMode" use="optional" default="auto"/>

<!-- page 395 -->

395 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  </xsd:complexType>
  <xsd:element name="contentPart" type="CT_ContentPart"/>
 </xsd:schema>
5.9
http://schemas.microsoft.com/office/spreadsheetml/2010/11/ac Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2010/11/ac"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2010/11/ac">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:complexType name="CT_AbsolutePath">
    <xsd:attribute name="url" use="required" type="x:ST_Xstring"/>
  </xsd:complexType>
  <xsd:element name="absPath" type="CT_AbsolutePath"/>
 </xsd:schema>
5.10 http://schemas.microsoft.com/office/spreadsheetml/2014/11/main Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2014/11/main"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2014/11/main"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlworkbook.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsheet.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlpivot.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlextconns.xsd"/>
  <xsd:complexType name="CT_ModelTimeGroupings">
    <xsd:sequence>
      <xsd:element name="modelTimeGrouping" minOccurs="1" maxOccurs="unbounded"
type="CT_ModelTimeGrouping"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="modelTimeGroupings" type="CT_ModelTimeGroupings"/>
  <xsd:complexType name="CT_ModelTimeGrouping">
    <xsd:sequence>
      <xsd:element name="calculatedTimeColumn" minOccurs="1" maxOccurs="unbounded"
type="CT_CalculatedTimeColumn"/>
    </xsd:sequence>
    <xsd:attribute name="tableName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="columnName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="columnId" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CalculatedTimeColumn">
    <xsd:attribute name="columnName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="columnId" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="contentType" type="ST_ModelTimeGroupingContentType" use="required"/>
    <xsd:attribute name="isSelected" type="xsd:boolean" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_ModelTimeGroupingContentType">
    <xsd:restriction base="x:ST_Xstring">

<!-- page 396 -->

396 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:enumeration value="years"/>
      <xsd:enumeration value="quarters"/>
      <xsd:enumeration value="monthsindex"/>
      <xsd:enumeration value="months"/>
      <xsd:enumeration value="daysindex"/>
      <xsd:enumeration value="days"/>
      <xsd:enumeration value="hours"/>
      <xsd:enumeration value="minutes"/>
      <xsd:enumeration value="seconds"/>
    </xsd:restriction>
  </xsd:simpleType>
 </xsd:schema>
5.11 http://schemas.microsoft.com/office/spreadsheetml/2015/02/main Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2015/02/main"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2015/02/main"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="formatCode16" type="x:ST_Xstring"/>
 </xsd:schema>
5.12 http://schemas.microsoft.com/office/spreadsheetml/2016/revision10 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="uid" type="x:ST_Guid"/>
  <xsd:attribute name="uidLastSave" type="x:ST_Guid"/>
 </xsd:schema>
5.13 http://schemas.microsoft.com/office/spreadsheetml/2016/revision9 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision9"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/revision9"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="uid" type="x:ST_Guid"/>
 </xsd:schema>
5.14 http://schemas.microsoft.com/office/spreadsheetml/2016/revision6 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision6"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/revision6"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">

<!-- page 397 -->

397 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="uid" type="x:ST_Guid"/>
  <xsd:attribute name="coauthVersionLast" type="xsd:unsignedInt"/>
  <xsd:attribute name="coauthVersionMax" type="xsd:unsignedInt"/>
 </xsd:schema>
5.15 http://schemas.microsoft.com/office/spreadsheetml/2014/revision Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2014/revision"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2014/revision"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xr6="http://schemas.microsoft.com/office/spreadsheetml/2016/revision6"
xmlns:xr10="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlstyles.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision6"
schemaLocation="xlrevexignore6.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10"
schemaLocation="xlrevexignore10.xsd"/>
  <xsd:simpleType name="ST_Xrevid">
    <xsd:restriction base="xsd:unsignedLong"/>
  </xsd:simpleType>
  <xsd:complexType name="CT_RevisionPtr">
    <xsd:attribute name="revIDLastSave" type="ST_Xrevid" use="required"/>
    <xsd:attribute name="documentId" type="x:ST_Xstring" use="required"/>
    <xsd:attribute ref="xr6:coauthVersionLast" use="optional" default="0"/>
    <xsd:attribute ref="xr6:coauthVersionMax" use="optional" default="0"/>
    <xsd:attribute ref="xr10:uidLastSave" use="optional"/>
  </xsd:complexType>
  <xsd:attribute name="uid" type="x:ST_Guid"/>
  <xsd:element name="revisionPtr" type="CT_RevisionPtr"/>
 </xsd:schema>
5.16 http://schemas.microsoft.com/office/spreadsheetml/2015/revision2 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2015/revision2"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2015/revision2"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="uid" type="x:ST_Guid"/>
 </xsd:schema>
5.17 http://schemas.microsoft.com/office/spreadsheetml/2016/revision3 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision3"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/revision3"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:attribute name="uid" type="x:ST_Guid"/>

<!-- page 398 -->

398 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
 </xsd:schema>
5.18 http://schemas.microsoft.com/office/spreadsheetml/2016/revision5 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/revision5"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/revision5"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlstyles.xsd"/>
 </xsd:schema>
5.19 http://schemas.microsoft.com/office/spreadsheetml/2016/pivotdefaultlayout
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2016/pivotdefaultlayout"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2016/pivotdefaultlayout"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlworkbook.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsheet.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlpivot.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlextconns.xsd"/>
  <xsd:element name="pivotTableDefinition16" type="CT_PivotTableDefinition16"/>
  <xsd:complexType name="CT_PivotTableDefinition16">
    <xsd:attribute name="EnabledSubtotalsDefault" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="SubtotalsOnTopDefault" type="xsd:boolean" use="optional"
default="true"/>
    <xsd:attribute name="InsertBlankRowDefault" type="xsd:boolean" use="optional"
default="false"/>
  </xsd:complexType>
 </xsd:schema>
5.20 http://schemas.microsoft.com/office/spreadsheetml/2017/richdata2 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2017/richdata2"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:x2="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2017/richdata2"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
schemaLocation="xl14.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsheet.xsd"/>

<!-- page 399 -->

399 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import
namespace="http://schemas.microsoft.com/office/spreadsheetml/2020/richvaluerefresh"
schemaLocation="xlrichvaluerefresh.xsd"/>
  <xsd:element name="filterColumn" type="CT_RichFilterColumn"/>
  <xsd:complexType name="CT_RichFilterColumn">
    <xsd:choice minOccurs="0" maxOccurs="1">
      <xsd:element name="filters" type="CT_RichFilters"/>
      <xsd:element name="top10" type="CT_RichTop10"/>
      <xsd:element name="customFilters" type="CT_CustomRichFilters"/>
      <xsd:element name="dynamicFilter" type="CT_DynamicRichFilter"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList"/>
    </xsd:choice>
  </xsd:complexType>
  <xsd:complexType name="CT_RichFilters">
    <xsd:sequence>
      <xsd:element name="filter" type="CT_RichFilter" minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element name="dateGroupItem" type="CT_RichDateGroupItem" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichFilter">
    <xsd:attribute name="key" type="ST_RichFilterKey" use="optional"/>
    <xsd:attribute name="val" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="blank" type="xsd:boolean" use="optional"/>
    <xsd:attribute name="nodata" type="xsd:boolean" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CustomRichFilters">
    <xsd:sequence>
      <xsd:choice minOccurs="1" maxOccurs="2">
        <xsd:element name="customFilter" minOccurs="0" maxOccurs="1"
type="CT_CustomRichFilter"/>
        <xsd:element name="extLst" minOccurs="0" maxOccurs="1" type="x:CT_ExtensionList"/>
      </xsd:choice>
    </xsd:sequence>
    <xsd:attribute name="and" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_CustomRichFilter">
    <xsd:complexContent>
      <xsd:extension base="x:CT_CustomFilter">
        <xsd:attribute name="key" type="ST_RichFilterKey" use="optional"/>
      </xsd:extension>
    </xsd:complexContent>
  </xsd:complexType>
  <xsd:complexType name="CT_RichTop10">
    <xsd:complexContent>
      <xsd:extension base="x:CT_Top10">
        <xsd:attribute name="key" type="ST_RichFilterKey" use="optional"/>
      </xsd:extension>
    </xsd:complexContent>
  </xsd:complexType>
  <xsd:complexType name="CT_DynamicRichFilter">
    <xsd:complexContent>
      <xsd:extension base="x:CT_DynamicFilter">
        <xsd:attribute name="key" type="ST_RichFilterKey" use="optional"/>
      </xsd:extension>
    </xsd:complexContent>
  </xsd:complexType>
  <xsd:complexType name="CT_RichDateGroupItem">
    <xsd:complexContent>
      <xsd:extension base="x:CT_DateGroupItem">
        <xsd:attribute name="key" type="ST_RichFilterKey" use="optional"/>
      </xsd:extension>
    </xsd:complexContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_RichFilterKey">
    <xsd:restriction base="x:ST_Xstring"/>
  </xsd:simpleType>
  <xsd:element name="richSortCondition" type="CT_RichSortCondition"/>

<!-- page 400 -->

400 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_RichSortCondition">
    <xsd:complexContent>
      <xsd:extension base="x2:CT_SortCondition">
        <xsd:attribute name="richSortKey" type="x:ST_Xstring" use="optional"/>
      </xsd:extension>
    </xsd:complexContent>
  </xsd:complexType>
  <xsd:element name="supportingPropertyBags" type="CT_SupportingPropertyBags"/>
  <xsd:complexType name="CT_SupportingPropertyBags">
    <xsd:sequence>
      <xsd:element name="spbArrays" minOccurs="0" maxOccurs="1"
type="CT_SupportingPropertyBagArrayData"/>
      <xsd:element name="spbData" minOccurs="1" maxOccurs="1"
type="CT_SupportingPropertyBagData"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagData">
    <xsd:sequence>
      <xsd:element name="spb" minOccurs="0" maxOccurs="unbounded"
type="CT_SupportingPropertyBag"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBag">
    <xsd:sequence>
      <xsd:element name="v" minOccurs="1" maxOccurs="unbounded"
type="CT_SupportingPropertyBagValue"/>
    </xsd:sequence>
    <xsd:attribute name="s" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagValue">
    <xsd:simpleContent>
      <xsd:extension base="x:ST_Xstring"/>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:element name="spbStructures" type="CT_SupportingPropertyBagStructures"/>
  <xsd:complexType name="CT_SupportingPropertyBagStructures">
    <xsd:sequence>
      <xsd:element name="s" minOccurs="0" maxOccurs="unbounded"
type="CT_SupportingPropertyBagStructure"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagStructure">
    <xsd:sequence>
      <xsd:element name="k" minOccurs="1" maxOccurs="unbounded"
type="CT_SupportingPropertyBagKey"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagKey">
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="t" type="ST_SupportingPropertyBagValueType" use="optional"
default="d"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_SupportingPropertyBagValueType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="d"/>
      <xsd:enumeration value="i"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="s"/>
      <xsd:enumeration value="spb"/>
      <xsd:enumeration value="spba"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_SupportingPropertyBagArrayData">
    <xsd:sequence>

<!-- page 401 -->

401 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="a" minOccurs="0" maxOccurs="unbounded"
type="CT_SupportingPropertyBagArray"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagArray">
    <xsd:sequence>
      <xsd:element name="v" minOccurs="1" maxOccurs="unbounded"
type="CT_SupportingPropertyBagArrayValue"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SupportingPropertyBagArrayValue">
    <xsd:simpleContent>
      <xsd:extension base="x:ST_Xstring">
        <xsd:attribute name="t" type="ST_SupportingPropertyBagArrayValueType" use="optional"
default="d"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_SupportingPropertyBagArrayValueType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="d"/>
      <xsd:enumeration value="i"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="s"/>
      <xsd:enumeration value="spb"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="arrayData" type="CT_ArrayData"/>
  <xsd:complexType name="CT_ArrayData">
    <xsd:sequence>
      <xsd:element name="a" minOccurs="0" maxOccurs="unbounded" type="CT_Array"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Array">
    <xsd:sequence>
      <xsd:element name="v" minOccurs="1" maxOccurs="unbounded" type="CT_ArrayValue"/>
    </xsd:sequence>
    <xsd:attribute name="r" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="c" type="xsd:unsignedInt" use="optional" default="1"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ArrayValue">
    <xsd:simpleContent>
      <xsd:extension base="x:ST_Xstring">
        <xsd:attribute name="t" type="ST_ArrayValueType" use="optional" default="d"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_ArrayValueType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="d"/>
      <xsd:enumeration value="i"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="e"/>
      <xsd:enumeration value="s"/>
      <xsd:enumeration value="r"/>
      <xsd:enumeration value="a"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="richStyleSheet" type="CT_RichStylesheet"/>
  <xsd:complexType name="CT_RichStylesheet">
    <xsd:sequence>
      <xsd:element name="dxfs" type="x:CT_Dxfs" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="richProperties" type="CT_RichFormatProperties" minOccurs="0"
maxOccurs="1"/>

<!-- page 402 -->

402 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="richStyles" type="CT_RichStyles" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichFormatProperties">
    <xsd:sequence>
      <xsd:element name="rPr" minOccurs="1" maxOccurs="unbounded"
type="CT_RichFormatProperty"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichFormatProperty">
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="t" type="ST_RichFormatPropertyType" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_RichFormatPropertyType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="n"/>
      <xsd:enumeration value="i"/>
      <xsd:enumeration value="s"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_RichStyles">
    <xsd:sequence>
      <xsd:element name="rSty" minOccurs="1" maxOccurs="unbounded" type="CT_RichStyle"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichStyle">
    <xsd:sequence>
      <xsd:element name="rpv" minOccurs="0" maxOccurs="unbounded"
type="CT_RichStylePropertyValue"/>
    </xsd:sequence>
    <xsd:attribute name="dxfid" type="x:ST_DxfId" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_RichStylePropertyValue">
    <xsd:simpleContent>
      <xsd:extension base="xsd:string">
        <xsd:attribute name="i" type="xsd:unsignedInt" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:element name="rvTypesInfo" type="CT_RichValueTypesInfo"/>
  <xsd:complexType name="CT_RichValueTypesInfo">
    <xsd:sequence>
      <xsd:element name="global" minOccurs="0" maxOccurs="1" type="CT_RichValueGlobalType"/>
      <xsd:element name="types" minOccurs="0" maxOccurs="1" type="CT_RichValueTypes"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueTypes">
    <xsd:sequence>
      <xsd:element name="type" minOccurs="0" maxOccurs="unbounded" type="CT_RichValueType"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueGlobalType">
    <xsd:sequence>
      <xsd:element name="keyFlags" minOccurs="0" maxOccurs="1"
type="CT_RichValueTypeKeyFlags"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueType">
    <xsd:sequence>
      <xsd:element name="keyFlags" minOccurs="0" maxOccurs="1"
type="CT_RichValueTypeKeyFlags"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>

<!-- page 403 -->

403 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_RichValueTypeKeyFlags">
    <xsd:sequence>
      <xsd:element name="key" minOccurs="1" maxOccurs="unbounded"
type="CT_RichValueTypeReservedKey"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueTypeReservedKey">
    <xsd:sequence>
      <xsd:element name="flag" minOccurs="1" maxOccurs="unbounded"
type="CT_RichValueTypeReservedKeyFlag"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueTypeReservedKeyFlag">
    <xsd:attribute name="name" type="xsd:string" use="required"/>
    <xsd:attribute name="value" type="xsd:boolean" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.21 http://schemas.microsoft.com/office/spreadsheetml/2017/richdata Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2017/richdata"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2017/richdata"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="rvData" type="CT_RichValueData"/>
  <xsd:complexType name="CT_RichValueData">
    <xsd:sequence>
      <xsd:element name="rv" minOccurs="0" maxOccurs="unbounded" type="CT_RichValue"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValue">
    <xsd:sequence>
      <xsd:element name="fb" minOccurs="0" maxOccurs="1" type="CT_RichValueFallback"/>
      <xsd:element name="v" minOccurs="1" maxOccurs="unbounded" type="CT_Value"/>
    </xsd:sequence>
    <xsd:attribute name="s" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Value">
    <xsd:simpleContent>
      <xsd:extension base="x:ST_Xstring"/>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:element name="rvStructures" type="CT_RichValueStructures"/>
  <xsd:complexType name="CT_RichValueFallback">
    <xsd:simpleContent>
      <xsd:extension base="xsd:string">
        <xsd:attribute name="t" type="ST_RichValueFallbackType" use="optional" default="n"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:simpleType name="ST_RichValueFallbackType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="n"/>
      <xsd:enumeration value="e"/>
      <xsd:enumeration value="s"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_RichValueStructures">
    <xsd:sequence>

<!-- page 404 -->

404 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="s" minOccurs="0" maxOccurs="unbounded"
type="CT_RichValueStructure"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueStructure">
    <xsd:sequence>
      <xsd:element name="k" minOccurs="1" maxOccurs="unbounded" type="CT_Key"/>
    </xsd:sequence>
    <xsd:attribute name="t" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Key">
    <xsd:attribute name="n" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="t" type="ST_RichValueValueType" use="optional" default="d"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_RichValueValueType">
    <xsd:restriction base="x:ST_Xstring">
      <xsd:enumeration value="d"/>
      <xsd:enumeration value="i"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="e"/>
      <xsd:enumeration value="s"/>
      <xsd:enumeration value="r"/>
      <xsd:enumeration value="a"/>
      <xsd:enumeration value="spb"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="rvb" type="CT_RichValueBlock"/>
  <xsd:complexType name="CT_RichValueBlock">
    <xsd:attribute name="i" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.22 http://schemas.microsoft.com/office/spreadsheetml/2018/calcfeatures
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2018/calcfeatures"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2018/calcfeatures"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="calcFeatures" type="CT_CalcFeatures"/>
  <xsd:complexType name="CT_CalcFeatures">
    <xsd:sequence>
      <xsd:element name="feature" minOccurs="1" maxOccurs="unbounded" type="CT_CalcFeature"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CalcFeature">
    <xsd:attribute name="name" type="xsd:string" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.23 http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments">

<!-- page 405 -->

405 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlsst.xsd"/>
  <xsd:import
namespace="http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2"
schemaLocation="xlThreadedComments2.xsd"/>
  <xsd:element name="personList" type="CT_PersonList"/>
  <xsd:complexType name="CT_PersonList">
    <xsd:sequence>
      <xsd:element name="person" type="CT_Person" minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Person">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="displayName" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="userId" type="x:ST_Xstring" use="optional"/>
    <xsd:attribute name="providerId" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:element name="ThreadedComments" type="CT_ThreadedComments"/>
  <xsd:complexType name="CT_ThreadedComments">
    <xsd:sequence>
      <xsd:element name="threadedComment" type="CT_ThreadedComment" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_ThreadedComment">
    <xsd:sequence>
      <xsd:element name="text" type="x:ST_Xstring" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="mentions" type="CT_ThreadedCommentMentions" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="extLst" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="ref" type="x:ST_Ref" use="optional"/>
    <xsd:attribute name="dT" type="xsd:dateTime" use="optional"/>
    <xsd:attribute name="personId" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="parentId" type="x:ST_Guid" use="optional"/>
    <xsd:attribute name="done" type="xsd:boolean" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ThreadedCommentMentions">
    <xsd:sequence>
      <xsd:element name="mention" type="CT_Mention" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Mention">
    <xsd:attribute name="mentionpersonId" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="mentionId" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="startIndex" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="length" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.24 http://schemas.microsoft.com/office/spreadsheetml/2018/08/main Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2018/08/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2018/08/main"
elementFormDefault="qualified">
  <xsd:attribute name="misleadingFormat" type="xsd:boolean"/>
 </xsd:schema>

<!-- page 406 -->

406 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
5.25 http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2017/dynamicarray"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="dynamicArrayProperties" type="CT_DynamicArrayProperties"/>
  <xsd:complexType name="CT_DynamicArrayProperties">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="fDynamic" type="xsd:boolean" use="optional"/>
    <xsd:attribute name="fCollapsed" type="xsd:boolean" use="optional"/>
  </xsd:complexType>
 </xsd:schema>
5.26 http://schemas.microsoft.com/office/spreadsheetml/2019/namedsheetviews
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2019/namedsheetviews"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
xmlns:xlrd2="http://schemas.microsoft.com/office/spreadsheetml/2017/richdata2"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2019/namedsheetviews">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlautofilter.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"
schemaLocation="xl14.xsd"/>
  <xsd:element name="namedSheetViews" type="CT_NamedSheetViews"/>
  <xsd:complexType name="CT_NamedSheetViews">
    <xsd:sequence>
      <xsd:element name="namedSheetView" type="CT_NamedSheetView" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_NamedSheetView">
    <xsd:sequence>
      <xsd:element name="nsvFilter" type="CT_NsvFilter" minOccurs="0" maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="name" type="xsd:string" use="required"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_NsvFilter">
    <xsd:sequence>
      <xsd:element name="columnFilter" minOccurs="0" maxOccurs="unbounded"
type="CT_ColumnFilter"/>
      <xsd:element name="sortRules" minOccurs="0" maxOccurs="1" type="CT_SortRules"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="filterId" type="x:ST_Guid" use="required"/>
    <xsd:attribute name="ref" type="x:ST_Ref" use="optional"/>
    <xsd:attribute name="tableId" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ColumnFilter">
    <xsd:sequence>
      <xsd:element name="dxf" type="x:CT_Dxf" minOccurs="0" maxOccurs="1"/>

<!-- page 407 -->

407 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="filter" minOccurs="0" maxOccurs="unbounded"
type="x:CT_FilterColumn"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attributeGroup ref="AG_ColumnId"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SortRules">
    <xsd:sequence>
      <xsd:element name="sortRule" minOccurs="0" maxOccurs="64" type="CT_SortRule"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="sortMethod" type="x:ST_SortMethod" use="optional" default="none"/>
    <xsd:attribute name="caseSensitive" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SortRule">
    <xsd:sequence>
      <xsd:element name="dxf" type="x:CT_Dxf" minOccurs="0" maxOccurs="1"/>
      <xsd:choice minOccurs="0" maxOccurs="1">
        <xsd:element name="sortCondition" minOccurs="0" maxOccurs="1"
type="x14:CT_SortCondition"/>
        <xsd:element name="richSortCondition" minOccurs="0" maxOccurs="1"
type="xlrd2:CT_RichSortCondition"/>
      </xsd:choice>
    </xsd:sequence>
    <xsd:attributeGroup ref="AG_ColumnId"/>
  </xsd:complexType>
  <xsd:attributeGroup name="AG_ColumnId">
    <xsd:attribute name="colId" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="id" type="x:ST_Guid" use="optional"/>
  </xsd:attributeGroup>
 </xsd:schema>
5.27 http://schemas.microsoft.com/office/spreadsheetml/2019/extlinksprops
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2019/extlinksprops"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2019/extlinksprops"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:element name="externalLinksPr" type="CT_ExternalLinksPr"/>
  <xsd:complexType name="CT_ExternalLinksPr">
    <xsd:attribute name="autoRefresh" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
 </xsd:schema>
5.28 http://schemas.microsoft.com/office/spreadsheetml/2020/richdatawebimage
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2020/richdatawebimage"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2020/richdatawebimage"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="webImagesSrd" type="CT_WebImagesSupportingRichData"/>
  <xsd:complexType name="CT_WebImagesSupportingRichData">
    <xsd:sequence>

<!-- page 408 -->

408 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="webImageSrd" minOccurs="0" maxOccurs="unbounded"
type="CT_WebImageSupportingRichData"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_WebImageSupportingRichData">
    <xsd:sequence>
      <xsd:element name="address" minOccurs="1" maxOccurs="1"
type="CT_WebImageSupportingRichDataRelationship"/>
      <xsd:element name="moreImagesAddress" minOccurs="0" maxOccurs="1"
type="CT_WebImageSupportingRichDataRelationship"/>
      <xsd:element name="blip" minOccurs="0" maxOccurs="1"
type="CT_WebImageSupportingRichDataRelationship"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_WebImageSupportingRichDataRelationship">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.29 http://schemas.microsoft.com/office/spreadsheetml/2020/pivotNov2020
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2020/pivotNov2020"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2020/pivotNov2020"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="implicitMeasureSupport" type="xsd:boolean"/>
  <xsd:element name="ignorableAfterVersion" type="CT_Ignorable"/>
  <xsd:complexType name="CT_Ignorable">
    <xsd:attribute name="version" type="xsd:unsignedByte" use="required"/>
  </xsd:complexType>
  <xsd:element name="dataFieldFutureData" type="CT_DataFieldFutureData"/>
  <xsd:complexType name="CT_DataFieldFutureData">
    <xsd:attribute name="version" type="xsd:unsignedByte" use="required"/>
    <xsd:attribute name="sourceField" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.30 http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments
2 Schema
 <xsd:schema xmlns="http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2020/threadedcomments2"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main">
  <xsd:import namespace="http://schemas.microsoft.com/office/excel/2006/main"
schemaLocation="xlmbasictypes.xsd"/>
  <xsd:complexType name="CT_ThreadedComments2Ext">
    <xsd:sequence>
      <xsd:element name="checksum" type="xsd:unsignedInt" minOccurs="1" maxOccurs="1"/>
      <xsd:sequence>
        <xsd:element name="hyperlink" type="CT_CommentHyperlink" minOccurs="0"
maxOccurs="unbounded"/>
      </xsd:sequence>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CommentHyperlink">
    <xsd:sequence>

<!-- page 409 -->

409 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="startIndex" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="length" type="xsd:unsignedInt" use="required"/>
    <xsd:attribute name="url" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.31 http://schemas.microsoft.com/office/spreadsheetml/2020/richvaluerefresh
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2020/richvaluerefresh"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2020/richvaluerefresh"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:element name="refreshIntervals" type="CT_RichValueRefreshIntervals"/>
  <xsd:complexType name="CT_RichValueRefreshIntervals">
    <xsd:sequence>
      <xsd:element name="refreshInterval" minOccurs="1" maxOccurs="unbounded"
type="CT_RichValueRefreshInterval"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueRefreshInterval">
    <xsd:attribute name="resourceIdInt" type="xsd:int" use="optional"/>
    <xsd:attribute name="resourceIdStr" type="xsd:string" use="optional"/>
    <xsd:attribute name="interval" type="xsd:int" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.32 http://schemas.microsoft.com/office/spreadsheetml/2022/pivotVersionInfo
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/pivotVersionInfo"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2022/pivotVersionInfo"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="cacheVersionInfo" type="CT_CacheVersionInfo"/>
  <xsd:complexType name="CT_CacheVersionInfo">
    <xsd:sequence>
      <xsd:element name="requiredFeature" minOccurs="0" maxOccurs="unbounded"
type="xsd:string"/>
      <xsd:element name="lastRefreshFeature" minOccurs="0" maxOccurs="unbounded"
type="xsd:string"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="pivotVersionInfo" type="CT_PivotVersionInfo"/>
  <xsd:complexType name="CT_PivotVersionInfo">
    <xsd:sequence>
      <xsd:element name="requiredFeature" minOccurs="0" maxOccurs="unbounded"
type="xsd:string"/>
      <xsd:element name="lastUpdateFeature" minOccurs="0" maxOccurs="unbounded"
type="xsd:string"/>
    </xsd:sequence>
  </xsd:complexType>
 </xsd:schema>

<!-- page 410 -->

410 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
5.33 http://schemas.microsoft.com/office/spreadsheetml/2022/pivotRichData
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/pivotRichData"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2022/pivotRichData"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="richInfo" type="CT_PivotCacheRichInfo"/>
  <xsd:complexType name="CT_PivotCacheRichInfo">
    <xsd:attribute name="pivotCacheGuid" use="required" type="x:ST_Guid"/>
    <xsd:attribute name="pivotIgnoreInvalidCache" use="optional" type="xsd:boolean"
default="false"/>
    <xsd:attribute ref="r:id" use="optional"/>
  </xsd:complexType>
 </xsd:schema>
5.34 http://schemas.microsoft.com/office/spreadsheetml/2021/extlinks2021
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2021/extlinks2021"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2021/extlinks2021"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:complexType name="CT_ExternalBookAlternateUrls">
    <xsd:sequence>
      <xsd:element name="absoluteUrl" type="CT_AlternateUrl" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="relativeUrl" type="CT_AlternateUrl" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="driveId" use="optional" type="xsd:string"/>
    <xsd:attribute name="itemId" use="optional" type="xsd:string"/>
  </xsd:complexType>
  <xsd:complexType name="CT_AlternateUrl">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
  <xsd:element name="alternateUrls" type="CT_ExternalBookAlternateUrls"/>
 </xsd:schema>
5.35 http://schemas.microsoft.com/office/spreadsheetml/2022/richvaluerel
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2022/richvaluerel"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/richvaluerel"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="richValueRels" type="CT_RichValueRels"/>
  <xsd:complexType name="CT_RichValueRels">
    <xsd:sequence>

<!-- page 411 -->

411 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="rel" minOccurs="0" maxOccurs="unbounded"
type="CT_RichValueRelRelationship"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_RichValueRelRelationship">
    <xsd:attribute ref="r:id" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.36 http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag
Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag"
elementFormDefault="qualified"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <xsd:import namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
schemaLocation="orel.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="FeaturePropertyBags" type="CT_FeaturePropertyBags"/>
  <xsd:complexType name="CT_FeaturePropertyBags">
    <xsd:sequence>
      <xsd:element name="bagExt" type="CT_BagExtensions" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="bag" type="CT_FeaturePropertyBag" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="count" type="xsd:unsignedInt" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_BagExtensions">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_FeaturePropertyBag">
    <xsd:choice minOccurs="0" maxOccurs="unbounded">
      <xsd:element name="a" type="CT_ArrayFeatureProperty"/>
      <xsd:element name="bagId" type="CT_BagFeatureProperty"/>
      <xsd:element name="i" type="CT_IntFeatureProperty"/>
      <xsd:element name="s" type="CT_StringFeatureProperty"/>
      <xsd:element name="b" type="CT_BoolFeatureProperty"/>
      <xsd:element name="d" type="CT_DecimalFeatureProperty"/>
      <xsd:element name="rel" type="CT_RelFeatureProperty"/>
    </xsd:choice>
    <xsd:attribute name="type" type="xsd:string" use="required"/>
    <xsd:attribute name="extRef" type="xsd:string" use="optional"/>
    <xsd:attribute name="bagExtId" type="xsd:unsignedInt" use="optional"/>
    <xsd:attribute name="att" type="xsd:string" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_BagFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="xsd:unsignedInt">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_IntFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="xsd:integer">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>

<!-- page 412 -->

412 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_StringFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="xsd:string">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_BoolFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="xsd:boolean">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_DecimalFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="xsd:double">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_RelFeatureProperty">
    <xsd:simpleContent>
      <xsd:extension base="r:ST_RelationshipId">
        <xsd:attribute name="k" type="xsd:string" use="required"/>
      </xsd:extension>
    </xsd:simpleContent>
  </xsd:complexType>
  <xsd:complexType name="CT_ArrayFeatureProperty">
    <xsd:choice minOccurs="0" maxOccurs="unbounded">
      <xsd:element name="bagId" type="xsd:unsignedInt"/>
      <xsd:element name="i" type="xsd:integer"/>
      <xsd:element name="s" type="xsd:string"/>
      <xsd:element name="b" type="xsd:boolean"/>
      <xsd:element name="d" type="xsd:double"/>
      <xsd:element name="rel" type="r:ST_RelationshipId"/>
    </xsd:choice>
    <xsd:attribute name="k" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_XfComplementExt">
    <xsd:sequence>
      <xsd:element ref="xfComplement" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="xfComplement" type="CT_XfComplement"/>
  <xsd:complexType name="CT_XfComplement">
    <xsd:attribute name="i" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_DXFComplementExt">
    <xsd:sequence>
      <xsd:element ref="DXFComplement" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="DXFComplement" type="CT_DXFComplement"/>
  <xsd:complexType name="CT_DXFComplement">
    <xsd:attribute name="i" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.37 http://schemas.microsoft.com/office/spreadsheetml/2023/msForms Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2023/msForms"
elementFormDefault="qualified" xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2023/msForms">

<!-- page 413 -->

413 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:complexType name="CT_MsFormsExt">
    <xsd:sequence>
      <xsd:element name="msForm" type="CT_MsForm" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_QuestionExt">
    <xsd:sequence>
      <xsd:element name="question" type="CT_Question" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_MsForm">
    <xsd:sequence>
      <xsd:element name="syncedQuestionId" type="x:ST_Xstring" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="id" type="x:ST_Xstring" use="required"/>
    <xsd:attribute name="isFormConnected" type="xsd:boolean" use="optional" default="false"/>
    <xsd:attribute name="maxResponseId" type="xsd:int" use="optional"/>
    <xsd:attribute name="latestEventMarker" type="x:ST_Xstring" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Question">
    <xsd:sequence>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="id" type="x:ST_Xstring" use="required"/>
  </xsd:complexType>
 </xsd:schema>
5.38 http://schemas.microsoft.com/office/spreadsheetml/2023/externalCodeServic
e Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2023/externalCodeService"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2023/externalCodeService"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:complexType name="CT_ExternalCodeServiceExt">
    <xsd:sequence>
      <xsd:element ref="externalCodeService" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="externalCodeService" type="CT_ExternalCodeService"/>
  <xsd:complexType name="CT_ExternalCodeService">
    <xsd:attribute name="autoShow" type="xsd:unsignedInt" default="1"/>
    <xsd:attribute name="timeout" type="xsd:unsignedInt"/>
  </xsd:complexType>
 </xsd:schema>
5.39 http://schemas.microsoft.com/office/spreadsheetml/2023/python Schema
 <xsd:schema targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2023/python"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2023/python"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="python" type="CT_Python"/>
  <xsd:complexType name="CT_Python">
    <xsd:sequence>
      <xsd:element name="environmentDefinition" minOccurs="0" maxOccurs="1"
type="CT_PythonEnvironmentDefinition"/>

<!-- page 414 -->

414 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
      <xsd:element name="pythonScripts" minOccurs="0" maxOccurs="1" type="CT_PythonScripts"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PythonScripts">
    <xsd:sequence minOccurs="1" maxOccurs="unbounded">
      <xsd:element name="pythonScript" type="CT_PythonScript"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PythonScript">
    <xsd:sequence>
      <xsd:element name="code" type="x:ST_Xstring" minOccurs="0" maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PythonEnvironmentDefinition">
    <xsd:sequence>
      <xsd:element name="initialization" type="CT_PythonInitialization" minOccurs="0"
maxOccurs="1"/>
      <xsd:element name="extLst" type="x:CT_ExtensionList" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="id" type="x:ST_Guid"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PythonInitialization">
    <xsd:sequence>
      <xsd:element name="code" type="x:ST_Xstring" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="userModified" type="xsd:boolean" use="optional" default="false"/>
  </xsd:complexType>
 </xsd:schema>
5.40 http://schemas.microsoft.com/office/spreadsheetml/2023/pivot2023Calculatio
n Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2023/pivot2023Calculation"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2023/pivot2023Calculation"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="aggregationInfo" type="CT_AggregationInfo"/>
  <xsd:complexType name="CT_AggregationInfo">
    <xsd:attribute name="aggregationType" type="ST_AggregationType" use="required"/>
    <xsd:attribute name="sourceField" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_AggregationType">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="distinctCount"/>
      <xsd:enumeration value="median"/>
      <xsd:enumeration value="distinctDuplicates"/>
      <xsd:enumeration value="countValuesDuplicated"/>
      <xsd:enumeration value="countRepeatValues"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="featureSupportInfo" type="CT_FeatureSupport"/>
  <xsd:complexType name="CT_FeatureSupport">
    <xsd:attribute name="featureName" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:element name="pivotFieldSubtotals" type="CT_PivotFieldSubtotals"/>
  <xsd:complexType name="CT_PivotFieldSubtotals">
    <xsd:sequence>
      <xsd:element name="subtotal" type="CT_PivotItemSubtotal" minOccurs="0"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>

<!-- page 415 -->

415 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
  <xsd:complexType name="CT_PivotItemSubtotal">
    <xsd:attribute name="subtotalType" type="ST_AggregationType" use="required"/>
    <xsd:attribute name="itemLocation" type="xsd:unsignedInt" use="required"/>
  </xsd:complexType>
  <xsd:element name="pivotAreaReferenceSubtotals" type="CT_PivotAreaReferenceSubtotals"/>
  <xsd:complexType name="CT_PivotAreaReferenceSubtotals">
    <xsd:sequence>
      <xsd:element name="subtotal" type="CT_PivotSubtotalType" minOccurs="1"
maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PivotSubtotalType">
    <xsd:attribute name="subtotalType" type="ST_AggregationType" use="required"/>
  </xsd:complexType>
  <xsd:element name="pivotFieldSubtotalLineItems" type="CT_PivotTableSubtotalLineItems"/>
  <xsd:complexType name="CT_PivotTableSubtotalLineItems">
    <xsd:sequence>
      <xsd:element name="subtotalLineItem" minOccurs="1" maxOccurs="unbounded"
type="CT_PivotItemSubtotal"/>
    </xsd:sequence>
  </xsd:complexType>
 </xsd:schema>
5.41 http://schemas.microsoft.com/office/spreadsheetml/2024/pivotAutoRefresh
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2024/pivotAutoRefresh"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2024/pivotAutoRefresh"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="autoRefresh" type="xsd:boolean"/>
 </xsd:schema>
5.42 http://schemas.microsoft.com/office/spreadsheetml/2024/workbookCompatibi
lityVersion Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2024/workbookCompatibility
Version" elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2024/workbookCompatibilityVersion"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:complexType name="CT_WorkbookCompatibilityVersionExt">
    <xsd:sequence>
      <xsd:element ref="version" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="version" type="CT_Version"/>
  <xsd:complexType name="CT_Version">
    <xsd:attribute name="warnBelowVersion" type="xsd:unsignedInt" use="optional"
default="0"/>
    <xsd:attribute name="setVersion" type="xsd:unsignedInt"/>
  </xsd:complexType>
 </xsd:schema>

<!-- page 416 -->

416 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
5.43 http://schemas.microsoft.com/office/spreadsheetml/2023/showDataTypeIcon
s Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2023/showDataTypeIcons"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2023/showDataTypeIcons"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:complexType name="CT_ShowDataTypeIconsExt">
    <xsd:sequence>
      <xsd:element ref="showDataTypeIcons" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="showDataTypeIcons" type="CT_ShowDataTypeIcons"/>
  <xsd:complexType name="CT_ShowDataTypeIcons">
    <xsd:attribute name="visible" type="xsd:boolean" use="optional" default="true"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ShowDataTypeIconsCustomSheetViewExt">
    <xsd:sequence>
      <xsd:element ref="showDataTypeIconsCustomSheetView" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="showDataTypeIconsCustomSheetView" type="CT_ShowDataTypeIcons"/>
 </xsd:schema>
5.44 http://schemas.microsoft.com/office/spreadsheetml/2025/externalCodeServic
e2 Schema
 <xsd:schema
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2025/externalCodeService2"
elementFormDefault="qualified"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2025/externalCodeService2"
xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <xsd:complexType name="CT_ExternalCodeServiceImageAsInputExt">
    <xsd:sequence>
      <xsd:element ref="externalCodeServiceImageAsInput" minOccurs="1" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="externalCodeServiceImageAsInput"
type="CT_ExternalCodeServiceImageAsInput"/>
  <xsd:complexType name="CT_ExternalCodeServiceImageAsInput">
    <xsd:attribute name="maxWidth" type="xsd:unsignedInt" default="0"/>
    <xsd:attribute name="maxHeight" type="xsd:unsignedInt" default="0"/>
  </xsd:complexType>
 </xsd:schema>
5.45 http://schemas.microsoft.com/office/spreadsheetml/2025/pivotDataSource
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns="http://schemas.microsoft.com/office/spreadsheetml/2025/pivotDataSource"
xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"
targetNamespace="http://schemas.microsoft.com/office/spreadsheetml/2025/pivotDataSource"
elementFormDefault="qualified">
  <xsd:import namespace="http://schemas.microsoft.com/office/excel/2006/main"
schemaLocation="xlmbasictypes.xsd"/>
  <xsd:import namespace="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
schemaLocation="xlbasictypes.xsd"/>
  <xsd:element name="pivotCacheDataSource" type="CT_PivotCacheDataSource"/>
  <xsd:complexType name="CT_PivotCacheDataSource">

<!-- page 417 -->

417 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
    <xsd:sequence>
      <xsd:element ref="xm:f" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="ref" type="x:ST_Ref"/>
  </xsd:complexType>
 </xsd:schema>
