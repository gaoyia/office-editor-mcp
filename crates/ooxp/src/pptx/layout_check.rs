//! layoutCheck：pptx 只读几何检查。规则见 docs/json-contract.md 的 `## layoutCheck`。
//! 不写文件、不生成图片、不调用 Office，也不进 opc 的写回路径。

use serde_json::{json, Map, Value};

use crate::error::Error;
use crate::opc::{self, Package};
use crate::xml::{self, Element, Node};

use super::shape;

pub fn layout_check(bytes: &[u8]) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let pres_part = opc::office_document_part(&pkg)?;
    let pres = xml::parse(pkg.get(&pres_part)?)?;
    let slides = shape::list_slides(&pkg, &pres_part, &pres.root)?;
    let (slide_cx, slide_cy) = slide_size(&pres.root);

    let mut out = Vec::with_capacity(slides.len());
    for s in &slides {
        let doc = xml::parse(pkg.get(&s.part)?)?;
        let host = format!("slide:{}", s.index);
        let mut entries = Vec::new();
        collect_shapes(&doc.root, &mut Vec::new(), &mut entries);
        let items = build_shapes(&entries, &host, slide_cx, slide_cy);
        out.push(slide_json(&host, &items));
    }
    Ok(json!({
        "schemaVersion": 1,
        "format": "pptx",
        "slideSize": { "cx": slide_cx, "cy": slide_cy },
        "slides": out,
    }))
}

fn slide_size(pres_root: &Element) -> (i64, i64) {
    let Some(sz) = pres_root.child_by_local("sldSz") else {
        return (0, 0);
    };
    (
        sz.attr_local("cx")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0),
        sz.attr_local("cy")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0),
    )
}

/// 一层组合的 `a:xfrm`。四个子元素都按属性逐个补缺省，已有属性保留。
#[derive(Clone, Copy)]
struct GroupXfrm {
    gx: i64,
    gy: i64,
    gcx: i64,
    gcy: i64,
    cx0: i64,
    cy0: i64,
    ccx: i64,
    ccy: i64,
}

impl GroupXfrm {
    /// 缺 `a:xfrm` 时四个元素都缺，补完是 ccx = ccy = 0，该层恒等。
    fn identity() -> Self {
        Self {
            gx: 0,
            gy: 0,
            gcx: 0,
            gcy: 0,
            cx0: 0,
            cy0: 0,
            ccx: 0,
            ccy: 0,
        }
    }
}

fn group_xfrm(grp: &Element) -> GroupXfrm {
    let Some(xfrm) = grp
        .child_by_local("grpSpPr")
        .and_then(|p| p.child_by_local("xfrm"))
    else {
        return GroupXfrm::identity();
    };
    let off = xfrm.child_by_local("off");
    let ext = xfrm.child_by_local("ext");
    let ch_off = xfrm.child_by_local("chOff");
    let ch_ext = xfrm.child_by_local("chExt");
    let gx = attr_or(off, "x", 0);
    let gy = attr_or(off, "y", 0);
    let gcx = attr_or(ext, "cx", 0);
    let gcy = attr_or(ext, "cy", 0);
    let cx0 = attr_or(ch_off, "x", 0);
    let cy0 = attr_or(ch_off, "y", 0);
    // chExt 的两个属性分别抄同一层 ext 补完缺省后的值。
    let ccx = attr_or(ch_ext, "cx", gcx);
    let ccy = attr_or(ch_ext, "cy", gcy);
    GroupXfrm {
        gx,
        gy,
        gcx,
        gcy,
        cx0,
        cy0,
        ccx,
        ccy,
    }
}

fn attr_or(el: Option<&Element>, name: &str, default: i64) -> i64 {
    el.and_then(|e| e.attr(name))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(default)
}

struct ShapeEntry<'a> {
    sp: &'a Element,
    /// 祖先组合链，外到内。换算时从内到外套用。
    chain: Vec<GroupXfrm>,
}

