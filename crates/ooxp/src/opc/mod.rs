//! OPC 包：ZIP、部件路径、按字节读写、未改部件原样拷回。

mod content_types;
mod names;
mod package;
mod rels;

pub use content_types::{ContentTypes, CONTENT_TYPES_PART};
pub use names::resolve_rel_target;
pub use package::Package;
pub use rels::{office_document_part, Relationship, Relationships};

pub const ROOT_RELS_PART: &str = "/_rels/.rels";
