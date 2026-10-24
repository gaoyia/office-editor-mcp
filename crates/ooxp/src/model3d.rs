//! 3D 模型：GLB 文件头、`am3d:model3d` 缺省 XML，以及相机 / 旋转 / 光源的 inspect 与补丁。
//! 不算渲染引擎，不把 GLB 塞进 inspect 树。
//! 不把 `am3d:ambientLight` / `ptLight` 放进树。


use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::xml::{Element, Node};

/// `[Content_Types].xml` 的 Default `glb`。
pub const CT_GLB: &str = "model/gltf.binary";
/// graphicData / 命名空间。
pub const AM3D_NS: &str = "http://schemas.microsoft.com/office/drawing/2017/model3d";
/// drawing 关系 Type。
pub const REL_MODEL3D: &str = "http://schemas.microsoft.com/office/2017/06/relationships/model3d";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// 缺省 1 英寸见方。
pub const DEFAULT_CX: i64 = 914_400;
pub const DEFAULT_CY: i64 = 914_400;
/// Excel 16.0 另存 914400×457200 时的 viewportSz。
const VIEWPORT_SZ: &str = "699851";

/// 文件头小端 `glTF`（`0x46546C67`），version=2。
pub fn looks_like_glb(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    magic == 0x4654_6C67 && version == 2
}

/// 读本地文件并按文件头认 GLB。不是 GLB 报 `InvalidImageFile`。
pub fn read_glb(path: &str) -> Result<Vec<u8>, Error> {
    let bytes = crate::host_files::read_user_file(path)?;
    if !looks_like_glb(&bytes) {
        return Err(Error::new(ErrorKind::InvalidImageFile));
    }
    Ok(bytes)
}

/// `mc:Choice/@Requires` 是否含 `am3d`。
pub fn choice_requires_am3d(choice: &Element) -> bool {
    let Some(req) = choice.attr_local("Requires") else {
        return false;
    };
    req.split([' ', ',']).any(|t| t == "am3d")
}

/// `a:graphicData/@uri` 是否为 model3d 命名空间。
pub fn is_model3d_graphic(el: &Element) -> bool {
    let mut hit = false;
    walk(el, &mut |e| {
        if hit {
            return;
        }
        if e.local_name() == "graphicData" {
            if let Some(uri) = e.attr_local("uri") {
                if uri.trim_end_matches('/') == AM3D_NS {
                    hit = true;
                }
            }
        }
    });
    hit
}

/// `am3d:model3d/@r:embed`。
pub fn glb_embed_rid(el: &Element) -> Option<String> {
    let mut found = None;
    walk(el, &mut |e| {
        if found.is_some() {
            return;
        }
        if e.local_name() == "model3d" {
            found = e.attr_local("embed").map(str::to_string);
        }
    });
    found
}

/// `am3d:raster/am3d:blip/@r:embed`。
pub fn raster_embed_rid(el: &Element) -> Option<String> {
    let mut found = None;
    walk(el, &mut |e| {
        if found.is_some() {
            return;
        }
        if e.local_name() == "blip" {
            found = e.attr_local("embed").map(str::to_string);
        }
    });
    found
}

/// 收集树上全部 `r:embed`（含 Fallback 海报）。
pub fn collect_embed_rids(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    walk(el, &mut |e| {
        if let Some(rid) = e.attr_local("embed") {
            if !out.iter().any(|x| x == rid) {
                out.push(rid.to_string());
            }
        }
    });
    out
}

pub fn walk<'a>(el: &'a Element, f: &mut impl FnMut(&'a Element)) {
    f(el);
    for c in el.element_children() {
        walk(c, f);
    }
}

/// 子孙里第一个 `am3d:model3d`。
pub fn model3d_in(el: &Element) -> Option<&Element> {
    if el.local_name() == "model3d" {
        return Some(el);
    }
    for c in el.element_children() {
        if let Some(found) = model3d_in(c) {
            return Some(found);
        }
    }
    None
}

