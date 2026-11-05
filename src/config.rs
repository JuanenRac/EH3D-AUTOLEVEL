//! The settings file of the project: `key = value`, one per line, `#` starts a comment. Today it has one key, `decimals`, the number of decimals the coordinates of the levelled
//! program are written with. The command line takes the same key as an option (`--decimals 3`), which wins over the file. A line that makes no sense is an error that says its number.

use crate::gcode::{DEFAULT_DECIMALS, MAX_DECIMALS, MIN_DECIMALS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// The decimals of X, Y and Z in the levelled program, from 1 to 6.
    pub decimals: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            decimals: DEFAULT_DECIMALS,
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

/// Reads a settings file. A key that is not known, a line with no `=` and a value that is not valid are errors with the number of the line; a key given twice is an error too.
pub fn parse(text: &str) -> Result<Settings, String> {
    let mut settings = Settings::default();
    let mut seen_decimals = false;
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
            other => return Err(format!("line {n}: '{other}' is not a setting")),
        }
    }
    Ok(settings)
}

/// Every setting with its default, as a file that [`parse`] reads back.
pub fn default_file() -> String {
    format!(
        "# EH3D-AUTOLEVEL settings\n# decimals of X, Y and Z in the levelled program ({MIN_DECIMALS} to {MAX_DECIMALS})\ndecimals = {DEFAULT_DECIMALS}\n"
    )
}
