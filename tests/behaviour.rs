//! Behaviour at the exact edge of each rule: the words of a G-code line, the way a coordinate is written, the correction of the axis A, the limits of the settings, and the surface
//! between the measured points (also with seeded maps). Nothing here uses a clock or the network.

use eh3d_autolevel::config::{
    parse_arcs, parse_decimals, parse_spike, parse_stream_max_shift, parse_stream_min_z,
    parse_stream_outside, parse_stream_safe_z, MAX_SPIKE_MM,
};
use eh3d_autolevel::gcode::{a_axis_shift, coord, words, ArcMode, MAX_DECIMALS, MIN_DECIMALS};
use eh3d_autolevel::surface::{
    bicubic, height, json_number, missing, rms_from_plane, spike_text, spikes, Method,
};
use eh3d_levelling::{Grid, HeightMap};

/// A small generator with a fixed seed (31 bits of state).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) & 0x7FFF_FFFF;
        self.0 >> 8
    }

    /// A number from `low` to `high`, in steps of a thousandth.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * (self.next() % 1001) as f64 / 1000.0
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn plane_map(a: f64, b: f64, cols: usize, rows: usize) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 120.0, 120.0), cols, rows));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, a * x + b * y).unwrap();
    }
    m
}

fn seeded_map(seed: u64, cols: usize, rows: usize) -> HeightMap {
    let mut rng = Lcg(seed);
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 80.0), cols, rows));
    for i in 0..m.grid.len() {
        m.set(i, rng.between(-1.0, 1.0)).unwrap();
    }
    m
}

// ---------------------------------------------------------------- the words of a line

#[test]
fn the_words_of_a_line_leave_out_the_comments_the_checksum_and_the_blanks_and_read_any_case() {
    assert_eq!(
        words("G1 X10.5 y-2 (note) ; tail", 7).unwrap(),
        vec![('G', 1.0), ('X', 10.5), ('Y', -2.0)]
    );
    assert_eq!(
        words("N10 G1 X5*45", 1).unwrap(),
        vec![('N', 10.0), ('G', 1.0), ('X', 5.0)]
    );
    assert_eq!(
        words("G 1 X 1 0", 1).unwrap(),
        vec![('G', 1.0), ('X', 10.0)]
    );
    assert_eq!(words("( a (b) c ) X1", 1).unwrap(), vec![('X', 1.0)]);
    assert!(words("", 1).unwrap().is_empty());
    assert!(words("(only a comment)", 1).unwrap().is_empty());
    assert!(words("; nothing", 1).unwrap().is_empty());
}

#[test]
fn a_number_may_start_with_a_sign_or_a_point_and_end_with_one() {
    assert_eq!(words("X+3", 1).unwrap(), vec![('X', 3.0)]);
    assert_eq!(words("X.5", 1).unwrap(), vec![('X', 0.5)]);
    assert_eq!(words("X5.", 1).unwrap(), vec![('X', 5.0)]);
    assert_eq!(words("X-0", 1).unwrap(), vec![('X', 0.0)]);
}

#[test]
fn a_word_that_is_not_one_names_its_line_and_what_was_written() {
    let why = words("X", 3).unwrap_err();
    assert!(
        why.contains("line 3") && why.contains("'X'") && why.contains("no valid number"),
        "{why}"
    );
    let why = words("1X", 4).unwrap_err();
    assert!(
        why.contains("line 4") && why.contains("'1'") && why.contains("not a letter"),
        "{why}"
    );
    for bad in ["X1.5.5", "X--1", "X1-", "G1 X", ")", "Y+"] {
        assert!(words(bad, 9).unwrap_err().contains("line 9"), "{bad}");
    }
}

// ---------------------------------------------------------------- the numbers that are written

#[test]
fn a_coordinate_has_the_decimals_asked_and_a_value_that_rounds_to_zero_has_no_sign() {
    assert_eq!(coord(1.5, 3), "1.500");
    assert_eq!(coord(-12.3456, 2), "-12.35");
    assert_eq!(coord(0.0, 3), "0.000");
    assert_eq!(coord(-0.0, 3), "0.000");
    assert_eq!(coord(-0.0004, 3), "0.000");
    assert_eq!(coord(-0.004, 2), "0.00");
    assert_eq!(coord(-0.04, 1), "0.0");
    assert_eq!(coord(-0.06, 1), "-0.1");
    for decimals in MIN_DECIMALS..=MAX_DECIMALS {
        let text = coord(3.25, decimals);
        assert_eq!(
            text.split('.').nth(1).map(str::len),
            Some(decimals),
            "{text}"
        );
    }
}

#[test]
fn a_coordinate_read_back_is_within_half_the_last_decimal_of_the_value() {
    let mut rng = Lcg(404);
    for _ in 0..300 {
        let v = rng.between(-500.0, 500.0);
        for decimals in [1, 3, 6] {
            let back: f64 = coord(v, decimals).parse().unwrap();
            assert!(
                (back - v).abs() <= 0.5 * 10f64.powi(-(decimals as i32)) + 1e-12,
                "{v}"
            );
        }
    }
}

