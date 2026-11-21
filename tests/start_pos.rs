//! The start position of the program, taken from what a machine reports instead of being assumed to be 0 on every axis. The machine is the simulated GRBL of the controller crate of the suite
//! (and the text of saved reports of GRBL and Marlin): nothing here reads a real machine. A machine that is moving, in alarm or silent gives no place, a report that cannot be trusted is refused,
//! and what is read from damaged text never panics.

use eh3d_autolevel::gcode::{correct, Options, UnknownAxis};
use eh3d_autolevel::start::{from_controller, from_report, parse_place};
use eh3d_controller::grbl::GrblController;
use eh3d_controller::grblsim::{Flavor, GrblDevice};
use eh3d_controller::mock::MockTransport;
use eh3d_controller::wire::{Impair, WireLink};
use eh3d_controller::Controller;
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

/// A table 100 x 100 mm that rises 0.01 mm for every mm along X.
fn table() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, _) = m.grid.point(i);
        m.set(i, 0.01 * x).unwrap();
    }
    m
}

fn machine_at(place: &str, impair: Impair) -> GrblController<WireLink<GrblDevice>> {
    let mut c = GrblController::new(WireLink::new(GrblDevice::new(Flavor::Grbl), impair));
    c.send_line("G21 G90").unwrap();
    c.send_line(place).unwrap();
    c
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter))
        .and_then(|v| v.parse().ok())
}

