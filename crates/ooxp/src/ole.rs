//! 嵌入对象的文件分类与 Package 包装。docx / pptx / xlsx 共用。
//! 不执行 OLE 服务器。

use std::path::Path;

use crate::opc::Package;

pub const REL_OLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject";
pub const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
pub const CT_OLE: &str = "application/vnd.openxmlformats-officedocument.oleObject";
pub const CT_XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const CT_XLSM: &str = "application/vnd.ms-excel.sheet.macroEnabled.12";
pub const CT_XLS: &str = "application/vnd.ms-excel";
pub const CT_DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const CT_DOCM: &str = "application/vnd.ms-word.document.macroEnabled.12";
pub const CT_PPTX: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub const CT_PPTM: &str = "application/vnd.ms-powerpoint.presentation.macroEnabled.12";

/// 1×1 PNG，插入时当预览占位，不画图标。
pub const PREVIEW_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];
/// 嵌入文件该怎么进包：Office 文件原样 + package 关系；其余打成 Ole10Native。
pub enum FileSpec {
    Office {
        prog_id: &'static str,
        stem: &'static str,
        ext: &'static str,
        content_type: &'static str,
    },
    Package,
}

impl FileSpec {
    pub fn prog_id(&self) -> &'static str {
        match self {
            Self::Office { prog_id, .. } => prog_id,
            Self::Package => "Package",
        }
    }

    pub fn stem(&self) -> &'static str {
        match self {
            Self::Office { stem, .. } => stem,
            Self::Package => "oleObject",
        }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            Self::Office { ext, .. } => ext,
            Self::Package => "bin",
        }
    }

    pub fn ole_obj_name(&self) -> &'static str {
        match self {
            Self::Office { prog_id, .. } if prog_id.starts_with("Excel") => "Worksheet",
            Self::Office { prog_id, .. } if prog_id.starts_with("Word") => "Document",
            Self::Office { prog_id, .. } if prog_id.starts_with("PowerPoint") => "Presentation",
            Self::Package => "Package",
            Self::Office { .. } => "Object",
        }
    }

    pub fn show_as_icon(&self) -> bool {
        matches!(self, Self::Package)
    }

    pub fn rel_type(&self) -> &'static str {
        match self {
            Self::Office { .. } => REL_PACKAGE,
            Self::Package => REL_OLE,
        }
    }
}

pub fn classify(path: &str) -> FileSpec {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" => FileSpec::Office {
            prog_id: "Excel.Sheet.12",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xlsx",
            content_type: CT_XLSX,
        },
        "xlsm" => FileSpec::Office {
            prog_id: "Excel.SheetMacroEnabled.12",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xlsm",
            content_type: CT_XLSM,
        },
        "xls" => FileSpec::Office {
            prog_id: "Excel.Sheet.8",
            stem: "Microsoft_Excel_Worksheet",
            ext: "xls",
            content_type: CT_XLS,
        },
        "docx" => FileSpec::Office {
            prog_id: "Word.Document.12",
            stem: "Microsoft_Word_Document",
            ext: "docx",
            content_type: CT_DOCX,
        },
        "docm" => FileSpec::Office {
            prog_id: "Word.DocumentMacroEnabled.12",
            stem: "Microsoft_Word_Document",
            ext: "docm",
            content_type: CT_DOCM,
        },
        "pptx" => FileSpec::Office {
            prog_id: "PowerPoint.Show.12",
            stem: "Microsoft_PowerPoint_Presentation",
            ext: "pptx",
            content_type: CT_PPTX,
        },
        "pptm" => FileSpec::Office {
            prog_id: "PowerPoint.ShowMacroEnabled.12",
            stem: "Microsoft_PowerPoint_Presentation",
            ext: "pptm",
            content_type: CT_PPTM,
        },
        _ => FileSpec::Package,
    }
}

pub fn next_numbered(pkg: &Package, dir: &str, stem: &str, ext: &str) -> String {
    let mut n = 1u32;
    loop {
        let name = format!("{dir}/{stem}{n}.{ext}");
        if !pkg.contains(&name) {
            return name;
        }
        n += 1;
    }
}