#[test]
fn a_number_in_a_report_is_null_when_json_cannot_write_it() {
    assert_eq!(json_number(1.5), "1.5");
    assert_eq!(json_number(0.0), "0");
    assert_eq!(json_number(-2.0), "-2");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(json_number(bad), "null");
    }
}

// ---------------------------------------------------------------- the axis A and the settings

#[test]
fn the_correction_of_the_axis_a_is_zero_at_zero_and_follows_one_minus_the_cosine() {
    assert_eq!(a_axis_shift(10.0, 0.0), 0.0);
    assert_eq!(a_axis_shift(0.0, 123.0), 0.0);
    assert!(near(a_axis_shift(10.0, 90.0), 10.0));
    assert!(near(a_axis_shift(10.0, 180.0), 20.0));
    assert!(near(a_axis_shift(-10.0, 90.0), -10.0));
    let mut rng = Lcg(12);
    for _ in 0..200 {
        let (r, a) = (rng.between(-100.0, 100.0), rng.between(-720.0, 720.0));
        assert!((a_axis_shift(r, a) - a_axis_shift(r, a + 360.0)).abs() < 1e-6);
        assert!((a_axis_shift(r, a) - a_axis_shift(r, -a)).abs() < 1e-9);
        if r > 0.0 {
            assert!(a_axis_shift(r, a) >= -1e-12 && a_axis_shift(r, a) <= 2.0 * r + 1e-12);
        }
    }
}

#[test]
fn the_limit_of_a_doubtful_reading_is_read_from_nothing_to_the_most_and_not_beyond() {
    assert_eq!(parse_spike("0").unwrap(), 0.0);
    assert_eq!(parse_spike(" 0.5 ").unwrap(), 0.5);
    assert_eq!(parse_spike("10").unwrap(), MAX_SPIKE_MM);
    for bad in ["10.01", "-0.1", "nan", "inf", "", "abc", "1,5"] {
        let why = parse_spike(bad).unwrap_err();
        assert!(why.contains("spike-mm"), "{bad}: {why}");
    }
}

#[test]
fn the_mode_of_the_arcs_is_refuse_or_cut_in_any_case_and_nothing_else() {
    assert_eq!(parse_arcs("REFUSE"), Ok(ArcMode::Refuse));
    assert_eq!(parse_arcs(" Cut "), Ok(ArcMode::Cut));
    for bad in ["", "cutting", "refuse cut", "yes"] {
        assert!(
            parse_arcs(bad)
                .unwrap_err()
                .contains("must be refuse or cut"),
            "{bad}"
        );
    }
}

#[test]
fn each_limit_of_the_stream_has_its_range_with_both_ends_inside() {
    assert_eq!(parse_stream_min_z("-1000").unwrap(), -1000.0);
    assert_eq!(parse_stream_min_z("0").unwrap(), 0.0);
    assert!(parse_stream_min_z("-1000.1")
        .unwrap_err()
        .contains("stream-min-z-mm"));
    assert!(parse_stream_min_z("0.1").is_err());
    assert_eq!(parse_stream_outside("0").unwrap(), 0.0);
    assert_eq!(parse_stream_outside("100").unwrap(), 100.0);
    assert!(parse_stream_outside("100.5")
        .unwrap_err()
        .contains("stream-outside-mm"));
    assert!(parse_stream_outside("-1").is_err());
    assert_eq!(parse_stream_safe_z("0").unwrap(), 0.0);
    assert!(parse_stream_safe_z("-0.1")
        .unwrap_err()
        .contains("stream-safe-z-mm"));
    assert_eq!(parse_stream_max_shift("0").unwrap(), 0.0);
    assert_eq!(parse_stream_max_shift("0.001").unwrap(), 0.001);
    assert_eq!(parse_stream_max_shift("100").unwrap(), 100.0);
    assert!(parse_stream_max_shift("0.0005")
        .unwrap_err()
        .contains("0 (no limit)"));
    assert!(parse_stream_max_shift("100.1").is_err());
    assert!(parse_stream_max_shift("nan").is_err());
}

#[test]
fn the_decimals_are_a_whole_number_from_the_fewest_to_the_most() {
    for d in MIN_DECIMALS..=MAX_DECIMALS {
        assert_eq!(parse_decimals(&d.to_string()), Ok(d));
    }
    assert_eq!(parse_decimals(" 4 "), Ok(4));
    for bad in ["0", "7", "-1", "3.0", "", "x"] {
        assert!(
            parse_decimals(bad).unwrap_err().contains("decimals"),
            "{bad}"
        );
    }
}

// ---------------------------------------------------------------- the surface

#[test]
fn the_bicubic_surface_goes_through_every_measured_point_of_a_seeded_map() {
    for seed in [1, 2, 3, 99] {
        let map = seeded_map(seed, 5, 4);
        for i in 0..map.grid.len() {
            let (x, y) = map.grid.point(i);
            let z = bicubic(&map, x, y).unwrap();
            assert!(near(z, map.get(i).unwrap()), "seed {seed} point {i}: {z}");
            for method in [Method::Bilinear, Method::Bicubic] {
                assert!(near(height(&map, x, y, method).unwrap(), z));
            }
        }
    }
}

