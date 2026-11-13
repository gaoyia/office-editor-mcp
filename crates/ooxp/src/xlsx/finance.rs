//! 财务函数的数值部分。缓存对照 Excel 16.0，不另引金融库。

use super::eval::{days_360, days_in_month, excel_serial, excel_ymd, shift_months};

const DIV0: &str = "#DIV/0!";
const NUM: &str = "#NUM!";

fn finite(value: f64) -> Result<f64, &'static str> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(NUM)
    }
}

fn frequency(value: f64) -> Result<i32, &'static str> {
    let freq = value.trunc() as i32;
    if freq == 1 || freq == 2 || freq == 4 {
        Ok(freq)
    } else {
        Err(NUM)
    }
}

fn basis_of(value: f64) -> Result<i32, &'static str> {
    let basis = value.trunc() as i32;
    if (0..=4).contains(&basis) {
        Ok(basis)
    } else {
        Err(NUM)
    }
}

fn day(serial: f64) -> f64 {
    serial.floor()
}

fn ymd(serial: f64, date1904: bool) -> Result<(i32, u32, u32), &'static str> {
    excel_ymd(day(serial), date1904).ok_or(NUM)
}

fn end_of_month(serial: f64, date1904: bool) -> Result<bool, &'static str> {
    let (year, month, day) = ymd(serial, date1904)?;
    Ok(day == days_in_month(year, month, date1904))
}

fn shift(serial: f64, months: i32, eom: bool, date1904: bool) -> Result<f64, &'static str> {
    shift_months(day(serial), months, eom, date1904).ok_or(NUM)
}

/// 两个日期之间的天数。基准 1、2、3 用实际天数，0 和 4 用 30/360。
fn spanned(start: f64, end: f64, basis: i32, date1904: bool) -> Result<f64, &'static str> {
    let start = day(start);
    let end = day(end);
    match basis {
        0 => days_360(start, end, false, date1904).ok_or(NUM),
        4 => days_360(start, end, true, date1904).ok_or(NUM),
        1..=3 => Ok(end - start),
        _ => Err(NUM),
    }
}

fn later(start: f64, end: f64) -> Result<(), &'static str> {
    if day(end) <= day(start) {
        Err(NUM)
    } else {
        Ok(())
    }
}

pub(crate) fn pv(rate: f64, nper: f64, pmt: f64, fv: f64, typ: f64) -> Result<f64, &'static str> {
    if rate == 0.0 {
        return finite(-(fv + pmt * nper));
    }
    if rate <= -1.0 {
        return Err(NUM);
    }
    let pow = (1.0 + rate).powf(nper);
    let factor = (1.0 + rate * typ) * (pow - 1.0) / rate;
    finite(-(fv + pmt * factor) / pow)
}

pub(crate) fn fv(rate: f64, nper: f64, pmt: f64, pv: f64, typ: f64) -> Result<f64, &'static str> {
    if rate == 0.0 {
        return finite(-(pv + pmt * nper));
    }
    if rate <= -1.0 {
        return Err(NUM);
    }
    let pow = (1.0 + rate).powf(nper);
    let factor = (1.0 + rate * typ) * (pow - 1.0) / rate;
    finite(-(pv * pow + pmt * factor))
}

pub(crate) fn pmt(rate: f64, nper: f64, pv: f64, fv: f64, typ: f64) -> Result<f64, &'static str> {
    if nper == 0.0 {
        return Err(NUM);
    }
    if rate == 0.0 {
        return finite(-(pv + fv) / nper);
    }
    if rate <= -1.0 {
        return Err(NUM);
    }
    let pow = (1.0 + rate).powf(nper);
    let factor = (1.0 + rate * typ) * (pow - 1.0) / rate;
    if factor == 0.0 {
        return Err(NUM);
    }
    finite(-(pv * pow + fv) / factor)
}

pub(crate) fn nper(rate: f64, pmt: f64, pv: f64, fv: f64, typ: f64) -> Result<f64, &'static str> {
    if rate == 0.0 {
        if pmt == 0.0 {
            return Err(DIV0);
        }
        return finite(-(pv + fv) / pmt);
    }
    if rate <= -1.0 {
        return Err(NUM);
    }
    let annuity = pmt * (1.0 + rate * typ);
    let num = annuity - fv * rate;
    let den = pv * rate + annuity;
    if den == 0.0 || num / den <= 0.0 {
        return Err(NUM);
    }
    finite((num / den).ln() / (1.0 + rate).ln())
}

pub(crate) fn ipmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv: f64,
    future: f64,
    typ: f64,
) -> Result<f64, &'static str> {
    let per = per.trunc();
    if per < 1.0 || (nper > 0.0 && per > nper && nper.fract() == 0.0) {
        return Err(NUM);
    }
    if typ != 0.0 && per == 1.0 {
        return Ok(0.0);
    }
    let pay = pmt(rate, nper, pv, future, typ)?;
    if rate == 0.0 {
        return Ok(0.0);
    }
    let balance = fv(rate, per - 1.0, pay, pv, typ)?;
    finite(balance * rate)
}

pub(crate) fn ppmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv: f64,
    fv: f64,
    typ: f64,
) -> Result<f64, &'static str> {
    let pay = pmt(rate, nper, pv, fv, typ)?;
    let interest = ipmt(rate, per, nper, pv, fv, typ)?;
    finite(pay - interest)
}

pub(crate) fn ispmt(rate: f64, per: f64, nper: f64, pv: f64) -> Result<f64, &'static str> {
    if nper == 0.0 {
        return Err(DIV0);
    }
    finite(pv * rate * (per / nper - 1.0))
}

pub(crate) fn cum_ipmt(
    rate: f64,
    nper: f64,
    pv: f64,
    start: f64,
    end: f64,
    typ: f64,
) -> Result<f64, &'static str> {
    cum_part(rate, nper, pv, start, end, typ, true)
}

pub(crate) fn cum_princ(
    rate: f64,
    nper: f64,
    pv: f64,
    start: f64,
    end: f64,
    typ: f64,
) -> Result<f64, &'static str> {
    cum_part(rate, nper, pv, start, end, typ, false)
}

