// EH3D-AUTOLEVEL - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_autolevel::config;
use eh3d_autolevel::gcode::{correct, mark_with_header, Options};
use eh3d_autolevel::surface::Method;
use eh3d_levelling::{simulated_table, Grid, HeightMap};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-autolevel: corrects the Z of a G-code program with the height map of the table

  eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic] [--max-lines 5000000] [--decimals 3] [--config FILE] [--dry-run]
  eh3d-autolevel config                            every setting with its default, as a file
  eh3d-autolevel info MAP.txt                      how many points, the spread and the tilt of the table
  eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS    a map of a simulated tilted table with a bump
  eh3d-autolevel [gui] [FILE...]                   the window (a build with --features gui): with no arguments, with files (a program, a map) or with gui it opens; the commands above never open it
  eh3d-autolevel gui [FILE...] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

--dry-run corrects and counts but writes nothing (OUT.nc may then be left out).
--decimals N writes X, Y and Z with N decimals (1 to 6, 3 by default); the same key is `decimals = N` in the file of --config, and the option wins over the file. The same goes for --max-lines (key `max-lines`).
A, B and C are interpolated like X and Y when a cutting move is cut in pieces.
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
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
    match cmd.as_str() {
        "apply" => {
            let mut opts = Options::default();
            if let Some(path) = flag(&mut args, "--config")? {
                let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                let settings = config::parse(&text).map_err(|e| format!("{path}: {e}"))?;
                opts.decimals = settings.decimals;
                opts.max_output_lines = settings.max_lines;
            }
            if let Some(d) = flag(&mut args, "--decimals")? {
                opts.decimals =
                    config::parse_decimals(&d).map_err(|e| format!("--decimals: {e}"))?;
            }
            if let Some(s) = flag(&mut args, "--segment")? {
                opts.max_segment_mm = num(&s, "--segment")?;
            }
            if let Some(r) = flag(&mut args, "--ref")? {
                let (x, y) = r.split_once(',').ok_or("--ref needs X,Y")?;
                opts.reference = (num(x, "--ref X")?, num(y, "--ref Y")?);
            }
            if let Some(m) = flag(&mut args, "--max-lines")? {
                let lines = num(&m, "--max-lines")?;
                if !(lines >= 1.0 && lines.fract() == 0.0 && lines <= 100_000_000.0) {
                    return Err(format!(
                        "--max-lines {m} must be a whole number from 1 to 100000000"
                    ));
                }
                opts.max_output_lines = lines as usize;
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
            fs::write(
                &tmp,
                mark_with_header(
                    &out,
                    &map,
                    &opts,
                    Some((
                        std::path::Path::new(&args[1])
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("program"),
                        text.as_str(),
                    )),
                ),
            )
            .and_then(|()| fs::rename(&tmp, &args[2]))
            .map_err(|e| {
                let _ = fs::remove_file(&tmp);
                format!("{}: {e}", args[2])
            })?;
            println!("{summary}");
            Ok(())
        }
        "config" => {
            finish(&args, 0)?;
            print!("{}", config::default_file());
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

/// Whether the program opens its window instead of running a command, and the files to open in it: no arguments, `gui` (with files or without), or only files given by the system to
/// open (names with an extension that are files; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Vec<String>> {
    match args {
        [] => Some(Vec::new()),
        [a, files @ ..] if a == "gui" => Some(files.to_vec()),
        files
            if files.iter().all(|f| {
                !f.starts_with('-')
                    && std::path::Path::new(f).extension().is_some()
                    && std::path::Path::new(f).is_file()
            }) =>
        {
            Some(files.to_vec())
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(files: Vec<String>) -> ExitCode {
    let lang = eh3d_autolevel::lang::Lang::from_env();
    let files = files.into_iter().map(std::path::PathBuf::from).collect();
    match eh3d_autolevel::gui::run(files, lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_autolevel::shot::parse_window_args(args, 64) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_autolevel::lang::Lang::from_env);
    match eh3d_autolevel::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_autolevel::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(files) = window_request(&args) {
        return open_window(files);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
