//! 统计函数的数值部分。对照 Excel 16.0 的缓存，不另引统计库。

const DIV0: &str = "#DIV/0!";
const NUM: &str = "#NUM!";
const NA: &str = "#N/A";

pub(crate) fn average(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.is_empty() {
        return Err(DIV0);
    }
    finite(nums.iter().sum::<f64>() / nums.len() as f64)
}

pub(crate) fn geomean(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.is_empty() || nums.iter().any(|n| *n <= 0.0) {
        return Err(NUM);
    }
    let sum = nums.iter().map(|n| n.ln()).sum::<f64>();
    finite((sum / nums.len() as f64).exp())
}

pub(crate) fn harmean(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.is_empty() || nums.iter().any(|n| *n <= 0.0) {
        return Err(NUM);
    }
    let sum = nums.iter().map(|n| 1.0 / n).sum::<f64>();
    finite(nums.len() as f64 / sum)
}

pub(crate) fn median(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.is_empty() {
        return Err(NUM);
    }
    let sorted = sorted(nums);
    let n = sorted.len();
    if n % 2 == 1 {
        Ok(sorted[n / 2])
    } else {
        Ok((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

pub(crate) fn mode(nums: &[f64]) -> Result<f64, &'static str> {
    let mut best: Option<(f64, usize)> = None;
    for &x in nums {
        let count = nums.iter().filter(|y| (*y - x).abs() < 1e-9).count();
        if count < 2 {
            continue;
        }
        match best {
            Some((_, seen)) if count <= seen => {}
            _ => best = Some((x, count)),
        }
    }
    best.map(|(value, _)| value).ok_or(NA)
}

pub(crate) fn trimmean(nums: &[f64], percent: f64) -> Result<f64, &'static str> {
    if nums.is_empty() || !(0.0..1.0).contains(&percent) {
        return Err(NUM);
    }
    let sorted = sorted(nums);
    let mut drop_n = (percent * sorted.len() as f64).floor() as usize;
    if drop_n % 2 == 1 {
        drop_n -= 1;
    }
    let each = drop_n / 2;
    if each * 2 >= sorted.len() {
        return Err(NUM);
    }
    average(&sorted[each..sorted.len() - each])
}

pub(crate) fn avedev(nums: &[f64]) -> Result<f64, &'static str> {
    let mean = average(nums)?;
    finite(nums.iter().map(|n| (n - mean).abs()).sum::<f64>() / nums.len() as f64)
}

pub(crate) fn devsq(nums: &[f64]) -> Result<f64, &'static str> {
    let mean = average(nums)?;
    finite(nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>())
}

pub(crate) fn variance(nums: &[f64], sample: bool) -> Result<f64, &'static str> {
    let denom = nums.len() as f64 - if sample { 1.0 } else { 0.0 };
    if denom <= 0.0 {
        return Err(DIV0);
    }
    let mean = average(nums)?;
    finite(nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / denom)
}

pub(crate) fn stdev(nums: &[f64], sample: bool) -> Result<f64, &'static str> {
    Ok(variance(nums, sample)?.sqrt())
}

pub(crate) fn percentile(nums: &[f64], p: f64) -> Result<f64, &'static str> {
    if nums.is_empty() || !(0.0..=1.0).contains(&p) {
        return Err(NUM);
    }
    let sorted = sorted(nums);
    if sorted.len() == 1 {
        return Ok(sorted[0]);
    }
    let k = p * (sorted.len() - 1) as f64;
    let i = k.floor() as usize;
    let frac = k - i as f64;
    if i + 1 >= sorted.len() {
        return Ok(sorted[sorted.len() - 1]);
    }
    Ok(sorted[i] + frac * (sorted[i + 1] - sorted[i]))
}

pub(crate) fn quartile(nums: &[f64], quart: f64) -> Result<f64, &'static str> {
    if quart.fract() != 0.0 || !(0.0..=4.0).contains(&quart) {
        return Err(NUM);
    }
    percentile(nums, quart / 4.0)
}