fn cum_part(
    rate: f64,
    nper: f64,
    pv: f64,
    start: f64,
    end: f64,
    typ: f64,
    interest: bool,
) -> Result<f64, &'static str> {
    let start = start.trunc();
    let end = end.trunc();
    let typ = typ.trunc();
    if start < 1.0 || end < start || nper < 1.0 || rate <= 0.0 || !(typ == 0.0 || typ == 1.0) {
        return Err(NUM);
    }
    let mut total = 0.0;
    let mut period = start;
    while period <= end {
        let part = if interest {
            ipmt(rate, period, nper, pv, 0.0, typ)?
        } else {
            ppmt(rate, period, nper, pv, 0.0, typ)?
        };
        total += part;
        period += 1.0;
    }
    finite(total)
}

pub(crate) fn rate(
    nper: f64,
    pmt: f64,
    pv: f64,
    fv: f64,
    typ: f64,
    guess: f64,
) -> Result<f64, &'static str> {
    if nper == 0.0 {
        return Err(NUM);
    }
    solve(guess, |rate| {
        if rate <= -1.0 {
            return None;
        }
        Some(annuity_gap(rate, nper, pmt, pv, fv, typ))
    })
}

fn annuity_gap(rate: f64, nper: f64, pmt: f64, pv: f64, fv: f64, typ: f64) -> f64 {
    if rate.abs() < 1e-14 {
        return pv + pmt * nper + fv;
    }
    let pow = (1.0 + rate).powf(nper);
    let factor = (1.0 + rate * typ) * (pow - 1.0) / rate;
    pv * pow + pmt * factor + fv
}

pub(crate) fn npv(rate: f64, values: &[f64]) -> Result<f64, &'static str> {
    if rate <= -1.0 {
        return Err(NUM);
    }
    let mut sum = 0.0;
    for (index, value) in values.iter().enumerate() {
        sum += value / (1.0 + rate).powf((index + 1) as f64);
    }
    finite(sum)
}

pub(crate) fn irr(values: &[f64], guess: f64) -> Result<f64, &'static str> {
    if !values.iter().any(|v| *v > 0.0) || !values.iter().any(|v| *v < 0.0) {
        return Err(NUM);
    }
    solve(guess, |rate| {
        if rate <= -1.0 {
            return None;
        }
        Some(npv_at(rate, values))
    })
}

fn npv_at(rate: f64, values: &[f64]) -> f64 {
    values.iter().enumerate().fold(0.0, |sum, (index, value)| {
        sum + value / (1.0 + rate).powf(index as f64)
    })
}

pub(crate) fn mirr(values: &[f64], finance: f64, reinvest: f64) -> Result<f64, &'static str> {
    let count = values.len();
    if count < 2 || finance <= -1.0 || reinvest <= -1.0 {
        return Err(NUM);
    }
    let last = (count - 1) as f64;
    let mut positive = 0.0;
    let mut negative = 0.0;
    for (index, value) in values.iter().enumerate() {
        if *value > 0.0 {
            positive += value * (1.0 + reinvest).powf(last - index as f64);
        } else if *value < 0.0 {
            negative += value / (1.0 + finance).powf(index as f64);
        }
    }
    if positive == 0.0 || negative == 0.0 {
        return Err(DIV0);
    }
    finite((positive / -negative).powf(1.0 / last) - 1.0)
}

pub(crate) fn xnpv(rate: f64, values: &[f64], dates: &[f64]) -> Result<f64, &'static str> {
    if rate <= -1.0 || values.len() != dates.len() || values.is_empty() {
        return Err(NUM);
    }
    let first = day(dates[0]);
    let mut sum = 0.0;
    for (value, date) in values.iter().zip(dates) {
        let when = day(*date);
        if when < first {
            return Err(NUM);
        }
        sum += value / (1.0 + rate).powf((when - first) / 365.0);
    }
    finite(sum)
}

pub(crate) fn xirr(values: &[f64], dates: &[f64], guess: f64) -> Result<f64, &'static str> {
    solve(guess, |rate| xnpv(rate, values, dates).ok())
}

pub(crate) fn fvschedule(principal: f64, rates: &[f64]) -> Result<f64, &'static str> {
    let mut value = principal;
    for rate in rates {
        value *= 1.0 + rate;
    }
    finite(value)
}

pub(crate) fn effect(nominal: f64, npery: f64) -> Result<f64, &'static str> {
    let periods = npery.trunc();
    if periods < 1.0 || nominal <= 0.0 {
        return Err(NUM);
    }
    finite((1.0 + nominal / periods).powf(periods) - 1.0)
}

pub(crate) fn nominal(effect_rate: f64, npery: f64) -> Result<f64, &'static str> {
    let periods = npery.trunc();
    if periods < 1.0 || effect_rate <= 0.0 {
        return Err(NUM);
    }
    finite(periods * ((effect_rate + 1.0).powf(1.0 / periods) - 1.0))
}

pub(crate) fn dollarde(fractional: f64, fraction: f64) -> Result<f64, &'static str> {
    dollar_convert(fractional, fraction, true)
}

pub(crate) fn dollarfr(decimal: f64, fraction: f64) -> Result<f64, &'static str> {
    dollar_convert(decimal, fraction, false)
}

fn dollar_convert(amount: f64, fraction: f64, to_decimal: bool) -> Result<f64, &'static str> {
    let fraction = fraction.trunc();
    if fraction < 1.0 {
        return Err(DIV0);
    }
    let whole = amount.trunc();
    let rest = amount - whole;
    let scale = 10_f64.powf(fraction.log10().ceil());
    let converted = if to_decimal {
        rest * scale / fraction
    } else {
        rest * fraction / scale
    };
    finite(whole + converted)
}

pub(crate) fn sln(cost: f64, salvage: f64, life: f64) -> Result<f64, &'static str> {
    if life == 0.0 {
        return Err(DIV0);
    }
    finite((cost - salvage) / life)
}

pub(crate) fn syd(cost: f64, salvage: f64, life: f64, period: f64) -> Result<f64, &'static str> {
    let life = life.trunc();
    let period = period.trunc();
    if life < 1.0 || period < 1.0 || period > life {
        return Err(NUM);
    }
    finite((cost - salvage) * (life - period + 1.0) * 2.0 / (life * (life + 1.0)))
}