/// 与 `inspect` 的 `shapes` 同集合：只收带 `p:txBody` 的 `p:sp`，沿 `p:grpSp` 递归，
/// 组合本身不产生条目。`mc:AlternateContent` 与 a14 公式框按 pptx 分册第 422 到 428 行。
fn collect_shapes<'a>(el: &'a Element, chain: &mut Vec<GroupXfrm>, out: &mut Vec<ShapeEntry<'a>>) {
    match el.local_name() {
        "AlternateContent" => {
            if crate::drawing_math::is_math_ac(el) {
                return;
            }
            if let Some(i) = shape::first_alt_child_index(el) {
                if let Some(Node::Element(child)) = el.children.get(i) {
                    collect_shapes(child, chain, out);
                }
            }
            return;
        }
        "oleObj" => return,
        "grpSp" => {
            chain.push(group_xfrm(el));
            for child in el.element_children() {
                collect_shapes(child, chain, out);
            }
            chain.pop();
            return;
        }
        _ => {}
    }
    if el.local_name() == "sp" && el.child_by_local("txBody").is_some() {
        out.push(ShapeEntry {
            sp: el,
            chain: chain.clone(),
        });
    }
    for child in el.element_children() {
        collect_shapes(child, chain, out);
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
}

struct ShapeItem {
    id: String,
    name: Option<String>,
    box_: Option<Rect>,
    out_of_bounds: bool,
    text_overflow: bool,
}

fn build_shapes(
    entries: &[ShapeEntry<'_>],
    host: &str,
    slide_cx: i64,
    slide_cy: i64,
) -> Vec<ShapeItem> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (id, _, _) = shape::shape_stable_id(e.sp, host, i + 1);
            let box_ = shape_box(e.sp, &e.chain);
            ShapeItem {
                id,
                name: shape::shape_name(e.sp),
                out_of_bounds: box_.is_some_and(|b| is_out(b, slide_cx, slide_cy)),
                text_overflow: box_.is_some_and(|b| text_overflows(e.sp, b)),
                box_,
            }
        })
        .collect()
}

/// 形状自身 `a:xfrm` 是组内坐标，沿祖先组合链换算到幻灯片 EMU。
/// 中间全部用有理数，最后四舍五入、半值远离零。
fn shape_box(sp: &Element, chain: &[GroupXfrm]) -> Option<Rect> {
    let xfrm = sp.child_by_local("spPr")?.child_by_local("xfrm")?;
    let off = xfrm.child_by_local("off");
    let ext = xfrm.child_by_local("ext");
    let x = attr_or(off, "x", 0);
    let y = attr_or(off, "y", 0);
    let cx = attr_or(ext, "cx", 0);
    let cy = attr_or(ext, "cy", 0);

    // 累积仿射：X_final = ax × X + bx，尺寸只缩放。
    let (mut ax, mut bx) = (Rat::one(), Rat::zero());
    let (mut ay, mut by) = (Rat::one(), Rat::zero());
    for g in chain.iter().rev() {
        if g.ccx == 0 || g.ccy == 0 {
            continue;
        }
        let sx = Rat::new(g.gcx as i128, g.ccx as i128);
        let tx = Rat::new(
            g.gx as i128 * g.ccx as i128 - g.cx0 as i128 * g.gcx as i128,
            g.ccx as i128,
        );
        ax = ax.mul(sx);
        bx = bx.mul(sx).add(tx);

        let sy = Rat::new(g.gcy as i128, g.ccy as i128);
        let ty = Rat::new(
            g.gy as i128 * g.ccy as i128 - g.cy0 as i128 * g.gcy as i128,
            g.ccy as i128,
        );
        ay = ay.mul(sy);
        by = by.mul(sy).add(ty);
    }
    Some(Rect {
        x: ax.mul_int(x).add(bx).round(),
        y: ay.mul_int(y).add(by).round(),
        cx: ax.mul_int(cx).round(),
        cy: ay.mul_int(cy).round(),
    })
}

/// 分母恒正的既约有理数。
#[derive(Clone, Copy)]
struct Rat {
    num: i128,
    den: i128,
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rat {
    fn new(num: i128, den: i128) -> Self {
        if den == 0 {
            return Self { num: 0, den: 1 };
        }
        let (n, d) = if den < 0 { (-num, -den) } else { (num, den) };
        let g = gcd(n.unsigned_abs(), d as u128);
        if g <= 1 {
            Self { num: n, den: d }
        } else {
            Self {
                num: n / g as i128,
                den: d / g as i128,
            }
        }
    }

    fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }

    fn mul(self, other: Self) -> Self {
        Self::new(self.num * other.num, self.den * other.den)
    }

    fn mul_int(self, v: i64) -> Self {
        Self::new(self.num * v as i128, self.den)
    }

    /// 四舍五入到最近整数，半值远离零：-1.5 -> -2，1.5 -> 2。
    fn round(self) -> i64 {
        let q = self.num / self.den;
        let r = self.num % self.den;
        let adjusted = if r.unsigned_abs() * 2 >= self.den as u128 {
            q + if self.num < 0 { -1 } else { 1 }
        } else {
            q
        };
        adjusted.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }
}

