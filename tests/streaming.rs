//! The correction made while the program is sent: an emitter takes the original program line by line, gives the corrected lines as the file correction would, keeps the depth limits and
//! stops in a safe state (the spindle off, the tool lifted, nothing more sent) when the map does not cover the place, when a Z would go too deep or the correction is too large, or when a
//! line cannot be levelled. A simulated machine is what the lines are tried on; nothing is sent to a real machine and the sending program of the suite does not call it yet.

use eh3d_autolevel::config::{default_file, parse};
use eh3d_autolevel::gcode::{correct, Options};
use eh3d_autolevel::stream::{send_program, HaltReason, SimMachine, StreamLimits, Streamer};
use eh3d_autolevel::surface::Method;
use eh3d_levelling::{Grid, HeightMap};
use std::process::Command;

/// A table 100 x 100 mm that rises `sx` mm for every mm along X and `sy` along Y.
fn table(sx: f64, sy: f64) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, sx * x + sy * y).unwrap();
    }
    m
}

const PROGRAM: &str = "G90 G21\nG0 Z5\nG0 X0 Y0\nM3 S1000\nG1 Z-0.5 F100\nG1 X60 Y0 F300\nG1 X60 Y40\nG0 Z5\nM5\nM2\n";

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() % 1_000_001) as f64 / 1_000_000.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter))
        .and_then(|v| v.parse().ok())
}

#[test]
fn the_program_is_levelled_while_it_is_sent() {
    let map = table(0.002, 0.001);
    let opts = Options {
        max_segment_mm: 5.0,
        ..Options::default()
    };
    // the lines come out one by one, and together they are what the correction of the whole file writes
    let mut emitter = Streamer::new(&map, &opts, StreamLimits::default()).unwrap();
    let mut streamed: Vec<String> = Vec::new();
    for (i, raw) in PROGRAM.lines().enumerate() {
        let lines = emitter.line(raw).unwrap();
        // a line of the program gives its own lines at once: nothing waits for the end of the file
        assert!(!lines.is_empty(), "line {}", i + 1);
        assert_eq!(emitter.lines_sent(), streamed.len() + lines.len());
        streamed.extend(lines);
    }
    let (whole, report) = correct(PROGRAM, &map, &opts).unwrap();
    assert_eq!(streamed.join("\n") + "\n", whole);
    assert_eq!(emitter.report(), report);
    assert!(emitter.halted().is_none());
    // a machine that is only a model: the tool went as deep below the table as the program asked, nothing hit the table in a rapid move, and the spindle is off at the end
    let mut machine = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    let outcome =
        send_program(PROGRAM, &map, &opts, StreamLimits::default(), &mut machine).unwrap();
    assert_eq!(outcome.sent, streamed);
    assert!(outcome.halt.is_none() && outcome.safe.is_empty());
    assert!(
        (machine.deepest_mm + 0.5).abs() < 2e-3,
        "{}",
        machine.deepest_mm
    );
    assert_eq!(machine.rapid_into_table, 0);
    assert!(!machine.spindle_on);
    // the table is not where the map says it is: the model of the machine shows how far the cut is from the depth asked
    let other = table(0.002, 0.003);
    let mut off = SimMachine::new(&other, (0.0, 0.0), Method::Bilinear);
    send_program(PROGRAM, &map, &opts, StreamLimits::default(), &mut off).unwrap();
    assert!(off.deepest_mm < -0.5 - 0.03, "{}", off.deepest_mm);
}

