//! Rewrites a G-code program so that the tool keeps the same depth into the board wherever the table is high or low: every cutting move (G1) is cut in pieces no longer than
//! `max_segment_mm` and each piece gets the Z that the map gives for its end; a rapid move (G0) only has the Z corrected at its end when it has a Z word.
//!
//! What is supported: absolute millimetres (G90, G21) and straight moves (G0, G1). What is refused, with the number of the line: relative distances (G91), inches (G20), arcs (G2, G3)
//! and a move before any G0 or G1. The program is assumed to start at X0 Y0 Z0. Outside the grid the height of the nearest edge is used.

use crate::surface::{height, Method};
use eh3d_levelling::HeightMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// The longest piece of a cutting move.
    pub max_segment_mm: f64,
    /// The point whose height is the zero of the correction (where the Z of the job was set).
    pub reference: (f64, f64),
    pub method: Method,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_segment_mm: 1.0,
            reference: (0.0, 0.0),
            method: Method::Bilinear,
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

/// The words of a line without its comments: `(...)` and everything after `;`.
fn words(line: &str, number: usize) -> Result<Vec<(char, f64)>, String> {
    let mut clean = String::new();
    let mut depth = 0;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
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

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Corrects `text` with `map`. The map must be complete.
pub fn correct(text: &str, map: &HeightMap, opts: &Options) -> Result<(String, Report), String> {
    map.require_complete().map_err(|e| e.to_string())?;
    if !(opts.max_segment_mm.is_finite() && opts.max_segment_mm > 0.0) {
        return Err(format!(
            "max_segment_mm is {}: it must be more than 0",
            opts.max_segment_mm
        ));
    }
    let at = |x: f64, y: f64, line: usize| {
        height(map, x, y, opts.method)
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
        let w = words(raw, n)?;
        let mut moves_here = false;
        for &(letter, value) in &w {
            match (letter, value) {
                ('G', v)
                    if v == 90.0
                        || v == 21.0
                        || v == 17.0
                        || (54.0..=59.0).contains(&v)
                        || v == 90.1
                        || v == 91.1 => {}
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
                ('X' | 'Y' | 'Z', _) => moves_here = true,
                _ => {}
            }
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
        let others: Vec<String> = w
            .iter()
            .filter(|(l, _)| !matches!(l, 'G' | 'X' | 'Y' | 'Z'))
            .map(|(l, v)| format!("{l}{}", fmt(*v)))
            .collect();
        let has_z = w.iter().any(|(l, _)| *l == 'Z');
        let mut track = |shift: f64| {
            report.shift_min = report.shift_min.min(shift);
            report.shift_max = report.shift_max.max(shift);
        };
        if m == 0 {
            let mut line = vec!["G0".to_string()];
            for (l, v) in w.iter().filter(|(l, _)| matches!(l, 'X' | 'Y')) {
                line.push(format!("{l}{}", fmt(*v)));
            }
            if has_z {
                let shift = at(target.0, target.1, n)? - zero;
                track(shift);
                line.push(format!("Z{}", fmt(target.2 + shift)));
                report.moves += 1;
            }
            line.extend(others);
            out.push_str(&line.join(" "));
            out.push('\n');
        } else {
            let length = (target.0 - pos.0).hypot(target.1 - pos.1);
            let pieces = ((length / opts.max_segment_mm).ceil().max(1.0)) as usize;
            if pieces > MAX_PIECES {
                return Err(format!("line {n}: the move would be cut in {pieces} pieces (more than {MAX_PIECES}); use a longer piece"));
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
                let mut line = format!("G1 X{} Y{} Z{}", fmt(x), fmt(y), fmt(z + shift));
                if k == 1 && !others.is_empty() {
                    line.push(' ');
                    line.push_str(&others.join(" "));
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
