//! A program is never levelled twice, and the command line refuses what it does not understand.

use eh3d_autolevel::gcode::{correct, levelled_line, mark, Options};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

fn tilted() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, x * 0.001).unwrap();
    }
    m
}

const PROGRAM: &str = "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X10 Y0\nM2\n";

#[test]
fn a_levelled_program_carries_a_mark_and_is_refused_the_second_time() {
    let map = tilted();
    let opts = Options::default();
    let (out, _) = correct(PROGRAM, &map, &opts).unwrap();
    assert!(levelled_line(&out).is_none(), "the bare result has no mark");
    let marked = mark(&out, &map, &opts);
    assert_eq!(levelled_line(&marked), Some(1));
    let first = marked.lines().next().unwrap();
    assert!(
        first.starts_with("(eh3d-autolevel ")
            && first.contains("map=")
            && first.contains("ref=0,0")
            && first.contains("Bilinear")
            && first.contains("seg=1)"),
        "{first}"
    );
    // the comment is a comment: the rest of the program is unchanged
    assert_eq!(&marked[first.len() + 1..], out);
    // levelling it again is refused with the line, and levelling the original again is fine
    let e = correct(&marked, &map, &opts).unwrap_err();
    assert!(
        e.contains("line 1") && e.contains("already levelled"),
        "{e}"
    );
    assert!(correct(PROGRAM, &map, &opts).is_ok());
}

