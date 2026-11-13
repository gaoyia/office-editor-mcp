//! pptx：inspect / create / apply。

mod anim_preset;
mod apply;
mod chart;
mod comments;
pub(crate) mod connector;
mod create;
mod create_xml;
mod custom_show;
mod field;
pub(crate) mod geom;
mod group;
mod handout;
mod hyperlink;
mod ink;
mod inspect;
mod layout_check;
mod list_pin;
mod master;
mod media;
mod model3d;
mod notes;
mod ole;
mod omml;
mod page_bg;
mod picture;
mod protect;
mod request;
mod section;
pub(crate) mod shape;
mod show;
mod slide_paint;
mod slides;
mod smartart;
mod table;
mod theme;
mod theme_map;
mod timing;
mod transition;
mod zoom;

pub use apply::{apply, dry_run};
pub use chart::export_chart_data;
pub use create::create;
pub use inspect::inspect;
pub use layout_check::layout_check;
pub use picture::export_media;
pub use request::{parse_ops, parse_scope};
pub use slide_paint::export_slide_image;

pub struct ExportedMedia {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub const PML_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const DML_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
pub const REL_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
pub const REL_SLIDE_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";
pub const REL_SLIDE_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
pub const REL_PRES_PROPS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps";
pub const REL_CORE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
pub const REL_APP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";

pub const CT_PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
pub const CT_SLIDE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
pub const CT_SLIDE_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";
pub const CT_SLIDE_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
pub const CT_PRES_PROPS: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presProps+xml";
pub const CT_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";
pub const CT_CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";
pub const CT_APP: &str = "application/vnd.openxmlformats-officedocument.extended-properties+xml";
