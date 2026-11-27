// EH3D-AUTOLEVEL - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_autolevel::config;
use eh3d_autolevel::gcode::{
    a_axis_notice, b_axis_notice, correct, mark_with_header, outside_warning, realtime_warning,
    report_json, shift_warning, Options,
};
use eh3d_autolevel::server;
use eh3d_autolevel::start;
use eh3d_autolevel::stream::{send_program, SimMachine, StreamLimits};
use eh3d_autolevel::surface::{map_json, missing, rms_from_plane, spike_text, spikes, Method};
use eh3d_autolevel::window::{read_limited, write_atomic, MAX_FILE_BYTES};
use eh3d_levelling::{simulated_table, Grid, HeightMap};
use eh3d_serverclient::Settings as LinkSettings;
use std::env;
use std::process::ExitCode;

const HELP: &str = "eh3d-autolevel: corrects the Z of a G-code program with the height map of the table

  eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic] [--max-lines 5000000] [--decimals 3] [--warn-shift MM] [--a-compensation off|tilt] [--a-radius MM] [--spike MM] [--arcs refuse|cut] [--arc-tolerance MM] [--start X,Y,Z | --start-report FILE] [--config FILE] [--dry-run] [--json]
  eh3d-autolevel stream MAP.txt IN.nc [--out FILE] [--segment 1.0] [--ref X,Y] [--bicubic] [--arcs refuse|cut] [--arc-tolerance MM] [--start X,Y,Z | --start-report FILE] [--config FILE] [--json]   levels the program line by line as an emitter would send it, to a simulated machine; exit code 2 when it stops
  eh3d-autolevel config                            every setting with its default, as a file
  eh3d-autolevel server [status] [--config FILE]   the link to the EH3D server (server_address, server_token, server_machine, server_timeout_s, server_retries, server_mdns and server_max_answer_mib of the settings file) and what the server answers
  eh3d-autolevel info MAP.txt [--spike MM] [--json]   how many points, the area, the spread, the tilt and the roughness of the table, the points not measured and the doubtful readings
  eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS [--noise MM] [--seed N] [--bad COL,ROW,MM]    a map of a simulated tilted table with a bump
  eh3d-autolevel [gui] [FILE...]                   the window (a build with --features gui): with no arguments, with files (a program, a map) or with gui it opens; the commands above never open it
  eh3d-autolevel gui [FILE...] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