pub(crate) fn db(
    cost: f64,
    salvage: f64,
    life: f64,
    period: f64,
    month: f64,
) -> Result<f64, &'static str> {
    let life = life.trunc();
    let period = period.trunc();
    let month = month.trunc();
    if cost <= 0.0 || life < 1.0 || period < 1.0 || !(1.0..=12.0).contains(&month) {
        return Err(NUM);
    }
    let last = if month == 12.0 { life } else { life + 1.0 };
    if period > last {
        return Err(NUM);
    }
    let rate = ((1.0 - (salvage / cost).powf(1.0 / life)) * 1000.0).round() / 1000.0;
    let mut total = 0.0;
    let mut depreciation = 0.0;
    let mut at = 1.0;
    while at <= period {
        depreciation = if at == 1.0 {
            cost * rate * month / 12.0
        } else if at == life + 1.0 {
            (cost - total) * rate * (12.0 - month) / 12.0
        } else {
            (cost - total) * rate
        };
        total += depreciation;
        at += 1.0;
    }
    finite(depreciation)
}

pub(crate) fn ddb(
    cost: f64,
    salvage: f64,
    life: f64,
    period: f64,
    factor: f64,
) -> Result<f64, &'static str> {
    let period = period.trunc();
    let life_whole = life.trunc();
    if cost < 0.0 || life_whole < 1.0 || period < 1.0 || period > life_whole || factor <= 0.0 {
        return Err(NUM);
    }
    let mut book = cost;
    let mut depreciation = 0.0;
    let mut at = 1.0;
    while at <= period {
        let declining = book * factor / life_whole;
        let room = book - salvage;
        depreciation = declining.min(room).max(0.0);
        book -= depreciation;
        at += 1.0;
    }
    finite(depreciation)
}

pub(crate) fn vdb(
    cost: f64,
    salvage: f64,
    life: f64,
    start: f64,
    end: f64,
    factor: f64,
    no_switch: bool,
) -> Result<f64, &'static str> {
    if cost < 0.0
        || salvage < 0.0
        || life <= 0.0
        || factor <= 0.0
        || start < 0.0
        || end < start
        || end > life
    {
        return Err(NUM);
    }
    let mut total = 0.0;
    let mut book = cost;
    let mut at = 0.0;
    while at < end - 1e-12 {
        let next = (at + 1.0).min(life);
        let declining = book * factor / life;
        let straight = if life > at {
            (book - salvage) / (life - at)
        } else {
            0.0
        };
        let mut depreciation = if no_switch {
            declining
        } else {
            declining.max(straight)
        };
        depreciation = depreciation.min(book - salvage).max(0.0);
        let from = start.max(at);
        let to = end.min(next);
        if to > from {
            total += depreciation * (to - from) / (next - at);
        }
        book -= depreciation;
        at = next;
    }
    finite(total)
}

pub(crate) fn amorlinc(
    cost: f64,
    purchased: f64,
    first: f64,
    salvage: f64,
    period: f64,
    rate: f64,
    basis: i32,
    date1904: bool,
) -> Result<f64, &'static str> {
    let period = period.trunc();
    if cost <= 0.0 || rate <= 0.0 || period < 0.0 || salvage < 0.0 {
        return Err(NUM);
    }
    later(purchased, first)?;
    let stub = cost * rate * year_frac(purchased, first, basis, date1904)?;
    if period == 0.0 {
        return finite(stub);
    }
    let mut left = cost - salvage - stub;
    let full = cost * rate;
    let mut at = 1.0;
    while at <= period {
        let depreciation = if left < full { left.max(0.0) } else { full };
        if at == period {
            return finite(depreciation);
        }
        left -= depreciation;
        at += 1.0;
    }
    Ok(0.0)
}

pub(crate) fn amordegrc(
    cost: f64,
    purchased: f64,
    first: f64,
    salvage: f64,
    period: f64,
    rate: f64,
    basis: i32,
    date1904: bool,
) -> Result<f64, &'static str> {
    let period = period.trunc();
    if cost <= 0.0 || rate <= 0.0 || period < 0.0 {
        return Err(NUM);
    }
    let life = 1.0 / rate;
    let coefficient = if (3.0..4.0).contains(&life) {
        1.5
    } else if (5.0..=6.0).contains(&life) {
        2.0
    } else if life > 6.0 {
        2.5
    } else {
        return Err(NUM);
    };
    later(purchased, first)?;
    let grown = rate * coefficient;
    let stub = (year_frac(purchased, first, basis, date1904)? * grown * cost).round();
    if period == 0.0 {
        return finite(stub);
    }
    let last = life.floor();
    if period >= last {
        return Ok(0.0);
    }
    let mut rest = cost - year_frac(purchased, first, basis, date1904)? * grown * cost;
    let mut depreciation = stub;
    let mut at = 1.0;
    while at <= period {
        if at == last - 1.0 {
            depreciation = (rest / 2.0).round();
        } else {
            let raw = rest * grown;
            depreciation = raw.round();
            rest -= raw;
        }
        at += 1.0;
    }
    let _ = salvage;
    finite(depreciation)
}

fn year_frac(start: f64, end: f64, basis: i32, date1904: bool) -> Result<f64, &'static str> {
    let days = spanned(start, end, basis, date1904)?;
    let year = match basis {
        0 | 2 | 4 => 360.0,
        3 => 365.0,
        1 => 365.0,
        _ => return Err(NUM),
    };
    // 基准 1 的年分数按实际天数除以这段所跨年份的平均长度。购买日样本不跨闰日，365 即可。
    if basis == 1 {
        return finite(days / year_length(start, end, date1904)?);
    }
    finite(days / year)
}

fn year_length(start: f64, end: f64, date1904: bool) -> Result<f64, &'static str> {
    let (y1, _, _) = ymd(start, date1904)?;
    let (y2, _, _) = ymd(end, date1904)?;
    if y1 == y2 {
        let leap = excel_serial(y1, 3, 1, date1904) - excel_serial(y1, 2, 28, date1904) > 1.0;
        return Ok(if leap { 366.0 } else { 365.0 });
    }
    Ok(365.0)
}

struct Coupon {
    previous: f64,
    next: f64,
    before: f64,
    after: f64,
    length: f64,
    count: i32,
}

