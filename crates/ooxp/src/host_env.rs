//! 调用方提供的时间、进程号和随机数。本机没有另行传入时，仍用系统的这三项。
//! GUID 和批注的时间字符串也在这里生成。

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

struct Env {
    now_nanos: fn() -> u128,
    unix_secs_millis: fn() -> (u64, u32),
    pid: fn() -> u128,
    random: fn(&mut [u8]),
}

thread_local! {
    static ENV: RefCell<Vec<Env>> = const { RefCell::new(Vec::new()) };
}

pub struct EnvGuard;

impl Drop for EnvGuard {
    fn drop(&mut self) {
        ENV.with(|slot| {
            slot.borrow_mut().pop();
        });
    }
}

pub fn install(
    now_nanos: fn() -> u128,
    unix_secs_millis: fn() -> (u64, u32),
    pid: fn() -> u128,
    random: fn(&mut [u8]),
) -> EnvGuard {
    ENV.with(|slot| {
        slot.borrow_mut().push(Env {
            now_nanos,
            unix_secs_millis,
            pid,
            random,
        });
    });
    EnvGuard
}

pub fn now_nanos() -> u128 {
    if let Some(v) = ENV.with(|slot| slot.borrow().last().map(|env| (env.now_nanos)())) {
        return v;
    }
    native_now_nanos()
}

pub fn unix_secs_millis() -> (u64, u32) {
    if let Some(v) = ENV.with(|slot| slot.borrow().last().map(|env| (env.unix_secs_millis)())) {
        return v;
    }
    native_unix_secs_millis()
}

pub fn pid() -> u128 {
    if let Some(v) = ENV.with(|slot| slot.borrow().last().map(|env| (env.pid)())) {
        return v;
    }
    native_pid()
}

static GUID_SEQ: AtomicU64 = AtomicU64::new(1);

/// 由时间、进程号和本进程计数拼出的 GUID，带花括号，大写。
pub fn new_guid() -> String {
    let t = now_nanos();
    let n = GUID_SEQ.fetch_add(1, Ordering::Relaxed) as u128;
    let proc = pid();
    let mix = t ^ (n << 48) ^ (proc << 16);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = (mix >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
    }
    braced_guid(b)
}

/// 由随机数拼出的 GUID，格式与 `new_guid` 相同。
pub fn random_guid() -> String {
    let mut b = [0u8; 16];
    fill_random(&mut b);
    braced_guid(b)
}

fn braced_guid(mut b: [u8; 16]) -> String {
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

pub fn utc_now_millis() -> String {
    let (secs, millis) = unix_secs_millis();
    unix_to_dt(secs, millis)
}

pub fn looks_like_guid(s: &str) -> bool {
    let inner = s
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or("");
    inner.len() == 36 && inner.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn unix_to_dt(secs: u64, millis: u32) -> String {
    let mut days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year = 1970u64;
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let dim = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for (i, d) in dim.iter().enumerate() {
        if days < *d {
            month = i as u64 + 1;
            break;
        }
        days -= *d;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}")
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && !y.is_multiple_of(100) || y.is_multiple_of(400)
}

pub fn fill_random(buf: &mut [u8]) {
    if let Some(fill) = ENV.with(|slot| slot.borrow().last().map(|env| env.random)) {
        fill(buf);
        return;
    }
    native_fill_random(buf);
}

#[cfg(not(target_arch = "wasm32"))]
fn native_now_nanos() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn native_now_nanos() -> u128 {
    0
}

#[cfg(not(target_arch = "wasm32"))]
fn native_unix_secs_millis() -> (u64, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs(), dur.subsec_millis())
}

#[cfg(target_arch = "wasm32")]
fn native_unix_secs_millis() -> (u64, u32) {
    (0, 0)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_pid() -> u128 {
    std::process::id() as u128
}

#[cfg(target_arch = "wasm32")]
fn native_pid() -> u128 {
    0
}

#[cfg(not(target_arch = "wasm32"))]
fn native_fill_random(buf: &mut [u8]) {
    getrandom::getrandom(buf).expect("系统随机数不可用");
}

#[cfg(target_arch = "wasm32")]
fn native_fill_random(buf: &mut [u8]) {
    let _ = buf;
    panic!("系统随机数不可用");
}

/// 本机命令行用的时钟和随机数。每次调用都问系统，不在入口拍一张快照。
#[cfg(not(target_arch = "wasm32"))]
pub fn install_native() -> EnvGuard {
    install(
        native_now_nanos,
        native_unix_secs_millis,
        native_pid,
        native_fill_random,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static OUTER: AtomicUsize = AtomicUsize::new(0);
    static INNER: AtomicUsize = AtomicUsize::new(0);

    fn fixed_now() -> u128 {
        1
    }

    fn fixed_millis() -> (u64, u32) {
        (1, 0)
    }

    fn fixed_pid() -> u128 {
        1
    }

    fn outer_random(buf: &mut [u8]) {
        OUTER.fetch_add(1, Ordering::SeqCst);
        buf.fill(1);
    }

    fn inner_random(buf: &mut [u8]) {
        INNER.fetch_add(1, Ordering::SeqCst);
        buf.fill(2);
    }

    #[test]
    fn inner_guard_restores_the_outer_random() {
        OUTER.store(0, Ordering::SeqCst);
        INNER.store(0, Ordering::SeqCst);
        let outer = install(fixed_now, fixed_millis, fixed_pid, outer_random);
        {
            let inner = install(fixed_now, fixed_millis, fixed_pid, inner_random);
            let mut buf = [0u8; 1];
            fill_random(&mut buf);
            assert_eq!(buf, [2]);
            drop(inner);
        }
        let mut buf = [0u8; 1];
        fill_random(&mut buf);
        assert_eq!(buf, [1]);
        assert_eq!(OUTER.load(Ordering::SeqCst), 1);
        assert_eq!(INNER.load(Ordering::SeqCst), 1);
        drop(outer);
    }
}
