# 6 Appendix B: Product Behavior

<!-- source-pages 418-427 -->

<!-- page 418 -->

418 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
6
Appendix B: Product Behavior
The information in this specification is applicable to the following Microsoft products or supplemental
software. References to product versions include updates to those products.

Microsoft Office Excel 2007

Microsoft Excel 2010

Microsoft Excel 2013

Microsoft Excel 2016

Microsoft Excel 2019

Microsoft Excel 2021

Microsoft Excel LTSC 2024
Exceptions, if any, are noted in this section. If an update version, service pack or Knowledge Base
(KB) number appears with a product name, the behavior changed in that update. The new behavior
also applies to subsequent updates unless otherwise specified. If a product edition appears with the
product version, behavior is different in that product edition.
Unless otherwise specified, any statement of optional behavior in this specification that is prescribed
using the terms "SHOULD" or "SHOULD NOT" implies product behavior in accordance with the
SHOULD or SHOULD NOT prescription. Unless otherwise specified, the term "MAY" implies that the
product does not follow the prescription.
<1> Section 2.2:  This namespace was introduced in Excel 2013.
<2> Section 2.2:  This namespace was introduced in Excel 2010.
<3> Section 2.2:  This namespace was introduced in Excel 2010.
<4> Section 2.2:  This namespace was introduced in Excel 2013.
<5> Section 2.2:  This namespace was introduced in Excel 2013.
<6> Section 2.2.2:  In Excel version 2509 and later, the total string length for longtext-params
SHOULD NOT exceed 4095 characters.
<7> Section 2.4.104:  Excel versions prior to Excel LTSC 2024 will not ignore this.
<8> Section 2.4.105:  Excel versions prior to Excel LTSC 2024 will not ignore this.
<9> Section 2.5.4:  This attribute is available only in Excel 2016.
<10> Section 2.6.3:  Office Excel 2007, Excel 2010, and Excel 2013 do not write the correct value
into this field sometimes. Although the value of this field is validated on load, it is not used at run
time.
<11> Section 2.6.3:  Office Excel 2007, Excel 2010, and Excel 2013 do not write the correct value
into this field sometimes. Although the value of this field is validated on load, it is not used at run
time.
<12> Section 2.6.10:  The following table shows the different versions of function accuracy that Excel
2010 and Excel 2013 supports.
Value
Meaning
0
Functions are calculated using the current application’s algorithms.

<!-- page 419 -->

419 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Value
Meaning
1
Functions are calculated using algorithms implemented in Office Excel 2007.
2
Functions are calculated using algorithms implemented in Excel 2010.

<13> Section 2.6.24:  Office Excel 2007 does not support multiple uses of the same OLAP measure
in one cache hierarchy, and does not discard the associated cache hierarchy when ignore is TRUE.
<14> Section 2.6.25:  Office Excel 2007 will not ignore the fld attribute of the ancestor
CT_DataField ([ISO/IEC29500-4:2016] section A.2).
<15> Section 2.6.30:  Office Excel 2007 does not ignore these complex types.
<16> Section 2.6.31:  Office Excel 2007 does not support multiple uses of the same OLAP measure in
one PivotTable view, and does not ignore this pivot field when ignore is TRUE.
<17> Section 2.6.34:  The 2007 Microsoft Office system does not load a file in which this field
contains a value that it does not recognize, or is not recognized by the underlying operating system.
The 2007 Office system recognizes the following language tags.
Language
Locale
Language tag
Afrikaans
South Africa
af-ZA
Albanian
Albania
sq-AL
Alsatian
France
gsw-FR
Amharic
Ethiopia
am-ET
Arabic
Algeria
ar-DZ
Arabic
Kingdom of Bahrain
ar-BH
Arabic
Egypt
ar-EG
Arabic
Iraq
ar-IQ
Arabic
Jordan
ar-JO
Arabic
Kuwait
ar-KW
Arabic
Lebanon
ar-LB
Arabic
Libya
ar-LY
Arabic
Morocco
ar-MA
Arabic
Oman
ar-OM
Arabic
Qatar
ar-QA
Arabic
Saudi Arabia
ar-SA
Arabic
Syria
ar-SY
Arabic
Tunisia
ar-TN

<!-- page 420 -->