/// 子孙里第一个 `am3d:model3d`（可变）。
pub fn model3d_in_mut(el: &mut Element) -> Option<&mut Element> {
    if el.local_name() == "model3d" {
        return Some(el);
    }
    let children = el.children.as_mut_slice();
    for child in children {
        if let Node::Element(e) = child {
            if let Some(found) = model3d_in_mut(e) {
                return Some(found);
            }
        }
    }
    None
}

/// inspect：`camera` / `rot` / `lightRig`。宿主可以是 graphicFrame 或 `am3d:model3d`。
pub fn inspect_view(host: &Element) -> (Value, Value, Value) {
    let m = model3d_in(host);
    (inspect_camera(m), inspect_rot(m), inspect_light_rig(m))
}

fn inspect_camera(model: Option<&Element>) -> Value {
    let cam = model.and_then(|m| m.child_by_local("camera"));
    let pos = cam.and_then(|c| c.child_by_local("pos"));
    let up = cam.and_then(|c| c.child_by_local("up"));
    let look = cam.and_then(|c| c.child_by_local("lookAt"));
    let mut m = Map::new();
    m.insert("pos".into(), xyz_json(pos, "x", "y", "z"));
    m.insert("up".into(), xyz_json(up, "dx", "dy", "dz"));
    m.insert("lookAt".into(), xyz_json(look, "x", "y", "z"));
    m.insert("fov".into(), fov_json(cam));
    Value::Object(m)
}

fn inspect_rot(model: Option<&Element>) -> Value {
    let rot = model
        .and_then(|m| m.child_by_local("trans"))
        .and_then(|t| t.child_by_local("rot"))
        .or_else(|| model.and_then(|m| m.child_by_local("rot")));
    let mut m = Map::new();
    m.insert("ax".into(), int_attr(rot, "ax"));
    m.insert("ay".into(), int_attr(rot, "ay"));
    m.insert("az".into(), int_attr(rot, "az"));
    Value::Object(m)
}

fn inspect_light_rig(model: Option<&Element>) -> Value {
    let Some(rig) = model
        .and_then(|m| m.child_by_local("spPr"))
        .and_then(|sp| sp.child_by_local("scene3d"))
        .and_then(|sc| sc.child_by_local("lightRig"))
    else {
        return Value::Null;
    };
    let rot = rig.child_by_local("rot");
    let mut m = Map::new();
    m.insert("rig".into(), str_attr(rig, "rig"));
    m.insert("dir".into(), str_attr(rig, "dir"));
    m.insert("lat".into(), int_attr(rot, "lat"));
    m.insert("lon".into(), int_attr(rot, "lon"));
    m.insert("rev".into(), int_attr(rot, "rev"));
    Value::Object(m)
}

fn xyz_json(el: Option<&Element>, a: &str, b: &str, c: &str) -> Value {
    let mut m = Map::new();
    m.insert(a.into(), int_attr(el, a));
    m.insert(b.into(), int_attr(el, b));
    m.insert(c.into(), int_attr(el, c));
    Value::Object(m)
}