pub(crate) fn percent_rank(nums: &[f64], x: f64, digits: i32) -> Result<f64, &'static str> {
    if nums.len() < 2 || digits < 1 {
        return Err(NUM);
    }
    let sorted = sorted(nums);
    if x < sorted[0] || x > sorted[sorted.len() - 1] {
        return Err(NA);
    }
    let n = sorted.len();
    let mut raw = 1.0;
    for i in 0..n - 1 {
        if (x - sorted[i]).abs() < 1e-12 {
            raw = i as f64 / (n - 1) as f64;
            break;
        }
        if x < sorted[i + 1] || i + 1 == n - 1 {
            let span = sorted[i + 1] - sorted[i];
            let frac = if span == 0.0 {
                0.0
            } else {
                (x - sorted[i]) / span
            };
            let lo = i as f64 / (n - 1) as f64;
            let hi = (i + 1) as f64 / (n - 1) as f64;
            raw = lo + frac * (hi - lo);
            break;
        }
    }
    Ok(sig_digits(raw, digits))
}

pub(crate) fn rank(x: f64, nums: &[f64], ascending: bool) -> Result<f64, &'static str> {
    if !nums.iter().any(|n| (n - x).abs() < 1e-9) {
        return Err(NA);
    }
    let better = if ascending {
        nums.iter().filter(|n| **n < x - 1e-9).count()
    } else {
        nums.iter().filter(|n| **n > x + 1e-9).count()
    };
    Ok((better + 1) as f64)
}

pub(crate) fn large(nums: &[f64], k: f64) -> Result<f64, &'static str> {
    order_stat(nums, k, true)
}

pub(crate) fn small(nums: &[f64], k: f64) -> Result<f64, &'static str> {
    order_stat(nums, k, false)
}

pub(crate) fn skew(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.len() < 3 {
        return Err(DIV0);
    }
    let n = nums.len() as f64;
    let mean = average(nums)?;
    let s = stdev(nums, true)?;
    if s == 0.0 {
        return Err(DIV0);
    }
    let sum = nums.iter().map(|x| ((x - mean) / s).powi(3)).sum::<f64>();
    finite(n / ((n - 1.0) * (n - 2.0)) * sum)
}

pub(crate) fn kurt(nums: &[f64]) -> Result<f64, &'static str> {
    if nums.len() < 4 {
        return Err(DIV0);
    }
    let n = nums.len() as f64;
    let mean = average(nums)?;
    let s = stdev(nums, true)?;
    if s == 0.0 {
        return Err(DIV0);
    }
    let sum = nums.iter().map(|x| ((x - mean) / s).powi(4)).sum::<f64>();
    let a = n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0)) * sum;
    let b = 3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0));
    finite(a - b)
}

pub(crate) fn standardize(x: f64, mean: f64, sd: f64) -> Result<f64, &'static str> {
    if sd <= 0.0 {
        return Err(NUM);
    }
    finite((x - mean) / sd)
}

pub(crate) struct Fit {
    pub slope: f64,
    pub intercept: f64,
}

pub(crate) fn lin_fit(xs: &[f64], ys: &[f64]) -> Result<Fit, &'static str> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return Err(NA);
    }
    let n = xs.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        sxx += (x - mx) * (x - mx);
        sxy += (x - mx) * (y - my);
    }
    if sxx == 0.0 {
        return Err(DIV0);
    }
    let slope = sxy / sxx;
    Ok(Fit {
        slope,
        intercept: my - slope * mx,
    })
}

pub(crate) fn correl(xs: &[f64], ys: &[f64]) -> Result<f64, &'static str> {
    let n = pair_len(xs, ys)?;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
        sxy += (x - mx) * (y - my);
    }
    if sxx == 0.0 || syy == 0.0 {
        return Err(DIV0);
    }
    finite(sxy / (sxx * syy).sqrt())
}

pub(crate) fn covar(xs: &[f64], ys: &[f64]) -> Result<f64, &'static str> {
    let n = pair_len(xs, ys)?;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut sum = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        sum += (x - mx) * (y - my);
    }
    finite(sum / n)
}

