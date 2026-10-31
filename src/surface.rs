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