fn int_attr(el: Option<&Element>, name: &str) -> Value {
    match el
        .and_then(|e| e.attr_local(name))
        .and_then(|s| s.parse::<i64>().ok())
    {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn str_attr(el: &Element, name: &str) -> Value {
    match el.attr_local(name) {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

fn fov_json(cam: Option<&Element>) -> Value {
    let Some(p) = cam.and_then(|c| c.child_by_local("perspective")) else {
        return Value::Null;
    };
    match p.attr_local("fov").and_then(|s| s.parse::<i64>().ok()) {
        Some(n) if n >= 0 => json!(n),
        _ => Value::Null,
    }
}

#[derive(Debug, Clone)]
pub struct AxisPatch {
    pub a: Option<i64>,
    pub b: Option<i64>,
    pub c: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CameraFieldsPatch {
    pub pos: Option<AxisPatch>,
    pub up: Option<AxisPatch>,
    pub look_at: Option<AxisPatch>,
    pub fov: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RotPatch {
    pub ax: Option<i64>,
    pub ay: Option<i64>,
    pub az: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct LightRigPatch {
    pub rig: Option<String>,
    pub dir: Option<String>,
    pub lat: Option<i64>,
    pub lon: Option<i64>,
    pub rev: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Model3dCameraPatch {
    pub camera: Option<CameraFieldsPatch>,
    pub rot: Option<RotPatch>,
    pub light_rig: Option<Option<LightRigPatch>>,
}

/// 解析 `setModel3dCamera` 的 `camera` / `rot` / `lightRig`。调用方先 `deny_keys` 顶层。
pub fn parse_camera_patch(obj: &Map<String, Value>, i: usize) -> Result<Model3dCameraPatch, Error> {
    let camera = parse_camera_fields(obj.get("camera"), i)?;
    let rot = parse_rot_fields(obj.get("rot"), i)?;
    let light_rig = parse_light_rig_fields(obj.get("lightRig"), i)?;
    if camera.is_none() && rot.is_none() && light_rig.is_none() {
        return Err(Error::with_path(
            ErrorKind::NeedSetModel3dCameraField,
            format!("ops[{i}]"),
        ));
    }
    Ok(Model3dCameraPatch {
        camera,
        rot,
        light_rig,
    })
}

fn parse_camera_fields(v: Option<&Value>, i: usize) -> Result<Option<CameraFieldsPatch>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(obj)) => {
            deny_keys_path(
                obj,
                &format!("ops[{i}].camera"),
                &["pos", "up", "lookAt", "fov"],
            )?;
            let fov = parse_opt_i64(obj.get("fov"), "fov", &format!("ops[{i}].camera.fov"))?;
            if let Some(n) = fov {
                if n < 0 {
                    return Err(Error::with_path(
                        ErrorKind::MinValue {
                            field: "fov".into(),
                            min: 0,
                        },
                        format!("ops[{i}].camera.fov"),
                    ));
                }
            }
            Ok(Some(CameraFieldsPatch {
                pos: parse_axes(
                    obj.get("pos"),
                    "pos",
                    &format!("ops[{i}].camera.pos"),
                    "x",
                    "y",
                    "z",
                )?,
                up: parse_axes(
                    obj.get("up"),
                    "up",
                    &format!("ops[{i}].camera.up"),
                    "dx",
                    "dy",
                    "dz",
                )?,
                look_at: parse_axes(
                    obj.get("lookAt"),
                    "lookAt",
                    &format!("ops[{i}].camera.lookAt"),
                    "x",
                    "y",
                    "z",
                )?,
                fov,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "camera".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].camera"),
        )),
    }
}

fn parse_rot_fields(v: Option<&Value>, i: usize) -> Result<Option<RotPatch>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(obj)) => {
            deny_keys_path(obj, &format!("ops[{i}].rot"), &["ax", "ay", "az"])?;
            Ok(Some(RotPatch {
                ax: parse_opt_i64(obj.get("ax"), "ax", &format!("ops[{i}].rot.ax"))?,
                ay: parse_opt_i64(obj.get("ay"), "ay", &format!("ops[{i}].rot.ay"))?,
                az: parse_opt_i64(obj.get("az"), "az", &format!("ops[{i}].rot.az"))?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "rot".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].rot"),
        )),
    }
}

fn parse_light_rig_fields(
    v: Option<&Value>,
    i: usize,
) -> Result<Option<Option<LightRigPatch>>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Object(obj)) => {
            deny_keys_path(
                obj,
                &format!("ops[{i}].lightRig"),
                &["rig", "dir", "lat", "lon", "rev"],
            )?;
            Ok(Some(Some(LightRigPatch {
                rig: parse_opt_str(obj.get("rig"), "rig", &format!("ops[{i}].lightRig.rig"))?,
                dir: parse_opt_str(obj.get("dir"), "dir", &format!("ops[{i}].lightRig.dir"))?,
                lat: parse_opt_i64(obj.get("lat"), "lat", &format!("ops[{i}].lightRig.lat"))?,
                lon: parse_opt_i64(obj.get("lon"), "lon", &format!("ops[{i}].lightRig.lon"))?,
                rev: parse_opt_i64(obj.get("rev"), "rev", &format!("ops[{i}].lightRig.rev"))?,
            })))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "lightRig".into(),
                expected: TypeName::Object,
            },
            format!("ops[{i}].lightRig"),
        )),
    }
}

