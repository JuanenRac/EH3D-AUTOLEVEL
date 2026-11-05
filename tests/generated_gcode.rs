//! The standard header of the levelled program, and the program read back with the reader of the suite and walked the way the G-code debugger walks it in its strict mode: nothing in
//! it may be a warning of the reader, a rapid move into the material or a cut with the spindle off.

use eh3d_autolevel::gcode::{correct, levelled_line, mark_with_header, Options};
use eh3d_levelling::{Grid, HeightMap};
use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};
use std::process::Command;

fn plane(a: f64, b: f64) -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, a * x + b * y).unwrap();
    }
    m
}

/// What the strict mode of the debugger would report for `text`.
fn alerts(text: &str) -> Vec<String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let mut found: Vec<String> = trace
        .diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .map(|d| format!("line {}: {}", d.line, d.message))
        .collect();
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        let lowest = s.from.z.min(s.to.z);
        if rapid && lowest < 0.0 && (s.from.x != s.to.x || s.from.y != s.to.y || s.to.z < s.from.z)
        {
            found.push(format!(
                "line {}: a rapid move goes into the material",
                s.line
            ));
        }
        if !rapid && !s.spindle_on && lowest < 0.0 {
            found.push(format!("line {}: it cuts with the spindle off", s.line));
        }
        if !rapid && s.feed <= 0.0 {
            found.push(format!("line {}: a cutting move has no feed", s.line));
        }
    }
    if !trace.ended {
        found.push("the program does not end".into());
    }
    found
}

const PROGRAM: &str =
    "G90 G21\nM3 S10000\nG0 Z2\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X50 Y0\nG1 X50 Y30\nG0 Z2\nM5\nM2\n";

#[test]
fn the_header_comes_before_the_mark_names_the_program_and_its_hash_and_follows_the_percent_sign() {
    let map = plane(0.0, 0.0);
    let opts = Options::default();
    let head = format!(
        "; generator: eh3d-autolevel {}\n; units: mm (G21)\n; source: job.nc sha256:da2477a3c847cd3b0b5aa488874b00bd5e960e56eb059a5fb483fb70d2dc57e0\n; tools: as in the original program\n",
        env!("CARGO_PKG_VERSION")
    );
    let plain = mark_with_header("M2\n", &map, &opts, Some(("job.nc", "M2\n")));
    assert!(plain.starts_with(&head), "{plain}");
    assert!(
        plain[head.len()..].starts_with("(eh3d-autolevel "),
        "{plain}"
    );
    assert!(plain.ends_with("M2\n"), "{plain}");
    assert_eq!(levelled_line(&plain), Some(5));
    let with_percent = mark_with_header("%\nM2\n%\n", &map, &opts, Some(("job.nc", "M2\n")));
    assert!(
        with_percent.starts_with(&format!("%\n{head}(eh3d-autolevel ")),
        "{with_percent}"
    );
    assert!(with_percent.ends_with("M2\n%\n"));
    // a program that does not come from a file says so
    let none = mark_with_header("M2\n", &map, &opts, None);
    assert!(none.contains("; source: none\n"), "{none}");
    // the result cannot be levelled again, header or not
    let e = correct(&plain, &map, &opts).unwrap_err();
    assert!(e.contains("already levelled"), "{e}");
}

#[test]
fn the_checker_used_here_does_find_a_cut_with_the_spindle_off() {
    assert!(!alerts("G21\nG1 Z-1 F100\nM2\n").is_empty());
    assert!(alerts("G21\nM3 S1000\nG1 Z-1 F100\nG0 Z2\nM5\nM2\n").is_empty());
}

#[test]
fn the_levelled_program_has_no_alert_and_the_header_does_not_change_what_it_cuts() {
    let map = plane(0.001, 0.0005);
    let opts = Options::default();
    let (body, _) = correct(PROGRAM, &map, &opts).unwrap();
    let all = mark_with_header(&body, &map, &opts, Some(("job.nc", PROGRAM)));
    let found = alerts(&all);
    assert!(found.is_empty(), "{found:?}\n{all}");
    // the same cuts with and without the header
    let with = interpret(&parse(&all, Dialect::Grbl));
    let without = interpret(&parse(&body, Dialect::Grbl));
    assert_eq!(with.segments.len(), without.segments.len());
    assert!(with.segments.len() > 10, "{} moves", with.segments.len());
    for (a, b) in with.segments.iter().zip(&without.segments) {
        assert_eq!((a.from, a.to), (b.from, b.to));
    }
}

#[test]
fn the_command_line_writes_the_header_first_and_its_output_has_no_alert() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let (map, prog, out) = (
        format!("{dir}/gen-map.txt"),
        format!("{dir}/gen-in.nc"),
        format!("{dir}/gen-out.nc"),
    );
    let run = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
            .args(args)
            .output()
            .expect("the program starts");
        (
            o.status.success(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (ok, e) = run(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    assert!(ok, "{e}");
    std::fs::write(&prog, PROGRAM).unwrap();
    let (ok, e) = run(&["apply", &map, &prog, &out]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(
        text.starts_with(&format!(
            "; generator: eh3d-autolevel {}\n; units: mm (G21)\n; source: gen-in.nc sha256:",
            env!("CARGO_PKG_VERSION")
        )),
        "{text}"
    );
    let found = alerts(&text);
    assert!(found.is_empty(), "{found:?}\n{text}");
}