#[test]
fn inside_the_grid_a_plane_is_reproduced_exactly_by_both_methods() {
    let map = plane_map(0.002, -0.001, 7, 7);
    let mut rng = Lcg(77);
    for _ in 0..200 {
        let (x, y) = (rng.between(20.0, 80.0), rng.between(20.0, 80.0));
        let want = 0.002 * x - 0.001 * y;
        for method in [Method::Bilinear, Method::Bicubic] {
            let got = height(&map, x, y, method).unwrap();
            assert!(
                (got - want).abs() < 1e-9,
                "{method:?} at ({x}, {y}): {got} not {want}"
            );
        }
    }
}

#[test]
fn outside_the_grid_the_height_is_the_one_of_the_nearest_edge() {
    let map = seeded_map(5, 6, 5);
    let (x1, y1) = (map.grid.x1, map.grid.y1);
    let corner = |x: f64, y: f64| bicubic(&map, x, y).unwrap();
    assert!(near(corner(-500.0, -500.0), map.get(0).unwrap()));
    assert!(near(
        corner(x1 + 500.0, y1 + 500.0),
        map.get(map.grid.len() - 1).unwrap()
    ));
    assert!(near(corner(-50.0, 40.0), corner(0.0, 40.0)));
    assert!(near(corner(50.0, y1 + 9.0), corner(50.0, y1)));
}

#[test]
fn a_point_that_was_not_measured_has_no_height_where_it_is_needed() {
    let mut map = seeded_map(8, 5, 5);
    let hole = 12;
    let kept = map.get(hole).unwrap();
    let mut gap = HeightMap::new(map.grid);
    for i in 0..map.grid.len() {
        if i != hole {
            gap.set(i, map.get(i).unwrap()).unwrap();
        }
    }
    let (x, y) = gap.grid.point(hole);
    assert_eq!(bicubic(&gap, x, y), None);
    assert_eq!(height(&gap, x, y, Method::Bicubic), None);
    assert_eq!(height(&gap, x, y, Method::Bilinear), None);
    assert_eq!(missing(&gap), vec![hole + 1]);
    map.set(hole, kept).unwrap();
    assert!(missing(&map).is_empty());
}

// ---------------------------------------------------------------- the doubtful readings

#[test]
fn a_single_bump_on_a_flat_table_is_the_only_doubtful_reading_and_the_limit_is_strict() {
    let mut map = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5), 0.0);
    map.set(12, 1.0).unwrap();
    let found = spikes(&map, 0.1);
    assert_eq!(found.len(), 1);
    let s = found[0];
    assert_eq!((s.index, s.col, s.row), (12, 2, 2));
    assert_eq!((s.x, s.y, s.z, s.expected), (50.0, 50.0, 1.0, 0.0));
    assert_eq!(s.deviation(), 1.0);
    assert!(spikes(&map, 1.0).is_empty());
    assert_eq!(spikes(&map, 0.999).len(), 1);
    for off in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(spikes(&map, off).is_empty(), "{off}");
    }
    assert!(spikes(&HeightMap::flat(map.grid, 3.0), 0.001).is_empty());
}

#[test]
fn a_reading_that_is_too_low_is_told_with_a_negative_deviation_and_a_corner_is_never_judged() {
    let mut map = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5), 2.0);
    map.set(12, 1.5).unwrap();
    let s = spikes(&map, 0.1)[0];
    assert!(near(s.deviation(), -0.5));
    let text = spike_text(&s);
    assert!(
        text.starts_with("point 13 of the map (column 3, row 3, at X50.000 Y50.000)"),
        "{text}"
    );
    assert!(text.ends_with("-0.5000 mm off"), "{text}");
    let mut corner = HeightMap::flat(map.grid, 0.0);
    corner.set(0, 9.0).unwrap();
    assert!(spikes(&corner, 0.1).iter().all(|s| s.index != 0));
}

#[test]
fn a_map_that_is_a_plane_is_at_no_distance_from_it_and_a_map_with_too_few_points_has_none() {
    let plane = plane_map(0.003, 0.002, 4, 4);
    assert!(rms_from_plane(&plane).unwrap() < 1e-9);
    assert!(spikes(&plane, 0.0001).is_empty());
    assert!(rms_from_plane(&HeightMap::flat(plane.grid, 1.0)).unwrap() < 1e-9);
    let mut bumped = plane.clone();
    bumped.set(5, 5.0).unwrap();
    assert!(rms_from_plane(&bumped).unwrap() > 0.1);
    let mut two = HeightMap::new(plane.grid);
    two.set(0, 0.0).unwrap();
    two.set(1, 0.1).unwrap();
    assert_eq!(rms_from_plane(&two), None);
    assert_eq!(rms_from_plane(&HeightMap::new(plane.grid)), None);
}