#[test]
fn the_mark_goes_after_a_percent_line_and_tells_the_map_apart() {
    let map = tilted();
    let opts = Options::default();
    let marked = mark("%\nG90\nM2\n%\n", &map, &opts);
    assert!(marked.starts_with("%\n(eh3d-autolevel "));
    assert_eq!(levelled_line(&marked), Some(2));
    // another map gives another hash
    let flat = HeightMap::flat(Grid::new((0.0, 0.0, 100.0, 100.0), 3, 3), 0.0);
    assert_ne!(mark("G90\n", &map, &opts), mark("G90\n", &flat, &opts));
    // the mark is found even when other lines come first and it is indented
    assert_eq!(
        levelled_line("G90\n  (eh3d-autolevel 0.0.1 map=00000000)\nM2\n"),
        Some(2)
    );
    assert_eq!(levelled_line("(a comment)\nG90\n"), None);
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_command_line_refuses_options_and_words_it_does_not_know() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let map = format!("{dir}/strict-map.txt");
    let (ok, _) = run(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    assert!(ok);
    // an option that is written wrong is an error, not the default
    let (ok, e) = run(&["info", &map, "--powr", "5"]);
    assert!(
        !ok && e.contains("--powr") && e.contains("unknown option"),
        "{e}"
    );
    let (ok, e) = run(&["info", &map, "extra"]);
    assert!(
        !ok && e.contains("extra") && e.contains("unexpected argument"),
        "{e}"
    );
    let prog = format!("{dir}/strict-in.nc");
    let out = format!("{dir}/strict-out.nc");
    std::fs::write(&prog, PROGRAM).unwrap();
    let (ok, e) = run(&["apply", &map, &prog, &out, "--segmnt", "2"]);
    assert!(!ok && e.contains("--segmnt"), "{e}");
    let (ok, e) = run(&["apply", &map, &prog, &out, "again"]);
    assert!(!ok && e.contains("again"), "{e}");
    // what is right still works, writes the mark, and its output cannot be levelled again
    let (ok, e) = run(&["apply", &map, &prog, &out, "--segment", "2"]);
    assert!(ok, "{e}");
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(
        written.starts_with("; generator: eh3d-autolevel "),
        "{written}"
    );
    assert!(written.contains("\n(eh3d-autolevel "), "{written}");
    let again = format!("{dir}/strict-again.nc");
    let (ok, e) = run(&["apply", &map, &out, &again]);
    assert!(!ok && e.contains("already levelled"), "{e}");
    assert!(!std::path::Path::new(&again).exists());
}

// ---- the marks and the command line, more edges ----

fn code(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str) -> String {
    format!("{}/more-{name}", env!("CARGO_TARGET_TMPDIR"))
}

#[test]
fn the_same_program_levelled_twice_gives_the_same_bytes_including_the_mark() {
    let map = tilted();
    let opts = Options::default();
    let one = mark(&correct(PROGRAM, &map, &opts).unwrap().0, &map, &opts);
    let two = mark(&correct(PROGRAM, &map, &opts).unwrap().0, &map, &opts);
    assert_eq!(one, two);
}

#[test]
fn the_mark_tells_the_options_apart() {
    use eh3d_autolevel::surface::Method;
    let map = tilted();
    let base = mark("G90\n", &map, &Options::default());
    for other in [
        Options {
            reference: (5.0, 0.0),
            ..Options::default()
        },
        Options {
            max_segment_mm: 0.25,
            ..Options::default()
        },
        Options {
            method: Method::Bicubic,
            ..Options::default()
        },
    ] {
        assert_ne!(
            mark("G90\n", &map, &other).lines().next(),
            base.lines().next(),
            "{other:?}"
        );
    }
    assert!(mark(
        "G90\n",
        &map,
        &Options {
            max_segment_mm: 0.25,
            ..Options::default()
        }
    )
    .contains("seg=0.25)"));
}

#[test]
fn a_comment_that_only_looks_like_the_mark_does_not_stop_a_program() {
    let map = tilted();
    for text in [
        "(eh3d autolevel was not run)\nG90\n",
        "G90 (eh3d-autolevel in a comment after code)\n",
        "; eh3d-autolevel\nG90\n",
    ] {
        assert!(levelled_line(text).is_none(), "{text}");
        assert!(correct(text, &map, &Options::default()).is_ok(), "{text}");
    }
}

#[test]
fn the_command_line_reports_a_bad_segment_a_bad_reference_and_a_missing_file() {
    let map = tmp("map.txt");
    let prog = tmp("in.nc");
    let out = tmp("out.nc");
    assert_eq!(
        code(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]).0,
        Some(0)
    );
    std::fs::write(&prog, PROGRAM).unwrap();
    let _ = std::fs::remove_file(&out);
    for (args, word) in [
        (
            vec!["apply", &map, &prog, &out, "--segment", "0"],
            "max_segment_mm",
        ),
        (
            vec!["apply", &map, &prog, &out, "--segment", "-2"],
            "max_segment_mm",
        ),
        (
            vec!["apply", &map, &prog, &out, "--segment", "fast"],
            "--segment",
        ),
        (
            vec!["apply", &map, &prog, &out, "--segment"],
            "needs a value",
        ),
        (vec!["apply", &map, &prog, &out, "--ref", "5"], "X,Y"),
        (
            vec!["apply", &map, &prog, &out, "--ref", "5,abc"],
            "--ref Y",
        ),
        (
            vec!["apply", &map, &prog, &out, "--ref", "inf,0"],
            "--ref X",
        ),
        (vec!["apply", &tmp("nope.txt"), &prog, &out], "nope.txt"),
        (vec!["apply", &map, &tmp("nope.nc"), &out], "nope.nc"),
        (vec!["apply", &map], "needs a map"),
        (vec!["info"], "needs a map"),
        (vec!["info", &tmp("nope.txt")], "nope.txt"),
    ] {
        let (c, _, e) = code(&args);
        assert_eq!(c, Some(1), "{args:?}: {e}");
        assert!(e.starts_with("error:") && e.contains(word), "{args:?}: {e}");
    }
    assert!(
        !std::path::Path::new(&out).exists(),
        "nothing is written after an error"
    );
}

