//! ZIP 打开、部件读写。未改条目整段搬本地记录和中央目录头，以留下 extra。

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, ErrorKind, OpcReason};

use super::names::{fold_part_name, normalize_part_name, same_part_name, zip_name_from_opc};
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
    /// 折叠名 → 包里留下的拼写。查找、contains 走这里。
    by_fold: HashMap<String, String>,
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
            by_fold: HashMap::new(),
        }
    }

    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice()))?;
        let mut parts = HashMap::new();
        let mut by_fold = HashMap::new();
        let mut original_order = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let raw_name = file.name().to_string();
            if raw_name.ends_with('/') {
                continue;
            }
            let opc = normalize_part_name(&raw_name)?;
            let fold = fold_part_name(&opc);
            if by_fold.contains_key(&fold) {
                return Err(Error::new(ErrorKind::OpcOpen {
                    reason: OpcReason::DuplicatePartName { name: opc },
                }));
            }
            let declared = file.size();
            let compressed = file.compressed_size();
            if expands_too_far(declared, compressed) {
                return Err(Error::new(ErrorKind::OpcOpen {
                    reason: OpcReason::DeclaredRatio,
                }));
            }
            let data = read_counted(&mut file, declared)?;
            original_order.push(opc.clone());
            by_fold.insert(fold, opc.clone());
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
            by_fold,
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

    fn stored_name(&self, name: &str) -> Option<&str> {
        let opc = normalize_part_name(name).ok()?;
        self.by_fold.get(&fold_part_name(&opc)).map(String::as_str)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.stored_name(name)
            .and_then(|key| self.parts.get(key))
            .is_some_and(|part| !part.deleted)
    }

    /// 从 1 起取调用方拼出的部件名，直到包里还没有这一份。
    pub fn first_free(&self, name_of: impl Fn(u32) -> String) -> String {
        let mut n = 1u32;
        loop {
            let name = name_of(n);
            if !self.contains(&name) {
                return name;
            }
            n += 1;
        }
    }

    pub fn get(&self, name: &str) -> Result<&[u8], Error> {
        let missing = || {
            Error::new(ErrorKind::PartMissing {
                name: name.to_string(),
            })
        };
        let Some(key) = self.stored_name(name) else {
            return Err(missing());
        };
        self.parts
            .get(key)
            .filter(|part| !part.deleted)
            .map(|part| part.data.as_slice())
            .ok_or_else(missing)
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
            if self.contains(n) && !self.original_order.iter().any(|o| same_part_name(o, n)) {
                names.push(n.clone());
            }
        }
        names
    }

    /// 写入或覆盖部件，并标脏。
    pub fn set(&mut self, name: &str, data: Vec<u8>) -> Result<(), Error> {
        let name = normalize_part_name(name)?;
        let fold = fold_part_name(&name);
        if let Some(stored) = self.by_fold.get(&fold).cloned() {
            if let Some(part) = self.parts.get_mut(&stored) {
                part.data = data;
                part.dirty = true;
                part.deleted = false;
                return Ok(());
            }
        }
        self.new_order.push(name.clone());
        self.by_fold.insert(fold, name.clone());
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
        let Some(key) = self.stored_name(name).map(str::to_string) else {
            return;
        };
        if let Some(part) = self.parts.get_mut(&key) {
            part.deleted = true;
            part.dirty = true;
        }
    }

    pub fn save_to_vec(&self) -> Result<Vec<u8>, Error> {
        let source = match &self.original_bytes {
            Some(bytes) => Some(SourceZip::read(bytes)?),
            None => None,
        };
        let mut out = Vec::new();
        let mut centrals = Vec::new();

        for name in &self.original_order {
            let Some(part) = self.parts.get(name) else {
                continue;
            };
            if part.deleted {
                continue;
            }
            if !part.dirty {
                if let (Some(bytes), Some(source), Some(idx)) = (
                    self.original_bytes.as_deref(),
                    source.as_ref(),
                    part.original_index,
                ) {
                    let (local, central) = source.clean_entry(bytes, idx)?;
                    push_entry(&mut out, &mut centrals, local, central)?;
                    continue;
                }
            }
            let (local, central) = rewrite_part(name, &part.data)?;
            push_entry(&mut out, &mut centrals, &local, central)?;
        }

        for name in &self.new_order {
            if self.original_order.iter().any(|o| same_part_name(o, name)) {
                continue;
            }
            let Some(part) = self.parts.get(name) else {
                continue;
            };
            if part.deleted {
                continue;
            }
            let (local, central) = rewrite_part(name, &part.data)?;
            push_entry(&mut out, &mut centrals, &local, central)?;
        }

        write_eocd(&mut out, &centrals)?;
        Ok(out)
    }
}

