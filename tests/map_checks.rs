//! The checks of the height map: doubtful readings, the roughness, the points not measured, the JSON of `info` and `apply`, the setting `spike-mm` and the noise of `demo-map`.

use eh3d_autolevel::config::{default_file, parse, parse_spike, Settings, DEFAULT_SPIKE_MM};
use eh3d_autolevel::gcode::{report_json, Report};
use eh3d_autolevel::surface::{map_json, missing, rms_from_plane, spike_text, spikes};
use eh3d_levelling::{Grid, HeightMap};
use std::fs;
use std::process::Command;

/// A plane that rises 0.001 mm for every mm along X and 0.002 along Y, 5 columns by 4 rows over 40 x 30 mm.
fn plane_map() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 40.0, 30.0), 5, 4));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, 0.001 * x + 0.002 * y).unwrap();
    }
    m
}

fn with(m: &HeightMap, i: usize, add: f64) -> HeightMap {
    let mut m = m.clone();
    let z = m.get(i).unwrap() + add;
    m.set(i, z).unwrap();
    m
}

// ---- doubtful readings ----

#[test]
fn a_plane_has_no_doubtful_reading_whatever_its_tilt() {
    assert!(spikes(&plane_map(), 0.0001).is_empty());
}

#[test]
fn one_bad_reading_is_the_only_point_listed_and_its_neighbours_are_not() {
    // the point 8 (column 3, row 2) reads 0.5 mm too high
    let m = with(&plane_map(), 7, 0.5);
    let s = spikes(&m, 0.1);
    assert_eq!(s.len(), 1, "{s:?}");
    assert_eq!((s[0].index, s[0].col, s[0].row), (7, 2, 1));
    assert!((s[0].deviation() - 0.5).abs() < 1e-9);
    assert!((s[0].x - 20.0).abs() < 1e-9 && (s[0].y - 10.0).abs() < 1e-9);
    // a reading that is too low is found too, with its sign
    let low = spikes(&with(&plane_map(), 7, -0.5), 0.1);
    assert_eq!(low.len(), 1);
    assert!((low[0].deviation() + 0.5).abs() < 1e-9);
}

#[test]
fn a_reading_under_the_limit_is_not_listed_and_the_limit_is_exclusive() {
    let m = with(&plane_map(), 7, 0.3);
    assert!(spikes(&m, 0.31).is_empty());
    assert_eq!(spikes(&m, 0.29).len(), 1);
}

#[test]
fn no_limit_means_no_check() {
    let m = with(&plane_map(), 7, 5.0);
    for limit in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(spikes(&m, limit).is_empty(), "{limit}");
    }
}

#[test]
fn the_corners_are_never_judged_and_the_edges_are_judged_by_their_pair() {
    let m = with(&plane_map(), 0, 3.0);
    // a corner has no pair of neighbours: it is not listed itself, its neighbours are
    assert!(spikes(&m, 0.1).iter().all(|s| s.index != 0));
    assert!(!spikes(&m, 0.1).is_empty());
    // the point 3 is on the first row, between two neighbours along X
    let edge = spikes(&with(&plane_map(), 2, 0.4), 0.1);
    assert_eq!(edge.len(), 1);
    assert_eq!((edge[0].col, edge[0].row), (2, 0));
    assert!((edge[0].deviation() - 0.4).abs() < 1e-9);
}

#[test]
fn points_that_were_not_measured_are_skipped_and_do_not_hide_a_neighbour() {
    let mut m = with(&plane_map(), 7, 0.5);
    // without the neighbours along X the point is still judged along Y
    let mut nm = HeightMap::new(m.grid);
    for i in 0..m.grid.len() {
        if i != 6 && i != 8 {
            nm.set(i, m.get(i).unwrap()).unwrap();
        }
    }
    assert_eq!(spikes(&nm, 0.1).len(), 1);
    // with none of the neighbours measured nothing can be said about it
    let mut alone = HeightMap::new(m.grid);
    alone.set(7, 9.0).unwrap();
    assert!(spikes(&alone, 0.1).is_empty());
    m = HeightMap::new(m.grid);
    assert!(spikes(&m, 0.1).is_empty());
}

#[test]
fn the_text_of_a_doubtful_reading_has_its_number_place_and_size() {
    let s = spikes(&with(&plane_map(), 7, 0.5), 0.1)[0];
    let t = spike_text(&s);
    assert_eq!(
        t,
        "point 8 of the map (column 3, row 2, at X20.000 Y10.000) reads +0.5400 mm against the +0.0400 mm that its neighbours give: +0.5000 mm off"
    );
}

// ---- roughness and points not measured ----

