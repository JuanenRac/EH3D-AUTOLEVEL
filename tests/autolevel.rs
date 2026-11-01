use eh3d_autolevel::gcode::{correct, Options};
use eh3d_autolevel::surface::{bicubic, height, Method};
use eh3d_levelling::{Grid, HeightMap};

/// A table 100 x 100 mm that rises 0.1 mm for every 100 mm along X, 5 x 5 points.
fn tilted() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, x * 0.001).unwrap();
    }
    m
}

fn z_of(line: &str) -> f64 {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix('Z'))
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn a_flat_table_changes_nothing_but_the_cutting_moves_are_cut_in_pieces() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3), 0.0);
    let (out, r) = correct(
        "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X3 Y0\nM2\n",
        &flat,
        &Options::default(),
    )
    .unwrap();
    assert_eq!(out, "G90 G21\nG0 X0 Y0\nG1 X0 Y0 Z-0.1 F100\nG1 X1 Y0 Z-0.1\nG1 X2 Y0 Z-0.1\nG1 X3 Y0 Z-0.1\nM2\n");
    assert_eq!((r.moves, r.pieces_added), (2, 2));
    assert_eq!((r.shift_min, r.shift_max), (0.0, 0.0));
}

#[test]
fn the_z_follows_a_tilted_table() {
    let (out, r) = correct(
        "G0 X0 Y0\nG1 Z-0.1 F50\nG1 X100 Y0\n",
        &tilted(),
        &Options {
            max_segment_mm: 25.0,
            ..Options::default()
        },
    )
    .unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        (z_of(lines[1]) + 0.1).abs() < 1e-9,
        "at the reference the correction is zero: {}",
        lines[1]
    );
    let last = z_of(lines.last().unwrap());
    assert!(
        (last - 0.0).abs() < 1e-9,
        "-0.1 plus 0.1 mm of rise at X100: {last}"
    );
    assert_eq!(
        lines.len(),
        6,
        "a move of 100 mm in pieces of 25 gives 4 pieces: {out}"
    );
    let zs: Vec<f64> = lines[2..].iter().map(|l| z_of(l)).collect();
    assert!(zs.windows(2).all(|w| w[1] > w[0]), "{zs:?}");
    assert!((r.shift_max - 0.1).abs() < 1e-9 && r.shift_min.abs() < 1e-9);
}

#[test]
fn the_reference_point_is_where_the_correction_is_zero() {
    let opts = Options {
        reference: (100.0, 0.0),
        ..Options::default()
    };
    let (out, _) = correct("G0 X0 Y0\nG1 Z-0.1\n", &tilted(), &opts).unwrap();
    assert!(
        (z_of(out.lines().nth(1).unwrap()) + 0.2).abs() < 1e-9,
        "{out}"
    );
}

#[test]
fn a_rapid_move_corrects_its_z_only_when_it_has_one() {
    let (out, _) = correct(
        "G0 X50 Y0\nG0 Z2\nG0 X10 Y10 Z1 F500\n",
        &tilted(),
        &Options::default(),
    )
    .unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "G0 X50 Y0");
    assert!((z_of(lines[1]) - 2.05).abs() < 1e-9, "{}", lines[1]);
    assert!(
        lines[2].starts_with("G0 X10 Y10 Z1.01") && lines[2].ends_with("F500"),
        "{}",
        lines[2]
    );
}

#[test]
fn the_words_that_are_not_a_move_travel_with_the_first_piece() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2), 0.0);
    let (out, _) = correct(
        "g1 x2 y0 f200 s1000 (a comment) ; and another\n",
        &flat,
        &Options {
            max_segment_mm: 1.0,
            ..Options::default()
        },
    )
    .unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, ["G1 X1 Y0 Z0 F200 S1000", "G1 X2 Y0 Z0"]);
}

#[test]
fn what_cannot_be_cut_in_pieces_is_refused_with_its_line() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2), 0.0);
    let o = Options::default();
    assert!(
        correct("G0 X1\nG91\n", &flat, &o)
            .unwrap_err()
            .contains("line 2")
            && correct("G91\n", &flat, &o).unwrap_err().contains("G91")
    );
    assert!(correct("G20\n", &flat, &o).unwrap_err().contains("inches"));
    assert!(correct("G1 X1\nG2 X2 Y2 I1 J0\n", &flat, &o)
        .unwrap_err()
        .contains("arcs"));
    assert!(correct("X5 Y5\n", &flat, &o)
        .unwrap_err()
        .contains("before any G0 or G1"));
    assert!(correct("G1 X1e\n", &flat, &o)
        .unwrap_err()
        .contains("no valid number"));
    assert!(correct("G1 X1 $\n", &flat, &o)
        .unwrap_err()
        .contains("not a letter"));
    assert!(correct("G1 X9999999\n", &flat, &o)
        .unwrap_err()
        .contains("beyond"));
}

