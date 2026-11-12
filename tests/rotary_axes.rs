//! The three rotary axes: A and B turn the tool about X and about Y, so each tilts the point that is cut and adds `r * (1 - cos(angle))` to Z (with its own radius, the two add up);
//! C turns about Z and never changes the height. Besides the values worked out by hand, a simulated machine walks the levelled program and checks, at every written line, that the depth
//! of the tool below the table is the depth the original program asked for, with the axes turning inside moves that are cut in pieces. Nothing has run on a machine with rotary axes.

use eh3d_autolevel::config::{default_file, parse, parse_b_compensation, parse_b_radius, Settings};
use eh3d_autolevel::gcode::{
    a_axis_shift, b_axis_notice, correct, AAxisCompensation, Options, Report,
};
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

/// A table 100 x 100 mm that rises `slope` mm for every mm along X and `slope_y` along Y.
fn table(slope: f64, slope_y: f64) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, slope * x + slope_y * y).unwrap();
    }
    m
}

fn both(ra: f64, rb: f64, segment: f64) -> Options {
    Options {
        max_segment_mm: segment,
        a_compensation: AAxisCompensation::Tilt,
        a_radius_mm: ra,
        b_compensation: AAxisCompensation::Tilt,
        b_radius_mm: rb,
        ..Options::default()
    }
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter))
        .and_then(|v| v.parse().ok())
}

#[test]
fn b_gives_the_same_correction_as_a_with_its_own_radius() {
    let opts = Options {
        max_segment_mm: 50.0,
        b_compensation: AAxisCompensation::Tilt,
        b_radius_mm: 50.0,
        ..Options::default()
    };
    let (out, r) = correct("G0 X0 Y0\nG1 X10 B90\nG1 B180\n", &table(0.0, 0.0), &opts).unwrap();
    assert_eq!(
        out,
        "G0 X0.000 Y0.000\nG1 X10.000 Y0.000 Z50.000 B90\nG1 B180\n"
    );
    // the line with only B has no X, Y or Z: it is carried as it is, the way A is
    assert_eq!(r.b_corrected, 1);
    assert!((r.b_shift_max - 50.0).abs() < 1e-9);
    assert_eq!(r.a_corrected, 0);
    let (out, r) = correct(
        "G0 X0 Y0\nG1 X10 Y0 B90 F100\nG1 X20 Y0\n",
        &table(0.0, 0.0),
        &opts,
    )
    .unwrap();
    assert!(
        out.contains("Z50.000 B90") && out.ends_with("G1 X20.000 Y0.000 Z50.000\n"),
        "{out}"
    );
    assert_eq!(r.b_corrected, 2);
}

#[test]
fn a_and_b_add_up_and_the_notice_names_b() {
    let (out, r) = correct(
        "G0 X0 Y0\nG1 X10 Y0 A90 B90 F100\n",
        &table(0.0, 0.0),
        &both(50.0, 30.0, 50.0),
    )
    .unwrap();
    assert!(out.contains("Z80.000"), "{out}");
    assert!((r.a_shift_max - 50.0).abs() < 1e-9 && (r.b_shift_max - 30.0).abs() < 1e-9);
    assert_eq!(
        b_axis_notice(&r, &both(50.0, 30.0, 50.0)).unwrap(),
        "Z corrected for the turn of axis B on 1 moves, up to +30.0000 mm (b-axis-radius-mm = 30)"
    );
    assert!(b_axis_notice(&r, &Options::default()).is_none());
    assert!(b_axis_notice(&Report::default(), &both(50.0, 30.0, 50.0)).is_none());
}

#[test]
fn c_turns_about_z_and_never_changes_the_height() {
    let opts = both(50.0, 30.0, 10.0);
    let plain = correct("G0 X0 Y0\nG1 X20 Y0 F100\n", &table(0.001, 0.0), &opts)
        .unwrap()
        .0;
    let turned = correct("G0 X0 Y0\nG1 X20 Y0 C720 F100\n", &table(0.001, 0.0), &opts).unwrap();
    // the same Z in every piece; C is interpolated and the last piece ends exactly on it
    let z_of = |t: &str| t.lines().filter_map(|l| word(l, 'Z')).collect::<Vec<_>>();
    assert_eq!(z_of(&plain), z_of(&turned.0));
    let c_of: Vec<f64> = turned.0.lines().filter_map(|l| word(l, 'C')).collect();
    assert_eq!(c_of, vec![360.0, 720.0]);
    assert_eq!((turned.1.a_corrected, turned.1.b_corrected), (0, 0));
}

#[test]
fn b_needs_a_radius_that_is_not_zero_and_not_beyond_a_thousand() {
    for radius in [0.0, 1000.5, -1000.5, f64::NAN, f64::INFINITY] {
        let opts = Options {
            b_compensation: AAxisCompensation::Tilt,
            b_radius_mm: radius,
            ..Options::default()
        };
        let e = correct("G0 X0 Y0\n", &table(0.0, 0.0), &opts).unwrap_err();
        assert!(
            e.contains("b-axis-radius-mm is") && e.contains("from -1000 to 1000"),
            "{radius}: {e}"
        );
    }
}

