//! Arcs (G2, G3) cut into straight pieces that follow the table: the pieces lie on the circle, no chord strays from the arc by more than the tolerance of the settings, the height of the map
//! is the one of each piece and the last piece ends where the arc ends. What cannot be told is refused with its line, arcs are refused unless the setting says otherwise, and random
//! programs with arcs are cut or refused without a panic. Tried on a computer only: no arc has been cut on a machine.

use eh3d_autolevel::arcs::{parse_tolerance, Arc};
use eh3d_autolevel::config::{default_file, parse};
use eh3d_autolevel::gcode::{correct, ArcMode, Options};
use eh3d_levelling::{Grid, HeightMap};
use std::f64::consts::PI;
use std::process::Command;

/// A table 100 x 100 mm that rises `sx` mm for every mm along X and `sy` along Y (a plane: the bilinear surface is exact on it).
fn table(sx: f64, sy: f64) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((-50.0, -50.0, 50.0, 50.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, sx * x + sy * y).unwrap();
    }
    m
}

fn cutting(tolerance: f64, segment: f64) -> Options {
    Options {
        arcs: ArcMode::Cut,
        arc_tolerance_mm: tolerance,
        max_segment_mm: segment,
        ..Options::default()
    }
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter))
        .and_then(|v| v.parse().ok())
}

/// The (x, y, z) of every line that is a G1 move.
fn moves(program: &str) -> Vec<(f64, f64, f64)> {
    program
        .lines()
        .filter(|l| l.contains("G1"))
        .map(|l| {
            (
                word(l, 'X').unwrap(),
                word(l, 'Y').unwrap(),
                word(l, 'Z').unwrap(),
            )
        })
        .collect()
}

fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() % 1_000_001) as f64 / 1_000_000.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// The distance from `p` to the segment `a` to `b`.
fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

#[test]
fn an_arc_is_cut_into_pieces_that_follow_the_table() {
    // a quarter of a circle of radius 10 about the origin, counter-clockwise, over a table that is tilted both ways
    let map = table(0.01, 0.02);
    let (out, r) = correct(
        "G90 G21\nG0 X10 Y0\nG3 X0 Y10 I-10 J0 F200\nM2\n",
        &map,
        &cutting(0.02, 100.0),
    )
    .unwrap();
    let pieces = moves(&out);
    assert!(pieces.len() > 4, "{out}");
    assert_eq!(r.arcs_cut, 1);
    assert_eq!(r.pieces_added, pieces.len() - 1);
    assert!(!out.contains('I') && !out.contains("G3"), "{out}");
    // the first piece carries the feed, the last ends where the arc ends
    assert!(out.contains(" F200"), "{out}");
    let last = *pieces.last().unwrap();
    assert!(
        last.0.abs() < 6e-4 && (last.1 - 10.0).abs() < 6e-4,
        "{last:?}"
    );
    let mut previous_angle = 0.0;
    for &(x, y, z) in &pieces {
        // each end is on the circle, goes round counter-clockwise, and its Z is the height of the table there (the plane is 0 at the origin, where the reference is)
        assert!((x.hypot(y) - 10.0).abs() < 1.5e-3, "({x}, {y})");
        let angle = y.atan2(x);
        assert!(angle > previous_angle, "{angle} after {previous_angle}");
        previous_angle = angle;
        assert!((z - (0.01 * x + 0.02 * y)).abs() < 2e-3, "({x}, {y}, {z})");
    }
}