fn parse_axes(
    v: Option<&Value>,
    field: &str,
    path: &str,
    a: &str,
    b: &str,
    c: &str,
) -> Result<Option<AxisPatch>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Object(obj)) => {
            deny_keys_path(obj, path, &[a, b, c])?;
            Ok(Some(AxisPatch {
                a: parse_opt_i64(obj.get(a), a, &format!("{path}.{a}"))?,
                b: parse_opt_i64(obj.get(b), b, &format!("{path}.{b}"))?,
                c: parse_opt_i64(obj.get(c), c, &format!("{path}.{c}"))?,
            }))
        }
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Object,
            },
            path.to_string(),
        )),
    }
}

fn parse_opt_i64(v: Option<&Value>, field: &str, path: &str) -> Result<Option<i64>, Error> {
    match v {
        None => Ok(None),
        Some(Value::Number(n)) => n.as_i64().map(Some).ok_or_else(|| {
            Error::with_path(
                ErrorKind::WrongType {
                    field: field.into(),
                    expected: TypeName::Integer,
                },
                path.to_string(),
            )
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::Integer,
            },
            path.to_string(),
        )),
    }
}

fn parse_opt_str(v: Option<&Value>, field: &str, path: &str) -> Result<Option<String>, Error> {
    match v {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: field.into(),
                expected: TypeName::String,
            },
            path.to_string(),
        )),
    }
}

fn deny_keys_path(obj: &Map<String, Value>, path: &str, allowed: &[&str]) -> Result<(), Error> {
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::with_path(
                ErrorKind::UnknownField { field: k.clone() },
                format!("{path}.{k}"),
            ));
        }
    }
    Ok(())
}

/// 把补丁写进 `am3d:model3d`。
pub fn apply_camera_patch(model: &mut Element, patch: &Model3dCameraPatch) -> Result<(), Error> {
    if let Some(cam) = &patch.camera {
        apply_camera_fields(model, cam);
    }
    if let Some(rot) = &patch.rot {
        apply_rot_fields(model, rot);
    }
    if let Some(lr) = &patch.light_rig {
        apply_light_rig(model, lr.as_ref());
    }
    Ok(())
}

fn apply_camera_fields(model: &mut Element, cam: &CameraFieldsPatch) {
    let camera = ensure_child(model, "camera", "am3d:camera", Some("spPr"));
    if let Some(pos) = &cam.pos {
        let el = ensure_child(camera, "pos", "am3d:pos", None);
        set_named(el, &[("x", pos.a), ("y", pos.b), ("z", pos.c)]);
    }
    if let Some(up) = &cam.up {
        let el = ensure_child(camera, "up", "am3d:up", Some("pos"));
        set_named(el, &[("dx", up.a), ("dy", up.b), ("dz", up.c)]);
    }
    if let Some(look) = &cam.look_at {
        let el = ensure_child(camera, "lookAt", "am3d:lookAt", Some("up"));
        set_named(el, &[("x", look.a), ("y", look.b), ("z", look.c)]);
    }
    if let Some(fov) = cam.fov {
        let el = ensure_child(camera, "perspective", "am3d:perspective", Some("lookAt"));
        el.set_attr("fov", fov.to_string());
    }
}

fn apply_rot_fields(model: &mut Element, rot: &RotPatch) {
    let trans = ensure_child(model, "trans", "am3d:trans", Some("camera"));
    let el = ensure_child(trans, "rot", "am3d:rot", None);
    set_named(el, &[("ax", rot.ax), ("ay", rot.ay), ("az", rot.az)]);
}