fn coupons(
    settlement: f64,
    maturity: f64,
    freq: i32,
    basis: i32,
    date1904: bool,
) -> Result<Coupon, &'static str> {
    later(settlement, maturity)?;
    let eom = end_of_month(maturity, date1904)?;
    let step = 12 / freq;
    let mut previous = day(maturity);
    let mut guard = 0;
    while previous > day(settlement) {
        previous = shift(previous, -step, eom, date1904)?;
        guard += 1;
        if guard > 10000 {
            return Err(NUM);
        }
    }
    let next = shift(previous, step, eom, date1904)?;
    let mut cursor = next;
    let mut count = 0;
    while cursor <= day(maturity) + 1e-9 {
        count += 1;
        if (cursor - day(maturity)).abs() < 1e-9 {
            break;
        }
        cursor = shift(cursor, step, eom, date1904)?;
        if count > 10000 {
            return Err(NUM);
        }
    }
    let length = period_length(previous, next, freq, basis, date1904)?;
    Ok(Coupon {
        previous,
        next,
        before: spanned(previous, settlement, basis, date1904)?,
        after: spanned(settlement, next, basis, date1904)?,
        length,
        count,
    })
}

fn period_length(
    previous: f64,
    next: f64,
    freq: i32,
    basis: i32,
    date1904: bool,
) -> Result<f64, &'static str> {
    match basis {
        0 | 2 | 4 => Ok(360.0 / f64::from(freq)),
        3 => Ok(365.0 / f64::from(freq)),
        1 => Ok(day(next) - day(previous)),
        _ => {
            let _ = date1904;
            Err(NUM)
        }
    }
}

pub(crate) fn coup_days(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let view = coupons(
        settlement,
        maturity,
        frequency(freq)?,
        basis_of(basis)?,
        date1904,
    )?;
    Ok(view.length)
}

pub(crate) fn coup_daybs(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let view = coupons(
        settlement,
        maturity,
        frequency(freq)?,
        basis_of(basis)?,
        date1904,
    )?;
    Ok(view.before)
}

pub(crate) fn coup_daysnc(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let view = coupons(
        settlement,
        maturity,
        frequency(freq)?,
        basis_of(basis)?,
        date1904,
    )?;
    Ok(view.after)
}

pub(crate) fn coup_ncd(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let _ = basis_of(basis)?;
    let view = coupons(settlement, maturity, frequency(freq)?, 0, date1904)?;
    Ok(view.next)
}

pub(crate) fn coup_pcd(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let _ = basis_of(basis)?;
    let view = coupons(settlement, maturity, frequency(freq)?, 0, date1904)?;
    Ok(view.previous)
}

pub(crate) fn coup_num(
    settlement: f64,
    maturity: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let _ = basis_of(basis)?;
    let view = coupons(settlement, maturity, frequency(freq)?, 0, date1904)?;
    Ok(f64::from(view.count))
}

pub(crate) fn price(
    settlement: f64,
    maturity: f64,
    rate: f64,
    yld: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || yld < 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let freq_i = frequency(freq)?;
    let view = coupons(settlement, maturity, freq_i, basis_of(basis)?, date1904)?;
    bond_price(rate, yld, redemption, f64::from(freq_i), &view)
}

fn bond_price(
    rate: f64,
    yld: f64,
    redemption: f64,
    freq: f64,
    view: &Coupon,
) -> Result<f64, &'static str> {
    let coupon = 100.0 * rate / freq;
    let base = 1.0 + yld / freq;
    if base <= 0.0 || view.length == 0.0 {
        return Err(NUM);
    }
    let mut present = 0.0;
    for k in 1..=view.count {
        let exponent = f64::from(k - 1) + view.after / view.length;
        let cash = if k == view.count {
            coupon + redemption
        } else {
            coupon
        };
        present += cash / base.powf(exponent);
    }
    finite(present - coupon * view.before / view.length)
}

pub(crate) fn yield_bond(
    settlement: f64,
    maturity: f64,
    rate: f64,
    pr: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if pr <= 0.0 {
        return Err(NUM);
    }
    let freq_i = frequency(freq)?;
    let basis_i = basis_of(basis)?;
    let view = coupons(settlement, maturity, freq_i, basis_i, date1904)?;
    solve(rate, |yld| {
        if yld < 0.0 {
            return None;
        }
        bond_price(rate, yld, redemption, f64::from(freq_i), &view)
            .ok()
            .map(|price| price - pr)
    })
}

pub(crate) fn duration(
    settlement: f64,
    maturity: f64,
    rate: f64,
    yld: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || yld < 0.0 {
        return Err(NUM);
    }
    let freq_i = frequency(freq)?;
    let view = coupons(settlement, maturity, freq_i, basis_of(basis)?, date1904)?;
    let freq = f64::from(freq_i);
    let coupon = 100.0 * rate / freq;
    let base = 1.0 + yld / freq;
    let mut weighted = 0.0;
    let mut present = 0.0;
    for k in 1..=view.count {
        let time = f64::from(k - 1) + view.after / view.length;
        let cash = if k == view.count {
            coupon + 100.0
        } else {
            coupon
        };
        let pv = cash / base.powf(time);
        weighted += time * pv;
        present += pv;
    }
    if present == 0.0 {
        return Err(NUM);
    }
    finite(weighted / present / freq)
}

pub(crate) fn mduration(
    settlement: f64,
    maturity: f64,
    rate: f64,
    yld: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    let mac = duration(settlement, maturity, rate, yld, freq, basis, date1904)?;
    let freq = f64::from(frequency(freq)?);
    finite(mac / (1.0 + yld / freq))
}

fn year_span(start: f64, end: f64, basis: i32, date1904: bool) -> Result<f64, &'static str> {
    let days = spanned(start, end, basis, date1904)?;
    let denom = match basis {
        0 | 2 | 4 => 360.0,
        3 => 365.0,
        1 => year_length(start, end, date1904)?,
        _ => return Err(NUM),
    };
    Ok(days / denom)
}

pub(crate) fn pricedisc(
    settlement: f64,
    maturity: f64,
    discount: f64,
    redemption: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if discount <= 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let span = year_span(settlement, maturity, basis_of(basis)?, date1904)?;
    finite(redemption * (1.0 - discount * span))
}