#[test]
fn a_move_outside_the_map_stops_the_stream_in_a_safe_state() {
    let map = table(0.002, 0.001);
    let program =
        "G0 Z5\nG0 X10 Y10\nM3 S1000\nG1 Z-0.5 F100\nG1 X50 Y10\nG1 X500 Y10\nG1 X10 Y10\nM5\n";
    let opts = Options::default();
    let mut machine = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    let outcome =
        send_program(program, &map, &opts, StreamLimits::default(), &mut machine).unwrap();
    let halt = outcome.halt.clone().expect("it stops");
    assert_eq!(halt.line, 6);
    assert!(
        matches!(halt.reason, HaltReason::OutsideMap { distance_mm } if (distance_mm - 400.0).abs() < 1e-6),
        "{halt:?}"
    );
    assert_eq!(halt.reason.code(), "outside-map");
    assert!(halt.to_string().starts_with("line 6:"), "{halt}");
    // nothing of the line that broke the rule, nor of the lines after it, was sent
    assert!(
        outcome.sent.iter().all(|l| !l.contains("X500")),
        "{:?}",
        outcome.sent
    );
    assert!(
        outcome.sent.last().unwrap().contains("X50.000"),
        "{:?}",
        outcome.sent
    );
    let before: Vec<&str> = program.lines().take(5).collect();
    let (first_five, _) = correct(
        &(before.join(
            "
",
        ) + "
"),
        &map,
        &opts,
    )
    .unwrap();
    assert_eq!(
        outcome.sent.join(
            "
"
        ) + "
",
        first_five
    );
    // the safe state: the spindle off and the tool lifted above the table, and the stream stays stopped
    assert_eq!(outcome.safe[0], "M5");
    assert!(outcome.safe[1].starts_with("G0 Z"), "{:?}", outcome.safe);
    assert!(!machine.spindle_on);
    assert!(machine.depth_mm() > 4.9, "{}", machine.depth_mm());
    let mut emitter = Streamer::new(&map, &opts, StreamLimits::default()).unwrap();
    for raw in program.lines().take(5) {
        emitter.line(raw).unwrap();
    }
    let sent = emitter.lines_sent();
    let first = emitter.line("G1 X500 Y10").unwrap_err();
    assert_eq!(emitter.line("G1 X20 Y10").unwrap_err(), first);
    assert_eq!(emitter.lines_sent(), sent);
    assert_eq!(emitter.halted(), Some(&first));
    // a margin that is a setting lets a small step outside through, and a bigger one still stops it
    let wide = StreamLimits {
        outside_mm: 450.0,
        ..StreamLimits::default()
    };
    let mut m = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    let ok = send_program(program, &map, &opts, wide, &mut m).unwrap();
    assert!(ok.halt.is_none(), "{:?}", ok.halt);
    assert!(ok.report.outside_points > 0 && ok.sent.iter().any(|l| l.contains("X500")));
    let tight = StreamLimits {
        outside_mm: 300.0,
        ..StreamLimits::default()
    };
    let mut m = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    assert!(send_program(program, &map, &opts, tight, &mut m)
        .unwrap()
        .halt
        .is_some());
}

#[test]
fn the_limits_of_depth_and_of_correction_stop_the_stream() {
    let map = table(0.0, 0.0);
    let program = "G0 Z5\nG0 X10 Y10\nG1 Z-0.4 F100\nG1 Z-1\nG1 X20\n";
    let deep = StreamLimits {
        min_z_mm: -0.5,
        ..StreamLimits::default()
    };
    let mut m = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    let o = send_program(program, &map, &Options::default(), deep, &mut m).unwrap();
    let halt = o.halt.unwrap();
    assert_eq!(halt.line, 4);
    assert!(
        matches!(halt.reason, HaltReason::TooDeep { z_mm } if (z_mm + 1.0).abs() < 1e-9),
        "{halt:?}"
    );
    assert!(
        o.sent.iter().all(|l| !l.contains("Z-1.000")),
        "{:?}",
        o.sent
    );
    assert!(m.deepest_mm >= -0.4 - 1e-9);
    // the default floor is far lower, and the line goes through
    let mut m = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
    assert!(send_program(
        program,
        &map,
        &Options::default(),
        StreamLimits::default(),
        &mut m
    )
    .unwrap()
    .halt
    .is_none());
    // a map that rises 0.1 mm for every mm: the correction passes 2 mm at X = 20
    let steep = table(0.1, 0.0);
    let program = "G0 X0 Y0\nG1 X10\nG1 X30\nG1 X50\n";
    let mut m = SimMachine::new(&steep, (0.0, 0.0), Method::Bilinear);
    let o = send_program(
        program,
        &steep,
        &Options::default(),
        StreamLimits::default(),
        &mut m,
    )
    .unwrap();
    let halt = o.halt.unwrap();
    assert_eq!(halt.line, 3);
    assert!(
        matches!(halt.reason, HaltReason::ShiftTooLarge { shift_mm } if (shift_mm - 3.0).abs() < 1e-6),
        "{halt:?}"
    );
    // 0 is no limit at all
    let free = StreamLimits {
        max_shift_mm: 0.0,
        ..StreamLimits::default()
    };
    let mut m = SimMachine::new(&steep, (0.0, 0.0), Method::Bilinear);
    assert!(
        send_program(program, &steep, &Options::default(), free, &mut m)
            .unwrap()
            .halt
            .is_none()
    );
}

#[test]
fn a_line_that_cannot_be_levelled_is_never_sent() {
    let map = table(0.0, 0.0);
    let opts = Options::default();
    let mut emitter = Streamer::new(&map, &opts, StreamLimits::default()).unwrap();
    emitter.line("G0 X10 Y10").unwrap();
    // an arc while arcs are refused, a code that moves the origin, a program that was levelled already
    let halt = emitter.line("G2 X20 Y10 I5").unwrap_err();
    assert_eq!(halt.line, 2);
    assert!(
        matches!(&halt.reason, HaltReason::Refused(why) if why.contains("arcs")),
        "{halt:?}"
    );
    let mut emitter = Streamer::new(&map, &opts, StreamLimits::default()).unwrap();
    let halt = emitter.line("G92 X0").unwrap_err();
    assert!(matches!(halt.reason, HaltReason::Refused(_)));
    let mut emitter = Streamer::new(&map, &opts, StreamLimits::default()).unwrap();
    let halt = emitter
        .line("(eh3d-autolevel 0.1.1 map=00000000)")
        .unwrap_err();
    assert_eq!(halt.reason, HaltReason::AlreadyLevelled);
    assert_eq!(halt.reason.code(), "already-levelled");
    // limits that make no sense are refused before any line is taken
    for bad in [
        StreamLimits {
            min_z_mm: f64::NAN,
            ..StreamLimits::default()
        },
        StreamLimits {
            max_shift_mm: -1.0,
            ..StreamLimits::default()
        },
        StreamLimits {
            outside_mm: f64::INFINITY,
            ..StreamLimits::default()
        },
        StreamLimits {
            safe_z_mm: 2000.0,
            ..StreamLimits::default()
        },
    ] {
        assert!(Streamer::new(&map, &opts, bad).is_err(), "{bad:?}");
    }
    // arcs that the settings allow are levelled while they are sent
    let cut = Options {
        arcs: eh3d_autolevel::gcode::ArcMode::Cut,
        ..Options::default()
    };
    let mut emitter = Streamer::new(&map, &cut, StreamLimits::default()).unwrap();
    emitter.line("G0 X10 Y0").unwrap();
    assert!(emitter.line("G3 X0 Y10 I-10 J0").unwrap().len() > 4);
}

#[test]
fn random_programs_are_sent_or_stopped_without_a_panic() {
    const WORDS: [&str; 22] = [
        "G0", "G1", "G2", "G90", "G91", "X10", "X-5", "X130", "Y20.5", "Y0", "Y95", "Z-0.5",
        "Z-12", "Z5", "F100", "M3", "M5", "S500", "I-5", "J0", "(note)", "A30",
    ];
    let map = table(0.002, 0.001);
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let (mut stopped, mut finished) = (0, 0);
    for round in 0..500 {
        let mut text = String::from("G0 X10 Y10 Z5\n");
        for _ in 0..1 + rng.below(8) {
            for k in 0..1 + rng.below(5) {
                if k > 0 {
                    text.push(' ');
                }
                text.push_str(WORDS[rng.below(WORDS.len())]);
            }
            text.push('\n');
        }
        let limits = StreamLimits {
            min_z_mm: -(rng.unit() * 20.0),
            max_shift_mm: if rng.below(2) == 0 {
                0.0
            } else {
                0.01 + rng.unit()
            },
            outside_mm: rng.unit() * 40.0,
            safe_z_mm: rng.unit() * 20.0,
        };
        let opts = Options {
            max_segment_mm: 0.5 + rng.unit() * 20.0,
            ..Options::default()
        };
        let mut machine = SimMachine::new(&map, (0.0, 0.0), Method::Bilinear);
        let outcome = send_program(&text, &map, &opts, limits, &mut machine).unwrap();
        for l in &outcome.sent {
            assert!(
                !l.contains("NaN") && !l.contains("inf"),
                "round {round}: {l}"
            );
            // nothing that was sent goes deeper than the floor or farther outside than the margin (three decimals of rounding)
            if let Some(z) = word(l, 'Z') {
                assert!(
                    z >= limits.min_z_mm - 1e-3,
                    "round {round}: {l} with {limits:?}"
                );
            }
            // (a rapid move with no Z is not levelled, so the map has no say in it)
            for (letter, hi) in [('X', 100.0), ('Y', 100.0)]
                .into_iter()
                .filter(|_| l.contains("G1"))
            {
                if let Some(v) = word(l, letter) {
                    assert!(
                        v >= -limits.outside_mm - 1e-3 && v <= hi + limits.outside_mm + 1e-3,
                        "round {round}: {l} with {limits:?}"
                    );
                }
            }
        }
        match outcome.halt {
            None => {
                finished += 1;
                // not stopped: the same lines the file correction writes
                let (whole, _) = correct(&text, &map, &opts).unwrap();
                assert_eq!(outcome.sent.join("\n") + "\n", whole, "round {round}");
                assert!(outcome.safe.is_empty());
            }
            Some(_) => {
                stopped += 1;
                assert_eq!(outcome.safe.len(), 2);
                assert_eq!(outcome.safe[0], "M5");
                assert!(!machine.spindle_on);
            }
        }
    }
    assert!(
        stopped > 50 && finished > 50,
        "{stopped} stopped, {finished} finished"
    );
}

#[test]
fn the_settings_hold_the_limits_and_the_command_stops_with_the_code_2() {
    // the file: the defaults are the conservative ones, each key has its range, and a bad line says its number
    let s = parse("").unwrap();
    assert_eq!(s.stream, StreamLimits::default());
    assert_eq!(StreamLimits::default().outside_mm, 0.0);
    let s = parse("stream-min-z-mm = -3\nstream-max-shift-mm = 0\nstream-outside-mm = 5\nstream-safe-z-mm = 12.5\n").unwrap();
    assert_eq!(
        s.stream,
        StreamLimits {
            min_z_mm: -3.0,
            max_shift_mm: 0.0,
            outside_mm: 5.0,
            safe_z_mm: 12.5
        }
    );
    for bad in [
        "stream-min-z-mm = 1\n",
        "stream-min-z-mm = -2000\n",
        "stream-max-shift-mm = 0.0001\n",
        "stream-max-shift-mm = 101\n",
        "stream-outside-mm = -1\n",
        "stream-safe-z-mm = 1001\n",
        "stream-safe-z-mm = nan\n",
    ] {
        assert!(
            parse(&format!("decimals = 3\n{bad}"))
                .unwrap_err()
                .starts_with("line 2:"),
            "{bad}"
        );
    }
    assert!(parse("stream-outside-mm = 1\nstream-outside-mm = 2\n")
        .unwrap_err()
        .contains("given twice"));
    let defaults = default_file();
    assert!(
        defaults.contains("stream-min-z-mm = -10") && defaults.contains("stream-safe-z-mm = 5")
    );
    // the command
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = |name: &str| format!("{dir}/stream-{name}");
    let run = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
            .args(args)
            .output()
            .unwrap();
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (map, good, bad, sent) = (
        path("map.txt"),
        path("good.nc"),
        path("bad.nc"),
        path("sent.nc"),
    );
    assert_eq!(
        run(&["demo-map", &map, "0", "0", "100", "100", "5", "5"]).0,
        Some(0)
    );
    std::fs::write(&good, PROGRAM).unwrap();
    std::fs::write(
        &bad,
        "G0 Z5\nG0 X10 Y10\nM3\nG1 Z-0.5 F100\nG1 X150 Y10\nM5\n",
    )
    .unwrap();
    let (code, out, e) = run(&["stream", &map, &good, "--out", &sent]);
    assert_eq!(code, Some(0), "{e}");
    assert!(
        out.contains("lines sent") && out.contains("deepest cut"),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(&sent).unwrap().lines().count(),
        out.split_whitespace()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap()
    );
    // the stream stops: exit code 2, the reason on the error output, and the safe lines at the end of the file that was kept
    let (code, out, e) = run(&["stream", &map, &bad, "--out", &sent]);
    assert_eq!(code, Some(2), "{out} {e}");
    assert!(
        e.contains("stopped: line 5:") && e.contains("outside the measured area"),
        "{e}"
    );
    let kept = std::fs::read_to_string(&sent).unwrap();
    let lines: Vec<&str> = kept.lines().collect();
    assert_eq!(lines[lines.len() - 2], "M5");
    assert!(
        lines[lines.len() - 1].starts_with("G0 Z") && !kept.contains("X150"),
        "{kept}"
    );
    let (code, out, _) = run(&["stream", &map, &bad, "--json"]);
    assert_eq!(code, Some(2));
    assert!(
        out.contains("\"stopped\":true")
            && out.contains("\"stop_line\":5")
            && out.contains("\"stop_reason\":\"outside-map\""),
        "{out}"
    );
    // the margin of the settings file lets the same program through
    let cfg = path("wide.cfg");
    std::fs::write(&cfg, "stream-outside-mm = 60\n").unwrap();
    assert_eq!(run(&["stream", &map, &bad, "--config", &cfg]).0, Some(0));
    // wrong use is an error (exit code 1), not a stop
    assert_eq!(run(&["stream", &map]).0, Some(1));
    assert_eq!(run(&["stream", &map, &good, "--nope"]).0, Some(1));
    assert_eq!(run(&["stream", &map, "no-such-file.nc"]).0, Some(1));
}

#[test]
fn the_stream_never_writes_over_what_it_reads_and_stops_at_a_jog() {
    let path = |name: &str| format!("{}/stream-over-{name}", env!("CARGO_TARGET_TMPDIR"));
    let (map, program, sent) = (path("map.txt"), path("job.nc"), path("sent.nc"));
    assert_eq!(
        run(&["demo-map", &map, "0", "0", "100", "100", "5", "5"]).0,
        Some(0)
    );
    std::fs::write(&program, PROGRAM).unwrap();
    let table = std::fs::read_to_string(&map).unwrap();
    for (out, what) in [(&program, "also the program"), (&map, "also the map")] {
        let (code, _, e) = run(&["stream", &map, &program, "--out", out]);
        assert_eq!(code, Some(1), "{e}");
        assert!(e.contains(what), "{e}");
    }
    assert_eq!(std::fs::read_to_string(&program).unwrap(), PROGRAM);
    assert_eq!(std::fs::read_to_string(&map).unwrap(), table);
    std::fs::write(&program, "G0 Z5\n$J=G91 X10 F500\n").unwrap();
    let (code, _, e) = run(&["stream", &map, &program, "--out", &sent]);
    assert_eq!(code, Some(2), "{e}");
    assert!(e.contains("stopped: line 2:") && e.contains("$J="), "{e}");
    assert!(!std::fs::read_to_string(&sent).unwrap().contains('$'));
}

/// The two lines a sender is given after a stop, for a program that stops, on `map` with the options `opts`.
fn stopped_safe_lines(map: &HeightMap, opts: &Options, program: &str) -> Vec<String> {
    let mut machine = SimMachine::new(map, (0.0, 0.0), Method::Bilinear);
    let outcome = send_program(program, map, opts, StreamLimits::default(), &mut machine).unwrap();
    assert!(outcome.halt.is_some(), "{:?}", outcome.sent);
    outcome.safe
}

#[test]
fn the_lift_after_a_stop_never_takes_the_tool_below_where_it_stands() {
    // the tool stands at Z50; the refused line would have gone down to -1, and nothing of it was given out
    let map = table(0.0, 0.0);
    let safe = stopped_safe_lines(
        &map,
        &Options::default(),
        "G0 Z50\nG0 X10 Y10\nG1 X500 Y10 Z-1 F100\n",
    );
    assert_eq!(safe, ["M5", "G0 Z50.000"]);
}

#[test]
fn the_lift_after_a_stop_does_not_use_the_correction_of_the_line_that_was_refused() {
    // one point of the map is 20 mm below the rest, as a probe that slipped would leave it
    let mut map = table(0.0, 0.0);
    let hole = (0..map.grid.len())
        .find(|&i| map.grid.point(i) == (50.0, 50.0))
        .unwrap();
    map.set(hole, -20.0).unwrap();
    let safe = stopped_safe_lines(
        &map,
        &Options::default(),
        "G0 Z5\nG0 X0 Y0\nG1 Z-0.5 F100\nG1 X50 Y50\n",
    );
    assert_eq!(safe, ["M5", "G0 Z5.000"]);
    // and a point 20 mm above the rest does not send the tool 25 mm up
    map.set(hole, 20.0).unwrap();
    let safe = stopped_safe_lines(
        &map,
        &Options::default(),
        "G0 Z5\nG0 X0 Y0\nG1 Z-0.5 F100\nG1 X50 Y50\n",
    );
    assert_eq!(safe, ["M5", "G0 Z5.000"]);
}

#[test]
fn the_lift_after_a_stop_on_the_first_line_stays_at_the_start_that_was_given() {
    let map = table(0.002, 0.001);
    let opts = Options {
        start: Some([10.0, 10.0, 50.0]),
        ..Options::default()
    };
    let safe = stopped_safe_lines(&map, &opts, "G1 X500 Y10 F100\n");
    assert_eq!(safe, ["M5", "G0 Z50.000"]);
    // a line that writes no Z does not move the place the tool is taken to be at
    let safe = stopped_safe_lines(&map, &opts, "M3 S1000\nG4 P1\nG1 X500 Y10 F100\n");
    assert_eq!(safe, ["M5", "G0 Z50.000"]);
}
