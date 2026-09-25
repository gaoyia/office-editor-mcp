# 5 Appendix A: Full XML Schemas

<!-- source-pages 98-110 -->

<!-- page 98 -->

98 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
5
Appendix A: Full XML Schemas
For ease of implementation, this section provides the full W3C XML Schemas for the new elements,
attributes, complex types, and simple types specified in the preceding sections. Any schema
references to namespaces included in ISO/IEC-29500:2008 refer specifically to the transitional
schemas as specified in [ISO/IEC29500-4:2016].
Schema name
Prefix
Section
http://schemas.microsoft.com/office/word/2012/wordml Schema
None.
5.2
http://schemas.microsoft.com/office/word/2016/wordml/cid Schema
None.
5.4
http://schemas.microsoft.com/office/word/2015/wordml/symex Schema
None.
5.3
http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash Schema
None.
5.7
http://schemas.microsoft.com/office/word/2024/wordml/sdtformatlock Schema
None.
5.9
http://schemas.microsoft.com/office/word/2023/wordml/word16du Schema
None.
5.8
http://schemas.microsoft.com/office/word/2010/wordml Schema
None.
5.1
http://schemas.microsoft.com/office/word/2018/wordml Schema
None.
5.5
http://schemas.microsoft.com/office/word/2018/wordml/cex Schema
None.
5.6

None.
5.10

5.1
http://schemas.microsoft.com/office/word/2010/wordml Schema
 <xsd:schema attributeFormDefault="qualified" blockDefault="#all"
elementFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/word/2010/wordml"
xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns="http://schemas.microsoft.com/office/word/2010/wordml">
  <xsd:import schemaLocation="oartbasetypes.xsd"
namespace="http://schemas.openxmlformats.org/drawingml/2006/main"/>
  <xsd:import schemaLocation="oartsplineproperties.xsd"
namespace="http://schemas.openxmlformats.org/drawingml/2006/main"/>
  <xsd:import id="rel" schemaLocation="orel.xsd"
namespace="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
  <xsd:import id="w" schemaLocation="word12.xsd"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>
  <xsd:complexType name="CT_LongHexNumber">
    <xsd:attribute name="val" type="w:ST_LongHexNumber" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_OnOff">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="true"/>
      <xsd:enumeration value="false"/>
      <xsd:enumeration value="0"/>
      <xsd:enumeration value="1"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_OnOff">
    <xsd:attribute name="val" type="ST_OnOff"/>
  </xsd:complexType>

<!-- page 99 -->

99 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
  <xsd:element name="docId" type="CT_LongHexNumber"/>
  <xsd:element name="conflictMode" type="CT_OnOff"/>
  <xsd:attributeGroup name="AG_Parids">
    <xsd:attribute name="paraId" type="w:ST_LongHexNumber"/>
    <xsd:attribute name="textId" type="w:ST_LongHexNumber"/>
  </xsd:attributeGroup>
  <xsd:attribute name="anchorId" type="w:ST_LongHexNumber"/>
  <xsd:attribute name="noSpellErr" type="w:ST_OnOff"/>
  <xsd:element name="customXmlConflictInsRangeStart" type="w:CT_TrackChange"/>
  <xsd:element name="customXmlConflictInsRangeEnd" type="w:CT_Markup"/>
  <xsd:element name="customXmlConflictDelRangeStart" type="w:CT_TrackChange"/>
  <xsd:element name="customXmlConflictDelRangeEnd" type="w:CT_Markup"/>
  <xsd:group name="EG_RunLevelConflicts">
    <xsd:sequence>
      <xsd:element name="conflictIns" type="w:CT_RunTrackChange" minOccurs="0"/>
      <xsd:element name="conflictDel" type="w:CT_RunTrackChange" minOccurs="0"/>
    </xsd:sequence>
  </xsd:group>
  <xsd:group name="EG_Conflicts">
    <xsd:choice>
      <xsd:element name="conflictIns" type="w:CT_TrackChange" minOccurs="0"/>
      <xsd:element name="conflictDel" type="w:CT_TrackChange" minOccurs="0"/>
    </xsd:choice>
  </xsd:group>
  <xsd:complexType name="CT_Percentage">
    <xsd:attribute name="val" type="a:ST_Percentage" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PositiveFixedPercentage">
    <xsd:attribute name="val" type="a:ST_PositiveFixedPercentage" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PositivePercentage">
    <xsd:attribute name="val" type="a:ST_PositivePercentage" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_SchemeColorVal">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="bg1"/>
      <xsd:enumeration value="tx1"/>
      <xsd:enumeration value="bg2"/>
      <xsd:enumeration value="tx2"/>
      <xsd:enumeration value="accent1"/>
      <xsd:enumeration value="accent2"/>
      <xsd:enumeration value="accent3"/>
      <xsd:enumeration value="accent4"/>
      <xsd:enumeration value="accent5"/>
      <xsd:enumeration value="accent6"/>
      <xsd:enumeration value="hlink"/>
      <xsd:enumeration value="folHlink"/>
      <xsd:enumeration value="dk1"/>
      <xsd:enumeration value="lt1"/>
      <xsd:enumeration value="dk2"/>
      <xsd:enumeration value="lt2"/>
      <xsd:enumeration value="phClr"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_RectAlignment">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="none"/>
      <xsd:enumeration value="tl"/>
      <xsd:enumeration value="t"/>
      <xsd:enumeration value="tr"/>
      <xsd:enumeration value="l"/>
      <xsd:enumeration value="ctr"/>
      <xsd:enumeration value="r"/>
      <xsd:enumeration value="bl"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="br"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_PathShadeType">