/// 把普通文件打成 OLE Package 复合文档（Ole10Native）。
pub fn wrap_package(filename: &str, data: &[u8]) -> Vec<u8> {
    let native = ole10_native(filename, data);
    write_cfb(vec![
        ("\u{0001}Ole".into(), ole_stream()),
        ("\u{0001}CompObj".into(), comp_obj_stream()),
        ("\u{0001}Ole10Native".into(), native),
    ])
}

fn ole10_native(filename: &str, data: &[u8]) -> Vec<u8> {
    let name = ascii_filename(filename);
    let mut inner = Vec::new();
    inner.extend_from_slice(&2u16.to_le_bytes());
    inner.extend_from_slice(name.as_bytes());
    inner.push(0);
    inner.extend_from_slice(name.as_bytes());
    inner.push(0);
    inner.extend_from_slice(&0u32.to_le_bytes());
    inner.extend_from_slice(&(data.len() as u32).to_le_bytes());
    inner.extend_from_slice(data);
    let mut out = Vec::new();
    out.extend_from_slice(&(inner.len() as u32).to_le_bytes());
    out.extend(inner);
    out
}

fn ascii_filename(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file.bin");
    name.chars()
        .map(|c| if c.is_ascii() && c != '\0' { c } else { '_' })
        .collect()
}

fn ole_stream() -> Vec<u8> {
    let mut v = vec![0x01, 0x00, 0x00, 0x02];
    v.extend_from_slice(&[0u8; 16]);
    v
}

fn comp_obj_stream() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0x01, 0x00, 0x09, 0x02, 0x00, 0x00, 0x00, 0x00]);
    let package = b"Package\0";
    v.extend_from_slice(&12u32.to_le_bytes());
    v.extend_from_slice(package);
    v.extend_from_slice(&[0, 0, 0, 0]);
    let clip = b"OLE10 Native\0";
    v.extend_from_slice(&(clip.len() as u32).to_le_bytes());
    v.extend_from_slice(clip);
    v.extend_from_slice(&[0u8; 8]);
    v
}

const CFB_SIG: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const SEC: usize = 512;
const EOC: u32 = 0xFFFF_FFFE;
const FATSECT: u32 = 0xFFFF_FFFD;
const FREE: u32 = 0xFFFF_FFFF;
const NONE: u32 = 0xFFFF_FFFF;

fn write_cfb(mut streams: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    streams.sort_by_key(|a| a.0.to_ascii_lowercase());
    let dir_secs = 1usize;
    let stream_secs: Vec<usize> = streams
        .iter()
        .map(|(_, d)| d.len().max(1).div_ceil(SEC).max(1))
        .collect();
    let data_secs: usize = stream_secs.iter().sum();
    let mut fat_secs = 1usize;
    loop {
        let total = fat_secs + dir_secs + data_secs;
        let need = total.div_ceil(128);
        if need <= fat_secs {
            break;
        }
        fat_secs = need;
    }
    let mut fat = vec![FREE; fat_secs * 128];
    for slot in fat.iter_mut().take(fat_secs) {
        *slot = FATSECT;
    }
    let dir_start = fat_secs as u32;
    chain(&mut fat, dir_start, dir_secs);
    let mut next = fat_secs + dir_secs;
    let mut dir = vec![DirEnt::root()];
    for ((name, data), nsec) in streams.iter().zip(stream_secs.iter()) {
        chain(&mut fat, next as u32, *nsec);
        dir.push(DirEnt::stream(name, next as u32, data.len() as u64));
        next += *nsec;
    }
    dir[0].child = 1;
    let last = dir.len() - 1;
    for (i, ent) in dir.iter_mut().enumerate().take(last + 1).skip(1) {
        ent.color = 1;
        ent.left = NONE;
        ent.right = if i < last { i as u32 + 1 } else { NONE };
    }
    while dir.len() < 4 {
        dir.push(DirEnt::empty());
    }

    let mut out = vec![0u8; SEC + (fat_secs + dir_secs + data_secs) * SEC];
    write_header(&mut out[..SEC], fat_secs as u32, dir_start);
    for (i, ent) in fat.iter().take(fat_secs * 128).enumerate() {
        let off = SEC + (i / 128) * SEC + (i % 128) * 4;
        out[off..off + 4].copy_from_slice(&ent.to_le_bytes());
    }
    let dir_off = SEC + fat_secs * SEC;
    for (i, ent) in dir.iter().enumerate() {
        write_dirent(&mut out[dir_off + i * 128..dir_off + (i + 1) * 128], ent);
    }
    let mut data_off = SEC + (fat_secs + dir_secs) * SEC;
    for ((_, data), nsec) in streams.iter().zip(stream_secs.iter()) {
        let n = *nsec * SEC;
        out[data_off..data_off + data.len()].copy_from_slice(data);
        data_off += n;
    }
    out
}

