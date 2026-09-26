//! 领域错误。`code` 字符串稳定，供命令行和调用方识别。
//! `message` 按语言渲染，默认英文。`ErrorKind` 不导出。

mod kind;
mod messages;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

pub(crate) use kind::{
    ErrorKind, IdReason, OpcReason, RangeReason, RelSource, ScopeNeed, SheetNameReason, TypeName,
    XmlIssue,
};

#[derive(Debug)]
pub struct Error {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    App {
        kind: ErrorKind,
        path: Option<String>,
    },
    Io {
        err: io::Error,
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidJson,
    UnsupportedSchema,
    UnsupportedAction,
    UnsupportedFormat,
    UnsupportedOp,
    InvalidField,
    InvalidRange,
    InvalidId,
    IdNotFound,
    ScopeRequired,
    TruncatedLimit,
    OpcOpen,
    OpcPartMissing,
    XmlBroken,
    Io,
    NotImplemented,
}

/// 错误文案语言。第一期只有英文和简体中文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    ZhCn,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "InvalidJson",
            Self::UnsupportedSchema => "UnsupportedSchema",
            Self::UnsupportedAction => "UnsupportedAction",
            Self::UnsupportedFormat => "UnsupportedFormat",
            Self::UnsupportedOp => "UnsupportedOp",
            Self::InvalidField => "InvalidField",
            Self::InvalidRange => "InvalidRange",
            Self::InvalidId => "InvalidId",
            Self::IdNotFound => "IdNotFound",
            Self::ScopeRequired => "ScopeRequired",
            Self::TruncatedLimit => "TruncatedLimit",
            Self::OpcOpen => "OpcOpen",
            Self::OpcPartMissing => "OpcPartMissing",
            Self::XmlBroken => "XmlBroken",
            Self::Io => "Io",
            Self::NotImplemented => "NotImplemented",
        }
    }

    /// 命令行退出码，见 `docs/errors.md`。
    pub fn exit_code(self) -> i32 {
        match self {
            Self::IdNotFound | Self::OpcOpen | Self::OpcPartMissing | Self::XmlBroken => 3,
            Self::Io => 4,
            _ => 2,
        }
    }
}

impl Lang {
    /// 宽松匹配语言标签。不认识的一律英文，不报错。
    pub fn parse(tag: &str) -> Self {
        let tag = tag.trim();
        if tag.is_empty() {
            return Self::En;
        }
        let primary = tag.split('-').next().unwrap_or("").to_ascii_lowercase();
        match primary.as_str() {
            "zh" => Self::ZhCn,
            _ => Self::En,
        }
    }

    /// 请求 `locale` 优先，其次命令行，再次环境变量，默认英文。
    pub fn resolve(request: Option<Self>, cli: Option<&str>, env: Option<&str>) -> Self {
        if let Some(lang) = request {
            return lang;
        }
        if let Some(s) = cli {
            return Self::parse(s);
        }
        if let Some(s) = env {
            return Self::parse(s);
        }
        Self::En
    }
}