pub(crate) fn yielddisc(
    settlement: f64,
    maturity: f64,
    pr: f64,
    redemption: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if pr <= 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let span = year_span(settlement, maturity, basis_of(basis)?, date1904)?;
    if span == 0.0 {
        return Err(NUM);
    }
    finite((redemption - pr) / pr / span)
}

pub(crate) fn disc(
    settlement: f64,
    maturity: f64,
    pr: f64,
    redemption: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if pr <= 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let span = year_span(settlement, maturity, basis_of(basis)?, date1904)?;
    if span == 0.0 {
        return Err(NUM);
    }
    finite((redemption - pr) / redemption / span)
}

pub(crate) fn intrate(
    settlement: f64,
    maturity: f64,
    investment: f64,
    redemption: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    yielddisc(
        settlement, maturity, investment, redemption, basis, date1904,
    )
}

pub(crate) fn received(
    settlement: f64,
    maturity: f64,
    investment: f64,
    discount: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if investment <= 0.0 || discount <= 0.0 {
        return Err(NUM);
    }
    let span = year_span(settlement, maturity, basis_of(basis)?, date1904)?;
    let denom = 1.0 - discount * span;
    if denom == 0.0 {
        return Err(NUM);
    }
    finite(investment / denom)
}

pub(crate) fn pricemat(
    settlement: f64,
    maturity: f64,
    issue: f64,
    rate: f64,
    yld: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || yld < 0.0 {
        return Err(NUM);
    }
    let basis = basis_of(basis)?;
    later(issue, settlement)?;
    later(settlement, maturity)?;
    let dim = year_span(issue, maturity, basis, date1904)?;
    let dsm = year_span(settlement, maturity, basis, date1904)?;
    let a = year_span(issue, settlement, basis, date1904)?;
    let denom = 1.0 + yld * dsm;
    if denom == 0.0 {
        return Err(NUM);
    }
    finite((100.0 + dim * rate * 100.0) / denom - a * rate * 100.0)
}

pub(crate) fn yieldmat(
    settlement: f64,
    maturity: f64,
    issue: f64,
    rate: f64,
    pr: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || pr <= 0.0 {
        return Err(NUM);
    }
    let basis = basis_of(basis)?;
    later(issue, settlement)?;
    later(settlement, maturity)?;
    let dim = year_span(issue, maturity, basis, date1904)?;
    let dsm = year_span(settlement, maturity, basis, date1904)?;
    let a = year_span(issue, settlement, basis, date1904)?;
    if dsm == 0.0 {
        return Err(NUM);
    }
    finite(((100.0 + dim * rate * 100.0) / (pr + a * rate * 100.0) - 1.0) / dsm)
}

pub(crate) fn accrint(
    issue: f64,
    first: f64,
    settlement: f64,
    rate: f64,
    par: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || par <= 0.0 {
        return Err(NUM);
    }
    let freq = frequency(freq)?;
    let basis = basis_of(basis)?;
    later(issue, settlement)?;
    let _ = first;
    let accrued = spanned(issue, settlement, basis, date1904)?;
    let length = period_length(
        issue,
        shift(issue, 12 / freq, false, date1904)?,
        freq,
        basis,
        date1904,
    )?;
    if length == 0.0 {
        return Err(NUM);
    }
    finite(par * rate / f64::from(freq) * accrued / length)
}

pub(crate) fn accrintm(
    issue: f64,
    settlement: f64,
    rate: f64,
    par: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || par <= 0.0 {
        return Err(NUM);
    }
    let basis = basis_of(basis)?;
    let span = year_span(issue, settlement, basis, date1904)?;
    finite(par * rate * span)
}

fn tbill_days(settlement: f64, maturity: f64) -> Result<f64, &'static str> {
    later(settlement, maturity)?;
    let days = day(maturity) - day(settlement);
    if days > 365.0 {
        return Err(NUM);
    }
    Ok(days)
}

pub(crate) fn tbilleq(settlement: f64, maturity: f64, discount: f64) -> Result<f64, &'static str> {
    if discount <= 0.0 {
        return Err(NUM);
    }
    let days = tbill_days(settlement, maturity)?;
    if days <= 182.0 {
        return finite(365.0 * discount / (360.0 - discount * days));
    }
    let price = 100.0 * (1.0 - discount * days / 360.0);
    let term = days / 365.0;
    let inside = term * term - (2.0 * term - 1.0) * (1.0 - 1.0 / (price / 100.0));
    if inside < 0.0 {
        return Err(NUM);
    }
    finite((-2.0 * term + 2.0 * inside.sqrt()) / (2.0 * term - 1.0))
}

pub(crate) fn tbillprice(
    settlement: f64,
    maturity: f64,
    discount: f64,
) -> Result<f64, &'static str> {
    if discount <= 0.0 {
        return Err(NUM);
    }
    let days = tbill_days(settlement, maturity)?;
    finite(100.0 * (1.0 - discount * days / 360.0))
}

pub(crate) fn tbillyield(settlement: f64, maturity: f64, pr: f64) -> Result<f64, &'static str> {
    if pr <= 0.0 {
        return Err(NUM);
    }
    let days = tbill_days(settlement, maturity)?;
    finite((100.0 - pr) / pr * 360.0 / days)
}

pub(crate) fn oddfprice(
    settlement: f64,
    maturity: f64,
    issue: f64,
    first: f64,
    rate: f64,
    yld: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || yld < 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let freq = frequency(freq)?;
    let basis = basis_of(basis)?;
    if !(day(maturity) > day(first) && day(first) > day(settlement) && day(settlement) > day(issue))
    {
        return Err(NUM);
    }
    let freq_f = f64::from(freq);
    let eom = end_of_month(first, date1904)?;
    let quasi = shift(first, -12 / freq, eom, date1904)?;
    let short = day(issue) >= day(quasi);
    let base = 1.0 + yld / freq_f;
    let coupon = 100.0 * rate / freq_f;
    if short {
        let early = coupons(first, maturity, freq, basis, date1904)?;
        let count = early.count + 1;
        let length = period_length(quasi, first, freq, basis, date1904)?;
        let dfc = spanned(issue, first, basis, date1904)?;
        let dsc = spanned(settlement, first, basis, date1904)?;
        let accrued_days = spanned(issue, settlement, basis, date1904)?;
        let mut present = redemption / base.powf(f64::from(count - 1) + dsc / length);
        present += coupon * dfc / length / base.powf(dsc / length);
        for k in 2..=count {
            present += coupon / base.powf(f64::from(k - 1) + dsc / length);
        }
        return finite(present - coupon * accrued_days / length);
    }
    odd_long_first(
        settlement, maturity, issue, first, rate, yld, redemption, freq, basis, date1904,
    )
}