fn apply_light_rig(model: &mut Element, patch: Option<&LightRigPatch>) {
    let Some(p) = patch else {
        if let Some(sp) = model.child_by_local_mut("spPr") {
            remove_children_local(sp, "scene3d");
        }
        return;
    };
    let sp = ensure_child(model, "spPr", "am3d:spPr", None);
    let had_scene = sp.child_by_local("scene3d").is_some();
    let scene = ensure_child(sp, "scene3d", "a:scene3d", None);
    if !had_scene {
        let cam = ensure_child(scene, "camera", "a:camera", None);
        if cam.attr_local("prst").is_none() {
            cam.set_attr("prst", "orthographicFront");
        }
    }
    let had_rig = scene.child_by_local("lightRig").is_some();
    let rig = ensure_child(scene, "lightRig", "a:lightRig", Some("camera"));
    if !had_rig {
        rig.set_attr("rig", p.rig.as_deref().unwrap_or("freezing"));
        rig.set_attr("dir", p.dir.as_deref().unwrap_or("t"));
        let rot = ensure_child(rig, "rot", "a:rot", None);
        rot.set_attr("lat", p.lat.unwrap_or(0).to_string());
        rot.set_attr("lon", p.lon.unwrap_or(0).to_string());
        rot.set_attr("rev", p.rev.unwrap_or(0).to_string());
        return;
    }
    if let Some(s) = &p.rig {
        rig.set_attr("rig", s.clone());
    }
    if let Some(s) = &p.dir {
        rig.set_attr("dir", s.clone());
    }
    if p.lat.is_some() || p.lon.is_some() || p.rev.is_some() {
        let rot = ensure_child(rig, "rot", "a:rot", None);
        set_named(rot, &[("lat", p.lat), ("lon", p.lon), ("rev", p.rev)]);
    }
}

fn ensure_child<'a>(
    parent: &'a mut Element,
    local: &str,
    qname: &str,
    after: Option<&str>,
) -> &'a mut Element {
    if parent.child_by_local(local).is_some() {
        return parent.child_by_local_mut(local).unwrap();
    }
    let el = Element::new(qname);
    if let Some(after) = after {
        if let Some(i) = parent.children.iter().position(|n| match n {
            Node::Element(e) => e.local_name() == after,
            Node::Text(_) => false,
        }) {
            parent.children.insert(i + 1, Node::Element(el));
            return parent.child_by_local_mut(local).unwrap();
        }
    }
    parent.push_element(el);
    parent.child_by_local_mut(local).unwrap()
}

fn set_named(el: &mut Element, pairs: &[(&str, Option<i64>)]) {
    for (name, val) in pairs {
        if let Some(n) = val {
            el.set_attr(name, n.to_string());
        }
    }
}

fn remove_children_local(el: &mut Element, local: &str) {
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        Node::Text(_) => true,
    });
}

