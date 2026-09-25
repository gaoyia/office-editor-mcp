# Annex E.  (informative) WordprocessingML Custom XML Data Extraction

<!-- source-pages 4422-4424 -->

<!-- page 4422 -->

ECMA-376 Part 1

4412

Annex E.
(informative)
WordprocessingML Custom XML Data
Extraction
This Annex is informative.
The custom XML markup capabilities described in §17.5 allow a WordprocessingML document to
contain custom XML semantics beyond those specified by ECMA-376. In order to extract those
semantics from within WordprocessingML content, an application may employ any desired method.
As an example, an XSL transformation which performs this task is included below, which, when applied
to the Main Document part, would extract any custom XML markup.
<?xml version="1.0" encoding="UTF-8" ?>
<xsl:stylesheet version="1.0"
xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
xmlns:w="http://purl.oclc.org/ooxml/wordprocessingml/main">

<!-- This parameter should have the same value as
ignoreMixedContent settings (see 17.15.1.54 in Part 1) -->
  <xsl:param name="ignoreMixedContent" select="false()"/>

  <!-- Some document structure checks -->
  <xsl:template match="/">
    <xsl:if test="count(//w:customXml/ancestor-or-
self::w:customXml[last()]) > 1">
      <xsl:message>Produced XML document will not be WF and will have more
then one root element.</xsl:message>
    </xsl:if>
    <!-- Process content of document -->
    <xsl:apply-templates/>
  </xsl:template>

  <!-- copy over custom XML elements -->
  <xsl:template match="w:customXml">
    <xsl:element name="{@w:element}" namespace="{@w:uri}">
      <!-- copy over attribute values -->

<!-- page 4423 -->

Annex E

4413
      <xsl:for-each select="w:customXmlPr/w:attr">
        <xsl:attribute name="{@w:name}" namespace="{@w:uri}">
          <xsl:value-of select="@w:val"/>
        </xsl:attribute>
      </xsl:for-each>
      <!-- process content -->
      <xsl:apply-templates/>
    </xsl:element>
  </xsl:template>

  <!-- copy over only text inside custom XML -->
  <xsl:template match="text()[ancestor::w:customXml[not(.//w:customXml)]]"
priority="10">
    <xsl:value-of select="."/>
  </xsl:template>

  <!-- warn about mixed content -->
  <xsl:template match="text()[ancestor::w:customXml]" priority="5">
    <xsl:choose>
      <xsl:when test="$ignoreMixedContent">
        <xsl:message>Stripping "<xsl:value-of select="."/>" from
output.</xsl:message>
        <xsl:message>This text is part of mixed content and would cause
non-valid result.</xsl:message>
      </xsl:when>
      <xsl:otherwise>
        <xsl:value-of select="."/>
      </xsl:otherwise>
    </xsl:choose>
  </xsl:template>

  <!-- warn about text which is not tagged -->
  <xsl:template match="text()">
    <xsl:message>Stripping "<xsl:value-of select="."/>" from
output.</xsl:message>
    <xsl:message>This text is not enclosed by root element and would cause
non-WF result.</xsl:message>
  </xsl:template>

  <!-- do not pick up deleted content -->
  <xsl:template match="w:del|w:moveFrom"/>

</xsl:stylesheet>

<!-- page 4424 -->

ECMA-376 Part 1

4414

Once this custom markup is extracted, the resulting XML document can be validated separately from the
WordprocessingML document.
For example, the custom XML for the example on p. 530, once extracted, would be:
<invoice xmlns="http://www.example.com/2006/invoice">
      <customerName>Tristan Davis</customerName>
</invoice>
An application can employ any desired method to find the appropriate schema(s) for validation. As an
example, one such approach using information defined by this Standard might be:

Locate the schema element (§23.2.1) in the Document Settings part whose uri attribute
matches the namespace of the root element in the XML document extracted from custom XML
markup

If that element also specifies a schemaLocation attribute, the resulting path is used to locate
the schema used for validation.

Once this schema is located, validation should be triggered based on the value of
doNotValidateAgainstSchema (§17.15.1.43).
End informative Annex.
