//! Rewrites a G-code program so that the tool keeps the same depth into the board wherever the table is high or low: every cutting move (G1) is cut in pieces no longer than
//! `max_segment_mm` and each piece gets the Z that the map gives for its end; a rapid move (G0) only has the Z corrected at its end when it has a Z word.
//!
//! What is supported: absolute millimetres (G90, G21) and straight moves (G0, G1). What is refused, with the number of the line: relative distances (G91), inches (G20), arcs (G2, G3)
//! and a move before any G0 or G1. The program is assumed to start at X0 Y0 Z0. Outside the grid the height of the nearest edge is used.

use crate::surface::{height, Method};
use eh3d_levelling::HeightMap;
use eh3d_parser_gcode::emit::{header, sha256_hex, Header};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// The longest piece of a cutting move.
    pub max_segment_mm: f64,
    /// The point whose height is the zero of the correction (where the Z of the job was set).
    pub reference: (f64, f64),
    pub method: Method,
    /// The most lines the corrected program may have. A program that would pass it is refused before the pieces are made, so a tiny `max_segment_mm` cannot fill the memory.
    pub max_output_lines: usize,
    /// How many decimals the coordinates X, Y and Z are written with (always that many, `X1.000`), from [`MIN_DECIMALS`] to [`MAX_DECIMALS`].
    pub decimals: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_segment_mm: 1.0,
            reference: (0.0, 0.0),
            method: Method::Bilinear,
            max_output_lines: DEFAULT_MAX_OUTPUT_LINES,
            decimals: DEFAULT_DECIMALS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Report {
    /// Moves that were corrected.
    pub moves: usize,
    /// Pieces added by cutting long moves.
    pub pieces_added: usize,
    /// The smallest and the largest correction of Z that was applied.
    pub shift_min: f64,
    pub shift_max: f64,
}

const MAX_COORDINATE: f64 = 1.0e6;
const MAX_PIECES: usize = 100_000;
/// The default of [`Options::max_output_lines`].
pub const DEFAULT_MAX_OUTPUT_LINES: usize = 5_000_000;
/// The default of [`Options::decimals`].
pub const DEFAULT_DECIMALS: usize = 3;
/// The fewest and the most decimals a coordinate may be written with.
pub const MIN_DECIMALS: usize = 1;
pub const MAX_DECIMALS: usize = 6;

/// The words of a line without its comments: `(...)` and everything after `;`.
pub fn words(line: &str, number: usize) -> Result<Vec<(char, f64)>, String> {
    let mut clean = String::new();
    let mut depth = 0;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            // the checksum of a numbered line (`N10 G1 X5*45`) is not a word
            '*' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => clean.push(c),
            _ => {}
        }
    }
    let chars: Vec<char> = clean.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let letter = chars[i].to_ascii_uppercase();
        if !letter.is_ascii_alphabetic() {
            return Err(format!(
                "line {number}: '{}' is not a letter of a word",
                chars[i]
            ));
        }
        let start = i + 1;
        i = start;
        while i < chars.len() && (chars[i].is_ascii_digit() || matches!(chars[i], '+' | '-' | '.'))
        {
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        let value = text
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("line {number}: '{letter}{text}' has no valid number"))?;
        out.push((letter, value));
    }
    Ok(out)
}

/// Whether a line is one that is passed on as it is: `$` commands of GRBL, `#` parameter assignments and the message commands M117 and M118 with their text. Only the lines that move
/// are held to the strict rules of the words; the strictness is about positions, not about every character a controller accepts.
fn is_not_a_move(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with('$') || t.starts_with('#') {
        return true;
    }
    let upper = t.to_ascii_uppercase();
    ["M117", "M118"].iter().any(|m| {
        upper
            .strip_prefix(m)
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.'))
    })
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// A coordinate with exactly `decimals` decimals; a value that rounds to zero is written without a sign.
pub fn coord(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_string()
    } else {
        s
    }
}

/// The start of the comment that marks a program as already levelled.
pub const MARK: &str = "(eh3d-autolevel";

/// The number of the line that carries the mark of a levelled program, when the text has one.
pub fn levelled_line(text: &str) -> Option<usize> {
    text.lines()
        .position(|l| l.trim_start().starts_with(MARK))
        .map(|i| i + 1)
}

