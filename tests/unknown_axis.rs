//! A move that leaves out an axis the program has not set yet: taken as 0 (the way it always was) or refused, by setting.

use eh3d_autolevel::config::{default_file, parse, parse_unknown_axis, Settings};
use eh3d_autolevel::gcode::{correct, Options, UnknownAxis};
use eh3d_levelling::{Grid, HeightMap};

fn flat() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    for i in 0..m.grid.len() {
        m.set(i, 0.0).unwrap();
    }
    m
}

fn refusing() -> Options {
    Options {
        unknown_axis: UnknownAxis::Refuse,
        ..Options::default()
    }
}

#[test]
fn by_default_the_program_starts_at_zero_as_it_always_did() {
    assert_eq!(Options::default().unknown_axis, UnknownAxis::AssumeZero);
    let program = "G90 G21\nG1 X10 F100\nM2\n";
    assert!(correct(program, &flat(), &Options::default()).is_ok());
}

#[test]
fn with_refuse_a_move_that_leaves_out_an_axis_not_set_yet_is_refused_with_its_line() {
    let program = "G90 G21\nG1 X10 F100\nM2\n";
    let error = correct(program, &flat(), &refusing()).unwrap_err();
    assert!(
        error.contains("line 2") && error.contains("leaves out Y"),
        "{error}"
    );
}

#[test]
fn with_refuse_a_program_that_sets_every_axis_first_is_corrected() {
    let program = "G90 G21\nG0 X0 Y0 Z5\nG1 X10 F100\nG1 Z-1\nM2\n";
    assert!(correct(program, &flat(), &refusing()).is_ok());
}

#[test]
fn the_setting_reads_and_writes_back() {
    assert_eq!(parse_unknown_axis("Refuse").unwrap(), UnknownAxis::Refuse);
    assert!(parse_unknown_axis("guess").is_err());
    assert_eq!(
        parse("unknown-axis = refuse\n").unwrap().unknown_axis,
        UnknownAxis::Refuse
    );
    assert!(parse("unknown-axis = refuse\nunknown-axis = refuse\n").is_err());
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
    assert!(default_file().contains("unknown-axis = assume-zero\n"));
}