<!-- page 100 -->

100 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="shape"/>
      <xsd:enumeration value="circle"/>
      <xsd:enumeration value="rect"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_LineCap">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="rnd"/>
      <xsd:enumeration value="sq"/>
      <xsd:enumeration value="flat"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_PresetLineDashVal">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="solid"/>
      <xsd:enumeration value="dot"/>
      <xsd:enumeration value="sysDot"/>
      <xsd:enumeration value="dash"/>
      <xsd:enumeration value="sysDash"/>
      <xsd:enumeration value="lgDash"/>
      <xsd:enumeration value="dashDot"/>
      <xsd:enumeration value="sysDashDot"/>
      <xsd:enumeration value="lgDashDot"/>
      <xsd:enumeration value="lgDashDotDot"/>
      <xsd:enumeration value="sysDashDotDot"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_PenAlignment">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="ctr"/>
      <xsd:enumeration value="in"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_CompoundLine">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="sng"/>
      <xsd:enumeration value="dbl"/>
      <xsd:enumeration value="thickThin"/>
      <xsd:enumeration value="thinThick"/>
      <xsd:enumeration value="tri"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_RelativeRect">
    <xsd:attribute name="l" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="t" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="r" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="b" use="optional" type="a:ST_Percentage"/>
  </xsd:complexType>
  <xsd:group name="EG_ColorTransform">
    <xsd:choice>
      <xsd:element name="tint" type="CT_PositiveFixedPercentage"/>
      <xsd:element name="shade" type="CT_PositiveFixedPercentage"/>
      <xsd:element name="alpha" type="CT_PositiveFixedPercentage"/>
      <xsd:element name="hueMod" type="CT_PositivePercentage"/>
      <xsd:element name="sat" type="CT_Percentage"/>
      <xsd:element name="satOff" type="CT_Percentage"/>
      <xsd:element name="satMod" type="CT_Percentage"/>
      <xsd:element name="lum" type="CT_Percentage"/>
      <xsd:element name="lumOff" type="CT_Percentage"/>
      <xsd:element name="lumMod" type="CT_Percentage"/>
    </xsd:choice>
  </xsd:group>
  <xsd:complexType name="CT_SRgbColor">
    <xsd:sequence>
      <xsd:group ref="EG_ColorTransform" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="val" type="w:ST_HexColorRGB" use="required"/>
  </xsd:complexType>

<!-- page 101 -->

