//! 保护口令哈希：Excel 工作表/工作簿与 PowerPoint 修改密码的 ISO SHA-512，
//! 以及 Word 限制编辑的遗留预处理加 SHA-512。不破解密码。

use sha2::{Digest, Sha512};

/// Excel 16.0、PowerPoint 16.0 与 Word 16.0 共用的 SHA-512 迭代。
///
/// `password` 按 UTF-16LE 编码（不要 BOM）。
/// `H0 = SHA-512(salt || password)`；`i = 0 .. spin-1` 时 `H_{i+1} = SHA-512(H_i || i 的 4 字节小端)`。
pub(crate) fn sha512_spin(password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    let mut pwd = Vec::with_capacity(password.len() * 2);
    for u in password.encode_utf16() {
        pwd.extend_from_slice(&u.to_le_bytes());
    }
    let mut digest: [u8; 64] = {
        let mut h = Sha512::new();
        h.update(salt);
        h.update(&pwd);
        h.finalize().into()
    };
    for i in 0..spin {
        let mut h = Sha512::new();
        h.update(digest);
        h.update(i.to_le_bytes());
        digest = h.finalize().into();
    }
    digest.to_vec()
}

/// Word 97 遗留口令哈希的 4 字节结果，写成大写两位十六进制。
///
/// 口令按 UTF-8 取前 15 字节。ECMA-376 Part 4 §14.8.1；`Example` → `7EEDCE64`。
pub(crate) fn word_legacy_hex(password: &str) -> String {
    let pw: Vec<u8> = password.as_bytes().iter().copied().take(15).collect();
    let n = pw.len();
    let mut high = if n == 0 { [0u8, 0] } else { HIGH_ORDER[n - 1] };
    for (i, &b) in pw.iter().enumerate() {
        let row = i + 15 - n;
        for (bit, pair) in ENCRYPTION_MATRIX[row].iter().enumerate() {
            if b & (1 << bit) == 0 {
                continue;
            }
            high[0] ^= pair[0];
            high[1] ^= pair[1];
        }
    }
    let mut low: u16 = 0;
    for &b in pw.iter().rev() {
        low = (((low >> 14) & 1) | ((low << 1) & 0x7fff)) ^ u16::from(b);
    }
    low = (((low >> 14) & 1) | ((low << 1) & 0x7fff)) ^ (n as u16) ^ 0xce4b;
    // 高字在前、低字在后，再整段倒序，与 Word 16.0 另存一致。
    let key = [high[0], high[1], (low >> 8) as u8, low as u8];
    let rev = [key[3], key[2], key[1], key[0]];
    format!("{:02X}{:02X}{:02X}{:02X}", rev[0], rev[1], rev[2], rev[3])
}

/// Word 限制编辑：先做遗留预处理，再走与 Excel 相同的 SHA-512 迭代。
pub(crate) fn word_sha512_spin(password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    sha512_spin(&word_legacy_hex(password), salt, spin)
}

const HIGH_ORDER: [[u8; 2]; 15] = [
    [0xE1, 0xF0],
    [0x1D, 0x0F],
    [0xCC, 0x9C],
    [0x84, 0xC0],
    [0x11, 0x0C],
    [0x0E, 0x10],
    [0xF1, 0xCE],
    [0x31, 0x3E],
    [0x18, 0x72],
    [0xE1, 0x39],
    [0xD4, 0x0F],
    [0x84, 0xF9],
    [0x28, 0x0C],
    [0xA9, 0x6A],
    [0x4E, 0xC3],
];