/// 四条严格不等式，相等不算。
fn is_out(b: Rect, slide_cx: i64, slide_cy: i64) -> bool {
    let right = b.x as i128 + b.cx as i128;
    let bottom = b.y as i128 + b.cy as i128;
    b.x < 0 || b.y < 0 || right > slide_cx as i128 || bottom > slide_cy as i128
}

fn overlap_list(items: &[ShapeItem]) -> Vec<Value> {
    let mut out = Vec::new();
    for i in 0..items.len() {
        let Some(a) = items[i].box_ else { continue };
        for j in (i + 1)..items.len() {
            let Some(b) = items[j].box_ else { continue };
            let iw =
                (a.x as i128 + a.cx as i128).min(b.x as i128 + b.cx as i128) - a.x.max(b.x) as i128;
            let ih =
                (a.y as i128 + a.cy as i128).min(b.y as i128 + b.cy as i128) - a.y.max(b.y) as i128;
            if iw > 0 && ih > 0 {
                out.push(json!({
                    "a": items[i].id,
                    "b": items[j].id,
                    "area": (iw * ih) as i64,
                }));
            }
        }
    }
    out
}

fn slide_json(host: &str, items: &[ShapeItem]) -> Value {
    let shapes: Vec<Value> = items
        .iter()
        .map(|it| {
            let mut obj = Map::new();
            obj.insert("id".into(), json!(it.id));
            if let Some(name) = &it.name {
                obj.insert("name".into(), json!(name));
            }
            if let Some(b) = it.box_ {
                obj.insert(
                    "box".into(),
                    json!({ "x": b.x, "y": b.y, "cx": b.cx, "cy": b.cy }),
                );
            }
            obj.insert("outOfBounds".into(), json!(it.out_of_bounds));
            Value::Object(obj)
        })
        .collect();
    let out_of_bounds: Vec<&str> = items
        .iter()
        .filter(|it| it.out_of_bounds)
        .map(|it| it.id.as_str())
        .collect();
    let text_overflow: Vec<Value> = items
        .iter()
        .filter(|it| it.text_overflow)
        .map(|it| json!({ "id": it.id, "estimated": true }))
        .collect();
    json!({
        "id": host,
        "shapes": shapes,
        "outOfBounds": out_of_bounds,
        "overlaps": overlap_list(items),
        "textOverflow": text_overflow,
    })
}

const DEFAULT_L_INS: i64 = 91_440;
const DEFAULT_R_INS: i64 = 91_440;
const DEFAULT_T_INS: i64 = 45_720;
const DEFAULT_B_INS: i64 = 45_720;
const DEFAULT_SZ: i64 = 1_800;