#[test]
fn a_move_that_would_make_too_many_pieces_is_refused() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2), 0.0);
    let e = correct(
        "G1 X900000\n",
        &flat,
        &Options {
            max_segment_mm: 0.001,
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(e.contains("pieces"), "{e}");
}

#[test]
fn a_map_with_holes_and_bad_options_are_refused() {
    let partial = HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2));
    assert!(correct("G1 X1\n", &partial, &Options::default()).is_err());
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2), 0.0);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(correct(
            "G1 X1\n",
            &flat,
            &Options {
                max_segment_mm: bad,
                ..Options::default()
            }
        )
        .unwrap_err()
        .contains("max_segment_mm"));
    }
}

#[test]
fn comments_and_blank_lines_and_other_codes_pass_through() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2), 0.0);
    let text = "(a program)\n\nG54\nM3 S12000\nT1 M6\nM5\nM30\n";
    let (out, r) = correct(text, &flat, &Options::default()).unwrap();
    assert_eq!(out, text);
    assert_eq!(r.moves, 0);
}

#[test]
fn bicubic_goes_through_the_measured_points_and_is_smooth_between_them() {
    let m = tilted();
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        assert!((bicubic(&m, x, y).unwrap() - m.get(i).unwrap()).abs() < 1e-9);
    }
    // a plane is reproduced exactly by both methods
    let (b, l) = (
        bicubic(&m, 37.0, 61.0).unwrap(),
        m.height_at(37.0, 61.0).unwrap(),
    );
    assert!((b - l).abs() < 1e-9 && (b - 0.037).abs() < 1e-9);
}

#[test]
fn bicubic_follows_a_curve_better_than_bilinear() {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 40.0, 40.0), 5, 5));
    let f = |x: f64| 0.0005 * (x - 20.0).powi(2);
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, f(x)).unwrap();
    }
    let x = 15.0;
    let (e_cubic, e_linear) = (
        (bicubic(&m, x, 20.0).unwrap() - f(x)).abs(),
        (m.height_at(x, 20.0).unwrap() - f(x)).abs(),
    );
    assert!(
        e_cubic < e_linear,
        "{e_cubic} should be smaller than {e_linear}"
    );
    assert_eq!(height(&m, x, 20.0, Method::Bicubic), bicubic(&m, x, 20.0));
    assert_eq!(height(&m, x, 20.0, Method::Bilinear), m.height_at(x, 20.0));
}

#[test]
fn outside_the_grid_the_nearest_edge_is_used_and_a_hole_in_the_map_is_none() {
    let m = tilted();
    assert!((bicubic(&m, -50.0, 50.0).unwrap() - 0.0).abs() < 1e-9);
    assert!((bicubic(&m, 500.0, 50.0).unwrap() - 0.1).abs() < 1e-9);
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 40.0, 40.0), 5, 5), 0.0);
    assert!(bicubic(
        &HeightMap::new(Grid::new((0.0, 0.0, 40.0, 40.0), 5, 5)),
        10.0,
        10.0
    )
    .is_none());
    assert!(bicubic(&flat, 10.0, 10.0).is_some());
}

#[test]
fn the_bicubic_option_corrects_a_program() {
    let (out, _) = correct(
        "G0 X0 Y0\nG1 Z-0.1\nG1 X50 Y0\n",
        &tilted(),
        &Options {
            method: Method::Bicubic,
            max_segment_mm: 50.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(
        (z_of(out.lines().last().unwrap()) - (-0.1 + 0.05)).abs() < 1e-9,
        "{out}"
    );
}

#[test]
fn codes_that_move_the_origin_or_the_offsets_are_refused_and_never_become_moves() {
    let tilted = tilted();
    let o = Options::default();
    for code in [
        "G92 X0 Y0 Z0",
        "G53 G0 Z-1",
        "G28 X0 Y0",
        "G30",
        "G10 L20 P1 X0 Y0 Z0",
        "G38.2 Z-5",
        "G43 H1 Z5",
    ] {
        let program = format!("G90 G21\nG0 X1 Y1\n{code}\nG1 X5 Y5\n");
        let err = correct(&program, &tilted, &o).unwrap_err();
        assert!(
            err.contains("line 3") && err.contains("not supported"),
            "{code}: {err}"
        );
    }
    // the codes that do not move anything keep going through, and a dwell is not a move
    let (out, _) = correct("%\nG17 G21 G90 G94\nG4 P1\nG4 X2\n%\n", &tilted, &o).unwrap();
    assert_eq!(out, "%\nG17 G21 G90 G94\nG4 P1\nG4 X2\n%\n");
}

#[test]
fn a_map_without_a_usable_height_is_refused_instead_of_writing_nan() {
    let flat_in_x = HeightMap::flat(Grid::new((0.0, 0.0, 0.0, 100.0), 3, 3), 0.0);
    let err = correct("G90\nG0 X1 Y1 Z1\n", &flat_in_x, &Options::default()).unwrap_err();
    assert!(err.contains("no height"), "{err}");
}
