//! The settings file of the project: `key = value`, one per line, `#` starts a comment. Today it has five keys: `decimals`, the number of decimals the coordinates of the levelled
//! program are written with, `max-lines`, the most lines the levelled program may have, and `warn-shift-mm`, the correction of Z above which a warning is shown. The command line takes the same keys as options (`--decimals 3`, `--max-lines 5000`, `--warn-shift 0.3`), which win over the file. `a-axis-compensation` (`off` or `tilt`; `--a-compensation`) and `a-axis-radius-mm` (`--a-radius`) add to Z the tilt that the turn of the axis A brings, and `b-axis-compensation` and `b-axis-radius-mm` (`--b-compensation`, `--b-radius`) the same for the axis B, and `unknown-axis` (`assume-zero` or `refuse`; `--unknown-axis`) says what a move does that leaves out an axis the program has not set yet; the axis C turns about Z and needs none. `arcs` (`refuse` or `cut`; `--arcs`) and `arc-tolerance-mm` (`--arc-tolerance`) say whether an arc (G2, G3) is cut in straight pieces and how far a piece may stray from it. The keys `stream-min-z-mm`, `stream-max-shift-mm`, `stream-outside-mm` and `stream-safe-z-mm` are the limits of the streaming emitter (see `stream`). The keys `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns` and `server_max_answer_mib` set the link to the EH3D server (see `server`). A line that makes no sense is an error that says its number.

use crate::arcs::{
    parse_tolerance, DEFAULT_ARC_TOLERANCE_MM, MAX_ARC_TOLERANCE_MM, MIN_ARC_TOLERANCE_MM,
};
use crate::gcode::{
    AAxisCompensation, ArcMode, Options, UnknownAxis, DEFAULT_DECIMALS, DEFAULT_MAX_OUTPUT_LINES,
    MAX_A_RADIUS_MM, MAX_DECIMALS, MAX_OUTPUT_LINES_LIMIT, MIN_DECIMALS,
};
use crate::stream::{StreamLimits, MAX_STREAM_SAFE_Z_MM};

/// The largest correction of Z that may be asked for as a warning limit, in mm.
pub const MAX_WARN_SHIFT_MM: f64 = 100.0;
/// The smallest correction of Z that may be asked for as a warning limit, in mm (0 gives no warning).
pub const MIN_WARN_SHIFT_MM: f64 = 0.001;
/// The most a point of the map may differ from its neighbours before it is shown as a doubtful reading, in mm that may be asked for (0 gives no check).
pub const MAX_SPIKE_MM: f64 = 10.0;
/// The default of that limit, in mm.
pub const DEFAULT_SPIKE_MM: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The decimals of X, Y and Z in the levelled program, from 1 to 6.
    pub decimals: usize,
    /// The most lines the levelled program may have, from 1 to 100000000.
    pub max_lines: usize,
    /// The correction of Z, in mm, above which `apply` shows a warning; 0 gives none, otherwise from 0.001 to 100.
    pub warn_shift_mm: f64,
    /// Whether the turn of the axis A is added to Z (`off` by default).
    pub a_axis_compensation: AAxisCompensation,
    /// The distance from the axis A to the cut, in mm, from -1000 to 1000; it must not be 0 when the compensation is `tilt`.
    pub a_axis_radius_mm: f64,
    pub b_axis_compensation: AAxisCompensation,
    pub b_axis_radius_mm: f64,
    /// What a move does that leaves out X, Y or Z before the program has set it: `assume-zero` (the program starts at 0, the way it always was) or `refuse`.
    pub unknown_axis: UnknownAxis,
    /// How far, in mm, a point of the map may read from what its neighbours give before `apply` and `info` show it as a doubtful reading; from 0 to 10, 0 gives no check.
    pub spike_mm: f64,
    /// What an arc does: `refuse` (the way it always was) or `cut` in straight pieces.
    pub arcs: ArcMode,
    /// The most an arc may stray from its straight pieces, in mm, from 0.001 to 1.
    pub arc_tolerance_mm: f64,
    /// The limits of the streaming emitter.
    pub stream: StreamLimits,
}

impl Settings {
    /// Puts the settings that change how a program is levelled into `opts`.
    pub fn apply_to(&self, opts: &mut Options) {
        opts.decimals = self.decimals;
        opts.max_output_lines = self.max_lines;
        opts.a_compensation = self.a_axis_compensation;
        opts.a_radius_mm = self.a_axis_radius_mm;
        opts.b_compensation = self.b_axis_compensation;
        opts.b_radius_mm = self.b_axis_radius_mm;
        opts.unknown_axis = self.unknown_axis;
        opts.arcs = self.arcs;
        opts.arc_tolerance_mm = self.arc_tolerance_mm;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            decimals: DEFAULT_DECIMALS,
            max_lines: DEFAULT_MAX_OUTPUT_LINES,
            warn_shift_mm: 0.0,
            a_axis_compensation: AAxisCompensation::Off,
            a_axis_radius_mm: 0.0,
            b_axis_compensation: AAxisCompensation::Off,
            b_axis_radius_mm: 0.0,
            unknown_axis: UnknownAxis::AssumeZero,
            spike_mm: DEFAULT_SPIKE_MM,
            arcs: ArcMode::Refuse,
            arc_tolerance_mm: DEFAULT_ARC_TOLERANCE_MM,
            stream: StreamLimits::default(),
        }
    }
}

