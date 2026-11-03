// EH3D-AUTOLEVEL - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_autolevel::gcode::{correct, mark, Options};
use eh3d_autolevel::surface::Method;
use eh3d_levelling::{simulated_table, Grid, HeightMap};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-autolevel: corrects the Z of a G-code program with the height map of the table

  eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic] [--dry-run]
  eh3d-autolevel info MAP.txt                      how many points, the spread and the tilt of the table
  eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS    a map of a simulated tilted table with a bump

--dry-run corrects and counts but writes nothing (OUT.nc may then be left out).
MAP.txt is the file that the probe of the suite writes (first line: levelling 1). Absolute millimetres and straight moves only; arcs, G91 and G20 are refused with their line.";

fn load_map(path: &str) -> Result<HeightMap, String> {
    HeightMap::from_text(&fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
        .map_err(|e| format!("{path}: {e}"))
}

/// True when two paths name the same file (compared once resolved; by their text when one does not exist yet).
fn same_file(a: &str, b: &str) -> bool {
    match (
        std::path::Path::new(a).canonicalize(),
        std::path::Path::new(b).canonicalize(),
    ) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "apply" => {
            let mut opts = Options::default();
            if let Some(s) = flag(&mut args, "--segment")? {
                opts.max_segment_mm = num(&s, "--segment")?;
            }
            if let Some(r) = flag(&mut args, "--ref")? {
                let (x, y) = r.split_once(',').ok_or("--ref needs X,Y")?;
                opts.reference = (num(x, "--ref X")?, num(y, "--ref Y")?);
            }
            if switch(&mut args, "--bicubic") {
                opts.method = Method::Bicubic;
            }
            let dry_run = switch(&mut args, "--dry-run");
            let needed = if dry_run { 2 } else { 3 };
            if args.len() < needed {
                return Err(if dry_run {
                    "apply needs a map and an input program".into()
                } else {
                    "apply needs a map, an input program and an output file".into()
                });
            }
            finish(&args, 3)?;
            // the output may be the program (levelling in place: the mark refuses a second run) but never the map
            if !dry_run && same_file(&args[2], &args[0]) {
                return Err(format!(
                    "{}: the output file is also the map; write the corrected program to another file",
                    args[2]
                ));
            }
            let map = load_map(&args[0])?;
            let text = fs::read_to_string(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
            let (out, r) = correct(&text, &map, &opts)?;
            let summary = format!(
                "{} moves corrected, {} pieces added, Z shifted from {:+.4} to {:+.4} mm",
                r.moves, r.pieces_added, r.shift_min, r.shift_max
            );
            if dry_run {
                // the program is corrected and counted, and nothing is written
                println!("dry run, nothing written: {summary}");
                return Ok(());
            }
            // written beside and renamed, so that a program levelled in place is never left half written
            let tmp = format!("{}.tmp", args[2]);
            fs::write(&tmp, mark(&out, &map, &opts))
                .and_then(|()| fs::rename(&tmp, &args[2]))
                .map_err(|e| {
                    let _ = fs::remove_file(&tmp);
                    format!("{}: {e}", args[2])
                })?;
            println!("{summary}");
            Ok(())
        }
        "info" => {
            finish(&args, 1)?;
            let map = load_map(args.first().ok_or("info needs a map")?)?;
            println!("{} of {} points measured", map.measured(), map.grid.len());
            if let Some((lo, hi, spread)) = map.spread() {
                println!("lowest {lo:+.4} mm, highest {hi:+.4} mm, spread {spread:.4} mm");
            }
            if let Some((tx, ty)) = map.tilt_mm_per_m() {
                println!("tilt {tx:+.3} mm/m along X, {ty:+.3} mm/m along Y");
            }
            Ok(())
        }
        "demo-map" => {
            if args.len() < 7 {
                return Err("demo-map needs OUT X0 Y0 X1 Y1 COLS ROWS".into());
            }
            finish(&args, 7)?;
            let n = |k: usize, w: &str| num(&args[k], w);
            let cols = args[5]
                .parse::<usize>()
                .map_err(|_| format!("COLS '{}' is not a whole number", args[5]))?;
            let rows = args[6]
                .parse::<usize>()
                .map_err(|_| format!("ROWS '{}' is not a whole number", args[6]))?;
            if !(2..=200).contains(&cols) || !(2..=200).contains(&rows) {
                return Err("COLS and ROWS must be from 2 to 200".into());
            }
            let mut map = HeightMap::new(Grid::new(
                (n(1, "X0")?, n(2, "Y0")?, n(3, "X1")?, n(4, "Y1")?),
                cols,
                rows,
            ));
            for i in 0..map.grid.len() {
                let (x, y) = map.grid.point(i);
                map.set(i, simulated_table(x, y))
                    .map_err(|e| e.to_string())?;
            }
            fs::write(&args[0], map.to_text()).map_err(|e| format!("{}: {e}", args[0]))?;
            println!("{} points written to {}", map.grid.len(), args[0]);
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