pub(crate) fn steyx(xs: &[f64], ys: &[f64]) -> Result<f64, &'static str> {
    if xs.len() < 3 {
        return Err(DIV0);
    }
    let fit = lin_fit(xs, ys)?;
    let mut sum = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        let err = y - (fit.intercept + fit.slope * x);
        sum += err * err;
    }
    finite((sum / (xs.len() as f64 - 2.0)).sqrt())
}

pub(crate) fn forecast(x: f64, xs: &[f64], ys: &[f64]) -> Result<f64, &'static str> {
    let fit = lin_fit(xs, ys)?;
    finite(fit.intercept + fit.slope * x)
}

pub(crate) fn log_fit(xs: &[f64], ys: &[f64]) -> Result<Fit, &'static str> {
    if ys.iter().any(|y| *y <= 0.0) {
        return Err(NUM);
    }
    let logs: Vec<f64> = ys.iter().map(|y| y.ln()).collect();
    let fit = lin_fit(xs, &logs)?;
    Ok(Fit {
        slope: fit.slope.exp(),
        intercept: fit.intercept.exp(),
    })
}

pub(crate) fn norms_dist(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

pub(crate) fn norms_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

pub(crate) fn norms_inv(p: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) {
        return Err(NUM);
    }
    // Peter J. Acklam 的有理逼近，再按正态密度做一次校正。
    let a = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    let b = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    let c = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    let d = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let q = p - 0.5;
    let mut x = if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p > 1.0 - plow {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else {
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    };
    let err = norms_dist(x) - p;
    let dens = norms_pdf(x);
    if dens > 0.0 {
        x -= err / dens;
    }
    finite(x)
}

pub(crate) fn norm_dist(x: f64, mean: f64, sd: f64, cum: bool) -> Result<f64, &'static str> {
    if sd <= 0.0 {
        return Err(NUM);
    }
    let z = (x - mean) / sd;
    if cum {
        finite(norms_dist(z))
    } else {
        finite(norms_pdf(z) / sd)
    }
}

pub(crate) fn norm_inv(p: f64, mean: f64, sd: f64) -> Result<f64, &'static str> {
    if sd <= 0.0 {
        return Err(NUM);
    }
    Ok(mean + sd * norms_inv(p)?)
}

pub(crate) fn confidence(alpha: f64, sd: f64, size: f64) -> Result<f64, &'static str> {
    if !(alpha > 0.0 && alpha < 1.0) || sd <= 0.0 || size < 1.0 {
        return Err(NUM);
    }
    let z = norms_inv(1.0 - alpha / 2.0)?;
    finite(z * sd / size.sqrt())
}

pub(crate) fn fisher(x: f64) -> Result<f64, &'static str> {
    if !(-1.0..1.0).contains(&x) {
        return Err(NUM);
    }
    finite(0.5 * ((1.0 + x) / (1.0 - x)).ln())
}

pub(crate) fn fisher_inv(y: f64) -> Result<f64, &'static str> {
    finite(y.tanh())
}

pub(crate) fn ln_gamma(z: f64) -> Result<f64, &'static str> {
    if z <= 0.0 && z.fract() == 0.0 {
        return Err(NUM);
    }
    finite(ln_gamma_raw(z))
}

pub(crate) fn gamma_dist(x: f64, alpha: f64, beta: f64, cum: bool) -> Result<f64, &'static str> {
    if alpha <= 0.0 || beta <= 0.0 || x < 0.0 {
        return Err(NUM);
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    if cum {
        finite(gamma_p(alpha, x / beta))
    } else {
        let log = -ln_gamma_raw(alpha) - alpha * beta.ln() + (alpha - 1.0) * x.ln() - x / beta;
        finite(log.exp())
    }
}

pub(crate) fn gamma_inv(p: f64, alpha: f64, beta: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) || alpha <= 0.0 || beta <= 0.0 {
        return Err(NUM);
    }
    let mut lo = 0.0;
    let mut hi = alpha * beta;
    if hi == 0.0 {
        hi = 1.0;
    }
    while gamma_p(alpha, hi / beta) < p {
        hi *= 2.0;
        if hi > 1e150 {
            return Err(NUM);
        }
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if gamma_p(alpha, mid / beta) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    finite(0.5 * (lo + hi))
}

