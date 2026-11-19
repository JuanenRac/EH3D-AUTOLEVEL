//! The height of the table between the measured points: bilinear (the one of the map) or bicubic (Catmull-Rom, smooth across the points).

use eh3d_levelling::HeightMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Bilinear,
    Bicubic,
}

/// Catmull-Rom between `p1` and `p2` at `t` (0..1), with `p0` and `p3` as the neighbours.
fn catmull_rom(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    0.5 * (2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t * t * t)
}

/// The height at `(x, y)` by bicubic interpolation; outside the grid it is the one of the nearest edge. `None` when a point that is needed was not measured.
pub fn bicubic(map: &HeightMap, x: f64, y: f64) -> Option<f64> {
    let g = &map.grid;
    let (sx, sy) = g.step();
    let fx = ((x - g.x0) / sx).clamp(0.0, (g.cols - 1) as f64);
    let fy = ((y - g.y0) / sy).clamp(0.0, (g.rows - 1) as f64);
    let (c1, r1) = (
        (fx.floor() as usize).min(g.cols - 2),
        (fy.floor() as usize).min(g.rows - 2),
    );
    let (tx, ty) = (fx - c1 as f64, fy - r1 as f64);
    let at = |c: isize, r: isize| {
        // at the edge the missing neighbour is the edge point itself
        let c = c.clamp(0, g.cols as isize - 1) as usize;
        let r = r.clamp(0, g.rows as isize - 1) as usize;
        map.get(r * g.cols + c)
    };
    let mut column = [0.0; 4];
    for (k, slot) in column.iter_mut().enumerate() {
        let r = r1 as isize + k as isize - 1;
        let c = c1 as isize;
        *slot = catmull_rom(at(c - 1, r)?, at(c, r)?, at(c + 1, r)?, at(c + 2, r)?, tx);
    }
    Some(catmull_rom(column[0], column[1], column[2], column[3], ty))
}

/// The height at `(x, y)` with the chosen method.
pub fn height(map: &HeightMap, x: f64, y: f64, method: Method) -> Option<f64> {
    match method {
        Method::Bilinear => map.height_at(x, y),
        Method::Bicubic => bicubic(map, x, y),
    }
}

/// A measured point that differs from what its neighbours say it should be: a reading that is probably wrong (the probe slipped, a chip lay under it) or a sharp step of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spike {
    /// The number of the point in the grid (`row * cols + col`).
    pub index: usize,
    pub col: usize,
    pub row: usize,
    /// Where the point is, in mm.
    pub x: f64,
    pub y: f64,
    /// What was measured and what the neighbours give, in mm.
    pub z: f64,
    pub expected: f64,
}

impl Spike {
    /// How far above (positive) or below (negative) the neighbours the reading is, in mm.
    pub fn deviation(&self) -> f64 {
        self.z - self.expected
    }
}

/// The height the neighbours of a point give: along each axis where both neighbours were measured, the mean of the two, and then the mean of those (for a plane, or any
/// straight slope, it is exactly the height of the point). `None` when no axis has both neighbours measured (a corner, or points that were not measured).
fn expected_from_neighbours(map: &HeightMap, col: usize, row: usize) -> Option<f64> {
    let g = &map.grid;
    let at = |c: usize, r: usize| map.get(r * g.cols + c);
    let mut sums = Vec::with_capacity(2);
    if col > 0 && col + 1 < g.cols {
        if let (Some(a), Some(b)) = (at(col - 1, row), at(col + 1, row)) {
            sums.push((a + b) / 2.0);
        }
    }
    if row > 0 && row + 1 < g.rows {
        if let (Some(a), Some(b)) = (at(col, row - 1), at(col, row + 1)) {
            sums.push((a + b) / 2.0);
        }
    }
    (!sums.is_empty()).then(|| sums.iter().sum::<f64>() / sums.len() as f64)
}