/// The value of the doubtful-reading limit as the file or the command line gives it: from 0 (no check) to 10 mm.
pub fn parse_spike(text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (0.0..=MAX_SPIKE_MM).contains(v))
        .ok_or_else(|| {
            format!(
                "spike-mm '{text}' must be a number from 0 to {MAX_SPIKE_MM} mm (0 for no check)"
            )
        })
}

/// The value of `arcs` as the file or the command line gives it: `refuse` or `cut`.
pub fn parse_arcs(text: &str) -> Result<ArcMode, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "refuse" => Ok(ArcMode::Refuse),
        "cut" => Ok(ArcMode::Cut),
        _ => Err(format!("arcs '{text}' must be refuse or cut")),
    }
}

/// A number of the streaming limits, between `lo` and `hi` (both included).
fn parse_stream_number(key: &str, text: &str, lo: f64, hi: f64) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (lo..=hi).contains(v))
        .ok_or_else(|| format!("{key} '{text}' must be a number from {lo} to {hi} mm"))
}

/// The value of `stream-min-z-mm`: the deepest Z the streamed program may reach, from -1000 to 0 mm.
pub fn parse_stream_min_z(text: &str) -> Result<f64, String> {
    parse_stream_number("stream-min-z-mm", text, -1000.0, 0.0)
}

/// The value of `stream-max-shift-mm`: the largest correction of Z the streamed program may get either way, 0 for no limit, otherwise from 0.001 to 100 mm.
pub fn parse_stream_max_shift(text: &str) -> Result<f64, String> {
    let v = parse_stream_number("stream-max-shift-mm", text, 0.0, MAX_WARN_SHIFT_MM)?;
    if v != 0.0 && v < MIN_WARN_SHIFT_MM {
        return Err(format!(
            "stream-max-shift-mm '{text}' must be 0 (no limit) or a number from {MIN_WARN_SHIFT_MM} to {MAX_WARN_SHIFT_MM} mm"
        ));
    }
    Ok(v)
}

/// The value of `stream-outside-mm`: how far outside the measured area the streamed program may go, from 0 to 100 mm.
pub fn parse_stream_outside(text: &str) -> Result<f64, String> {
    parse_stream_number("stream-outside-mm", text, 0.0, 100.0)
}

/// The value of `stream-safe-z-mm`: the Z the tool is lifted to when the stream stops, from 0 to 1000 mm.
pub fn parse_stream_safe_z(text: &str) -> Result<f64, String> {
    parse_stream_number("stream-safe-z-mm", text, 0.0, MAX_STREAM_SAFE_Z_MM)
}

/// The value of `decimals` as the file or the command line gives it: a whole number from 1 to 6.
pub fn parse_decimals(text: &str) -> Result<usize, String> {
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|d| (MIN_DECIMALS..=MAX_DECIMALS).contains(d))
        .ok_or_else(|| {
            format!(
                "decimals '{text}' must be a whole number from {MIN_DECIMALS} to {MAX_DECIMALS}"
            )
        })
}

/// The value of `max-lines` as the file gives it: a whole number from 1 to 100000000.
pub fn parse_max_lines(text: &str) -> Result<usize, String> {
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=MAX_OUTPUT_LINES_LIMIT).contains(n))
        .ok_or_else(|| {
            format!("max-lines '{text}' must be a whole number from 1 to {MAX_OUTPUT_LINES_LIMIT}")
        })
}

/// The value of `warn-shift-mm` as the file or the command line gives it: 0 (no warning) or a number from 0.001 to 100 mm.
pub fn parse_warn_shift(text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| *v == 0.0 || (MIN_WARN_SHIFT_MM..=MAX_WARN_SHIFT_MM).contains(v))
        .ok_or_else(|| {
            format!("warn-shift-mm '{text}' must be 0 (no warning) or a number from {MIN_WARN_SHIFT_MM} to {MAX_WARN_SHIFT_MM} mm")
        })
}

/// The value of `unknown-axis` as the file or the command line gives it: `assume-zero` or `refuse`.
pub fn parse_unknown_axis(text: &str) -> Result<UnknownAxis, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "assume-zero" => Ok(UnknownAxis::AssumeZero),
        "refuse" => Ok(UnknownAxis::Refuse),
        _ => Err(format!(
            "unknown-axis '{text}' must be assume-zero or refuse"
        )),
    }
}