--to-server (apply, info --json and demo-map) keeps the corrected program, the JSON of info or the map on the EH3D server as an artifact, with the values of the run; it needs --config FILE with server_address and server_token and is refused before any work when they are missing (and with --dry-run, which writes nothing).
--dry-run corrects and counts but writes nothing (OUT.nc may then be left out).
The output of apply and of stream --out is never the program or the map that are read. A `$` command of GRBL passes through only when it asks ($, $$, $#, $G, $I, $N): a jog ($J=), the homing ($H) and every other one are refused with their line.
--spike MM (0.1 by default, 0 to 10; 0 gives no check) shows, on the error output, a point of the map that reads more than MM above or below what its neighbours give (a probe that slipped, a chip under the tip): the program is written all the same. The key of the file is `spike-mm`. info lists the same points. --json prints the result of apply or of info as one JSON object, for other programs, instead of the sentence.
--noise MM and --seed N of demo-map add to every point of the simulated table a reading error of up to MM (0 to 1; the same seed gives the same map) and --bad COL,ROW,MM adds MM to one point (COL and ROW from 1): a map to try the check of doubtful readings on.
--decimals N writes X, Y and Z with N decimals (1 to 6, 3 by default); the same key is `decimals = N` in the file of --config, and the option wins over the file. The same goes for --max-lines (key `max-lines`).
--unknown-axis refuse (assume-zero by default: the program starts at 0 on every axis) refuses a move that leaves out X, Y or Z before the program has set it, with the number of the line; the key of the file is `unknown-axis`.
--warn-shift MM (0 by default: no warning; otherwise 0.001 to 100) shows a warning on the error output when the correction of Z passes MM up or down; the program is written all the same. The key in the file is `warn-shift-mm`.
--a-compensation tilt (off by default) adds r * (1 - cos A) to Z for the turn of the axis A, with r from --a-radius MM (-1000 to 1000, not 0); the keys of the file are `a-axis-compensation` and `a-axis-radius-mm`, and the options win over the file. The axis B (a turn about Y) has the same with --b-compensation tilt and --b-radius MM (`b-axis-compensation`, `b-axis-radius-mm`) and the two add up; the axis C turns about Z and needs none (its words are carried like A and B). A notice on the error output says when it was applied.
A, B and C are interpolated like X and Y when a cutting move is cut in pieces.
--arcs cut (refuse by default; key `arcs`) cuts an arc (G2, G3 in the plane of X and Y, with I and J or with R) in straight pieces that follow the table; --arc-tolerance MM (key `arc-tolerance-mm`, 0.001 to 1, 0.01 by default) is the most a piece may stray from the arc, and a piece is never longer than --segment either.
--start X,Y,Z says where the tool stands when the program starts (as the machine reports it, so with the correction of the table in Z); --start-report FILE reads it from a saved status report of GRBL or answer to M114 of Marlin. Without either the program is taken to start at 0 on every axis.
stream sends the program through the emitter to a simulated machine, line by line, and stops with the exit code 2 (the spindle off and the tool lifted) when a line goes outside the measured area by more than `stream-outside-mm`, would go below `stream-min-z-mm`, gets a correction beyond `stream-max-shift-mm` or cannot be levelled; --out FILE keeps the lines that were sent. Nothing is sent to a real machine.
MAP.txt is the file that the probe of the suite writes (first line: levelling 1). Absolute millimetres; straight moves, and arcs when --arcs cut is given; G91 and G20 are refused with their line.";

/// The text of a file given on the command line, with the same size limit as the window.
fn read_text(path: &str) -> Result<String, String> {
    read_limited(std::path::Path::new(path), MAX_FILE_BYTES)
}

fn load_map(path: &str) -> Result<HeightMap, String> {
    HeightMap::from_text(&read_text(path)?).map_err(|e| format!("{path}: {e}"))
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

/// Refuses an output that is the map or the program that are read.
fn never_over_an_input(out: &str, map: &str, program: &str) -> Result<(), String> {
    for (input, what) in [(map, "map"), (program, "program")] {
        if same_file(out, input) {
            return Err(format!(
                "{out}: the output file is also the {what}; write to another file"
            ));
        }
    }
    Ok(())
}

/// A reading error of the simulated table: up to `amplitude` mm either way, from a small generator (xorshift) that the same seed repeats.
fn reading_error(state: &mut u64, amplitude: f64) -> f64 {
    if amplitude == 0.0 {
        return 0.0;
    }
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    ((*state % 20_001) as f64 / 10_000.0 - 1.0) * amplitude
}

/// The link to the EH3D server for `--to-server`: the keys of the settings file, which must name a server and hold its token.
fn link_settings(config: Option<&str>) -> Result<LinkSettings, String> {
    let path =
        config.ok_or("--to-server needs --config FILE with server_address and server_token")?;
    let settings = server::settings_from_file(path)?;
    server::require(&settings)?;
    Ok(settings)
}

/// `server [status] [--config FILE]`: the link, with the token as `***`, and what the server answers.
fn server_command(mut args: Vec<String>) -> Result<(), String> {
    let config = flag(&mut args, "--config")?;
    let word = args.first().cloned().unwrap_or_else(|| "status".into());
    finish(&args, 1)?;
    if word != "status" {
        return Err(format!("server {word}: unknown (use: server status)"));
    }
    let settings = match config {
        Some(path) => server::settings_from_file(&path)?,
        None => LinkSettings::default(),
    };
    print!("{}", server::status_report(settings)?);
    Ok(())
}

/// Keeps a file on the server and says so; the file was already written, so a refusal is told as such.
fn keep(
    link: &LinkSettings,
    name: &str,
    format: &str,
    data: &[u8],
    params: &[(&str, &str)],
) -> Result<(), String> {
    let line = server::keep(link.clone(), name, format, data, params)
        .map_err(|e| format!("{name}: not kept on the server: {e}"))?;
    println!("{line}");
    Ok(())
}

/// The last part of a path, as text.
fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string()
}

/// The name of a path without its extension.
fn stem_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string()
}