#[test]
fn the_pieces_never_stray_from_the_arc_by_more_than_the_tolerance() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..300 {
        let radius = 0.5 + rng.unit() * 200.0;
        let tolerance = 0.001 + rng.unit() * 0.999;
        let start_angle = rng.unit() * 2.0 * PI;
        let sweep = (0.05 + rng.unit() * 1.95) * PI * if rng.below(2) == 0 { 1.0 } else { -1.0 };
        let start = (radius * start_angle.cos(), radius * start_angle.sin());
        let end = (
            radius * (start_angle + sweep).cos(),
            radius * (start_angle + sweep).sin(),
        );
        // the centre is the origin: the offsets from the start are minus the start
        let arc = Arc::from_words(
            start,
            end,
            (Some(-start.0), Some(-start.1)),
            None,
            sweep > 0.0,
            false,
        )
        .unwrap();
        assert!((arc.sweep - sweep).abs() < 1e-6, "{} / {sweep}", arc.sweep);
        let pieces = arc.pieces(tolerance, 1.0e9);
        let mut points = vec![start];
        points.extend(arc.points(pieces));
        // the pieces end on the circle and together turn through the whole arc
        for p in &points {
            assert!((p.0.hypot(p.1) - radius).abs() < 1e-9 * radius.max(1.0));
        }
        // every chord keeps within the tolerance of the arc, and so does every point of the arc from the chords
        for pair in points.windows(2) {
            let mid = ((pair[0].0 + pair[1].0) / 2.0, (pair[0].1 + pair[1].1) / 2.0);
            assert!(radius - mid.0.hypot(mid.1) <= tolerance + 1e-9);
        }
        for k in 0..=50 {
            let on_arc = arc.point(k as f64 / 50.0);
            let nearest = points
                .windows(2)
                .map(|p| distance_to_segment(on_arc, p[0], p[1]))
                .fold(f64::INFINITY, f64::min);
            assert!(nearest <= tolerance + 1e-9, "{nearest} > {tolerance}");
        }
    }
}

#[test]
fn a_piece_is_never_longer_than_the_segment_either() {
    // a tolerance that asks for few pieces and a short segment that asks for many
    let arc = Arc::from_words(
        (50.0, 0.0),
        (0.0, 50.0),
        (Some(-50.0), Some(0.0)),
        None,
        true,
        false,
    )
    .unwrap();
    let by_tolerance = arc.pieces(1.0, 1.0e9);
    let by_segment = arc.pieces(1.0, 2.0);
    assert!(by_segment > by_tolerance);
    // the chords are no longer than the arc of a piece
    let longest = arc.length() / by_segment as f64;
    assert!(longest <= 2.0 + 1e-9, "{longest}");
}

#[test]
fn a_full_circle_the_radius_form_and_a_helix_are_cut_right() {
    let map = table(0.0, 0.0);
    // a full circle: its end is its start and it has no X or Y at all
    let (out, r) = correct("G0 X10 Y0\nG3 I-10 J0\n", &map, &cutting(0.05, 100.0)).unwrap();
    let ring = moves(&out);
    assert_eq!(r.arcs_cut, 1);
    assert!(ring.len() >= 4, "{out}");
    assert!(ring[0].1 > 0.0, "counter-clockwise goes up first: {ring:?}");
    let last = *ring.last().unwrap();
    assert!(
        (last.0 - 10.0).abs() < 6e-4 && last.1.abs() < 6e-4,
        "{last:?}"
    );
    let (out, _) = correct("G0 X10 Y0\nG2 I-10\n", &map, &cutting(0.05, 100.0)).unwrap();
    assert!(moves(&out)[0].1 < 0.0, "clockwise goes down first: {out}");
    // the radius: a quarter turn clockwise from (0, 0) to (10, 10) about (10, 0), and the arc of three quarters about (0, 10) with a negative R
    let minor = Arc::from_words(
        (0.0, 0.0),
        (10.0, 10.0),
        (None, None),
        Some(10.0),
        false,
        false,
    )
    .unwrap();
    assert!((minor.cx - 10.0).abs() < 1e-9 && minor.cy.abs() < 1e-9);
    assert!((minor.sweep + PI / 2.0).abs() < 1e-9, "{}", minor.sweep);
    let major = Arc::from_words(
        (0.0, 0.0),
        (10.0, 10.0),
        (None, None),
        Some(-10.0),
        false,
        false,
    )
    .unwrap();
    assert!(major.cx.abs() < 1e-9 && (major.cy - 10.0).abs() < 1e-9);
    assert!(
        (major.sweep + 3.0 * PI / 2.0).abs() < 1e-9,
        "{}",
        major.sweep
    );
    // a helix: Z goes in a straight line from where it was to where the arc ends
    let (out, _) = correct(
        "G0 X10 Y0 Z0\nG3 X0 Y10 Z-1 I-10 J0\n",
        &map,
        &cutting(0.05, 100.0),
    )
    .unwrap();
    let helix = moves(&out);
    for pair in helix.windows(2) {
        assert!(pair[1].2 < pair[0].2, "{helix:?}");
    }
    assert!((helix.last().unwrap().2 + 1.0).abs() < 1e-9);
    // the centre given as a place (G90.1) and the offset (G91.1) draw the same arc
    let (a, _) = correct(
        "G91.1\nG0 X10 Y5\nG3 X-10 Y5 I-10 J0\n",
        &map,
        &cutting(0.05, 100.0),
    )
    .unwrap();
    let (b, _) = correct(
        "G90.1\nG0 X10 Y5\nG3 X-10 Y5 I0 J5\n",
        &map,
        &cutting(0.05, 100.0),
    )
    .unwrap();
    assert_eq!(moves(&a), moves(&b));
}

