//! Excel 的字节函数按 Windows ANSI 代码页计。拆开的双字节换成空格。

#[derive(Clone, Copy)]
pub(crate) struct Marked {
    byte: u8,
    /// 0 表示单字节。同一对的前导字节和后续字节共用一个非 0 编号。
    pair: u32,
    lead: bool,
}

pub(crate) fn encode(text: &str) -> Option<Vec<Marked>> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let bytes = wide_to_bytes(&wide)?;
    Some(mark(&bytes))
}

pub(crate) fn decode(units: &[Marked]) -> Option<String> {
    let healed = heal(units);
    let bytes: Vec<u8> = healed.into_iter().map(|unit| unit.byte).collect();
    let wide = bytes_to_wide(&bytes)?;
    String::from_utf16(&wide).ok()
}

pub(crate) fn slice(units: &[Marked], start: usize, len: usize) -> Vec<Marked> {
    let end = start.saturating_add(len);
    let cut: Vec<Marked> = units
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= start && *i < end)
        .map(|(_, unit)| *unit)
        .collect();
    heal(&cut)
}

pub(crate) fn replace(
    units: &[Marked],
    start: usize,
    len: usize,
    new_units: &[Marked],
) -> Vec<Marked> {
    let start = start.min(units.len());
    let end = start.saturating_add(len).min(units.len());
    let max = units.iter().map(|unit| unit.pair).max().unwrap_or(0);
    let shifted: Vec<Marked> = new_units
        .iter()
        .map(|unit| {
            let mut copy = *unit;
            if copy.pair != 0 {
                copy.pair = copy.pair.saturating_add(max);
            }
            copy
        })
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(&units[..start]);
    out.extend_from_slice(&shifted);
    out.extend_from_slice(&units[end..]);
    heal(&out)
}

/// 第一个字符在 ANSI 代码页里的码。空串没有码。单字节能编成 1 字节，汉字编成 2 字节。
pub(crate) fn code_of(text: &str) -> Option<u32> {
    let ch = text.chars().next()?;
    let units = encode(&ch.to_string())?;
    match units.as_slice() {
        [one] => Some(u32::from(one.byte)),
        [hi, lo] => Some((u32::from(hi.byte) << 8) | u32::from(lo.byte)),
        _ => None,
    }
}

pub(crate) fn char_of(code: u32) -> Option<String> {
    if code == 0 || code > 0xFFFF {
        return None;
    }
    let units = if code < 256 {
        vec![Marked {
            byte: code as u8,
            pair: 0,
            lead: false,
        }]
    } else {
        vec![
            Marked {
                byte: (code >> 8) as u8,
                pair: 1,
                lead: true,
            },
            Marked {
                byte: (code & 0xFF) as u8,
                pair: 1,
                lead: false,
            },
        ]
    };
    decode(&units)
}

fn mark(bytes: &[u8]) -> Vec<Marked> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut pair = 1u32;
    while i < bytes.len() {
        if is_lead(bytes[i]) && i + 1 < bytes.len() {
            out.push(Marked {
                byte: bytes[i],
                pair,
                lead: true,
            });
            out.push(Marked {
                byte: bytes[i + 1],
                pair,
                lead: false,
            });
            pair += 1;
            i += 2;
        } else {
            out.push(Marked {
                byte: bytes[i],
                pair: 0,
                lead: false,
            });
            i += 1;
        }
    }
    out
}

fn heal(units: &[Marked]) -> Vec<Marked> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        if unit.pair != 0 && unit.lead {
            let paired = units
                .get(i + 1)
                .is_some_and(|next| next.pair == unit.pair && !next.lead);
            if paired {
                out.push(unit);
                out.push(units[i + 1]);
                i += 2;
                continue;
            }
            out.push(space());
            i += 1;
        } else if unit.pair != 0 {
            out.push(space());
            i += 1;
        } else {
            out.push(unit);
            i += 1;
        }
    }
    out
}

fn space() -> Marked {
    Marked {
        byte: b' ',
        pair: 0,
        lead: false,
    }
}

fn is_lead(byte: u8) -> bool {
    #[cfg(windows)]
    {
        unsafe { IsDBCSLeadByteEx(GetACP(), byte) != 0 }
    }
    #[cfg(not(windows))]
    {
        let _ = byte;
        false
    }
}

fn wide_to_bytes(wide: &[u16]) -> Option<Vec<u8>> {
    if wide.is_empty() {
        return Some(Vec::new());
    }
    #[cfg(windows)]
    unsafe {
        let need = WideCharToMultiByte(
            0,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if need <= 0 {
            return None;
        }
        let mut buf = vec![0u8; need as usize];
        let wrote = WideCharToMultiByte(
            0,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            buf.as_mut_ptr(),
            need,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if wrote <= 0 {
            return None;
        }
        buf.truncate(wrote as usize);
        Some(buf)
    }
    #[cfg(not(windows))]
    {
        let mut out = Vec::with_capacity(wide.len());
        for unit in wide {
            if *unit < 128 {
                out.push(*unit as u8);
            } else {
                return None;
            }
        }
        Some(out)
    }
}

fn bytes_to_wide(bytes: &[u8]) -> Option<Vec<u16>> {
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    #[cfg(windows)]
    unsafe {
        let need = MultiByteToWideChar(
            0,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        if need <= 0 {
            return None;
        }
        let mut buf = vec![0u16; need as usize];
        let wrote = MultiByteToWideChar(
            0,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            buf.as_mut_ptr(),
            need,
        );
        if wrote <= 0 {
            return None;
        }
        buf.truncate(wrote as usize);
        Some(buf)
    }
    #[cfg(not(windows))]
    {
        if bytes.iter().all(|byte| byte.is_ascii()) {
            Some(bytes.iter().map(|byte| u16::from(*byte)).collect())
        } else {
            None
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn WideCharToMultiByte(
        code_page: u32,
        flags: u32,
        src: *const u16,
        src_len: i32,
        dst: *mut u8,
        dst_len: i32,
        default_char: *const u8,
        used: *mut i32,
    ) -> i32;
    fn MultiByteToWideChar(
        code_page: u32,
        flags: u32,
        src: *const u8,
        src_len: i32,
        dst: *mut u16,
        dst_len: i32,
    ) -> i32;
    fn IsDBCSLeadByteEx(code_page: u32, test: u8) -> i32;
    fn GetACP() -> u32;
}