101 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
  <xsd:complexType name="CT_SchemeColor">
    <xsd:sequence>
      <xsd:group ref="EG_ColorTransform" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
    <xsd:attribute name="val" type="ST_SchemeColorVal" use="required"/>
  </xsd:complexType>
  <xsd:group name="EG_ColorChoice">
    <xsd:choice>
      <xsd:element name="srgbClr" type="CT_SRgbColor"/>
      <xsd:element name="schemeClr" type="CT_SchemeColor"/>
    </xsd:choice>
  </xsd:group>
  <xsd:complexType name="CT_Color">
    <xsd:sequence>
      <xsd:group ref="EG_ColorChoice"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_GradientStop">
    <xsd:sequence>
      <xsd:group ref="EG_ColorChoice"/>
    </xsd:sequence>
    <xsd:attribute name="pos" type="a:ST_PositiveFixedPercentage" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_GradientStopList">
    <xsd:sequence>
      <xsd:element name="gs" type="CT_GradientStop" minOccurs="2" maxOccurs="10"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_LinearShadeProperties">
    <xsd:attribute name="ang" type="a:ST_PositiveFixedAngle" use="optional"/>
    <xsd:attribute name="scaled" type="ST_OnOff" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_PathShadeProperties">
    <xsd:sequence>
      <xsd:element name="fillToRect" type="CT_RelativeRect" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="path" type="ST_PathShadeType" use="optional"/>
  </xsd:complexType>
  <xsd:group name="EG_ShadeProperties">
    <xsd:choice>
      <xsd:element name="lin" type="CT_LinearShadeProperties"/>
      <xsd:element name="path" type="CT_PathShadeProperties"/>
    </xsd:choice>
  </xsd:group>
  <xsd:complexType name="CT_SolidColorFillProperties">
    <xsd:sequence>
      <xsd:group ref="EG_ColorChoice" minOccurs="0"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_GradientFillProperties">
    <xsd:sequence>
      <xsd:element name="gsLst" type="CT_GradientStopList" minOccurs="0"/>
      <xsd:group ref="EG_ShadeProperties" minOccurs="0"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:group name="EG_FillProperties">
    <xsd:choice>
      <xsd:element name="noFill" type="w:CT_Empty"/>
      <xsd:element name="solidFill" type="CT_SolidColorFillProperties"/>
      <xsd:element name="gradFill" type="CT_GradientFillProperties"/>
    </xsd:choice>
  </xsd:group>
  <xsd:complexType name="CT_PresetLineDashProperties">
    <xsd:attribute name="val" type="ST_PresetLineDashVal" use="optional"/>
  </xsd:complexType>
  <xsd:group name="EG_LineDashProperties">
    <xsd:choice>
      <xsd:element name="prstDash" type="CT_PresetLineDashProperties"/>
    </xsd:choice>

<!-- page 102 -->

102 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
  </xsd:group>
  <xsd:complexType name="CT_LineJoinMiterProperties">
    <xsd:attribute name="lim" type="a:ST_PositivePercentage" use="optional"/>
  </xsd:complexType>
  <xsd:group name="EG_LineJoinProperties">
    <xsd:choice>
      <xsd:element name="round" type="w:CT_Empty"/>
      <xsd:element name="bevel" type="w:CT_Empty"/>
      <xsd:element name="miter" type="CT_LineJoinMiterProperties"/>
    </xsd:choice>
  </xsd:group>
  <xsd:simpleType name="ST_PresetCameraType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="legacyObliqueTopLeft"/>
      <xsd:enumeration value="legacyObliqueTop"/>
      <xsd:enumeration value="legacyObliqueTopRight"/>
      <xsd:enumeration value="legacyObliqueLeft"/>
      <xsd:enumeration value="legacyObliqueFront"/>
      <xsd:enumeration value="legacyObliqueRight"/>
      <xsd:enumeration value="legacyObliqueBottomLeft"/>
      <xsd:enumeration value="legacyObliqueBottom"/>
      <xsd:enumeration value="legacyObliqueBottomRight"/>
      <xsd:enumeration value="legacyPerspectiveTopLeft"/>
      <xsd:enumeration value="legacyPerspectiveTop"/>
      <xsd:enumeration value="legacyPerspectiveTopRight"/>
      <xsd:enumeration value="legacyPerspectiveLeft"/>
      <xsd:enumeration value="legacyPerspectiveFront"/>
      <xsd:enumeration value="legacyPerspectiveRight"/>
      <xsd:enumeration value="legacyPerspectiveBottomLeft"/>
      <xsd:enumeration value="legacyPerspectiveBottom"/>
      <xsd:enumeration value="legacyPerspectiveBottomRight"/>
      <xsd:enumeration value="orthographicFront"/>
      <xsd:enumeration value="isometricTopUp"/>
      <xsd:enumeration value="isometricTopDown"/>
      <xsd:enumeration value="isometricBottomUp"/>
      <xsd:enumeration value="isometricBottomDown"/>
      <xsd:enumeration value="isometricLeftUp"/>
      <xsd:enumeration value="isometricLeftDown"/>
      <xsd:enumeration value="isometricRightUp"/>
      <xsd:enumeration value="isometricRightDown"/>
      <xsd:enumeration value="isometricOffAxis1Left"/>
      <xsd:enumeration value="isometricOffAxis1Right"/>
      <xsd:enumeration value="isometricOffAxis1Top"/>
      <xsd:enumeration value="isometricOffAxis2Left"/>
      <xsd:enumeration value="isometricOffAxis2Right"/>
      <xsd:enumeration value="isometricOffAxis2Top"/>
      <xsd:enumeration value="isometricOffAxis3Left"/>
      <xsd:enumeration value="isometricOffAxis3Right"/>
      <xsd:enumeration value="isometricOffAxis3Bottom"/>
      <xsd:enumeration value="isometricOffAxis4Left"/>
      <xsd:enumeration value="isometricOffAxis4Right"/>
      <xsd:enumeration value="isometricOffAxis4Bottom"/>
      <xsd:enumeration value="obliqueTopLeft"/>
      <xsd:enumeration value="obliqueTop"/>
      <xsd:enumeration value="obliqueTopRight"/>
      <xsd:enumeration value="obliqueLeft"/>
      <xsd:enumeration value="obliqueRight"/>
      <xsd:enumeration value="obliqueBottomLeft"/>
      <xsd:enumeration value="obliqueBottom"/>
      <xsd:enumeration value="obliqueBottomRight"/>
      <xsd:enumeration value="perspectiveFront"/>
      <xsd:enumeration value="perspectiveLeft"/>
      <xsd:enumeration value="perspectiveRight"/>
      <xsd:enumeration value="perspectiveAbove"/>
      <xsd:enumeration value="perspectiveBelow"/>
      <xsd:enumeration value="perspectiveAboveLeftFacing"/>
      <xsd:enumeration value="perspectiveAboveRightFacing"/>
      <xsd:enumeration value="perspectiveContrastingLeftFacing"/>
      <xsd:enumeration value="perspectiveContrastingRightFacing"/>

