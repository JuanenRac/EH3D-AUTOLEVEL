//! The correction of Z for the turn of the axis A: `r * (1 - cos(A))` added to Z after the height of the table, with values worked out by hand (r = 50 mm: A45 gives 14.6447 mm, A60
//! gives 25, A90 gives 50, A180 gives 100), the settings file and the command line.

use eh3d_autolevel::config::{default_file, parse, parse_a_compensation, parse_a_radius, Settings};
use eh3d_autolevel::gcode::{
    a_axis_notice, a_axis_shift, correct, AAxisCompensation, Options, Report,
};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

/// A table 100 x 100 mm that rises 0.001 mm for every mm along X (`slope` 0.001) or is flat (0).
fn plane(slope: f64) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, slope * x).unwrap();
    }
    m
}

fn tilt(radius: f64, segment: f64) -> Options {
    Options {
        max_segment_mm: segment,
        a_compensation: AAxisCompensation::Tilt,
        a_radius_mm: radius,
        ..Options::default()
    }
}

#[test]
fn the_correction_is_radius_times_one_minus_cosine_of_the_angle() {
    for (r, a, want) in [
        (50.0, 0.0, 0.0),
        (50.0, 45.0, 14.64466094067262),
        (50.0, 60.0, 25.0),
        (50.0, 90.0, 50.0),
        (50.0, 180.0, 100.0),
        (-50.0, 90.0, -50.0),
        (30.0, 30.0, 4.019237886466839),
        (20.0, 120.0, 30.0),
    ] {
        let got = a_axis_shift(r, a);
        assert!(
            (got - want).abs() < 1e-9,
            "r {r} A {a}: {got} instead of {want}"
        );
    }
    // a full turn puts the point back
    assert!(a_axis_shift(50.0, 360.0).abs() < 1e-9);
}

#[test]
fn a_cut_move_gets_the_correction_of_each_piece_after_the_height_of_the_table() {
    // 10 mm in two pieces: A45 and A90; the table adds 0.005 and 0.010
    let (out, r) = correct(
        "G0 X0 Y0\nG1 X10 Y0 A90 F100\n",
        &plane(0.001),
        &tilt(50.0, 5.0),
    )
    .unwrap();
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X5.000 Y0.000 Z14.650 A45.000 F100\nG1 X10.000 Y0.000 Z50.010 A90.000\n"
    );
    assert_eq!(r.a_corrected, 2);
    assert!((r.a_shift_max - 50.0).abs() < 1e-9, "{r:?}");
    // the report of the shift of Z includes it
    assert!((r.shift_max - 50.01).abs() < 1e-9, "{r:?}");
}

#[test]
fn off_changes_nothing_even_with_a_radius() {
    let program = "G0 X0 Y0\nG1 X10 Y0 A90 F100\n";
    let plain = correct(program, &plane(0.001), &Options::default()).unwrap();
    let with_radius = Options {
        a_radius_mm: 50.0,
        ..Options::default()
    };
    let off = correct(program, &plane(0.001), &with_radius).unwrap();
    assert_eq!(plain, off);
    assert_eq!(off.1.a_corrected, 0);
    assert_eq!(off.1.a_shift_max, 0.0);
}

#[test]
fn a_rapid_move_with_z_and_a_move_in_one_piece_use_the_a_where_the_move_ends() {
    let (out, _) = correct("G0 X0 Y0\nG0 X0 Y0 A60 Z2\n", &plane(0.0), &tilt(50.0, 1.0)).unwrap();
    assert_eq!(out, "G0 X0.000 Y0.000\nG0 X0.000 Y0.000 Z27.000 A60\n");
    // a rapid move with no Z has nothing to correct
    let (out, r) = correct("G0 X5 Y0 A60\n", &plane(0.0), &tilt(50.0, 1.0)).unwrap();
    assert_eq!(out, "G0 X5.000 Y0.000 A60\n");
    assert_eq!(r.a_corrected, 0);
    // one piece: the words stay as written, Z has the correction of the end
    let (out, _) = correct("G0 X0 Y0\nG1 X10 A90\n", &plane(0.0), &tilt(-50.0, 10.0)).unwrap();
    assert_eq!(out, "G0 X0.000 Y0.000\nG1 X10.000 Y0.000 Z-50.000 A90\n");
}