#[test]
fn the_command_line_never_levels_a_file_over_itself_and_never_levels_it_twice() {
    let map = tmp("inplace-map.txt");
    let prog = tmp("inplace.nc");
    code(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    std::fs::write(&prog, PROGRAM).unwrap();
    let (c, _, e) = code(&["apply", &map, &prog, &prog]);
    assert_eq!(c, Some(1));
    assert!(e.contains("also the program"), "{e}");
    assert_eq!(std::fs::read_to_string(&prog).unwrap(), PROGRAM);
    let out = tmp("inplace-out.nc");
    let (c, _, e) = code(&["apply", &map, &prog, &out]);
    assert_eq!(c, Some(0), "{e}");
    let first = std::fs::read_to_string(&out).unwrap();
    assert!(first.starts_with("; generator: eh3d-autolevel "), "{first}");
    assert!(first.contains("\n(eh3d-autolevel "), "{first}");
    let (c, _, e) = code(&["apply", &map, &out, &tmp("inplace-again.nc")]);
    assert_eq!(c, Some(1));
    assert!(e.contains("already levelled"), "{e}");
}

#[test]
fn the_command_line_names_an_output_folder_that_does_not_exist() {
    let map = tmp("nofolder-map.txt");
    let prog = tmp("nofolder.nc");
    code(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    std::fs::write(&prog, PROGRAM).unwrap();
    let missing = format!("{}/no/such/folder/out.nc", env!("CARGO_TARGET_TMPDIR"));
    let (c, _, e) = code(&["apply", &map, &prog, &missing]);
    assert_eq!(c, Some(1));
    assert!(e.contains("out.nc"), "{e}");
}

#[test]
fn the_demo_map_checks_its_numbers_and_never_panics_on_strange_corners() {
    let out = tmp("demo.txt");
    for (args, ok) in [
        (
            vec!["demo-map", &out, "0", "0", "100", "100", "1", "3"],
            false,
        ),
        (
            vec!["demo-map", &out, "0", "0", "100", "100", "3", "201"],
            false,
        ),
        (
            vec!["demo-map", &out, "0", "0", "100", "100", "2.5", "3"],
            false,
        ),
        (
            vec!["demo-map", &out, "0", "0", "100", "100", "-3", "3"],
            false,
        ),
        (
            vec!["demo-map", &out, "0", "0", "100", "100", "200", "200"],
            true,
        ),
        (vec!["demo-map", &out, "0", "0", "100"], false),
        (
            vec!["demo-map", &out, "0", "0", "nan", "100", "3", "3"],
            false,
        ),
    ] {
        let (c, _, e) = code(&args);
        assert_eq!(c, Some(if ok { 0 } else { 1 }), "{args:?}: {e}");
    }
    // corners that are equal or reversed are odd but must end with an exit code, never a crash
    for corners in [
        ["0", "0", "0", "0"],
        ["100", "100", "0", "0"],
        ["5", "5", "5", "50"],
    ] {
        let args = [
            "demo-map", &out, corners[0], corners[1], corners[2], corners[3], "3", "3",
        ];
        let (c, _, e) = code(&args);
        assert!(matches!(c, Some(0) | Some(1)), "{corners:?}: {c:?} {e}");
    }
}

#[test]
fn info_describes_the_map_and_a_cut_map_file_is_an_error_that_names_it() {
    let map = tmp("info-map.txt");
    code(&["demo-map", &map, "0", "0", "100", "100", "4", "3"]);
    let (c, o, _) = code(&["info", &map]);
    assert_eq!(c, Some(0));
    assert!(
        o.contains("12 of 12 points measured") && o.contains("spread") && o.contains("tilt"),
        "{o}"
    );
    let text = std::fs::read_to_string(&map).unwrap();
    let cut = tmp("info-cut.txt");
    let mut found_error = false;
    for end in [0, 5, text.len() / 2, text.len() - 3] {
        if !text.is_char_boundary(end) {
            continue;
        }
        std::fs::write(&cut, &text[..end]).unwrap();
        let (c, _, e) = code(&["info", &cut]);
        assert!(
            matches!(c, Some(0) | Some(1)),
            "no crash on a map cut at {end}"
        );
        if c == Some(1) {
            found_error = true;
            assert!(e.contains("info-cut.txt"), "{e}");
        }
    }
    assert!(
        found_error,
        "a map cut in the middle must be refused at least once"
    );
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