const ENCRYPTION_MATRIX: [[[u8; 2]; 7]; 15] = [
    [
        [0xAE, 0xFC],
        [0x4D, 0xD9],
        [0x9B, 0xB2],
        [0x27, 0x45],
        [0x4E, 0x8A],
        [0x9D, 0x14],
        [0x2A, 0x09],
    ],
    [
        [0x7B, 0x61],
        [0xF6, 0xC2],
        [0xFD, 0xA5],
        [0xEB, 0x6B],
        [0xC6, 0xF7],
        [0x9D, 0xCF],
        [0x2B, 0xBF],
    ],
    [
        [0x45, 0x63],
        [0x8A, 0xC6],
        [0x05, 0xAD],
        [0x0B, 0x5A],
        [0x16, 0xB4],
        [0x2D, 0x68],
        [0x5A, 0xD0],
    ],
    [
        [0x03, 0x75],
        [0x06, 0xEA],
        [0x0D, 0xD4],
        [0x1B, 0xA8],
        [0x37, 0x50],
        [0x6E, 0xA0],
        [0xDD, 0x40],
    ],
    [
        [0xD8, 0x49],
        [0xA0, 0xB3],
        [0x51, 0x47],
        [0xA2, 0x8E],
        [0x55, 0x3D],
        [0xAA, 0x7A],
        [0x44, 0xD5],
    ],
    [
        [0x6F, 0x45],
        [0xDE, 0x8A],
        [0xAD, 0x35],
        [0x4A, 0x4B],
        [0x94, 0x96],
        [0x39, 0x0D],
        [0x72, 0x1A],
    ],
    [
        [0xEB, 0x23],
        [0xC6, 0x67],
        [0x9C, 0xEF],
        [0x29, 0xFF],
        [0x53, 0xFE],
        [0xA7, 0xFC],
        [0x5F, 0xD9],
    ],
    [
        [0x47, 0xD3],
        [0x8F, 0xA6],
        [0x0F, 0x6D],
        [0x1E, 0xDA],
        [0x3D, 0xB4],
        [0x7B, 0x68],
        [0xF6, 0xD0],
    ],
    [
        [0xB8, 0x61],
        [0x60, 0xE3],
        [0xC1, 0xC6],
        [0x93, 0xAD],
        [0x37, 0x7B],
        [0x6E, 0xF6],
        [0xDD, 0xEC],
    ],
    [
        [0x45, 0xA0],
        [0x8B, 0x40],
        [0x06, 0xA1],
        [0x0D, 0x42],
        [0x1A, 0x84],
        [0x35, 0x08],
        [0x6A, 0x10],
    ],
    [
        [0xAA, 0x51],
        [0x44, 0x83],
        [0x89, 0x06],
        [0x02, 0x2D],
        [0x04, 0x5A],
        [0x08, 0xB4],
        [0x11, 0x68],
    ],
    [
        [0x76, 0xB4],
        [0xED, 0x68],
        [0xCA, 0xF1],
        [0x85, 0xC3],
        [0x1B, 0xA7],
        [0x37, 0x4E],
        [0x6E, 0x9C],
    ],
    [
        [0x37, 0x30],
        [0x6E, 0x60],
        [0xDC, 0xC0],
        [0xA9, 0xA1],
        [0x43, 0x63],
        [0x86, 0xC6],
        [0x1D, 0xAD],
    ],
    [
        [0x33, 0x31],
        [0x66, 0x62],
        [0xCC, 0xC4],
        [0x89, 0xA9],
        [0x03, 0x73],
        [0x06, 0xE6],
        [0x0D, 0xCC],
    ],
    [
        [0x10, 0x21],
        [0x20, 0x42],
        [0x40, 0x84],
        [0x81, 0x08],
        [0x12, 0x31],
        [0x24, 0x62],
        [0x48, 0xC4],
    ],
];

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    #[test]
    fn excel_sheet_sample_matches() {
        let salt = STANDARD.decode("XU0YjfLYkdb5QbNqCNOyHA==").unwrap();
        let want = STANDARD
            .decode("6nKP+s3+T6I4ifGSbzortB+RwF5ZmjfkV3J86fejw5GFCree1zkmdy4Q3+xVUx9SdeihNagWdWE4gIWNb2OkWg==")
            .unwrap();
        assert_eq!(sha512_spin("secret", &salt, 100_000), want);
    }

    #[test]
    fn word_legacy_example_is_7eedce64() {
        assert_eq!(word_legacy_hex("Example"), "7EEDCE64");
        assert_eq!(word_legacy_hex("secret"), "A7DA4D01");
    }

    #[test]
    fn word_word16_protect_sample_matches() {
        let salt = STANDARD.decode("V4hMlT5ctr9w9wpUWhIHwg==").unwrap();
        let want = STANDARD
            .decode("lHSWYm7xlRXHKBrKtEnTcCOB+r3ZxNvnQ1cBeaumnBx1qdAlEOFT39EcTLOHK+pj3SNVZzx2T9mtHG/epYA+kw==")
            .unwrap();
        assert_eq!(word_sha512_spin("secret", &salt, 100_000), want);
    }

    #[test]
    fn powerpoint_writepassword_sample_matches() {
        let salt = STANDARD.decode("0DqIT0r3r/vfq1WJSH0csw==").unwrap();
        let want = STANDARD
            .decode("BojOzuAyO7brNl1Dj8sT9mQ0x+WPTqSINbM/4URAwXWzOnpgZkOYqQ4icqbolDQN87l2q7HnwHSOF+DFjhd8eQ==")
            .unwrap();
        assert_eq!(sha512_spin("secret", &salt, 100_000), want);
    }
}