pub(crate) fn beta_dist(x: f64, alpha: f64, beta: f64) -> Result<f64, &'static str> {
    if alpha <= 0.0 || beta <= 0.0 || !(0.0..=1.0).contains(&x) {
        return Err(NUM);
    }
    finite(beta_i(alpha, beta, x))
}

pub(crate) fn beta_inv(p: f64, alpha: f64, beta: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) || alpha <= 0.0 || beta <= 0.0 {
        return Err(NUM);
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if beta_i(alpha, beta, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    finite(0.5 * (lo + hi))
}

pub(crate) fn binom_dist(k: f64, n: f64, p: f64, cum: bool) -> Result<f64, &'static str> {
    if !(0.0..=1.0).contains(&p) || n < 0.0 || k < 0.0 {
        return Err(NUM);
    }
    let n = n.trunc() as u64;
    let k = k.trunc() as u64;
    if k > n {
        return Err(NUM);
    }
    if cum {
        let mut sum = 0.0;
        for i in 0..=k {
            sum += binom(n, i) * p.powi(i as i32) * (1.0 - p).powi((n - i) as i32);
        }
        finite(sum)
    } else {
        finite(binom(n, k) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32))
    }
}

pub(crate) fn crit_binom(n: f64, p: f64, alpha: f64) -> Result<f64, &'static str> {
    if !(0.0..=1.0).contains(&p) || !(0.0..=1.0).contains(&alpha) || n < 0.0 {
        return Err(NUM);
    }
    let n = n.trunc() as u64;
    for k in 0..=n {
        if binom_dist(k as f64, n as f64, p, true)? >= alpha {
            return Ok(k as f64);
        }
    }
    Ok(n as f64)
}

pub(crate) fn poisson(k: f64, lambda: f64, cum: bool) -> Result<f64, &'static str> {
    if lambda < 0.0 || k < 0.0 {
        return Err(NUM);
    }
    let k = k.trunc() as u32;
    let mut term = (-lambda).exp();
    if !cum {
        for i in 1..=k {
            term *= lambda / f64::from(i);
        }
        return finite(term);
    }
    let mut sum = term;
    for i in 1..=k {
        term *= lambda / f64::from(i);
        sum += term;
    }
    finite(sum)
}

pub(crate) fn neg_binom(failures: f64, success: f64, p: f64) -> Result<f64, &'static str> {
    if !(0.0..=1.0).contains(&p) || failures < 0.0 || success < 1.0 {
        return Err(NUM);
    }
    let f = failures.trunc() as u64;
    let s = success.trunc() as u64;
    finite(binom(f + s - 1, f) * p.powi(s as i32) * (1.0 - p).powi(f as i32))
}

pub(crate) fn expon_dist(x: f64, lambda: f64, cum: bool) -> Result<f64, &'static str> {
    if lambda <= 0.0 || x < 0.0 {
        return Err(NUM);
    }
    if cum {
        finite(1.0 - (-lambda * x).exp())
    } else {
        finite(lambda * (-lambda * x).exp())
    }
}

pub(crate) fn weibull(x: f64, alpha: f64, beta: f64, cum: bool) -> Result<f64, &'static str> {
    if alpha <= 0.0 || beta <= 0.0 || x < 0.0 {
        return Err(NUM);
    }
    let z = (x / beta).powf(alpha);
    if cum {
        finite(1.0 - (-z).exp())
    } else if x == 0.0 {
        if alpha == 1.0 {
            finite(alpha / beta)
        } else if alpha < 1.0 {
            Err(NUM)
        } else {
            Ok(0.0)
        }
    } else {
        finite((alpha / beta.powf(alpha)) * x.powf(alpha - 1.0) * (-z).exp())
    }
}

pub(crate) fn hypgeom(
    sample_s: f64,
    number_sample: f64,
    pop_s: f64,
    pop: f64,
) -> Result<f64, &'static str> {
    let k = trunc_u(sample_s)?;
    let n = trunc_u(number_sample)?;
    let big_k = trunc_u(pop_s)?;
    let big_n = trunc_u(pop)?;
    if n > big_n || big_k > big_n || k > n || k > big_k {
        return Err(NUM);
    }
    let rest = big_n - big_k;
    let need = n - k;
    if need > rest {
        return Err(NUM);
    }
    finite(binom(big_k, k) * binom(rest, need) / binom(big_n, n))
}

