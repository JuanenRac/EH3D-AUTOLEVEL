//! `apply --dry-run` works out and counts what it would do, and writes nothing.

use std::path::Path;
use std::process::Command;

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

fn files() -> (String, String, String) {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let map = format!("{dir}/dry-map.txt");
    let prog = format!("{dir}/dry-in.nc");
    let (ok, _, e) = run(&["demo-map", &map, "0", "0", "100", "100", "3", "3"]);
    assert!(ok, "{e}");
    std::fs::write(&prog, "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X50 Y0\nM2\n").unwrap();
    (map, prog, format!("{dir}/dry-out.nc"))
}

#[test]
fn a_dry_run_counts_the_same_and_writes_nothing() {
    let (map, prog, out) = files();
    let _ = std::fs::remove_file(&out);
    let (ok, real, e) = run(&["apply", &map, &prog, &out]);
    assert!(ok, "{e}");
    assert!(Path::new(&out).exists());
    std::fs::remove_file(&out).unwrap();
    let (ok, dry, e) = run(&["apply", &map, &prog, &out, "--dry-run"]);
    assert!(ok, "{e}");
    assert!(!Path::new(&out).exists(), "a dry run writes nothing");
    assert!(dry.contains("dry run"), "{dry}");
    // the same count as the real run
    assert!(dry.contains(real.trim()), "{dry} / {real}");
}

#[test]
fn a_dry_run_needs_no_output_file_but_still_refuses_what_is_wrong() {
    let (map, prog, out) = files();
    let (ok, dry, e) = run(&["apply", &map, &prog, "--dry-run"]);
    assert!(ok && dry.contains("moves corrected"), "{dry} {e}");
    let (ok, _, e) = run(&["apply", &map, "--dry-run"]);
    assert!(!ok && e.contains("needs a map and an input program"), "{e}");
    // a program that was levelled already is refused in a dry run too
    let (ok, _, e) = run(&["apply", &map, &prog, &out]);
    assert!(ok, "{e}");
    let (ok, _, e) = run(&["apply", &map, &out, "--dry-run"]);
    assert!(!ok && e.contains("already levelled"), "{e}");
    let (ok, _, e) = run(&["apply", &map, &prog, "--dry-run", "--oops"]);
    assert!(!ok && e.contains("--oops"), "{e}");
}
