//! The decimals of the coordinates of the levelled program: three by default, from 1 to 6 as a setting of the settings file or of the command line.

use eh3d_autolevel::config::{default_file, parse, parse_decimals, Settings};
use eh3d_autolevel::gcode::{coord, correct, Options};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-autolevel");

fn flat() -> HeightMap {
    HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3), 0.0)
}

fn with(decimals: usize) -> Options {
    Options {
        decimals,
        ..Options::default()
    }
}

const PROGRAM: &str = "G0 X1.23456 Y2\nG1 X1.23456 Y2 Z-0.12345\n";

#[test]
fn the_coordinates_have_three_decimals_by_default() {
    assert_eq!(Options::default().decimals, 3);
    let (out, _) = correct(PROGRAM, &flat(), &Options::default()).unwrap();
    assert_eq!(out, "G0 X1.235 Y2.000\nG1 X1.235 Y2.000 Z-0.123\n");
}

#[test]
fn the_decimals_follow_the_option_from_one_to_six() {
    let (one, _) = correct(PROGRAM, &flat(), &with(1)).unwrap();
    assert_eq!(one, "G0 X1.2 Y2.0\nG1 X1.2 Y2.0 Z-0.1\n");
    let (six, _) = correct(PROGRAM, &flat(), &with(6)).unwrap();
    assert_eq!(
        six,
        "G0 X1.234560 Y2.000000\nG1 X1.234560 Y2.000000 Z-0.123450\n"
    );
}

#[test]
fn a_number_of_decimals_out_of_range_is_refused_by_the_library() {
    for bad in [0, 7, 100] {
        let e = correct(PROGRAM, &flat(), &with(bad)).unwrap_err();
        assert!(
            e.contains(&format!("decimals is {bad}")) && e.contains("from 1 to 6"),
            "{bad}: {e}"
        );
    }
}

#[test]
fn a_value_that_rounds_to_zero_has_no_sign() {
    assert_eq!(coord(-0.0004, 3), "0.000");
    assert_eq!(coord(-0.0006, 3), "-0.001");
    assert_eq!(coord(0.0, 2), "0.00");
    let (out, _) = correct("G1 X-0.0004 Y0 Z-0.0001\n", &flat(), &with(3)).unwrap();
    assert_eq!(out, "G1 X0.000 Y0.000 Z0.000\n");
}

#[test]
fn the_other_words_keep_their_own_way_of_being_written() {
    let (out, _) = correct("G1 X1 F300.5 S1000\n", &flat(), &Options::default()).unwrap();
    assert_eq!(out, "G1 X1.000 Y0.000 Z0.000 F300.5 S1000\n");
}

#[test]
fn the_settings_file_has_comments_blank_lines_and_the_default() {
    assert_eq!(parse("").unwrap(), Settings::default());
    assert_eq!(parse("# nothing\n\n   \n").unwrap().decimals, 3);
    assert_eq!(parse("decimals = 5 # five\n").unwrap().decimals, 5);
    assert_eq!(parse("  decimals=1\n").unwrap().decimals, 1);
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
}

#[test]
fn a_bad_settings_file_is_an_error_that_says_the_line() {
    for (text, line, word) in [
        ("\n\ndecimals = 9\n", 3, "from 1 to 6"),
        ("decimals = 0\n", 1, "from 1 to 6"),
        ("decimals = 2.5\n", 1, "whole number"),
        ("decimals = -1\n", 1, "whole number"),
        ("decimals = abc\n", 1, "'abc'"),
        ("# c\ndecimals\n", 2, "key = value"),
        ("decimals = 3\nfeed = 1\n", 2, "not a setting"),
        ("decimals = 3\ndecimals = 4\n", 2, "twice"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
    assert_eq!(parse_decimals(" 4 ").unwrap(), 4);
    assert!(parse_decimals("").is_err());
}

#[test]
fn the_example_settings_file_is_the_default_file() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/autolevel.cfg"
    ))
    .unwrap();
    assert_eq!(parse(&text).unwrap(), Settings::default());
    assert_eq!(text.replace("\r\n", "\n"), default_file());
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN).args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn files(tag: &str) -> (String, String, String) {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let map = format!("{dir}/dec-{tag}-map.txt");
    let prog = format!("{dir}/dec-{tag}-in.nc");
    let out = format!("{dir}/dec-{tag}-out.nc");
    let (ok, _, e) = run(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    assert!(ok, "{e}");
    std::fs::write(&prog, "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X10 Y0\nM2\n").unwrap();
    let _ = std::fs::remove_file(&out);
    (map, prog, out)
}

fn has_last_move(out: &str, text: &str) -> bool {
    std::fs::read_to_string(out)
        .unwrap()
        .lines()
        .any(|l| l.starts_with(text))
}

#[test]
fn the_command_line_writes_three_decimals_unless_told_otherwise() {
    let (map, prog, out) = files("default");
    let (ok, _, e) = run(&["apply", &map, &prog, &out]);
    assert!(ok, "{e}");
    assert!(has_last_move(&out, "G1 X10.000 Y0.000 Z"));
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--decimals", "2"]);
    assert!(ok, "{e}");
    assert!(has_last_move(&out, "G1 X10.00 Y0.00 Z"));
}

#[test]
fn the_command_line_refuses_a_bad_number_of_decimals() {
    let (map, prog, out) = files("bad");
    for bad in ["0", "7", "x", "2.5", "-1"] {
        let (ok, _, e) = run(&["apply", &map, &prog, &out, "--decimals", bad]);
        assert!(
            !ok && e.contains("--decimals") && e.contains("from 1 to 6"),
            "{bad}: {e}"
        );
    }
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn the_settings_file_of_the_command_line_is_read_and_the_option_wins() {
    let (map, prog, out) = files("config");
    let cfg = format!("{}/dec-config.cfg", env!("CARGO_TARGET_TMPDIR"));
    std::fs::write(&cfg, "# four\ndecimals = 4\n").unwrap();
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--config", &cfg]);
    assert!(ok, "{e}");
    assert!(has_last_move(&out, "G1 X10.0000 Y0.0000 Z"));
    let (ok, _, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--config",
        &cfg,
        "--decimals",
        "1",
    ]);
    assert!(ok, "{e}");
    assert!(has_last_move(&out, "G1 X10.0 Y0.0 Z"));
    // a bad file is an error with its name and the number of the line
    std::fs::write(&cfg, "decimals = 3\ndecimals = 8\n").unwrap();
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--config", &cfg]);
    assert!(
        !ok && e.contains("dec-config.cfg") && e.contains("line 2"),
        "{e}"
    );
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--config", "no-such-file.cfg"]);
    assert!(!ok && e.contains("no-such-file.cfg"), "{e}");
}

#[test]
fn the_config_command_prints_the_default_file() {
    let (ok, text, e) = run(&["config"]);
    assert!(ok, "{e}");
    assert_eq!(text, default_file());
    let (ok, _, e) = run(&["config", "extra"]);
    assert!(!ok && e.contains("extra"), "{e}");
}