/// Office 16.0 另存的相机 / 灯光 / trans。inspect 给出 `camera` / `rot`；缺省 `lightRig` 为 `null`。
pub fn build_model3d_el(glb_rid: &str, raster_rid: &str, cx: i64, cy: i64) -> Element {
    let mut m = Element::new("am3d:model3d")
        .with_attr("xmlns:am3d", AM3D_NS)
        .with_attr("xmlns:r", REL_NS)
        .with_attr("r:embed", glb_rid);
    let mut sp = Element::new("am3d:spPr");
    let mut xfrm = Element::new("a:xfrm");
    xfrm.push_element(
        Element::new("a:off")
            .with_attr("x", "0")
            .with_attr("y", "0"),
    );
    xfrm.push_element(
        Element::new("a:ext")
            .with_attr("cx", cx.to_string())
            .with_attr("cy", cy.to_string()),
    );
    sp.push_element(xfrm);
    let mut geom = Element::new("a:prstGeom").with_attr("prst", "rect");
    geom.push_element(Element::new("a:avLst"));
    sp.push_element(geom);
    m.push_element(sp);

    let mut cam = Element::new("am3d:camera");
    cam.push_element(
        Element::new("am3d:pos")
            .with_attr("x", "0")
            .with_attr("y", "0")
            .with_attr("z", "66519324"),
    );
    cam.push_element(
        Element::new("am3d:up")
            .with_attr("dx", "0")
            .with_attr("dy", "36000000")
            .with_attr("dz", "0"),
    );
    cam.push_element(
        Element::new("am3d:lookAt")
            .with_attr("x", "0")
            .with_attr("y", "0")
            .with_attr("z", "0"),
    );
    cam.push_element(Element::new("am3d:perspective").with_attr("fov", "2700000"));
    m.push_element(cam);

    let mut trans = Element::new("am3d:trans");
    trans.push_element(
        Element::new("am3d:meterPerModelUnit")
            .with_attr("n", "1000000")
            .with_attr("d", "1000000"),
    );
    trans.push_element(
        Element::new("am3d:preTrans")
            .with_attr("dx", "-18000000")
            .with_attr("dy", "-18000000")
            .with_attr("dz", "0"),
    );
    let mut scale = Element::new("am3d:scale");
    scale.push_element(
        Element::new("am3d:sx")
            .with_attr("n", "1000000")
            .with_attr("d", "1000000"),
    );
    scale.push_element(
        Element::new("am3d:sy")
            .with_attr("n", "1000000")
            .with_attr("d", "1000000"),
    );
    scale.push_element(
        Element::new("am3d:sz")
            .with_attr("n", "1000000")
            .with_attr("d", "1000000"),
    );
    trans.push_element(scale);
    trans.push_element(Element::new("am3d:rot"));
    trans.push_element(
        Element::new("am3d:postTrans")
            .with_attr("dx", "0")
            .with_attr("dy", "0")
            .with_attr("dz", "0"),
    );
    m.push_element(trans);

    let mut raster = Element::new("am3d:raster")
        .with_attr("rName", "Office3DRenderer")
        .with_attr("rVer", "16.0.8326");
    raster.push_element(Element::new("am3d:blip").with_attr("r:embed", raster_rid));
    m.push_element(raster);
    m.push_element(Element::new("am3d:objViewport").with_attr("viewportSz", VIEWPORT_SZ));

    let mut ambient = Element::new("am3d:ambientLight");
    let mut clr = Element::new("am3d:clr");
    clr.push_element(
        Element::new("a:scrgbClr")
            .with_attr("r", "50000")
            .with_attr("g", "50000")
            .with_attr("b", "50000"),
    );
    ambient.push_element(clr);
    ambient.push_element(
        Element::new("am3d:illuminance")
            .with_attr("n", "500000")
            .with_attr("d", "1000000"),
    );
    m.push_element(ambient);

    m.push_element(pt_light(
        "100000", "75000", "50000", "9765625", "21959998", "70920001", "16344003",
    ));
    m.push_element(pt_light(
        "40000",
        "60000",
        "95000",
        "12250000",
        "-37964106",
        "51130435",
        "57631972",
    ));
    m.push_element(pt_light(
        "86837",
        "72700",
        "100000",
        "3125000",
        "-37739122",
        "58056624",
        "-34769649",
    ));
    m
}

fn pt_light(r: &str, g: &str, b: &str, intensity: &str, x: &str, y: &str, z: &str) -> Element {
    let mut light = Element::new("am3d:ptLight").with_attr("rad", "0");
    let mut clr = Element::new("am3d:clr");
    clr.push_element(
        Element::new("a:scrgbClr")
            .with_attr("r", r)
            .with_attr("g", g)
            .with_attr("b", b),
    );
    light.push_element(clr);
    light.push_element(
        Element::new("am3d:intensity")
            .with_attr("n", intensity)
            .with_attr("d", "1000000"),
    );
    light.push_element(
        Element::new("am3d:pos")
            .with_attr("x", x)
            .with_attr("y", y)
            .with_attr("z", z),
    );
    light
}

