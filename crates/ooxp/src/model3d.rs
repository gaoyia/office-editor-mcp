//! 3D 模型：GLB 文件头与 `am3d:model3d` 默认相机 / 灯光。
//! 不算渲染引擎，不把 GLB 塞进 inspect 树。

use std::path::PathBuf;

use crate::error::{io_path, Error, ErrorKind};
use crate::xml::Element;

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
    let p = PathBuf::from(path);
    let bytes = std::fs::read(&p).map_err(|e| io_path(e, p))?;
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

/// Office 16.0 另存的相机 / 灯光 / trans。不进 inspect。
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
}
