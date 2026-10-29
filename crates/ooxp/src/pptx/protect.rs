//! Presentation 的 `p:modifyVerifier`。可写 PowerPoint 16.0 的 SHA-512 哈希，不破解密码。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::password::sha512_spin;
use crate::xml::{Document, Element, Node};

#[derive(Debug, Clone)]
pub struct SetProtectionOp {
    pub enforced: bool,
    pub password: Option<String>,
}

struct Sha512Spec {
    salt: Vec<u8>,
    hash: Vec<u8>,
    spin: u32,
}

const SPIN_COUNT: u32 = 100_000;

const HASH_ATTRS: &[&str] = &[
    "algorithmName",
    "hashValue",
    "saltValue",
    "spinCount",
    "spinValue",
    "hash",
    "salt",
    "password",
    "hashData",
    "saltData",
    "cryptProviderType",
    "cryptAlgorithmClass",
    "cryptAlgorithmType",
    "cryptAlgorithmSid",
    "cryptSpinCount",
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
    let hashed = pres
        .root
        .child_by_local("modifyVerifier")
        .is_some_and(has_password);
    if hashed {
        gate_existing_hash(
            pres.root.child_by_local("modifyVerifier"),
            op.password.as_deref(),
            i,
        )?;
    }
    if !op.enforced {
        if pres.root.child_by_local("modifyVerifier").is_none() {
            return Ok(());
        }
        pres.root.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "modifyVerifier",
            _ => true,
        });
        pkg.set(pres_part, pres.to_bytes()?)?;
        return Ok(());
    }
    if op.password.is_none()
        && pres
            .root
            .child_by_local("modifyVerifier")
            .is_some_and(|e| !has_password(e))
    {
        return Ok(());
    }
    let el = ensure_verifier(&mut pres.root);
    if let Some(pw) = &op.password {
        write_sha512(el, pw);
    } else {
        strip_hash_attrs(el);
    }
    pkg.set(pres_part, pres.to_bytes()?)?;
    Ok(())
}

fn gate_existing_hash(el: Option<&Element>, password: Option<&str>, i: usize) -> Result<(), Error> {
    let Some(el) = el.filter(|e| has_password(e)) else {
        return Ok(());
    };
    let Some(pw) = password else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    let Some(spec) = sha512_spec(el) else {
        return Err(Error::with_path(
            ErrorKind::ProtectionHasPassword,
            format!("ops[{i}]"),
        ));
    };
    if sha512_spin(pw, &spec.salt, spec.spin) != spec.hash {
        return Err(Error::with_path(
            ErrorKind::ProtectionPasswordWrong,
            format!("ops[{i}].password"),
        ));
    }
    Ok(())
}

fn sha512_spec(el: &Element) -> Option<Sha512Spec> {
    let sid = el.attr_local("cryptAlgorithmSid")?;
    if sid != "14" {
        return None;
    }
    let hash = STANDARD.decode(el.attr_local("hashData")?).ok()?;
    let salt = STANDARD.decode(el.attr_local("saltData")?).ok()?;
    let spin: u32 = el.attr_local("spinCount")?.parse().ok()?;
    if hash.is_empty() || salt.is_empty() || spin == 0 || spin > crate::password::MAX_CHECK_SPIN {
        return None;
    }
    Some(Sha512Spec { salt, hash, spin })
}

fn write_sha512(el: &mut Element, password: &str) {
    strip_hash_attrs(el);
    let mut salt = [0u8; 16];
    crate::host_env::fill_random(&mut salt);
    let hash = sha512_spin(password, &salt, SPIN_COUNT);
    el.set_attr("cryptProviderType", "rsaAES");
    el.set_attr("cryptAlgorithmClass", "hash");
    el.set_attr("cryptAlgorithmType", "typeAny");
    el.set_attr("cryptAlgorithmSid", "14");
    el.set_attr("spinCount", SPIN_COUNT.to_string());
    el.set_attr("saltData", STANDARD.encode(salt));
    el.set_attr("hashData", STANDARD.encode(&hash));
}

fn strip_hash_attrs(el: &mut Element) {
    el.attrs.retain(|(k, _)| {
        let local = k.rsplit(':').next().unwrap_or(k);
        !HASH_ATTRS.contains(&local)
    });
}

fn ensure_verifier(root: &mut Element) -> &mut Element {
    if root.child_by_local("modifyVerifier").is_some() {
        return root
            .child_by_local_mut("modifyVerifier")
            .expect("刚确认有 modifyVerifier");
    }
    let at = insert_index(root);
    root.children
        .insert(at, Node::Element(Element::new("p:modifyVerifier")));
    root.child_by_local_mut("modifyVerifier")
        .expect("刚插入 modifyVerifier")
}

fn insert_index(root: &Element) -> usize {
    const AFTER: &[&str] = &[
        "sldMasterIdLst",
        "notesMasterIdLst",
        "handoutMasterIdLst",
        "sldIdLst",
        "sldSz",
        "notesSz",
        "smartTags",
        "embeddedFontLst",
        "custShowLst",
        "photoAlbum",
        "custDataLst",
        "kinsoku",
        "defaultTextStyle",
    ];
    let mut last_after = None;
    let mut first_ext = None;
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else {
            continue;
        };
        let name = e.local_name();
        if AFTER.contains(&name) {
            last_after = Some(i);
        }
        if first_ext.is_none() && name == "extLst" {
            first_ext = Some(i);
        }
    }
    if let Some(i) = last_after {
        return i + 1;
    }
    if let Some(i) = first_ext {
        return i;
    }
    root.children.len()
}

#[cfg(test)]
#[test]
fn huge_spin_is_not_a_hash_spec() {
    let el = Element::new("p:modifyVerifier")
        .with_attr("cryptAlgorithmSid", "14")
        .with_attr("hashData", STANDARD.encode([1u8; 64]))
        .with_attr("saltData", STANDARD.encode([2u8; 16]))
        .with_attr("spinCount", "10000001");
    assert!(sha512_spec(&el).is_none());
    let err = gate_existing_hash(Some(&el), Some("secret"), 0).unwrap_err();
    assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    let ok = Element::new("p:modifyVerifier")
        .with_attr("cryptAlgorithmSid", "14")
        .with_attr("hashData", STANDARD.encode([1u8; 64]))
        .with_attr("saltData", STANDARD.encode([2u8; 16]))
        .with_attr("spinCount", "100000");
    assert!(sha512_spec(&ok).is_some());
}