<!-- page 103 -->

103 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
      <xsd:enumeration value="perspectiveHeroicLeftFacing"/>
      <xsd:enumeration value="perspectiveHeroicRightFacing"/>
      <xsd:enumeration value="perspectiveHeroicExtremeLeftFacing"/>
      <xsd:enumeration value="perspectiveHeroicExtremeRightFacing"/>
      <xsd:enumeration value="perspectiveRelaxed"/>
      <xsd:enumeration value="perspectiveRelaxedModerately"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Camera">
    <xsd:attribute name="prst" use="required" type="ST_PresetCameraType"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SphereCoords">
    <xsd:attribute name="lat" type="a:ST_PositiveFixedAngle" use="required"/>
    <xsd:attribute name="lon" type="a:ST_PositiveFixedAngle" use="required"/>
    <xsd:attribute name="rev" type="a:ST_PositiveFixedAngle" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_LightRigType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="legacyFlat1"/>
      <xsd:enumeration value="legacyFlat2"/>
      <xsd:enumeration value="legacyFlat3"/>
      <xsd:enumeration value="legacyFlat4"/>
      <xsd:enumeration value="legacyNormal1"/>
      <xsd:enumeration value="legacyNormal2"/>
      <xsd:enumeration value="legacyNormal3"/>
      <xsd:enumeration value="legacyNormal4"/>
      <xsd:enumeration value="legacyHarsh1"/>
      <xsd:enumeration value="legacyHarsh2"/>
      <xsd:enumeration value="legacyHarsh3"/>
      <xsd:enumeration value="legacyHarsh4"/>
      <xsd:enumeration value="threePt"/>
      <xsd:enumeration value="balanced"/>
      <xsd:enumeration value="soft"/>
      <xsd:enumeration value="harsh"/>
      <xsd:enumeration value="flood"/>
      <xsd:enumeration value="contrasting"/>
      <xsd:enumeration value="morning"/>
      <xsd:enumeration value="sunrise"/>
      <xsd:enumeration value="sunset"/>
      <xsd:enumeration value="chilly"/>
      <xsd:enumeration value="freezing"/>
      <xsd:enumeration value="flat"/>
      <xsd:enumeration value="twoPt"/>
      <xsd:enumeration value="glow"/>
      <xsd:enumeration value="brightRoom"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:simpleType name="ST_LightRigDirection">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="tl"/>
      <xsd:enumeration value="t"/>
      <xsd:enumeration value="tr"/>
      <xsd:enumeration value="l"/>
      <xsd:enumeration value="r"/>
      <xsd:enumeration value="bl"/>
      <xsd:enumeration value="b"/>
      <xsd:enumeration value="br"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_LightRig">
    <xsd:sequence>
      <xsd:element name="rot" type="CT_SphereCoords" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="rig" type="ST_LightRigType" use="required"/>
    <xsd:attribute name="dir" type="ST_LightRigDirection" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_BevelPresetType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="relaxedInset"/>

<!-- page 104 -->

