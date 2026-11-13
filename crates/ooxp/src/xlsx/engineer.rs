//! 工程函数的数值部分。缓存对照 Excel 16.0，不另引工程库。

use std::f64::consts::PI;

use super::stats::erf;

const DIV0: &str = "#DIV/0!";
const NUM: &str = "#NUM!";
const VALUE: &str = "#VALUE!";
const NA: &str = "#N/A";

const BIN_DIGITS: usize = 10;
const BIN_BITS: u32 = 10;
const OCT_DIGITS: usize = 10;
const OCT_BITS: u32 = 30;
const HEX_DIGITS: usize = 10;
const HEX_BITS: u32 = 40;

fn finite(value: f64) -> Result<f64, &'static str> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(NUM)
    }
}

fn order_of(order: f64) -> Result<i32, &'static str> {
    if !order.is_finite() {
        return Err(NUM);
    }
    let order = order.trunc();
    if !(0.0..=170.0).contains(&order) {
        return Err(NUM);
    }
    Ok(order as i32)
}

fn as_int(value: f64) -> Result<i64, &'static str> {
    if !value.is_finite() {
        return Err(NUM);
    }
    let value = value.trunc();
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(NUM);
    }
    Ok(value as i64)
}

/// 位数向 0 取整。只认 1 到最大宽度。
fn places_width(places: Option<f64>, digits: usize) -> Result<Option<usize>, &'static str> {
    let Some(places) = places else {
        return Ok(None);
    };
    if !places.is_finite() {
        return Err(NUM);
    }
    let places = places.trunc();
    if places < 1.0 || places > digits as f64 {
        return Err(NUM);
    }
    Ok(Some(places as usize))
}

fn digamma(m: i32) -> f64 {
    const GAMMA: f64 = 0.577_215_664_901_532_9;
    let mut harmonic = 0.0;
    for i in 1..m {
        harmonic += 1.0 / f64::from(i);
    }
    -GAMMA + harmonic
}

fn series_start(half: f64, n: i32) -> Result<f64, &'static str> {
    let mut term = 1.0;
    for i in 1..=n {
        term *= half / f64::from(i);
        if !term.is_finite() {
            return Err(NUM);
        }
    }
    Ok(term)
}

fn bessel_j(x: f64, n: i32) -> Result<f64, &'static str> {
    let half = x / 2.0;
    let mut term = series_start(half, n)?;
    let mut sum = term;
    let step = -half * half;
    for k in 0..200 {
        let next = term * step / (f64::from(k + 1) * f64::from(n + k + 1));
        if !next.is_finite() {
            return Err(NUM);
        }
        if next.abs() <= 1e-16 * sum.abs().max(1.0) {
            break;
        }
        term = next;
        sum += term;
    }
    finite(sum)
}

fn bessel_i(x: f64, n: i32) -> Result<f64, &'static str> {
    let half = x / 2.0;
    let mut term = series_start(half, n)?;
    let mut sum = term;
    let step = half * half;
    for k in 0..200 {
        let next = term * step / (f64::from(k + 1) * f64::from(n + k + 1));
        if !next.is_finite() {
            return Err(NUM);
        }
        if next.abs() <= 1e-16 * sum.abs().max(1.0) {
            break;
        }
        term = next;
        sum += term;
    }
    finite(sum)
}

fn i_and_psi(half: f64, n: i32) -> Result<(f64, f64), &'static str> {
    let mut term = series_start(half, n)?;
    let mut i_sum = term;
    let mut psi_sum = (digamma(1) + digamma(n + 1)) * term;
    let step = half * half;
    for k in 0..200 {
        let next = term * step / (f64::from(k + 1) * f64::from(n + k + 1));
        if !next.is_finite() {
            return Err(NUM);
        }
        if next.abs() <= 1e-16 * i_sum.abs().max(1.0) {
            break;
        }
        term = next;
        i_sum += term;
        psi_sum += (digamma(k + 2) + digamma(n + k + 2)) * term;
    }
    Ok((i_sum, psi_sum))
}