#[test]
fn an_arc_is_modal_and_a_straight_move_ends_it() {
    let map = table(0.0, 0.0);
    let (out, r) = correct(
        "G0 X10 Y0\nG3 X0 Y10 I-10 J0\nX-10 Y0 I0 J-10\nG1 X0 Y0\n",
        &map,
        &cutting(0.05, 100.0),
    )
    .unwrap();
    assert_eq!(r.arcs_cut, 2, "{out}");
    let pieces = moves(&out);
    // the second quarter goes through the third corner of the circle and the last move is the straight one, in one piece of length 10 < 100
    assert!(pieces
        .iter()
        .any(|p| (p.0 + 10.0).abs() < 6e-4 && p.1.abs() < 6e-4));
    assert_eq!(*pieces.last().unwrap(), (0.0, 0.0, 0.0));
}

#[test]
fn what_cannot_be_told_is_refused_with_its_line() {
    let map = table(0.0, 0.0);
    let cut = cutting(0.01, 1.0);
    let refused = |text: &str, opts: &Options| correct(text, &map, opts).unwrap_err();
    // arcs are refused until the setting says otherwise
    let e = refused("G0 X10 Y0\nG3 X0 Y10 I-10\n", &Options::default());
    assert!(
        e.starts_with("line 2:") && e.contains("arcs (G2, G3) are not supported"),
        "{e}"
    );
    // no centre and no radius, a radius that is too small, an end that is not on the circle
    let e = refused("G0 X10 Y0\nG3 X0 Y10\n", &cut);
    assert!(e.starts_with("line 2:") && e.contains("no centre"), "{e}");
    let e = refused("G0 X0 Y0\nG2 X100 Y0 R10\n", &cut);
    assert!(e.contains("smaller than half"), "{e}");
    let e = refused("G0 X10 Y0\nG3 X0 Y20 I-10 J0\n", &cut);
    assert!(e.contains("not on the circle"), "{e}");
    // R together with I, R with the end on the start, G90.1 with only one of the two, several turns, a radius of 0
    let e = refused("G0 X0 Y0\nG2 X10 Y10 R10 I1\n", &cut);
    assert!(e.contains("R and also I or J"), "{e}");
    let e = refused("G0 X5 Y5\nG2 X5 Y5 R3\n", &cut);
    assert!(e.contains("end other than its start"), "{e}");
    let e = refused("G90.1\nG0 X10 Y5\nG3 X-10 Y5 I0\n", &cut);
    assert!(e.contains("both I and J"), "{e}");
    let e = refused("G0 X10 Y0\nG3 X0 Y10 I-10 P2\n", &cut);
    assert!(e.contains("several turns"), "{e}");
    let e = refused("G0 X10 Y0\nG3 I0 J0\n", &cut);
    assert!(e.contains("radius of the arc is 0"), "{e}");
    // the other planes and the rest of what is refused stay refused
    assert!(refused("G18\nG0 X0 Y0\nG2 X5 Z5 I2\n", &cut).contains("G18 is not supported"));
    assert!(refused("G91\nG0 X10 Y0\n", &cut).contains("G91"));
    // the tolerance is checked wherever the options come from
    let bad = cutting(0.0, 1.0);
    assert!(correct("G0 X1 Y1\n", &map, &bad)
        .unwrap_err()
        .contains("arc-tolerance-mm"));
}

