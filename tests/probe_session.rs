//! A probing session against a twin of the probe node: the readings of a table, with the faults a probe has (no contact, a garbled line, a reading of another
//! point, a spike) and the way back from them (try again, resume a session that was cut, re-measure the point that does not fit). The twin is deterministic: the
//! same seed gives the same readings. Nothing here has run on a real probe.

use eh3d_autolevel::gcode::{correct, Options};
use eh3d_levelling::{parse_reading, simulated_table, Grid, HeightMap};

/// What goes wrong with a reading of one point, once.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fault {
    /// The probe did not touch: no line comes.
    NoContact,
    /// A line that is not a reading.
    Garbled,
    /// The reading of the next point, as if the probe had moved on.
    WrongPoint,
    /// The right point with a height that is `mm` too high (a chip under the probe).
    Spike(f64),
}

struct ProbeTwin {
    seed: u64,
    pending: Vec<(usize, Fault)>,
}

impl ProbeTwin {
    fn new(seed: u64, faults: &[(usize, Fault)]) -> ProbeTwin {
        ProbeTwin {
            seed,
            pending: faults.to_vec(),
        }
    }

    /// A small deviation that depends only on the seed and the point, so that measuring a point again gives the same figure.
    fn noise(&self, point: usize) -> f64 {
        let mut z = self.seed ^ (point as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 0.004
    }

    fn line(&self, grid: &Grid, point: usize, extra: f64) -> String {
        let (x, y) = grid.point(point);
        format!(
            "Z {x} {y} {:.4}",
            simulated_table(x, y) + self.noise(point) + extra
        )
    }

    /// The line the probe node sends when it is asked to measure `point`, or nothing.
    fn probe(&mut self, grid: &Grid, point: usize) -> Option<String> {
        let at = self.pending.iter().position(|(p, _)| *p == point);
        let fault = at.map(|i| self.pending.remove(i).1);
        match fault {
            None => Some(self.line(grid, point, 0.0)),
            Some(Fault::NoContact) => None,
            Some(Fault::Garbled) => Some("Z 10 banana".to_string()),
            Some(Fault::WrongPoint) => Some(self.line(grid, (point + 1) % grid.len(), 0.0)),
            Some(Fault::Spike(mm)) => Some(self.line(grid, point, mm)),
        }
    }
}

fn grid() -> Grid {
    Grid::new((0.0, 0.0, 200.0, 160.0), 5, 5)
}

/// The session the way the suite runs it: ask for the next point, read the line, check that it is that point, store it. A point is tried up to
/// `attempts` times; the session stops after `stop_after` readings (a cut). The events are returned.
fn session(
    map: &mut HeightMap,
    twin: &mut ProbeTwin,
    attempts: usize,
    stop_after: usize,
) -> Vec<String> {
    let mut events = Vec::new();
    let mut taken = 0;
    while let Some(point) = map.next_point() {
        let mut stored = false;
        for _ in 0..attempts {
            let Some(text) = twin.probe(&map.grid, point) else {
                events.push(format!("no-contact {point}"));
                continue;
            };
            let Ok((x, y, z)) = parse_reading(&text) else {
                events.push(format!("bad-reading {point}"));
                continue;
            };
            let (px, py) = map.grid.point(point);
            if (x - px).abs() > 1e-6 || (y - py).abs() > 1e-6 {
                events.push(format!("wrong-point {point}"));
                continue;
            }
            map.set(point, z).unwrap();
            events.push(format!("ok {point}"));
            stored = true;
            break;
        }
        if !stored {
            // the point stays empty: the session ends with an incomplete map and says so
            break;
        }
        taken += 1;
        if taken >= stop_after {
            break;
        }
    }
    events
}

fn complete(seed: u64) -> HeightMap {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(seed, &[]);
    session(&mut map, &mut twin, 1, usize::MAX);
    map
}

const PROGRAM: &str = "G90 G21\nM3 S10000\nG0 Z2\nG0 X10 Y10\nG1 Z-0.1 F60\nG1 X190 Y10 F200\nG1 X190 Y140\nG0 Z2\nM5\nM2\n";

#[test]
fn a_clean_session_measures_the_table_and_the_program_is_levelled_the_same_every_time() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(1, &[]);
    let events = session(&mut map, &mut twin, 1, usize::MAX);
    assert_eq!(events.len(), 25);
    assert!(events.iter().all(|e| e.starts_with("ok ")));
    assert!(map.is_complete());
    for i in 0..map.grid.len() {
        let (x, y) = map.grid.point(i);
        assert!((map.get(i).unwrap() - simulated_table(x, y)).abs() < 0.003);
    }
    let (a, _) = correct(PROGRAM, &map, &Options::default()).unwrap();
    let (b, _) = correct(PROGRAM, &complete(1), &Options::default()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_probe_that_does_not_touch_is_tried_again() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(1, &[(7, Fault::NoContact)]);
    let events = session(&mut map, &mut twin, 3, usize::MAX);
    assert!(events.contains(&"no-contact 7".to_string()));
    assert!(events.contains(&"ok 7".to_string()));
    assert!(map.is_complete());
    assert_eq!(map.to_text(), complete(1).to_text());
}

#[test]
fn half_a_map_is_refused_and_the_session_resumed_completes_it() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(1, &[(7, Fault::NoContact)]);
    let events = session(&mut map, &mut twin, 1, usize::MAX);
    assert_eq!(events.last().unwrap(), "no-contact 7");
    assert!(!map.is_complete());
    let refused = correct(PROGRAM, &map, &Options::default()).unwrap_err();
    assert!(refused.contains("measured"), "{refused}");
    // the point that failed is the next one to measure, and measuring it again finishes the map
    assert_eq!(map.next_point(), Some(7));
    let rest = session(&mut map, &mut twin, 1, usize::MAX);
    assert_eq!(rest.first().unwrap(), "ok 7");
    assert!(correct(PROGRAM, &map, &Options::default()).is_ok());
}