fn finite_sum(half: f64, n: i32, alternate: bool) -> Result<f64, &'static str> {
    if n == 0 {
        return Ok(0.0);
    }
    let mut fact = 1.0;
    for i in 1..n {
        fact *= f64::from(i);
        if !fact.is_finite() {
            return Err(NUM);
        }
    }
    let mut term = fact;
    let mut acc = term;
    let half_sq = half * half;
    for k in 1..n {
        let mut step = half_sq / (f64::from(k) * f64::from(n - k));
        if alternate {
            step = -step;
        }
        term *= step;
        if !term.is_finite() {
            return Err(NUM);
        }
        acc += term;
    }
    finite(acc / half.powi(n))
}

fn bessel_y(x: f64, n: i32) -> Result<f64, &'static str> {
    if x <= 0.0 {
        return Err(NUM);
    }
    let half = x / 2.0;
    let j = bessel_j(x, n)?;
    let mut term = series_start(half, n)?;
    let mut psi = (digamma(1) + digamma(n + 1)) * term;
    let step = -half * half;
    for k in 0..200 {
        let next = term * step / (f64::from(k + 1) * f64::from(n + k + 1));
        if !next.is_finite() {
            return Err(NUM);
        }
        if next.abs() <= 1e-16 * j.abs().max(1.0) {
            break;
        }
        term = next;
        psi += (digamma(k + 2) + digamma(n + k + 2)) * term;
    }
    let sum = (2.0 / PI) * half.ln() * j - finite_sum(half, n, false)? / PI - psi / PI;
    finite(sum)
}

fn bessel_k(x: f64, n: i32) -> Result<f64, &'static str> {
    if x <= 0.0 {
        return Err(NUM);
    }
    let half = x / 2.0;
    let (i_sum, psi_sum) = i_and_psi(half, n)?;
    let log_sign = if n % 2 == 0 { -1.0 } else { 1.0 };
    let psi_sign = if n % 2 == 0 { 1.0 } else { -1.0 };
    let sum =
        0.5 * finite_sum(half, n, true)? + log_sign * half.ln() * i_sum + psi_sign * 0.5 * psi_sum;
    finite(sum)
}

fn bessel(kind: &str, x: f64, order: f64) -> Result<f64, &'static str> {
    if !x.is_finite() {
        return Err(NUM);
    }
    let n = order_of(order)?;
    match kind {
        "BESSELI" => bessel_i(x, n),
        "BESSELJ" => bessel_j(x, n),
        "BESSELK" => bessel_k(x, n),
        "BESSELY" => bessel_y(x, n),
        _ => Err(NUM),
    }
}

pub(crate) fn besseli(x: f64, order: f64) -> Result<f64, &'static str> {
    bessel("BESSELI", x, order)
}

pub(crate) fn besselj(x: f64, order: f64) -> Result<f64, &'static str> {
    bessel("BESSELJ", x, order)
}

pub(crate) fn besselk(x: f64, order: f64) -> Result<f64, &'static str> {
    bessel("BESSELK", x, order)
}

pub(crate) fn bessely(x: f64, order: f64) -> Result<f64, &'static str> {
    bessel("BESSELY", x, order)
}

fn parse_bits(text: &str, radix: u32, digits: usize, bits: u32) -> Result<i64, &'static str> {
    let text = text.trim();
    if text.is_empty() || text.len() > digits {
        return Err(NUM);
    }
    if !text.chars().all(|ch| ch.is_digit(radix)) {
        return Err(NUM);
    }
    let raw = i64::from_str_radix(text, radix).map_err(|_| NUM)?;
    if text.len() == digits {
        let sign = 1_i64 << (bits - 1);
        if raw & sign != 0 {
            return Ok(raw - (1_i64 << bits));
        }
    }
    Ok(raw)
}