104 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
      <xsd:enumeration value="circle"/>
      <xsd:enumeration value="slope"/>
      <xsd:enumeration value="cross"/>
      <xsd:enumeration value="angle"/>
      <xsd:enumeration value="softRound"/>
      <xsd:enumeration value="convex"/>
      <xsd:enumeration value="coolSlant"/>
      <xsd:enumeration value="divot"/>
      <xsd:enumeration value="riblet"/>
      <xsd:enumeration value="hardEdge"/>
      <xsd:enumeration value="artDeco"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Bevel">
    <xsd:attribute name="w" type="a:ST_PositiveCoordinate" use="optional"/>
    <xsd:attribute name="h" type="a:ST_PositiveCoordinate" use="optional"/>
    <xsd:attribute name="prst" type="ST_BevelPresetType" use="optional"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_PresetMaterialType">
    <xsd:restriction base="xsd:token">
      <xsd:enumeration value="legacyMatte"/>
      <xsd:enumeration value="legacyPlastic"/>
      <xsd:enumeration value="legacyMetal"/>
      <xsd:enumeration value="legacyWireframe"/>
      <xsd:enumeration value="matte"/>
      <xsd:enumeration value="plastic"/>
      <xsd:enumeration value="metal"/>
      <xsd:enumeration value="warmMatte"/>
      <xsd:enumeration value="translucentPowder"/>
      <xsd:enumeration value="powder"/>
      <xsd:enumeration value="dkEdge"/>
      <xsd:enumeration value="softEdge"/>
      <xsd:enumeration value="clear"/>
      <xsd:enumeration value="flat"/>
      <xsd:enumeration value="softmetal"/>
      <xsd:enumeration value="none"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Glow">
    <xsd:sequence>
      <xsd:group ref="EG_ColorChoice"/>
    </xsd:sequence>
    <xsd:attribute name="rad" use="optional" type="a:ST_PositiveCoordinate"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Shadow">
    <xsd:sequence>
      <xsd:group ref="EG_ColorChoice"/>
    </xsd:sequence>
    <xsd:attribute name="blurRad" use="optional" type="a:ST_PositiveCoordinate"/>
    <xsd:attribute name="dist" use="optional" type="a:ST_PositiveCoordinate"/>
    <xsd:attribute name="dir" use="optional" type="a:ST_PositiveFixedAngle"/>
    <xsd:attribute name="sx" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="sy" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="kx" use="optional" type="a:ST_FixedAngle"/>
    <xsd:attribute name="ky" use="optional" type="a:ST_FixedAngle"/>
    <xsd:attribute name="algn" use="optional" type="ST_RectAlignment"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Reflection">
    <xsd:attribute name="blurRad" use="optional" type="a:ST_PositiveCoordinate"/>
    <xsd:attribute name="stA" use="optional" type="a:ST_PositiveFixedPercentage"/>
    <xsd:attribute name="stPos" use="optional" type="a:ST_PositiveFixedPercentage"/>
    <xsd:attribute name="endA" use="optional" type="a:ST_PositiveFixedPercentage"/>
    <xsd:attribute name="endPos" use="optional" type="a:ST_PositiveFixedPercentage"/>
    <xsd:attribute name="dist" use="optional" type="a:ST_PositiveCoordinate"/>
    <xsd:attribute name="dir" use="optional" type="a:ST_PositiveFixedAngle"/>
    <xsd:attribute name="fadeDir" use="optional" type="a:ST_PositiveFixedAngle"/>
    <xsd:attribute name="sx" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="sy" use="optional" type="a:ST_Percentage"/>
    <xsd:attribute name="kx" use="optional" type="a:ST_FixedAngle"/>

<!-- page 105 -->

