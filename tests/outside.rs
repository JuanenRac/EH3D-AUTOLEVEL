//! A program that goes outside the measured area, or a reference point outside it, is counted and warned about.

use eh3d_autolevel::gcode::{correct, outside_warning, Options};
use eh3d_levelling::{Grid, HeightMap};

/// A table from (20, 20) to (80, 60), 3 x 3 points, flat.
fn measured() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((20.0, 20.0, 80.0, 60.0), 3, 3));
    for i in 0..m.grid.len() {
        m.set(i, 0.0).unwrap();
    }
    m
}

fn options(reference: (f64, f64)) -> Options {
    Options {
        reference,
        ..Options::default()
    }
}

#[test]
fn moves_outside_the_area_are_counted_with_their_distance() {
    let program = "G90 G21\nG0 X30 Y30\nG1 Z-0.1 F100\nG1 X100 Y30\nM2\n";
    let (_, r) = correct(program, &measured(), &options((30.0, 30.0))).unwrap();
    assert!(r.outside_points > 0, "{r:?}");
    assert!((r.outside_max_mm - 20.0).abs() < 1e-6, "{r:?}");
    assert_eq!(r.reference_outside_mm, 0.0);
    let text = outside_warning(&r).unwrap();
    assert!(text.contains("outside the measured area"), "{text}");
}

#[test]
fn a_program_inside_the_area_is_not_warned_about() {
    let program = "G90 G21\nG0 X30 Y30\nG1 Z-0.1 F100\nG1 X70 Y50\nM2\n";
    let (_, r) = correct(program, &measured(), &options((30.0, 30.0))).unwrap();
    assert_eq!((r.outside_points, r.outside_max_mm), (0, 0.0));
    assert!(outside_warning(&r).is_none());
}

#[test]
fn a_reference_point_outside_the_area_is_said() {
    let program = "G90 G21\nG0 X30 Y30\nG1 Z-0.1 F100\nG1 X70 Y50\nM2\n";
    let (_, r) = correct(program, &measured(), &options((0.0, 0.0))).unwrap();
    assert!(
        (r.reference_outside_mm - 20.0_f64.hypot(20.0)).abs() < 1e-6,
        "{r:?}"
    );
    assert!(outside_warning(&r).unwrap().contains("reference point"));
}