/// The value of `a-axis-compensation` as the file or the command line gives it: `off` or `tilt`.
pub fn parse_a_compensation(text: &str) -> Result<AAxisCompensation, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(AAxisCompensation::Off),
        "tilt" => Ok(AAxisCompensation::Tilt),
        _ => Err(format!("a-axis-compensation '{text}' must be off or tilt")),
    }
}

/// The value of `a-axis-radius-mm` as the file or the command line gives it: a number from -1000 to 1000 mm.
pub fn parse_a_radius(text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && v.abs() <= MAX_A_RADIUS_MM)
        .ok_or_else(|| {
            format!("a-axis-radius-mm '{text}' must be a number from -{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM} mm")
        })
}

/// The value of `b-axis-compensation` as the file or the command line gives it: `off` or `tilt`.
pub fn parse_b_compensation(text: &str) -> Result<AAxisCompensation, String> {
    parse_a_compensation(text).map_err(|e| e.replacen("a-axis", "b-axis", 1))
}

/// The value of `b-axis-radius-mm` as the file or the command line gives it: a number from -1000 to 1000 mm.
pub fn parse_b_radius(text: &str) -> Result<f64, String> {
    parse_a_radius(text).map_err(|e| e.replacen("a-axis", "b-axis", 1))
}

/// Reads a settings file. A key that is not known, a line with no `=` and a value that is not valid are errors with the number of the line; a key given twice is an error too.
pub fn parse(text: &str) -> Result<Settings, String> {
    let mut settings = Settings::default();
    let mut seen_decimals = false;
    let mut seen_max_lines = false;
    let mut seen_warn_shift = false;
    let mut seen_a_comp: Option<usize> = None;
    let mut seen_a_radius = false;
    let mut seen_b_comp: Option<usize> = None;
    let mut seen_b_radius = false;
    let mut seen_unknown_axis = false;
    let mut seen_spike = false;
    // the keys that are only given once: arcs, arc-tolerance-mm and the four of the streaming emitter
    let mut seen_once: Vec<&str> = Vec::new();
    // the keys of the link to the EH3D server are read by `server::settings_from_file`; here they are only checked
    let mut seen_server: Vec<&str> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {n}: '{line}' is not 'key = value'"))?;
        let (key, value) = (key.trim(), value.trim());
        match key {
            "decimals" => {
                if seen_decimals {
                    return Err(format!("line {n}: decimals is given twice"));
                }
                seen_decimals = true;
                settings.decimals = parse_decimals(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "max-lines" => {
                if seen_max_lines {
                    return Err(format!("line {n}: max-lines is given twice"));
                }
                seen_max_lines = true;
                settings.max_lines =
                    parse_max_lines(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "warn-shift-mm" => {
                if seen_warn_shift {
                    return Err(format!("line {n}: warn-shift-mm is given twice"));
                }
                seen_warn_shift = true;
                settings.warn_shift_mm =
                    parse_warn_shift(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "a-axis-compensation" => {
                if seen_a_comp.is_some() {
                    return Err(format!("line {n}: a-axis-compensation is given twice"));
                }
                seen_a_comp = Some(n);
                settings.a_axis_compensation =
                    parse_a_compensation(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "a-axis-radius-mm" => {
                if seen_a_radius {
                    return Err(format!("line {n}: a-axis-radius-mm is given twice"));
                }
                seen_a_radius = true;
                settings.a_axis_radius_mm =
                    parse_a_radius(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "b-axis-compensation" => {
                if seen_b_comp.is_some() {
                    return Err(format!("line {n}: b-axis-compensation is given twice"));
                }
                seen_b_comp = Some(n);
                settings.b_axis_compensation =
                    parse_b_compensation(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "b-axis-radius-mm" => {
                if seen_b_radius {
                    return Err(format!("line {n}: b-axis-radius-mm is given twice"));
                }
                seen_b_radius = true;
                settings.b_axis_radius_mm =
                    parse_b_radius(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "unknown-axis" => {
                if seen_unknown_axis {
                    return Err(format!("line {n}: unknown-axis is given twice"));
                }
                seen_unknown_axis = true;
                settings.unknown_axis =
                    parse_unknown_axis(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "spike-mm" => {
                if seen_spike {
                    return Err(format!("line {n}: spike-mm is given twice"));
                }
                seen_spike = true;
                settings.spike_mm = parse_spike(value).map_err(|e| format!("line {n}: {e}"))?;
            }
            "arcs"
            | "arc-tolerance-mm"
            | "stream-min-z-mm"
            | "stream-max-shift-mm"
            | "stream-outside-mm"
            | "stream-safe-z-mm" => {
                if seen_once.contains(&key) {
                    return Err(format!("line {n}: {key} is given twice"));
                }
                seen_once.push(key);
                let bad = |e: String| format!("line {n}: {e}");
                match key {
                    "arcs" => settings.arcs = parse_arcs(value).map_err(bad)?,
                    "arc-tolerance-mm" => {
                        settings.arc_tolerance_mm = parse_tolerance(value).map_err(bad)?
                    }
                    "stream-min-z-mm" => {
                        settings.stream.min_z_mm = parse_stream_min_z(value).map_err(bad)?
                    }
                    "stream-max-shift-mm" => {
                        settings.stream.max_shift_mm = parse_stream_max_shift(value).map_err(bad)?
                    }
                    "stream-outside-mm" => {
                        settings.stream.outside_mm = parse_stream_outside(value).map_err(bad)?
                    }
                    _ => settings.stream.safe_z_mm = parse_stream_safe_z(value).map_err(bad)?,
                }
            }
            other if crate::server::is_server_key(other) => {
                crate::server::check_key(other, value).map_err(|e| format!("line {n}: {e}"))?;
                if seen_server.contains(&other) {
                    return Err(format!("line {n}: {other} is given twice"));
                }
                seen_server.push(other);
            }
            other => return Err(format!("line {n}: '{other}' is not a setting")),
        }
    }
    if let (AAxisCompensation::Tilt, Some(n)) = (settings.a_axis_compensation, seen_a_comp) {
        if settings.a_axis_radius_mm == 0.0 {
            return Err(format!(
                "line {n}: a-axis-compensation = tilt needs a-axis-radius-mm other than 0"
            ));
        }
    }
    if let (AAxisCompensation::Tilt, Some(n)) = (settings.b_axis_compensation, seen_b_comp) {
        if settings.b_axis_radius_mm == 0.0 {
            return Err(format!(
                "line {n}: b-axis-compensation = tilt needs b-axis-radius-mm other than 0"
            ));
        }
    }
    Ok(settings)
}

/// Every setting with its default, as a file that [`parse`] reads back.
pub fn default_file() -> String {
    format!(
        "# EH3D-AUTOLEVEL settings\n# decimals of X, Y and Z in the levelled program ({MIN_DECIMALS} to {MAX_DECIMALS})\ndecimals = {DEFAULT_DECIMALS}\n# the most lines the levelled program may have (1 to {MAX_OUTPUT_LINES_LIMIT})\nmax-lines = {DEFAULT_MAX_OUTPUT_LINES}\n# warn when the correction of Z passes this many mm up or down ({MIN_WARN_SHIFT_MM} to {MAX_WARN_SHIFT_MM}, 0 for no warning)\nwarn-shift-mm = 0\n# Z correction for the turn of axis A: off (nothing changes) or tilt (adds radius * (1 - cos A) to Z)\na-axis-compensation = off\n# distance in mm from the axis A to the point that is cut, used by tilt (-{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}, not 0 for tilt)\na-axis-radius-mm = 0\n# Z correction for the turn of axis B (a turn about Y): off or tilt, the same as for A; the axis C turns about Z and needs none\nb-axis-compensation = off\n# distance in mm from the axis B to the point that is cut, used by tilt (-{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}, not 0 for tilt)\nb-axis-radius-mm = 0\n# a move that leaves out X, Y or Z before the program has set it: assume-zero (the program starts at 0) or refuse (the program is refused with the number of the line)\nunknown-axis = assume-zero\n# show a point of the map that reads this many mm above or below what its neighbours give, as a doubtful reading (0 to {MAX_SPIKE_MM}, 0 for no check)\nspike-mm = {DEFAULT_SPIKE_MM}\n# an arc (G2, G3): refuse (the program is refused with the number of the line) or cut (cut in straight pieces that follow the table)\narcs = refuse\n# the most that an arc may stray from its straight pieces, in mm ({MIN_ARC_TOLERANCE_MM} to {MAX_ARC_TOLERANCE_MM}); a piece is never longer than the segment either\narc-tolerance-mm = {DEFAULT_ARC_TOLERANCE_MM}\n# the streaming emitter stops when a corrected Z would go below this many mm (-1000 to 0)\nstream-min-z-mm = {}\n# it stops when the correction of Z passes this many mm either way (0 for no limit, otherwise {MIN_WARN_SHIFT_MM} to {MAX_WARN_SHIFT_MM})\nstream-max-shift-mm = {}\n# it stops when a move goes more than this many mm outside the measured area (0 to 100)\nstream-outside-mm = {}\n# the Z the tool is lifted to when the stream stops, before the correction (0 to {MAX_STREAM_SAFE_Z_MM})\nstream-safe-z-mm = {}\n",
        StreamLimits::default().min_z_mm,
        StreamLimits::default().max_shift_mm,
        StreamLimits::default().outside_mm,
        StreamLimits::default().safe_z_mm
    )
}