struct SourceZip {
    cd_start: u64,
    /// 按下标：本地头偏移、中央目录头偏移。
    entries: Vec<(u64, u64)>,
}

impl SourceZip {
    fn read(bytes: &[u8]) -> Result<Self, Error> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))?;
        let cd_start = archive.central_directory_start();
        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            entries.push((file.header_start(), file.central_header_start()));
        }
        Ok(Self { cd_start, entries })
    }

    fn clean_entry<'a>(&self, bytes: &'a [u8], index: usize) -> Result<(&'a [u8], Vec<u8>), Error> {
        let Some(&(header, central_at)) = self.entries.get(index) else {
            return Err(zip_broken("missing entry"));
        };
        let local = local_record(bytes, header, &self.entries, self.cd_start)?;
        let central = central_record(bytes, central_at)?;
        Ok((local, central))
    }
}

fn push_entry(
    out: &mut Vec<u8>,
    centrals: &mut Vec<Vec<u8>>,
    local: &[u8],
    mut central: Vec<u8>,
) -> Result<(), Error> {
    let offset = u64::try_from(out.len()).map_err(|_| zip_broken("offset"))?;
    patch_central_offset(&mut central, offset)?;
    out.extend_from_slice(local);
    centrals.push(central);
    Ok(())
}

fn rewrite_part(name: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let bytes = {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer.start_file(zip_name_from_opc(name), opts)?;
            writer.write_all(data)?;
            writer.finish()?;
        }
        cursor.into_inner()
    };
    let (header, central_at, cd_start) = {
        let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice()))?;
        if archive.len() != 1 {
            return Err(zip_broken("rewritten entry"));
        }
        let cd_start = archive.central_directory_start();
        let file = archive.by_index(0)?;
        (file.header_start(), file.central_header_start(), cd_start)
    };
    let local = slice_bytes(&bytes, header, cd_start)?.to_vec();
    let central = central_record(&bytes, central_at)?;
    Ok((local, central))
}

fn local_record<'a>(
    bytes: &'a [u8],
    header: u64,
    entries: &[(u64, u64)],
    cd_start: u64,
) -> Result<&'a [u8], Error> {
    let header_at = u64_as_usize(header)?;
    if bytes.get(header_at..header_at.saturating_add(4)) != Some(b"PK\x03\x04") {
        return Err(zip_broken("local header"));
    }
    if cd_start < header || cd_start > bytes.len() as u64 {
        return Err(zip_broken("local header"));
    }
    let next = entries
        .iter()
        .map(|(start, _)| *start)
        .filter(|start| *start > header && *start <= cd_start)
        .min()
        .unwrap_or(cd_start);
    let end = match exact_local_end(bytes, header_at) {
        Some(exact) if exact > next => return Err(zip_broken("local header")),
        Some(exact) => exact,
        None => next,
    };
    if end <= header {
        return Err(zip_broken("local header"));
    }
    slice_bytes(bytes, header, end)
}

/// 没有数据描述符、大小也放得进 32 位时，本地记录的结束位置。
fn exact_local_end(bytes: &[u8], header_at: usize) -> Option<u64> {
    let head = bytes.get(header_at..header_at + 30)?;
    let flags = u16::from_le_bytes([head[6], head[7]]);
    if flags & 0x0008 != 0 {
        return None;
    }
    let compressed = u32::from_le_bytes([head[18], head[19], head[20], head[21]]);
    if compressed == u32::MAX {
        return None;
    }
    let name_len = u16::from_le_bytes([head[26], head[27]]) as u64;
    let extra_len = u16::from_le_bytes([head[28], head[29]]) as u64;
    Some(
        (header_at as u64)
            .saturating_add(30)
            .saturating_add(name_len)
            .saturating_add(extra_len)
            .saturating_add(u64::from(compressed)),
    )
}