#[test]
fn the_axis_is_modal_a_move_without_a_word_keeps_the_correction_of_where_it_stands() {
    let (out, r) = correct(
        "G0 X0 Y0\nG1 X10 A90\nG1 X20\nG1 A180\nG1 X30\n",
        &plane(0.0),
        &tilt(50.0, 10.0),
    )
    .unwrap();
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X10.000 Y0.000 Z50.000 A90\nG1 X20.000 Y0.000 Z50.000\nG1 A180\nG1 X30.000 Y0.000 Z100.000\n"
    );
    assert_eq!(r.a_corrected, 3);
    assert!((r.a_shift_max - 100.0).abs() < 1e-9);
}

#[test]
fn a_program_that_never_turns_a_is_written_the_same_with_the_compensation_on() {
    let program = "G0 X0 Y0\nG1 X10 Y0 F100 M5\n";
    let plain = correct(program, &plane(0.001), &Options::default()).unwrap();
    let on = correct(program, &plane(0.001), &tilt(50.0, 1.0)).unwrap();
    assert_eq!(plain.0, on.0);
    assert_eq!(on.1.a_corrected, 0);
    assert!(a_axis_notice(&on.1, &tilt(50.0, 1.0)).is_none());
}

#[test]
fn the_notice_names_the_moves_the_largest_correction_and_the_radius() {
    let r = Report {
        a_corrected: 2,
        a_shift_max: 50.0,
        ..Report::default()
    };
    assert_eq!(
        a_axis_notice(&r, &tilt(50.0, 1.0)).unwrap(),
        "Z corrected for the turn of axis A on 2 moves, up to +50.0000 mm (a-axis-radius-mm = 50)"
    );
    assert!(a_axis_notice(&r, &Options::default()).is_none());
}

#[test]
fn tilt_needs_a_radius_that_is_not_zero_and_not_beyond_a_thousand() {
    for radius in [0.0, 1000.5, -1000.5, f64::NAN, f64::INFINITY] {
        let e = correct("G0 X0 Y0\n", &plane(0.0), &tilt(radius, 1.0)).unwrap_err();
        assert!(
            e.contains("a-axis-radius-mm is") && e.contains("from -1000 to 1000"),
            "{radius}: {e}"
        );
    }
    assert!(correct("G0 X0 Y0\n", &plane(0.0), &tilt(1000.0, 1.0)).is_ok());
    assert!(correct("G0 X0 Y0\n", &plane(0.0), &tilt(-1000.0, 1.0)).is_ok());
}

