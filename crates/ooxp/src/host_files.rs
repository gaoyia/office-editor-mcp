//! 调用方提供的用户文件。库按路径来取。
//! 本机由 `install_reader` 在真正用到该路径时再读磁盘；事先放进 `install` 的字节直接取。

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use crate::error::{io_path, Error};

pub(crate) struct Cached {
    bytes: Option<Vec<u8>>,
    raw_os: Option<i32>,
    kind: io::ErrorKind,
}

type Reader = fn(&str) -> Cached;

struct Frame {
    files: HashMap<String, Cached>,
    reader: Option<Reader>,
}

thread_local! {
    static STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

pub struct FileGuard;

impl Drop for FileGuard {
    fn drop(&mut self) {
        STACK.with(|slot| {
            slot.borrow_mut().pop();
        });
    }
}

pub fn install(files: HashMap<String, Cached>) -> FileGuard {
    push(Frame {
        files,
        reader: None,
    });
    FileGuard
}

/// 先记下怎么读。真正打开磁盘要等到某条操作来取这个路径。
#[cfg(not(target_arch = "wasm32"))]
pub fn install_reader(read: fn(&str) -> Cached) -> FileGuard {
    push(Frame {
        files: HashMap::new(),
        reader: Some(read),
    });
    FileGuard
}

fn push(frame: Frame) {
    STACK.with(|slot| slot.borrow_mut().push(frame));
}

pub fn read_user_file(path: &str) -> Result<Vec<u8>, Error> {
    if let Some(hit) = cached(path) {
        return hit;
    }
    let reader = STACK.with(|slot| slot.borrow().last().and_then(|frame| frame.reader));
    let Some(reader) = reader else {
        return Err(missing(path));
    };
    let loaded = reader(path);
    let result = cached_result(path, &loaded);
    STACK.with(|slot| {
        if let Some(frame) = slot.borrow_mut().last_mut() {
            frame.files.insert(path.to_string(), loaded);
        }
    });
    result
}

fn cached(path: &str) -> Option<Result<Vec<u8>, Error>> {
    STACK.with(|slot| {
        let guard = slot.borrow();
        let frame = guard.last()?;
        frame.files.get(path).map(|item| cached_result(path, item))
    })
}

fn cached_result(path: &str, item: &Cached) -> Result<Vec<u8>, Error> {
    match item {
        Cached {
            bytes: Some(bytes), ..
        } => Ok(bytes.clone()),
        Cached {
            raw_os: Some(code), ..
        } => Err(io_path(
            io::Error::from_raw_os_error(*code),
            PathBuf::from(path),
        )),
        Cached { kind, .. } => Err(io_path(
            io::Error::new(*kind, "read failed"),
            PathBuf::from(path),
        )),
    }
}

fn missing(path: &str) -> Error {
    io_path(
        io::Error::new(io::ErrorKind::NotFound, "file was not provided"),
        PathBuf::from(path),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn cache_read(path: &str) -> Cached {
    match std::fs::read(path) {
        Ok(bytes) => Cached {
            bytes: Some(bytes),
            raw_os: None,
            kind: io::ErrorKind::Other,
        },
        Err(err) => Cached {
            bytes: None,
            raw_os: err.raw_os_error(),
            kind: err.kind(),
        },
    }
}

pub fn success(path: &str, bytes: Vec<u8>) -> (String, Cached) {
    (
        path.to_string(),
        Cached {
            bytes: Some(bytes),
            raw_os: None,
            kind: io::ErrorKind::Other,
        },
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static HITS: AtomicUsize = AtomicUsize::new(0);

    fn hit(path: &str) -> Cached {
        HITS.fetch_add(1, Ordering::SeqCst);
        Cached {
            bytes: Some(path.as_bytes().to_vec()),
            raw_os: None,
            kind: io::ErrorKind::Other,
        }
    }

    #[test]
    fn reader_waits_until_the_path_is_used() {
        HITS.store(0, Ordering::SeqCst);
        let _guard = install_reader(hit);
        assert_eq!(HITS.load(Ordering::SeqCst), 0);
        assert_eq!(read_user_file("a.png").unwrap(), b"a.png");
        assert_eq!(HITS.load(Ordering::SeqCst), 1);
        assert_eq!(read_user_file("a.png").unwrap(), b"a.png");
        assert_eq!(HITS.load(Ordering::SeqCst), 1);
    }

    static OUTER: AtomicUsize = AtomicUsize::new(0);
    static INNER: AtomicUsize = AtomicUsize::new(0);

    fn outer_read(path: &str) -> Cached {
        OUTER.fetch_add(1, Ordering::SeqCst);
        Cached {
            bytes: Some(format!("outer:{path}").into_bytes()),
            raw_os: None,
            kind: io::ErrorKind::Other,
        }
    }

    fn inner_read(path: &str) -> Cached {
        INNER.fetch_add(1, Ordering::SeqCst);
        Cached {
            bytes: Some(format!("inner:{path}").into_bytes()),
            raw_os: None,
            kind: io::ErrorKind::Other,
        }
    }

    #[test]
    fn inner_guard_restores_the_outer_reader() {
        OUTER.store(0, Ordering::SeqCst);
        INNER.store(0, Ordering::SeqCst);
        let outer = install_reader(outer_read);
        {
            let inner = install_reader(inner_read);
            assert_eq!(read_user_file("a.bin").unwrap(), b"inner:a.bin");
            drop(inner);
        }
        assert_eq!(read_user_file("b.bin").unwrap(), b"outer:b.bin");
        assert_eq!(OUTER.load(Ordering::SeqCst), 1);
        assert_eq!(INNER.load(Ordering::SeqCst), 1);
        drop(outer);
    }
}
