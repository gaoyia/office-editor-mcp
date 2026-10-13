//! docx request：revision。

use serde_json::Value;

use crate::error::Error;

use super::common::*;
use super::types::*;

pub(super) fn parse_revision_op(
    obj: &serde_json::Map<String, Value>,
    i: usize,
) -> Result<RevisionOp, Error> {
    deny_keys(obj, i, &["op", "id"])?;
    let id = require_op_id(obj, i)?;
    let target = super::super::revision::parse_rev_ref(&id, &format!("ops[{i}].id"))?;
    Ok(RevisionOp { id, target })
}
