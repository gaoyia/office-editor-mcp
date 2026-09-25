# 4 Security Considerations

<!-- source-pages 368-368 -->

<!-- page 368 -->

368 / 437
[MS-XLSX] - v20260519
Excel (.xlsx) Extensions to the Office Open XML SpreadsheetML File Format
Copyright © 2026 Microsoft Corporation
Release: May 19, 2026
4
Security Considerations
4.1
Security Considerations for Implementers
The password verifier features available in the file format are used to prevent accidental modification,
rather than being used as security features. It is possible to remove the passwords by removing the
records containing the verifier values.
The translation of passwords from a double-byte Unicode string to a new character string in the ANSI
code page of the current system converts any Unicode character that cannot be mapped to the ANSI
code page of the current system to the 0x3F character in that code page, as described in
[ISO/IEC29500-1:2016] section 18.2.29. Replacing these characters with "0x3F" when the hash is
verified will generate positive hash value matches. In certain locales, this can be a significant portion
of the everyday character set.
Further security considerations regarding the file encryption algorithms are described in [MS-
OFFCRYPTO] section 4.1.3.
4.2
Index of Security Fields
None.
