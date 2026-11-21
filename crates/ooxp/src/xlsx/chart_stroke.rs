//! Geometry-only stroke prerequisites. No preset dash periods or Excel offsets.
//! Explicit solid-line caps are native-validated; other stroke users retain parser gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cap {
    Round,
    Flat,
    Square,
}
impl Cap {
    pub(super) fn from_ooxml(value: &str) -> Option<Self> {
        match value {
            "rnd" => Some(Self::Round),
            "flat" => Some(Self::Flat),
            "sq" => Some(Self::Square),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Style {
    pub(super) width: f64,
    pub(super) cap: Cap,
}
impl Style {
    pub(super) fn new(width: f64, cap: Cap) -> Option<Self> {
        (width.is_finite() && width > 0.0).then_some(Self { width, cap })
    }
}
pub(super) type Point = (f64, f64);
#[derive(Debug, Clone, Copy)]
pub(super) struct Segment {
    pub(super) from: Point,
    pub(super) to: Point,
    pub(super) start: f64,
    pub(super) length: f64,
    pub(super) index: usize,
}
impl Segment {
    pub(super) fn at(&self, distance: f64) -> Option<Point> {
        if !distance.is_finite() {
            return None;
        }
        if self.length == 0.0 {
            return Some(self.from);
        }
        let t = (distance / self.length).clamp(0.0, 1.0);
        Some((
            self.from.0 + (self.to.0 - self.from.0) * t,
            self.from.1 + (self.to.1 - self.from.1) * t,
        ))
    }
}
/// One continuous run. Gaps are represented by separate runs, not phase resets
/// at each vertex. Zero-length edges retain their identity and consume no arc.
pub(super) struct Run<'a> {
    points: &'a [Point],
    pub(super) length: f64,
}
impl<'a> Run<'a> {
    pub(super) fn new(points: &'a [Point]) -> Option<Self> {
        if points.len() < 2 || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return None;
        }
        let length = points
            .windows(2)
            .map(|p| (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1))
            .sum::<f64>();
        length.is_finite().then_some(Self { points, length })
    }
    pub(super) fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        let mut arc = 0.0;
        self.points.windows(2).enumerate().map(move |(index, p)| {
            let length = (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1);
            let segment = Segment {
                from: p[0],
                to: p[1],
                start: arc,
                length,
                index,
            };
            arc += length;
            segment
        })
    }
    pub(super) fn at(&self, distance: f64) -> Option<Point> {
        if !distance.is_finite() {
            return None;
        }
        let distance = distance.clamp(0.0, self.length);
        for segment in self.segments() {
            if distance <= segment.start + segment.length {
                return segment.at(distance - segment.start);
            }
        }
        self.points.last().copied()
    }
    pub(super) fn last_segment(&self) -> usize {
        self.points.len() - 2
    }
}
/// Signed distance to an explicit-cap segment. Round endpoints are half circles;
/// square caps extend by half the width; flat caps end at the supplied endpoint.
/// Degenerate edges have no unique oriented cap rectangle and are excluded here.
pub(super) struct CapSegment {
    from: Point,
    direction: Point,
    length: f64,
    radius: f64,
    first: Cap,
    last: Cap,
}
impl CapSegment {
    pub(super) fn new(from: Point, to: Point, width: f64, first: Cap, last: Cap) -> Option<Self> {
        let style = Style::new(width, first)?;
        if ![from.0, from.1, to.0, to.1].iter().all(|x| x.is_finite()) {
            return None;
        }
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let length = dx.hypot(dy);
        if !length.is_finite() || length == 0.0 {
            return None;
        }
        Some(Self {
            from,
            direction: (dx / length, dy / length),
            length,
            radius: style.width / 2.0,
            first,
            last,
        })
    }
    pub(super) fn signed_distance(&self, point: Point) -> f64 {
        let dx = point.0 - self.from.0;
        let dy = point.1 - self.from.1;
        let u = dx * self.direction.0 + dy * self.direction.1;
        let v = -dx * self.direction.1 + dy * self.direction.0;
        if u < 0.0 && self.first == Cap::Round {
            return u.hypot(v) - self.radius;
        }
        if u > self.length && self.last == Cap::Round {
            return (u - self.length).hypot(v) - self.radius;
        }
        let low = match self.first {
            Cap::Round => f64::NEG_INFINITY,
            Cap::Flat => 0.0,
            Cap::Square => -self.radius,
        };
        let high = match self.last {
            Cap::Round => f64::INFINITY,
            Cap::Flat => self.length,
            Cap::Square => self.length + self.radius,
        };
        let qx = (low - u).max(u - high);
        let qy = v.abs() - self.radius;
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_cap_tokens_and_positive_finite_styles() {
        for (s, c) in [
            ("rnd", Cap::Round),
            ("flat", Cap::Flat),
            ("sq", Cap::Square),
        ] {
            assert_eq!(Cap::from_ooxml(s), Some(c));
        }
        assert!(Cap::from_ooxml("default").is_none());
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(Style::new(width, Cap::Round).is_none());
        }
        assert_eq!(Style::new(2.0, Cap::Square).unwrap().cap, Cap::Square);
    }
    #[test]
    fn arc_coordinates_continue_across_vertices() {
        let p = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0), (6.0, 4.0)];
        let r = Run::new(&p).unwrap();
        assert_eq!(r.length, 10.0);
        assert_eq!(
            r.segments()
                .map(|s| (s.start, s.length))
                .collect::<Vec<_>>(),
            [(0.0, 3.0), (3.0, 4.0), (7.0, 3.0)]
        );
        assert_eq!(r.at(5.0), Some((3.0, 2.0)));
        assert_eq!(r.at(8.0), Some((4.0, 4.0)));
    }
    #[test]
    fn zero_edges_consume_no_arc_but_keep_identity() {
        let p = [(0.0, 0.0), (0.0, 0.0), (3.0, 4.0)];
        let r = Run::new(&p).unwrap();
        let s = r.segments().collect::<Vec<_>>();
        assert_eq!((s[0].length, s[1].start, s[1].index), (0.0, 0.0, 1));
        assert_eq!(r.at(0.0), Some(p[0]));
        assert_eq!(r.length, 5.0);
    }
    #[test]
    fn separate_gap_runs_start_a_new_arc_domain() {
        let p = [(0.0, 0.0), (4.0, 0.0)];
        let q = [(9.0, 0.0), (12.0, 0.0)];
        assert_eq!(Run::new(&p).unwrap().segments().next().unwrap().start, 0.0);
        assert_eq!(Run::new(&q).unwrap().segments().next().unwrap().start, 0.0);
    }
    #[test]
    fn arc_queries_clamp_and_reject_nonfinite_domains() {
        let p = [(1.0, 2.0), (4.0, 6.0)];
        let r = Run::new(&p).unwrap();
        assert_eq!(r.at(-1.0), Some(p[0]));
        assert_eq!(r.at(20.0), Some(p[1]));
        assert!(r.at(f64::NAN).is_none());
        assert!(Run::new(&[]).is_none());
        assert!(Run::new(&[(0.0, 0.0), (f64::INFINITY, 0.0)]).is_none());
    }
    #[test]
    fn explicit_caps_have_documented_endpoint_extents() {
        for (cap, at_start, near_corner) in [
            (Cap::Round, -2.0, 0.5_f64.hypot(1.9) - 2.0),
            (Cap::Flat, 0.0, 0.5),
            (Cap::Square, -2.0, -0.1),
        ] {
            let g = CapSegment::new((0.0, 0.0), (10.0, 0.0), 4.0, cap, cap).unwrap();
            assert!((g.signed_distance((0.0, 0.0)) - at_start).abs() < 1e-12);
            assert!((g.signed_distance((-0.5, 1.9)) - near_corner).abs() < 1e-12);
        }
    }
    #[test]
    fn geometry_is_invariant_under_translation_and_right_angle_rotation() {
        for cap in [Cap::Round, Cap::Flat, Cap::Square] {
            let h = CapSegment::new((0.0, 0.0), (10.0, 0.0), 4.0, cap, cap).unwrap();
            let v = CapSegment::new((30.0, 40.0), (30.0, 50.0), 4.0, cap, cap).unwrap();
            for p in [(-1.0, 1.5), (0.0, 0.0), (5.0, 3.0), (11.0, 2.5)] {
                assert!(
                    (h.signed_distance(p) - v.signed_distance((30.0 - p.1, 40.0 + p.0))).abs()
                        < 1e-12
                );
            }
        }
    }
    #[test]
    fn first_and_last_caps_are_independent_not_reset_at_every_edge() {
        let g = CapSegment::new((0.0, 0.0), (10.0, 0.0), 4.0, Cap::Flat, Cap::Round).unwrap();
        assert_eq!(g.signed_distance((-1.0, 0.0)), 1.0);
        assert_eq!(g.signed_distance((11.0, 0.0)), -1.0);
    }
    #[test]
    fn degenerate_or_nonfinite_oriented_cap_segments_are_not_invented() {
        assert!(CapSegment::new((0.0, 0.0), (0.0, 0.0), 2.0, Cap::Square, Cap::Square).is_none());
        assert!(
            CapSegment::new((0.0, 0.0), (f64::NAN, 0.0), 2.0, Cap::Round, Cap::Round).is_none()
        );
    }
}