105 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
    <xsd:attribute name="ky" use="optional" type="a:ST_FixedAngle"/>
    <xsd:attribute name="algn" use="optional" type="ST_RectAlignment"/>
  </xsd:complexType>
  <xsd:complexType name="CT_FillTextEffect">
    <xsd:sequence>
      <xsd:group ref="EG_FillProperties" minOccurs="0"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_TextOutlineEffect">
    <xsd:sequence>
      <xsd:group ref="EG_FillProperties" minOccurs="0"/>
      <xsd:group ref="EG_LineDashProperties" minOccurs="0"/>
      <xsd:group ref="EG_LineJoinProperties" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="w" use="optional" type="a:ST_LineWidth"/>
    <xsd:attribute name="cap" use="optional" type="ST_LineCap"/>
    <xsd:attribute name="cmpd" use="optional" type="ST_CompoundLine"/>
    <xsd:attribute name="algn" use="optional" type="ST_PenAlignment"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Scene3D">
    <xsd:sequence>
      <xsd:element name="camera" type="CT_Camera"/>
      <xsd:element name="lightRig" type="CT_LightRig"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_Props3D">
    <xsd:sequence>
      <xsd:element name="bevelT" type="CT_Bevel" minOccurs="0"/>
      <xsd:element name="bevelB" type="CT_Bevel" minOccurs="0"/>
      <xsd:element name="extrusionClr" type="CT_Color" minOccurs="0"/>
      <xsd:element name="contourClr" type="CT_Color" minOccurs="0"/>
    </xsd:sequence>
    <xsd:attribute name="extrusionH" type="a:ST_PositiveCoordinate" use="optional"/>
    <xsd:attribute name="contourW" type="a:ST_PositiveCoordinate" use="optional"/>
    <xsd:attribute name="prstMaterial" type="ST_PresetMaterialType" use="optional"/>
  </xsd:complexType>
  <xsd:group name="EG_RPrTextEffects">
    <xsd:sequence>
      <xsd:element name="glow" minOccurs="0" type="CT_Glow"/>
      <xsd:element name="shadow" minOccurs="0" type="CT_Shadow"/>
      <xsd:element name="reflection" minOccurs="0" type="CT_Reflection"/>
      <xsd:element name="textOutline" minOccurs="0" type="CT_TextOutlineEffect"/>
      <xsd:element name="textFill" minOccurs="0" type="CT_FillTextEffect"/>
      <xsd:element name="scene3d" minOccurs="0" type="CT_Scene3D"/>
      <xsd:element name="props3d" minOccurs="0" type="CT_Props3D"/>
    </xsd:sequence>
  </xsd:group>
  <xsd:simpleType name="ST_Ligatures">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="none"/>
      <xsd:enumeration value="standard"/>
      <xsd:enumeration value="contextual"/>
      <xsd:enumeration value="historical"/>
      <xsd:enumeration value="discretional"/>
      <xsd:enumeration value="standardContextual"/>
      <xsd:enumeration value="standardHistorical"/>
      <xsd:enumeration value="contextualHistorical"/>
      <xsd:enumeration value="standardDiscretional"/>
      <xsd:enumeration value="contextualDiscretional"/>
      <xsd:enumeration value="historicalDiscretional"/>
      <xsd:enumeration value="standardContextualHistorical"/>
      <xsd:enumeration value="standardContextualDiscretional"/>
      <xsd:enumeration value="standardHistoricalDiscretional"/>
      <xsd:enumeration value="contextualHistoricalDiscretional"/>
      <xsd:enumeration value="all"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Ligatures">
    <xsd:attribute name="val" type="ST_Ligatures" use="required"/>

<!-- page 106 -->

106 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
  </xsd:complexType>
  <xsd:simpleType name="ST_NumForm">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="default"/>
      <xsd:enumeration value="lining"/>
      <xsd:enumeration value="oldStyle"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_NumForm">
    <xsd:attribute name="val" type="ST_NumForm" use="required"/>
  </xsd:complexType>
  <xsd:simpleType name="ST_NumSpacing">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="default"/>
      <xsd:enumeration value="proportional"/>
      <xsd:enumeration value="tabular"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_NumSpacing">
    <xsd:attribute name="val" type="ST_NumSpacing" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_StyleSet">
    <xsd:attribute name="id" type="w:ST_UnsignedDecimalNumber" use="required"/>
    <xsd:attribute name="val" type="ST_OnOff" use="optional"/>
  </xsd:complexType>
  <xsd:complexType name="CT_StylisticSets">
    <xsd:sequence minOccurs="0">
      <xsd:element name="styleSet" minOccurs="0" maxOccurs="unbounded" type="CT_StyleSet"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:group name="EG_RPrOpenType">
    <xsd:sequence>
      <xsd:element name="ligatures" minOccurs="0" type="CT_Ligatures"/>
      <xsd:element name="numForm" minOccurs="0" type="CT_NumForm"/>
      <xsd:element name="numSpacing" minOccurs="0" type="CT_NumSpacing"/>
      <xsd:element name="stylisticSets" minOccurs="0" type="CT_StylisticSets"/>
      <xsd:element name="cntxtAlts" minOccurs="0" type="CT_OnOff"/>
    </xsd:sequence>
  </xsd:group>
  <xsd:element name="discardImageEditingData" type="CT_OnOff"/>
  <xsd:element name="defaultImageDpi" type="CT_DefaultImageDpi"/>
  <xsd:complexType name="CT_DefaultImageDpi">
    <xsd:attribute name="val" type="w:ST_DecimalNumber" use="required"/>
  </xsd:complexType>
  <xsd:element name="entityPicker" type="w:CT_Empty"/>
  <xsd:complexType name="CT_SdtCheckboxSymbol">
    <xsd:attribute name="font" type="w:ST_String"/>
    <xsd:attribute name="val" type="w:ST_ShortHexNumber"/>
  </xsd:complexType>
  <xsd:complexType name="CT_SdtCheckbox">
    <xsd:sequence>
      <xsd:element name="checked" type="CT_OnOff" minOccurs="0"/>
      <xsd:element name="checkedState" type="CT_SdtCheckboxSymbol" minOccurs="0"/>
      <xsd:element name="uncheckedState" type="CT_SdtCheckboxSymbol" minOccurs="0"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:element name="checkbox" type="CT_SdtCheckbox"/>
 </xsd:schema>