fn write_digits(mut raw: u64, radix: u32) -> String {
    if raw == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while raw > 0 {
        let digit = (raw % u64::from(radix)) as u32;
        out.push(
            char::from_digit(digit, radix)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
        raw /= u64::from(radix);
    }
    out.reverse();
    out.into_iter().collect()
}

/// 负数已经占满补码宽度。位数只要落在 1 到 10，就忽略它，仍写出整段补码。
fn emit(
    value: i64,
    radix: u32,
    digits: usize,
    bits: u32,
    places: Option<f64>,
) -> Result<String, &'static str> {
    let width = places_width(places, digits)?;
    let min = -(1_i64 << (bits - 1));
    let max = (1_i64 << (bits - 1)) - 1;
    if value < min || value > max {
        return Err(NUM);
    }
    let raw = if value < 0 {
        (value + (1_i64 << bits)) as u64
    } else {
        value as u64
    };
    let text = write_digits(raw, radix);
    if value < 0 {
        return Ok(text);
    }
    if let Some(width) = width {
        if text.len() > width {
            return Err(NUM);
        }
        return Ok(format!("{text:0>width$}"));
    }
    Ok(text)
}

pub(crate) fn bin2dec(text: &str) -> Result<f64, &'static str> {
    Ok(parse_bits(text, 2, BIN_DIGITS, BIN_BITS)? as f64)
}

pub(crate) fn oct2dec(text: &str) -> Result<f64, &'static str> {
    Ok(parse_bits(text, 8, OCT_DIGITS, OCT_BITS)? as f64)
}

pub(crate) fn hex2dec(text: &str) -> Result<f64, &'static str> {
    Ok(parse_bits(text, 16, HEX_DIGITS, HEX_BITS)? as f64)
}

pub(crate) fn dec2bin(value: f64, places: Option<f64>) -> Result<String, &'static str> {
    emit(as_int(value)?, 2, BIN_DIGITS, BIN_BITS, places)
}

pub(crate) fn dec2oct(value: f64, places: Option<f64>) -> Result<String, &'static str> {
    emit(as_int(value)?, 8, OCT_DIGITS, OCT_BITS, places)
}

pub(crate) fn dec2hex(value: f64, places: Option<f64>) -> Result<String, &'static str> {
    emit(as_int(value)?, 16, HEX_DIGITS, HEX_BITS, places)
}

pub(crate) fn bin2hex(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 2, BIN_DIGITS, BIN_BITS)?,
        16,
        HEX_DIGITS,
        HEX_BITS,
        places,
    )
}

pub(crate) fn bin2oct(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 2, BIN_DIGITS, BIN_BITS)?,
        8,
        OCT_DIGITS,
        OCT_BITS,
        places,
    )
}

pub(crate) fn hex2bin(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 16, HEX_DIGITS, HEX_BITS)?,
        2,
        BIN_DIGITS,
        BIN_BITS,
        places,
    )
}

pub(crate) fn hex2oct(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 16, HEX_DIGITS, HEX_BITS)?,
        8,
        OCT_DIGITS,
        OCT_BITS,
        places,
    )
}

pub(crate) fn oct2bin(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 8, OCT_DIGITS, OCT_BITS)?,
        2,
        BIN_DIGITS,
        BIN_BITS,
        places,
    )
}

pub(crate) fn oct2hex(text: &str, places: Option<f64>) -> Result<String, &'static str> {
    emit(
        parse_bits(text, 8, OCT_DIGITS, OCT_BITS)?,
        16,
        HEX_DIGITS,
        HEX_BITS,
        places,
    )
}

#[derive(Clone, Copy)]
struct Cplx {
    re: f64,
    im: f64,
    suffix: Option<char>,
}

fn split_ri(body: &str) -> Result<(f64, f64), &'static str> {
    let bytes = body.as_bytes();
    let mut at = None;
    for i in 1..bytes.len() {
        let ch = bytes[i];
        if (ch == b'+' || ch == b'-') && bytes[i - 1] != b'e' && bytes[i - 1] != b'E' {
            at = Some(i);
        }
    }
    let Some(i) = at else {
        return Err(NUM);
    };
    let re: f64 = body[..i].parse().map_err(|_| NUM)?;
    let imag = &body[i..];
    let im = if imag == "+" {
        1.0
    } else if imag == "-" {
        -1.0
    } else {
        imag.parse::<f64>().map_err(|_| NUM)?
    };
    if !re.is_finite() || !im.is_finite() {
        return Err(NUM);
    }
    Ok((re, im))
}