#[test]
fn a_garbled_line_and_the_reading_of_another_point_are_not_stored() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(1, &[(3, Fault::Garbled), (11, Fault::WrongPoint)]);
    let events = session(&mut map, &mut twin, 2, usize::MAX);
    assert!(events.contains(&"bad-reading 3".to_string()));
    assert!(events.contains(&"wrong-point 11".to_string()));
    assert_eq!(map.to_text(), complete(1).to_text());
}

#[test]
fn a_session_that_was_cut_is_saved_and_resumed_to_the_same_map() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(4, &[]);
    session(&mut map, &mut twin, 1, 10);
    assert_eq!(map.measured(), 10);
    // the power goes: what was measured is kept as text and read back
    let saved = map.to_text();
    let mut resumed = HeightMap::from_text(&saved).unwrap();
    assert_eq!(resumed.measured(), 10);
    let mut twin = ProbeTwin::new(4, &[]);
    let events = session(&mut resumed, &mut twin, 1, usize::MAX);
    assert_eq!(events.len(), 15);
    assert_eq!(resumed.to_text(), complete(4).to_text());
}

#[test]
fn a_spike_is_found_by_how_far_it_is_from_the_plane_and_measured_again() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(2, &[(12, Fault::Spike(5.0))]);
    session(&mut map, &mut twin, 1, usize::MAX);
    assert!(map.is_complete());
    let residuals = |m: &HeightMap| -> Vec<usize> {
        let plane = m.plane().unwrap();
        (0..m.grid.len())
            .filter(|&i| {
                let (x, y) = m.grid.point(i);
                (m.get(i).unwrap() - (plane.a * x + plane.b * y + plane.c)).abs() > 1.0
            })
            .collect()
    };
    assert_eq!(residuals(&map), vec![12]);
    let (_, _, range) = map.spread().unwrap();
    assert!(range > 4.0);
    // measuring the point again (the twin has no spike left) puts it right
    let line = twin.probe(&map.grid, 12).unwrap();
    let (_, _, z) = parse_reading(&line).unwrap();
    map.set(12, z).unwrap();
    assert!(residuals(&map).is_empty());
    assert_eq!(map.to_text(), complete(2).to_text());
}

#[test]
fn a_point_that_does_not_touch_in_the_attempts_given_ends_the_session_with_the_map_incomplete() {
    let mut map = HeightMap::new(grid());
    let mut twin = ProbeTwin::new(3, &[(5, Fault::NoContact)]);
    let events = session(&mut map, &mut twin, 1, usize::MAX);
    // the route goes 0 to 4, then 9 down to 5: nine points are measured before the tenth fails
    assert_eq!(map.measured(), 9);
    assert_eq!(events.last().unwrap(), "no-contact 5");
    assert!(correct(PROGRAM, &map, &Options::default()).is_err());
}
