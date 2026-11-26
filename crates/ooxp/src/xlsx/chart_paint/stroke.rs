//! Shared stroke geometry and contract-derived dash patterns. No Excel offsets.
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
/// Internal join geometry is independent from true run/dash end caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Join { Round, Bevel, Miter { limit: u32 } }

/// Local join disk clipped by the outer offset-edge chord. Its visible outer
/// boundary is the bevel hull; the interior overlaps both butt-ended bodies,
/// avoiding an artificial AA seam where triangle/rectangles merely touch.
/// No miter length, angle threshold, device offset or preset table.
pub(super) struct BevelJoin { vertices: [Point;3], radius: f64 }
impl BevelJoin {
    pub(super) fn new(a: Point,b: Point,c: Point,width: f64)->Option<Self> {
        let incoming=CapSegment::new(a,b,width,Cap::Flat,Cap::Flat)?;
        let outgoing=CapSegment::new(b,c,width,Cap::Flat,Cap::Flat)?;
        let u=incoming.direction;let v=outgoing.direction;
        let turn=u.0*v.1-u.1*v.0;
        // No unique outer triangle for a straight edge/exact reversal. Do not
        // invent a round/square degeneracy rule here.
        if turn==0.0 {return None;}
        let side=if turn>0.0 {-1.0}else{1.0};let radius=width/2.0;
        let p=(b.0-side*u.1*radius,b.1+side*u.0*radius);
        let q=(b.0-side*v.1*radius,b.1+side*v.0*radius);
        [p.0,p.1,q.0,q.1].iter().all(|x|x.is_finite()).then_some(Self{vertices:[b,p,q],radius})
    }
    pub(super) fn bounds(&self)->(Point,Point) {
        let b=self.vertices[0];((b.0-self.radius,b.1-self.radius),(b.0+self.radius,b.1+self.radius))
    }
    pub(super) fn signed_distance(&self,p:Point)->f64 {
        let b=self.vertices[0];let a=self.vertices[1];let c=self.vertices[2];let e=(c.0-a.0,c.1-a.1);
        let cross=|p:Point|e.0*(p.1-a.1)-e.1*(p.0-a.0);
        let side=if cross(b)>0.0 {1.0}else{-1.0};
        let plane=-side*cross(p)/e.0.hypot(e.1);
        let circle=(p.0-b.0).hypot(p.1-b.1)-self.radius;
        circle.max(plane)
    }
}

