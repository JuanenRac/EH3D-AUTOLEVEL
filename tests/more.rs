//! More tests: the mark of a levelled program, the bicubic surface at the edges and on a plane, and the methods side by side.

use eh3d_autolevel::gcode::{correct, levelled_line, mark, Options, MARK};
use eh3d_autolevel::surface::{bicubic, height, Method};
use eh3d_levelling::{Grid, HeightMap};

fn plane(a: f64, b: f64) -> HeightMap {
    // z = a * x + b * y over a 100 x 100 table, 5 x 5 points
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, a * x + b * y).unwrap();
    }
    m
}

#[test]
fn a_program_without_the_mark_has_no_levelled_line() {
    assert_eq!(levelled_line(""), None);
    assert_eq!(levelled_line("G0 X0 Y0\nM2\n"), None);
    // the word in the middle of a line is not the mark
    assert_eq!(levelled_line("G0 X0 (eh3d-autolevel x)\n"), None);
}

#[test]
fn the_mark_is_found_on_its_line_even_with_blanks_before_it() {
    assert_eq!(levelled_line(&format!("{MARK} 1)\nG0 X0\n")), Some(1));
    assert_eq!(levelled_line(&format!("%\n{MARK} 1)\nG0 X0\n")), Some(2));
    assert_eq!(levelled_line(&format!("G90\nG21\n   {MARK} 1)\n")), Some(3));
}

#[test]
fn marking_puts_the_comment_first_or_after_the_percent_and_keeps_the_program() {
    let map = plane(0.001, 0.0);
    let opts = Options::default();
    let plain = mark("G0 X0 Y0\nM2\n", &map, &opts);
    assert!(plain.starts_with(MARK), "{plain}");
    assert!(plain.ends_with("G0 X0 Y0\nM2\n"));
    assert_eq!(levelled_line(&plain), Some(1));
    let with_percent = mark("%\nG0 X0 Y0\nM2\n%\n", &map, &opts);
    assert!(
        with_percent.starts_with("%\n(eh3d-autolevel"),
        "{with_percent}"
    );
    assert!(with_percent.ends_with("G0 X0 Y0\nM2\n%\n"));
    assert_eq!(levelled_line(&with_percent), Some(2));
}

#[test]
fn the_mark_names_the_map_and_changes_with_it_and_not_with_anything_else() {
    let opts = Options::default();
    let a = mark("M2\n", &plane(0.001, 0.0), &opts);
    let a_again = mark("M2\n", &plane(0.001, 0.0), &opts);
    let b = mark("M2\n", &plane(0.002, 0.0), &opts);
    assert_eq!(a, a_again);
    assert_ne!(a, b);
    assert!(a.contains("map="), "{a}");
    assert!(a.contains(env!("CARGO_PKG_VERSION")), "{a}");
}

#[test]
fn a_program_that_carries_the_mark_is_refused_and_so_is_levelling_it_twice() {
    let map = plane(0.001, 0.0);
    let opts = Options::default();
    let marked = mark("G0 X0 Y0\nG1 Z-0.1 F100\nG1 X50 Y0\nM2\n", &map, &opts);
    let e = correct(&marked, &map, &opts).unwrap_err();
    assert!(!e.is_empty());
    assert!(e.to_lowercase().contains("level"), "{e}");
}

#[test]
fn bicubic_on_a_flat_table_is_that_height_everywhere_and_outside_the_table_too() {
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 4, 4), 0.25);
    for (x, y) in [
        (0.0, 0.0),
        (33.0, 71.0),
        (100.0, 100.0),
        (-40.0, 500.0),
        (250.0, -9.0),
    ] {
        let z = bicubic(&flat, x, y).unwrap();
        assert!((z - 0.25).abs() < 1e-12, "{x},{y}: {z}");
    }
}

#[test]
fn bicubic_follows_a_tilted_plane_exactly_between_the_inner_points() {
    let m = plane(0.001, 0.0005);
    for (x, y) in [(25.0, 25.0), (37.5, 62.5), (50.0, 50.0), (74.0, 26.0)] {
        let z = bicubic(&m, x, y).unwrap();
        let want = 0.001 * x + 0.0005 * y;
        assert!((z - want).abs() < 1e-9, "{x},{y}: {z} vs {want}");
    }
}

#[test]
fn outside_the_table_the_height_is_the_one_of_the_nearest_edge() {
    let m = plane(0.001, 0.0);
    let edge = bicubic(&m, 0.0, 50.0).unwrap();
    assert!((bicubic(&m, -80.0, 50.0).unwrap() - edge).abs() < 1e-12);
    let far = bicubic(&m, 100.0, 50.0).unwrap();
    assert!((bicubic(&m, 900.0, 50.0).unwrap() - far).abs() < 1e-12);
}

#[test]
fn a_map_with_unmeasured_points_has_no_bicubic_height() {
    let m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    assert_eq!(bicubic(&m, 50.0, 50.0), None);
    assert_eq!(height(&m, 50.0, 50.0, Method::Bicubic), None);
    assert_eq!(height(&m, 50.0, 50.0, Method::Bilinear), None);
}

#[test]
fn both_methods_give_the_measured_height_at_the_points_and_agree_on_a_plane() {
    let m = plane(0.002, -0.001);
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        let measured = m.get(i).unwrap();
        for method in [Method::Bilinear, Method::Bicubic] {
            let z = height(&m, x, y, method).unwrap();
            assert!((z - measured).abs() < 1e-9, "{method:?} at {x},{y}");
        }
    }
    let (lin, cub) = (
        height(&m, 41.0, 60.0, Method::Bilinear).unwrap(),
        height(&m, 41.0, 60.0, Method::Bicubic).unwrap(),
    );
    assert!((lin - cub).abs() < 1e-9);
}

#[test]
fn a_segment_length_that_is_not_more_than_zero_is_refused_and_so_is_an_incomplete_map() {
    let map = plane(0.001, 0.0);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let opts = Options {
            max_segment_mm: bad,
            ..Options::default()
        };
        let e = correct("G0 X0 Y0\nM2\n", &map, &opts).unwrap_err();
        assert!(e.contains("max_segment_mm"), "{bad}: {e}");
    }
    let empty = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    assert!(correct("G0 X0 Y0\nM2\n", &empty, &Options::default()).is_err());
}

#[test]
fn the_percent_lines_of_a_postprocessor_are_kept_and_the_levelling_does_not_change_the_xy() {
    let map = plane(0.001, 0.0);
    let (out, _) = correct(
        "%\nG90 G21\nG0 X10 Y20\nG1 Z-0.1 F100\nG1 X30 Y20\nM2\n%\n",
        &map,
        &Options::default(),
    )
    .unwrap();
    assert!(out.starts_with("%\n") && out.ends_with("%\n"), "{out}");
    assert!(out.contains("X30 Y20"));
}
