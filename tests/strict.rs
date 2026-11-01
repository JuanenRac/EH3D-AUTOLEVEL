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
    assert!(std::fs::read_to_string(&out)
        .unwrap()
        .starts_with("(eh3d-autolevel "));
    let again = format!("{dir}/strict-again.nc");
    let (ok, e) = run(&["apply", &map, &out, &again]);
    assert!(!ok && e.contains("already levelled"), "{e}");
    assert!(!std::path::Path::new(&again).exists());
}
