use crate::error::{Error, ErrorKind};
use crate::xml::Element;

use super::*;


pub(crate) fn theme_accents(theme_xml: Option<&[u8]>) -> [[u8; 3]; 6] {
    let mut accents = FALLBACK_ACCENTS;
    let Some(bytes) = theme_xml else {
        return accents;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return accents;
    };
    let Some(scheme) = find_deep(&doc.root, "clrScheme") else {
        return accents;
    };
    for (i, name) in [
        "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(color) = scheme
            .child_by_local(name)
            .and_then(|slot| color_of_slot(slot, &accents))
        {
            accents[i] = color;
        }
    }
    accents
}

/// Caller-supplied live system colors, keyed by DrawingML `sysClr/@val`.
/// Official Chart.Export paints the live named system color and ignores
/// `lastClr`. The library never queries ambient OS colors and never uses
/// `lastClr` as the current frame/plot fill.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SystemColorPalette {
    pub(super) entries: Vec<(String, [u8; 3])>,
}

impl SystemColorPalette {
    pub(crate) fn insert(&mut self, val: impl Into<String>, rgb: [u8; 3]) -> &mut Self {
        let val = val.into();
        if let Some((_, color)) = self.entries.iter_mut().find(|(name, _)| *name == val) {
            *color = rgb;
        } else {
            self.entries.push((val, rgb));
        }
        self
    }

    pub(crate) fn get(&self, val: &str) -> Option<[u8; 3]> {
        self.entries
            .iter()
            .find(|(name, _)| name == val)
            .map(|(_, color)| *color)
    }
}

#[derive(Clone)]
pub(super) struct ThemeScheme {
    pub(super) accents: [[u8; 3]; 6],
    pub(super) dk1: Option<[u8; 3]>,
    pub(super) lt1: Option<[u8; 3]>,
    pub(super) dk2: Option<[u8; 3]>,
    pub(super) lt2: Option<[u8; 3]>,
    pub(super) hlink: Option<[u8; 3]>,
    pub(super) fol_hlink: Option<[u8; 3]>,
    pub(super) sys: SystemColorPalette,
}

pub(super) fn theme_slot_rgb(slot: &Element, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    if let Some(sys) = slot.child_by_local("sysClr") {
        // Theme clrScheme records the slot in lastClr. Chart-frame/plot
        // sysClr fills are a different path: Excel paints the live system
        // color named by @val and ignores lastClr. Those fills resolve
        // only from an explicit caller palette; otherwise they stay NI.
        return parse_hex(sys.attr_local("lastClr")?);
    }
    color_of_slot(slot, accents)
}

pub(super) fn theme_scheme(theme_xml: Option<&[u8]>, accents: [[u8; 3]; 6]) -> ThemeScheme {
    let mut scheme = ThemeScheme {
        accents,
        dk1: None,
        lt1: None,
        dk2: None,
        lt2: None,
        hlink: None,
        fol_hlink: None,
        sys: SystemColorPalette::default(),
    };
    let Some(bytes) = theme_xml else {
        return scheme;
    };
    let Ok(doc) = crate::xml::parse(bytes) else {
        return scheme;
    };
    let Some(slots) = find_deep(&doc.root, "clrScheme") else {
        return scheme;
    };
    let color = |name: &str| {
        slots
            .child_by_local(name)
            .and_then(|slot| theme_slot_rgb(slot, &accents))
    };
    scheme.dk1 = color("dk1");
    scheme.lt1 = color("lt1");
    scheme.dk2 = color("dk2");
    scheme.lt2 = color("lt2");
    scheme.hlink = color("hlink");
    scheme.fol_hlink = color("folHlink");
    scheme
}

pub(super) fn scheme_slot(name: &str, scheme: &ThemeScheme) -> Option<[u8; 3]> {
    Some(match name {
        "accent1" => scheme.accents[0],
        "accent2" => scheme.accents[1],
        "accent3" => scheme.accents[2],
        "accent4" => scheme.accents[3],
        "accent5" => scheme.accents[4],
        "accent6" => scheme.accents[5],
        "dk1" | "tx1" => scheme.dk1?,
        "lt1" | "bg1" => scheme.lt1?,
        "dk2" | "tx2" => scheme.dk2?,
        "lt2" | "bg2" => scheme.lt2?,
        "hlink" => scheme.hlink?,
        "folHlink" => scheme.fol_hlink?,
        _ => return None,
    })
}

/// IEC 61966-2-1 sRGB. Excel chart-frame tint/shade interpolate in this space,
/// not in gamma-encoded bytes and not with a pure 2.2 gamma.
pub(super) fn srgb_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn linear_to_srgb(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let s = if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}


/// Official Chart.Export t187 48-case srgbClr matrix, probe (5,5):
/// a:comp is the encoded mix-to-gray complement (max+min)-channel.
/// Grays stay put; FF0000 becomes 00FFFF; C04020 becomes 20A0C0.
/// This is not linear IEC invert (a:inv) and not 255-c on mid-tones.
pub(super) fn frame_comp(rgb: [u8; 3]) -> [u8; 3] {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    let sum = u16::from(max) + u16::from(min);
    rgb.map(|channel| (sum - u16::from(channel)) as u8)
}

/// Official: a:inv is IEC 61966-2-1 linear invert, then encode.
/// linear_to_srgb(1 - srgb_to_linear(c)). Not encoded 255-c, not a pure 2.2 gamma.
pub(super) fn frame_inv(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|channel| linear_to_srgb(1.0 - srgb_to_linear(channel)))
}

/// Official: a:gamma is the IEC sRGB encode, treating the stored byte as linear 0..1.
/// linear_to_srgb(c/255). Not identity, not decode, not a pure 2.2 gamma.
pub(super) fn frame_gamma(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|channel| linear_to_srgb(f64::from(channel) / 255.0))
}