fn odd_long_first(
    settlement: f64,
    maturity: f64,
    issue: f64,
    first: f64,
    rate: f64,
    yld: f64,
    redemption: f64,
    freq: i32,
    basis: i32,
    date1904: bool,
) -> Result<f64, &'static str> {
    let freq_f = f64::from(freq);
    let eom = end_of_month(first, date1904)?;
    let step = 12 / freq;
    let mut points = vec![day(first)];
    let mut guard = 0;
    while day(points[0]) > day(issue) {
        let prev = shift(points[0], -step, eom, date1904)?;
        points.insert(0, prev);
        guard += 1;
        if guard > 1000 {
            return Err(NUM);
        }
    }
    let base = 1.0 + yld / freq_f;
    let coupon = 100.0 * rate / freq_f;
    let mut odd_ratio = 0.0;
    let mut accrued_ratio = 0.0;
    for pair in points.windows(2) {
        let quasi = pair[0];
        let start = quasi.max(day(issue));
        let end = pair[1].min(day(first));
        if start >= end {
            continue;
        }
        let length = period_length(
            quasi,
            shift(quasi, step, eom, date1904)?,
            freq,
            basis,
            date1904,
        )?;
        let span = spanned(start, end, basis, date1904)?;
        odd_ratio += span / length;
        if day(settlement) >= end {
            accrued_ratio += span / length;
        } else if day(settlement) > start {
            accrued_ratio += spanned(start, settlement, basis, date1904)? / length;
        }
    }
    let after_first = coupons(first, maturity, freq, basis, date1904)?;
    let n = after_first.count;
    let nq = points
        .iter()
        .filter(|date| **date > day(settlement) && **date < day(first))
        .count();
    let length = period_length(points[points.len() - 2], first, freq, basis, date1904)?;
    let dsc = spanned(
        settlement,
        points
            .iter()
            .copied()
            .find(|date| *date > day(settlement))
            .unwrap_or(first),
        basis,
        date1904,
    )?;
    let mut present = redemption / base.powf(f64::from(n + nq as i32) + dsc / length);
    present += coupon * odd_ratio / base.powf(f64::from(nq as i32) + dsc / length);
    for k in 1..=n {
        present += coupon / base.powf(f64::from(k + nq as i32) + dsc / length);
    }
    finite(present - coupon * accrued_ratio)
}

pub(crate) fn oddfyield(
    settlement: f64,
    maturity: f64,
    issue: f64,
    first: f64,
    rate: f64,
    pr: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if pr <= 0.0 {
        return Err(NUM);
    }
    solve(rate, |yld| {
        oddfprice(
            settlement, maturity, issue, first, rate, yld, redemption, freq, basis, date1904,
        )
        .ok()
        .map(|price| price - pr)
    })
}

pub(crate) fn oddlprice(
    settlement: f64,
    maturity: f64,
    last: f64,
    rate: f64,
    yld: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if rate < 0.0 || yld < 0.0 || redemption <= 0.0 {
        return Err(NUM);
    }
    let freq = frequency(freq)?;
    let basis = basis_of(basis)?;
    if !(day(maturity) > day(settlement) && day(settlement) > day(last)) {
        return Err(NUM);
    }
    let (future, accrued) = odd_last_ratios(last, settlement, maturity, freq, basis, date1904)?;
    let freq_f = f64::from(freq);
    let coupon = 100.0 * rate / freq_f;
    let denom = 1.0 + yld / freq_f * (future - accrued);
    if denom <= 0.0 {
        return Err(NUM);
    }
    finite((redemption + coupon * future) / denom - coupon * accrued)
}

fn odd_last_ratios(
    last: f64,
    settlement: f64,
    maturity: f64,
    freq: i32,
    basis: i32,
    date1904: bool,
) -> Result<(f64, f64), &'static str> {
    let eom = end_of_month(last, date1904)?;
    let step = 12 / freq;
    let mut points = vec![day(last)];
    let mut guard = 0;
    while *points.last().unwrap_or(&0.0) < day(maturity) - 1e-9 {
        let next = shift(*points.last().unwrap_or(&last), step, eom, date1904)?;
        if next >= day(maturity) {
            points.push(day(maturity));
            break;
        }
        points.push(next);
        guard += 1;
        if guard > 1000 {
            return Err(NUM);
        }
    }
    if *points.last().unwrap_or(&0.0) < day(maturity) - 1e-9 {
        points.push(day(maturity));
    }
    let mut future = 0.0;
    let mut accrued = 0.0;
    for pair in points.windows(2) {
        let length = period_length(
            pair[0],
            shift(pair[0], step, eom, date1904)?,
            freq,
            basis,
            date1904,
        )?;
        let span = spanned(pair[0], pair[1], basis, date1904)?;
        future += span / length;
        if day(settlement) >= pair[1] {
            accrued += span / length;
        } else if day(settlement) > pair[0] {
            accrued += spanned(pair[0], settlement, basis, date1904)? / length;
        }
    }
    Ok((future, accrued))
}

pub(crate) fn oddlyield(
    settlement: f64,
    maturity: f64,
    last: f64,
    rate: f64,
    pr: f64,
    redemption: f64,
    freq: f64,
    basis: f64,
    date1904: bool,
) -> Result<f64, &'static str> {
    if pr <= 0.0 {
        return Err(NUM);
    }
    solve(rate, |yld| {
        oddlprice(
            settlement, maturity, last, rate, yld, redemption, freq, basis, date1904,
        )
        .ok()
        .map(|price| price - pr)
    })
}

