//! Dashed strokes, as the canvas draws them: phases, intervals, and the
//! polylines and circles a dash pattern cuts.

/// The smallest dash piece that survives, in px.
pub const MIN_PIECE_PX: f64 = 0.01;

/// The dash phase of a pattern: the offset its first piece starts at.
pub fn dash_phase(pattern: [f64; 2]) -> f64 {
    pattern[0] / 2.0
}

/// The dash intervals along a length: on/off pairs from an offset.
pub fn dash_intervals(length: f64, pattern: [f64; 2], offset: f64) -> Vec<[f64; 2]> {
    let period = pattern[0] + pattern[1];
    let mut out: Vec<[f64; 2]> = Vec::new();
    if period <= 0.0 || length <= 0.0 {
        return out;
    }
    let mut o = offset;
    while o > period {
        o -= period;
    }
    while o < 0.0 {
        o += period;
    }
    let mut pos = -o;
    while pos < length {
        let from = pos.max(0.0);
        let to = (pos + pattern[0]).min(length);
        if to - from >= MIN_PIECE_PX {
            out.push([from, to]);
        }
        pos += period;
    }
    out
}

/// Cuts polylines by a dash pattern from an offset.
pub fn dash_polyline(points: &[[f64; 2]], pattern: [f64; 2], offset: f64) -> Vec<Vec<[f64; 2]>> {
    if points.len() < 2 {
        return Vec::new();
    }
    // The distance along the polyline at each vertex.
    let mut along = vec![0.0; points.len()];
    for i in 1..points.len() {
        let dx = points[i][0] - points[i - 1][0];
        let dy = points[i][1] - points[i - 1][1];
        along[i] = along[i - 1] + dx.hypot(dy);
    }
    let length = along[points.len() - 1];
    let at = |seg: usize, d: f64| -> [f64; 2] {
        let span = along[seg + 1] - along[seg];
        let t = if span == 0.0 {
            0.0
        } else {
            (d - along[seg]) / span
        };
        let a = points[seg];
        let b = points[seg + 1];
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    };
    let mut dashes: Vec<Vec<[f64; 2]>> = Vec::new();
    // The segment the walk is in; intervals come in order.
    let mut seg = 0;
    for interval in dash_intervals(length, pattern, offset) {
        while seg + 2 < points.len() && along[seg + 1] <= interval[0] {
            seg += 1;
        }
        let mut dash = vec![at(seg, interval[0])];
        let mut end_seg = seg;
        while end_seg + 2 < points.len() && along[end_seg + 1] < interval[1] {
            end_seg += 1;
            dash.push(points[end_seg]);
        }
        dash.push(at(end_seg, interval[1]));
        dashes.push(dash);
        seg = end_seg;
    }
    dashes
}

/// Cuts a circle of a radius by a dash pattern from an offset.
pub fn dash_circle(r: f64, pattern: [f64; 2], offset: f64) -> Vec<[f64; 2]> {
    let length = 2.0 * std::f64::consts::PI * r;
    let mut arcs = dash_intervals(length, pattern, offset);
    let n = arcs.len();
    if n >= 2 && arcs[0][0] == 0.0 && arcs[n - 1][1] == length {
        arcs[0][0] = arcs[n - 1][0] - length;
        arcs.truncate(n - 1);
    }
    for a in &mut arcs {
        a[0] /= r;
        a[1] /= r;
    }
    arcs
}
