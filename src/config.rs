//! The settings file of the project: `key = value`, one per line, `#` starts a comment. Today it has five keys: `decimals`, the number of decimals the coordinates of the levelled
//! program are written with, `max-lines`, the most lines the levelled program may have, and `warn-shift-mm`, the correction of Z above which a warning is shown. The command line takes the same keys as options (`--decimals 3`, `--max-lines 5000`, `--warn-shift 0.3`), which win over the file. `a-axis-compensation` (`off` or `tilt`; `--a-compensation`) and `a-axis-radius-mm` (`--a-radius`) add to Z the tilt that the turn of the axis A brings, and `b-axis-compensation` and `b-axis-radius-mm` (`--b-compensation`, `--b-radius`) the same for the axis B; the axis C turns about Z and needs none. A line that makes no sense is an error that says its number.

use crate::gcode::{
    AAxisCompensation, DEFAULT_DECIMALS, DEFAULT_MAX_OUTPUT_LINES, MAX_A_RADIUS_MM, MAX_DECIMALS,
    MAX_OUTPUT_LINES_LIMIT, MIN_DECIMALS,
};

/// The largest correction of Z that may be asked for as a warning limit, in mm.
pub const MAX_WARN_SHIFT_MM: f64 = 100.0;
/// The smallest correction of Z that may be asked for as a warning limit, in mm (0 gives no warning).
pub const MIN_WARN_SHIFT_MM: f64 = 0.001;

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
        }
    }
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
        "# EH3D-AUTOLEVEL settings\n# decimals of X, Y and Z in the levelled program ({MIN_DECIMALS} to {MAX_DECIMALS})\ndecimals = {DEFAULT_DECIMALS}\n# the most lines the levelled program may have (1 to {MAX_OUTPUT_LINES_LIMIT})\nmax-lines = {DEFAULT_MAX_OUTPUT_LINES}\n# warn when the correction of Z passes this many mm up or down ({MIN_WARN_SHIFT_MM} to {MAX_WARN_SHIFT_MM}, 0 for no warning)\nwarn-shift-mm = 0\n# Z correction for the turn of axis A: off (nothing changes) or tilt (adds radius * (1 - cos A) to Z)\na-axis-compensation = off\n# distance in mm from the axis A to the point that is cut, used by tilt (-{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}, not 0 for tilt)\na-axis-radius-mm = 0\n# Z correction for the turn of axis B (a turn about Y): off or tilt, the same as for A; the axis C turns about Z and needs none\nb-axis-compensation = off\n# distance in mm from the axis B to the point that is cut, used by tilt (-{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}, not 0 for tilt)\nb-axis-radius-mm = 0\n"
    )
}