/// The points that read more than `limit_mm` above or below what their neighbours give, in the order of the grid. A limit of 0 (or one that is not a number) finds none: it
/// is the way to switch the check off. A corner has no pair of neighbours and is never judged. The neighbours of a bad reading are off from what their own neighbours give too
/// (by a quarter of it, at most), so only the point that is furthest off of the ones around it is listed.
pub fn spikes(map: &HeightMap, limit_mm: f64) -> Vec<Spike> {
    if !(limit_mm.is_finite() && limit_mm > 0.0) {
        return Vec::new();
    }
    let g = &map.grid;
    let deviation: Vec<Option<(f64, f64, f64)>> = (0..g.len())
        .map(|index| {
            let (col, row) = (index % g.cols, index / g.cols);
            let z = map.get(index)?;
            let expected = expected_from_neighbours(map, col, row)?;
            Some((z, expected, (z - expected).abs()))
        })
        .collect();
    let off = |col: isize, row: isize| -> f64 {
        if col < 0 || row < 0 || col >= g.cols as isize || row >= g.rows as isize {
            return 0.0;
        }
        deviation[row as usize * g.cols + col as usize].map_or(0.0, |d| d.2)
    };
    let mut out = Vec::new();
    for (index, d) in deviation.iter().enumerate() {
        let Some((z, expected, dev)) = *d else {
            continue;
        };
        let (col, row) = (index % g.cols, index / g.cols);
        let (c, r) = (col as isize, row as isize);
        let biggest_around = off(c - 1, r)
            .max(off(c + 1, r))
            .max(off(c, r - 1))
            .max(off(c, r + 1));
        if dev > limit_mm && dev >= biggest_around {
            let (x, y) = g.point(index);
            out.push(Spike {
                index,
                col,
                row,
                x,
                y,
                z,
                expected,
            });
        }
    }
    out
}

/// The line `apply` and `info` show for a spike.
pub fn spike_text(s: &Spike) -> String {
    format!(
        "point {} of the map (column {}, row {}, at X{:.3} Y{:.3}) reads {:+.4} mm against the {:+.4} mm that its neighbours give: {:+.4} mm off",
        s.index + 1,
        s.col + 1,
        s.row + 1,
        s.x,
        s.y,
        s.z,
        s.expected,
        s.deviation()
    )
}

/// The root mean square of the distance, in mm, from the measured points to the plane that is closest to the table (0 for a map that is a plane); `None` when the plane
/// cannot be fitted (fewer than three points measured, or points in a line).
pub fn rms_from_plane(map: &HeightMap) -> Option<f64> {
    let plane = map.plane()?;
    let mut sum = 0.0;
    let mut n = 0usize;
    for i in 0..map.grid.len() {
        if let Some(z) = map.get(i) {
            let (x, y) = map.grid.point(i);
            let d = z - (plane.a * x + plane.b * y + plane.c);
            sum += d * d;
            n += 1;
        }
    }
    (n > 0).then(|| (sum / n as f64).sqrt())
}

/// The numbers (from 1) of the points of the grid that were not measured.
pub fn missing(map: &HeightMap) -> Vec<usize> {
    (0..map.grid.len())
        .filter(|&i| map.get(i).is_none())
        .map(|i| i + 1)
        .collect()
}

/// A number as JSON: `null` when it is not finite (JSON has no way to write it).
pub fn json_number(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "null".into()
    }
}

fn json_option(v: Option<f64>) -> String {
    v.map_or_else(|| "null".to_string(), json_number)
}

/// What `info --json` prints: one JSON object with the grid, how many points were measured and which not, the spread, the tilt, the distance from the plane and the points that
/// read above or below their neighbours by more than `spike_limit_mm`.
pub fn map_json(map: &HeightMap, spike_limit_mm: f64) -> String {
    let g = &map.grid;
    let (sx, sy) = g.step();
    let spread = map.spread();
    let tilt = map.tilt_mm_per_m();
    let missing_items: Vec<String> = missing(map).iter().map(|i| i.to_string()).collect();
    let spike_items: Vec<String> = spikes(map, spike_limit_mm)
        .iter()
        .map(|s| {
            format!(
                "{{\"point\":{},\"column\":{},\"row\":{},\"x\":{},\"y\":{},\"z\":{},\"expected\":{}}}",
                s.index + 1,
                s.col + 1,
                s.row + 1,
                json_number(s.x),
                json_number(s.y),
                json_number(s.z),
                json_number(s.expected)
            )
        })
        .collect();
    format!(
        "{{\"cols\":{},\"rows\":{},\"points\":{},\"measured\":{},\"x0\":{},\"y0\":{},\"x1\":{},\"y1\":{},\"step_x\":{},\"step_y\":{},\"lowest\":{},\"highest\":{},\"spread\":{},\"tilt_x_mm_per_m\":{},\"tilt_y_mm_per_m\":{},\"rms_from_plane\":{},\"missing\":[{}],\"spikes\":[{}]}}",
        g.cols,
        g.rows,
        g.len(),
        map.measured(),
        json_number(g.x0),
        json_number(g.y0),
        json_number(g.x1),
        json_number(g.y1),
        json_number(sx),
        json_number(sy),
        json_option(spread.map(|s| s.0)),
        json_option(spread.map(|s| s.1)),
        json_option(spread.map(|s| s.2)),
        json_option(tilt.map(|t| t.0)),
        json_option(tilt.map(|t| t.1)),
        json_option(rms_from_plane(map)),
        missing_items.join(","),
        spike_items.join(",")
    )
}
