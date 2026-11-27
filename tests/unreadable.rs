//! A line that cannot be read whole is refused: rewriting only the part that was read would send another program to the machine.

use eh3d_autolevel::gcode::{correct, Options};
use eh3d_levelling::{Grid, HeightMap};

fn tilted() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, x * 0.001).unwrap();
    }
    m
}

fn refused(line: &str) -> String {
    let program = format!("G21 G90\nG0 X0 Y0 Z1\n{line}\nM2\n");
    match correct(&program, &tilted(), &Options::default()) {
        Ok((out, _)) => panic!("{line:?} was accepted:\n{out}"),
        Err(why) => {
            assert!(why.starts_with("line 3"), "{line:?}: {why}");
            why
        }
    }
}

#[test]
fn a_star_that_is_not_a_checksum_does_not_cut_the_line() {
    refused("G1 X1*2 Y50 F100");
    refused("G1 X1 * Y50 F100");
    // a checksum, with or without a comment after it, is read as before
    for line in [
        "N10 G1 X10 F100*45",
        "N10 G1 X10 F100*45 ; note",
        "G1 X10 F100 * 7 (note)",
    ] {
        let program = format!("G21 G90\nG0 X0 Y0 Z1\n{line}\n");
        assert!(
            correct(&program, &tilted(), &Options::default()).is_ok(),
            "{line}"
        );
    }
}

#[test]
fn a_word_written_twice_is_not_settled_by_taking_one() {
    for line in [
        "G1 X1 X50 F100",
        "G1 Z-1 Z5",
        "G1 X1 F100 F9000",
        "G0 G1 X5 F100",
        "G1 G0 X5",
        "G1 X5 S100 S9000",
    ] {
        refused(line);
    }
}

#[test]
fn a_comment_that_a_controller_ends_sooner_is_refused() {
    // a controller ends the comment at the first bracket: X50 is a word for it
    refused("G1 X1 (a (b) X50) F100");
    refused("G1 X1 ) F100");
}

#[test]
fn a_parameter_line_may_not_carry_a_move() {
    for line in [
        "#1=5 G0 X100",
        "#1=5 G1 Z-3",
        "#<depth>=1 M3",
        "#1 G0 X5",
        "#=5",
    ] {
        refused(line);
    }
    let program = "#1=5\n#<depth> = -0.5 (a note)\n#2=#1\nG21 G90\n";
    assert!(correct(program, &tilted(), &Options::default()).is_ok());
}

#[test]
fn a_control_character_never_passes() {
    for line in [
        "G90 (a\u{18}b)",
        "M5 ; \u{3}",
        "M117 hi\u{18}",
        "$G\u{0}",
        "G1\u{a0}X5 F100",
    ] {
        refused(line);
    }
    // a tab and the accents of a comment are ordinary
    let program = "G21\tG90\n(diseño)\n; Ø0.8\n";
    assert!(correct(program, &tilted(), &Options::default()).is_ok());
}

#[test]
fn a_work_offset_that_is_not_a_whole_one_is_refused() {
    refused("G54.5 G0 X5");
}