fn parse_complex(text: &str) -> Result<Cplx, &'static str> {
    let text = text.trim();
    if text.is_empty() {
        return Err(NUM);
    }
    let last = text.chars().last().unwrap_or('0');
    let suffix = match last {
        'i' | 'I' => Some('i'),
        'j' | 'J' => Some('j'),
        _ => None,
    };
    let body = if suffix.is_some() {
        &text[..text.len() - last.len_utf8()]
    } else {
        text
    };
    if suffix.is_some() && (body.is_empty() || body == "+") {
        return Ok(Cplx {
            re: 0.0,
            im: 1.0,
            suffix,
        });
    }
    if suffix.is_some() && body == "-" {
        return Ok(Cplx {
            re: 0.0,
            im: -1.0,
            suffix,
        });
    }
    if suffix.is_none() {
        let re: f64 = body.parse().map_err(|_| NUM)?;
        if !re.is_finite() {
            return Err(NUM);
        }
        return Ok(Cplx {
            re,
            im: 0.0,
            suffix: None,
        });
    }
    if let Ok(im) = body.parse::<f64>() {
        if im.is_finite() {
            return Ok(Cplx {
                re: 0.0,
                im,
                suffix,
            });
        }
    }
    let (re, im) = split_ri(body)?;
    Ok(Cplx { re, im, suffix })
}

fn merge_suffix(left: Option<char>, right: Option<char>) -> Result<Option<char>, &'static str> {
    match (left, right) {
        (Some(a), Some(b)) if a != b => Err(VALUE),
        (Some(a), _) | (_, Some(a)) => Ok(Some(a)),
        _ => Ok(None),
    }
}

fn combine(left: Cplx, right: Cplx, re: f64, im: f64) -> Result<Cplx, &'static str> {
    Ok(Cplx {
        re,
        im,
        suffix: merge_suffix(left.suffix, right.suffix)?,
    })
}

/// 系数按 15 位有效数字。绝对值小于 1e-9 或达到 1e15 时用科学计数法。
fn format_coeff(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let neg = value.is_sign_negative();
    let value = value.abs();
    let mut exp = value.log10().floor() as i32;
    if !(-300..300).contains(&exp) {
        return "0".to_string();
    }
    let mut sig = (value * 10f64.powi(14 - exp)).round();
    if !sig.is_finite() {
        return "0".to_string();
    }
    if sig >= 1e15 {
        sig /= 10.0;
        exp += 1;
    }
    let mut n = sig as u64;
    let mut chars = Vec::new();
    if n == 0 {
        chars.push('0');
    } else {
        while n > 0 {
            chars.push(char::from_digit((n % 10) as u32, 10).unwrap_or('0'));
            n /= 10;
        }
        chars.reverse();
    }
    while chars.len() > 1 && chars.last() == Some(&'0') {
        chars.pop();
    }
    let body = if !(-9..15).contains(&exp) {
        let mut mant = String::new();
        mant.push(chars[0]);
        if chars.len() > 1 {
            mant.push('.');
            for ch in &chars[1..] {
                mant.push(*ch);
            }
        }
        if exp < 0 {
            format!("{mant}E{exp}")
        } else {
            format!("{mant}E+{exp}")
        }
    } else if exp >= 0 {
        let whole = (exp as usize) + 1;
        let mut out = String::new();
        if chars.len() <= whole {
            for ch in &chars {
                out.push(*ch);
            }
            for _ in chars.len()..whole {
                out.push('0');
            }
        } else {
            for ch in &chars[..whole] {
                out.push(*ch);
            }
            out.push('.');
            for ch in &chars[whole..] {
                out.push(*ch);
            }
        }
        out
    } else {
        let mut out = String::from("0.");
        for _ in 0..(-exp - 1) {
            out.push('0');
        }
        for ch in &chars {
            out.push(*ch);
        }
        out
    };
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

fn format_complex(re: f64, im: f64, suffix: Option<char>) -> String {
    let suffix = suffix.unwrap_or('i');
    let re_s = format_coeff(re);
    let im_s = format_coeff(im.abs());
    if im_s == "0" {
        return re_s;
    }
    let im_body = if im_s == "1" { String::new() } else { im_s };
    if re_s == "0" {
        if im.is_sign_negative() {
            format!("-{im_body}{suffix}")
        } else {
            format!("{im_body}{suffix}")
        }
    } else if im.is_sign_negative() {
        format!("{re_s}-{im_body}{suffix}")
    } else {
        format!("{re_s}+{im_body}{suffix}")
    }
}

fn show(z: Cplx) -> Result<String, &'static str> {
    if !z.re.is_finite() || !z.im.is_finite() {
        return Err(NUM);
    }
    Ok(format_complex(z.re, z.im, z.suffix))
}