pub(crate) fn permut(n: f64, k: f64) -> Result<f64, &'static str> {
    let n = trunc_u(n)?;
    let k = trunc_u(k)?;
    if k > n {
        return Err(NUM);
    }
    let mut p = 1.0;
    for i in 0..k {
        p *= (n - i) as f64;
    }
    finite(p)
}

pub(crate) fn chi_dist(x: f64, df: f64) -> Result<f64, &'static str> {
    if x < 0.0 || df < 1.0 || df.fract() != 0.0 {
        return Err(NUM);
    }
    finite(1.0 - gamma_p(df / 2.0, x / 2.0))
}

pub(crate) fn chi_inv(p: f64, df: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) || df < 1.0 || df.fract() != 0.0 {
        return Err(NUM);
    }
    invert(|x| chi_dist(x, df).unwrap_or(1.0), p, 0.0, df.max(1.0))
}

pub(crate) fn f_dist(x: f64, df1: f64, df2: f64) -> Result<f64, &'static str> {
    if x < 0.0 || df1 < 1.0 || df2 < 1.0 {
        return Err(NUM);
    }
    let arg = df2 / (df2 + df1 * x);
    finite(beta_i(df2 / 2.0, df1 / 2.0, arg))
}

pub(crate) fn f_inv(p: f64, df1: f64, df2: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) || df1 < 1.0 || df2 < 1.0 {
        return Err(NUM);
    }
    invert(|x| f_dist(x, df1, df2).unwrap_or(1.0), p, 0.0, 1.0)
}

pub(crate) fn f_test(v1: f64, n1: usize, v2: f64, n2: usize) -> Result<f64, &'static str> {
    if n1 < 2 || n2 < 2 || v1 < 0.0 || v2 <= 0.0 {
        return Err(DIV0);
    }
    let f = v1 / v2;
    let mut p = 2.0 * f_dist(f, (n1 - 1) as f64, (n2 - 1) as f64)?;
    if p > 1.0 {
        p = 2.0 - p;
    }
    finite(p)
}

pub(crate) fn t_dist(t: f64, df: f64, tails: i32) -> Result<f64, &'static str> {
    if df < 1.0 || (tails != 1 && tails != 2) {
        return Err(NUM);
    }
    let x = t.abs();
    let arg = df / (df + x * x);
    let two = beta_i(df / 2.0, 0.5, arg);
    if tails == 2 {
        finite(two)
    } else {
        finite(two / 2.0)
    }
}

pub(crate) fn t_inv(p: f64, df: f64) -> Result<f64, &'static str> {
    if !(p > 0.0 && p < 1.0) || df < 1.0 {
        return Err(NUM);
    }
    invert(|x| t_dist(x, df, 2).unwrap_or(1.0), p, 0.0, 1.0)
}

pub(crate) fn log_norm(x: f64, mean: f64, sd: f64) -> Result<f64, &'static str> {
    if x <= 0.0 || sd <= 0.0 {
        return Err(NUM);
    }
    finite(norms_dist((x.ln() - mean) / sd))
}

pub(crate) fn log_inv(p: f64, mean: f64, sd: f64) -> Result<f64, &'static str> {
    if sd <= 0.0 {
        return Err(NUM);
    }
    finite((mean + sd * norms_inv(p)?).exp())
}

pub(crate) fn z_test(nums: &[f64], mu: f64, sigma: Option<f64>) -> Result<f64, &'static str> {
    if nums.is_empty() {
        return Err(NA);
    }
    let mean = average(nums)?;
    let sd = match sigma {
        Some(s) if s <= 0.0 => return Err(NUM),
        Some(s) => s,
        None => stdev(nums, true)?,
    };
    let z = (mean - mu) / (sd / (nums.len() as f64).sqrt());
    finite(1.0 - norms_dist(z))
}

