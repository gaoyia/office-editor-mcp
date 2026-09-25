# 6 Appendix B: Product Behavior

<!-- source-pages 111-113 -->

<!-- page 111 -->

111 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
6
Appendix B: Product Behavior
The information in this specification is applicable to the following Microsoft products or supplemental
software. References to product versions include updates to those products.

Microsoft Office Word 2007

Microsoft Word 2010

Microsoft Word 2013

Microsoft Word 2016

Microsoft Word 2019

Microsoft Word 2021

Microsoft Word LTSC 2024
Exceptions, if any, are noted in this section. If an update version, service pack or Knowledge Base
(KB) number appears with a product name, the behavior changed in that update. The new behavior
also applies to subsequent updates unless otherwise specified. If a product edition appears with the
product version, behavior is different in that product edition.
Unless otherwise specified, any statement of optional behavior in this specification that is prescribed
using the terms "SHOULD" or "SHOULD NOT" implies product behavior in accordance with the
SHOULD or SHOULD NOT prescription. Unless otherwise specified, the term "MAY" implies that the
product does not follow the prescription.
<1> Section 2.1.1:  Office Word 2007 treats this part as an unknown relationship as specified in
[ISO/IEC29500-1:2016] section 9.1.7.
<2> Section 2.1.1:  This part is written out only by Microsoft Word 2010. Word 2013 and later will not
write out this part.
<3> Section 2.1.2:  This part is not available in Word 2010 and earlier.
<4> Section 2.1.3:  This part is not available in Word 2010 and earlier.
<5> Section 2.1.4:  This part is not available in Word 2013 and earlier.
<6> Section 2.1.5:  This part is not available in Word 2016 and earlier.
<7> Section 2.2.8:  This extension is not available in Word 2010 and earlier.
<8> Section 2.2.9:  This extension is not available in Word 2010 and earlier.
<9> Section 2.2.10:  This extension is not available in Word 2010 and earlier.
<10> Section 2.2.11:  This extension is not available in Word 2013 and earlier.
<11> Section 2.2.12:  This extension is only available when used with Microsoft Office 365.
<12> Section 2.3.4:  This element is not available in Word 2010 and earlier.
<13> Section 2.3.5:  Word 2010 ignores a compatSetting element with a val attribute whose value
is 15.
<14> Section 2.3.6:  This element is not available in Word 2010 and earlier.
<15> Section 2.3.7:  This element is not available in Word 2013 and earlier

<!-- page 112 -->

112 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
<16> Section 2.3.8:  This element is not available in Word 2013 and earlier.
<17> Section 2.5.1.1:  This element is not available in Word 2010 and earlier.
<18> Section 2.5.1.2:  This element is not available in Word 2010 and earlier.
<19> Section 2.5.1.3:  This element is not available in Word 2010 and earlier.
<20> Section 2.5.1.4:  This element is not available in Word 2010 and earlier.
<21> Section 2.5.1.5:  This element is not available in Word 2010 and earlier.
<22> Section 2.5.1.6:  This element is not available in Word 2010 and earlier.
<23> Section 2.5.1.7:  This element is not available in Word 2010 and earlier.
<24> Section 2.5.1.8:  This element is not available in Word 2010 and earlier.
<25> Section 2.5.1.9:  This element is not available in Word 2010 and earlier.
<26> Section 2.5.1.10:  This element is not available in Word 2010 and earlier.
<27> Section 2.5.1.11:  This element is not available in Word 2010 and earlier.
<28> Section 2.5.1.12:  This element is not available in Word 2010 and earlier.
<29> Section 2.5.1.13:  This element is not available in Word 2010 and earlier.
<30> Section 2.5.2.1:  This attribute is not available in Word 2010 and earlier.
<31> Section 2.5.3.1:  This type is not available in Word 2010 and earlier.
<32> Section 2.5.3.2:  This type is not available in Word 2010 and earlier.
<33> Section 2.5.3.4:  This type is not available in Word 2010 and earlier.
<34> Section 2.5.3.5:  This type is not available in Word 2010 and earlier.
<35> Section 2.5.3.6:  This type is not available in Word 2010 and earlier.
<36> Section 2.5.3.6:  This value is only available when used with Office 365.
<37> Section 2.5.3.6:  Older versions of Microsoft Word Online on Office 365 emitted values
consisting of a unique identifier followed by the constant text "@LIVE.COM".
<38> Section 2.5.3.7:  This type is not available in Word 2010 and earlier.
<39> Section 2.5.3.8:  This type is not available in Word 2010 and earlier.
<40> Section 2.5.4.2:  This type is not available in Word 2010 and earlier.
<41> Section 2.6.1.3:  Word 2010 and later treat the content as a tracked deletion.
<42> Section 2.6.1.4:  Word 2010 and later treat the parent as a tracked deletion.
<43> Section 2.6.1.5:  Word 2010 and later treat the parent as a tracked insertion.
<44> Section 2.6.1.6:  Word 2010 and later treat the content as a tracked insertion.
<45> Section 2.6.1.8:  Word 2010 and later ignore this element.
<46> Section 2.6.1.9:  Word 2010 and later ignore this element.

<!-- page 113 -->

113 / 119
[MS-DOCX] - v20260818
Word Extensions to the Office Open XML (.docx) File Format
Copyright © 2026 Microsoft Corporation
Release: August 18, 2026
<47> Section 2.6.1.10:  Word 2010 and later ignore this element.
<48> Section 2.6.1.11:  Word 2010 and later ignore this element.
<49> Section 2.6.3.1:  Word 2010 and later limit this to max 2147483646.
<50> Section 2.6.3.1:  Word 2010 and later limit this to max 2147483646.
<51> Section 2.6.3.2:  Word 2010 and later write orthographicFront for this attribute and ignore the
value when reading.
<52> Section 2.6.3.6:  Word 2010 and later limit this to max 2147483646.
<53> Section 2.6.3.23:  Word 2010 and later limit this to max 2147483646.
<54> Section 2.6.3.23:  Word 2010 and later limit this to max 2147483646.
<55> Section 2.6.3.24:  Word 2010 and later limit this to max 2147483646.
<56> Section 2.6.3.24:  Word 2010 and later limit this to max 2147483646.
<57> Section 2.6.3.30:  Word 2010 and later limit this to max 2147483646.
<58> Section 2.6.3.30:  Word 2010 and later limit this to max 2147483646.
<59> Section 2.8.1.1:  This element is not available in Word 2010 and earlier.
<60> Section 2.8.3.1:  This type is not available in Word 2013 and earlier.
<61> Section 2.8.3.2:  This type is not available in Word 2013 and earlier.
<62> Section 2.10.3.1:  When intelligentPlaceholder is true, the Office 365 version of Word ignores
the content of the comment. Other versions of Word treat the comment as a regular comment.