pub(crate) fn complex(re: f64, im: f64, suffix: Option<&str>) -> Result<String, &'static str> {
    if !re.is_finite() || !im.is_finite() {
        return Err(NUM);
    }
    let suffix = match suffix.map(str::trim) {
        None => Some('i'),
        Some("i" | "I") => Some('i'),
        Some("j" | "J") => Some('j'),
        Some(_) => return Err(VALUE),
    };
    show(Cplx { re, im, suffix })
}

pub(crate) fn imabs(text: &str) -> Result<f64, &'static str> {
    let z = parse_complex(text)?;
    finite(z.re.hypot(z.im))
}

pub(crate) fn imaginary(text: &str) -> Result<f64, &'static str> {
    Ok(parse_complex(text)?.im)
}

pub(crate) fn imreal(text: &str) -> Result<f64, &'static str> {
    Ok(parse_complex(text)?.re)
}

pub(crate) fn imargument(text: &str) -> Result<f64, &'static str> {
    let z = parse_complex(text)?;
    if z.re == 0.0 && z.im == 0.0 {
        return Err(DIV0);
    }
    finite(z.im.atan2(z.re))
}

pub(crate) fn imconjugate(text: &str) -> Result<String, &'static str> {
    let z = parse_complex(text)?;
    show(Cplx {
        re: z.re,
        im: -z.im,
        suffix: z.suffix,
    })
}

fn im_cos(z: Cplx) -> Cplx {
    Cplx {
        re: z.re.cos() * z.im.cosh(),
        im: -z.re.sin() * z.im.sinh(),
        suffix: z.suffix,
    }
}

fn im_sin(z: Cplx) -> Cplx {
    Cplx {
        re: z.re.sin() * z.im.cosh(),
        im: z.re.cos() * z.im.sinh(),
        suffix: z.suffix,
    }
}

fn im_exp(z: Cplx) -> Cplx {
    let mag = z.re.exp();
    Cplx {
        re: mag * z.im.cos(),
        im: mag * z.im.sin(),
        suffix: z.suffix,
    }
}

fn im_ln(z: Cplx) -> Result<Cplx, &'static str> {
    let radius = z.re.hypot(z.im);
    if radius == 0.0 {
        return Err(NUM);
    }
    Ok(Cplx {
        re: radius.ln(),
        im: z.im.atan2(z.re),
        suffix: z.suffix,
    })
}

fn im_sqrt(z: Cplx) -> Cplx {
    let radius = z.re.hypot(z.im).sqrt();
    let theta = z.im.atan2(z.re) / 2.0;
    Cplx {
        re: radius * theta.cos(),
        im: radius * theta.sin(),
        suffix: z.suffix,
    }
}

pub(crate) fn imcos(text: &str) -> Result<String, &'static str> {
    show(im_cos(parse_complex(text)?))
}

pub(crate) fn imsin(text: &str) -> Result<String, &'static str> {
    show(im_sin(parse_complex(text)?))
}

pub(crate) fn imexp(text: &str) -> Result<String, &'static str> {
    show(im_exp(parse_complex(text)?))
}

