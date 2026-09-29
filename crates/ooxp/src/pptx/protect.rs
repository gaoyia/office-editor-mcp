//! Presentation 的 `p:modifyVerifier`。不写哈希，不破解密码。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

#[derive(Debug, Clone)]
pub struct SetProtectionOp {
    pub enforced: bool,
}

const HASH_ATTRS: &[&str] = &[
    "algorithmName",
    "hashValue",
    "saltValue",
    "spinCount",
    "spinValue",
    "hash",
    "salt",
    "password",
];

pub fn inspect(root: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "hasPassword".into(),
        json!(root
            .child_by_local("modifyVerifier")
            .is_some_and(has_password)),
    );
    Value::Object(obj)
}

fn has_password(el: &Element) -> bool {
    HASH_ATTRS.iter().any(|n| {
        el.attr(n)
            .or_else(|| el.attr_local(n))
            .is_some_and(|s| !s.is_empty())
    })
}

pub fn apply(
    pkg: &mut Package,
    pres_part: &str,
    pres: &mut Document,
    op: &SetProtectionOp,
    i: usize,
) -> Result<(), Error> {
    let some = pres.root.child_by_local("modifyVerifier");
    if some.is_some_and(has_password) {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    }
    if op.enforced {
        return Err(Error::with_path(
            ErrorKind::ProtectionPassword,
            format!("ops[{i}].enforced"),
        ));
    }
    if some.is_none() {
        return Ok(());
    }
    pres.root.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != "modifyVerifier",
        _ => true,
    });
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}
