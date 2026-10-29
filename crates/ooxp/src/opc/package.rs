//! ZIP 打开、部件读写、保存时未改条目 raw copy。

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, ErrorKind, OpcReason};

use super::names::{normalize_part_name, zip_name_from_opc};
use super::{CONTENT_TYPES_PART, ROOT_RELS_PART};

/// 声明的未压缩大小达到压缩后大小的这个倍数就拒绝。
const EXPAND_LIMIT: u64 = 4096;

#[derive(Debug)]
struct Part {
    data: Vec<u8>,
    dirty: bool,
    deleted: bool,
    original_index: Option<usize>,
}

#[derive(Debug)]
pub struct Package {
    original_bytes: Option<Vec<u8>>,
    /// 打开时 ZIP 中央目录顺序（OPC 名）。
    original_order: Vec<String>,
    /// 新建部件的追加顺序。
    new_order: Vec<String>,
    parts: HashMap<String, Part>,
}

fn expands_too_far(declared: u64, compressed: u64) -> bool {
    if declared == 0 {
        return false;
    }
    if compressed == 0 {
        return true;
    }
    match compressed.checked_mul(EXPAND_LIMIT) {
        Some(limit) => declared >= limit,
        None => false,
    }
}

fn read_counted(file: &mut impl Read, declared: u64) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let next = (data.len() as u64).saturating_add(n as u64);
        if next > declared {
            return Err(Error::new(ErrorKind::OpcOpen {
                reason: OpcReason::ExceedsDeclared,
            }));
        }
        data.extend_from_slice(&buf[..n]);
    }
    Ok(data)
}

