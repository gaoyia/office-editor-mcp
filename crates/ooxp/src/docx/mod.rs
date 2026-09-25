//! docx：inspect / create / apply。

mod apply;
mod create;
mod inspect;
mod para;
mod request;

pub use apply::apply;
pub use create::create;
pub use inspect::inspect;
pub use request::{parse_ops, parse_scope};

pub const WML_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const W14_NS: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
pub const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
pub const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
pub const REL_CORE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const REL_APP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";

pub const CT_DOCUMENT: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
pub const CT_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
pub const CT_CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
pub const CT_APP: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