#[test]
fn a_plane_has_no_roughness_and_a_bad_reading_has() {
    assert!(rms_from_plane(&plane_map()).unwrap() < 1e-9);
    let rough = rms_from_plane(&with(&plane_map(), 7, 0.5)).unwrap();
    // one of 20 points is 0.5 off the plane, and the plane gives way a little: about 0.5 / sqrt(20)
    assert!(rough > 0.09 && rough < 0.12, "{rough}");
}

#[test]
fn the_roughness_needs_a_plane_to_be_fitted() {
    let g = Grid::new((0.0, 0.0, 10.0, 10.0), 3, 3);
    assert_eq!(rms_from_plane(&HeightMap::new(g)), None);
    let mut two = HeightMap::new(g);
    two.set(0, 1.0).unwrap();
    two.set(1, 2.0).unwrap();
    assert_eq!(rms_from_plane(&two), None);
}

#[test]
fn the_points_not_measured_are_numbered_from_one() {
    let g = Grid::new((0.0, 0.0, 10.0, 10.0), 3, 2);
    let mut m = HeightMap::new(g);
    assert_eq!(missing(&m), vec![1, 2, 3, 4, 5, 6]);
    for i in [0, 2, 3] {
        m.set(i, 0.0).unwrap();
    }
    assert_eq!(missing(&m), vec![2, 5, 6]);
    assert!(missing(&plane_map()).is_empty());
}

// ---- the JSON ----

#[test]
fn the_json_of_a_map_has_its_grid_its_numbers_and_its_doubtful_points() {
    let j = map_json(&with(&plane_map(), 7, 0.5), 0.1);
    for part in [
        "\"cols\":5,\"rows\":4,\"points\":20,\"measured\":20",
        "\"x0\":0,\"y0\":0,\"x1\":40,\"y1\":30",
        "\"step_x\":10,\"step_y\":10",
        "\"missing\":[]",
        "\"spikes\":[{\"point\":8,\"column\":3,\"row\":2,\"x\":20,\"y\":10",
    ] {
        assert!(j.contains(part), "{part} in {j}");
    }
    assert!(!j.contains('\n') && j.starts_with('{') && j.ends_with('}'));
}

#[test]
fn the_json_of_a_map_with_nothing_measured_has_null_numbers() {
    let j = map_json(
        &HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2)),
        0.1,
    );
    assert!(
        j.contains("\"lowest\":null,\"highest\":null,\"spread\":null"),
        "{j}"
    );
    assert!(j.contains("\"rms_from_plane\":null"), "{j}");
    assert!(
        j.contains("\"missing\":[1,2,3,4]") && j.contains("\"spikes\":[]"),
        "{j}"
    );
}

#[test]
fn the_json_of_a_report_has_every_count() {
    let r = Report {
        moves: 4,
        pieces_added: 9,
        shift_min: -0.01,
        shift_max: 0.02,
        a_corrected: 1,
        outside_points: 2,
        outside_max_mm: 1.5,
        ..Report::default()
    };
    let j = report_json(&r, true, 3);
    assert_eq!(
        j,
        "{\"dry_run\":true,\"moves\":4,\"pieces_added\":9,\"shift_min\":-0.01,\"shift_max\":0.02,\"a_corrected\":1,\"a_shift_max\":0,\"b_corrected\":0,\"b_shift_max\":0,\"outside_points\":2,\"outside_max_mm\":1.5,\"reference_outside_mm\":0,\"map_spikes\":3}"
    );
    let nan = report_json(
        &Report {
            shift_max: f64::NAN,
            ..r
        },
        false,
        0,
    );
    assert!(
        nan.contains("\"shift_max\":null") && nan.contains("\"dry_run\":false"),
        "{nan}"
    );
}

// ---- the setting ----

#[test]
fn the_limit_of_a_doubtful_reading_is_a_setting_with_a_default_a_range_and_a_line_in_the_file() {
    assert_eq!(Settings::default().spike_mm, DEFAULT_SPIKE_MM);
    assert_eq!(parse("spike-mm = 0.25\n").unwrap().spike_mm, 0.25);
    assert_eq!(parse("spike-mm = 0 # no check\n").unwrap().spike_mm, 0.0);
    assert_eq!(parse_spike(" 10 ").unwrap(), 10.0);
    for bad in ["-0.1", "10.5", "x", "NaN", "inf", ""] {
        let e = parse_spike(bad).unwrap_err();
        assert!(
            e.contains("spike-mm") && e.contains("0 to 10"),
            "{bad}: {e}"
        );
    }
    let e = parse("spike-mm = 11\n").unwrap_err();
    assert!(e.starts_with("line 1:"), "{e}");
    let e = parse("spike-mm = 1\nspike-mm = 2\n").unwrap_err();
    assert!(e.contains("line 2") && e.contains("twice"), "{e}");
    assert!(default_file().contains("\nspike-mm = 0.1\n"));
    assert_eq!(parse(&default_file()).unwrap(), Settings::default());
}