/// The simulated machine: it reads the levelled program and, at every line that writes X, Y and Z, says how deep the tip of the tool is below the surface of the table: the Z written
/// minus the lift that the turn of A and of B brings (`r * (1 - cos)`), minus the height of the table at that point. A, B and C stay where the last line put them.
fn depth_below_table(
    program: &str,
    map: &HeightMap,
    ra: f64,
    rb: f64,
    reference: (f64, f64),
) -> Vec<f64> {
    let zero = eh3d_autolevel::surface::height(
        map,
        reference.0,
        reference.1,
        eh3d_autolevel::surface::Method::Bilinear,
    )
    .unwrap();
    let (mut a, mut b) = (0.0, 0.0);
    let mut depths = Vec::new();
    for line in program.lines() {
        a = word(line, 'A').unwrap_or(a);
        b = word(line, 'B').unwrap_or(b);
        if let (Some(x), Some(y), Some(z)) = (word(line, 'X'), word(line, 'Y'), word(line, 'Z')) {
            let lift = a_axis_shift(ra, a) + a_axis_shift(rb, b);
            let surface = eh3d_autolevel::surface::height(
                map,
                x,
                y,
                eh3d_autolevel::surface::Method::Bilinear,
            )
            .unwrap()
                - zero;
            depths.push(z - lift - surface);
        }
    }
    depths
}

#[test]
fn on_a_simulated_machine_the_depth_below_the_table_is_the_one_the_program_asked_for() {
    // a table that rises along X and along Y, a cut 0.2 mm deep with A, B and C turning inside moves that are cut in pieces
    let map = table(0.0004, 0.0002);
    let program = "G0 X0 Y0\nG1 Z-0.2 F100\nG1 X40 Y10 A30 B-20 C90\nG1 X80 Y60 A-45 B60 C180\nG1 X10 Y90 A0 B0 C0\n";
    let (out, report) = correct(program, &map, &both(35.0, -22.0, 2.0)).unwrap();
    let depths = depth_below_table(&out, &map, 35.0, -22.0, (0.0, 0.0));
    assert!(depths.len() > 40, "{}", depths.len());
    // the first move to X0 Y0 has no Z; every written line with Z keeps the depth of the job (-0.2) to within the 3 decimals of the output
    for (i, d) in depths.iter().enumerate() {
        assert!((d - (-0.2)).abs() < 0.002, "line {i}: depth {d}");
    }
    assert!(report.a_corrected > 0 && report.b_corrected > 0);
    // without the compensation the same program would sink or lift the tool by the tilt: the simulator sees it
    let (plain, _) = correct(
        program,
        &map,
        &Options {
            max_segment_mm: 2.0,
            ..Options::default()
        },
    )
    .unwrap();
    let worst = depth_below_table(&plain, &map, 35.0, -22.0, (0.0, 0.0))
        .iter()
        .fold(0.0_f64, |m, d| m.max((d + 0.2).abs()));
    assert!(worst > 1.0, "{worst}");
}

#[test]
fn the_settings_file_has_the_compensation_and_the_radius_of_b() {
    let d = Settings::default();
    assert_eq!(
        (d.b_axis_compensation, d.b_axis_radius_mm),
        (AAxisCompensation::Off, 0.0)
    );
    assert_eq!(parse(&default_file()).unwrap(), d);
    assert!(
        default_file().contains("b-axis-compensation = off\n")
            && default_file().contains("b-axis-radius-mm = 0\n")
    );
    let s = parse("b-axis-compensation = TILT\nb-axis-radius-mm = -12.5\n").unwrap();
    assert_eq!(
        (s.b_axis_compensation, s.b_axis_radius_mm),
        (AAxisCompensation::Tilt, -12.5)
    );
    assert_eq!(
        parse_b_compensation(" Off ").unwrap(),
        AAxisCompensation::Off
    );
    assert_eq!(parse_b_radius(" 1000 ").unwrap(), 1000.0);
    for (text, line, word) in [
        (
            "b-axis-compensation = on\n",
            1,
            "b-axis-compensation 'on' must be off or tilt",
        ),
        (
            "b-axis-radius-mm = 1000.1\n",
            1,
            "b-axis-radius-mm '1000.1'",
        ),
        (
            "b-axis-compensation = off\nb-axis-compensation = tilt\n",
            2,
            "twice",
        ),
        ("b-axis-radius-mm = 1\nb-axis-radius-mm = 2\n", 2, "twice"),
        (
            "\nb-axis-compensation = tilt\n",
            2,
            "b-axis-radius-mm other than 0",
        ),
    ] {
        let e = parse(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
    let example = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/autolevel.cfg"
    ))
    .unwrap();
    assert_eq!(example.replace("\r\n", "\n"), default_file());
}

#[test]
fn the_command_line_applies_the_b_compensation_and_says_so() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("b_axis");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (map, prog) = (dir.join("map.txt"), dir.join("in.nc"));
    std::fs::write(&map, table(0.0, 0.0).to_text()).unwrap();
    std::fs::write(&prog, "G0 X0 Y0\nG1 X10 Y0 B90 F100\n").unwrap();
    let out = dir.join("out.nc");
    let run = |extra: &[&str]| {
        let mut args = vec![
            "apply",
            map.to_str().unwrap(),
            prog.to_str().unwrap(),
            out.to_str().unwrap(),
            "--segment",
            "50",
        ];
        args.extend_from_slice(extra);
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
            .args(&args)
            .output()
            .expect("the program starts");
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (ok, e) = run(&["--b-compensation", "tilt", "--b-radius", "40"]);
    assert!(ok, "{e}");
    assert!(e.contains("warning: Z corrected for the turn of axis B on 1 moves, up to +40.0000 mm (b-axis-radius-mm = 40)"), "{e}");
    assert!(std::fs::read_to_string(&out)
        .unwrap()
        .contains("Z40.000 B90"));
    let (ok, e) = run(&["--b-compensation", "tilt"]);
    assert!(!ok && e.contains("b-axis-radius-mm is"), "{e}");
    let (ok, e) = run(&["--b-radius", "1001"]);
    assert!(!ok && e.contains("--b-radius"), "{e}");
}
