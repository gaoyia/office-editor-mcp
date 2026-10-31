use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::InsertCommentOp;

pub(crate) fn parse_comments(
    v: Option<&Value>,
    slide_i: usize,
) -> Result<Vec<InsertCommentOp>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].comments");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "comments".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_comment(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}