fn solve(guess: f64, mut f: impl FnMut(f64) -> Option<f64>) -> Result<f64, &'static str> {
    let mut x = if guess.is_finite() && guess > -0.99 {
        guess
    } else {
        0.1
    };
    if x == 0.0 {
        x = 0.1;
    }
    for _ in 0..80 {
        let y = f(x).ok_or(NUM)?;
        if !y.is_finite() {
            return Err(NUM);
        }
        if y.abs() < 1e-10 {
            return finite(x);
        }
        let step = (x.abs() * 1e-6).max(1e-8);
        let y2 = f(x + step).ok_or(NUM)?;
        let slope = (y2 - y) / step;
        if !slope.is_finite() || slope.abs() < 1e-14 {
            return Err(NUM);
        }
        let next = x - y / slope;
        if !next.is_finite() || next <= -0.999999 {
            return Err(NUM);
        }
        if (next - x).abs() <= 1e-12 {
            return finite(next);
        }
        x = next;
    }
    Err(NUM)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: Result<f64, &'static str>, expect: f64) {
        let got = got.unwrap_or_else(|err| panic!("{err}"));
        let tol = 1e-6_f64.max(expect.abs() * 1e-8);
        assert!((got - expect).abs() < tol, "{got} vs {expect}");
    }

    fn date(y: i32, m: u32, d: u32) -> f64 {
        excel_serial(y, m, d, false)
    }

    #[test]
    fn annuities_match_excel_16() {
        let monthly = 0.08 / 12.0;
        near(pv(monthly, 240.0, 500.0, 0.0, 0.0), -59777.14585118782);
        near(pv(monthly, 240.0, 500.0, 0.0, 1.0), -60175.660156862396);
        near(pv(0.0, 12.0, 100.0, 0.0, 0.0), -1200.0);
        near(fv(monthly, 120.0, -100.0, 0.0, 0.0), 18294.603518170716);
        near(fv(0.0, 12.0, -100.0, 0.0, 0.0), 1200.0);
        near(pmt(monthly, 120.0, 10000.0, 0.0, 0.0), -121.32759435535694);
        near(pmt(0.0, 12.0, 1200.0, 0.0, 0.0), -100.0);
        near(
            ipmt(monthly, 1.0, 120.0, 10000.0, 0.0, 0.0),
            -66.66666666666667,
        );
        near(ipmt(monthly, 1.0, 120.0, 10000.0, 0.0, 1.0), 0.0);
        near(
            ppmt(monthly, 1.0, 120.0, 10000.0, 0.0, 0.0),
            -54.66092768869026,
        );
        near(
            ppmt(monthly, 1.0, 120.0, 10000.0, 0.0, 1.0),
            -120.52410035300356,
        );
        near(nper(monthly, -100.0, 10000.0, 0.0, 0.0), 165.34054113030956);
        near(nper(0.0, -100.0, 1200.0, 0.0, 0.0), 12.0);
        near(
            rate(120.0, -100.0, 10000.0, 0.0, 0.0, 0.1),
            0.003114181946001214,
        );
        assert_eq!(rate(10.0, 100.0, 100.0, 0.0, 0.0, 0.1), Err(NUM));
        near(ispmt(monthly, 1.0, 120.0, 10000.0), -66.11111111111111);
        near(
            cum_ipmt(monthly, 120.0, 10000.0, 1.0, 12.0, 0.0),
            -775.4066262939383,
        );
        near(
            cum_princ(monthly, 120.0, 10000.0, 1.0, 12.0, 0.0),
            -680.5245059703449,
        );
        assert_eq!(cum_ipmt(monthly, 120.0, 10000.0, 13.0, 1.0, 0.0), Err(NUM));
        assert_eq!(ipmt(0.1, 0.0, 10.0, 1000.0, 0.0, 0.0), Err(NUM));
    }

    #[test]
    fn cashflows_match_excel_16() {
        near(
            npv(0.1, &[-10000.0, 3000.0, 4200.0, 6800.0]),
            1188.4434123352207,
        );
        near(irr(&[-100.0, 60.0, 60.0], 0.1), 0.1306623862918075);
        near(mirr(&[-100.0, 60.0, 60.0], 0.1, 0.12), 0.12782977438973497);
        let dates = [date(2018, 1, 1), date(2018, 6, 1), date(2019, 1, 1)];
        near(xnpv(0.09, &[-100.0, 60.0, 60.0], &dates), 12.94445781868918);
        near(xirr(&[-100.0, 60.0, 60.0], &dates, 0.1), 0.2996664583683014);
        near(fvschedule(1000.0, &[0.1, 0.05]), 1155.0);
        near(effect(0.1, 4.0), 0.10381289062499977);
        near(nominal(0.1, 4.0), 0.09645475633778045);
        assert_eq!(effect(0.1, 0.0), Err(NUM));
        near(dollarde(1.02, 16.0), 1.125);
        near(dollarfr(1.125, 16.0), 1.02);
        assert_eq!(dollarde(1.1, 0.0), Err(DIV0));
    }

    #[test]
    fn depreciation_match_excel_16() {
        near(sln(10000.0, 1000.0, 5.0), 1800.0);
        near(sln(1000.0, 2000.0, 5.0), -200.0);
        assert_eq!(sln(1000.0, 100.0, 0.0), Err(DIV0));
        near(syd(10000.0, 1000.0, 5.0, 1.0), 3000.0);
        assert_eq!(syd(10000.0, 1000.0, 5.0, 0.0), Err(NUM));
        near(db(10000.0, 1000.0, 5.0, 1.0, 12.0), 3690.0);
        near(db(10000.0, 1000.0, 5.0, 1.0, 6.0), 1845.0);
        near(db(10000.0, 1000.0, 5.0, 6.0, 6.0), 238.52712458788181);
        assert_eq!(db(10000.0, 1000.0, 5.0, 6.0, 12.0), Err(NUM));
        near(ddb(10000.0, 1000.0, 5.0, 1.0, 2.0), 4000.0);
        near(ddb(10000.0, 1000.0, 5.0, 1.0, 1.5), 3000.0);
        near(ddb(10000.0, 1000.0, 5.0, 5.0, 2.0), 296.0);
        near(vdb(10000.0, 1000.0, 5.0, 0.0, 1.0, 2.0, false), 4000.0);
        near(
            vdb(2400.0, 300.0, 3650.0, 0.0, 1.0, 2.0, false),
            1.3150684931506849,
        );
        near(
            vdb(2400.0, 300.0, 120.0, 6.0, 18.0, 2.0, false),
            396.30605326475086,
        );
        let bought = date(2018, 1, 1);
        let first = date(2018, 4, 1);
        near(
            amorlinc(2400.0, bought, first, 300.0, 0.0, 0.15, 0, false),
            90.0,
        );
        near(
            amorlinc(2400.0, bought, first, 300.0, 1.0, 0.15, 0, false),
            360.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 0.0, 0.15, 0, false),
            225.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 1.0, 0.15, 0, false),
            816.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 2.0, 0.15, 0, false),
            510.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 3.0, 0.15, 0, false),
            319.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 4.0, 0.15, 0, false),
            199.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 5.0, 0.15, 0, false),
            166.0,
        );
        near(
            amordegrc(2400.0, bought, first, 300.0, 6.0, 0.15, 0, false),
            0.0,
        );
    }

    #[test]
    fn bonds_match_excel_16() {
        let settle = date(2018, 4, 1);
        let mature = date(2021, 1, 1);
        let issue = date(2018, 1, 1);
        near(coup_daybs(settle, mature, 2.0, 0.0, false), 90.0);
        near(coup_days(settle, mature, 2.0, 0.0, false), 180.0);
        near(coup_days(settle, mature, 2.0, 1.0, false), 181.0);
        near(coup_days(settle, mature, 2.0, 3.0, false), 182.5);
        near(coup_daysnc(settle, mature, 2.0, 0.0, false), 90.0);
        near(coup_num(settle, mature, 2.0, 0.0, false), 6.0);
        near(coup_num(settle, mature, 1.0, 0.0, false), 3.0);
        near(coup_num(settle, mature, 4.0, 0.0, false), 11.0);
        assert_eq!(coup_days(settle, mature, 3.0, 0.0, false), Err(NUM));
        near(coup_ncd(settle, mature, 2.0, 0.0, false), date(2018, 7, 1));
        near(coup_pcd(settle, mature, 2.0, 0.0, false), date(2018, 1, 1));
        near(
            coup_ncd(date(2018, 4, 15), date(2021, 3, 31), 2.0, 0.0, false),
            date(2018, 9, 30),
        );
        near(
            coup_pcd(date(2018, 4, 15), date(2021, 3, 31), 2.0, 0.0, false),
            date(2018, 3, 31),
        );
        near(
            price(settle, mature, 0.08, 0.09, 100.0, 2.0, 0.0, false),
            97.5889177004159,
        );
        near(
            yield_bond(settle, mature, 0.08, 95.0, 100.0, 2.0, 0.0, false),
            0.10117660926247074,
        );
        near(
            duration(settle, mature, 0.08, 0.09, 2.0, 0.0, false),
            2.4717014098798824,
        );
        near(
            mduration(settle, mature, 0.08, 0.09, 2.0, 0.0, false),
            2.3652645070620886,
        );
        near(pricedisc(settle, mature, 0.05, 100.0, 0.0, false), 86.25);
        near(
            yielddisc(settle, mature, 95.0, 100.0, 0.0, false),
            0.019138755980861243,
        );
        near(
            disc(settle, mature, 95.0, 100.0, 0.0, false),
            0.0181818181818182,
        );
        near(
            intrate(settle, mature, 95.0, 100.0, 0.0, false),
            0.019138755980861243,
        );
        near(
            received(settle, mature, 1000.0, 0.05, 0.0, false),
            1159.4202898550725,
        );
        near(
            pricemat(settle, mature, issue, 0.08, 0.09, 0.0, false),
            97.39879759519037,
        );
        near(
            yieldmat(settle, mature, issue, 0.08, 95.0, 0.0, false),
            0.10121836925960638,
        );
        near(
            accrint(
                issue,
                date(2018, 7, 1),
                settle,
                0.1,
                1000.0,
                2.0,
                0.0,
                false,
            ),
            25.0,
        );
        near(
            accrint(
                issue,
                date(2018, 7, 1),
                settle,
                0.1,
                1000.0,
                2.0,
                1.0,
                false,
            ),
            24.861878453038674,
        );
        near(accrintm(issue, settle, 0.1, 1000.0, 0.0, false), 25.0);
        assert_eq!(
            price(settle, date(2018, 3, 1), 0.08, 0.09, 100.0, 2.0, 0.0, false),
            Err(NUM)
        );
        assert_eq!(
            price(settle, mature, 0.08, 0.09, 100.0, 2.0, 5.0, false),
            Err(NUM)
        );
        let bill_end = date(2018, 6, 30);
        near(tbilleq(issue, bill_end, 0.09), 0.09554973821989529);
        near(tbillprice(issue, bill_end, 0.09), 95.5);
        near(tbillyield(issue, bill_end, 98.0), 0.04081632653061224);
        near(tbilleq(issue, date(2019, 1, 1), 0.09), 0.09801111030950427);
        assert_eq!(tbilleq(issue, date(2019, 1, 2), 0.09), Err(NUM));
    }

    #[test]
    fn odd_coupons_match_excel_16() {
        let settle = date(2008, 11, 11);
        let mature = date(2021, 3, 1);
        let issue = date(2008, 10, 15);
        let first = date(2009, 3, 1);
        near(
            oddfprice(
                settle, mature, issue, first, 0.0785, 0.0625, 100.0, 2.0, 1.0, false,
            ),
            113.59771747407883,
        );
        near(
            oddfyield(
                settle, mature, issue, first, 0.0575, 84.5, 100.0, 2.0, 0.0, false,
            ),
            0.07724554159729888,
        );
        near(
            oddfprice(
                settle,
                mature,
                date(2008, 8, 15),
                first,
                0.0785,
                0.0625,
                100.0,
                2.0,
                1.0,
                false,
            ),
            113.57332119825286,
        );
        near(
            oddlprice(
                settle, mature, issue, 0.0785, 0.0625, 100.0, 2.0, 1.0, false,
            ),
            110.8745242841996,
        );
        near(
            oddlyield(settle, mature, issue, 0.0575, 84.5, 100.0, 2.0, 0.0, false),
            0.08254808631495333,
        );
        near(
            oddlprice(
                date(2020, 12, 1),
                date(2021, 2, 15),
                date(2020, 9, 1),
                0.0785,
                0.0625,
                100.0,
                2.0,
                1.0,
                false,
            ),
            100.30600306183553,
        );
    }
}
