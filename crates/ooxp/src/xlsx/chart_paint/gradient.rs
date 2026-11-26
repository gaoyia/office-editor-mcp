//! Ordered opaque sRGB stop tables, independent of gradient geometry and the
//! host's interpolation policy. Missing endpoints/coincident stops stay outside
//! this baseline; no implicit stops or discontinuity rules are invented.
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StopTable(Arc<[(u32, [u8; 3])]>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    pub(crate) start: [u8; 3],
    pub(crate) end: [u8; 3],
    pub(crate) fraction: f64,
}

impl StopTable {
    pub(crate) fn new(mut stops: Vec<(u32, [u8; 3])>) -> Option<Self> {
        if stops.len() < 2 || stops.iter().any(|(pos, _)| *pos > 100_000) {
            return None;
        }
        stops.sort_by_key(|(pos, _)| *pos);
        if stops.first()?.0 != 0
            || stops.last()?.0 != 100_000
            || stops.windows(2).any(|pair| pair[0].0 == pair[1].0)
        {
            return None;
        }
        Some(Self(stops.into()))
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Binary search selects the interval; normalization is local to that
    /// interval. The two-endpoint case retains the input bits for legacy
    /// interpolation rather than multiplying/dividing the coordinate again.
    pub(crate) fn segment(&self, t: f64) -> Option<Segment> {
        if !t.is_finite() {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        if self.0.len() == 2 {
            return Some(Segment {
                start: self.0[0].1,
                end: self.0[1].1,
                fraction: t,
            });
        }
        let position = t * 100_000.0;
        let right = self
            .0
            .partition_point(|(pos, _)| f64::from(*pos) <= position)
            .clamp(1, self.0.len() - 1);
        let left = self.0[right - 1];
        let right = self.0[right];
        Some(Segment {
            start: left.1,
            end: right.1,
            fraction: ((position - f64::from(left.0)) / f64::from(right.0 - left.0))
                .clamp(0.0, 1.0),
        })
    }
}

impl Segment {
    /// Standard encoded-sRGB channel interpolation. The caller deliberately
    /// chooses its policy; this must not replace a host's locked two-stop gamma.
    pub(crate) fn encoded_srgb(self) -> [u8; 3] {
        std::array::from_fn(|i| {
            (f64::from(self.start[i])
                + (f64::from(self.end[i]) - f64::from(self.start[i])) * self.fraction)
                .round()
                .clamp(0.0, 255.0) as u8
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: [u8; 3] = [234, 247, 247];
    const B: [u8; 3] = [15, 96, 130];
    const C: [u8; 3] = [240, 64, 32];

    #[test]
    fn constructor_canonicalizes_and_rejects_unmeasured_domains() {
        let table = StopTable::new(vec![(100_000, C), (0, A), (25_000, B)]).unwrap();
        assert_eq!(table.len(), 3);
        for stops in [
            vec![],
            vec![(0, A)],
            vec![(1, A), (100_000, B)],
            vec![(0, A), (99_999, B)],
            vec![(0, A), (100_001, B)],
            vec![(0, A), (25_000, B), (25_000, C), (100_000, C)],
        ] {
            assert!(StopTable::new(stops).is_none());
        }
    }

    #[test]
    fn unequal_stop_intervals_are_normalized_independently() {
        let table = StopTable::new(vec![(0, A), (25_000, B), (100_000, C)]).unwrap();
        let first = table.segment(0.125).unwrap();
        assert_eq!((first.start, first.end, first.fraction), (A, B, 0.5));
        let second = table.segment(0.625).unwrap();
        assert_eq!((second.start, second.end, second.fraction), (B, C, 0.5));
        assert_eq!(table.segment(0.25).unwrap().encoded_srgb(), B);
        assert_eq!(table.segment(0.0).unwrap().encoded_srgb(), A);
        assert_eq!(table.segment(1.0).unwrap().encoded_srgb(), C);
    }

    #[test]
    fn four_stops_select_each_interval_without_geometry() {
        let d = [128, 64, 224];
        let table = StopTable::new(vec![(0, A), (20_000, B), (65_000, d), (100_000, C)]).unwrap();
        for (t, from, to) in [(0.1, A, B), (0.425, B, d), (0.825, d, C)] {
            let segment = table.segment(t).unwrap();
            assert_eq!((segment.start, segment.end), (from, to));
            assert!((segment.fraction - 0.5).abs() < 1e-12);
        }
    }

    #[test]
    fn legacy_two_endpoint_coordinates_keep_exact_bits() {
        let table = StopTable::new(vec![(0, A), (100_000, B)]).unwrap();
        for width in [2, 3, 7, 17, 424, 4096] {
            for x in 0..width {
                let t = (f64::from(x) + 0.5) / f64::from(width);
                assert_eq!(table.segment(t).unwrap().fraction.to_bits(), t.to_bits());
            }
        }
    }

    #[test]
    fn nonfinite_queries_are_rejected_and_finite_queries_clamp() {
        let table = StopTable::new(vec![(0, A), (50_000, B), (100_000, C)]).unwrap();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(table.segment(bad).is_none());
        }
        assert_eq!(table.segment(-1.0).unwrap().encoded_srgb(), A);
        assert_eq!(table.segment(2.0).unwrap().encoded_srgb(), C);
    }

    #[test]
    fn cloned_tables_share_storage_instead_of_copying_every_pixel() {
        let table = StopTable::new((0..=1000).map(|i| (i * 100, A)).collect()).unwrap();
        let cloned = table.clone();
        assert!(Arc::ptr_eq(&table.0, &cloned.0));
        assert_eq!(cloned.segment(0.12345).unwrap().encoded_srgb(), A);
    }

    #[test]
    fn encoded_interpolation_has_exact_colors_at_stop_boundaries() {
        let table = StopTable::new(vec![(0, A), (50_000, B), (100_000, C)]).unwrap();
        assert_eq!(table.segment(0.25).unwrap().encoded_srgb(), [125, 172, 189]);
        assert_eq!(table.segment(0.5).unwrap().encoded_srgb(), B);
        assert_eq!(table.segment(0.75).unwrap().encoded_srgb(), [128, 80, 81]);
    }
}
