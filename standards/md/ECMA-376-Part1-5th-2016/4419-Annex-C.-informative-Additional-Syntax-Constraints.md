# Annex C.  (informative) Additional Syntax Constraints

<!-- source-pages 4419-4419 -->

<!-- page 4419 -->

Annex C

4409
Annex C.
(informative)
Additional Syntax Constraints
This clause is informative.
Although the set of normative XML Schemas included in the accompanying file named OfficeOpenXML-
XMLSchema.zip (discussed in Annex A) specifies a majority of the requirements needed to ensure
document conformance, there also exists a set of constraints that cannot easily be expressed in the XML
Schema syntax (otherwise referred to as additional syntax constraints). These additional constraints are
specified in the normative content of this Part of ECMA-376, and are included in the requirements for
document conformance.
The following is a sample of such constraints specified in this Part of ECMA-376:
Subclause
Constraint
WordprocessingML
Paragraphs
The caps element shall not be present with the smallCaps element on the same
run, since they are mutually exclusive in terms of appearance.
WordprocessingML
Headers and Footers
If the relationship type of the relationship specified by this element shall be
present, have a TargetMode attribute value of Internal, and a Type attribute
value of
http://purl.oclc.org/ooxml/officeDocument/relationships/header.
WordprocessingML
Annotations
Each "cross-structure" annotation shall have a start element whose id attribute
value matches the id attribute value of the corresponding end element.
SpreadsheetML
Formulas
The syntax of the value of a formula shall conform to the structure of a formula
as defined in the Formulas subclause.
SpreadsheetML
Styles
The xfId attribute on the cellStyles element shall specify an xf record which is
present in the cellStyleXfs collection.
PresentationML
Slides
Each sldId attribute in a sldLst shall have a unique value for its id attribute
(within the scope of the collection).
PresentationML
Slides
A customShow element specifies a range of slides (a start and end slide) in
PresentationML to define the set of slides that define a slide show. There shall
be slides in the presentation that have slide IDs referenced in these attributes
and the start slide shall be before the end slide.
DrawingML Main
The id attribute on the stCxn and endCxn elements specifies the start and end
shapes to be connected. Shapes with matching id attribute values shall exist
elsewhere in the spTree.
End of informative text.
