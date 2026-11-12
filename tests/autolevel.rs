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
    assert_eq!(out, "G90 G21\nG0 X0.000 Y0.000\nG1 X0.000 Y0.000 Z-0.100 F100\nG1 X1.000 Y0.000 Z-0.100\nG1 X2.000 Y0.000 Z-0.100\nG1 X3.000 Y0.000 Z-0.100\nM2\n");
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
    assert_eq!(lines[0], "G0 X50.000 Y0.000");
    assert!((z_of(lines[1]) - 2.05).abs() < 1e-9, "{}", lines[1]);
    assert!(
        lines[2].starts_with("G0 X10.000 Y10.000 Z1.01") && lines[2].ends_with("F500"),
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
    assert_eq!(
        lines,
        [
            "G1 X1.000 Y0.000 Z0.000 F200 S1000",
            "G1 X2.000 Y0.000 Z0.000"
        ]
    );
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
    let unmeasured = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    let err = correct("G90\nG0 X1 Y1 Z1\n", &unmeasured, &Options::default()).unwrap_err();
    assert!(
        err.contains("no height") || err.contains("measured"),
        "{err}"
    );
}

#[test]
fn an_area_with_no_width_gives_a_finite_height() {
    let flat_in_x = HeightMap::flat(Grid::new((0.0, 0.0, 0.0, 100.0), 3, 3), 0.0);
    let (out, _) = correct("G90\nG0 X1 Y1 Z1\n", &flat_in_x, &Options::default()).unwrap();
    assert!(!out.contains("NaN") && !out.contains("inf"), "{out}");
}

// ---- more edges: comments, modal moves, line ends, piece counts, flat tables and the marks ----

fn flat_map(level: f64) -> HeightMap {
    HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3), level)
}

#[test]
fn a_comment_that_names_a_forbidden_code_is_only_a_comment() {
    let (out, r) = correct(
        "(use G91 and G20 later; G2 too)\nG1 X1 (G91 ; not a comment end) Y0 ; G92\n",
        &flat_map(0.0),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        out.lines().next().unwrap(),
        "(use G91 and G20 later; G2 too)"
    );
    assert!(out.contains("G1 X1.000 Y0.000 Z0.000"), "{out}");
    assert_eq!(r.moves, 1);
}

#[test]
fn a_height_that_is_the_same_everywhere_changes_no_z_whatever_its_value() {
    let program =
        "G90 G21\nG0 X0 Y0 Z5\nG1 Z-0.2 F60\nG1 X37.5 Y12.25\nG1 X60 Y80 Z-0.3\nG0 Z5\nM2\n";
    let opts = Options {
        max_segment_mm: 7.0,
        ..Options::default()
    };
    let (base, _) = correct(program, &flat_map(0.0), &opts).unwrap();
    for level in [0.3, -2.0, 1.0e-9, 123.456] {
        let (out, r) = correct(program, &flat_map(level), &opts).unwrap();
        assert_eq!(out, base, "level {level}");
        assert!(r.shift_min.abs() < 1e-9 && r.shift_max.abs() < 1e-9);
    }
}

