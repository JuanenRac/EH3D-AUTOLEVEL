//! The warning for a correction of Z that is larger than a limit: values worked out by hand for a table that rises 0.001 mm for every mm along X, the settings file and the command line.

use eh3d_autolevel::config::{default_file, parse, parse_warn_shift, Settings};
use eh3d_autolevel::gcode::{correct, shift_warning, Options, Report};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

/// A table 100 x 100 mm that rises 0.001 mm for every mm along X, 5 x 5 points: at X = 50 it stands 0.05 mm above X = 0.
fn tilted() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, x * 0.001).unwrap();
    }
    m
}

const PROGRAM: &str = "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X50 Y0\nM2\n";

#[test]
fn the_warning_names_the_largest_correction_with_its_sign_and_the_limit() {
    let r = Report {
        moves: 3,
        pieces_added: 0,
        shift_min: -0.12,
        shift_max: 0.35,
    };
    assert_eq!(
        shift_warning(&r, 0.3).unwrap(),
        "the correction of Z reaches +0.3500 mm, above warn-shift-mm = 0.3"
    );
    // the downward correction is the larger one
    let down = Report {
        shift_min: -0.5,
        ..r
    };
    assert_eq!(
        shift_warning(&down, 0.4).unwrap(),
        "the correction of Z reaches -0.5000 mm, above warn-shift-mm = 0.4"
    );
    // not above the limit, no limit, no corrected move
    assert!(shift_warning(&r, 0.5).is_none());
    assert!(shift_warning(&r, 0.0).is_none());
    assert!(shift_warning(&r, f64::NAN).is_none());
    assert!(shift_warning(&Report { moves: 0, ..r }, 0.1).is_none());
}

#[test]
fn a_real_correction_of_fifty_millimetres_along_the_tilt_is_five_hundredths() {
    let (_, r) = correct(PROGRAM, &tilted(), &Options::default()).unwrap();
    assert!(
        (r.shift_max - 0.05).abs() < 1e-9 && r.shift_min.abs() < 1e-9,
        "{r:?}"
    );
    let w = shift_warning(&r, 0.04).unwrap();
    assert!(w.contains("+0.0500") && w.contains("0.04"), "{w}");
    assert!(shift_warning(&r, 0.06).is_none());
}

#[test]
fn the_limit_is_zero_or_from_one_thousandth_to_a_hundred_millimetres() {
    assert_eq!(Settings::default().warn_shift_mm, 0.0);
    assert_eq!(parse("").unwrap().warn_shift_mm, 0.0);
    assert_eq!(
        parse("warn-shift-mm = 0.3 # a third\n")
            .unwrap()
            .warn_shift_mm,
        0.3
    );
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
    assert!(default_file().contains("warn-shift-mm = 0\n"));
    assert_eq!(parse_warn_shift(" 100 ").unwrap(), 100.0);
    assert_eq!(parse_warn_shift("0.001").unwrap(), 0.001);
    assert_eq!(parse_warn_shift("0").unwrap(), 0.0);
    for (text, line) in [
        ("warn-shift-mm = 0.0009\n", 1),
        ("\nwarn-shift-mm = 100.5\n", 2),
        ("warn-shift-mm = -1\n", 1),
        ("warn-shift-mm = nan\n", 1),
        ("warn-shift-mm = inf\n", 1),
        ("warn-shift-mm = big\n", 1),
        ("warn-shift-mm = 1\nwarn-shift-mm = 2\n", 2),
        ("warn_shift_mm = 1\n", 1),
    ] {
        let e = parse(text).unwrap_err();
        assert!(e.starts_with(&format!("line {line}:")), "{text:?}: {e}");
    }
    let e = parse("warn-shift-mm = 0.0009\n").unwrap_err();
    assert!(e.contains("from 0.001 to 100 mm"), "{e}");
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn apply_warns_on_the_error_output_and_writes_the_program_all_the_same() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("warn_shift");
    std::fs::create_dir_all(&dir).unwrap();
    let (map, prog, cfg) = (dir.join("map.txt"), dir.join("in.nc"), dir.join("a.cfg"));
    std::fs::write(&map, tilted().to_text()).unwrap();
    std::fs::write(&prog, PROGRAM).unwrap();
    let (m, p, c) = (
        map.to_str().unwrap(),
        prog.to_str().unwrap(),
        cfg.to_str().unwrap(),
    );
    let out = |name: &str| dir.join(name).to_str().unwrap().to_string();
    // by default there is no warning
    let o = out("plain.nc");
    let (ok, stdout, e) = run(&["apply", m, p, &o]);
    assert!(ok && !e.contains("warning"), "{e}");
    assert!(stdout.contains("moves corrected"), "{stdout}");
    // the option below the correction warns, and the file is written
    let o = out("warned.nc");
    let (ok, _, e) = run(&["apply", m, p, &o, "--warn-shift", "0.04"]);
    assert!(
        ok && e.contains("warning: ") && e.contains("+0.0500"),
        "{e}"
    );
    assert!(std::path::Path::new(&o).exists());
    // above the correction: nothing
    let (ok, _, e) = run(&["apply", m, p, &out("quiet.nc"), "--warn-shift", "0.06"]);
    assert!(ok && !e.contains("warning"), "{e}");
    // a dry run warns too
    let (ok, stdout, e) = run(&["apply", m, p, "--dry-run", "--warn-shift", "0.04"]);
    assert!(
        ok && stdout.contains("dry run") && e.contains("+0.0500"),
        "{e}"
    );
    // the file gives the limit and the option wins over it
    std::fs::write(&cfg, "warn-shift-mm = 0.04\n").unwrap();
    let (ok, _, e) = run(&["apply", m, p, "--dry-run", "--config", c]);
    assert!(ok && e.contains("warning"), "{e}");
    let (ok, _, e) = run(&[
        "apply",
        m,
        p,
        "--dry-run",
        "--config",
        c,
        "--warn-shift",
        "0",
    ]);
    assert!(ok && !e.contains("warning"), "{e}");
    // a limit out of range is an error
    let (ok, _, e) = run(&["apply", m, p, "--dry-run", "--warn-shift", "500"]);
    assert!(
        !ok && e.contains("--warn-shift") && e.contains("from 0.001 to 100 mm"),
        "{e}"
    );
}