fn chain(fat: &mut [u32], start: u32, n: usize) {
    for i in 0..n {
        let cur = start as usize + i;
        fat[cur] = if i + 1 == n {
            EOC
        } else {
            start + i as u32 + 1
        };
    }
}

fn write_header(buf: &mut [u8], fat_secs: u32, dir_start: u32) {
    buf[..8].copy_from_slice(&CFB_SIG);
    buf[0x18..0x1A].copy_from_slice(&0x003Eu16.to_le_bytes());
    buf[0x1A..0x1C].copy_from_slice(&0x0003u16.to_le_bytes());
    buf[0x1C..0x1E].copy_from_slice(&0xFFFEu16.to_le_bytes());
    buf[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
    buf[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    buf[0x2C..0x30].copy_from_slice(&fat_secs.to_le_bytes());
    buf[0x30..0x34].copy_from_slice(&dir_start.to_le_bytes());
    buf[0x38..0x3C].copy_from_slice(&4096u32.to_le_bytes());
    buf[0x3C..0x40].copy_from_slice(&EOC.to_le_bytes());
    buf[0x44..0x48].copy_from_slice(&EOC.to_le_bytes());
    for i in 0..109 {
        let off = 0x4C + i * 4;
        let v = if (i as u32) < fat_secs {
            i as u32
        } else {
            FREE
        };
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

struct DirEnt {
    name: String,
    kind: u8,
    color: u8,
    left: u32,
    right: u32,
    child: u32,
    start: u32,
    size: u64,
}

impl DirEnt {
    fn root() -> Self {
        Self {
            name: "Root Entry".into(),
            kind: 5,
            color: 1,
            left: NONE,
            right: NONE,
            child: NONE,
            start: EOC,
            size: 0,
        }
    }

    fn stream(name: &str, start: u32, size: u64) -> Self {
        Self {
            name: name.into(),
            kind: 2,
            color: 1,
            left: NONE,
            right: NONE,
            child: NONE,
            start,
            size,
        }
    }

    fn empty() -> Self {
        Self {
            name: String::new(),
            kind: 0,
            color: 0,
            left: NONE,
            right: NONE,
            child: NONE,
            start: EOC,
            size: 0,
        }
    }
}

fn write_dirent(buf: &mut [u8], e: &DirEnt) {
    let utf16: Vec<u16> = e.name.encode_utf16().collect();
    let mut name_bytes = [0u8; 64];
    for (i, u) in utf16.iter().take(31).enumerate() {
        name_bytes[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
    }
    buf[..64].copy_from_slice(&name_bytes);
    let name_len = if e.kind == 0 {
        0u16
    } else {
        ((utf16.len() + 1) * 2) as u16
    };
    buf[0x40..0x42].copy_from_slice(&name_len.to_le_bytes());
    buf[0x42] = e.kind;
    buf[0x43] = e.color;
    buf[0x44..0x48].copy_from_slice(&e.left.to_le_bytes());
    buf[0x48..0x4C].copy_from_slice(&e.right.to_le_bytes());
    buf[0x4C..0x50].copy_from_slice(&e.child.to_le_bytes());
    buf[0x74..0x78].copy_from_slice(&e.start.to_le_bytes());
    buf[0x78..0x80].copy_from_slice(&e.size.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_package_is_cfb_and_keeps_bytes() {
        let payload = b"hello-ole-package";
        let bin = wrap_package("notes.txt", payload);
        assert_eq!(&bin[..8], &CFB_SIG);
        assert!(bin.windows(payload.len()).any(|w| w == payload));
    }
}