420 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Arabic
U.A.E.
ar-AE
Arabic
Yemen
ar-YE
Armenian
Armenia
hy-AM
Assamese
India
as-IN
Azerbaijani (Cyrillic)
Azerbaijan
az-AZ-Cyrl
Azerbaijani (Latin)
Azerbaijan
az-AZ-Latn
Bangla
Bangladesh
bn-BD
Bangla (Bangla Script)
India
bn-IN
Bashkir
Russia
ba-RU
Basque
Basque (Basque)
eu-ES
Belarusian
Belarus
be-BY
Bhutanese
Bhutan
bo-BT
Bosnian (Cyrillic)
Bosnia and Herzegovina
bs-BA-Cyrl
Bosnian (Latin)
Bosnia and Herzegovina
bs-BA-Latn
Breton
France
br-FR
Bulgarian
Bulgaria
bg-BG
Catalan
Catalan
ca-ES
Chinese
Hong Kong SAR
zh-HK
Chinese
Macao SAR
zh-MO
Chinese
PRC
zh-CN
Chinese
Singapore
zh-SG
Chinese
Taiwan
zh-TW
Corsican
France
co-FR
Croatian
Croatia
hr-HR
Croatian (Latin)
Bosnia and Herzegovina
hr-BA-Latn
Czech
Czech Republic
cs-CZ
Danish
Denmark
da-DK
Dari
Afghanistan
prs-AF

<!-- page 421 -->

421 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Divehi
Maldives
div-MV
Dutch
Belgium
nl-BE
Dutch
Netherlands
nl-NL
English
Australia
en-AU
English
Belize
en-BZ
English
Canada
en-CA
English
Caribbean
en-CB
English
India
en-IN
English
Ireland
en-IE
English
Jamaica
en-JM
English
Malaysia
en-MY
English
New Zealand
en-NZ
English
Philippines
en-PH
English
South Africa
en-ZA
English
Trinidad
en-TT
English
United Kingdom
en-GB
English
United States
en-US
English
Zimbabwe
en-ZW
Estonian
Estonia
et-EE
Faroese
Faroe Islands
fo-FO
Filipino
Philippines
fil-PH
Finnish
Finland
fi-FI
French
Belgium
fr-BE
French
Canada
fr-CA
French
France
fr-FR
French
Luxembourg
fr-LU
French
Monaco
fr-MC
French
Switzerland
fr-CH

<!-- page 422 -->

422 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Frisian
Netherlands
fy-NL
Galician
Galicia
gl-ES
Georgian
Georgia
ka-GE
German
Austria
de-AT
German
Germany
de-DE
German
Liechtenstein
de-LI
German
Luxembourg
de-LU
German
Switzerland
de-CH
Greek
Greece
el-GR
Greenlandic
Greenland
kl-GL
Gujarati (Gujarati Script)
India
gu-IN
Hausa (Latin)
Nigeria
ha-NG-Latn
Hebrew
Israel
he-IL
Hindi
India
hi-IN
Hungarian
Hungary
hu-HU
Icelandic
Iceland
is-IS
Igbo
Nigeria
ig-NG
Inari Sami
Finland
smn-FI
Indonesian
Indonesia
id-ID
Inuktitut (Latin)
Canada
iu-CA-Latn
Inuktitut (Syllabics)
Canada
iu-CA-Cans
Irish
Ireland
ga-IE
isiXhosa / Xhosa
South Africa
xh-ZA
isiZulu / Zulu
South Africa
zu-ZA
Italian
Italy
it-IT
Italian
Switzerland
it-CH
Japanese
Japan
ja-JP
Kannada (Kannada Script)
India
kn-IN

<!-- page 423 -->

423 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Kazakh
Kazakhstan
kk-KZ
Khmer
Cambodia
kh-KH
K'iche
Guatemala
qut-GT
Kinyarwanda
Rwanda
rw-RW
Konkani
India
kok-IN
Korean
Korea
ko-KR
Kyrgyz
Kyrgyzstan
ky-KG
Lao
Lao PDR
lo-LA
Latvian
Latvia
lv-LV
Lithuanian
Lithuania
lt-LT
Lower Sorbian
Germany
wee-DE
Lule Sami
Norway
smj-NO
Lule Sami
Sweden
smj-SE
Luxembourgish
Luxembourg
lb-LU
Macedonian (North Macedonia)
North Macedonia, Republic of
mk-MK
Malay
Brunei
ms-BN
Malay
Malaysia
ms-MY
Malayalam (Malayalam Script)
India
ml-IN
Maltese
Malta
mt-MT
Maori
New Zealand
mi-NZ
Mapudungun
Chile
arn-CL
Marathi
India
mr-IN
Mohawk
Mohawk
moh-CA
Mongolian (Cyrillic)
Mongolia
mn-MN
Mongolian (Mongolian)
PRC
mn-CN-Mong
Nepali
Federal Democratic Republic of Nepal
ne-NP
Northern Sami
Finland
se-FI
Northern Sami
Norway
se-NO

<!-- page 424 -->

