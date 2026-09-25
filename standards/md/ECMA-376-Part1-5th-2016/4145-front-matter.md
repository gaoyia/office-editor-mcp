# front-matter

<!-- source-pages 4145-4145 -->

<!-- page 4145 -->

Annex B

4135
Annex B.
(informative)
Schemas – RELAX NG
This clause is informative.
This Office Open XML specification includes a family of schemas defined using the RELAX NG syntax. The
definitions of these schemas follow below, and they also  reside in an accompanying file named OfficeOpenXML-
RELAXNG-Strict.zip, which is distributed in electronic form.
If discrepancies exist between the RELAX NG version of a schema and its corresponding XML Schema, the XML
Schema is the definitive version.
The RELAX NG schemas in this annex are generated automatically from the corresponding W3C XML Schema
schemas in Annex A, namespace prefix mapping in Annex D, Root Element Locations in Annex K, and then hand-
edited.
There are important differences between the RELAX NG schemas and XSD schemas.  This annex clarifies
differences in schema organizations and those in expressed constraints.
1. Regarding schema organization:
There is a RELAX NG schema (DrawingML_Chart.rnc, for example) dedicated to each OPC part
(DrawingML Chart, for example).  That RELAX NG schema includes all RELAX NG schema modules such as
dml-chart.rnc.  However, there are no such W3C XML Schema schemas dedicated to each OPC part.
Some schema modules in W3C XML Schema import other schema modules.
There is a RELAX NG schema xml.rnc, which captures xml:lang, xml:space, xml:base, and xml:id.
However, there are no such W3C XML Schema schemas.
2. Differences in expressed constraints:
The RELAX NG schemas specify root elements of each OPC part, while the XSD schemas do not.  For
example, DrawingML_Chart.rnc specifies that the root element of the OPC part DrawingML Chart is a
chartSpace element.
The RELAX NG schemas do not specify default values, while the XSD schemas do.  For example, dml-
ahrt.rnc does not specify a default value for the attribute val in dchrt_CT_Boolean, but merely provides
a comment about the default value.  Meanwhile, dml-chart.xsd specifies true as the default value of
this attribute in CT_Boolean.
The RELAX NG schemas mimic minOccurs="0", minOccurs="1", maxOccurs="1", and
maxOccurs="unbounded" in XSD schemas exactly, but do not mimic minOccurs="32" or
maxOccurs="256".  Such minOccurs and maxOccurs are loosely simulated by + or *.  For example,