pub(crate) fn imln(text: &str) -> Result<String, &'static str> {
    show(im_ln(parse_complex(text)?)?)
}

pub(crate) fn imlog10(text: &str) -> Result<String, &'static str> {
    let z = im_ln(parse_complex(text)?)?;
    let scale = 10f64.ln();
    show(Cplx {
        re: z.re / scale,
        im: z.im / scale,
        suffix: z.suffix,
    })
}

pub(crate) fn imlog2(text: &str) -> Result<String, &'static str> {
    let z = im_ln(parse_complex(text)?)?;
    let scale = 2f64.ln();
    show(Cplx {
        re: z.re / scale,
        im: z.im / scale,
        suffix: z.suffix,
    })
}

pub(crate) fn imsqrt(text: &str) -> Result<String, &'static str> {
    show(im_sqrt(parse_complex(text)?))
}

pub(crate) fn impower(text: &str, power: f64) -> Result<String, &'static str> {
    if !power.is_finite() {
        return Err(NUM);
    }
    let z = parse_complex(text)?;
    let radius = z.re.hypot(z.im);
    if radius == 0.0 {
        if power > 0.0 {
            return show(Cplx {
                re: 0.0,
                im: 0.0,
                suffix: z.suffix,
            });
        }
        return Err(NUM);
    }
    let theta = z.im.atan2(z.re) * power;
    let mag = radius.powf(power);
    show(Cplx {
        re: mag * theta.cos(),
        im: mag * theta.sin(),
        suffix: z.suffix,
    })
}

pub(crate) fn imsub(left: &str, right: &str) -> Result<String, &'static str> {
    let a = parse_complex(left)?;
    let b = parse_complex(right)?;
    show(combine(a, b, a.re - b.re, a.im - b.im)?)
}

pub(crate) fn imdiv(left: &str, right: &str) -> Result<String, &'static str> {
    let a = parse_complex(left)?;
    let b = parse_complex(right)?;
    let denom = b.re * b.re + b.im * b.im;
    if denom == 0.0 {
        return Err(NUM);
    }
    let re = (a.re * b.re + a.im * b.im) / denom;
    let im = (a.im * b.re - a.re * b.im) / denom;
    show(combine(a, b, re, im)?)
}

pub(crate) fn imsum(parts: &[&str]) -> Result<String, &'static str> {
    let mut acc = Cplx {
        re: 0.0,
        im: 0.0,
        suffix: None,
    };
    for part in parts {
        let z = parse_complex(part)?;
        acc = combine(acc, z, acc.re + z.re, acc.im + z.im)?;
    }
    show(acc)
}

pub(crate) fn improduct(parts: &[&str]) -> Result<String, &'static str> {
    let mut acc = Cplx {
        re: 1.0,
        im: 0.0,
        suffix: None,
    };
    for part in parts {
        let z = parse_complex(part)?;
        let re = acc.re * z.re - acc.im * z.im;
        let im = acc.re * z.im + acc.im * z.re;
        acc = combine(acc, z, re, im)?;
    }
    show(acc)
}

pub(crate) fn convert(value: f64, from: &str, to: &str) -> Result<f64, &'static str> {
    if !value.is_finite() {
        return Err(NUM);
    }
    let same = matches!(
        (from, to),
        ("lbm", "lbm") | ("kg", "kg") | ("C", "C") | ("F", "F")
    );
    if same {
        return Ok(value);
    }
    match (from, to) {
        ("lbm", "kg") => finite(value * 0.453_592_37),
        ("kg", "lbm") => finite(value / 0.453_592_37),
        ("C", "F") => finite(value * 9.0 / 5.0 + 32.0),
        ("F", "C") => finite((value - 32.0) * 5.0 / 9.0),
        _ => Err(NA),
    }
}

pub(crate) fn delta(left: f64, right: f64) -> Result<f64, &'static str> {
    if !left.is_finite() || !right.is_finite() {
        return Err(NUM);
    }
    Ok(if left == right { 1.0 } else { 0.0 })
}

