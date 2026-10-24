//! 调用方事先读好的用户文件。库按 JSON 里的路径来取，不再自己打开磁盘。

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

thread_local! {
    static FILES: RefCell<Option<HashMap<String, Cached>>> = const { RefCell::new(None) };
}

pub struct FileGuard;

impl Drop for FileGuard {
    fn drop(&mut self) {
        FILES.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

pub fn install(files: HashMap<String, Cached>) -> FileGuard {
    FILES.with(|slot| {
        *slot.borrow_mut() = Some(files);
    });
    FileGuard
}

pub fn read_user_file(path: &str) -> Result<Vec<u8>, Error> {
    FILES.with(|slot| {
        let guard = slot.borrow();
        let Some(map) = guard.as_ref() else {
            return Err(missing(path));
        };
        match map.get(path) {
            Some(Cached {
                bytes: Some(bytes),
                ..
            }) => Ok(bytes.clone()),
            Some(Cached {
                raw_os: Some(code),
                ..
            }) => Err(io_path(
                io::Error::from_raw_os_error(*code),
                PathBuf::from(path),
            )),
            Some(Cached { kind, .. }) => Err(io_path(
                io::Error::new(*kind, "read failed"),
                PathBuf::from(path),
            )),
            None => Err(missing(path)),
        }
    })
}

fn missing(path: &str) -> Error {
    io_path(
        io::Error::new(io::ErrorKind::NotFound, "file was not provided"),
        PathBuf::from(path),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_json(value: &serde_json::Value) -> FileGuard {
    let mut paths = Vec::new();
    collect_paths(value, &mut paths);
    paths.sort();
    paths.dedup();
    let map = paths
        .into_iter()
        .map(|path| {
            let cached = cache_read(&path);
            (path, cached)
        })
        .collect();
    install(map)
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_paths(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "file" || key == "sound" {
                    if let Some(path) = child.as_str() {
                        if !path.is_empty() {
                            out.push(path.to_string());
                        }
                    }
                }
                collect_paths(child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_paths(item, out);
            }
        }
        _ => {}
    }
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
