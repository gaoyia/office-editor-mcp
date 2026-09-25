//! 领域错误。`code` 字符串稳定，供命令行和调用方识别。

use std::io;
use std::path::PathBuf;

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{message}")]
    App {
        code: ErrorCode,
        message: String,
        path: Option<String>,
    },
    #[error("读盘或写盘失败：{0}")]
    Io(#[from] io::Error),
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

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::App {
            code,
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(code: ErrorCode, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::App {
            code,
            message: message.into(),
            path: Some(path.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::App { code, .. } => *code,
            Self::Io(_) => ErrorCode::Io,
        }
    }

    pub fn field_path(&self) -> Option<&str> {
        match self {
            Self::App { path, .. } => path.as_deref(),
            Self::Io(_) => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.code().exit_code()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut error = json!({
            "code": self.code().as_str(),
            "message": self.to_string(),
        });
        if let Some(p) = self.field_path() {
            error["path"] = json!(p);
        }
        json!({ "ok": false, "error": error })
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        match err {
            zip::result::ZipError::Io(e) => Self::Io(e),
            other => Self::new(ErrorCode::OpcOpen, format!("无法作为 ZIP 打开：{other}")),
        }
    }
}

/// 写文件失败时带上路径。
pub fn io_path(err: io::Error, path: PathBuf) -> Error {
    Error::new(ErrorCode::Io, format!("无法访问 {}：{err}", path.display()))
}