/// The corrected program with a comment line that says it was levelled, with which map (a hash of its text), reference point, method and piece length. The comment goes first,
/// or after the `%` that some postprocessors put at the start. A program with the mark is refused by `correct`, so a levelled program is never levelled twice.
pub fn mark(program: &str, map: &HeightMap, opts: &Options) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for b in map.to_text().bytes() {
        hash = (hash ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    let line = format!(
        "{MARK} {} map={hash:08x} ref={},{} {:?} seg={})\n",
        env!("CARGO_PKG_VERSION"),
        fmt(opts.reference.0),
        fmt(opts.reference.1),
        opts.method,
        fmt(opts.max_segment_mm)
    );
    // the '%' that opens the program stays first, with the line end it has
    for lead in ["%\r\n", "%\n"] {
        if let Some(rest) = program.strip_prefix(lead) {
            return format!("{lead}{line}{rest}");
        }
    }
    format!("{line}{program}")
}

/// Like [`mark`] and with the standard header of the suite in front of the mark: the generator and its version, the units, the program the work came from (its name and the SHA-256
/// of its text) and the tools. The header is comment lines, so the program is the same for a controller. It goes after the `%` that opens a program, like the mark.
pub fn mark_with_header(
    program: &str,
    map: &HeightMap,
    opts: &Options,
    source: Option<(&str, &str)>,
) -> String {
    let marked = mark(program, map, opts);
    let head = header(&Header {
        generator: "eh3d-autolevel",
        version: env!("CARGO_PKG_VERSION"),
        source: source.map(|(name, text)| (name, sha256_hex(text.as_bytes()))),
        tools: "as in the original program",
    });
    for lead in ["%\r\n", "%\n"] {
        if let Some(rest) = marked.strip_prefix(lead) {
            return format!("{lead}{head}{rest}");
        }
    }
    format!("{head}{marked}")
}

/// Corrects `text` with `map`. The map must be complete. A program that already carries the mark of a levelled one is refused: its Z is already corrected.
pub fn correct(text: &str, map: &HeightMap, opts: &Options) -> Result<(String, Report), String> {
    if let Some(n) = levelled_line(text) {
        return Err(format!("line {n}: this program was already levelled by eh3d-autolevel; level the original program, not its output"));
    }
    map.require_complete().map_err(|e| e.to_string())?;
    if !(MIN_DECIMALS..=MAX_DECIMALS).contains(&opts.decimals) {
        return Err(format!(
            "decimals is {}: it must be a whole number from {MIN_DECIMALS} to {MAX_DECIMALS}",
            opts.decimals
        ));
    }
    let d = opts.decimals;
    if !(opts.max_segment_mm.is_finite() && opts.max_segment_mm > 0.0) {
        return Err(format!(
            "max_segment_mm is {}: it must be more than 0",
            opts.max_segment_mm
        ));
    }
    let at = |x: f64, y: f64, line: usize| {
        height(map, x, y, opts.method)
            .filter(|h| h.is_finite())
            .ok_or_else(|| format!("line {line}: the map has no height at ({x:.3}, {y:.3})"))
    };
    let zero = at(opts.reference.0, opts.reference.1, 0)?;
    let mut out = String::new();
    let mut report = Report {
        shift_min: f64::INFINITY,
        shift_max: f64::NEG_INFINITY,
        ..Report::default()
    };
    let mut mode: Option<u32> = None;
    let mut pos = (0.0, 0.0, 0.0);
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        // the '%' that some postprocessors put at the start and at the end of the program is not a word
        if raw.trim() == "%" {
            out.push_str(raw);
            out.push('\n');
            continue;
        }
        // lines that are not moves and have no words to read: a system command of GRBL (`$H`), a parameter assignment (`#1=5`) and the text of a message (`M117 Hello`)
        if is_not_a_move(raw) {
            out.push_str(raw);
            out.push('\n');
            continue;
        }
        let w = words(raw, n)?;
        let mut moves_here = false;
        let mut dwell = false;
        // the modal G words that share the line with a move (G90, G54...) go in front of the rewritten move
        let mut kept: Vec<String> = Vec::new();
        for &(letter, value) in &w {
            match (letter, value) {
                ('G', v)
                    if v == 90.0
                        || v == 21.0
                        || v == 17.0
                        || v == 40.0
                        || v == 49.0
                        || v == 61.0
                        || v == 64.0
                        || v == 80.0
                        || v == 94.0
                        || (54.0..=59.0).contains(&v)
                        || v == 90.1
                        || v == 91.1 => kept.push(format!("G{}", fmt(v))),
                // a dwell takes its time in P (or X on some controllers): the line is not a move
                ('G', 4.0) => dwell = true,
                ('G', 91.0) => {
                    return Err(format!(
                        "line {n}: relative distances (G91) are not supported"
                    ))
                }
                ('G', 20.0) => return Err(format!("line {n}: inches (G20) are not supported")),
                ('G', 2.0 | 3.0) => {
                    return Err(format!(
                    "line {n}: arcs (G2, G3) are not supported: they cannot be cut in pieces here"
                ))
                }
                ('G', v) if v == 0.0 || v == 1.0 => mode = Some(v as u32),
                // every other code changes the origin, the offsets or the way the machine moves (G10, G28, G30, G38, G43, G53, G92...):
                // turning it into a plain move would be wrong, and dropping it would change the program
                ('G', v) => {
                    return Err(format!(
                        "line {n}: G{} is not supported (only G0, G1, G4, G17, G21, G40, G49, G54-G59, G61, G64, G80, G90, G90.1, G91.1 and G94 are)",
                        fmt(v)
                    ))
                }
                ('X' | 'Y' | 'Z', _) => moves_here = true,
                _ => {}
            }
        }
        if dwell {
            moves_here = false;
        }
        if !moves_here {
            out.push_str(raw);
            out.push('\n');
            continue;
        }
        let m = mode.ok_or_else(|| format!("line {n}: a move before any G0 or G1"))?;
        let get = |c: char, default: f64| {
            w.iter()
                .rev()
                .find(|(l, _)| *l == c)
                .map(|(_, v)| *v)
                .unwrap_or(default)
        };
        let target = (get('X', pos.0), get('Y', pos.1), get('Z', pos.2));
        for v in [target.0, target.1, target.2] {
            if v.abs() > MAX_COORDINATE {
                return Err(format!(
                    "line {n}: the coordinate {v} is beyond {MAX_COORDINATE} mm"
                ));
            }
        }
        // the M words of a move happen when the move is done (`G1 X50 M5` stops the spindle at X50, not at the first piece): in a cut move they go with the last piece
        let (m_words, others): (Vec<String>, Vec<String>) = {
            let rest: Vec<(char, f64)> = w
                .iter()
                .filter(|(l, _)| !matches!(l, 'G' | 'X' | 'Y' | 'Z'))
                .copied()
                .collect();
            let text = |&(l, v): &(char, f64)| format!("{l}{}", fmt(v));
            (
                rest.iter().filter(|(l, _)| *l == 'M').map(text).collect(),
                rest.iter().filter(|(l, _)| *l != 'M').map(text).collect(),
            )
        };
        let has_z = w.iter().any(|(l, _)| *l == 'Z');
        let mut track = |shift: f64| {
            report.shift_min = report.shift_min.min(shift);
            report.shift_max = report.shift_max.max(shift);
        };
        if m == 0 {
            let mut line = kept.clone();
            line.push("G0".to_string());
            for (l, v) in w.iter().filter(|(l, _)| matches!(l, 'X' | 'Y')) {
                line.push(format!("{l}{}", coord(*v, d)));
            }
            if has_z {
                let shift = at(target.0, target.1, n)? - zero;
                track(shift);
                line.push(format!("Z{}", coord(target.2 + shift, d)));
                report.moves += 1;
            }
            line.extend(others);
            line.extend(m_words);
            out.push_str(&line.join(" "));
            out.push('\n');
        } else {
            let length = (target.0 - pos.0).hypot(target.1 - pos.1);
            let pieces = ((length / opts.max_segment_mm).ceil().max(1.0)) as usize;
            if pieces > MAX_PIECES {
                return Err(format!("line {n}: the move would be cut in {pieces} pieces (more than {MAX_PIECES}); use a longer piece"));
            }
            if i + report.pieces_added + pieces > opts.max_output_lines {
                return Err(format!("line {n}: the corrected program would have more than {} lines; use a longer piece (--segment) or raise the limit (--max-lines)", opts.max_output_lines));
            }
            for k in 1..=pieces {
                let t = k as f64 / pieces as f64;
                let (x, y, z) = (
                    pos.0 + (target.0 - pos.0) * t,
                    pos.1 + (target.1 - pos.1) * t,
                    pos.2 + (target.2 - pos.2) * t,
                );
                let shift = at(x, y, n)? - zero;
                track(shift);
                let mut line = format!(
                    "G1 X{} Y{} Z{}",
                    coord(x, d),
                    coord(y, d),
                    coord(z + shift, d)
                );
                if k == 1 && !kept.is_empty() {
                    line = format!("{} {line}", kept.join(" "));
                }
                if k == 1 && !others.is_empty() {
                    line.push(' ');
                    line.push_str(&others.join(" "));
                }
                if k == pieces && !m_words.is_empty() {
                    line.push(' ');
                    line.push_str(&m_words.join(" "));
                }
                out.push_str(&line);
                out.push('\n');
            }
            report.moves += 1;
            report.pieces_added += pieces - 1;
        }
        pos = target;
    }
    if report.moves == 0 {
        report.shift_min = 0.0;
        report.shift_max = 0.0;
    }
    Ok((out, report))
}