#[test]
fn the_settings_file_has_the_compensation_and_the_radius() {
    assert_eq!(
        Settings::default().a_axis_compensation,
        AAxisCompensation::Off
    );
    assert_eq!(Settings::default().a_axis_radius_mm, 0.0);
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
    assert!(default_file().contains("a-axis-compensation = off\n"));
    assert!(default_file().contains("a-axis-radius-mm = 0\n"));
    let s = parse("a-axis-compensation = TILT # on\na-axis-radius-mm = -12.5\n").unwrap();
    assert_eq!(s.a_axis_compensation, AAxisCompensation::Tilt);
    assert_eq!(s.a_axis_radius_mm, -12.5);
    // the radius may come first; off with no radius is fine
    assert!(parse("a-axis-radius-mm = 40\na-axis-compensation = tilt\n").is_ok());
    assert!(parse("a-axis-compensation = off\n").is_ok());
    assert_eq!(
        parse_a_compensation(" Off ").unwrap(),
        AAxisCompensation::Off
    );
    assert_eq!(parse_a_radius(" 1000 ").unwrap(), 1000.0);
    assert_eq!(AAxisCompensation::Tilt.name(), "tilt");
    for (text, line, word) in [
        ("a-axis-compensation = on\n", 1, "must be off or tilt"),
        ("\na-axis-compensation = \n", 2, "must be off or tilt"),
        ("a-axis-radius-mm = 1000.1\n", 1, "from -1000 to 1000 mm"),
        ("a-axis-radius-mm = nan\n", 1, "'nan'"),
        ("a-axis-radius-mm = big\n", 1, "'big'"),
        (
            "a-axis-compensation = off\na-axis-compensation = tilt\n",
            2,
            "twice",
        ),
        ("a-axis-radius-mm = 1\na-axis-radius-mm = 2\n", 2, "twice"),
        ("\na-axis-compensation = tilt\n", 2, "other than 0"),
        (
            "a-axis-compensation = tilt\na-axis-radius-mm = 0\n",
            1,
            "other than 0",
        ),
        ("a_axis_radius_mm = 5\n", 1, "not a setting"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
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
fn the_command_line_applies_the_compensation_warns_and_wins_over_the_file() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("a_axis");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (map, prog, cfg) = (dir.join("map.txt"), dir.join("in.nc"), dir.join("a.cfg"));
    std::fs::write(&map, plane(0.001).to_text()).unwrap();
    std::fs::write(&prog, "G0 X0 Y0\nG1 X10 Y0 A90 F100\n").unwrap();
    let (m, p, c) = (
        map.to_str().unwrap(),
        prog.to_str().unwrap(),
        cfg.to_str().unwrap(),
    );
    let out = |name: &str| dir.join(name).to_str().unwrap().to_string();
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    // nothing asked: no notice and no correction
    let (ok, _, e) = run(&["apply", m, p, &out("plain.nc"), "--segment", "5"]);
    assert!(ok && !e.contains("axis A"), "{e}");
    assert!(read("plain.nc").contains("Z0.010 A90.000"));
    // the options give it: the notice goes to the error output and the file is written
    let (ok, stdout, e) = run(&[
        "apply",
        m,
        p,
        &out("tilt.nc"),
        "--segment",
        "5",
        "--a-compensation",
        "tilt",
        "--a-radius",
        "50",
    ]);
    assert!(ok && stdout.contains("moves corrected"), "{stdout} {e}");
    assert!(
        e.contains("warning: Z corrected for the turn of axis A on 2 moves, up to +50.0000 mm (a-axis-radius-mm = 50)"),
        "{e}"
    );
    let text = read("tilt.nc");
    assert!(
        text.contains("Z14.650 A45.000") && text.contains("Z50.010 A90.000"),
        "{text}"
    );
    // a dry run says it too
    let (ok, stdout, e) = run(&[
        "apply",
        m,
        p,
        "--dry-run",
        "--segment",
        "5",
        "--a-compensation",
        "tilt",
        "--a-radius",
        "50",
    ]);
    assert!(
        ok && stdout.contains("dry run") && e.contains("axis A"),
        "{e}"
    );
    // the file gives both keys, and the options win over it
    std::fs::write(&cfg, "a-axis-compensation = tilt\na-axis-radius-mm = 40\n").unwrap();
    let (ok, _, e) = run(&[
        "apply",
        m,
        p,
        &out("file.nc"),
        "--segment",
        "10",
        "--config",
        c,
    ]);
    assert!(ok && e.contains("(a-axis-radius-mm = 40)"), "{e}");
    // 40 * (1 - cos 90) + 0.010
    assert!(
        read("file.nc").contains("Z40.010 A90"),
        "{}",
        read("file.nc")
    );
    let (ok, _, e) = run(&[
        "apply",
        m,
        p,
        &out("radius.nc"),
        "--segment",
        "10",
        "--config",
        c,
        "--a-radius",
        "50",
    ]);
    assert!(ok && read("radius.nc").contains("Z50.010 A90"), "{e}");
    let (ok, _, e) = run(&[
        "apply",
        m,
        p,
        &out("off.nc"),
        "--segment",
        "10",
        "--config",
        c,
        "--a-compensation",
        "off",
    ]);
    assert!(
        ok && !e.contains("axis A") && read("off.nc").contains("Z0.010 A90"),
        "{e}"
    );
    // wrong values are errors that name the option
    let (ok, _, e) = run(&["apply", m, p, "--dry-run", "--a-compensation", "on"]);
    assert!(
        !ok && e.contains("--a-compensation") && e.contains("off or tilt"),
        "{e}"
    );
    let (ok, _, e) = run(&["apply", m, p, "--dry-run", "--a-radius", "2000"]);
    assert!(
        !ok && e.contains("--a-radius") && e.contains("from -1000 to 1000"),
        "{e}"
    );
    let (ok, _, e) = run(&["apply", m, p, "--dry-run", "--a-compensation", "tilt"]);
    assert!(!ok && e.contains("a-axis-radius-mm is 0"), "{e}");
}