// ---- the command line ----

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-autolevel"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_autolevel_checks_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(dir: &std::path::Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_string()
}

const PROGRAM: &str = "G90 G21\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X50 Y0\nM2\n";

#[test]
fn demo_map_noise_is_repeatable_bounded_and_off_by_default() {
    let dir = scratch("noise");
    let (a, b, c, flat) = (
        path(&dir, "a.txt"),
        path(&dir, "b.txt"),
        path(&dir, "c.txt"),
        path(&dir, "flat.txt"),
    );
    let base = ["0", "0", "100", "100", "6", "6"];
    let mk = |out: &str, extra: &[&str]| {
        let mut args = vec!["demo-map", out];
        args.extend(base);
        args.extend(extra);
        run(&args)
    };
    assert_eq!(mk(&a, &["--noise", "0.05", "--seed", "7"]).0, Some(0));
    assert_eq!(mk(&b, &["--noise", "0.05", "--seed", "7"]).0, Some(0));
    assert_eq!(mk(&c, &["--noise", "0.05", "--seed", "8"]).0, Some(0));
    assert_eq!(mk(&flat, &[]).0, Some(0));
    let (ta, tb, tc) = (
        fs::read_to_string(&a).unwrap(),
        fs::read_to_string(&b).unwrap(),
        fs::read_to_string(&c).unwrap(),
    );
    assert_eq!(ta, tb, "the same seed gives the same map");
    assert_ne!(ta, tc, "another seed gives another one");
    // every point is within the noise of the simulated table
    let noisy = HeightMap::from_text(&ta).unwrap();
    let clean = HeightMap::from_text(&fs::read_to_string(&flat).unwrap()).unwrap();
    let mut worst: f64 = 0.0;
    for i in 0..noisy.grid.len() {
        worst = worst.max((noisy.get(i).unwrap() - clean.get(i).unwrap()).abs());
    }
    assert!(worst > 0.0 && worst <= 0.05 + 1e-4, "{worst}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn demo_map_refuses_a_wrong_noise_seed_or_bad_point() {
    let dir = scratch("demo_bad");
    let out = path(&dir, "m.txt");
    let base = |extra: &[&str]| {
        let mut args = vec!["demo-map", out.as_str(), "0", "0", "100", "100", "6", "6"];
        args.extend(extra.iter().copied());
        run(&args)
    };
    for (extra, what) in [
        (vec!["--noise", "2"], "--noise 2 must be from 0 to 1"),
        (vec!["--noise", "x"], "--noise 'x' is not a number"),
        (vec!["--seed", "-1"], "--seed '-1' is not a whole number"),
        (vec!["--bad", "1,2"], "--bad needs COL,ROW,MM"),
        (vec!["--bad", "0,2,0.5"], "--bad COL '0'"),
        (vec!["--bad", "7,2,0.5"], "outside the map of 6 by 6"),
        (vec!["--bad", "2,2,x"], "--bad MM 'x' is not a number"),
    ] {
        let (code, _, err) = base(&extra);
        assert_eq!(code, Some(1), "{extra:?}");
        assert!(err.contains(what), "{extra:?}: {err}");
    }
    assert!(!std::path::Path::new(&out).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn info_lists_the_doubtful_reading_of_a_simulated_bad_point_and_the_other_numbers() {
    let dir = scratch("info");
    let map = path(&dir, "m.txt");
    let (code, _, err) = run(&[
        "demo-map", &map, "0", "0", "100", "100", "6", "6", "--bad", "3,4,0.4",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let (code, out, err) = run(&["info", &map]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("36 of 36 points measured"), "{out}");
    assert!(out.contains("6 by 6 points over X 0.000 to 100.000 mm and Y 0.000 to 100.000 mm, 20.000 by 20.000 mm apart"), "{out}");
    assert!(
        out.contains("roughness (root mean square distance from the closest plane)"),
        "{out}"
    );
    assert!(
        out.contains("doubtful reading: point 21 of the map (column 3, row 4,"),
        "{out}"
    );
    assert_eq!(out.matches("doubtful reading").count(), 1, "{out}");
    // with the check off, or the limit above the bad reading, nothing is listed
    for limit in ["0", "0.5"] {
        let (_, out, _) = run(&["info", &map, "--spike", limit]);
        assert!(!out.contains("doubtful"), "{limit}: {out}");
    }
    let (code, _, err) = run(&["info", &map, "--spike", "11"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--spike") && err.contains("0 to 10"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn info_says_which_points_were_not_measured() {
    let dir = scratch("missing");
    let map = path(&dir, "m.txt");
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 4, 4));
    for i in 0..16 {
        if i != 5 && i != 6 {
            let (x, y) = m.grid.point(i);
            m.set(i, 0.001 * x + 0.002 * y).unwrap();
        }
    }
    fs::write(&map, m.to_text()).unwrap();
    let (code, out, _) = run(&["info", &map]);
    assert_eq!(code, Some(0));
    assert!(out.contains("14 of 16 points measured"), "{out}");
    assert!(out.contains("2 points not measured: 6, 7"), "{out}");
    // more than ten are cut short
    let empty = HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 4, 4));
    fs::write(&map, empty.to_text()).unwrap();
    let (_, out, _) = run(&["info", &map]);
    assert!(
        out.contains("16 points not measured: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, ..."),
        "{out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn info_json_is_one_object_and_apply_json_replaces_the_sentence() {
    let dir = scratch("json");
    let map = path(&dir, "m.txt");
    let prog = path(&dir, "p.nc");
    let outp = path(&dir, "o.nc");
    fs::write(&prog, PROGRAM).unwrap();
    run(&[
        "demo-map", &map, "0", "0", "100", "100", "6", "6", "--bad", "3,4,0.4",
    ]);
    let (code, out, _) = run(&["info", &map, "--json"]);
    assert_eq!(code, Some(0));
    assert_eq!(out.lines().count(), 1);
    assert!(
        out.contains("\"cols\":6,\"rows\":6,\"points\":36,\"measured\":36"),
        "{out}"
    );
    assert!(
        out.contains("\"spikes\":[{\"point\":21,\"column\":3,\"row\":4"),
        "{out}"
    );
    let (code, out, err) = run(&["apply", &map, &prog, &outp, "--json"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.starts_with("{\"dry_run\":false,\"moves\":"), "{out}");
    assert!(out.contains("\"map_spikes\":1}"), "{out}");
    assert!(!out.contains("moves corrected"), "{out}");
    assert!(std::path::Path::new(&outp).exists());
    let (code, out, _) = run(&["apply", &map, &prog, "--dry-run", "--json"]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("{\"dry_run\":true,"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn apply_warns_of_a_doubtful_reading_and_writes_the_program_all_the_same() {
    let dir = scratch("apply");
    let map = path(&dir, "m.txt");
    let prog = path(&dir, "p.nc");
    let outp = path(&dir, "o.nc");
    fs::write(&prog, PROGRAM).unwrap();
    run(&[
        "demo-map", &map, "0", "0", "100", "100", "6", "6", "--bad", "3,4,0.4",
    ]);
    let (code, out, err) = run(&["apply", &map, &prog, &outp]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        err.contains("warning: point 21 of the map (column 3, row 4,"),
        "{err}"
    );
    assert!(out.contains("moves corrected") && std::path::Path::new(&outp).exists());
    // the check off by the option, and by the file; the option wins over the file
    let _ = fs::remove_file(&outp);
    let (_, _, err) = run(&["apply", &map, &prog, &outp, "--spike", "0"]);
    assert!(
        !err.contains("doubtful") && !err.contains("point 21"),
        "{err}"
    );
    let cfg = path(&dir, "autolevel.cfg");
    fs::write(&cfg, "spike-mm = 0\n").unwrap();
    let _ = fs::remove_file(&outp);
    let (_, _, err) = run(&["apply", &map, &prog, &outp, "--config", &cfg]);
    assert!(!err.contains("point 21"), "{err}");
    let _ = fs::remove_file(&outp);
    let (_, _, err) = run(&[
        "apply", &map, &prog, &outp, "--config", &cfg, "--spike", "0.1",
    ]);
    assert!(err.contains("point 21"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_clean_simulated_table_gives_no_doubtful_reading_with_the_default_limit() {
    let dir = scratch("clean");
    let map = path(&dir, "m.txt");
    let prog = path(&dir, "p.nc");
    fs::write(&prog, PROGRAM).unwrap();
    run(&["demo-map", &map, "0", "0", "240", "180", "9", "7"]);
    let (_, out, _) = run(&["info", &map]);
    assert!(!out.contains("doubtful"), "{out}");
    let (code, _, err) = run(&["apply", &map, &prog, "--dry-run"]);
    assert_eq!(code, Some(0));
    assert!(!err.contains("warning: point"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}
