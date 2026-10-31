// EH3D-AUTOLEVEL - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_autolevel::gcode::{correct, Options};
use eh3d_autolevel::surface::Method;
use eh3d_levelling::{simulated_table, Grid, HeightMap};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-autolevel: corrects the Z of a G-code program with the height map of the table

  eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic]
  eh3d-autolevel info MAP.txt                      how many points, the spread and the tilt of the table
  eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS    a map of a simulated tilted table with a bump

MAP.txt is the file that the probe of the suite writes (first line: levelling 1). Absolute millimetres and straight moves only; arcs, G91 and G20 are refused with their line.";

fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>, String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        let v = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?
            .clone();
        args.drain(i..=i + 1);
        return Ok(Some(v));
    }
    Ok(None)
}

fn number(text: &str, what: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what} '{text}' is not a number"))
}

fn load_map(path: &str) -> Result<HeightMap, String> {
    HeightMap::from_text(&fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
        .map_err(|e| format!("{path}: {e}"))
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
                opts.max_segment_mm = number(&s, "--segment")?;
            }
            if let Some(r) = flag(&mut args, "--ref")? {
                let (x, y) = r.split_once(',').ok_or("--ref needs X,Y")?;
                opts.reference = (number(x, "--ref X")?, number(y, "--ref Y")?);
            }
            if let Some(i) = args.iter().position(|a| a == "--bicubic") {
                args.remove(i);
                opts.method = Method::Bicubic;
            }
            if args.len() < 3 {
                return Err("apply needs a map, an input program and an output file".into());
            }
            let map = load_map(&args[0])?;
            let text = fs::read_to_string(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
            let (out, r) = correct(&text, &map, &opts)?;
            fs::write(&args[2], out).map_err(|e| format!("{}: {e}", args[2]))?;
            println!(
                "{} moves corrected, {} pieces added, Z shifted from {:+.4} to {:+.4} mm",
                r.moves, r.pieces_added, r.shift_min, r.shift_max
            );
            Ok(())
        }
        "info" => {
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
            let n = |k: usize, w: &str| number(&args[k], w);
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