pub(crate) fn chi_test(actual: &[f64], expected: &[f64], df: f64) -> Result<f64, &'static str> {
    if actual.len() != expected.len() || actual.is_empty() {
        return Err(NA);
    }
    if expected.contains(&0.0) || df < 1.0 {
        return Err(DIV0);
    }
    let stat = actual
        .iter()
        .zip(expected)
        .map(|(a, e)| (a - e).powi(2) / e)
        .sum::<f64>();
    chi_dist(stat, df)
}

fn order_stat(nums: &[f64], k: f64, large: bool) -> Result<f64, &'static str> {
    if nums.is_empty() || k < 1.0 {
        return Err(NUM);
    }
    let k = k.trunc() as usize;
    if k == 0 || k > nums.len() {
        return Err(NUM);
    }
    let mut sorted = sorted(nums);
    if large {
        sorted.reverse();
    }
    Ok(sorted[k - 1])
}

fn sorted(nums: &[f64]) -> Vec<f64> {
    let mut sorted = nums.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted
}

fn sig_digits(x: f64, digits: i32) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let exp = x.abs().log10().floor();
    let scale = 10f64.powf(f64::from(digits) - exp - 1.0);
    (x * scale).round() / scale
}

fn pair_len(xs: &[f64], ys: &[f64]) -> Result<f64, &'static str> {
    if xs.len() != ys.len() || xs.is_empty() {
        return Err(NA);
    }
    Ok(xs.len() as f64)
}

fn finite(n: f64) -> Result<f64, &'static str> {
    if n.is_finite() {
        Ok(n)
    } else {
        Err(NUM)
    }
}

fn trunc_u(n: f64) -> Result<u64, &'static str> {
    if n < 0.0 || n > u64::MAX as f64 {
        return Err(NUM);
    }
    Ok(n.trunc() as u64)
}

fn binom(n: u64, k: u64) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut c = 1.0;
    for i in 0..k {
        c *= (n - i) as f64;
        c /= (i + 1) as f64;
    }
    c
}

fn ln_gamma_raw(z: f64) -> f64 {
    if z < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * z).sin()).ln() - ln_gamma_raw(1.0 - z);
    }
    const COEF: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let z = z - 1.0;
    let mut x = COEF[0];
    for (i, coef) in COEF.iter().enumerate().skip(1) {
        x += coef / (z + i as f64);
    }
    let t = z + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

fn gamma_p(shape: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < shape + 1.0 {
        let mut term = 1.0 / shape;
        let mut sum = term;
        for n in 1..200 {
            term *= x / (shape + n as f64);
            sum += term;
            if term.abs() < sum.abs() * 1e-14 {
                break;
            }
        }
        (sum * (-x + shape * x.ln() - ln_gamma_raw(shape)).exp()).clamp(0.0, 1.0)
    } else {
        (1.0 - gamma_q(shape, x)).clamp(0.0, 1.0)
    }
}

fn gamma_q(a: f64, x: f64) -> f64 {
    let prefix = (-x + a * x.ln() - ln_gamma_raw(a)).exp();
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1e-30;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..200 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = b + an / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-14 {
            break;
        }
    }
    (prefix * h).clamp(0.0, 1.0)
}

fn beta_i(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let ln_bt =
        ln_gamma_raw(a + b) - ln_gamma_raw(a) - ln_gamma_raw(b) + a * x.ln() + b * (1.0 - x).ln();
    let front = ln_bt.exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        (front * beta_cf(a, b, x) / a).clamp(0.0, 1.0)
    } else {
        (1.0 - front * beta_cf(b, a, 1.0 - x) / b).clamp(0.0, 1.0)
    }
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..200 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-14 {
            break;
        }
    }
    h
}

fn invert(
    f: impl Fn(f64) -> f64,
    target: f64,
    mut lo: f64,
    mut hi: f64,
) -> Result<f64, &'static str> {
    while f(hi) > target {
        hi *= 2.0;
        if hi > 1e150 {
            return Err(NUM);
        }
        if hi == 0.0 {
            hi = 1.0;
        }
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(mid) > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    finite(0.5 * (lo + hi))
}