fn central_record(bytes: &[u8], start: u64) -> Result<Vec<u8>, Error> {
    let start_at = u64_as_usize(start)?;
    if bytes.get(start_at..start_at.saturating_add(4)) != Some(b"PK\x01\x02") {
        return Err(zip_broken("central header"));
    }
    if start_at + 46 > bytes.len() {
        return Err(zip_broken("central header"));
    }
    let name_len = u16_at(bytes, start_at + 28) as usize;
    let extra_len = u16_at(bytes, start_at + 30) as usize;
    let comment_len = u16_at(bytes, start_at + 32) as usize;
    let end = 46usize
        .checked_add(name_len)
        .and_then(|n| n.checked_add(extra_len))
        .and_then(|n| n.checked_add(comment_len))
        .and_then(|n| n.checked_add(start_at))
        .ok_or_else(|| zip_broken("central header"))?;
    if end > bytes.len() {
        return Err(zip_broken("central header"));
    }
    Ok(bytes[start_at..end].to_vec())
}

fn patch_central_offset(central: &mut Vec<u8>, offset: u64) -> Result<(), Error> {
    if central.len() < 46 || central.get(..4) != Some(b"PK\x01\x02") {
        return Err(zip_broken("central header"));
    }
    let name_len = u16_at(central, 28) as usize;
    let extra_len = u16_at(central, 30) as usize;
    let comment_len = u16_at(central, 32) as usize;
    let extra_at = 46 + name_len;
    let extra_end = extra_at + extra_len;
    if extra_end.checked_add(comment_len) != Some(central.len()) {
        return Err(zip_broken("central header"));
    }
    let comp32 = u32_at(central, 20);
    let uncomp32 = u32_at(central, 24);
    let off32 = u32_at(central, 42);

    if let Some((field_at, field_len)) = find_zip64(central, extra_at, extra_end)? {
        let mut pos = 0usize;
        let wide = field_len >= 24;
        if wide || uncomp32 == u32::MAX {
            pos += 8;
        }
        if wide || comp32 == u32::MAX {
            pos += 8;
        }
        let offset_present = wide || off32 == u32::MAX;
        if offset_present {
            if pos + 8 > field_len {
                return Err(zip_broken("zip64 extra"));
            }
            let at = field_at + 4 + pos;
            central[at..at + 8].copy_from_slice(&offset.to_le_bytes());
            if offset > u32::MAX as u64 {
                central[42..46].copy_from_slice(&u32::MAX.to_le_bytes());
            } else if off32 != u32::MAX {
                central[42..46].copy_from_slice(&(offset as u32).to_le_bytes());
            }
            return Ok(());
        }
        if offset <= u32::MAX as u64 && off32 != u32::MAX {
            central[42..46].copy_from_slice(&(offset as u32).to_le_bytes());
            return Ok(());
        }
        return extend_zip64_offset(central, field_at, field_len, offset);
    }

    if offset <= u32::MAX as u64 && off32 != u32::MAX {
        central[42..46].copy_from_slice(&(offset as u32).to_le_bytes());
        return Ok(());
    }
    if off32 == u32::MAX {
        return Err(zip_broken("zip64 extra"));
    }
    insert_zip64_offset(central, offset)
}

fn find_zip64(
    central: &[u8],
    extra_at: usize,
    extra_end: usize,
) -> Result<Option<(usize, usize)>, Error> {
    let mut i = extra_at;
    let mut found = None;
    while i + 4 <= extra_end {
        let id = u16_at(central, i);
        let len = u16_at(central, i + 2) as usize;
        let next = i + 4 + len;
        if next > extra_end {
            return Err(zip_broken("central extra"));
        }
        if id == 0x0001 {
            if found.is_some() {
                return Err(zip_broken("zip64 extra"));
            }
            found = Some((i, len));
        }
        i = next;
    }
    if i != extra_end {
        return Err(zip_broken("central extra"));
    }
    Ok(found)
}

fn extend_zip64_offset(
    central: &mut Vec<u8>,
    field_at: usize,
    field_len: usize,
    offset: u64,
) -> Result<(), Error> {
    let new_len = field_len + 8;
    let new_extra = u16_at(central, 30) as usize + 8;
    if new_len > u16::MAX as usize || new_extra > u16::MAX as usize {
        return Err(zip_broken("zip64 extra"));
    }
    central[field_at + 2..field_at + 4].copy_from_slice(&(new_len as u16).to_le_bytes());
    central[30..32].copy_from_slice(&(new_extra as u16).to_le_bytes());
    central[42..46].copy_from_slice(&u32::MAX.to_le_bytes());
    bump_zip64_version(central);
    let insert_at = field_at + 4 + field_len;
    central.splice(insert_at..insert_at, offset.to_le_bytes());
    Ok(())
}