#[test]
fn the_start_of_the_program_is_the_position_the_machine_reports() {
    let mut machine = machine_at("G0 X10 Y20 Z-0.9", Impair::clean());
    // the machine is still moving the first time it is asked: no place is taken then; once it is idle it gives the place of the work
    let moving = from_controller(&mut machine).unwrap_err();
    assert!(moving.contains("Run"), "{moving}");
    let place = from_controller(&mut machine).unwrap();
    assert_eq!(place, [10.0, 20.0, -0.9]);
    // the Z that the machine reports already carries the correction of the table (0.1 mm at X = 10): the program's own Z is -1.0, so the pieces of a move that
    // leaves out Y and Z start from there and keep Y = 20
    let map = table();
    let opts = Options {
        max_segment_mm: 10.0,
        start: Some(place),
        ..Options::default()
    };
    let (out, r) = correct("G1 X30 F100\n", &map, &opts).unwrap();
    let rows: Vec<&str> = out.lines().collect();
    assert_eq!(rows.len(), 2, "{out}");
    assert_eq!(rows[0], "G1 X20.000 Y20.000 Z-0.800 F100", "{out}");
    assert_eq!(rows[1], "G1 X30.000 Y20.000 Z-0.700");
    assert_eq!(r.moves, 1);
    // without the place the same line is levelled from 0, 0, 0 (the way it always was) and with unknown-axis = refuse it is refused; with the place it is not
    let (zero, _) = correct(
        "G1 X30 F100\n",
        &map,
        &Options {
            max_segment_mm: 10.0,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(zero.starts_with("G1 X10.000 Y0.000 Z0.100"), "{zero}");
    let strict = Options {
        unknown_axis: UnknownAxis::Refuse,
        ..Options::default()
    };
    assert!(correct("G1 X30 F100\n", &map, &strict)
        .unwrap_err()
        .contains("unknown-axis"));
    assert!(correct(
        "G1 X30 F100\n",
        &map,
        &Options {
            start: Some(place),
            ..strict
        }
    )
    .is_ok());
    // a place that is not a number, or is out of the limits, is refused before anything is levelled
    for bad in [
        [f64::NAN, 0.0, 0.0],
        [0.0, f64::INFINITY, 0.0],
        [2.0e6, 0.0, 0.0],
    ] {
        let e = correct(
            "G1 X1\n",
            &map,
            &Options {
                start: Some(bad),
                ..Options::default()
            },
        )
        .unwrap_err();
        assert!(e.contains("start position"), "{e}");
    }
}

#[test]
fn a_machine_in_alarm_or_silent_gives_no_place() {
    let mut locked = GrblController::new(WireLink::new(
        GrblDevice::new(Flavor::GrblHal).locked(),
        Impair::clean(),
    ));
    let e = from_controller(&mut locked).unwrap_err();
    assert!(e.contains("Alarm"), "{e}");
    // nothing answers
    let mut silent = GrblController::new(MockTransport::with_replies(&[]));
    let e = from_controller(&mut silent).unwrap_err();
    assert!(e.contains("did not give its place"), "{e}");
    // an answer that is not a report
    let mut garbled = GrblController::new(MockTransport::with_replies(&["ok", "[MSG:hello]"]));
    assert!(from_controller(&mut garbled).is_err());
}

#[test]
fn the_place_is_never_wrong_on_a_cable_that_cuts_and_adds_noise() {
    let mut given = 0;
    for seed in 1..=120_u64 {
        let impair = Impair {
            seed,
            chunk_max: 1 + (seed % 4) as usize,
            noise_per_mille: 300,
            ..Impair::clean()
        };
        let mut machine = machine_at("G0 X10 Y20 Z-0.9", impair);
        // asked until it is idle; a place that comes is the right one
        let mut answer = Err(String::new());
        for _ in 0..4 {
            answer = from_controller(&mut machine);
            if answer.is_ok() {
                break;
            }
        }
        if let Ok(place) = answer {
            given += 1;
            assert_eq!(place, [10.0, 20.0, -0.9], "seed {seed}");
        }
    }
    assert!(given > 60, "only {given} of 120 cables gave a place");
}

#[test]
fn a_saved_report_gives_the_place_and_one_that_cannot_be_trusted_is_refused() {
    // GRBL: the position of the work, or the position of the machine with the work offset
    assert_eq!(
        from_report("<Idle|WPos:10.000,20.000,-0.900|FS:0,0>").unwrap(),
        [10.0, 20.0, -0.9]
    );
    assert_eq!(
        from_report("<Idle|MPos:12.000,23.000,-0.400|FS:0,0|WCO:2.000,3.000,0.500>").unwrap(),
        [10.0, 20.0, -0.9]
    );
    // the banner and the lines that are not the report are passed over; the first report is the one that counts
    assert_eq!(
        from_report(
            "Grbl 1.1h ['$' for help]\r\n<Hold:0|WPos:1.000,2.000,3.000>\r\n<Idle|WPos:9,9,9>\r\n"
        )
        .unwrap(),
        [1.0, 2.0, 3.0]
    );
    // Marlin: the answer to M114 (the three axes before Count)
    assert_eq!(
        from_report("X:10.00 Y:20.00 Z:-0.90 E:0.00 Count X:800 Y:1600 Z:-72\nok").unwrap(),
        [10.0, 20.0, -0.9]
    );
    // the position of the machine without the offset says nothing about the work; a machine that moves or is in alarm gives no place; neither does text with no report
    let e = from_report("<Idle|MPos:12.000,23.000,-0.400|FS:0,0>").unwrap_err();
    assert!(e.contains("work offset"), "{e}");
    assert!(from_report("<Run|WPos:1,2,3>").unwrap_err().contains("Run"));
    assert!(from_report("<Alarm|WPos:1,2,3>")
        .unwrap_err()
        .contains("Alarm"));
    assert!(from_report("").unwrap_err().contains("no status report"));
    assert!(from_report("hello\nworld").is_err());
    // a number that is not a number, or is out of the limits
    assert!(from_report("<Idle|WPos:1e9,2,3>").is_err());
    assert!(from_report("<Idle|WPos:nan,2,3>").is_err());
    assert!(from_report("X:1.00 Y:inf Z:3.00 E:0.00 Count X:1").is_err());
    // the place on the command line
    assert_eq!(parse_place("10, 20 ,-0.9").unwrap(), [10.0, 20.0, -0.9]);
    for bad in ["", "1,2", "1,2,3,4", "a,b,c", "1,2,nan", "1,2,1e9"] {
        assert!(parse_place(bad).is_err(), "{bad}");
    }
}

#[test]
fn damaged_reports_never_panic_and_give_only_places_within_the_limits() {
    let valid = [
        "<Idle|MPos:12.000,23.000,-0.400|FS:0,0|WCO:2.000,3.000,0.500>\n",
        "<Idle|WPos:10.000,20.000,-0.900|FS:0,0>\n",
        "X:10.00 Y:20.00 Z:-0.90 E:0.00 Count X:800 Y:1600 Z:-72\nok\n",
    ];
    let mut seen_ok = 0;
    for text in valid {
        // cut at every byte
        for end in 0..=text.len() {
            if let Ok(p) = from_report(&text[..end]) {
                seen_ok += 1;
                assert!(p.iter().all(|v| v.is_finite() && v.abs() <= 1.0e6), "{p:?}");
            }
        }
    }
    assert!(seen_ok > 3);
    // random text made of the pieces of a report
    let pieces = [
        "<", ">", "|", ":", ",", "Idle", "Run", "WPos", "MPos", "WCO", "X", "Y", "Z", "Count", "-",
        ".", "1", "22", "e9", "nan", "inf", " ", "\n", "ok",
    ];
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    for _ in 0..4000 {
        let mut text = String::new();
        for _ in 0..1 + state % 24 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            text.push_str(pieces[(state % pieces.len() as u64) as usize]);
        }
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        if let Ok(p) = from_report(&text) {
            assert!(
                p.iter().all(|v| v.is_finite() && v.abs() <= 1.0e6),
                "{text:?} gave {p:?}"
            );
        }
    }
}

#[test]
fn the_command_line_takes_the_start_from_a_place_or_from_a_saved_report() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = |name: &str| format!("{dir}/start-{name}");
    let run = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
            .args(args)
            .output()
            .unwrap();
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (map, prog, out) = (path("map.txt"), path("in.nc"), path("out.nc"));
    assert!(run(&["demo-map", &map, "0", "0", "100", "100", "5", "5"]).0);
    std::fs::write(&prog, "G90 G21\nG1 X30 F100\n").unwrap();
    let first_move = |file: &str| {
        std::fs::read_to_string(file)
            .unwrap()
            .lines()
            .find(|l| l.starts_with("G1"))
            .unwrap()
            .to_string()
    };
    // the place given by hand: Y stays where the machine stands
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--start", "10,20,-1"]);
    assert!(ok, "{e}");
    let with_place = first_move(&out);
    assert_eq!(word(&with_place, 'Y'), Some(20.0), "{with_place}");
    // the place read from a saved report gives the same program
    let report = path("report.txt");
    std::fs::write(&report, "<Idle|WPos:10.000,20.000,-1.000|FS:0,0>\n").unwrap();
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--start-report", &report]);
    assert!(ok, "{e}");
    assert_eq!(first_move(&out), with_place);
    // without it the move starts from Y = 0
    assert!(run(&["apply", &map, &prog, &out]).0);
    assert_eq!(word(&first_move(&out), 'Y'), Some(0.0));
    // both together, a place that is not X,Y,Z, a missing or useless report: errors that name the option
    let (ok, _, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--start",
        "1,2,3",
        "--start-report",
        &report,
    ]);
    assert!(!ok && e.contains("cannot be given together"), "{e}");
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--start", "1,2"]);
    assert!(!ok && e.contains("--start"), "{e}");
    let (ok, _, e) = run(&[
        "apply",
        &map,
        &prog,
        &out,
        "--start-report",
        &path("none.txt"),
    ]);
    assert!(!ok && e.contains("none.txt"), "{e}");
    std::fs::write(&report, "<Idle|MPos:1.000,2.000,3.000|FS:0,0>\n").unwrap();
    let (ok, _, e) = run(&["apply", &map, &prog, &out, "--start-report", &report]);
    assert!(!ok && e.contains("work offset"), "{e}");
}