/// 浅解析请求里的 `locale`。解析不了或没有该字段时返回 `None`。
pub fn peek_lang(input_json: &str) -> Option<Lang> {
    let value: serde_json::Value = serde_json::from_str(input_json).ok()?;
    let loc = value.get("locale")?.as_str()?;
    Some(Lang::parse(loc))
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            inner: Inner::App { kind, path: None },
        }
    }

    pub(crate) fn with_path(kind: ErrorKind, path: impl Into<String>) -> Self {
        Self {
            inner: Inner::App {
                kind,
                path: Some(path.into()),
            },
        }
    }

    /// 给已有错误补上 JSON 字段路径，不改身份。
    pub(crate) fn at(self, path: impl Into<String>) -> Self {
        match self.inner {
            Inner::App { kind, .. } => Self {
                inner: Inner::App {
                    kind,
                    path: Some(path.into()),
                },
            },
            other => Self { inner: other },
        }
    }

    pub(crate) fn xml(issue: XmlIssue) -> Self {
        Self::new(ErrorKind::XmlBroken {
            issue,
            detail: None,
        })
    }

    pub(crate) fn xml_detail(issue: XmlIssue, detail: impl ToString) -> Self {
        Self::new(ErrorKind::XmlBroken {
            issue,
            detail: Some(detail.to_string()),
        })
    }

    pub fn code(&self) -> ErrorCode {
        match &self.inner {
            Inner::App { kind, .. } => kind.code(),
            Inner::Io { .. } => ErrorCode::Io,
        }
    }

    pub fn field_path(&self) -> Option<&str> {
        match &self.inner {
            Inner::App { path, .. } => path.as_deref(),
            Inner::Io { .. } => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.code().exit_code()
    }

    fn render(&self, lang: Lang) -> String {
        match &self.inner {
            Inner::App { kind, .. } => messages::render(lang, kind),
            Inner::Io { err, path } => render_io(lang, err, path.as_deref()),
        }
    }

    fn os_message(&self) -> Option<String> {
        let Inner::Io { err, .. } = &self.inner else {
            return None;
        };
        let s = err.to_string();
        if s.is_empty() {
            return None;
        }
        if let Some(n) = err.raw_os_error() {
            let suffix = format!(" (os error {n})");
            if let Some(stripped) = s.strip_suffix(&suffix) {
                let t = stripped.trim();
                if t.is_empty() {
                    return None;
                }
                return Some(t.to_string());
            }
        }
        Some(s)
    }

    pub fn to_json(&self, lang: Lang) -> serde_json::Value {
        let mut error = json!({
            "code": self.code().as_str(),
            "message": self.render(lang),
        });
        if let Some(p) = self.field_path() {
            error["path"] = json!(p);
        }
        if let Some(os) = self.os_message() {
            error["osMessage"] = json!(os);
        }
        json!({ "ok": false, "error": error })
    }
}

fn render_io(lang: Lang, err: &io::Error, path: Option<&Path>) -> String {
    let os = err.raw_os_error();
    match (lang, path, os) {
        (Lang::En, Some(p), Some(n)) => format!("Cannot access \"{}\" (os error {n})", p.display()),
        (Lang::En, Some(p), None) => format!("Cannot access \"{}\"", p.display()),
        (Lang::En, None, Some(n)) => format!("I/O failed (os error {n})"),
        (Lang::En, None, None) => "I/O failed".into(),
        (Lang::ZhCn, Some(p), Some(n)) => {
            format!("无法访问「{}」（os error {n}）", p.display())
        }
        (Lang::ZhCn, Some(p), None) => format!("无法访问「{}」", p.display()),
        (Lang::ZhCn, None, Some(n)) => format!("读写失败（os error {n}）"),
        (Lang::ZhCn, None, None) => "读写失败".into(),
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(Lang::En))
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            inner: Inner::Io { err, path: None },
        }
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        match err {
            zip::result::ZipError::Io(e) => Self {
                inner: Inner::Io { err: e, path: None },
            },
            other => Self::new(ErrorKind::OpcOpen {
                reason: OpcReason::Zip {
                    detail: other.to_string(),
                },
            }),
        }
    }
}