5.2
http://schemas.microsoft.com/office/word/2012/wordml Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2012/wordml"
targetNamespace="http://schemas.microsoft.com/office/word/2012/wordml">

<!-- page 107 -->

107 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:element name="color" type="w12:CT_Color"/>
  <xsd:simpleType name="ST_SdtAppearance">
    <xsd:restriction base="xsd:string">
      <xsd:enumeration value="boundingBox"/>
      <xsd:enumeration value="tags"/>
      <xsd:enumeration value="hidden"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:element name="dataBinding" type="w12:CT_DataBinding"/>
  <xsd:complexType name="CT_SdtAppearance">
    <xsd:attribute name="val" type="ST_SdtAppearance"/>
  </xsd:complexType>
  <xsd:element name="appearance" type="CT_SdtAppearance"/>
  <xsd:complexType name="CT_CommentsEx">
    <xsd:sequence>
      <xsd:element name="commentEx" type="CT_CommentEx" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CommentEx">
    <xsd:attribute name="paraId" type="w12:ST_LongHexNumber" use="required"/>
    <xsd:attribute name="paraIdParent" type="w12:ST_LongHexNumber" use="optional"/>
    <xsd:attribute name="done" type="w12:ST_OnOff" use="optional"/>
  </xsd:complexType>
  <xsd:element name="commentsEx" type="CT_CommentsEx"/>
  <xsd:complexType name="CT_People">
    <xsd:sequence>
      <xsd:element name="person" type="CT_Person" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_PresenceInfo">
    <xsd:attribute name="providerId" type="xsd:string" use="required"/>
    <xsd:attribute name="userId" type="xsd:string" use="required"/>
  </xsd:complexType>
  <xsd:complexType name="CT_Person">
    <xsd:sequence>
      <xsd:element name="presenceInfo" type="CT_PresenceInfo" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="author" type="w12:ST_String" use="required"/>
  </xsd:complexType>
  <xsd:element name="people" type="CT_People"/>
  <xsd:complexType name="CT_SdtRepeatedSection">
    <xsd:sequence>
      <xsd:element name="sectionTitle" type="w12:CT_String" minOccurs="0"/>
      <xsd:element name="doNotAllowInsertDeleteSection" type="w12:CT_OnOff" minOccurs="0"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:simpleType name="ST_Guid">
    <xsd:restriction base="xsd:token">
      <xsd:pattern value="\{[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}\}"/>
    </xsd:restriction>
  </xsd:simpleType>
  <xsd:complexType name="CT_Guid">
    <xsd:attribute name="val" type="ST_Guid"/>
  </xsd:complexType>
  <xsd:element name="repeatingSection" type="CT_SdtRepeatedSection"/>
  <xsd:element name="repeatingSectionItem" type="w12:CT_Empty"/>
  <xsd:element name="chartTrackingRefBased" type="w12:CT_OnOff"/>
  <xsd:element name="collapsed" type="w12:CT_OnOff"/>
  <xsd:element name="docId" type="CT_Guid"/>
  <xsd:element name="footnoteColumns" type="w12:CT_DecimalNumber"/>
  <xsd:element name="webExtensionLinked" type="w12:CT_OnOff"/>
  <xsd:element name="webExtensionCreated" type="w12:CT_OnOff"/>
  <xsd:attribute name="restartNumberingAfterBreak" type="w12:ST_OnOff"/>
 </xsd:schema>

<!-- page 108 -->

108 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
5.3
http://schemas.microsoft.com/office/word/2015/wordml/symex Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2015/wordml/symex"
targetNamespace="http://schemas.microsoft.com/office/word/2015/wordml/symex">
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:complexType name="CT_SymEx">
    <xsd:attribute name="font" type="w12:ST_String"/>
    <xsd:attribute name="char" type="w12:ST_LongHexNumber"/>
  </xsd:complexType>
  <xsd:element name="symEx" type="CT_SymEx"/>
 </xsd:schema>
5.4
http://schemas.microsoft.com/office/word/2016/wordml/cid Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2016/wordml/cid"
targetNamespace="http://schemas.microsoft.com/office/word/2016/wordml/cid">
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:attributeGroup name="decimaldurableId">
    <xsd:attribute name="durableId" type="w12:ST_DecimalNumber" use="optional"/>
  </xsd:attributeGroup>
  <xsd:attribute name="durableId" type="w12:ST_LongHexNumber"/>
  <xsd:complexType name="CT_CommentsIds">
    <xsd:sequence>
      <xsd:element name="commentId" type="CT_CommentId" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CommentId">
    <xsd:attribute name="paraId" type="w12:ST_LongHexNumber" use="required"/>
    <xsd:attribute name="durableId" type="w12:ST_LongHexNumber" use="required"/>
  </xsd:complexType>
  <xsd:element name="commentsIds" type="CT_CommentsIds"/>
 </xsd:schema>
