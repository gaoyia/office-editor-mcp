//! 三家 EMF / WMF / TIFF 测试共用的样例字节。
#![allow(dead_code)]

pub fn placeable_wmf() -> Vec<u8> {
    let inch: u16 = 96;
    let left: i16 = 0;
    let top: i16 = 0;
    let right: i16 = 32;
    let bottom: i16 = 16;
    let mut aldus = Vec::new();
    aldus.extend_from_slice(&0x9AC6CDD7u32.to_le_bytes());
    aldus.extend_from_slice(&0u16.to_le_bytes());
    aldus.extend_from_slice(&left.to_le_bytes());
    aldus.extend_from_slice(&top.to_le_bytes());
    aldus.extend_from_slice(&right.to_le_bytes());
    aldus.extend_from_slice(&bottom.to_le_bytes());
    aldus.extend_from_slice(&inch.to_le_bytes());
    aldus.extend_from_slice(&0u32.to_le_bytes());
    let mut cs: u16 = 0;
    for i in (0..20).step_by(2) {
        cs ^= u16::from_le_bytes(aldus[i..i + 2].try_into().unwrap());
    }
    aldus.extend_from_slice(&cs.to_le_bytes());
    let mut header = Vec::new();
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&9u16.to_le_bytes());
    header.extend_from_slice(&0x0300u16.to_le_bytes());
    header.extend_from_slice(&12u32.to_le_bytes());
    header.extend_from_slice(&0u16.to_le_bytes());
    header.extend_from_slice(&3u32.to_le_bytes());
    header.extend_from_slice(&0u16.to_le_bytes());
    let mut eof = Vec::new();
    eof.extend_from_slice(&3u32.to_le_bytes());
    eof.extend_from_slice(&0u16.to_le_bytes());
    let mut out = aldus;
    out.extend_from_slice(&header);
    out.extend_from_slice(&eof);
    out
}

pub fn emf_header() -> Vec<u8> {
    let mut b = vec![0u8; 88];
    b[0..4].copy_from_slice(&1u32.to_le_bytes());
    b[4..8].copy_from_slice(&88u32.to_le_bytes());
    b[16..20].copy_from_slice(&31i32.to_le_bytes());
    b[20..24].copy_from_slice(&15i32.to_le_bytes());
    b[32..36].copy_from_slice(&847i32.to_le_bytes());
    b[36..40].copy_from_slice(&423i32.to_le_bytes());
    b[40..44].copy_from_slice(&0x464D4520u32.to_le_bytes());
    b
}

pub fn tiff_32x16() -> Vec<u8> {
    let mut b = vec![0u8; 8 + 2 + 24 + 4];
    b[0..4].copy_from_slice(b"II*\0");
    b[4..8].copy_from_slice(&8u32.to_le_bytes());
    b[8..10].copy_from_slice(&2u16.to_le_bytes());
    write_ifd_short(&mut b, 10, 256, 32);
    write_ifd_short(&mut b, 22, 257, 16);
    b
}

fn write_ifd_short(b: &mut [u8], off: usize, tag: u16, val: u16) {
    b[off..off + 2].copy_from_slice(&tag.to_le_bytes());
    b[off + 2..off + 4].copy_from_slice(&3u16.to_le_bytes());
    b[off + 4..off + 8].copy_from_slice(&1u32.to_le_bytes());
    b[off + 8..off + 10].copy_from_slice(&val.to_le_bytes());
}

pub const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];