impl Package {
    pub fn create() -> Self {
        Self {
            original_bytes: None,
            original_order: Vec::new(),
            new_order: Vec::new(),
            parts: HashMap::new(),
        }
    }

    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice()))?;
        let mut parts = HashMap::new();
        let mut original_order = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let raw_name = file.name().to_string();
            if raw_name.ends_with('/') {
                continue;
            }
            let opc = normalize_part_name(&raw_name)?;
            let declared = file.size();
            let compressed = file.compressed_size();
            if expands_too_far(declared, compressed) {
                return Err(Error::new(ErrorKind::OpcOpen {
                    reason: OpcReason::DeclaredRatio,
                }));
            }
            let data = read_counted(&mut file, declared)?;
            original_order.push(opc.clone());
            parts.insert(
                opc,
                Part {
                    data,
                    dirty: false,
                    deleted: false,
                    original_index: Some(i),
                },
            );
        }

        let pkg = Self {
            original_bytes: Some(bytes),
            original_order,
            new_order: Vec::new(),
            parts,
        };
        if !pkg.contains(CONTENT_TYPES_PART) {
            return Err(Error::new(ErrorKind::OpcOpen {
                reason: OpcReason::MissingContentTypes,
            }));
        }
        if !pkg.contains(ROOT_RELS_PART) {
            return Err(Error::new(ErrorKind::OpcOpen {
                reason: OpcReason::MissingRootRels,
            }));
        }
        Ok(pkg)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.parts.get(name).map(|p| !p.deleted).unwrap_or(false)
    }

    pub fn get(&self, name: &str) -> Result<&[u8], Error> {
        self.parts
            .get(name)
            .filter(|p| !p.deleted)
            .map(|p| p.data.as_slice())
            .ok_or_else(|| {
                Error::new(ErrorKind::PartMissing {
                    name: name.to_string(),
                })
            })
    }

    /// 列出未删除部件，顺序为原顺序再加新建。
    #[allow(dead_code)]
    pub fn part_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for n in &self.original_order {
            if self.contains(n) {
                names.push(n.clone());
            }
        }
        for n in &self.new_order {
            if self.contains(n) && !self.original_order.iter().any(|o| o == n) {
                names.push(n.clone());
            }
        }
        names
    }

    /// 写入或覆盖部件，并标脏。
    pub fn set(&mut self, name: &str, data: Vec<u8>) -> Result<(), Error> {
        let name = normalize_part_name(name)?;
        if let Some(part) = self.parts.get_mut(&name) {
            part.data = data;
            part.dirty = true;
            part.deleted = false;
            return Ok(());
        }
        self.new_order.push(name.clone());
        self.parts.insert(
            name,
            Part {
                data,
                dirty: true,
                deleted: false,
                original_index: None,
            },
        );
        Ok(())
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(part) = self.parts.get_mut(name) {
            part.deleted = true;
            part.dirty = true;
        }
    }

    pub fn save_to_vec(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let deflate =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            let mut orig = match &self.original_bytes {
                Some(b) => Some(ZipArchive::new(Cursor::new(b.as_slice()))?),
                None => None,
            };

            for name in &self.original_order {
                let Some(part) = self.parts.get(name) else {
                    continue;
                };
                if part.deleted {
                    continue;
                }
                if !part.dirty {
                    if let (Some(archive), Some(idx)) = (orig.as_mut(), part.original_index) {
                        let file = archive.by_index(idx)?;
                        writer.raw_copy_file(file)?;
                        continue;
                    }
                }
                writer.start_file(zip_name_from_opc(name), deflate)?;
                writer.write_all(&part.data)?;
            }

            for name in &self.new_order {
                if self.original_order.iter().any(|o| o == name) {
                    continue;
                }
                let Some(part) = self.parts.get(name) else {
                    continue;
                };
                if part.deleted {
                    continue;
                }
                writer.start_file(zip_name_from_opc(name), deflate)?;
                writer.write_all(&part.data)?;
            }

            writer.finish()?;
        }
        Ok(cursor.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::opc::CONTENT_TYPES_PART;

    fn minimal_package_bytes() -> Vec<u8> {
        let mut pkg = Package::create();
        pkg.set(
            CONTENT_TYPES_PART,
            br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#.to_vec(),
        )
        .unwrap();
        pkg.set(ROOT_RELS_PART, br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#.to_vec())
            .unwrap();
        pkg.set("/keep.xml", b"unchanged-bytes".to_vec()).unwrap();
        pkg.set("/edit.xml", b"old".to_vec()).unwrap();
        pkg.save_to_vec().unwrap()
    }

    #[test]
    fn roundtrip_preserves_untouched_part() {
        let bytes = minimal_package_bytes();
        let mut pkg = Package::open_bytes(bytes).unwrap();
        let names = pkg.part_names();
        assert!(names.iter().any(|n| n == "/keep.xml"));
        pkg.set("/edit.xml", b"new".to_vec()).unwrap();
        let out = pkg.save_to_vec().unwrap();
        let reopened = Package::open_bytes(out).unwrap();
        assert_eq!(reopened.get("/keep.xml").unwrap(), b"unchanged-bytes");
        assert_eq!(reopened.get("/edit.xml").unwrap(), b"new");
    }

    #[test]
    fn missing_content_types_is_opc_open() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file("_rels/.rels", opt).unwrap();
            writer.write_all(b"<Relationships/>").unwrap();
            writer.finish().unwrap();
        }
        let err = Package::open_bytes(cursor.into_inner()).unwrap_err();
        assert_eq!(err.code(), ErrorCode::OpcOpen);
    }

    #[test]
    fn declared_ratio_is_opc_open() {
        let mut bytes = stored_zip("a.txt", b"x");
        set_uncompressed(&mut bytes, 4096);
        let err = Package::open_bytes(bytes).unwrap_err();
        assert_eq!(err.code(), ErrorCode::OpcOpen);
        assert!(err.to_string().contains("4096"), "{}", err);
    }

    #[test]
    fn zero_compressed_with_a_declared_size_is_opc_open() {
        let mut bytes = stored_zip("a.txt", b"x");
        set_sizes(&mut bytes, 0, 8);
        let err = Package::open_bytes(bytes).unwrap_err();
        assert_eq!(err.code(), ErrorCode::OpcOpen);
        assert!(err.to_string().contains("4096"), "{}", err);
    }

    #[test]
    fn output_past_the_declared_size_is_opc_open() {
        let mut bytes = stored_zip("a.txt", b"abcdefgh");
        set_uncompressed(&mut bytes, 1);
        let err = Package::open_bytes(bytes).unwrap_err();
        assert_eq!(err.code(), ErrorCode::OpcOpen);
        assert!(err.to_string().contains("central directory"), "{}", err);
    }

    fn stored_zip(name: &str, data: &[u8]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file(name, opt).unwrap();
            writer.write_all(data).unwrap();
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    fn set_uncompressed(bytes: &mut [u8], declared: u32) {
        let compressed = u32::from_le_bytes(size_field(bytes, true));
        set_sizes(bytes, compressed, declared);
    }

    fn set_sizes(bytes: &mut [u8], compressed: u32, declared: u32) {
        patch_pair(bytes, b"PK\x03\x04", 18, compressed, declared);
        patch_pair(bytes, b"PK\x01\x02", 20, compressed, declared);
    }

    fn size_field(bytes: &[u8], compressed: bool) -> [u8; 4] {
        let at = find_sig(bytes, b"PK\x03\x04").unwrap() + if compressed { 18 } else { 22 };
        bytes[at..at + 4].try_into().unwrap()
    }

    fn patch_pair(bytes: &mut [u8], sig: &[u8; 4], at: usize, compressed: u32, declared: u32) {
        let start = find_sig(bytes, sig).unwrap();
        bytes[start + at..start + at + 4].copy_from_slice(&compressed.to_le_bytes());
        bytes[start + at + 4..start + at + 8].copy_from_slice(&declared.to_le_bytes());
    }

    fn find_sig(bytes: &[u8], sig: &[u8; 4]) -> Option<usize> {
        bytes.windows(4).position(|window| window == sig)
    }
}
