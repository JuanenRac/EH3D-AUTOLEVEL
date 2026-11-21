//! Arcs (G2 clockwise, G3 counter-clockwise, in the plane of X and Y) cut into straight pieces, so that each piece can get the Z that the map gives for its end. The pieces are chords:
//! every end lies on the circle, and the most that a chord strays from the arc (its sagitta) is the tolerance that is a setting; a piece is never longer than `max_segment_mm` either, so
//! the table is followed along the curve as it is along a straight move. The Z of the arc goes in a straight line from where it was to where it ends (a helix when it changes).
//!
//! The centre is given by the offsets I and J from the start (or by the place itself with G90.1) or by the radius R (a negative R is the arc of more than half a turn); a full circle is an
//! arc whose end is its start and has I or J. What cannot be told is refused with the reason: no centre, a radius that is 0 or too small, an end that is not on the circle of the start.

use std::f64::consts::PI;

/// The smallest and the largest distance an arc may stray from its pieces, in mm.
pub const MIN_ARC_TOLERANCE_MM: f64 = 0.001;
pub const MAX_ARC_TOLERANCE_MM: f64 = 1.0;
/// The default of the tolerance, in mm.
pub const DEFAULT_ARC_TOLERANCE_MM: f64 = 0.01;

/// An angle this small, in radians, is taken as no angle at all.
const EPS_ANGLE: f64 = 1.0e-9;

/// A circular arc: its centre, its radius, the angle it starts at and how far it turns (positive counter-clockwise), in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    pub cx: f64,
    pub cy: f64,
    pub radius: f64,
    pub start_angle: f64,
    pub sweep: f64,
}

impl Arc {
    /// The arc that goes from `start` to `end`. `offset` is I and J (a missing one is 0, but with `absolute_centre` both are needed and they are the centre itself) and `radius` is R; one
    /// of the two forms is needed and not both.
    pub fn from_words(
        start: (f64, f64),
        end: (f64, f64),
        offset: (Option<f64>, Option<f64>),
        radius: Option<f64>,
        counter_clockwise: bool,
        absolute_centre: bool,
    ) -> Result<Arc, String> {
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let centre = match radius {
            Some(r) => {
                if offset.0.is_some() || offset.1.is_some() {
                    return Err("the arc has R and also I or J: give one of the two".into());
                }
                if r == 0.0 {
                    return Err("the radius R of the arc is 0".into());
                }
                let chord = dx.hypot(dy);
                if chord < 1.0e-9 {
                    return Err("an arc with R needs an end other than its start (a full circle needs I and J)".into());
                }
                let ar = r.abs();
                if chord / 2.0 > ar * (1.0 + 1.0e-9) {
                    return Err(format!(
                        "the radius R{r} is smaller than half the distance between the ends of the arc ({:.4} mm)",
                        chord / 2.0
                    ));
                }
                let height = (ar * ar - chord * chord / 4.0).max(0.0).sqrt();
                // the centre of the arc of less than half a turn is on the left of the chord for a counter-clockwise arc and on the right for a clockwise one; a negative R turns it round
                let mut side = if counter_clockwise { 1.0 } else { -1.0 };
                if r < 0.0 {
                    side = -side;
                }
                (
                    (start.0 + end.0) / 2.0 - dy / chord * height * side,
                    (start.1 + end.1) / 2.0 + dx / chord * height * side,
                )
            }
            None => {
                if offset.0.is_none() && offset.1.is_none() {
                    return Err("the arc has no centre (I and J) and no radius (R)".into());
                }
                if absolute_centre {
                    match offset {
                        (Some(i), Some(j)) => (i, j),
                        _ => {
                            return Err("with G90.1 the centre of the arc needs both I and J".into())
                        }
                    }
                } else {
                    (
                        start.0 + offset.0.unwrap_or(0.0),
                        start.1 + offset.1.unwrap_or(0.0),
                    )
                }
            }
        };
        let r0 = (start.0 - centre.0).hypot(start.1 - centre.1);
        let r1 = (end.0 - centre.0).hypot(end.1 - centre.1);
        if !(r0.is_finite() && r1.is_finite()) || r0 < 1.0e-6 {
            return Err("the radius of the arc is 0 or is not a number".into());
        }
        if (r1 - r0).abs() > 0.005 * r0 + 0.005 {
            return Err(format!(
                "the end of the arc is not on the circle that starts at its start (radius {r0:.4} mm at the start, {r1:.4} mm at the end)"
            ));
        }
        let a0 = (start.1 - centre.1).atan2(start.0 - centre.0);
        let a1 = (end.1 - centre.1).atan2(end.0 - centre.0);
        let mut sweep = a1 - a0;
        if counter_clockwise {
            while sweep <= EPS_ANGLE {
                sweep += 2.0 * PI;
            }
        } else {
            while sweep >= -EPS_ANGLE {
                sweep -= 2.0 * PI;
            }
        }
        Ok(Arc {
            cx: centre.0,
            cy: centre.1,
            radius: r0,
            start_angle: a0,
            sweep,
        })
    }

    /// The length of the arc in mm.
    pub fn length(&self) -> f64 {
        self.radius * self.sweep.abs()
    }

    /// How many straight pieces the arc is cut in: enough for the sagitta of a chord not to pass `tolerance_mm`, and for no piece to be longer than `max_segment_mm`; never fewer than
    /// one, and a chord never covers more than a quarter of a turn.
    pub fn pieces(&self, tolerance_mm: f64, max_segment_mm: f64) -> usize {
        let step = if tolerance_mm >= self.radius {
            PI / 2.0
        } else {
            (2.0 * (1.0 - tolerance_mm / self.radius).acos()).min(PI / 2.0)
        };
        let by_tolerance = (self.sweep.abs() / step).ceil();
        let by_length = (self.length() / max_segment_mm).ceil();
        by_tolerance.max(by_length).max(1.0) as usize
    }

    /// The point of the arc a fraction `t` of the way along it (0 at the start, 1 at the end).
    pub fn point(&self, t: f64) -> (f64, f64) {
        let a = self.start_angle + self.sweep * t;
        (
            self.cx + self.radius * a.cos(),
            self.cy + self.radius * a.sin(),
        )
    }

    /// The points where the pieces end, from the first piece to the last, cut in `pieces` pieces.
    pub fn points(&self, pieces: usize) -> Vec<(f64, f64)> {
        (1..=pieces)
            .map(|k| self.point(k as f64 / pieces as f64))
            .collect()
    }
}

/// The value of `arc-tolerance-mm` as the file or the command line gives it: a number from 0.001 to 1 mm.
pub fn parse_tolerance(text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (MIN_ARC_TOLERANCE_MM..=MAX_ARC_TOLERANCE_MM).contains(v))
        .ok_or_else(|| {
            format!("arc-tolerance-mm '{text}' must be a number from {MIN_ARC_TOLERANCE_MM} to {MAX_ARC_TOLERANCE_MM} mm")
        })
}