#[test]
fn the_settings_and_the_command_line_switch_arcs_on() {
    // the file: off by default, read with its range, a value that makes no sense is an error with its line, a key given twice too
    assert_eq!(parse("").unwrap().arcs, ArcMode::Refuse);
    let s = parse("arcs = cut\narc-tolerance-mm = 0.05\n").unwrap();
    assert_eq!((s.arcs, s.arc_tolerance_mm), (ArcMode::Cut, 0.05));
    assert!(parse("arcs = maybe\n").unwrap_err().starts_with("line 1:"));
    assert!(parse("decimals = 3\narc-tolerance-mm = 5\n")
        .unwrap_err()
        .starts_with("line 2:"));
    assert!(parse("arcs = cut\narcs = refuse\n")
        .unwrap_err()
        .contains("given twice"));
    assert!(parse_tolerance("0.0005").is_err() && parse_tolerance("nan").is_err());
    let defaults = default_file();
    assert!(defaults.contains("arcs = refuse") && defaults.contains("arc-tolerance-mm = 0.01"));
    assert_eq!(parse(&defaults).unwrap(), parse("").unwrap());
    // the file of the repository is the one the command prints
    let shipped = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/autolevel.cfg"
    ))
    .unwrap();
    assert_eq!(lf(&shipped), lf(&defaults));
    // the command line: --arcs cut cuts the arc, without it the program is refused
    let dir = env!("CARGO_TARGET_TMPDIR");
    let (map, prog, out) = (
        format!("{dir}/arcs-map.txt"),
        format!("{dir}/arcs-in.nc"),
        format!("{dir}/arcs-out.nc"),
    );
    let run = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
            .args(args)
            .output()
            .unwrap();
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    assert!(run(&["demo-map", &map, "-50", "-50", "50", "50", "5", "5"]).0);
    std::fs::write(&prog, "G0 X10 Y0\nG3 X0 Y10 I-10 J0\n").unwrap();
    let (ok, _, e) = run(&["apply", &map, &prog, &out]);
    assert!(!ok && e.contains("arcs (G2, G3) are not supported"), "{e}");
    let (ok, o, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--arcs",
        "cut",
        "--arc-tolerance",
        "0.05",
    ]);
    assert!(ok && o.contains("1 arcs cut"), "{o} {e}");
    assert!(!std::fs::read_to_string(&out).unwrap().contains("G3"));
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--arcs", "sometimes"]);
    assert!(!ok && e.contains("--arcs"), "{e}");
    let (ok, _, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--arcs",
        "cut",
        "--arc-tolerance",
        "9",
    ]);
    assert!(!ok && e.contains("--arc-tolerance"), "{e}");
}

#[test]
fn random_programs_with_arcs_are_cut_or_refused_without_a_panic() {
    const WORDS: [&str; 24] = [
        "G0", "G1", "G2", "G3", "G90.1", "G91.1", "X10", "X-20.5", "Y0", "Y15", "Z-1", "Z2",
        "I-10", "I5", "J0", "J-7.5", "R10", "R-12", "R0.001", "P2", "F300", "M3", "A30", "(note)",
    ];
    let map = table(0.001, 0.002);
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let (mut cut, mut refused) = (0, 0);
    for round in 0..600 {
        let mut text = String::from("G0 X10 Y0 Z0\n");
        for _ in 0..1 + rng.below(8) {
            // a word twice and two motion codes on a line are refused, so a line here has one of each
            let mut used = Vec::new();
            for _ in 0..1 + rng.below(6) {
                let word = WORDS[rng.below(WORDS.len())];
                let kind = match word {
                    "G0" | "G1" | "G2" | "G3" => 'g',
                    "G90.1" | "G91.1" | "(note)" => '-',
                    _ => word.chars().next().unwrap_or('-'),
                };
                if kind != '-' && used.contains(&kind) {
                    continue;
                }
                used.push(kind);
                if used.len() > 1 {
                    text.push(' ');
                }
                text.push_str(word);
            }
            text.push('\n');
        }
        let opts = cutting(0.001 + rng.unit(), 0.5 + 20.0 * rng.unit());
        match correct(&text, &map, &opts) {
            Ok((out, r)) => {
                cut += usize::from(r.arcs_cut > 0);
                for l in out.lines() {
                    assert!(
                        !l.contains("NaN") && !l.contains("inf"),
                        "round {round}: {l}"
                    );
                }
            }
            Err(e) => {
                refused += 1;
                assert!(!e.is_empty());
            }
        }
        // the same program cut at any byte is cut or refused too
        if round % 20 == 0 {
            for end in 0..text.len() {
                if text.is_char_boundary(end) {
                    let _ = correct(&text[..end], &map, &opts);
                }
            }
        }
    }
    assert!(cut > 20 && refused > 20, "{cut} cut, {refused} refused");
}