pub(crate) fn erf(x: f64) -> f64 {
    let sign = x.signum();
    let x = x.abs();
    if x == 0.0 {
        return 0.0;
    }
    if x > 6.0 {
        return sign;
    }
    let mut sum = 0.0;
    let mut term = x;
    for n in 0..120 {
        sum += term / (2 * n + 1) as f64;
        term *= -x * x / (n + 1) as f64;
        if term.abs() < 1e-18 * sum.abs().max(1.0) {
            break;
        }
    }
    sign * 2.0 * sum / std::f64::consts::PI.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: Result<f64, &'static str>, expect: f64) {
        let got = got.unwrap();
        assert!((got - expect).abs() < 1e-8, "got {got} expect {expect}");
    }

    #[test]
    fn excel_16_distribution_samples() {
        let data = [1.0, 2.0, 3.0, 4.0, 10.0];
        near(beta_dist(0.5, 2.0, 3.0), 0.6875);
        near(beta_inv(0.5, 2.0, 3.0), 0.3857275681323895);
        near(binom_dist(3.0, 5.0, 0.5, true), 0.8125);
        near(binom_dist(3.0, 5.0, 0.5, false), 0.3125);
        near(chi_dist(3.0, 2.0), 0.22313016014842982);
        near(chi_inv(0.05, 2.0), 5.991464547107982);
        near(confidence(0.05, 1.5, 30.0), 0.536758243115147);
        near(crit_binom(10.0, 0.5, 0.9), 7.0);
        near(expon_dist(1.0, 2.0, true), 0.8646647167633873);
        near(expon_dist(1.0, 2.0, false), 0.2706705664732254);
        near(f_dist(2.0, 3.0, 4.0), 0.2563871975281761);
        near(f_inv(0.05, 3.0, 4.0), 6.591382116425579);
        near(fisher(0.5), 0.5493061443340549);
        near(fisher_inv(0.5), 0.46211715726000974);
        near(gamma_dist(2.0, 1.0, 1.0, true), 0.8646647167633873);
        near(gamma_dist(2.0, 1.0, 1.0, false), 0.13533528323661267);
        near(gamma_inv(0.5, 1.0, 1.0), std::f64::consts::LN_2);
        near(ln_gamma(3.0), std::f64::consts::LN_2);
        near(hypgeom(1.0, 4.0, 4.0, 12.0), 0.45252525252525244);
        near(kurt(&data), 3.1519999999999992);
        near(log_inv(0.5, 0.0, 1.0), 1.0);
        near(log_norm(1.0, 0.0, 1.0), 0.5);
        near(neg_binom(2.0, 3.0, 0.5), 0.1875);
        near(norm_dist(0.0, 0.0, 1.0, true), 0.5);
        near(norm_dist(1.0, 0.0, 1.0, false), 0.24197072451914337);
        near(norm_inv(0.5, 0.0, 1.0), 0.0);
        near(Ok(norms_dist(0.0)), 0.5);
        near(norms_inv(0.5), 0.0);
        near(permut(5.0, 2.0), 20.0);
        near(poisson(2.0, 1.0, true), 0.9196986029286058);
        near(poisson(2.0, 1.0, false), 0.18393972058572114);
        near(skew(&data), 1.697056274847714);
        near(standardize(10.0, 4.0, 2.0), 3.0);
        near(t_dist(2.0, 5.0, 2), 0.10193947882985836);
        near(t_inv(0.05, 5.0), 2.570581835636315);
        near(weibull(2.0, 1.0, 1.0, true), 0.8646647167633873);
        near(weibull(2.0, 1.0, 1.0, false), 0.1353352832366127);
        near(z_test(&data, 4.0, None), 0.5);
        let v1 = variance(&data, true).unwrap();
        let paired = [2.0, 4.0, 6.0, 8.0, 20.0];
        let v2 = variance(&paired, true).unwrap();
        near(
            f_test(v1, data.len(), v2, paired.len()),
            0.20800000000000005,
        );
        near(
            chi_test(&[1.0, 2.0, 3.0], &[2.0, 2.0, 2.0], 2.0),
            0.6065306597126334,
        );
    }
}
