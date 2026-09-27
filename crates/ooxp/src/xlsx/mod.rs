//! xlsx：inspect / create / apply。

pub(crate) mod a1;
mod apply;
mod cf;
mod chart;
mod chartsheet;
mod create;
mod formula;
mod hf;
mod inspect;
mod layout;
mod picture;
mod pivot;
mod request;
mod sheets;
mod style;
mod table;
mod validation;

pub use apply::apply;
pub use create::create;
pub use inspect::inspect;
pub use picture::export_media;
pub use request::{parse_ops, parse_scope};

pub struct ExportedMedia {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
pub const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
pub const REL_CHARTSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet";
pub const REL_DIALOGSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet";
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
pub const REL_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";
pub const REL_CALC_CHAIN: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain";
pub const REL_CORE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const REL_APP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";

pub const CT_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
pub const CT_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
pub const CT_CHARTSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml";
pub const CT_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";
pub const CT_SHARED_STRINGS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
pub const CT_CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
pub const CT_APP: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