pub(crate) fn gestep(number: f64, step: f64) -> Result<f64, &'static str> {
    if !number.is_finite() || !step.is_finite() {
        return Err(NUM);
    }
    Ok(if number >= step { 1.0 } else { 0.0 })
}

pub(crate) fn erf_at(x: f64) -> Result<f64, &'static str> {
    if !x.is_finite() {
        return Err(NUM);
    }
    finite(erf(x))
}

pub(crate) fn erf_between(lower: f64, upper: f64) -> Result<f64, &'static str> {
    if !lower.is_finite() || !upper.is_finite() {
        return Err(NUM);
    }
    finite(erf(upper) - erf(lower))
}

pub(crate) fn erfc_at(x: f64) -> Result<f64, &'static str> {
    if !x.is_finite() {
        return Err(NUM);
    }
    finite(1.0 - erf(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: Result<f64, &'static str>, expect: f64) {
        let got = got.unwrap();
        let tol = 1e-8_f64.max(expect.abs() * 1e-8);
        assert!((got - expect).abs() < tol, "got {got} expect {expect}");
    }

    #[test]
    fn excel_16_engineering_samples() {
        near(besseli(1.5, 1.0), 0.981_666_428_475_166);
        near(besseli(1.5, 1.9), 0.981_666_428_475_166);
        near(besselj(1.5, 1.0), 0.557_936_507_890_804_3);
        near(besselj(-1.5, 1.0), -0.557_936_507_890_804_3);
        near(besselk(1.5, 1.0), 0.277_387_803_632_258_7);
        near(bessely(1.5, 1.0), -0.412_308_626_895_903_75);
        assert!(besselj(1.5, -1.0).is_err());
        assert!(bessely(-1.0, 1.0).is_err());

        assert_eq!(bin2dec("1010").unwrap(), 10.0);
        assert_eq!(bin2dec("1111111111").unwrap(), -1.0);
        assert!(bin2dec("2").is_err());
        assert_eq!(bin2hex("1010", None).unwrap(), "A");
        assert_eq!(bin2hex("1111111111", Some(4.0)).unwrap(), "FFFFFFFFFF");
        assert_eq!(bin2oct("1010", None).unwrap(), "12");
        assert_eq!(dec2bin(10.0, None).unwrap(), "1010");
        assert_eq!(dec2bin(10.0, Some(8.9)).unwrap(), "00001010");
        assert_eq!(dec2bin(-1.0, None).unwrap(), "1111111111");
        assert_eq!(dec2bin(-1.0, Some(4.0)).unwrap(), "1111111111");
        assert!(dec2bin(512.0, None).is_err());
        assert!(dec2bin(10.0, Some(2.0)).is_err());
        assert!(dec2bin(-1.0, Some(11.0)).is_err());
        assert_eq!(dec2hex(255.0, Some(4.0)).unwrap(), "00FF");
        assert_eq!(dec2hex(-1.0, None).unwrap(), "FFFFFFFFFF");
        assert_eq!(dec2hex(-1.0, Some(4.0)).unwrap(), "FFFFFFFFFF");
        assert_eq!(dec2oct(8.0, Some(4.0)).unwrap(), "0010");
        assert_eq!(dec2oct(-1.0, None).unwrap(), "7777777777");
        assert_eq!(hex2bin("A", None).unwrap(), "1010");
        assert_eq!(hex2dec("FF").unwrap(), 255.0);
        assert_eq!(hex2dec("FFFFFFFFFF").unwrap(), -1.0);
        assert_eq!(hex2dec("8000000000").unwrap(), -549_755_813_888.0);
        assert_eq!(hex2oct("A", None).unwrap(), "12");
        assert_eq!(oct2bin("10", None).unwrap(), "1000");
        assert_eq!(oct2dec("10").unwrap(), 8.0);
        assert_eq!(oct2dec("7777777777").unwrap(), -1.0);
        assert_eq!(oct2hex("10", None).unwrap(), "8");

        assert_eq!(complex(3.0, 4.0, None).unwrap(), "3+4i");
        assert_eq!(complex(3.0, 4.0, Some("j")).unwrap(), "3+4j");
        assert_eq!(complex(0.0, 1.0, None).unwrap(), "i");
        assert_eq!(complex(3.0, 0.0, None).unwrap(), "3");
        assert_eq!(complex(0.0, -1.0, None).unwrap(), "-i");
        assert_eq!(complex(3.0, -4.0, None).unwrap(), "3-4i");
        assert_eq!(complex(0.0, 2.0, None).unwrap(), "2i");
        assert_eq!(complex(0.0, -2.0, None).unwrap(), "-2i");
        assert_eq!(complex(3.0, 1.0, None).unwrap(), "3+i");
        assert_eq!(complex(3.0, -1.0, None).unwrap(), "3-i");
        assert_eq!(complex(0.0, 0.0, None).unwrap(), "0");
        assert_eq!(complex(0.0, 1.0, Some("j")).unwrap(), "j");
        assert_eq!(imabs("3+4i").unwrap(), 5.0);
        assert_eq!(imabs("j").unwrap(), 1.0);
        assert_eq!(imaginary("3+4i").unwrap(), 4.0);
        near(imargument("3+4i"), 0.927_295_218_001_612_2);
        assert_eq!(imargument("0").unwrap_err(), DIV0);
        assert_eq!(imconjugate("3+4i").unwrap(), "3-4i");
        assert_eq!(imdiv("1+2i", "3+4i").unwrap(), "0.44+0.08i");
        assert_eq!(imdiv("1", "0").unwrap_err(), NUM);
        assert_eq!(improduct(&["1+2i", "3+4i"]).unwrap(), "-5+10i");
        assert_eq!(imreal("3+4i").unwrap(), 3.0);
        assert_eq!(imsqrt("3+4i").unwrap(), "2+i");
        assert_eq!(imsub("3+4i", "1+i").unwrap(), "2+3i");
        assert_eq!(imsum(&["i", "i"]).unwrap(), "2i");
        assert_eq!(imsum(&["i", "j"]).unwrap_err(), VALUE);
        assert_eq!(
            imcos("3+4i").unwrap(),
            "-27.0349456030742-3.85115333481178i"
        );
        assert_eq!(imexp("1+i").unwrap(), "1.46869393991589+2.28735528717884i");
        assert_eq!(imln("1+i").unwrap(), "0.346573590279973+0.785398163397448i");
        assert_eq!(
            imlog10("1+i").unwrap(),
            "0.150514997831991+0.34109408846046i"
        );
        assert_eq!(imlog2("1+i").unwrap(), "0.5+1.1330900354568i");
        assert_eq!(imsin("1+i").unwrap(), "1.29845758141598+0.634963914784736i");
        let powered = impower("1+i", 2.0).unwrap();
        assert!(
            powered.ends_with("+2i") && powered.contains('E'),
            "{powered}"
        );

        near(convert(1.0, "lbm", "kg"), 0.453_592_37);
        assert_eq!(convert(100.0, "C", "F").unwrap(), 212.0);
        near(convert(1.0, "kg", "lbm"), 2.204_622_621_848_775_7);
        assert_eq!(convert(32.0, "F", "C").unwrap(), 0.0);
        assert_eq!(convert(1.0, "m", "kg").unwrap_err(), NA);
        assert_eq!(delta(5.0, 5.0).unwrap(), 1.0);
        assert_eq!(delta(5.0, 4.0).unwrap(), 0.0);
        assert_eq!(delta(5.0, 0.0).unwrap(), 0.0);
        assert_eq!(gestep(5.0, 4.0).unwrap(), 1.0);
        assert_eq!(gestep(5.0, 0.0).unwrap(), 1.0);
        assert_eq!(gestep(-1.0, 0.0).unwrap(), 0.0);
        near(erf_at(1.0), 0.842_700_792_949_714_9);
        near(erf_between(-1.0, 1.0), 1.685_401_585_899_429_8);
        near(erfc_at(1.0), 0.157_299_207_050_285_13);
        assert_eq!(erf_at(0.0).unwrap(), 0.0);
    }
}