/// Offset-edge intersection with Office's inner-to-outer corner limit. This
/// distance / full pen width is the same ratio as tip distance / half width.
/// Limit units are OOXML positive percentages (100000 = 100%). Exceeding the
/// limit falls back to the existing bevel hull, independently at each vertex.
/// MS-OI29500 20.1.8.43 and same-source 1/2/4/8x native controls define this,
/// not an empirical angle threshold. Exact straight/reversed edges have no
/// unique outer intersection and stay outside this nondegenerate primitive.
pub(super) struct MiterJoin { bevel: BevelJoin, tip: Option<Point> }
impl MiterJoin {
    pub(super) fn new(a: Point, b: Point, c: Point, width: f64, limit: u32) -> Option<Self> {
        let bevel = BevelJoin::new(a, b, c, width)?;
        let p = bevel.vertices[1];
        let q = bevel.vertices[2];
        let sum = (p.0 + q.0 - 2.0*b.0, p.1 + q.1 - 2.0*b.1);
        let length = sum.0.hypot(sum.1);
        if length == 0.0 || !length.is_finite() { return None; }
        let direction = (sum.0/length, sum.1/length);
        let cosine = ((p.0-b.0)*direction.0 + (p.1-b.1)*direction.1)/bevel.radius;
        if cosine <= 0.0 || !cosine.is_finite() { return None; }
        let ratio = 1.0/cosine;
        // Compare before extending: a near reversal must not create a giant
        // mask or nonfinite tip just to discover it exceeds the supplied limit.
        let tip = if ratio > f64::from(limit)/100_000.0 { None } else {
            let distance = bevel.radius*ratio;
            let tip = (b.0 + direction.0*distance, b.1 + direction.1*distance);
            if !tip.0.is_finite() || !tip.1.is_finite() { return None; }
            Some(tip)
        };
        Some(Self { bevel, tip })
    }
    pub(super) fn bounds(&self) -> (Point, Point) {
        let (mut low, mut high) = self.bevel.bounds();
        if let Some(p) = self.tip {
            low = (low.0.min(p.0), low.1.min(p.1));
            high = (high.0.max(p.0), high.1.max(p.1));
        }
        (low, high)
    }
    pub(super) fn signed_distance(&self, point: Point) -> f64 {
        let Some(tip) = self.tip else { return self.bevel.signed_distance(point); };
        let [b, p, q] = self.bevel.vertices;
        let segment_distance = |a: Point, c: Point| {
            let edge = (c.0-a.0, c.1-a.1);
            let length = edge.0.hypot(edge.1);
            if length == 0.0 { return (point.0-a.0).hypot(point.1-a.1); }
            let u = (edge.0/length, edge.1/length);
            let t = ((point.0-a.0)*u.0 + (point.1-a.1)*u.1).clamp(0.0, length);
            (point.0-a.0-u.0*t).hypot(point.1-a.1-u.1*t)
        };
        // Boundary of the convex hull of the local disk and offset-edge tip:
        // two tangent segments plus the remaining circular arc. Using the real
        // boundary (not two touching triangles) avoids an interior AA seam.
        let v = (point.0-b.0, point.1-b.1);
        let distance = v.0.hypot(v.1);
        let axis = (tip.0-b.0, tip.1-b.1);
        let axis_length = axis.0.hypot(axis.1);
        let chord = ((p.0-b.0)*axis.0 + (p.1-b.1)*axis.1)/(self.bevel.radius*axis_length);
        let arc = if distance == 0.0 || (v.0*axis.0+v.1*axis.1)/(distance*axis_length) <= chord {
            (distance-self.bevel.radius).abs()
        } else { (point.0-p.0).hypot(point.1-p.1).min((point.0-q.0).hypot(point.1-q.1)) };
        let boundary = arc.min(segment_distance(p, tip)).min(segment_distance(tip, q));
        let cross = |a: Point, c: Point| (c.0-a.0)*(point.1-a.1)-(c.1-a.1)*(point.0-a.0);
        let signs = [cross(p, tip), cross(tip, q), cross(q, p)];
        let area = (tip.0-p.0)*(q.1-p.1)-(tip.1-p.1)*(q.0-p.0);
        let in_triangle = area != 0.0 && (signs.iter().all(|s| *s>=0.0) || signs.iter().all(|s| *s<=0.0));
        if distance <= self.bevel.radius || in_triangle { -boundary } else { boundary }
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

/// Native controls distinguish explicit pens from the inherited chart pen.
/// Explicit positive widths round to a nearest device pixel with a one-pixel
/// minimum; an omitted width must keep its separately resolved chart default.
/// This is not a replacement for bar/area outlines or legacy scatter pens.
pub(super) fn explicit_pen_width_px(px: f64) -> Option<f64> {
    (px.is_finite() && px >= 0.0).then(|| if px == 0.0 { 0.0 } else { px.round().max(1.0) })
}

/// Exact same-direction collinear subdivisions must not invent round joins.
/// This is topology simplification, not coordinate snapping or an angle epsilon.
/// True turns and reversals retain their vertices and existing join semantics.
pub(super) fn simplify_collinear(points: &mut Vec<Point>) {
    let mut used = 0;
    for read in 0..points.len() {
        let p = points[read];
        while used >= 2 {
            let a = points[used - 2];
            let b = points[used - 1];
            let u = (b.0 - a.0, b.1 - a.1);
            let v = (p.0 - b.0, p.1 - b.1);
            if u.0 * v.1 - u.1 * v.0 != 0.0 || u.0 * v.0 + u.1 * v.1 < 0.0 {
                break;
            }
            used -= 1;
        }
        points[used] = p;
        used += 1;
    }
    points.truncate(used);
}

/// Ordered custom dash/space lengths in device units, derived from OOXML
/// percentages of the resolved pen width. No empirical period/phase adjustment.
#[derive(Debug)]
pub(super) struct DashPattern {
    stops: Vec<(f64, f64)>,
}
impl DashPattern {
    /// DrawingML ST_PresetLineDashVal binary runs: each bit is one pen width.
    /// These are contract tokens, not an estimated Excel period table. Native
    /// two-width controls verify their interior lengths/phase; caller separately
    /// gates supported caps/hosts and retains legend/layout residuals.
    pub(super) fn from_preset_width(value: &str, width: f64) -> Option<Self> {
        let bits = match value {
            "dash" => "1111000", "dashDot" => "11110001000", "dot" => "1000",
            "lgDash" => "11111111000", "lgDashDot" => "111111110001000",
            "lgDashDotDot" => "1111111100010001000", "sysDash" => "1110",
            "sysDot" => "10", "sysDashDot" => "111010", "sysDashDotDot" => "11101010",
            _ => return None, // solid has no finite repeating off interval.
        };
        let bits = bits.as_bytes();
        let mut stops = Vec::new();
        let mut start = 0;
        while start < bits.len() {
            let on = bits[start..].iter().take_while(|&&b| b == b'1').count();
            let off = bits[start+on..].iter().take_while(|&&b| b == b'0').count();
            if on == 0 || off == 0 { return None; }
            stops.push((on as f64 * 100_000.0, off as f64 * 100_000.0));
            start += on + off;
        }
        Self::from_width_percent(&stops, width)
    }
    pub(super) fn from_width_percent(stops: &[(f64, f64)], width: f64) -> Option<Self> {
        if !width.is_finite() || width <= 0.0 || stops.is_empty() || stops.len() > 256 {
            return None;
        }
        let mut scaled = Vec::with_capacity(stops.len());
        let mut period = 0.0;
        for &(dash, space) in stops {
            // Zero-length dots/touching stops have separate cap semantics, not
            // this positive-length flat-dash baseline.
            if !dash.is_finite() || !space.is_finite() || dash <= 0.0 || space <= 0.0 {
                return None;
            }
            let d = dash * width / 100_000.0;
            let sp = space * width / 100_000.0;
            if !d.is_finite() || !sp.is_finite() || d <= 0.0 || sp <= 0.0 {
                return None;
            }
            period += d + sp;
            scaled.push((d, sp));
        }
        period.is_finite().then_some(Self { stops: scaled })
    }
    /// Whole-run intervals, never one independent pattern per polyline edge.
    /// Resource overflow is an error, not silent pattern truncation.
    pub(super) fn painted_intervals(&self, length: f64, budget: usize) -> Option<Vec<(f64, f64)>> {
        if !length.is_finite() || length < 0.0 {
            return None;
        }
        let mut intervals = Vec::new();
        let mut position = 0.0;
        let mut stop = 0;
        while position < length {
            if intervals.len() == budget {
                return None;
            }
            let (dash, space) = self.stops[stop];
            let end = position + dash;
            if !end.is_finite() || end <= position {
                return None;
            }
            intervals.push((position, end.min(length)));
            if end >= length {
                break;
            }
            let next = end + space;
            if !next.is_finite() || next <= end {
                return None;
            }
            position = next;
            stop = (stop + 1) % self.stops.len();
        }
        Some(intervals)
    }
}
/// A painted piece has dash caps distinct from the true polyline endpoints.
pub(super) struct CappedDash {
    pub(super) points: Vec<Point>,
    pub(super) first: Cap,
    pub(super) last: Cap,
}
impl DashPattern {
    /// Round native controls preserve the logical period, but include the caps
    /// within the logical dash length: the centerline is one pen shorter.
    /// Clip that nominal centerline to the run, never shorten an already clipped
    /// last dash. Native short-round controls clamp logical d to one pen before
    /// advancing the period: d <= 100% is a point, not a shorter-period circle.
    fn round_intervals(&self, length: f64, width: f64, budget: usize) -> Option<Vec<(f64,f64)>> {
        if !length.is_finite() || length < 0.0 || !width.is_finite() || width <= 0.0 { return None; }
        let mut result=Vec::new();
        let mut start=0.0;
        let mut stop=0;
        while start < length {
            if result.len()==budget { return None; }
            let (dash,space)=self.stops[stop];
            let effective_dash=dash.max(width);
            // Preserve an exact point even when fractional spaces accumulate;
            // adding then subtracting a pen could round below the start.
            let nominal_end=if dash<=width {start}else{start+dash-width};
            if !nominal_end.is_finite() || nominal_end<start { return None; }
            result.push((start,nominal_end.min(length)));
            let next=start+effective_dash+space;
            if !next.is_finite() || next<=start { return None; }
            start=next;
            stop=(stop+1)%self.stops.len();
        }
        Some(result)
    }
}
impl Run<'_> {
    pub(super) fn dashed_capped(&self, pattern: &DashPattern, style: Style, budget: usize) -> Option<Vec<CappedDash>> {
        let intervals=if style.cap==Cap::Round {
            pattern.round_intervals(self.length,style.width,budget)?
        } else { pattern.painted_intervals(self.length,budget)? };
        let paths=self.paths_for_intervals(&intervals)?;
        Some(paths.into_iter().zip(intervals).map(|(points,(start,end))| {
            let (first,last)=match style.cap {
                Cap::Round => (Cap::Round,Cap::Round),
                Cap::Flat => (Cap::Flat,Cap::Flat),
                // Square is a true run-end cap, not a cap on every dash.
                Cap::Square => (if start==0.0 {Cap::Square}else{Cap::Flat},
                    if end==self.length {Cap::Square}else{Cap::Flat}),
            };
            CappedDash {points,first,last}
        }).collect())
    }
    /// Subdivide a continuous polyline in O(edges + intervals). Vertices inside
    /// a painted dash are retained as joins, not new capped dash endpoints.
    #[cfg(test)]
    pub(super) fn dashed(&self, pattern: &DashPattern, budget: usize) -> Option<Vec<Vec<Point>>> {
        let intervals = pattern.painted_intervals(self.length, budget)?;
        self.paths_for_intervals(&intervals)
    }
    fn paths_for_intervals(&self, intervals: &[(f64,f64)]) -> Option<Vec<Vec<Point>>> {
        let mut edges = self.segments().filter(|edge| edge.length > 0.0).peekable();
        let mut paths = Vec::with_capacity(intervals.len());
        for &(start, end) in intervals {
            while edges.peek().is_some_and(|e| e.start + e.length <= start) {
                edges.next();
            }
            let mut edge = *edges.peek()?;
            let mut path = vec![edge.at(start - edge.start)?];
            while edge.start + edge.length < end {
                path.push(edge.to);
                edges.next();
                edge = *edges.peek()?;
            }
            path.push(edge.at(end - edge.start)?);
            paths.push(path);
        }
        Some(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preset_dash_binary_contract_scales_all_positive_runs() {
        for (name, units) in [
            ("dash",vec![(4.,3.)]), ("dashDot",vec![(4.,3.),(1.,3.)]),
            ("dot",vec![(1.,3.)]), ("lgDash",vec![(8.,3.)]),
            ("lgDashDot",vec![(8.,3.),(1.,3.)]),
            ("lgDashDotDot",vec![(8.,3.),(1.,3.),(1.,3.)]),
            ("sysDash",vec![(3.,1.)]), ("sysDot",vec![(1.,1.)]),
            ("sysDashDot",vec![(3.,1.),(1.,1.)]),
            ("sysDashDotDot",vec![(3.,1.),(1.,1.),(1.,1.)]),
        ] {
            for width in [1.,2.5,6.,12.] {
                let pattern=DashPattern::from_preset_width(name,width).unwrap();
                assert_eq!(pattern.stops,units.iter().map(|(d,s)|(d*width,s*width)).collect::<Vec<_>>());
            }
        }
    }
    #[test]
    fn preset_dash_rejects_unknown_solid_and_unresolved_widths() {
        for name in ["", "solid", "Dash", " dash", "1111000", "unknown"] {
            assert!(DashPattern::from_preset_width(name,6.).is_none());
        }
        for width in [0.,-1.,f64::NAN,f64::INFINITY] {
            assert!(DashPattern::from_preset_width("dash",width).is_none());
        }
    }
    #[test]
    fn preset_dash_uses_shared_run_phase_caps_and_resource_budget() {
        let points=[(0.,0.),(5.,0.),(5.,30.),(60.,30.)];let run=Run::new(&points).unwrap();
        let preset=DashPattern::from_preset_width("dashDot",2.).unwrap();
        let custom=DashPattern::from_width_percent(&[(400000.,300000.),(100000.,300000.)],2.).unwrap();
        for cap in [Cap::Flat,Cap::Round] {
            let a=run.dashed_capped(&preset,Style::new(2.,cap).unwrap(),32).unwrap();
            let b=run.dashed_capped(&custom,Style::new(2.,cap).unwrap(),32).unwrap();
            assert_eq!(a.len(),b.len());
            for (a,b) in a.iter().zip(b.iter()) {assert_eq!(a.points,b.points);assert_eq!(a.first,b.first);assert_eq!(a.last,b.last);}
        }
        assert_eq!(preset.painted_intervals(30.,8).unwrap(),[(0.,8.),(14.,16.),(22.,30.)]);
        assert!(preset.painted_intervals(30.,2).is_none());
    }

    #[test]
    fn bevel_outer_hull_shaves_round_corner_and_reflects_turns() {
        let b=BevelJoin::new((0.0,0.0),(10.0,0.0),(10.0,10.0),4.0).unwrap();
        assert!(b.signed_distance((10.5,-0.5))<0.0);
        assert!(b.signed_distance((10.0,0.0)) < -1.0); // No artificial internal AA edge at the vertex.
        assert!(b.signed_distance((11.5,-1.5))>0.0);
        assert!(b.signed_distance((11.0,-1.0)).abs()<1e-12);
        let reflected=BevelJoin::new((0.0,0.0),(10.0,0.0),(10.0,-10.0),4.0).unwrap();
        for p in [(10.5,-0.5),(11.5,-1.5),(8.0,-1.0)] {assert!((b.signed_distance(p)-reflected.signed_distance((p.0,-p.1))).abs()<1e-12);}
    }
    #[test]
    fn bevel_has_no_invented_degenerate_or_nonfinite_join() {
        for points in [[(0.0,0.0),(0.0,0.0),(1.0,1.0)],[(0.0,0.0),(1.0,0.0),(2.0,0.0)],[(0.0,0.0),(1.0,0.0),(0.0,0.0)],[(f64::NAN,0.0),(1.0,0.0),(1.0,1.0)]] {assert!(BevelJoin::new(points[0],points[1],points[2],4.0).is_none());}
        for width in [0.0,-1.0,f64::INFINITY] {assert!(BevelJoin::new((0.0,0.0),(1.0,0.0),(1.0,1.0),width).is_none());}
    }
    #[test]
    fn bevel_geometry_scales_and_translates_without_angle_tables() {
        let a=BevelJoin::new((0.0,0.0),(10.0,0.0),(15.0,8.0),4.0).unwrap();
        let b=BevelJoin::new((7.0,9.0),(37.0,9.0),(52.0,33.0),12.0).unwrap();
        for p in [(10.0,-1.0),(12.0,-2.0),(9.0,1.0)] {assert!((3.0*a.signed_distance(p)-b.signed_distance((3.0*p.0+7.0,3.0*p.1+9.0))).abs()<1e-10);}
    }
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
    #[test]
    fn explicit_device_pen_matrix_does_not_ceil_every_positive_width() {
        let widths = [
            0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0, 4.0,
        ];
        let expected = [1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 6.0];
        for (pt, px) in widths.into_iter().zip(expected) {
            assert_eq!(explicit_pen_width_px(pt * 106.0 / 72.0), Some(px));
        }
        assert_eq!(explicit_pen_width_px(0.0), Some(0.0));
        for v in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(explicit_pen_width_px(v).is_none());
        }
    }
    #[test]
    fn custom_percentages_scale_by_resolved_width_not_absolute_units() {
        for w in [1.0, 3.0, 6.0] {
            let p = DashPattern::from_width_percent(&[(200000.0, 100000.0)], w).unwrap();
            assert_eq!(
                p.painted_intervals(w * 8.0, 16).unwrap(),
                [(0.0, 2.0 * w), (3.0 * w, 5.0 * w), (6.0 * w, 8.0 * w)]
            );
        }
    }
    #[test]
    fn ordered_multi_stop_pattern_repeats_without_collapsing_to_one_pair() {
        let p = DashPattern::from_width_percent(&[(100000.0, 100000.0), (300000.0, 200000.0)], 6.0)
            .unwrap();
        assert_eq!(
            p.painted_intervals(60.0, 16).unwrap(),
            [(0.0, 6.0), (12.0, 30.0), (42.0, 48.0), (54.0, 60.0)]
        );
    }
    #[test]
    fn painted_dash_keeps_internal_vertex_and_gap_owns_new_phase_domain() {
        let points = [(0.0, 0.0), (3.0, 0.0), (3.0, 4.0), (6.0, 4.0)];
        let p = DashPattern::from_width_percent(&[(500000.0, 100000.0)], 1.0).unwrap();
        let paths = Run::new(&points).unwrap().dashed(&p, 16).unwrap();
        assert_eq!(paths[0], [(0.0, 0.0), (3.0, 0.0), (3.0, 2.0)]);
        assert_eq!(paths[1], [(3.0, 3.0), (3.0, 4.0), (6.0, 4.0)]);
        let other = [(20.0, 0.0), (30.0, 0.0)];
        assert_eq!(
            Run::new(&other).unwrap().dashed(&p, 16).unwrap()[0],
            [(20.0, 0.0), (25.0, 0.0)]
        );
    }
    #[test]
    fn zero_edges_and_exact_vertex_boundaries_do_not_reset_or_duplicate_a_dash() {
        let points = [(0.0, 0.0), (0.0, 0.0), (2.0, 0.0), (2.0, 0.0), (8.0, 0.0)];
        let p = DashPattern::from_width_percent(&[(200000.0, 100000.0)], 1.0).unwrap();
        assert_eq!(
            Run::new(&points).unwrap().dashed(&p, 16).unwrap(),
            vec![
                vec![(0.0, 0.0), (2.0, 0.0)],
                vec![(3.0, 0.0), (5.0, 0.0)],
                vec![(6.0, 0.0), (8.0, 0.0)]
            ]
        );
    }
    #[test]
    fn equivalent_collinear_tessellations_have_identical_dash_endpoints() {
        let p = DashPattern::from_width_percent(&[(200000.0, 100000.0)], 2.0).unwrap();
        let a = [(0.0, 0.0), (20.0, 0.0)];
        let b = [(0.0, 0.0), (3.0, 0.0), (11.0, 0.0), (20.0, 0.0)];
        let ends = |points: &[Point]| {
            Run::new(points)
                .unwrap()
                .dashed(&p, 16)
                .unwrap()
                .into_iter()
                .map(|v| (v[0], *v.last().unwrap()))
                .collect::<Vec<_>>()
        };
        assert_eq!(ends(&a), ends(&b));
    }
    #[test]
    fn invalid_zero_nonfinite_and_resource_exhaustion_do_not_spin_or_truncate() {
        for pair in [
            (0.0, 1.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
        ] {
            assert!(DashPattern::from_width_percent(&[pair], 1.0).is_none());
        }
        assert!(DashPattern::from_width_percent(&[], 1.0).is_none());
        assert!(DashPattern::from_width_percent(&[(1.0, 1.0); 257], 1.0).is_none());
        let p = DashPattern::from_width_percent(&[(200000.0, 100000.0)], 1.0).unwrap();
        assert!(p.painted_intervals(100.0, 2).is_none());
        assert!(p.painted_intervals(f64::INFINITY, 2).is_none());
        assert!(p.painted_intervals(-1.0, 2).is_none());
        assert_eq!(p.painted_intervals(0.0, 0).unwrap(), []);
    }
    #[test]
    fn exact_collinear_simplification_keeps_real_turns_and_reversals() {
        for mut points in [
            vec![(0.0, 0.0), (2.0, 0.0), (5.0, 0.0)],
            vec![(0.0, 0.0), (2.0, 2.0), (5.0, 5.0)],
            vec![(0.0, 0.0), (0.0, 0.0), (0.0, 5.0)],
        ] {
            let ends = [points[0], *points.last().unwrap()];
            simplify_collinear(&mut points);
            assert_eq!(points, ends);
        }
        for points in [
            vec![(0.0, 0.0), (2.0, 0.0), (2.0, 5.0)],
            vec![(0.0, 0.0), (2.0, 0.0), (0.0, 0.0)],
        ] {
            let mut actual = points.clone();
            simplify_collinear(&mut actual);
            assert_eq!(actual, points);
        }
    }
    #[test]
    fn round_dash_caps_are_included_in_logical_lengths_without_changing_periods() {
        for w in [1.0,3.0,6.0] {
            let pattern=DashPattern::from_width_percent(&[(200000.0,100000.0)],w).unwrap();
            let points=[(0.0,0.0),(w*8.0,0.0)];
            let paths=Run::new(&points).unwrap().dashed_capped(&pattern,Style::new(w,Cap::Round).unwrap(),10).unwrap();
            assert_eq!(paths.len(),3);
            for (i,path) in paths.iter().enumerate() {
                let start=i as f64*w*3.0;
                assert_eq!(path.points,[(start,0.0),(start+w,0.0)]);
                assert_eq!((path.first,path.last),(Cap::Round,Cap::Round));
                // Two half-width caps + one-width body = the requested 200%.
                assert_eq!(path.points[1].0-path.points[0].0+w,w*2.0);
            }
        }
    }
    #[test]
    fn round_nominal_centerline_is_clipped_after_cap_length_resolution() {
        let p=DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
        for length in [2.0,5.0,20.0,23.0] {
            let points=[(0.0,0.0),(length,0.0)];
            let v=Run::new(&points).unwrap().dashed_capped(&p,Style::new(6.0,Cap::Round).unwrap(),10).unwrap();
            assert_eq!(v.last().unwrap().points.last(),Some(&(length,0.0)));
        }
    }
    #[test]
    fn square_cap_applies_only_at_true_run_ends_not_internal_dash_ends() {
        let p=DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
        let points=[(0.0,0.0),(50.0,0.0)];
        let v=Run::new(&points).unwrap().dashed_capped(&p,Style::new(6.0,Cap::Square).unwrap(),10).unwrap();
        assert_eq!(v.len(),3);
        assert_eq!((v[0].first,v[0].last),(Cap::Square,Cap::Flat));
        assert_eq!((v[1].first,v[1].last),(Cap::Flat,Cap::Flat));
        assert_eq!((v[2].first,v[2].last),(Cap::Flat,Cap::Flat));
        // The true endpoint falls in a space: never invent a terminal mark.
        assert_eq!(v[2].points.last(),Some(&(48.0,0.0)));
        let clipped=[(0.0,0.0),(22.0,0.0)];
        let v=Run::new(&clipped).unwrap().dashed_capped(&p,Style::new(6.0,Cap::Square).unwrap(),10).unwrap();
        assert_eq!((v[1].first,v[1].last),(Cap::Flat,Cap::Square));
    }
    #[test]
    fn round_multi_stop_lengths_keep_order_and_include_caps_for_every_stop() {
        let p=DashPattern::from_width_percent(&[(200000.0,100000.0),(300000.0,200000.0)],4.0).unwrap();
        assert_eq!(p.round_intervals(64.0,4.0,10).unwrap(),[(0.0,4.0),(12.0,20.0),(32.0,36.0),(44.0,52.0)]);
    }
    #[test]
    fn capped_dashes_preserve_true_turns_and_restart_only_for_separate_runs() {
        let p=DashPattern::from_width_percent(&[(300000.0,100000.0)],2.0).unwrap();
        let a=[(0.0,0.0),(3.0,0.0),(3.0,12.0)];
        let b=[(10.0,0.0),(25.0,0.0)];
        let style=Style::new(2.0,Cap::Round).unwrap();
        let v=Run::new(&a).unwrap().dashed_capped(&p,style,10).unwrap();
        assert_eq!(v[0].points,[(0.0,0.0),(3.0,0.0),(3.0,1.0)]);
        assert_eq!(v[1].points,[(3.0,5.0),(3.0,9.0)]);
        let other=Run::new(&b).unwrap().dashed_capped(&p,style,10).unwrap();
        assert_eq!(other[0].points,[(10.0,0.0),(14.0,0.0)]);
    }
    #[test]
    fn round_short_bodies_share_a_pen_sized_dot_and_resource_errors_stay_explicit() {
        for percent in [1.0,25000.0,50000.0,100000.0] {
            let p=DashPattern::from_width_percent(&[(percent,100000.0)],6.0).unwrap();
            assert_eq!(p.round_intervals(30.0,6.0,10).unwrap(),[(0.0,0.0),(12.0,12.0),(24.0,24.0)]);
        }
        let p=DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
        assert!(p.round_intervals(50.0,6.0,2).is_none());
        for bad in [-1.0,f64::NAN,f64::INFINITY] {assert!(p.round_intervals(bad,6.0,10).is_none());}
        assert!(p.round_intervals(50.0,0.0,10).is_none());
    }

    #[test]
    fn round_short_dash_clamps_both_body_and_period_at_multiple_device_widths() {
        for width in [1.0,3.0,6.0] {
            let mut expected=None;
            for d in [1.0,25000.0,50000.0,100000.0] {
                let p=DashPattern::from_width_percent(&[(d,100000.0)],width).unwrap();
                let intervals=p.round_intervals(width*5.0,width,10).unwrap();
                assert_eq!(intervals,[(0.0,0.0),(2.0*width,2.0*width),(4.0*width,4.0*width)]);
                if let Some(ref e)=expected {assert_eq!(&intervals,e);}else{expected=Some(intervals);}
            }
        }
    }
    #[test]
    fn round_dot_points_use_continuous_arc_and_real_gap_runs_restart() {
        let p=DashPattern::from_width_percent(&[(50000.0,100000.0)],2.0).unwrap();
        let a=[(0.0,0.0),(3.0,0.0),(3.0,10.0)];let b=[(10.0,10.0),(20.0,10.0)];
        let style=Style::new(2.0,Cap::Round).unwrap();
        let dots=Run::new(&a).unwrap().dashed_capped(&p,style,10).unwrap();
        assert_eq!(dots.iter().map(|x|x.points.clone()).collect::<Vec<_>>(),[vec![(0.0,0.0);2],vec![(3.0,1.0);2],vec![(3.0,5.0);2],vec![(3.0,9.0);2]]);
        let other=Run::new(&b).unwrap().dashed_capped(&p,style,10).unwrap();assert_eq!(other[0].points,[(10.0,10.0);2]);
    }
    #[test]
    fn round_mixed_dot_dash_table_clamps_each_stop_without_changing_order() {
        let p=DashPattern::from_width_percent(&[(50000.0,100000.0),(300000.0,100000.0)],2.0).unwrap();
        assert_eq!(p.round_intervals(23.0,2.0,10).unwrap(),[(0.0,0.0),(4.0,8.0),(12.0,12.0),(16.0,20.0)]);
    }
    #[test]
    fn round_fractional_space_dots_remain_exact_points_and_obey_budget() {
        let p=DashPattern::from_width_percent(&[(25000.0,33333.0)],6.0).unwrap();
        let dots=p.round_intervals(1000.0,6.0,1000).unwrap();assert!(dots.len()>100);
        assert!(dots.iter().all(|(a,b)|a==b));assert!(p.round_intervals(1000.0,6.0,10).is_none());
    }
    #[test]
    fn final_round_phase_uses_nominal_body_before_clipping_not_the_logical_tail() {
        let p=DashPattern::from_width_percent(&[(300000.0,100000.0)],6.0).unwrap();
        assert_eq!(p.round_intervals(38.5,6.0,10).unwrap(),[(0.0,12.0),(24.0,36.0)]);
        let p=DashPattern::from_width_percent(&[(200000.0,50000.0)],6.0).unwrap();
        assert_eq!(p.round_intervals(38.5,6.0,10).unwrap(),[(0.0,6.0),(15.0,21.0),(30.0,36.0)]);
        // A terminal gap does not stretch the last round body to the endpoint.
        assert_eq!(p.round_intervals(35.0,6.0,10).unwrap(),[(0.0,6.0),(15.0,21.0),(30.0,35.0)]);
    }

}

#[cfg(test)]
mod miter_tests {
    use super::*;
    #[test]
    fn miter_intersects_offset_edges_and_applies_dimensionless_limit() {
        let points = [(0.0,0.0),(20.0,0.0),(20.0,20.0)];
        let m = MiterJoin::new(points[0],points[1],points[2],4.0,200000).unwrap();
        let b = MiterJoin::new(points[0],points[1],points[2],4.0,100000).unwrap();
        let bevel = BevelJoin::new(points[0],points[1],points[2],4.0).unwrap();
        assert_eq!(m.tip,Some((22.0,-2.0)));
        assert!(m.signed_distance((21.8,-1.8))<0.0);
        for p in [(21.8,-1.8),(20.0,0.0),(19.0,1.0),(30.0,30.0)] {
            assert_eq!(b.signed_distance(p),bevel.signed_distance(p));
        }
        assert!(MiterJoin::new(points[0],points[1],points[2],4.0,141421).unwrap().tip.is_none());
        assert!(MiterJoin::new(points[0],points[1],points[2],4.0,141422).unwrap().tip.is_some());
    }
    #[test]
    fn miter_is_mirrored_scaled_and_reversed_without_an_angle_table() {
        let a=(0.0,12.0);let b=(4.0,0.0);let c=(8.0,12.0);
        let m=MiterJoin::new(a,b,c,4.0,800000).unwrap();
        let rev=MiterJoin::new(c,b,a,4.0,800000).unwrap();
        let reflect=|p:Point|(30.0-p.0*3.0,10.0+p.1*3.0);
        let mirror=MiterJoin::new(reflect(a),reflect(b),reflect(c),12.0,800000).unwrap();
        for p in [(4.0,-5.0),(3.5,-3.0),(0.0,0.0),(4.0,1.0),(20.0,20.0)] {
            let d=m.signed_distance(p);
            assert!((rev.signed_distance(p)-d).abs()<1e-10);
            assert!((mirror.signed_distance(reflect(p))-d*3.0).abs()<1e-10);
        }
    }
    #[test]
    fn miter_boundary_has_no_artificial_chord_seam_and_expands_bounds() {
        let m=MiterJoin::new((0.0,0.0),(20.0,0.0),(20.0,20.0),4.0,800000).unwrap();
        assert!(m.signed_distance((21.0,-1.0)) < -0.9);
        assert!(m.signed_distance((22.0,-2.0)).abs()<1e-12);
        assert!(m.signed_distance((22.2,-2.2))>0.0);
        let acute=MiterJoin::new((30.0,70.0),(40.0,30.0),(50.0,70.0),12.0,800000).unwrap();
        assert!(acute.bounds().0.1<6.0);
    }
    #[test]
    fn miter_degenerate_nonfinite_and_near_reversal_are_bounded() {
        for c in [(20.0,0.0),(0.0,0.0),(f64::NAN,1.0)] {
            assert!(MiterJoin::new((0.0,0.0),(10.0,0.0),c,4.0,800000).is_none());
        }
        assert!(MiterJoin::new((0.0,0.0),(10.0,0.0),(10.0,10.0),f64::INFINITY,800000).is_none());
        let near=MiterJoin::new((0.0,0.0),(10.0,0.0),(0.0,0.000001),4.0,800000).unwrap();
        assert!(near.tip.is_none());assert_eq!(near.bounds(),((8.0,-2.0),(12.0,2.0)));
        assert!(near.signed_distance((10.0,0.0)).is_finite());
        // A nearly straight floating-point triangle can have zero area. Its
        // three zero cross products must not make every point inside.
        let flat=MiterJoin::new((0.0,0.0),(10.0,0.0),(20.0,1e-14),4.0,800000).unwrap();
        assert!(flat.signed_distance((10.0,-100.0))>90.0);
    }
}