5.5
http://schemas.microsoft.com/office/word/2018/wordml Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2018/wordml"
targetNamespace="http://schemas.microsoft.com/office/word/2018/wordml">
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:complexType name="CT_Extension">
    <xsd:sequence>
      <xsd:any processContents="lax"/>
    </xsd:sequence>
    <xsd:attribute name="uri" type="xsd:token"/>
  </xsd:complexType>
  <xsd:complexType name="CT_ExtensionList">
    <xsd:sequence>
      <xsd:element name="ext" type="CT_Extension" minOccurs="0" maxOccurs="unbounded"/>
    </xsd:sequence>
  </xsd:complexType>

<!-- page 109 -->

109 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
 </xsd:schema>
5.6
http://schemas.microsoft.com/office/word/2018/wordml/cex Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
xmlns:w16="http://schemas.microsoft.com/office/word/2018/wordml"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2018/wordml/cex"
targetNamespace="http://schemas.microsoft.com/office/word/2018/wordml/cex">
  <xsd:import id="w16" namespace="http://schemas.microsoft.com/office/word/2018/wordml"
schemaLocation="word16.xsd"/>
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:complexType name="CT_CommentsExtensible">
    <xsd:sequence>
      <xsd:element name="commentExtensible" type="CT_CommentExtensible" minOccurs="0"
maxOccurs="unbounded"/>
      <xsd:element name="extLst" type="w16:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
  </xsd:complexType>
  <xsd:complexType name="CT_CommentExtensible">
    <xsd:sequence>
      <xsd:element name="extLst" type="w16:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="durableId" type="w12:ST_LongHexNumber" use="required"/>
    <xsd:attribute name="dateUtc" type="w12:ST_DateTime" use="optional"/>
    <xsd:attribute name="intelligentPlaceholder" type="w12:ST_OnOff" use="optional"/>
  </xsd:complexType>
  <xsd:element name="commentsExtensible" type="CT_CommentsExtensible"/>
 </xsd:schema>
5.7
http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash"
targetNamespace="http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash">
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:attribute name="storeItemChecksum" type="w12:ST_String"/>
 </xsd:schema>
5.8
http://schemas.microsoft.com/office/word/2023/wordml/word16du Schema
 <xsd:schema xmlns="http://schemas.microsoft.com/office/word/2023/wordml/word16du"
xmlns:xsd="http://www.w3.org/2001/XMLSchema" blockDefault="#all"
elementFormDefault="qualified" attributeFormDefault="qualified"
targetNamespace="http://schemas.microsoft.com/office/word/2023/wordml/word16du">
  <xsd:attribute name="dateUtc" type="xsd:dateTime"/>
 </xsd:schema>
5.9
http://schemas.microsoft.com/office/word/2024/wordml/sdtformatlock
Schema
 <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
xmlns:w12="http://schemas.openxmlformats.org/wordprocessingml/2006/main"

<!-- page 110 -->

110 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
elementFormDefault="qualified" attributeFormDefault="qualified" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2024/wordml/sdtformatlock"
targetNamespace="http://schemas.microsoft.com/office/word/2024/wordml/sdtformatlock">
  <xsd:import id="w12"
namespace="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
schemaLocation="word12.xsd"/>
  <xsd:attribute name="formattingAllowed" type="w12:ST_OnOff"/>
 </xsd:schema>
5.10 http://schemas.microsoft.com/office/word/2026/wordml/cei Schema
 <xsd:schema xmlns:w16="http://schemas.microsoft.com/office/word/2018/wordml"
xmlns:xsd="http://www.w3.org/2001/XMLSchema" blockDefault="#all"
xmlns="http://schemas.microsoft.com/office/word/2026/wordml/cei"
targetNamespace="http://schemas.microsoft.com/office/word/2026/wordml/cei"
elementFormDefault="qualified" attributeFormDefault="qualified">
  <xsd:import id="w16" namespace="http://schemas.microsoft.com/office/word/2018/wordml"
schemaLocation="word16.xsd"/>
  <xsd:complexType name="CT_CommentEntityInfo">
    <xsd:sequence>
      <xsd:element name="extLst" type="w16:CT_ExtensionList" minOccurs="0" maxOccurs="1"/>
    </xsd:sequence>
    <xsd:attribute name="entityType" type="xsd:unsignedInt" use="optional" default="0"/>
  </xsd:complexType>
  <xsd:element name="commentEntityInfo" type="CT_CommentEntityInfo"/>
 </xsd:schema>