#[test]
fn a_move_is_cut_in_exactly_as_many_pieces_as_its_length_asks_for() {
    for (seg, to, pieces) in [
        (2.5, 10.0, 4),
        (3.0, 10.0, 4),
        (10.0, 10.0, 1),
        (10.0, 10.000001, 2),
        (100.0, 10.0, 1),
    ] {
        let (out, r) = correct(
            &format!("G0 X0 Y0\nG1 X{to} Y0\n"),
            &flat_map(0.0),
            &Options {
                max_segment_mm: seg,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(out.lines().count(), 1 + pieces, "seg {seg} to {to}: {out}");
        assert_eq!(r.pieces_added, pieces - 1);
        let last: Vec<&str> = out.lines().last().unwrap().split_whitespace().collect();
        assert_eq!(
            last[1], "X10.000",
            "the last piece ends at the target (to three decimals)"
        );
    }
}

#[test]
fn a_move_with_no_length_is_one_piece_and_a_plunge_keeps_the_position() {
    let (out, r) = correct(
        "G0 X20 Y30\nG1 Z-1\nG1 Z-1\n",
        &flat_map(0.0),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        out,
        "G0 X20.000 Y30.000\nG1 X20.000 Y30.000 Z-1.000\nG1 X20.000 Y30.000 Z-1.000\n"
    );
    assert_eq!((r.moves, r.pieces_added), (2, 0));
}

#[test]
fn the_move_mode_stays_until_another_is_given_and_a_line_with_no_axis_is_left_alone() {
    let (out, _) = correct(
        "G1 F100\nX5 Y0\nG0\nX9 Y0\nG1\nX10\n",
        &flat_map(0.0),
        &Options {
            max_segment_mm: 50.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "G1 F100\nG1 X5.000 Y0.000 Z0.000\nG0\nG0 X9.000 Y0.000\nG1\nG1 X10.000 Y0.000 Z0.000\n"
    );
}

#[test]
fn windows_line_ends_give_the_same_program_as_unix_line_ends() {
    let unix = "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X10 Y0\nM2\n";
    let windows = unix.replace('\n', "\r\n");
    let opts = Options::default();
    let (a, ra) = correct(unix, &tilted(), &opts).unwrap();
    let (b, rb) = correct(&windows, &tilted(), &opts).unwrap();
    assert_eq!((a.clone(), ra), (b.clone(), rb));
    assert!(!b.contains('\r'));
    // a last line without a line end gets one
    let (c, _) = correct("G90\nM2", &tilted(), &opts).unwrap();
    assert_eq!(c, "G90\nM2\n");
}

#[test]
fn the_empty_program_and_a_program_of_comments_come_back_unchanged_with_a_zero_report() {
    let (out, r) = correct("", &flat_map(0.0), &Options::default()).unwrap();
    assert_eq!(
        (out.as_str(), r.moves, r.shift_min, r.shift_max),
        ("", 0, 0.0, 0.0)
    );
    let (out, r) = correct(
        "(nothing)\n\n; nothing\n",
        &flat_map(0.0),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(out, "(nothing)\n\n; nothing\n");
    assert_eq!(r, Default::default());
}

#[test]
fn parameters_expressions_and_stray_characters_are_refused_with_the_line_not_guessed() {
    for (bad, line) in [
        ("G1 X[1+2]", 2),
        ("G1 X5 %", 2),
        ("G1 X1.2.3", 2),
        ("G1 X--1", 2),
        ("G1 X", 2),
    ] {
        let e = correct(
            &format!("G1 X1\n{bad}\n"),
            &flat_map(0.0),
            &Options::default(),
        )
        .unwrap_err();
        assert!(e.starts_with(&format!("line {line}")), "{bad}: {e}");
    }
    // spaces inside a word are allowed by the language: G 1 X 5 is G1 X5
    let (out, _) = correct(
        "G 1 X 5\n",
        &flat_map(0.0),
        &Options {
            max_segment_mm: 50.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(out, "G1 X5.000 Y0.000 Z0.000\n");
}

#[test]
fn numbers_may_have_a_sign_a_leading_dot_or_a_trailing_dot() {
    let (out, _) = correct(
        "G1 X+5. Y.5 Z-.25\n",
        &flat_map(0.0),
        &Options {
            max_segment_mm: 50.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(out, "G1 X5.000 Y0.500 Z-0.250\n");
}

#[test]
fn the_coordinate_limit_is_exact_and_applies_to_every_axis() {
    let flat = flat_map(0.0);
    let o = Options {
        max_segment_mm: 1.0e7,
        ..Options::default()
    };
    assert!(correct("G1 X1000000\n", &flat, &o).is_ok());
    for bad in ["G1 X1000000.01", "G1 Y-1000001", "G1 Z2000000", "G0 Z-1e6"] {
        let r = correct(&format!("{bad}\n"), &flat, &o);
        assert!(r.is_err() || bad == "G0 Z-1e6", "{bad}");
    }
}

#[test]
fn the_report_follows_the_shift_of_the_table_under_the_moves() {
    let (_, r) = correct(
        "G0 X0 Y0\nG1 X100 Y0\n",
        &tilted(),
        &Options {
            max_segment_mm: 20.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(
        (r.shift_min - 0.02).abs() < 1e-9 && (r.shift_max - 0.1).abs() < 1e-9,
        "{r:?}"
    );
    assert_eq!((r.moves, r.pieces_added), (1, 4));
    let (_, r) = correct("G0 X100 Y0 Z1\n", &tilted(), &Options::default()).unwrap();
    assert!(
        (r.shift_min - 0.1).abs() < 1e-9 && r.shift_min == r.shift_max,
        "a rapid with a Z counts as a corrected move: {r:?}"
    );
}

#[test]
fn a_reference_outside_the_grid_uses_the_height_of_the_nearest_edge() {
    let outside = Options {
        reference: (-500.0, 50.0),
        ..Options::default()
    };
    let edge = Options {
        reference: (0.0, 50.0),
        ..Options::default()
    };
    let program = "G0 X0 Y0\nG1 Z-0.1\nG1 X100 Y0\n";
    assert_eq!(
        correct(program, &tilted(), &outside).unwrap().0,
        correct(program, &tilted(), &edge).unwrap().0
    );
}

#[test]
fn bicubic_reaches_the_corners_and_the_edges_of_the_smallest_grid() {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2));
    for (i, h) in [0.0, 0.1, 0.2, 0.35].into_iter().enumerate() {
        m.set(i, h).unwrap();
    }
    for (x, y, want) in [
        (0.0, 0.0, 0.0),
        (10.0, 0.0, 0.1),
        (0.0, 10.0, 0.2),
        (10.0, 10.0, 0.35),
    ] {
        assert!(
            (bicubic(&m, x, y).unwrap() - want).abs() < 1e-12,
            "({x}, {y})"
        );
    }
    let mid = bicubic(&m, 5.0, 5.0).unwrap();
    assert!(mid > 0.0 && mid < 0.35 && mid.is_finite());
}

#[test]
fn bicubic_never_leaves_the_range_of_the_neighbours_by_more_than_the_overshoot_of_a_spline() {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 40.0, 40.0), 5, 5));
    for i in 0..m.grid.len() {
        m.set(i, if i % 2 == 0 { 0.0 } else { 0.05 }).unwrap();
    }
    let mut worst: f64 = 0.0;
    for k in 0..=80 {
        let v = bicubic(&m, f64::from(k) * 0.5, 17.0).unwrap();
        assert!(v.is_finite());
        worst = worst.max((v - 0.025).abs());
    }
    assert!(
        worst < 0.05,
        "a zig-zag of 0.05 mm must not swing by more than that: {worst}"
    );
}

#[test]
fn modal_g_words_on_a_move_line_are_kept() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3), 0.0);
    let (out, _) = correct(
        "G90 G54 G0 X10 Y10\nG21 G1 X10 Y10 Z-0.1 F100\n",
        &flat,
        &Options::default(),
    )
    .unwrap();
    let mut lines = out.lines();
    assert_eq!(lines.next().unwrap(), "G90 G54 G0 X10.000 Y10.000");
    assert_eq!(lines.next().unwrap(), "G21 G1 X10.000 Y10.000 Z-0.100 F100");
}