/// The options about arcs and about where the program starts, taken out of `args` (they win over the settings file).
fn take_arc_and_start_options(args: &mut Vec<String>, opts: &mut Options) -> Result<(), String> {
    if let Some(a) = flag(args, "--arcs")? {
        opts.arcs = config::parse_arcs(&a).map_err(|e| format!("--arcs: {e}"))?;
    }
    if let Some(t) = flag(args, "--arc-tolerance")? {
        opts.arc_tolerance_mm = eh3d_autolevel::arcs::parse_tolerance(&t)
            .map_err(|e| format!("--arc-tolerance: {e}"))?;
    }
    let place = flag(args, "--start")?;
    let report = flag(args, "--start-report")?;
    opts.start = match (place, report) {
        (Some(_), Some(_)) => {
            return Err("--start and --start-report cannot be given together".into())
        }
        (Some(p), None) => Some(start::parse_place(&p).map_err(|e| format!("--start: {e}"))?),
        (None, Some(path)) => Some(
            start::from_report(&read_text(&path)?)
                .map_err(|e| format!("--start-report {path}: {e}"))?,
        ),
        (None, None) => None,
    };
    Ok(())
}

/// `stream MAP.txt IN.nc`: the program goes through the emitter, line by line, to a simulated machine. `Ok(true)` when the stream stopped before the end of the program.
fn stream_command(mut args: Vec<String>) -> Result<bool, String> {
    let mut opts = Options::default();
    let mut limits = StreamLimits::default();
    if let Some(path) = flag(&mut args, "--config")? {
        let settings = config::parse(&read_text(&path)?).map_err(|e| format!("{path}: {e}"))?;
        settings.apply_to(&mut opts);
        limits = settings.stream;
    }
    take_arc_and_start_options(&mut args, &mut opts)?;
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
    let out_path = flag(&mut args, "--out")?;
    let json = switch(&mut args, "--json");
    if args.len() < 2 {
        return Err("stream needs a map and an input program".into());
    }
    finish(&args, 2)?;
    if let Some(path) = &out_path {
        never_over_an_input(path, &args[0], &args[1])?;
    }
    let map = load_map(&args[0])?;
    let text = read_text(&args[1])?;
    let mut machine = SimMachine::new(&map, opts.reference, opts.method);
    let outcome = send_program(&text, &map, &opts, limits, &mut machine)?;
    if let Some(path) = &out_path {
        let mut body: Vec<&str> = outcome.sent.iter().map(String::as_str).collect();
        body.extend(outcome.safe.iter().map(String::as_str));
        write_atomic(
            std::path::Path::new(path),
            &format!("{}\n", body.join("\n")),
        )?;
    }
    let halt_text = outcome.halt.as_ref().map(|h| h.to_string());
    if json {
        use eh3d_autolevel::surface::json_number as jn;
        println!(
            "{{\"stopped\":{},\"stop_line\":{},\"stop_reason\":{},\"sent\":{},\"safe_lines\":{},\"deepest_mm\":{},\"rapid_into_table\":{},\"moves\":{},\"pieces_added\":{},\"arcs_cut\":{}}}",
            outcome.halt.is_some(),
            outcome.halt.as_ref().map_or("null".to_string(), |h| h.line.to_string()),
            outcome.halt.as_ref().map_or("null".to_string(), |h| format!("\"{}\"", h.reason.code())),
            outcome.sent.len(),
            outcome.safe.len(),
            jn(machine.deepest_mm),
            machine.rapid_into_table,
            outcome.report.moves,
            outcome.report.pieces_added,
            outcome.report.arcs_cut
        );
    } else {
        println!(
            "{} lines sent, {} moves, {} pieces added, deepest cut {:+.4} mm from the surface of the table, {} rapid moves below it",
            outcome.sent.len(),
            outcome.report.moves,
            outcome.report.pieces_added,
            machine.deepest_mm,
            machine.rapid_into_table
        );
    }
    if let Some(text) = halt_text {
        eprintln!("stopped: {text}");
        eprintln!("then sent: {}", outcome.safe.join(" ; "));
        return Ok(true);
    }
    Ok(false)
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
    if cmd == "server" {
        return server_command(args);
    }
    match cmd.as_str() {
        "apply" => {
            let mut opts = Options::default();
            let mut warn_shift = 0.0;
            let mut spike = config::DEFAULT_SPIKE_MM;
            let config_path = flag(&mut args, "--config")?;
            if let Some(path) = &config_path {
                let text = read_text(path)?;
                let settings = config::parse(&text).map_err(|e| format!("{path}: {e}"))?;
                settings.apply_to(&mut opts);
                warn_shift = settings.warn_shift_mm;
                spike = settings.spike_mm;
            }
            if let Some(u) = flag(&mut args, "--unknown-axis")? {
                opts.unknown_axis =
                    config::parse_unknown_axis(&u).map_err(|e| format!("--unknown-axis: {e}"))?;
            }
            take_arc_and_start_options(&mut args, &mut opts)?;
            if let Some(c) = flag(&mut args, "--a-compensation")? {
                opts.a_compensation = config::parse_a_compensation(&c)
                    .map_err(|e| format!("--a-compensation: {e}"))?;
            }
            if let Some(r) = flag(&mut args, "--a-radius")? {
                opts.a_radius_mm =
                    config::parse_a_radius(&r).map_err(|e| format!("--a-radius: {e}"))?;
            }
            if let Some(c) = flag(&mut args, "--b-compensation")? {
                opts.b_compensation = config::parse_b_compensation(&c)
                    .map_err(|e| format!("--b-compensation: {e}"))?;
            }
            if let Some(r) = flag(&mut args, "--b-radius")? {
                opts.b_radius_mm =
                    config::parse_b_radius(&r).map_err(|e| format!("--b-radius: {e}"))?;
            }
            if let Some(w) = flag(&mut args, "--warn-shift")? {
                warn_shift =
                    config::parse_warn_shift(&w).map_err(|e| format!("--warn-shift: {e}"))?;
            }
            if let Some(s) = flag(&mut args, "--spike")? {
                spike = config::parse_spike(&s).map_err(|e| format!("--spike: {e}"))?;
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
            let json = switch(&mut args, "--json");
            let to_server = switch(&mut args, "--to-server");
            if to_server && dry_run {
                return Err(
                    "--to-server keeps the corrected program, and --dry-run writes none".into(),
                );
            }
            // the link is checked before any work is done
            let link = if to_server {
                Some(link_settings(config_path.as_deref())?)
            } else {
                None
            };
            let needed = if dry_run { 2 } else { 3 };
            if args.len() < needed {
                return Err(if dry_run {
                    "apply needs a map and an input program".into()
                } else {
                    "apply needs a map, an input program and an output file".into()
                });
            }
            finish(&args, 3)?;
            // what is read is never written over
            if !dry_run {
                never_over_an_input(&args[2], &args[0], &args[1])?;
            }
            let map = load_map(&args[0])?;
            let text = read_text(&args[1])?;
            let (out, r) = correct(&text, &map, &opts)?;
            let doubtful = spikes(&map, spike);
            for s in &doubtful {
                eprintln!("warning: {}", spike_text(s));
            }
            let mut summary = format!(
                "{} moves corrected, {} pieces added, Z shifted from {:+.4} to {:+.4} mm",
                r.moves, r.pieces_added, r.shift_min, r.shift_max
            );
            if r.arcs_cut > 0 {
                summary.push_str(&format!(", {} arcs cut", r.arcs_cut));
            }
            if dry_run {
                // the program is corrected and counted, and nothing is written
                if json {
                    println!("{}", report_json(&r, true, doubtful.len()));
                } else {
                    println!("dry run, nothing written: {summary}");
                }
                if let Some(n) = a_axis_notice(&r, &opts) {
                    eprintln!("warning: {n}");
                }
                if let Some(n) = b_axis_notice(&r, &opts) {
                    eprintln!("warning: {n}");
                }
                if let Some(w) = shift_warning(&r, warn_shift) {
                    eprintln!("warning: {w}");
                }
                if let Some(w) = outside_warning(&r) {
                    eprintln!("warning: {w}");
                }
                if let Some(w) = realtime_warning(&r) {
                    eprintln!("warning: {w}");
                }
                return Ok(());
            }
            // written beside and renamed, so that the output is never left half written
            let corrected = mark_with_header(
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
            );
            write_atomic(std::path::Path::new(&args[2]), &corrected)?;
            if json {
                println!("{}", report_json(&r, false, doubtful.len()));
            } else {
                println!("{summary}");
            }
            if let Some(n) = a_axis_notice(&r, &opts) {
                eprintln!("warning: {n}");
            }
            if let Some(n) = b_axis_notice(&r, &opts) {
                eprintln!("warning: {n}");
            }
            if let Some(w) = shift_warning(&r, warn_shift) {
                eprintln!("warning: {w}");
            }
            if let Some(w) = outside_warning(&r) {
                eprintln!("warning: {w}");
            }
            if let Some(w) = realtime_warning(&r) {
                eprintln!("warning: {w}");
            }
            if let Some(link) = &link {
                let figures = [
                    r.moves.to_string(),
                    r.pieces_added.to_string(),
                    format!("{:+.4}", r.shift_min),
                    format!("{:+.4}", r.shift_max),
                    opts.decimals.to_string(),
                    opts.max_segment_mm.to_string(),
                ];
                let method = if matches!(opts.method, Method::Bicubic) {
                    "bicubic"
                } else {
                    "bilinear"
                };
                let map_name = file_name_of(&args[0]);
                let program_name = file_name_of(&args[1]);
                keep(
                    link,
                    &args[2],
                    server::media_type(&args[2]),
                    corrected.as_bytes(),
                    &[
                        ("command", "apply"),
                        ("map", map_name.as_str()),
                        ("program", program_name.as_str()),
                        ("method", method),
                        ("moves", figures[0].as_str()),
                        ("pieces_added", figures[1].as_str()),
                        ("shift_min_mm", figures[2].as_str()),
                        ("shift_max_mm", figures[3].as_str()),
                        ("decimals", figures[4].as_str()),
                        ("segment_mm", figures[5].as_str()),
                    ],
                )?;
            }
            Ok(())
        }
        "config" => {
            finish(&args, 0)?;
            print!("{}", config::default_file());
            Ok(())
        }
        "info" => {
            let config_path = flag(&mut args, "--config")?;
            let spike = match flag(&mut args, "--spike")? {
                Some(s) => config::parse_spike(&s).map_err(|e| format!("--spike: {e}"))?,
                None => config::DEFAULT_SPIKE_MM,
            };
            let json = switch(&mut args, "--json");
            let to_server = switch(&mut args, "--to-server");
            if to_server && !json {
                return Err("--to-server is an option of info --json".into());
            }
            // the link is checked before any work is done
            let link = if to_server {
                Some(link_settings(config_path.as_deref())?)
            } else {
                None
            };
            finish(&args, 1)?;
            let map = load_map(args.first().ok_or("info needs a map")?)?;
            let doubtful = spikes(&map, spike);
            if json {
                let report = map_json(&map, spike);
                println!("{report}");
                if let Some(link) = &link {
                    let spike_text = spike.to_string();
                    keep(
                        link,
                        &format!("{}.info.json", stem_of(&args[0])),
                        "application/json",
                        report.as_bytes(),
                        &[("command", "info"), ("spike_mm", spike_text.as_str())],
                    )?;
                }
                return Ok(());
            }
            println!("{} of {} points measured", map.measured(), map.grid.len());
            let g = &map.grid;
            let (sx, sy) = g.step();
            println!(
                "{} by {} points over X {:.3} to {:.3} mm and Y {:.3} to {:.3} mm, {:.3} by {:.3} mm apart",
                g.cols, g.rows, g.x0, g.x1, g.y0, g.y1, sx, sy
            );
            if let Some((lo, hi, spread)) = map.spread() {
                println!("lowest {lo:+.4} mm, highest {hi:+.4} mm, spread {spread:.4} mm");
            }
            if let Some((tx, ty)) = map.tilt_mm_per_m() {
                println!("tilt {tx:+.3} mm/m along X, {ty:+.3} mm/m along Y");
            }
            if let Some(rms) = rms_from_plane(&map) {
                println!(
                    "roughness (root mean square distance from the closest plane) {rms:.4} mm"
                );
            }
            let absent = missing(&map);
            if !absent.is_empty() {
                let shown: Vec<String> = absent.iter().take(10).map(|i| i.to_string()).collect();
                println!(
                    "{} points not measured: {}{}",
                    absent.len(),
                    shown.join(", "),
                    if absent.len() > 10 { ", ..." } else { "" }
                );
            }
            for s in &doubtful {
                println!("doubtful reading: {}", spike_text(s));
            }
            Ok(())
        }
        "demo-map" => {
            let config_path = flag(&mut args, "--config")?;
            let to_server = switch(&mut args, "--to-server");
            // the link is checked before any work is done
            let link = if to_server {
                Some(link_settings(config_path.as_deref())?)
            } else {
                None
            };
            let noise = match flag(&mut args, "--noise")? {
                Some(v) => {
                    let n = num(&v, "--noise")?;
                    if !(0.0..=1.0).contains(&n) {
                        return Err(format!("--noise {v} must be from 0 to 1 mm"));
                    }
                    n
                }
                None => 0.0,
            };
            let seed = match flag(&mut args, "--seed")? {
                Some(v) => v
                    .parse::<u64>()
                    .map_err(|_| format!("--seed '{v}' is not a whole number"))?,
                None => 1,
            };
            let bad = flag(&mut args, "--bad")?;
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
            let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
            for i in 0..map.grid.len() {
                let (x, y) = map.grid.point(i);
                map.set(i, simulated_table(x, y) + reading_error(&mut state, noise))
                    .map_err(|e| e.to_string())?;
            }
            if let Some(b) = bad {
                let parts: Vec<&str> = b.split(',').collect();
                let [c, r, d] = parts[..] else {
                    return Err("--bad needs COL,ROW,MM".into());
                };
                let whole = |t: &str, what: &str| {
                    t.trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|v| *v >= 1)
                        .ok_or_else(|| format!("--bad {what} '{t}' is not a whole number from 1"))
                };
                let (c, r) = (whole(c, "COL")?, whole(r, "ROW")?);
                if c > cols || r > rows {
                    return Err(format!(
                        "--bad {c},{r} is outside the map of {cols} by {rows} points"
                    ));
                }
                let i = (r - 1) * cols + (c - 1);
                let z = map.get(i).unwrap_or(0.0) + num(d, "--bad MM")?;
                map.set(i, z).map_err(|e| e.to_string())?;
            }
            write_atomic(std::path::Path::new(&args[0]), &map.to_text())?;
            println!("{} points written to {}", map.grid.len(), args[0]);
            if let Some(link) = &link {
                let (cols_text, rows_text) = (cols.to_string(), rows.to_string());
                keep(
                    link,
                    &args[0],
                    server::media_type(&args[0]),
                    map.to_text().as_bytes(),
                    &[
                        ("command", "demo-map"),
                        ("cols", cols_text.as_str()),
                        ("rows", rows_text.as_str()),
                    ],
                )?;
            }
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
    if args.first().map(String::as_str) == Some("stream") {
        return match stream_command(args[1..].to_vec()) {
            Ok(false) => ExitCode::SUCCESS,
            // the verification did not pass: the stream stopped
            Ok(true) => ExitCode::from(2),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        };
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
