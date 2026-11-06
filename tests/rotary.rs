//! The rotary axes A, B and C: when a cutting move is cut in pieces their words are interpolated in a straight line like X and Y, and the settings file gives the limit of lines.

use eh3d_autolevel::config::{default_file, parse, parse_max_lines, Settings};
use eh3d_autolevel::gcode::{correct, Options};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

fn plane(a: f64) -> HeightMap {
    // z = a * x over a 100 x 100 table, 5 x 5 points
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, a * x).unwrap();
    }
    m
}

fn pieces_of(segment: f64) -> Options {
    Options {
        max_segment_mm: segment,
        ..Options::default()
    }
}

fn level(text: &str, segment: f64) -> String {
    correct(text, &plane(0.001), &pieces_of(segment)).unwrap().0
}

#[test]
fn a_cut_move_interpolates_a_between_where_it_was_and_where_it_goes() {
    // 10 mm in two pieces of 5: the halfway point is A45, the Z of the table comes from X and Y only
    let out = level("G0 X0 Y0\nG1 X10 Y0 A90 F100\n", 5.0);
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X5.000 Y0.000 Z0.005 A45.000 F100\nG1 X10.000 Y0.000 Z0.010 A90.000\n"
    );
}

#[test]
fn the_three_axes_are_written_in_order_with_the_decimals_of_the_settings() {
    let opts = Options {
        max_segment_mm: 2.0,
        decimals: 1,
        ..Options::default()
    };
    let (out, _) = correct("G0 X0 Y0\nG1 X4 C20 B-10\n", &plane(0.0), &opts).unwrap();
    assert_eq!(
        out,
        "G0 X0.0 Y0.0\nG1 X2.0 Y0.0 Z0.0 B-5.0 C10.0\nG1 X4.0 Y0.0 Z0.0 B-10.0 C20.0\n"
    );
}

#[test]
fn the_axes_are_modal_and_a_move_without_the_word_does_not_write_it() {
    let out = level("G0 X0 Y0\nG1 X10 A90\nG1 X20\nG1 X30 A0\n", 5.0);
    let cuts: Vec<&str> = out.lines().filter(|l| l.starts_with("G1")).collect();
    assert_eq!(cuts.len(), 6, "{out}");
    assert!(cuts[0].ends_with("A45.000") && cuts[1].ends_with("A90.000"));
    // the second move has no A word: its pieces have none
    assert!(
        cuts[2].ends_with("Z0.015") && cuts[3].ends_with("Z0.020"),
        "{out}"
    );
    // the third goes from the 90 that is still there down to 0
    assert!(
        cuts[4].ends_with("A45.000") && cuts[5].ends_with("A0.000"),
        "{out}"
    );
}

#[test]
fn a_line_with_only_rotary_words_and_a_rapid_move_set_where_the_axes_stand() {
    let out = level("G0 X0 Y0\nG1 A30\nG1 X10 A90\n", 5.0);
    assert!(out.contains("\nG1 A30\n"), "{out}");
    assert!(
        out.contains("A60.000") && out.trim_end().ends_with("A90.000"),
        "{out}"
    );
    let out = level("G0 X0 Y0 A180\nG1 X10 A0\n", 5.0);
    assert!(out.starts_with("G0 X0.000 Y0.000 A180\n"), "{out}");
    assert!(
        out.contains("A90.000") && out.trim_end().ends_with("A0.000"),
        "{out}"
    );
}

#[test]
fn a_move_in_one_piece_and_a_program_without_rotary_words_are_written_as_before() {
    // one piece: the words stay where they were, as they were written
    let out = level("G0 X0 Y0\nG1 X5 A90 F200 M5\n", 25.0);
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X5.000 Y0.000 Z0.005 A90 F200 M5\n"
    );
    // no A, B or C: the pieces are the same as without the feature
    let out = level("G0 X0 Y0\nG1 X10 F100 M5\n", 5.0);
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X5.000 Y0.000 Z0.005 F100\nG1 X10.000 Y0.000 Z0.010 M5\n"
    );
}

#[test]
fn the_last_piece_ends_exactly_where_the_move_says() {
    // three pieces of a third: 0.1 + 0.1 + 0.1 must not leave 29.999999 behind
    let out = level("G0 X0 Y0\nG1 X3 A30\n", 1.0);
    assert!(out.trim_end().ends_with("A30.000"), "{out}");
    assert!(out.contains("A10.000") && out.contains("A20.000"), "{out}");
}

#[test]
fn the_settings_file_has_the_limit_of_lines() {
    assert_eq!(Settings::default().max_lines, 5_000_000);
    assert_eq!(
        parse("max-lines = 1000 # a thousand\n").unwrap().max_lines,
        1000
    );
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
    assert!(default_file().contains("max-lines = 5000000"));
    assert_eq!(parse_max_lines(" 7 ").unwrap(), 7);
    for (text, line, word) in [
        ("max-lines = 0\n", 1, "from 1 to 100000000"),
        ("\nmax-lines = 100000001\n", 2, "from 1 to 100000000"),
        ("max-lines = 2.5\n", 1, "whole number"),
        ("max-lines = many\n", 1, "'many'"),
        ("max-lines = 5\nmax-lines = 6\n", 2, "twice"),
        ("max_lines = 5\n", 1, "not a setting"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
}

#[test]
fn the_limit_of_the_file_is_used_and_the_option_wins() {
    let bin = env!("CARGO_BIN_EXE_eh3d-autolevel");
    let dir = env!("CARGO_TARGET_TMPDIR");
    let (map, prog, out, cfg) = (
        format!("{dir}/rot-map.txt"),
        format!("{dir}/rot-in.nc"),
        format!("{dir}/rot-out.nc"),
        format!("{dir}/rot.cfg"),
    );
    let run = |args: &[&str]| {
        let o = Command::new(bin).args(args).output().unwrap();
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (ok, e) = run(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    assert!(ok, "{e}");
    std::fs::write(&prog, "G0 X0 Y0\nG1 X10 Y0 A90\n").unwrap();
    std::fs::write(&cfg, "max-lines = 5\n").unwrap();
    // ten pieces of one millimetre do not fit in five lines
    let (ok, e) = run(&["apply", &map, &prog, &out, "--config", &cfg]);
    assert!(!ok && e.contains("more than 5 lines"), "{e}");
    let (ok, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--config",
        &cfg,
        "--max-lines",
        "50",
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(
        text.contains("A9.000") && text.contains("A90.000"),
        "{text}"
    );
}
