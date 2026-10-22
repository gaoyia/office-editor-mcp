//! 在一段可见文字里按原文定位一截。docx 段落和 DrawingML 文本框共用。
//! 下标是 Unicode 标量，查找不从码位中间切开。

pub(crate) fn locate_span(
    visible: &str,
    cursor_bytes: usize,
    needle: Option<&str>,
    after: Option<&str>,
    before: Option<&str>,
    nth: usize,
) -> Option<(usize, usize)> {
    if let Some(needle) = needle {
        let mut from = cursor_bytes;
        let mut seen = 0usize;
        while from < visible.len() {
            let rel = visible[from..].find(needle)?;
            let abs = from + rel;
            let end = abs + needle.len();
            let preceded = after.is_none_or(|prefix| visible[..abs].ends_with(prefix));
            let followed = before.is_none_or(|suffix| visible[end..].starts_with(suffix));
            if preceded && followed {
                seen += 1;
                if seen == nth {
                    return Some((abs, end));
                }
            }
            let step = visible[abs..]
                .chars()
                .next()
                .map(|ch| ch.len_utf8())
                .unwrap_or(1);
            from = abs + step;
        }
        return None;
    }
    let after = after?;
    let before = before?;
    let mut from = cursor_bytes;
    let mut seen = 0usize;
    while from < visible.len() {
        let rel = visible[from..].find(after)?;
        let abs = from + rel;
        let span_start = abs + after.len();
        if let Some(rel_before) = visible[span_start..].find(before) {
            if rel_before > 0 {
                seen += 1;
                if seen == nth {
                    return Some((span_start, span_start + rel_before));
                }
            }
        }
        let step = visible[abs..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1);
        from = abs + step;
    }
    None
}