/// 库测试与手测用的最小合法 GLB（一个三角形）。
pub fn sample_glb() -> Vec<u8> {
    let mut json = br#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","max":[1.0,1.0,0.0],"min":[0.0,0.0,0.0]}],"bufferViews":[{"buffer":0,"byteLength":36}],"buffers":[{"byteLength":36}]}"#.to_vec();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = Vec::with_capacity(36);
    for v in [0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bin.extend_from_slice(&v.to_le_bytes());
    }
    let json_len = json.len() as u32;
    let bin_len = bin.len() as u32;
    let total = 12 + 8 + json_len + 8 + bin_len;
    let mut out = Vec::with_capacity(total as usize);
    out.extend_from_slice(&0x4654_6C67u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&total.to_le_bytes());
    out.extend_from_slice(&json_len.to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&bin_len.to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_glb_header() {
        let b = sample_glb();
        assert!(looks_like_glb(&b));
        assert!(!looks_like_glb(b"glTF"));
        assert!(!looks_like_glb(&[0x89, 0x50, 0x4E, 0x47]));
    }

    #[test]
    fn inspect_default_camera_rot_light() {
        let m = build_model3d_el("rId1", "rId2", 914400, 914400);
        let (cam, rot, lr) = inspect_view(&m);
        assert_eq!(cam["pos"]["x"], 0);
        assert_eq!(cam["pos"]["z"], 66519324);
        assert_eq!(cam["up"]["dy"], 36000000);
        assert_eq!(cam["fov"], 2700000);
        assert!(rot["ax"].is_null());
        assert!(rot["ay"].is_null());
        assert!(rot["az"].is_null());
        assert!(lr.is_null());
    }

    #[test]
    fn apply_camera_rot_and_light_rig() {
        let mut m = build_model3d_el("rId1", "rId2", 914400, 914400);
        apply_camera_patch(
            &mut m,
            &Model3dCameraPatch {
                camera: Some(CameraFieldsPatch {
                    pos: Some(AxisPatch {
                        a: Some(50_400_000),
                        b: None,
                        c: None,
                    }),
                    up: None,
                    look_at: None,
                    fov: Some(1_892_375),
                }),
                rot: Some(RotPatch {
                    ax: Some(1_060_962),
                    ay: Some(-532_862),
                    az: Some(-169_045),
                }),
                light_rig: Some(Some(LightRigPatch {
                    rig: Some("freezing".into()),
                    dir: Some("t".into()),
                    lat: None,
                    lon: None,
                    rev: Some(18_000_000),
                })),
            },
        )
        .unwrap();
        let (cam, rot, lr) = inspect_view(&m);
        assert_eq!(cam["pos"]["x"], 50_400_000);
        assert_eq!(cam["pos"]["z"], 66519324);
        assert_eq!(cam["fov"], 1_892_375);
        assert_eq!(rot["ax"], 1_060_962);
        assert_eq!(rot["ay"], -532_862);
        assert_eq!(rot["az"], -169_045);
        assert_eq!(lr["rig"], "freezing");
        assert_eq!(lr["dir"], "t");
        assert_eq!(lr["lat"], 0);
        assert_eq!(lr["rev"], 18_000_000);

        apply_camera_patch(
            &mut m,
            &Model3dCameraPatch {
                camera: None,
                rot: None,
                light_rig: Some(None),
            },
        )
        .unwrap();
        let (_, _, lr2) = inspect_view(&m);
        assert!(lr2.is_null());
    }

    #[test]
    fn parse_camera_patch_needs_field_and_rejects_neg_fov() {
        use crate::error::ErrorCode;
        let empty = Map::new();
        let err = parse_camera_patch(&empty, 0).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        assert!(err.to_string().contains("camera"));
        let mut obj = Map::new();
        obj.insert("camera".into(), json!({ "fov": -1 }));
        let err = parse_camera_patch(&obj, 0).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        assert!(err.to_string().contains("fov"));
    }
}