/// Official: a:invGamma is the IEC sRGB decode scaled back to a byte.
/// round(srgb_to_linear(c) * 255). Not encode, not identity, not a pure 2.2 gamma.
pub(super) fn frame_inv_gamma(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|channel| (srgb_to_linear(channel) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Official: a:gray is Rec.709 / sRGB luma on encoded bytes, then a gray triple.
/// Y = round(0.2126 R + 0.7152 G + 0.0722 B). Not linear-IEC luma, not Rec.601,
/// and not the 54/183/19 integer shift, which missed official FF0000 and C04020 by one.
pub(super) fn frame_gray(rgb: [u8; 3]) -> [u8; 3] {
    let y = (0.2126 * f64::from(rgb[0]) + 0.7152 * f64::from(rgb[1]) + 0.0722 * f64::from(rgb[2]))
        .round()
        .clamp(0.0, 255.0) as u8;
    [y, y, y]
}

fn standalone_empty_transform(node: &Element, name: &str) -> bool {
    node.local_name() == name
        && node
            .attrs
            .iter()
            .all(|(attr, _)| attr.as_str() == "xmlns" || attr.starts_with("xmlns:"))
        && node.element_children().next().is_none()
        && node.text_content().trim().is_empty()
}


/// Official t189 pair matrix: apply already-locked empty srgbClr
/// transforms in XML document order. Reverse order was rejected by 17
/// order-discriminating official (5,5) pixels. schemeClr / sysClr stay
/// unsupported. Length 3+ was not in that matrix and stays unsupported.
pub(super) fn apply_locked_srgb_transforms(mut rgb: [u8; 3], names: &[&str]) -> Option<[u8; 3]> {
    if names.is_empty()
        || !names
            .iter()
            .all(|name| matches!(*name, "comp" | "inv" | "gamma" | "invGamma" | "gray"))
    {
        return None;
    }
    for name in names {
        rgb = match *name {
            "comp" => frame_comp(rgb),
            "inv" => frame_inv(rgb),
            "gamma" => frame_gamma(rgb),
            "invGamma" => frame_inv_gamma(rgb),
            "gray" => frame_gray(rgb),
            _ => return None,
        };
    }
    Some(rgb)
}

fn locked_srgb_transform_pair(color: &Element, kids: &[&Element], names: &[&str]) -> bool {
    color.local_name() == "srgbClr"
        && names.len() == 2
        && names
            .iter()
            .all(|name| matches!(*name, "comp" | "inv" | "gamma" | "invGamma" | "gray"))
        && kids
            .iter()
            .zip(names.iter())
            .all(|(kid, name)| standalone_empty_transform(kid, name))
}

pub(super) fn frame_tint(rgb: [u8; 3], val: u32) -> [u8; 3] {
    let share = f64::from(val) / 100_000.0;
    rgb.map(|channel| {
        let lin = srgb_to_linear(channel);
        linear_to_srgb(lin * share + (1.0 - share))
    })
}

pub(super) fn frame_shade(rgb: [u8; 3], val: u32) -> [u8; 3] {
    let share = f64::from(val) / 100_000.0;
    rgb.map(|channel| linear_to_srgb(srgb_to_linear(channel) * share))
}

pub(super) fn frame_lum(rgb: [u8; 3], lum_mod: Option<u32>, lum_off: Option<i32>) -> [u8; 3] {
    let (hue, sat, light) = rgb_hsl(rgb);
    let mut light = light;
    if let Some(factor) = lum_mod {
        light *= f64::from(factor) / 100_000.0;
    }
    if let Some(offset) = lum_off {
        light += f64::from(offset) / 100_000.0;
    }
    hsl_rgb(hue, sat, light.clamp(0.0, 1.0))
}

/// Integer-thousandth spelling of DrawingML ST_Percentage. Keep malformed
/// nodes and other lexical forms unsupported rather than dropping semantics.
pub(super) fn parse_signed_percentage(node: &Element) -> Option<i32> {
    if node.attrs.len() != 1 || node.element_children().next().is_some() {
        return None;
    }
    let raw = node.attr_local("val")?;
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

/// Preserve the existing bounded luminance-offset contract.
pub(super) fn parse_luminance_offset(node: &Element) -> Option<i32> {
    parse_signed_percentage(node).filter(|value| (-100_000..=100_000).contains(value))
}

/// HSL saturation modulation with hue and luminance held fixed. In encoded
/// RGB this scales each channel's distance from (min + max) / 2. Do not clamp
/// saturation first: signed / >100% factors can invert / oversaturate, with
/// clipping only on conversion back to RGB. Existing luminance math is untouched.
pub(super) fn frame_sat(rgb: [u8; 3], factor: i32) -> [u8; 3] {
    let mid = (f64::from(*rgb.iter().min().unwrap()) + f64::from(*rgb.iter().max().unwrap())) / 2.0;
    rgb.map(|channel| {
        (mid + (f64::from(channel) - mid) * f64::from(factor) / 100_000.0)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

/// DrawingML satOff adds percentage points to HSL saturation while keeping
/// hue and luminance. Only the unambiguous in-gamut standalone subset is
/// enabled: native extrapolation and achromatic hue selection are unverified.
/// Do not infer clipping / inversion from the different satMod transform.
pub(super) fn frame_sat_offset(rgb: [u8; 3], offset: i32) -> Option<[u8; 3]> {
    if offset == 0 {
        return Some(rgb);
    }
    let (hue, sat, light) = rgb_hsl(rgb);
    let shifted = sat + f64::from(offset) / 100_000.0;
    if sat == 0.0 || !(0.0..=1.0).contains(&shifted) {
        return None;
    }
    Some(hsl_rgb(hue, shifted, light))
}

/// Absolute DrawingML saturation replaces S, retaining H and L. Limit this
/// standalone profile to the documented typical range; do not guess native
/// extrapolation or assign an arbitrary hue to a previously gray color.
pub(super) fn frame_saturation(rgb: [u8; 3], value: i32) -> Option<[u8; 3]> {
    if !(0..=100_000).contains(&value) {
        return None;
    }
    let (hue, sat, light) = rgb_hsl(rgb);
    if sat == 0.0 {
        return (value == 0).then_some(rgb);
    }
    Some(hsl_rgb(hue, f64::from(value) / 100_000.0, light))
}

/// Absolute DrawingML luminance replaces HSL L, retaining H and S. The
/// typical 0..100% subset is explicit; out-of-range native behavior is not
/// inferred. Achromatic input is well-defined because S remains zero.
pub(super) fn frame_luminance(rgb: [u8; 3], value: i32) -> Option<[u8; 3]> {
    if !(0..=100_000).contains(&value) {
        return None;
    }
    let (hue, sat, _) = rgb_hsl(rgb);
    Some(hsl_rgb(hue, sat, f64::from(value) / 100_000.0))
}

/// Absolute DrawingML hue uses ST_PositiveFixedAngle, not a percentage or
/// offset: replace H and preserve S/L. The schema domain is [0, 360 degrees)
/// with 60000 units per degree; reject invalid turns instead of guessing wrap.
pub(super) fn frame_absolute_hue(rgb: [u8; 3], angle: i32) -> Option<[u8; 3]> {
    if !(0..21_600_000).contains(&angle) {
        return None;
    }
    let (_, sat, light) = rgb_hsl(rgb);
    Some(hsl_rgb(f64::from(angle) / 21_600_000.0, sat, light))
}

/// Exactly two alphaMod operations, with the ceiling applied at EACH stage.
/// First opacity is min(first,100000)/100000 exactly; reuse the unchanged
/// precise product helper for the second stage without intermediate rounding.
/// Saturation makes the factor order significant. This is not a general
/// chain evaluator or a claim of combined native fidelity.
pub(super) fn frame_alpha_mod_pair(rgb: [u8; 3], first: &Element, second: &Element) -> Option<FrameFill> {
    let parse = |node: &Element| {
        let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
        if node.local_name() != "alphaMod" || attrs.len() != 1 || attrs[0].0 != "val"
            || node.element_children().next().is_some() || !node.text_content().trim().is_empty() { return None; }
        u32::try_from(parse_frame_integer(&attrs[0].1)?).ok()
    };
    let initial = parse(first)?.min(100_000);
    let factor = parse(second)?;
    Some(frame_alpha_mod_fill(rgb, initial, factor))
}

/// Exactly one alphaMod and alphaOff, evaluated in their XML order.
/// ECMA20.1.2.3.2/.3 specifies the bounds of EACH operation; postponing
/// saturation until the end or reordering the operations changes the result.
/// Preserve numerator/1e10 through the existing exact-alpha final conversion.
/// This is an isolated sRGB library profile, not a generic chain evaluator or
/// a claim of combined native fidelity. Both strict operands are always read.
pub(super) fn frame_alpha_mod_off(rgb: [u8; 3], modulation: &Element, offset: &Element, modulation_first: bool) -> Option<FrameFill> {
    let parse = |node: &Element| {
        let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
        if attrs.len() != 1 || attrs[0].0 != "val" || node.element_children().next().is_some() || !node.text_content().trim().is_empty() { return None; }
        parse_frame_integer(&attrs[0].1)
    };
    if modulation.local_name() != "alphaMod" || offset.local_name() != "alphaOff" { return None; }
    let factor = u64::from(u32::try_from(parse(modulation)?).ok()?);
    let off = parse(offset)?;
    if !(-100_000..=100_000).contains(&off) { return None; }
    let product = if modulation_first {
        // Starting opacity is100%; cap modulation BEFORE applying the offset.
        ((100_000 * factor).min(ALPHA_PRODUCT_DENOM) as i64 + i64::from(off) * 100_000)
            .clamp(0, ALPHA_PRODUCT_DENOM as i64) as u64
    } else {
        // Clamp the offset stage first, then keep the exact modulation product.
        let initial = (100_000 + i64::from(off)).clamp(0, 100_000) as u64;
        (initial * factor).min(ALPHA_PRODUCT_DENOM)
    };
    Some(if product == ALPHA_PRODUCT_DENOM { FrameFill::Solid(rgb) }
        else if product % 100_000 == 0 { FrameFill::Translucent(rgb, (product / 100_000) as u32) }
        else { FrameFill::AlphaProduct(rgb, product) })
}

/// Isolated component replacement plus absolute opacity for the Excel frame profile.
/// ECMA20.1.2.3.4/.10/.23 changes one RGB component;20.1.2.3.1 sets
/// opacity without changing color. Apply these independent fields in either
/// order only for this two-operation sRGB profile, not arbitrary chains.
/// Reuse the independently established setter transfer and exact opacity;
/// combined Excel export/color fidelity remains a separate evidence boundary.
/// Validate both operands even when opacity is zero.
pub(super) fn frame_rgb_component_alpha(rgb: [u8; 3], component: &Element, alpha: &Element) -> Option<FrameFill> {
    let attrs: Vec<_> = alpha.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if alpha.local_name() != "alpha" || attrs.len() != 1 || attrs[0].0 != "val"
        || alpha.element_children().next().is_some()
        || !alpha.text_content().trim().is_empty()
    { return None; }
    let opacity = u32::try_from(parse_frame_integer(&attrs[0].1)?).ok()?;
    if opacity > 100_000 { return None; }
    let rgb = frame_rgb_component(rgb, component)?;
    Some(if opacity == 100_000 { FrameFill::Solid(rgb) } else { FrameFill::Translucent(rgb, opacity) })
}

/// Isolated absolute-alpha plus one RGB adjustment, in either evidenced order.
/// ECMA20.1.2.3.1 sets opacity without changing color; the RGB operation changes
/// one color component. Eighteen new paired native controls validate BOTH orders
/// in this Excel sRGB-only profile; this does not reorder arbitrary transforms.
/// Validate both operands even at zero opacity; preserve existing exact-alpha
/// helpers/products and all other bases/mixed sequences unchanged.
pub(super) fn frame_rgb_adjustment_alpha(rgb: [u8; 3], adjustment: &Element, alpha: &Element) -> Option<FrameFill> {
    let attrs: Vec<_> = alpha.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if alpha.local_name() != "alpha" || attrs.len() != 1 || attrs[0].0 != "val"
        || alpha.element_children().next().is_some()
        || !alpha.text_content().trim().is_empty()
    { return None; }
    let opacity = u32::try_from(parse_frame_integer(&attrs[0].1)?).ok()?;
    if opacity > 100_000 { return None; }
    let rgb = frame_rgb_adjustment(rgb, adjustment)?;
    Some(if opacity == 100_000 { FrameFill::Solid(rgb) } else { FrameFill::Translucent(rgb, opacity) })
}

/// Excel-only standalone RGB-channel modulation/offset, normal-domain subset.
/// ECMA20.1.2.3.5/.6/.11/.12/.24/.25 specify component multiplication/addition
/// and result bounds0..100%. Its encoded-byte examples differ from our new
/// controlled Excel sRGB results; retain that explicit contract divergence.
/// Independent native controls (not setter inference) support IEC linear-light
/// arithmetic on sRGB bases. Scheme-source controls remain unresolved and NI.
pub(super) fn frame_rgb_adjustment(mut rgb: [u8; 3], node: &Element) -> Option<[u8; 3]> {
    let (channel, offset) = match node.local_name() {
        "redMod" => (0, false), "greenMod" => (1, false), "blueMod" => (2, false),
        "redOff" => (0, true), "greenOff" => (1, true), "blueOff" => (2, true),
        _ => return None,
    };
    let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if attrs.len() != 1 || attrs[0].0 != "val"
        || node.element_children().next().is_some()
        || !node.text_content().trim().is_empty()
    { return None; }
    let value = parse_frame_integer(&attrs[0].1)?;
    if !(if offset { -100_000..=100_000 } else { 0..=100_000 }).contains(&value) {
        return None;
    }
    let linear = srgb_to_linear(rgb[channel]);
    let amount = f64::from(value) / 100_000.0;
    // Existing exact IEC encoder applies the normatively specified result bounds.
    rgb[channel] = linear_to_srgb(if offset { linear + amount } else { linear * amount });
    Some(rgb)
}

/// Standalone normal-domain red/green/blue component replacement for the
/// Excel frame/Cartesian-plot profile. ECMA-376 20.1.2.3.4/.10/.23 leaves the
/// other two components unchanged. The predeclared 48-control native matrix
/// discriminates linear-IEC component intensity from encoded-byte percentage.
/// Reuse the exact IEC encoder, not a guessed gamma or sample compensation.
/// Broader ST_Percentage ranges/spellings and transform mixtures remain NI.
pub(super) fn frame_rgb_component(mut rgb: [u8; 3], node: &Element) -> Option<[u8; 3]> {
    let channel = match node.local_name() {
        "red" => 0,
        "green" => 1,
        "blue" => 2,
        _ => return None,
    };
    let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if attrs.len() != 1
        || attrs[0].0 != "val"
        || node.element_children().next().is_some()
        || !node.text_content().trim().is_empty()
    {
        return None;
    }
    rgb[channel] = linear_srgb_channel(parse_frame_integer(&attrs[0].1)?)?;
    Some(rgb)
}

/// Encode a normal-domain linear sRGB/scRGB component using the IEC61966-2-1
/// transfer function published by ICC (sRGB characterization,A.8), not a
/// guessed power2.2. Quantize nearest to eight bits; do not extrapolate/clamp.
pub(super) fn linear_srgb_channel(value: i32) -> Option<u8> {
    if !(0..=100_000).contains(&value) {
        return None;
    }
    let linear = f64::from(value) / 100_000.0;
    let encoded = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    Some((encoded * 255.0).round() as u8)
}

/// Untransformed CT_ScRgbColor normal-domain subset. ECMA-376 Part1(2016),
///20.1.2.3.30 specifies linear gamma1.0 and0..100%, with50% gray=BCBCBC.
/// ST_Percentage permits broader spellings/ranges, but no such extrapolation
/// or transforms are enabled here. Preserve all old color/transform paths.
pub(super) fn parse_frame_scrgb_color(node: &Element) -> Option<[u8; 3]> {
    let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if node.local_name() != "scrgbClr"
        || attrs.len() != 3
        || attrs.iter().any(|(name, _)| !matches!(name.as_str(), "r" | "g" | "b"))
        || node.element_children().next().is_some()
        || !node.text_content().trim().is_empty()
    {
        return None;
    }
    Some([
        linear_srgb_channel(parse_frame_integer(node.attr_local("r")?)?)?,
        linear_srgb_channel(parse_frame_integer(node.attr_local("g")?)?)?,
        linear_srgb_channel(parse_frame_integer(node.attr_local("b")?)?)?,
    ])
}

/// Exact named RGB values from ECMA-376 Part1 (2016),20.1.10.48.
/// This is not a CSS palette or case/abbreviation normalization. In particular
/// darkSeaGreen/dkSeaGreen and lightGoldenrodYellow/ltGoldenrodYellow have
/// distinct entries in that normative table. No Office-version corrections.
pub(super) fn drawingml_preset_rgb(value: &str) -> Option<[u8; 3]> {
    let rgb = match value {
        "aliceBlue" => [240, 248, 255],
        "antiqueWhite" => [250, 235, 215],
        "aqua" => [0, 255, 255],
        "aquamarine" => [127, 255, 212],
        "azure" => [240, 255, 255],
        "beige" => [245, 245, 220],
        "bisque" => [255, 228, 196],
        "black" => [0, 0, 0],
        "blanchedAlmond" => [255, 235, 205],
        "blue" => [0, 0, 255],
        "blueViolet" => [138, 43, 226],
        "brown" => [165, 42, 42],
        "burlyWood" => [222, 184, 135],
        "cadetBlue" => [95, 158, 160],
        "chartreuse" => [127, 255, 0],
        "chocolate" => [210, 105, 30],
        "coral" => [255, 127, 80],
        "cornflowerBlue" => [100, 149, 237],
        "cornsilk" => [255, 248, 220],
        "crimson" => [220, 20, 60],
        "cyan" => [0, 255, 255],
        "darkBlue" => [0, 0, 139],
        "darkCyan" => [0, 139, 139],
        "darkGoldenrod" => [184, 134, 11],
        "darkGray" => [169, 169, 169],
        "darkGreen" => [0, 100, 0],
        "darkGrey" => [169, 169, 169],
        "darkKhaki" => [189, 183, 107],
        "darkMagenta" => [139, 0, 139],
        "darkOliveGreen" => [85, 107, 47],
        "darkOrange" => [255, 140, 0],
        "darkOrchid" => [153, 50, 204],
        "darkRed" => [139, 0, 0],
        "darkSalmon" => [233, 150, 122],
        "darkSeaGreen" => [143, 188, 143],
        "darkSlateBlue" => [72, 61, 139],
        "darkSlateGray" => [47, 79, 79],
        "darkSlateGrey" => [47, 79, 79],
        "darkTurquoise" => [0, 206, 209],
        "darkViolet" => [148, 0, 211],
        "deepPink" => [255, 20, 147],
        "deepSkyBlue" => [0, 191, 255],
        "dimGray" => [105, 105, 105],
        "dimGrey" => [105, 105, 105],
        "dkBlue" => [0, 0, 139],
        "dkCyan" => [0, 139, 139],
        "dkGoldenrod" => [184, 134, 11],
        "dkGray" => [169, 169, 169],
        "dkGreen" => [0, 100, 0],
        "dkGrey" => [169, 169, 169],
        "dkKhaki" => [189, 183, 107],
        "dkMagenta" => [139, 0, 139],
        "dkOliveGreen" => [85, 107, 47],
        "dkOrange" => [255, 140, 0],
        "dkOrchid" => [153, 50, 204],
        "dkRed" => [139, 0, 0],
        "dkSalmon" => [233, 150, 122],
        "dkSeaGreen" => [143, 188, 139],
        "dkSlateBlue" => [72, 61, 139],
        "dkSlateGray" => [47, 79, 79],
        "dkSlateGrey" => [47, 79, 79],
        "dkTurquoise" => [0, 206, 209],
        "dkViolet" => [148, 0, 211],
        "dodgerBlue" => [30, 144, 255],
        "firebrick" => [178, 34, 34],
        "floralWhite" => [255, 250, 240],
        "forestGreen" => [34, 139, 34],
        "fuchsia" => [255, 0, 255],
        "gainsboro" => [220, 220, 220],
        "ghostWhite" => [248, 248, 255],
        "gold" => [255, 215, 0],
        "goldenrod" => [218, 165, 32],
        "gray" => [128, 128, 128],
        "green" => [0, 128, 0],
        "greenYellow" => [173, 255, 47],
        "grey" => [128, 128, 128],
        "honeydew" => [240, 255, 240],
        "hotPink" => [255, 105, 180],
        "indianRed" => [205, 92, 92],
        "indigo" => [75, 0, 130],
        "ivory" => [255, 255, 240],
        "khaki" => [240, 230, 140],
        "lavender" => [230, 230, 250],
        "lavenderBlush" => [255, 240, 245],
        "lawnGreen" => [124, 252, 0],
        "lemonChiffon" => [255, 250, 205],
        "lightBlue" => [173, 216, 230],
        "lightCoral" => [240, 128, 128],
        "lightCyan" => [224, 255, 255],
        "lightGoldenrodYellow" => [250, 250, 210],
        "lightGray" => [211, 211, 211],
        "lightGreen" => [144, 238, 144],
        "lightGrey" => [211, 211, 211],
        "lightPink" => [255, 182, 193],
        "lightSalmon" => [255, 160, 122],
        "lightSeaGreen" => [32, 178, 170],
        "lightSkyBlue" => [135, 206, 250],
        "lightSlateGray" => [119, 136, 153],
        "lightSlateGrey" => [119, 136, 153],
        "lightSteelBlue" => [176, 196, 222],
        "lightYellow" => [255, 255, 224],
        "lime" => [0, 255, 0],
        "limeGreen" => [50, 205, 50],
        "linen" => [250, 240, 230],
        "ltBlue" => [173, 216, 230],
        "ltCoral" => [240, 128, 128],
        "ltCyan" => [224, 255, 255],
        "ltGoldenrodYellow" => [250, 250, 120],
        "ltGray" => [211, 211, 211],
        "ltGreen" => [144, 238, 144],
        "ltGrey" => [211, 211, 211],
        "ltPink" => [255, 182, 193],
        "ltSalmon" => [255, 160, 122],
        "ltSeaGreen" => [32, 178, 170],
        "ltSkyBlue" => [135, 206, 250],
        "ltSlateGray" => [119, 136, 153],
        "ltSlateGrey" => [119, 136, 153],
        "ltSteelBlue" => [176, 196, 222],
        "ltYellow" => [255, 255, 224],
        "magenta" => [255, 0, 255],
        "maroon" => [128, 0, 0],
        "medAquamarine" => [102, 205, 170],
        "medBlue" => [0, 0, 205],
        "mediumAquamarine" => [102, 205, 170],
        "mediumBlue" => [0, 0, 205],
        "mediumOrchid" => [186, 85, 211],
        "mediumPurple" => [147, 112, 219],
        "mediumSeaGreen" => [60, 179, 113],
        "mediumSlateBlue" => [123, 104, 238],
        "mediumSpringGreen" => [0, 250, 154],
        "mediumTurquoise" => [72, 209, 204],
        "mediumVioletRed" => [199, 21, 133],
        "medOrchid" => [186, 85, 211],
        "medPurple" => [147, 112, 219],
        "medSeaGreen" => [60, 179, 113],
        "medSlateBlue" => [123, 104, 238],
        "medSpringGreen" => [0, 250, 154],
        "medTurquoise" => [72, 209, 204],
        "medVioletRed" => [199, 21, 133],
        "midnightBlue" => [25, 25, 112],
        "mintCream" => [245, 255, 250],
        "mistyRose" => [255, 228, 225],
        "moccasin" => [255, 228, 181],
        "navajoWhite" => [255, 222, 173],
        "navy" => [0, 0, 128],
        "oldLace" => [253, 245, 230],
        "olive" => [128, 128, 0],
        "oliveDrab" => [107, 142, 35],
        "orange" => [255, 165, 0],
        "orangeRed" => [255, 69, 0],
        "orchid" => [218, 112, 214],
        "paleGoldenrod" => [238, 232, 170],
        "paleGreen" => [152, 251, 152],
        "paleTurquoise" => [175, 238, 238],
        "paleVioletRed" => [219, 112, 147],
        "papayaWhip" => [255, 239, 213],
        "peachPuff" => [255, 218, 185],
        "peru" => [205, 133, 63],
        "pink" => [255, 192, 203],
        "plum" => [221, 160, 221],
        "powderBlue" => [176, 224, 230],
        "purple" => [128, 0, 128],
        "red" => [255, 0, 0],
        "rosyBrown" => [188, 143, 143],
        "royalBlue" => [65, 105, 225],
        "saddleBrown" => [139, 69, 19],
        "salmon" => [250, 128, 114],
        "sandyBrown" => [244, 164, 96],
        "seaGreen" => [46, 139, 87],
        "seaShell" => [255, 245, 238],
        "sienna" => [160, 82, 45],
        "silver" => [192, 192, 192],
        "skyBlue" => [135, 206, 235],
        "slateBlue" => [106, 90, 205],
        "slateGray" => [112, 128, 144],
        "slateGrey" => [112, 128, 144],
        "snow" => [255, 250, 250],
        "springGreen" => [0, 255, 127],
        "steelBlue" => [70, 130, 180],
        "tan" => [210, 180, 140],
        "teal" => [0, 128, 128],
        "thistle" => [216, 191, 216],
        "tomato" => [255, 99, 71],
        "turquoise" => [64, 224, 208],
        "violet" => [238, 130, 238],
        "wheat" => [245, 222, 179],
        "white" => [255, 255, 255],
        "whiteSmoke" => [245, 245, 245],
        "yellow" => [255, 255, 0],
        "yellowGreen" => [154, 205, 50],
        _ => return None,
    };
    Some(rgb)
}

/// Strict untransformed CT_PresetColor subset. Other schema-legal transform
/// children remain NI until verified for this base; namespace declarations
/// are not semantic attributes. Keep this isolated from old transform paths.
pub(super) fn parse_frame_preset_color(node: &Element) -> Option<[u8; 3]> {
    let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if node.local_name() != "prstClr"
        || attrs.len() != 1
        || attrs[0].0 != "val"
        || node.element_children().next().is_some()
        || !node.text_content().trim().is_empty()
    {
        return None;
    }
    drawingml_preset_rgb(&attrs[0].1)
}

/// Strict integer spelling shared by the new HSL base-color attributes.
/// Keep existing transform parsers untouched; percent strings and other
/// otherwise legal OOXML spellings are outside this explicit profile.
pub(super) fn parse_frame_integer(raw: &str) -> Option<i32> {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

/// Untransformed DrawingML HSL base color. Use the existing perceptual HSL
/// conversion in its normal domain, not a guessed extrapolation or extra
/// gamma stage. Transforms on this new base remain explicitly unsupported.
pub(super) fn parse_frame_hsl_color(node: &Element) -> Option<[u8; 3]> {
    let attrs: Vec<_> = node.attrs.iter().filter(|(name, _)| name.as_str() != "xmlns" && !name.starts_with("xmlns:")).collect();
    if node.local_name() != "hslClr"
        || attrs.len() != 3
        || attrs.iter().any(|(name, _)| !matches!(name.as_str(), "hue" | "sat" | "lum"))
        || node.element_children().next().is_some()
        || !node.text_content().trim().is_empty()
    {
        return None;
    }
    let hue = parse_frame_integer(node.attr_local("hue")?)?;
    let sat = parse_frame_integer(node.attr_local("sat")?)?;
    let lum = parse_frame_integer(node.attr_local("lum")?)?;
    if !(0..21_600_000).contains(&hue)
        || !(0..=100_000).contains(&sat)
        || !(0..=100_000).contains(&lum)
    {
        return None;
    }
    Some(hsl_rgb(f64::from(hue) / 21_600_000.0, f64::from(sat) / 100_000.0, f64::from(lum) / 100_000.0))
}

/// DrawingML CT_PositivePercentage uses ST_Percentage (i32) with minimum 0.
/// It is not alpha's bounded 0..100000 percentage. Keep the same strict node
/// and integer-thousandth lexical contract as the other new color transforms.
pub(super) fn parse_positive_percentage(node: &Element) -> Option<u32> {
    u32::try_from(parse_signed_percentage(node)?).ok()
}

/// Modulate the angular HSL hue, retaining saturation and lightness. Reduce
/// complete turns before the existing RGB conversion; clamping at one turn
/// would turn every overshoot into red and contradict native hueMod behavior.
pub(super) fn frame_hue(rgb: [u8; 3], factor: u32) -> [u8; 3] {
    let (hue, sat, light) = rgb_hsl(rgb);
    hsl_rgb((hue * f64::from(factor) / 100_000.0).rem_euclid(1.0), sat, light)
}

/// Excel frame rendering uses a signed ST_Angle offset (60000 per degree).
/// Native controls discriminate periodic hue from the clamp wording in the
/// SDK HueOffset.Val remarks. This is an Excel-only frame profile, not a claim
/// that the two contracts agree or that other hosts have been validated.
pub(super) fn frame_hue_offset(rgb: [u8; 3], offset: i32) -> [u8; 3] {
    let (hue, sat, light) = rgb_hsl(rgb);
    hsl_rgb((hue + f64::from(offset) / (360.0 * 60_000.0)).rem_euclid(1.0), sat, light)
}

/// An alpha offset adds signed percentage points, not a multiplicative ratio.
/// Only integer-thousandth operands are admitted here; exact alphaMod products
/// and unverified transform mixtures continue through their existing paths.
pub(super) fn frame_alpha_off_fill(rgb: [u8; 3], initial: u32, offset: i32) -> FrameFill {
    let alpha = (i64::from(initial) + i64::from(offset)).clamp(0, 100_000) as u32;
    if alpha == 100_000 { FrameFill::Solid(rgb) } else { FrameFill::Translucent(rgb, alpha) }
}

pub(super) fn frame_alpha_mod_fill(rgb: [u8; 3], initial: u32, factor: u32) -> FrameFill {
    let product = (u64::from(initial) * u64::from(factor)).min(ALPHA_PRODUCT_DENOM);
    if product == ALPHA_PRODUCT_DENOM {
        FrameFill::Solid(rgb)
    } else if product % 100_000 == 0 {
        FrameFill::Translucent(rgb, (product / 100_000) as u32)
    } else {
        FrameFill::AlphaProduct(rgb, product)
    }
}

pub(super) fn alpha_product_to_u8(product: u64) -> u8 {
    ((product * 255 + ALPHA_PRODUCT_DENOM / 2) / ALPHA_PRODUCT_DENOM) as u8
}

pub(super) fn frame_initial_alpha(node: &Element) -> Option<u32> {
    parse_positive_percentage(node).filter(|alpha| *alpha <= 100_000)
}

pub(super) fn frame_alpha_fill(rgb: [u8; 3], alpha: &Element) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let alpha = parse_ooxml_alpha(alpha.attr_local("val")).ok_or_else(unsupported)?;
    if alpha == 100_000 {
        Ok(FrameFill::Solid(rgb))
    } else {
        Ok(FrameFill::Translucent(rgb, alpha))
    }
}

pub(super) fn resolve_frame_solid(color: &Element, scheme: &ThemeScheme) -> Result<FrameFill, Error> {
    let unsupported = || Error::with_path(ErrorKind::ChartImageUnsupported, "id");
    let rgb = match color.local_name() {
        "scrgbClr" => return parse_frame_scrgb_color(color).map(FrameFill::Solid).ok_or_else(unsupported),
        "prstClr" => return parse_frame_preset_color(color).map(FrameFill::Solid).ok_or_else(unsupported),
        // Return before generic transform dispatch: this base is supported
        // only without transforms. Excel frame/plot entry guards are unchanged.
        "hslClr" => return parse_frame_hsl_color(color).map(FrameFill::Solid).ok_or_else(unsupported),
        "srgbClr" => color
            .attr_local("val")
            .and_then(parse_hex)
            .ok_or_else(unsupported)?,
        "schemeClr" => {
            let name = color.attr_local("val").ok_or_else(unsupported)?;
            scheme_slot(name, scheme).ok_or_else(unsupported)?
        }
        "sysClr" => {
            // Official window lastClr=00FF00 exported white; windowText
            // lastClr=FF0000 exported black. lastClr is not current.
            // Transforms on sysClr were not in the official matrix.
            let name = color.attr_local("val").ok_or_else(unsupported)?;
            let rgb = scheme.sys.get(name).ok_or_else(unsupported)?;
            if color.element_children().next().is_some() {
                return Err(unsupported());
            }
            return Ok(FrameFill::Solid(rgb));
        }
        _ => return Err(unsupported()),
    };
    let kids: Vec<&Element> = color.element_children().collect();
    let names: Vec<&str> = kids.iter().map(|child| child.local_name()).collect();
    let pct = |node: &Element| parse_ooxml_alpha(node.attr_local("val")).ok_or_else(unsupported);
    match names.as_slice() {
        [] => Ok(FrameFill::Solid(rgb)),
        ["alphaMod", "alphaMod"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_alpha_mod_pair(rgb, kids[0], kids[1]).ok_or_else(unsupported)
        }
        ["alphaMod", "alphaOff"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_alpha_mod_off(rgb, kids[0], kids[1], true).ok_or_else(unsupported)
        }
        ["alphaOff", "alphaMod"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_alpha_mod_off(rgb, kids[1], kids[0], false).ok_or_else(unsupported)
        }
        ["red", "alpha"] | ["green", "alpha"] | ["blue", "alpha"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_rgb_component_alpha(rgb, kids[0], kids[1]).ok_or_else(unsupported)
        }
        ["alpha", "red"] | ["alpha", "green"] | ["alpha", "blue"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_rgb_component_alpha(rgb, kids[1], kids[0]).ok_or_else(unsupported)
        }
        ["redMod", "alpha"] | ["greenMod", "alpha"] | ["blueMod", "alpha"]
        | ["redOff", "alpha"] | ["greenOff", "alpha"] | ["blueOff", "alpha"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_rgb_adjustment_alpha(rgb, kids[0], kids[1]).ok_or_else(unsupported)
        }
        ["alpha", "redMod"] | ["alpha", "greenMod"] | ["alpha", "blueMod"]
        | ["alpha", "redOff"] | ["alpha", "greenOff"] | ["alpha", "blueOff"] => {
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            frame_rgb_adjustment_alpha(rgb, kids[1], kids[0]).ok_or_else(unsupported)
        }
        ["redMod"] | ["greenMod"] | ["blueMod"] | ["redOff"] | ["greenOff"] | ["blueOff"] => {
            // Three new scheme controls diverged from the declared theme/reference.
            // Do not guess their style source or widen scheme/new-base semantics.
            if color.local_name() != "srgbClr" { return Err(unsupported()); }
            Ok(FrameFill::Solid(frame_rgb_adjustment(rgb, kids[0]).ok_or_else(unsupported)?))
        }
        ["red"] | ["green"] | ["blue"] => Ok(FrameFill::Solid(
            frame_rgb_component(rgb, kids[0]).ok_or_else(unsupported)?,
        )),
        ["alpha"] => frame_alpha_fill(rgb, kids[0]),
        ["alphaOff"] => Ok(frame_alpha_off_fill(rgb, 100_000, parse_luminance_offset(kids[0]).ok_or_else(unsupported)?)),
        ["alpha", "alphaOff"] => Ok(frame_alpha_off_fill(rgb, frame_initial_alpha(kids[0]).ok_or_else(unsupported)?, parse_luminance_offset(kids[1]).ok_or_else(unsupported)?)),
        ["alphaOff", "alpha"] => {
            // A later absolute alpha overrides, but does not excuse an invalid offset.
            parse_luminance_offset(kids[0]).ok_or_else(unsupported)?;
            Ok(frame_alpha_off_fill(rgb, frame_initial_alpha(kids[1]).ok_or_else(unsupported)?, 0))
        }
        ["hueOff"] => Ok(FrameFill::Solid(frame_hue_offset(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?))),
        ["hueOff", "alpha"] => frame_alpha_fill(frame_hue_offset(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?), kids[1]),
        ["alphaMod"] => Ok(frame_alpha_mod_fill(rgb, 100_000, parse_positive_percentage(kids[0]).ok_or_else(unsupported)?)),
        ["alpha", "alphaMod"] => Ok(frame_alpha_mod_fill(rgb, frame_initial_alpha(kids[0]).ok_or_else(unsupported)?, parse_positive_percentage(kids[1]).ok_or_else(unsupported)?)),
        ["alphaMod", "alpha"] => {
            // Absolute alpha sets the new opacity; it does not multiply it.
            // Still validate the overwritten operand rather than ignoring it.
            parse_positive_percentage(kids[0]).ok_or_else(unsupported)?;
            Ok(frame_alpha_mod_fill(rgb, frame_initial_alpha(kids[1]).ok_or_else(unsupported)?, 100_000))
        }

        ["tint"] => Ok(FrameFill::Solid(frame_tint(rgb, pct(kids[0])?))),
        ["tint", "alpha"] => frame_alpha_fill(frame_tint(rgb, pct(kids[0])?), kids[1]),
        ["shade"] => Ok(FrameFill::Solid(frame_shade(rgb, pct(kids[0])?))),
        ["shade", "alpha"] => frame_alpha_fill(frame_shade(rgb, pct(kids[0])?), kids[1]),
        // Reuse the strict leaf/i32 lexical parser, but interpret this value as
        // fixed-angle units. The helper enforces the angle's schema domain.
        ["hue"] => Ok(FrameFill::Solid(frame_absolute_hue(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?).ok_or_else(unsupported)?)),
        ["hueMod"] => Ok(FrameFill::Solid(frame_hue(rgb, parse_positive_percentage(kids[0]).ok_or_else(unsupported)?))),
        ["hueMod", "alpha"] => frame_alpha_fill(frame_hue(rgb, parse_positive_percentage(kids[0]).ok_or_else(unsupported)?), kids[1]),
        ["sat"] => Ok(FrameFill::Solid(frame_saturation(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?).ok_or_else(unsupported)?)),
        ["satOff"] => Ok(FrameFill::Solid(frame_sat_offset(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?).ok_or_else(unsupported)?)),
        ["satMod"] => Ok(FrameFill::Solid(frame_sat(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?))),
        ["satMod", "alpha"] => frame_alpha_fill(frame_sat(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?), kids[1]),
        ["lum"] => Ok(FrameFill::Solid(frame_luminance(rgb, parse_signed_percentage(kids[0]).ok_or_else(unsupported)?).ok_or_else(unsupported)?)),
        ["lumMod"] => Ok(FrameFill::Solid(frame_lum(rgb, Some(pct(kids[0])?), None))),
        ["lumMod", "alpha"] => frame_alpha_fill(frame_lum(rgb, Some(pct(kids[0])?), None), kids[1]),
        ["lumOff"] => Ok(FrameFill::Solid(frame_lum(
            rgb,
            None,
            Some(parse_luminance_offset(kids[0]).ok_or_else(unsupported)?),
        ))),
        ["lumOff", "alpha"] => frame_alpha_fill(
            frame_lum(rgb, None, Some(parse_luminance_offset(kids[0]).ok_or_else(unsupported)?)),
            kids[1],
        ),
        ["lumMod", "lumOff"] => Ok(FrameFill::Solid(frame_lum(
            rgb,
            Some(pct(kids[0])?),
            Some(parse_luminance_offset(kids[1]).ok_or_else(unsupported)?),
        ))),
        ["lumMod", "lumOff", "alpha"] => frame_alpha_fill(
            frame_lum(rgb, Some(pct(kids[0])?), Some(parse_luminance_offset(kids[1]).ok_or_else(unsupported)?)),
            kids[2],
        ),
        ["comp"] | ["inv"] | ["gamma"] | ["invGamma"] | ["gray"] => {
            // Official t187 matrix locked these on bare srgbClr + one empty
            // transform element. t189 unlocked length-2 chains of the same
            // empty leaves, applied in document order. schemeClr / sysClr stay
            // unsupported.
            if color.local_name() != "srgbClr" || !standalone_empty_transform(kids[0], names[0]) {
                return Err(unsupported());
            }
            Ok(FrameFill::Solid(match names[0] {
                "comp" => frame_comp(rgb),
                "inv" => frame_inv(rgb),
                "gamma" => frame_gamma(rgb),
                "invGamma" => frame_inv_gamma(rgb),
                "gray" => frame_gray(rgb),
                _ => return Err(unsupported()),
            }))
        }
        other if locked_srgb_transform_pair(color, kids.as_slice(), other) => {
            Ok(FrameFill::Solid(
                apply_locked_srgb_transforms(rgb, other).ok_or_else(unsupported)?,
            ))
        }
        _ => Err(unsupported()),
    }
}

impl LinearFrameGrad {
    pub(super) fn sample(&self, t: f64) -> [u8; 3] {
        let segment = self.stops.segment(t).expect("finite axis-aligned fill coordinate");
        if self.stops.len() == 2 {
            // Preserve the locked two-stop policy exactly. Fresh three/four-stop
            // Excel controls instead use encoded-sRGB piecewise interpolation.
            let share = office_grad_weight(segment.fraction);
            std::array::from_fn(|i| office_gamma_mix(segment.start[i], segment.end[i], share))
        } else {
            segment.encoded_srgb()
        }
    }
}

pub(super) fn office_grad_weight(t: f64) -> f64 {
    let x = t.clamp(0.0, 1.0) * 16.0;
    let i = (x.floor() as usize).min(OFFICE_GRAD_WEIGHT.len() - 2);
    let f = x - i as f64;
    OFFICE_GRAD_WEIGHT[i] * (1.0 - f) + OFFICE_GRAD_WEIGHT[i + 1] * f
}

pub(super) fn office_gamma_mix(start: u8, end: u8, share: f64) -> u8 {
    let from = (f64::from(start) / 255.0).powf(2.2);
    let to = (f64::from(end) / 255.0).powf(2.2);
    let mixed = (from + (to - from) * share).clamp(0.0, 1.0);
    (mixed.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
}

pub(super) fn color_has_non_opaque_alpha(slot: &Element) -> bool {
    let Some(node) = slot
        .child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })
    else {
        return false;
    };
    node.element_children()
        .any(|child| match child.local_name() {
            "alpha" => child
                .attr_local("val")
                .and_then(|raw| raw.parse::<f64>().ok())
                .is_none_or(|val| (val - 100_000.0).abs() > 1e-6),
            "alphaMod" | "alphaOff" => true,
            _ => false,
        })
}

pub(super) fn series_color(ser: &Element, accents: [[u8; 3]; 6]) -> Option<[u8; 3]> {
    let fill = ser.child_by_local("spPr")?.child_by_local("solidFill")?;
    color_of_slot(fill, &accents)
}

pub(super) fn color_of_slot(slot: &Element, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    let node = slot
        .child_by_local("srgbClr")
        .or_else(|| slot.child_by_local("sysClr"))
        .or_else(|| slot.child_by_local("schemeClr"))
        .or_else(|| {
            if matches!(slot.local_name(), "srgbClr" | "sysClr" | "schemeClr") {
                Some(slot)
            } else {
                None
            }
        })?;
    let mut rgb = if node.local_name() == "schemeClr" {
        scheme_rgb(node.attr_local("val")?, accents)?
    } else {
        parse_hex(
            node.attr_local("val")
                .or_else(|| node.attr_local("lastClr"))?,
        )?
    };
    for child in node.element_children() {
        let Some(raw) = child.attr_local("val") else {
            continue;
        };
        let Some(num) = raw.parse::<f64>().ok() else {
            continue;
        };
        let frac = num / 100_000.0;
        match child.local_name() {
            "lumMod" => rgb = lum_mod(rgb, frac),
            "lumOff" => rgb = lum_off(rgb, frac),
            "shade" => rgb = scale_rgb(rgb, frac),
            "tint" => rgb = tint_rgb(rgb, frac),
            _ => {}
        }
    }
    Some(rgb)
}

pub(super) fn scheme_rgb(name: &str, accents: &[[u8; 3]; 6]) -> Option<[u8; 3]> {
    Some(match name {
        "accent1" => accents[0],
        "accent2" => accents[1],
        "accent3" => accents[2],
        "accent4" => accents[3],
        "accent5" => accents[4],
        "accent6" => accents[5],
        "dk1" | "tx1" => [0, 0, 0],
        "lt1" | "bg1" => [255, 255, 255],
        "dk2" | "tx2" => [0x1F, 0x49, 0x7D],
        "lt2" | "bg2" => [0xEE, 0xEC, 0xE1],
        _ => return None,
    })
}

pub(super) fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim();
    if text.len() < 6 {
        return None;
    }
    let text = &text[text.len() - 6..];
    Some([
        u8::from_str_radix(&text[0..2], 16).ok()?,
        u8::from_str_radix(&text[2..4], 16).ok()?,
        u8::from_str_radix(&text[4..6], 16).ok()?,
    ])
}

pub(super) fn lum_mod(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    let (h, s, l) = rgb_hsl(rgb);
    hsl_rgb(h, s, (l * factor).clamp(0.0, 1.0))
}

pub(super) fn lum_off(rgb: [u8; 3], offset: f64) -> [u8; 3] {
    let (h, s, l) = rgb_hsl(rgb);
    hsl_rgb(h, s, (l + offset).clamp(0.0, 1.0))
}

pub(super) fn scale_rgb(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| ((c as f64) * factor).round().clamp(0.0, 255.0) as u8)
}

pub(super) fn tint_rgb(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| {
        let v = c as f64;
        (v + (255.0 - v) * factor).round().clamp(0.0, 255.0) as u8
    })
}

pub(super) fn rgb_hsl(rgb: [u8; 3]) -> (f64, f64, f64) {
    let r = rgb[0] as f64 / 255.0;
    let g = rgb[1] as f64 / 255.0;
    let b = rgb[2] as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-8 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-8 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-8 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

pub(super) fn hsl_rgb(h: f64, s: f64, l: f64) -> [u8; 3] {
    if s <= 1e-8 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return [v, v, v];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        hue(p, q, h + 1.0 / 3.0),
        hue(p, q, h),
        hue(p, q, h - 1.0 / 3.0),
    ]
}

pub(super) fn hue(p: f64, q: f64, mut t: f64) -> u8 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    let v = if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    };
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}
