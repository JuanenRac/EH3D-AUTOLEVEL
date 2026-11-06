//! The settings file of the project: `key = value`, one per line, `#` starts a comment. Today it has two keys: `decimals`, the number of decimals the coordinates of the levelled
//! program are written with, and `max-lines`, the most lines the levelled program may have. The command line takes the same keys as options (`--decimals 3`, `--max-lines 5000`), which win over the file. A line that makes no sense is an error that says its number.

use crate::gcode::{
    DEFAULT_DECIMALS, DEFAULT_MAX_OUTPUT_LINES, MAX_DECIMALS, MAX_OUTPUT_LINES_LIMIT, MIN_DECIMALS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// The decimals of X, Y and Z in the levelled program, from 1 to 6.
    pub decimals: usize,
    /// The most lines the levelled program may have, from 1 to 100000000.
    pub max_lines: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            decimals: DEFAULT_DECIMALS,
            max_lines: DEFAULT_MAX_OUTPUT_LINES,
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

/// Reads a settings file. A key that is not known, a line with no `=` and a value that is not valid are errors with the number of the line; a key given twice is an error too.
pub fn parse(text: &str) -> Result<Settings, String> {
    let mut settings = Settings::default();
    let mut seen_decimals = false;
    let mut seen_max_lines = false;
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
            other => return Err(format!("line {n}: '{other}' is not a setting")),
        }
    }
    Ok(settings)
}

/// Every setting with its default, as a file that [`parse`] reads back.
pub fn default_file() -> String {
    format!(
        "# EH3D-AUTOLEVEL settings\n# decimals of X, Y and Z in the levelled program ({MIN_DECIMALS} to {MAX_DECIMALS})\ndecimals = {DEFAULT_DECIMALS}\n# the most lines the levelled program may have (1 to {MAX_OUTPUT_LINES_LIMIT})\nmax-lines = {DEFAULT_MAX_OUTPUT_LINES}\n"
    )
}
