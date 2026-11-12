//! xlsx：inspect / create / apply。

pub(crate) mod a1;
mod apply;
mod background;
mod cell;
mod cf;
mod chart;
mod chart_drawing;
mod chartsheet;
mod comments;
mod connection;
mod create;
mod create_xml;
mod custom_view;
mod dbcs;
mod draw_pin;
mod eval;
mod extlink;
mod formula;
mod group;
mod hf;
mod hyperlink;
mod ink;
mod inspect;
mod layout;
mod list_pin;
mod model3d;
mod names;
mod ole;
mod omml;
mod outline;
mod picture;
mod pivot;
mod print;
mod protect;
mod request;
mod shape;
mod sheets;
mod slicer;
mod smartart;
mod sparkline;
mod style;
mod table;
mod theme;
mod validation;
mod view;

pub(crate) use apply::apply_in;
pub use apply::{apply, dry_run};
pub use create::create;
pub(crate) use create::create_in;
#[allow(unused_imports)]
pub use create::create_with_workbook_ct;
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
pub const REL_MACROSHEET: &str =
    "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet";
pub const REL_INTL_MACROSHEET: &str =
    "http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet";
pub const REL_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
pub const REL_SHARED_STRINGS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
pub const REL_CALC_CHAIN: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain";
pub const REL_CORE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const REL_APP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";

pub const CT_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
pub const CT_WORKBOOK_MACRO: &str = "application/vnd.ms-excel.sheet.macroEnabled.main+xml";
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
