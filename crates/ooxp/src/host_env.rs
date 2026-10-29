//! 调用方提供的时间、进程号和随机数。本机没有另行传入时，仍用系统的这三项。

use std::cell::RefCell;

struct Env {
    now_nanos: fn() -> u128,
    unix_secs_millis: fn() -> (u64, u32),
    unix_secs_nanos: fn() -> (u64, u32),
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
    unix_secs_nanos: fn() -> (u64, u32),
    pid: fn() -> u128,
    random: fn(&mut [u8]),
) -> EnvGuard {
    ENV.with(|slot| {
        slot.borrow_mut().push(Env {
            now_nanos,
            unix_secs_millis,
            unix_secs_nanos,
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

pub fn unix_secs_nanos() -> (u64, u32) {
    if let Some(v) = ENV.with(|slot| slot.borrow().last().map(|env| (env.unix_secs_nanos)())) {
        return v;
    }
    native_unix_secs_nanos()
}

pub fn pid() -> u128 {
    if let Some(v) = ENV.with(|slot| slot.borrow().last().map(|env| (env.pid)())) {
        return v;
    }
    native_pid()
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
fn native_unix_secs_nanos() -> (u64, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs(), dur.subsec_nanos())
}

#[cfg(target_arch = "wasm32")]
fn native_unix_secs_nanos() -> (u64, u32) {
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
        native_unix_secs_nanos,
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

    fn fixed_nanos() -> (u64, u32) {
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
        let outer = install(
            fixed_now,
            fixed_millis,
            fixed_nanos,
            fixed_pid,
            outer_random,
        );
        {
            let inner = install(
                fixed_now,
                fixed_millis,
                fixed_nanos,
                fixed_pid,
                inner_random,
            );
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