fn insert_zip64_offset(central: &mut Vec<u8>, offset: u64) -> Result<(), Error> {
    let name_len = u16_at(central, 28) as usize;
    let new_extra = u16_at(central, 30) as usize + 12;
    if new_extra > u16::MAX as usize {
        return Err(zip_broken("zip64 extra"));
    }
    central[30..32].copy_from_slice(&(new_extra as u16).to_le_bytes());
    central[42..46].copy_from_slice(&u32::MAX.to_le_bytes());
    bump_zip64_version(central);
    let mut block = Vec::with_capacity(12);
    block.extend_from_slice(&1u16.to_le_bytes());
    block.extend_from_slice(&8u16.to_le_bytes());
    block.extend_from_slice(&offset.to_le_bytes());
    let extra_at = 46 + name_len;
    central.splice(extra_at..extra_at, block);
    Ok(())
}

fn bump_zip64_version(central: &mut [u8]) {
    if u16_at(central, 6) < 45 {
        central[6..8].copy_from_slice(&45u16.to_le_bytes());
    }
}

fn write_eocd(out: &mut Vec<u8>, centrals: &[Vec<u8>]) -> Result<(), Error> {
    let cd_offset = u64::try_from(out.len()).map_err(|_| zip_broken("offset"))?;
    for central in centrals {
        out.extend_from_slice(central);
    }
    let cd_size = u64::try_from(out.len()).map_err(|_| zip_broken("offset"))? - cd_offset;
    let count = centrals.len() as u64;
    let need_zip64 =
        count > u16::MAX as u64 || cd_size > u32::MAX as u64 || cd_offset > u32::MAX as u64;
    if need_zip64 {
        let zip64_at = u64::try_from(out.len()).map_err(|_| zip_broken("offset"))?;
        out.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        out.extend_from_slice(&44u64.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&zip64_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
    }
    let count16 = if count > u16::MAX as u64 {
        u16::MAX
    } else {
        count as u16
    };
    let size32 = if cd_size > u32::MAX as u64 {
        u32::MAX
    } else {
        cd_size as u32
    };
    let off32 = if cd_offset > u32::MAX as u64 {
        u32::MAX
    } else {
        cd_offset as u32
    };
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count16.to_le_bytes());
    out.extend_from_slice(&count16.to_le_bytes());
    out.extend_from_slice(&size32.to_le_bytes());
    out.extend_from_slice(&off32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    Ok(())
}

fn slice_bytes(bytes: &[u8], start: u64, end: u64) -> Result<&[u8], Error> {
    let start = u64_as_usize(start)?;
    let end = u64_as_usize(end)?;
    if start > end || end > bytes.len() {
        return Err(zip_broken("entry bounds"));
    }
    Ok(&bytes[start..end])
}

fn u64_as_usize(value: u64) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| zip_broken("offset"))
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn zip_broken(detail: &str) -> Error {
    Error::new(ErrorKind::OpcOpen {
        reason: OpcReason::Zip {
            detail: detail.to_string(),
        },
    })
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
    fn untouched_stored_extra_is_kept() {
        let shared = [0x34, 0x12, 4, 0, 0x11, 0x22, 0x33, 0x44];
        let central_only = [0x35, 0x12, 2, 0, 0x09, 0x09];
        let bytes = zip_with_extra();
        let unchanged = Package::open_bytes(bytes.clone())
            .unwrap()
            .save_to_vec()
            .unwrap();
        assert_eq!(count_slice(&unchanged, &shared), 2);
        assert_eq!(count_slice(&unchanged, &central_only), 1);
        let mut pkg = Package::open_bytes(bytes).unwrap();
        pkg.set("/edit.xml", b"new".to_vec()).unwrap();
        let out = pkg.save_to_vec().unwrap();
        let reopened = Package::open_bytes(out.clone()).unwrap();
        assert_eq!(reopened.get("/keep.xml").unwrap(), b"keep-bytes");
        assert_eq!(reopened.get("/edit.xml").unwrap(), b"new");
        assert_eq!(
            count_slice(&out, &shared),
            2,
            "本地头和中央目录都应留下 extra"
        );
        assert_eq!(
            count_slice(&out, &central_only),
            1,
            "只在中央目录的 extra 丢了"
        );
        let mut archive = ZipArchive::new(Cursor::new(out.as_slice())).unwrap();
        let file = archive.by_name("keep.xml").unwrap();
        let extra = file.extra_data().unwrap_or_default();
        assert!(
            extra.windows(shared.len()).any(|window| window == shared),
            "{extra:?}"
        );
        assert!(
            extra
                .windows(central_only.len())
                .any(|window| window == central_only),
            "{extra:?}"
        );
    }

    #[test]
    fn central_offset_patch_updates_zip64_and_plain_fields() {
        let mut plain = central_header(b"a", &[], 5);
        patch_central_offset(&mut plain, 20).unwrap();
        assert_eq!(u32_at(&plain, 42), 20);

        let mut extra = vec![0x01, 0x00, 8, 0];
        extra.extend_from_slice(&10u64.to_le_bytes());
        let mut zip64 = central_header(b"a", &extra, u32::MAX);
        patch_central_offset(&mut zip64, 99).unwrap();
        assert_eq!(u64::from_le_bytes(zip64[51..59].try_into().unwrap()), 99);

        let mut grow = central_header(b"a", &[], 5);
        patch_central_offset(&mut grow, u64::from(u32::MAX) + 3).unwrap();
        assert_eq!(u32_at(&grow, 42), u32::MAX);
        assert_eq!(
            u64::from_le_bytes(grow[51..59].try_into().unwrap()),
            u64::from(u32::MAX) + 3
        );
    }

    fn central_header(name: &[u8], extra: &[u8], offset: u32) -> Vec<u8> {
        let mut header = vec![0u8; 46];
        header[0..4].copy_from_slice(b"PK\x01\x02");
        header[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        header[30..32].copy_from_slice(&(extra.len() as u16).to_le_bytes());
        header[42..46].copy_from_slice(&offset.to_le_bytes());
        header.extend_from_slice(name);
        header.extend_from_slice(extra);
        header
    }

    fn count_slice(hay: &[u8], needle: &[u8]) -> usize {
        hay.windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }

    fn zip_with_extra() -> Vec<u8> {
        use zip::write::FullFileOptions;

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file("[Content_Types].xml", stored).unwrap();
            writer.write_all(b"<Types/>").unwrap();
            writer.start_file("_rels/.rels", stored).unwrap();
            writer.write_all(b"<Relationships/>").unwrap();
            let mut keep = FullFileOptions::default().compression_method(CompressionMethod::Stored);
            keep.add_extra_data(
                0x1234,
                vec![0x11, 0x22, 0x33, 0x44].into_boxed_slice(),
                false,
            )
            .unwrap();
            keep.add_extra_data(0x1235, vec![0x09, 0x09].into_boxed_slice(), true)
                .unwrap();
            writer.start_file("keep.xml", keep).unwrap();
            writer.write_all(b"keep-bytes").unwrap();
            writer.start_file("edit.xml", stored).unwrap();
            writer.write_all(b"old").unwrap();
            writer.finish().unwrap();
        }
        cursor.into_inner()
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

    #[test]
    fn different_case_target_reads_the_part() {
        let bytes = zip_files(&[
            ("[Content_Types].xml", b"<Types/>"),
            (
                "_rels/.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            ("xl/Workbook.xml", b"<workbook/>"),
        ]);
        let pkg = Package::open_bytes(bytes).unwrap();
        assert!(pkg
            .part_names()
            .iter()
            .any(|name| name == "/xl/Workbook.xml"));
        let target = crate::opc::resolve_rel_target("/", "xl/workbook.xml").unwrap();
        assert_eq!(pkg.get(&target).unwrap(), b"<workbook/>");
        let out = pkg.save_to_vec().unwrap();
        let spelled = b"xl/Workbook.xml";
        assert!(out.windows(spelled.len()).any(|window| window == spelled));
    }

    #[test]
    fn duplicate_folded_names_are_opc_open() {
        let bytes = zip_files(&[
            ("[Content_Types].xml", b"<Types/>"),
            ("_rels/.rels", b"<Relationships/>"),
            ("xl/Workbook.xml", b"a"),
            ("xl/workbook.xml", b"b"),
        ]);
        let err = Package::open_bytes(bytes).unwrap_err();
        assert_eq!(err.code(), ErrorCode::OpcOpen);
        assert!(err.to_string().contains("Duplicate part name"), "{err}");
    }

    fn zip_files(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            for (name, data) in files {
                let opt =
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
                writer.start_file(*name, opt).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }
}