/// 纯整数阈值：rows × sz × 762 > h × 5。不读字体度量，不排文字。
fn text_overflows(sp: &Element, b: Rect) -> bool {
    let Some(tx) = sp.child_by_local("txBody") else {
        return false;
    };
    let body_pr = tx.child_by_local("bodyPr");
    let w = b.cx as i128
        - attr_or(body_pr, "lIns", DEFAULT_L_INS) as i128
        - attr_or(body_pr, "rIns", DEFAULT_R_INS) as i128;
    let h = b.cy as i128
        - attr_or(body_pr, "tIns", DEFAULT_T_INS) as i128
        - attr_or(body_pr, "bIns", DEFAULT_B_INS) as i128;

    let mut sz = 0i64;
    let mut fragments: Vec<usize> = Vec::new();
    for p in tx.element_children().filter(|e| e.local_name() == "p") {
        let mut chars = 0usize;
        for child in p.element_children() {
            match child.local_name() {
                "br" => {
                    fragments.push(chars);
                    chars = 0;
                }
                "r" => {
                    for t in child.children_by_local("t") {
                        chars += t.text_content().chars().count();
                    }
                    if let Some(s) = child
                        .child_by_local("rPr")
                        .and_then(|r| r.attr("sz"))
                        .and_then(|s| s.parse::<i64>().ok())
                    {
                        sz = sz.max(s);
                    }
                }
                _ => {}
            }
        }
        fragments.push(chars);
    }
    if fragments.iter().sum::<usize>() == 0 {
        return false;
    }
    let sz = if sz <= 0 { DEFAULT_SZ } else { sz };
    let sz_emu = sz as i128 * 127;
    let n = ((2 * w) / sz_emu).max(1);
    let rows: i128 = fragments
        .iter()
        .map(|c| (((*c as i128) + n - 1) / n).max(1))
        .sum();
    rows * sz as i128 * 762 > h * 5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(name: &str, attrs: &[(&str, &str)], children: Vec<Element>) -> Element {
        let mut e = Element::new(name);
        for (k, v) in attrs {
            e.attrs.push(((*k).to_string(), (*v).to_string()));
        }
        for c in children {
            e.push_element(c);
        }
        e
    }

    fn sp_with(x: &str, y: &str, cx: &str, cy: &str, tx: Option<Element>) -> Element {
        let xfrm = el(
            "xfrm",
            &[],
            vec![
                el("off", &[("x", x), ("y", y)], vec![]),
                el("ext", &[("cx", cx), ("cy", cy)], vec![]),
            ],
        );
        let mut children = vec![el("spPr", &[], vec![xfrm])];
        if let Some(tx) = tx {
            children.push(tx);
        }
        el("sp", &[], children)
    }

    fn item(id: &str, box_: Option<Rect>) -> ShapeItem {
        ShapeItem {
            id: id.to_string(),
            name: None,
            box_,
            out_of_bounds: false,
            text_overflow: false,
        }
    }

    fn rect(x: i64, y: i64, cx: i64, cy: i64) -> Rect {
        Rect { x, y, cx, cy }
    }

    #[test]
    fn group_defaults_are_per_attribute() {
        // <a:off x="914400"/> 得 gx = 914400、gy = 0；<a:chExt cy="100"/> 得 ccx = gcx、ccy = 100。
        let grp = el(
            "grpSp",
            &[],
            vec![el(
                "grpSpPr",
                &[],
                vec![el(
                    "xfrm",
                    &[],
                    vec![
                        el("off", &[("x", "914400")], vec![]),
                        el("ext", &[("cx", "2000"), ("cy", "1000")], vec![]),
                        el("chOff", &[("x", "10")], vec![]),
                        el("chExt", &[("cy", "100")], vec![]),
                    ],
                )],
            )],
        );
        let g = group_xfrm(&grp);
        assert_eq!((g.gx, g.gy), (914_400, 0));
        assert_eq!((g.gcx, g.gcy), (2_000, 1_000));
        assert_eq!((g.cx0, g.cy0), (10, 0));
        assert_eq!((g.ccx, g.ccy), (2_000, 100));
    }

    #[test]
    fn group_chain_scales_and_rounds_half_away_from_zero() {
        // 缩放 1/2：x = 3 -> 1.5 -> 2，x = -3 -> -1.5 -> -2。
        let g = GroupXfrm {
            gx: 0,
            gy: 0,
            gcx: 1,
            gcy: 1,
            cx0: 0,
            cy0: 0,
            ccx: 2,
            ccy: 2,
        };
        let positive = shape_box(&sp_with("3", "3", "4", "4", None), &[g]).unwrap();
        assert_eq!(
            (positive.x, positive.y, positive.cx, positive.cy),
            (2, 2, 2, 2)
        );
        let negative = shape_box(&sp_with("-3", "-3", "4", "4", None), &[g]).unwrap();
        assert_eq!(
            (negative.x, negative.y, negative.cx, negative.cy),
            (-2, -2, 2, 2)
        );
    }

    #[test]
    fn nested_groups_apply_inner_to_outer() {
        let inner = GroupXfrm {
            gx: 100,
            gy: 200,
            gcx: 4,
            gcy: 4,
            cx0: 0,
            cy0: 0,
            ccx: 2,
            ccy: 2,
        };
        let outer = GroupXfrm {
            gx: 1_000,
            gy: 2_000,
            gcx: 3,
            gcy: 3,
            cx0: 0,
            cy0: 0,
            ccx: 1,
            ccy: 1,
        };
        // 内层 x' = 100 + 2x，外层 x'' = 1000 + 3x'，合计 x'' = 1300 + 6x。
        let b = shape_box(&sp_with("10", "0", "2", "2", None), &[outer, inner]).unwrap();
        assert_eq!((b.x, b.y, b.cx, b.cy), (1_360, 2_600, 12, 12));
    }

    #[test]
    fn zero_child_extent_is_identity_layer() {
        let g = GroupXfrm {
            gx: 999,
            gy: 999,
            gcx: 1,
            gcy: 1,
            cx0: 0,
            cy0: 0,
            ccx: 0,
            ccy: 0,
        };
        let b = shape_box(&sp_with("7", "8", "9", "10", None), &[g]).unwrap();
        assert_eq!((b.x, b.y, b.cx, b.cy), (7, 8, 9, 10));
        assert!(shape_box(&el("sp", &[], vec![]), &[]).is_none());
    }

    #[test]
    fn out_of_bounds_uses_strict_inequalities() {
        assert!(!is_out(rect(0, 0, 100, 100), 100, 100));
        assert!(!is_out(rect(50, 50, 50, 50), 100, 100));
        assert!(is_out(rect(-1, 0, 10, 10), 100, 100));
        assert!(is_out(rect(0, -1, 10, 10), 100, 100));
        assert!(is_out(rect(95, 0, 6, 10), 100, 100));
        assert!(is_out(rect(0, 95, 10, 6), 100, 100));
    }

    #[test]
    fn touching_edges_do_not_overlap() {
        let touching = vec![
            item("a", Some(rect(0, 0, 10, 10))),
            item("b", Some(rect(10, 0, 10, 10))),
        ];
        assert!(overlap_list(&touching).is_empty());

        let crossing = vec![
            item("a", Some(rect(0, 0, 10, 10))),
            item("b", Some(rect(5, 5, 10, 10))),
            item("c", None),
        ];
        let list = overlap_list(&crossing);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["a"], "a");
        assert_eq!(list[0]["b"], "b");
        assert_eq!(list[0]["area"], 25);
    }

    fn tx_body(paras: Vec<Element>, body_attrs: &[(&str, &str)]) -> Element {
        let mut children = vec![el("bodyPr", body_attrs, vec![])];
        children.extend(paras);
        el("txBody", &[], children)
    }

    fn para(children: Vec<Element>) -> Element {
        el("p", &[], children)
    }

    fn run(text: &str, sz: &str) -> Element {
        el(
            "r",
            &[],
            vec![
                el("rPr", &[("sz", sz)], vec![]),
                Element::new("t").with_text(text),
            ],
        )
    }

    #[test]
    fn text_overflow_is_pure_integer_comparison() {
        // sz = 1800 -> szEMU = 228600。inset 缺省：左右各 91440，上下各 45720。
        // n=1：w = 300000 - 182880 = 117120，floor(2w/szEMU) = 1，一个字 -> rows = 1。
        // 左侧 1 × 1800 × 762 = 1371600，h × 5 以 274320 为界。
        let one = tx_body(vec![para(vec![run("a", "1800")])], &[]);
        assert!(text_overflows(
            &sp_with("0", "0", "300000", "274320", Some(one.clone())),
            rect(0, 0, 300_000, 274_320)
        ));
        assert!(!text_overflows(
            &sp_with("0", "0", "300000", "374320", Some(one)),
            rect(0, 0, 300_000, 374_320)
        ));

        // n=2：w = 482880 - 182880 = 300000，floor(600000/228600) = 2；三个字 -> rows = 2。
        // 左侧 2 × 1800 × 762 = 2743200，h × 5 以 548640 为界。
        let three = tx_body(vec![para(vec![run("abc", "1800")])], &[]);
        assert!(text_overflows(
            &sp_with("0", "0", "482880", "591440", Some(three.clone())),
            rect(0, 0, 482_880, 591_440)
        ));
        assert!(!text_overflows(
            &sp_with("0", "0", "482880", "691440", Some(three)),
            rect(0, 0, 482_880, 691_440)
        ));
    }

    #[test]
    fn text_overflow_counts_breaks_and_empty_fragments() {
        // 两段各一个字、没有 a:br 是 2 行；带一个 br 的空片段也计 1 行。
        let two_paras = tx_body(
            vec![para(vec![run("a", "1800")]), para(vec![run("b", "1800")])],
            &[],
        );
        let br_para = tx_body(
            vec![para(vec![
                run("a", "1800"),
                el("br", &[], vec![]),
                run("", "1800"),
            ])],
            &[],
        );
        let b = rect(0, 0, 300_000, 274_320);
        assert!(text_overflows(
            &sp_with("0", "0", "300000", "274320", Some(two_paras)),
            b
        ));
        assert!(text_overflows(
            &sp_with("0", "0", "300000", "274320", Some(br_para)),
            b
        ));

        let empty = tx_body(vec![para(vec![])], &[]);
        assert!(!text_overflows(
            &sp_with("0", "0", "300000", "274320", Some(empty)),
            b
        ));
    }
}