/// 写文件失败时带上路径。
pub fn io_path(err: io::Error, path: PathBuf) -> Error {
    Error {
        inner: Inner::Io {
            err,
            path: Some(path),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind as IoErrorKind};

    #[test]
    fn every_kind_renders_nonempty_in_both_langs() {
        for kind in ErrorKind::all_samples() {
            for lang in [Lang::En, Lang::ZhCn] {
                let text = messages::render(lang, &kind);
                assert!(
                    !text.is_empty(),
                    "{kind:?} 在 {lang:?} 下得到空文案"
                );
                // 文案里一律不许出现花括号：真漏了替换是 bug，写成模板给人看也会被
                // 当成 bug。要举例 id 的形状就写 `slide:N`。
                assert!(
                    !text.contains('{'),
                    "{kind:?} 在 {lang:?} 下仍有未替换占位符：{text}"
                );
            }
        }
    }

    #[test]
    fn lang_parse_is_loose() {
        assert_eq!(Lang::parse("zh"), Lang::ZhCn);
        assert_eq!(Lang::parse("zh-CN"), Lang::ZhCn);
        assert_eq!(Lang::parse("zh-Hans"), Lang::ZhCn);
        assert_eq!(Lang::parse("zh-SG"), Lang::ZhCn);
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("en-US"), Lang::En);
        assert_eq!(Lang::parse("en-GB"), Lang::En);
        assert_eq!(Lang::parse("ja"), Lang::En);
        assert_eq!(Lang::parse(""), Lang::En);
        assert_eq!(Lang::parse("???"), Lang::En);
    }

    #[test]
    fn lang_resolve_priority() {
        assert_eq!(
            Lang::resolve(Some(Lang::ZhCn), Some("en"), Some("en")),
            Lang::ZhCn
        );
        assert_eq!(Lang::resolve(None, Some("zh-CN"), Some("en")), Lang::ZhCn);
        assert_eq!(Lang::resolve(None, None, Some("zh-Hans")), Lang::ZhCn);
        assert_eq!(Lang::resolve(None, None, None), Lang::En);
    }

    #[test]
    fn peek_lang_reads_locale_only() {
        assert_eq!(peek_lang(r#"{"locale":"zh-CN"}"#), Some(Lang::ZhCn));
        assert_eq!(peek_lang(r#"{"locale":"ja"}"#), Some(Lang::En));
        assert_eq!(peek_lang(r#"{}"#), None);
        assert_eq!(peek_lang("not json"), None);
        assert_eq!(peek_lang(r#"{"locale":1}"#), None);
    }

    #[test]
    fn to_json_default_english_and_chinese() {
        let err = Error::new(ErrorKind::UnsupportedSchema);
        let en = err.to_json(Lang::En);
        assert_eq!(en["error"]["code"], "UnsupportedSchema");
        assert_eq!(en["error"]["message"], "schemaVersion must be 1");
        assert!(en["error"].get("osMessage").is_none());
        let zh = err.to_json(Lang::ZhCn);
        assert_eq!(zh["error"]["message"], "schemaVersion 必须是 1");
        assert_eq!(err.to_string(), "schemaVersion must be 1");
    }

    #[test]
    fn io_message_keeps_os_text_out_of_message() {
        let err = io_path(
            IoError::new(IoErrorKind::NotFound, "系统找不到指定的路径。"),
            PathBuf::from(r"C:\nope\missing.xlsx"),
        );
        let en = err.to_json(Lang::En);
        let msg = en["error"]["message"].as_str().unwrap();
        assert!(msg.contains("Cannot access"));
        assert!(msg.contains(r"C:\nope\missing.xlsx") || msg.contains("missing.xlsx"));
        assert!(!msg.contains("系统找不到"));
        assert_eq!(en["error"]["osMessage"], "系统找不到指定的路径。");
        let zh = err.to_json(Lang::ZhCn);
        let zh_msg = zh["error"]["message"].as_str().unwrap();
        assert!(zh_msg.contains("无法访问"));
        assert!(!zh_msg.contains("系统找不到"));
    }

    #[test]
    fn exit_codes_unchanged() {
        assert_eq!(ErrorCode::InvalidJson.exit_code(), 2);
        assert_eq!(ErrorCode::IdNotFound.exit_code(), 3);
        assert_eq!(ErrorCode::OpcOpen.exit_code(), 3);
        assert_eq!(ErrorCode::Io.exit_code(), 4);
        assert_eq!(ErrorCode::NotImplemented.exit_code(), 2);
    }
}