424 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Northern Sami
Sweden
se-SE
Norwegian (Bokmål)
Norway
nb-NO
Norwegian (Nynorsk)
Norway
nn-NO
Occitan
France
oc-FR
Odia (Odia Script)
India
or-IN
Pashto
Afghanistan
ps-AF
Persian
Iran
fa-IR
Polish
Poland
pl-PL
Portuguese
Brazil
pt-BR
Portuguese
Portugal
pt-PT
Punjabi (Gurmukhi Script)
India
pa-IN
Quechua
Bolivia
quz-BO
Quechua
Ecuador
quz-EC
Quechua
Peru
quz-PE
Romanian
Romania
ro-RO
Romansh
Switzerland
rm-CH
Russian
Russia
ru-RU
Sakha
Russia
sah-RU
Sanskrit
India
sa-IN
Serbian (Cyrillic)
Bosnia and Herzegovina
sr-BA-Cyrl
Serbian (Cyrillic)
Serbia
sr-SP-Cyrl
Serbian (Latin)
Bosnia and Herzegovina
sr-BA-Latn
Serbian (Latin)
Serbia
sr-SP-Latn
Sesotho sa Leboa / Northern Sotho
South Africa
ns-ZA
Setswana / Tswana
South Africa
tn-ZA
Sinhala
Sri Lanka
si-LK
Skolt Sami
Finland
sms-FI
Slovak
Slovakia
sk-SK

<!-- page 425 -->

425 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Slovenian
Slovenia
sl-SI
Southern Sami
Norway
sma-NO
Southern Sami
Sweden
sma-SE
Spanish
Argentina
es-AR
Spanish
Bolivia
es-BO
Spanish
Chile
es-CL
Spanish
Columbia
es-CO
Spanish
Costa Rica
es-CR
Spanish
Dominican Republic
es-DO
Spanish
Ecuador
es-EC
Spanish
El Salvador
es-SV
Spanish
Guatemala
es-GT
Spanish
Honduras
es-HN
Spanish
Mexico
es-MX
Spanish
Nicaragua
es-NI
Spanish
Panama
es-PA
Spanish
Paraguay
es-PY
Spanish
Peru
es-PE
Spanish
Commonwealth of Puerto Rico
es-PR
Spanish
Spain
es-ES
Spanish
United States
es-US
Spanish
Uruguay
es-UY
Spanish
Venezuela
es-VE
Swahili
Kenya
sw-KE
Swedish
Finland
sv-FI
Swedish
Sweden
sv-SE
Syriac
Syria
syr-SY
Tajik (Cyrillic)
Tajikistan
tg-TJ-Cyrl

<!-- page 426 -->

426 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
Language
Locale
Language tag
Tamazight (Latin)
Algeria
tmz-DZ-Latn
Tamil
India
ta-IN
Tatar
Russia
tt-RU
Telugu (Telugu Script)
India
te-IN
Thai
Thailand
th-TH
Tibetan
PRC
bo-CN
Turkish
Turkey
tr-TR
Turkmen
Turkmenistan
tk-TM
Uighur
PRC
ug-CN
Ukrainian
Ukraine
uk-UA
Upper Sorbian
Germany
wen-DE
Urdu
Pakistan
ur-PK
Uzbek (Cyrillic)
Uzbekistan
uz-UZ-Cyrl
Uzbek (Latin)
Uzbekistan
uz-UZ-Latn
Vietnamese
Viet Nam
vi-VN
Welsh
United Kingdom
cy-GB
Wolof
Senegal
wo-SN
Yi
PRC
ii-CN
Yoruba
Nigeria
yo-NG

<18> Section 2.6.47:  Office Excel 2007 does not ignore the ancestor CT_PivotHierarchy element.
<19> Section 2.6.48:  Office Excel 2007 does not ignore the ancestor CT_CacheField element.
<20> Section 2.6.63:  Excel 2010 and Excel 2013 requires that the corresponding ListItem
([ISO/IEC29500-4:2016] section 19.4.2.36) be present.
<21> Section 2.6.65:  Excel 2010 loads and roundtrips this value for scroll bars and spin boxes, but it
does not support its functionality.
<22> Section 2.6.65:  Excel 2010 requires that the corresponding FmlaRange ([ISO/IEC29500-
4:2016] section 19.4.2.29) be present.
<23> Section 2.6.65:  Excel 2010 and Excel 2013 ignore this attribute. This attribute is being
deprecated.

<!-- page 427 -->

427 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
<24> Section 2.6.65:  This attribute is being deprecated in Excel 2010. Its functionality will be
replaced with attribute noThreeD.
<25> Section 2.6.65:  Excel 2010 loads and roundtrips this value, but it only supports its functionality
for scroll bar form controls when run in a dialog box .
<26> Section 2.6.65:  Excel 2010 does not support this attribute.
<27> Section 2.6.65:  Excel 2010 does not support this attribute.
<28> Section 2.6.220:  Excel uses the UTF-16LE encoding for the comment string when computing
the CRC32 checksum.
<29> Section 2.7.16:  Excel 2010 uses this value only when the drop-down control is run in a dialog
box, in all other cases the drop-down control behaves as a standard combo box.
