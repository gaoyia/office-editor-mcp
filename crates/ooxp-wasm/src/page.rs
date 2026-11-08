//! 只在 wasm 上编进页面。本机测试走 `Host::run_inner`。

use wasm_bindgen::prelude::*;

use super::{Host, Outcome};

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

/// 页面持有的会话。名字在 JS 里是 `Session`。
#[wasm_bindgen(js_name = Session)]
pub struct Session {
    inner: Host,
}

#[wasm_bindgen(js_class = Session)]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Session {
        Session {
            inner: Host::boot(),
        }
    }

    pub fn clear_files(&mut self) {
        self.inner.clear_files();
    }

    /// `name` 必须和领域 JSON 里的 `file` 或 `sound` 原文一致。
    pub fn put_file(&mut self, name: &str, bytes: &[u8]) {
        self.inner.put_file(name, bytes);
    }

    pub fn run(&mut self, request_json: &str, package: Option<Vec<u8>>) -> JsValue {
        outcome_to_js(self.inner.run_inner(request_json, package.as_deref()))
    }
}

fn outcome_to_js(outcome: Outcome) -> JsValue {
    match outcome {
        Outcome::Fail(value) => to_js(&value),
        Outcome::Json(value) => {
            let obj = js_sys::Object::new();
            set(&obj, "ok", JsValue::TRUE);
            set(&obj, "kind", JsValue::from_str("json"));
            set(&obj, "value", to_js(&value));
            obj.into()
        }
        Outcome::Bytes {
            path,
            bytes,
            content_type,
            ops_applied,
        } => {
            let obj = js_sys::Object::new();
            set(&obj, "ok", JsValue::TRUE);
            set(&obj, "kind", JsValue::from_str("bytes"));
            set(&obj, "path", JsValue::from_str(&path));
            set(
                &obj,
                "bytes",
                js_sys::Uint8Array::from(bytes.as_slice()).into(),
            );
            if let Some(kind) = content_type {
                set(&obj, "contentType", JsValue::from_str(&kind));
            }
            if let Some(n) = ops_applied {
                set(&obj, "opsApplied", JsValue::from_f64(n as f64));
            }
            obj.into()
        }
    }
}

fn set(target: &js_sys::Object, key: &str, value: JsValue) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), &value);
}

fn to_js(value: &serde_json::Value) -> JsValue {
    let text = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    js_sys::JSON::parse(&text).unwrap_or(JsValue::NULL)
}
