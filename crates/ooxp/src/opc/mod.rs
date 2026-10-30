//! OPC 包：ZIP、部件路径、按字节读写、未改部件原样拷回。

mod content_types;
mod names;
mod package;
mod rels;

pub use content_types::{ContentTypes, CONTENT_TYPES_PART};
pub use names::{
    rel_target, rels_part_name, resolve_rel_target, same_part_name, source_part_of_rels,
    split_dir_file,
};
pub use package::Package;
pub use rels::{
    office_document_part, part_still_referenced, reject_strict, resolve_id, Relationship,
    Relationships,
};

pub const ROOT_RELS_PART: &str = "/_rels/.rels";

#[cfg(test)]
mod identity_save;
