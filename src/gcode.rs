//! Rewrites a G-code program so that the tool keeps the same depth into the board wherever the table is high or low: every cutting move (G1) is cut in pieces no longer than
//! `max_segment_mm` and each piece gets the Z that the map gives for its end; a rapid move (G0) only has the Z corrected at its end when it has a Z word.
//!
//! What is supported: absolute millimetres (G90, G21) and straight moves (G0, G1), and arcs (G2, G3 in the plane of X and Y, see [`crate::arcs`]) when [`Options::arcs`] is
//! [`ArcMode::Cut`]. What is refused, with the number of the line: relative distances (G91), inches (G20), arcs while they are not cut and a move before any G0 or G1. The program is
//! assumed to start at X0 Y0 Z0 A0 B0 C0, or at [`Options::start`]. Outside the grid the height of the nearest edge is used. [`Leveller`] does the work one line at a time (the streaming
//! emitter uses it) and [`correct`] feeds it a whole program.
//!
//! The rotary axes A, B and C are modal like the others: when a cutting move is cut in pieces their words are interpolated in a straight line between the value they had and the value
//! the move gives, as X and Y are, and written with the same decimals; the height of the table comes from X and Y only. A program with no A, B or C is written as it always was.
//!
//! With [`AAxisCompensation::Tilt`] the turn of the axis A also moves the point that is cut in Z: `r * (1 - cos(A))`, with `r` the distance from the axis to that point
//! ([`Options::a_radius_mm`]) and A in degrees, is added to the Z of every written move, after the height of the table (see `docs/a-axis-compensation.md`). The axis B (a turn about Y)
//! has the same compensation with its own radius ([`Options::b_compensation`], [`Options::b_radius_mm`]) and the two add up. The axis C turns about Z, so it never changes the height of the
//! cut: its words are interpolated and written like the others and nothing is added to Z for them.

use crate::arcs::{Arc, DEFAULT_ARC_TOLERANCE_MM, MAX_ARC_TOLERANCE_MM, MIN_ARC_TOLERANCE_MM};
use crate::surface::{height, Method};
use eh3d_levelling::HeightMap;
use eh3d_parser_gcode::emit::{header, sha256_hex, Header};

/// What the turn of the axis A does to Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AAxisCompensation {
    /// Nothing: Z is the height of the table only (the way it always was).
    #[default]
    Off,
    /// The tilt that the turn of A brings is added to Z: `r * (1 - cos(A))`.
    Tilt,
}

impl AAxisCompensation {
    /// The word of the settings file and of the command line.
    pub fn name(self) -> &'static str {
        match self {
            AAxisCompensation::Off => "off",
            AAxisCompensation::Tilt => "tilt",
        }
    }
}

/// What to do with a move that leaves out an axis the program has not set yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownAxis {
    /// The program is taken to start at 0 on every axis (the way it always was).
    #[default]
    AssumeZero,
    /// The program is refused with the number of the line: nothing says where the axis stands.
    Refuse,
}

impl UnknownAxis {
    /// The word of the settings file and of the command line.
    pub fn name(self) -> &'static str {
        match self {
            UnknownAxis::AssumeZero => "assume-zero",
            UnknownAxis::Refuse => "refuse",
        }
    }
}

/// What to do with an arc (G2, G3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArcMode {
    /// The program is refused with the number of the line (the way it always was).
    #[default]
    Refuse,
    /// The arc is cut in straight pieces that follow the table (see [`crate::arcs`]).
    Cut,
}

impl ArcMode {
    /// The word of the settings file and of the command line.
    pub fn name(self) -> &'static str {
        match self {
            ArcMode::Refuse => "refuse",
            ArcMode::Cut => "cut",
        }
    }
}

/// The largest distance from the axis A to the cut that may be given, in mm (up or down; the sign turns the correction round).
pub const MAX_A_RADIUS_MM: f64 = 1000.0;

/// The correction of Z, in mm, for the axis A standing at `a_deg` degrees when the cut is `radius_mm` from the axis: `radius_mm * (1 - cos(a_deg))`; 0 at A = 0.
pub fn a_axis_shift(radius_mm: f64, a_deg: f64) -> f64 {
    let angle: f64 = a_deg.to_radians();
    radius_mm * (1.0 - angle.cos())
}

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
    /// Whether the turn of the axis A is added to Z.
    pub a_compensation: AAxisCompensation,
    /// The distance from the axis A to the cut, in mm, for [`AAxisCompensation::Tilt`] (not 0 then; from -1000 to 1000).
    pub a_radius_mm: f64,
    /// Whether the turn of the axis B is added to Z (the same compensation as A, with its own radius).
    pub b_compensation: AAxisCompensation,
    /// The distance from the axis B to the cut, in mm, for [`AAxisCompensation::Tilt`] (not 0 then; from -1000 to 1000).
    pub b_radius_mm: f64,
    /// What a move does that leaves out X, Y or Z before the program has set it (`assume-zero` by default).
    pub unknown_axis: UnknownAxis,
    /// What an arc does: refused (by default) or cut in straight pieces.
    pub arcs: ArcMode,
    /// The most an arc may stray from its straight pieces, in mm (from 0.001 to 1).
    pub arc_tolerance_mm: f64,
    /// Where the tool stands when the program starts (X, Y and Z, as the machine reports them, so with the correction of the table in Z); `None` is the way it always was: at 0 on every axis.
    pub start: Option<[f64; 3]>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_segment_mm: 1.0,
            reference: (0.0, 0.0),
            method: Method::Bilinear,
            max_output_lines: DEFAULT_MAX_OUTPUT_LINES,
            decimals: DEFAULT_DECIMALS,
            a_compensation: AAxisCompensation::Off,
            a_radius_mm: 0.0,
            b_compensation: AAxisCompensation::Off,
            b_radius_mm: 0.0,
            unknown_axis: UnknownAxis::AssumeZero,
            arcs: ArcMode::Refuse,
            arc_tolerance_mm: DEFAULT_ARC_TOLERANCE_MM,
            start: None,
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
    /// How many written moves got a correction of Z from the turn of the axis A (0 when the compensation is off or A stayed at 0).
    pub a_corrected: usize,
    /// The largest correction of Z that the axis A gave, with its sign (0 when none).
    pub a_shift_max: f64,
    /// The same for the axis B.
    pub b_corrected: usize,
    pub b_shift_max: f64,
    /// How many arcs were cut in pieces.
    pub arcs_cut: usize,
    /// How many corrected points lie outside the measured area (their height is the one of the nearest edge, which was not measured there).
    pub outside_points: usize,
    /// The largest distance, in mm, from a corrected point to the measured area.
    pub outside_max_mm: f64,
    /// How far, in mm, the reference point lies from the measured area (0 when it is inside).
    pub reference_outside_mm: f64,
}

/// The distance in mm from `(x, y)` to the measured area (0 inside).
fn distance_outside(grid: &eh3d_levelling::Grid, x: f64, y: f64) -> f64 {
    let dx = (grid.x0 - x).max(x - grid.x1).max(0.0);
    let dy = (grid.y0 - y).max(y - grid.y1).max(0.0);
    dx.hypot(dy)
}

/// The warning for a program that goes outside the measured area, or whose reference point is outside it: the height there is the one of the nearest edge, which nobody measured.
/// `None` when everything is inside.
pub fn outside_warning(report: &Report) -> Option<String> {
    let mut parts = Vec::new();
    if report.outside_points > 0 {
        parts.push(format!(
            "{} corrected points lie outside the measured area, up to {:.2} mm from it, and use the height of its nearest edge",
            report.outside_points, report.outside_max_mm
        ));
    }
    if report.reference_outside_mm > 0.0 {
        parts.push(format!(
            "the reference point lies {:.2} mm outside the measured area",
            report.reference_outside_mm
        ));
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// The notice for a program whose Z was corrected for the turn of the axis A: how many moves and the largest correction. `None` when no move was corrected for it.
pub fn a_axis_notice(report: &Report, opts: &Options) -> Option<String> {
    (opts.a_compensation == AAxisCompensation::Tilt && report.a_corrected > 0).then(|| {
        format!(
            "Z corrected for the turn of axis A on {} moves, up to {:+.4} mm (a-axis-radius-mm = {})",
            report.a_corrected, report.a_shift_max, opts.a_radius_mm
        )
    })
}

/// The notice for a program whose Z was corrected for the turn of the axis B: how many moves and the largest correction. `None` when no move was corrected for it.
pub fn b_axis_notice(report: &Report, opts: &Options) -> Option<String> {
    (opts.b_compensation == AAxisCompensation::Tilt && report.b_corrected > 0).then(|| {
        format!(
            "Z corrected for the turn of axis B on {} moves, up to {:+.4} mm (b-axis-radius-mm = {})",
            report.b_corrected, report.b_shift_max, opts.b_radius_mm
        )
    })
}

/// The warning for a program whose Z was moved by more than `limit_mm` (up or down): the largest correction, with its sign, and the limit. `None` when the limit is 0 (no
/// warning), when no move was corrected or when no correction is above the limit.
pub fn shift_warning(report: &Report, limit_mm: f64) -> Option<String> {
    if !(limit_mm.is_finite() && limit_mm > 0.0) || report.moves == 0 {
        return None;
    }
    let worst: f64 = if report.shift_max.abs() >= report.shift_min.abs() {
        report.shift_max
    } else {
        report.shift_min
    };
    (worst.abs() > limit_mm).then(|| {
        format!("the correction of Z reaches {worst:+.4} mm, above warn-shift-mm = {limit_mm}")
    })
}

/// The rotary axes, in the order they are written.
const ROTARY: [char; 3] = ['A', 'B', 'C'];
const MAX_COORDINATE: f64 = 1.0e6;
const MAX_PIECES: usize = 100_000;
/// The default of [`Options::max_output_lines`].
pub const DEFAULT_MAX_OUTPUT_LINES: usize = 5_000_000;
/// The most lines the limit of the output may be set to.
pub const MAX_OUTPUT_LINES_LIMIT: usize = 100_000_000;
/// The default of [`Options::decimals`].
pub const DEFAULT_DECIMALS: usize = 3;
/// The fewest and the most decimals a coordinate may be written with.
pub const MIN_DECIMALS: usize = 1;
pub const MAX_DECIMALS: usize = 6;

/// Whether what follows a `*` is the checksum of a numbered line: digits, and after them nothing but a comment.
fn is_checksum(rest: &str) -> bool {
    let rest = rest.trim_start();
    let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    after.len() < rest.len() && {
        let after = after.trim_start();
        after.is_empty() || after.starts_with(';') || after.starts_with('(')
    }
}

/// The words of a line without its comments: `(...)` and everything after `;`. A bracket inside a comment or a closing one with no comment is refused (a controller ends the
/// comment at the first `)`, so what follows would be words for it), and so is a `*` that is not the checksum of the line.
pub fn words(line: &str, number: usize) -> Result<Vec<(char, f64)>, String> {
    let mut clean = String::new();
    let mut open = false;
    for (at, c) in line.char_indices() {
        match c {
            ';' if !open => break,
            // the checksum of a numbered line (`N10 G1 X5*45`) is not a word
            '*' if !open => {
                if line.get(at + 1..).is_some_and(is_checksum) {
                    break;
                }
                return Err(format!(
                    "line {number}: '*' is not followed by the checksum of the line: what comes after it would be lost"
                ));
            }
            '(' if !open => open = true,
            ')' if open => open = false,
            '(' | ')' => {
                return Err(format!(
                    "line {number}: a bracket inside a comment or without one: a controller ends a comment at the first ')'"
                ))
            }
            _ if !open => clean.push(c),
            _ => {}
        }
    }
    // only the blanks of ASCII count for nothing: another one reaches the controller as a byte it may obey
    let chars: Vec<char> = clean.chars().filter(|c| !c.is_ascii_whitespace()).collect();
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

/// Whether a line is one that is passed on as it is: `$` commands of GRBL (the ones `refuse_hidden_commands` lets through), `#` parameter assignments and the message commands M117 and M118 with their text. Only the lines that move
/// are held to the strict rules of the words; the strictness is about positions, not about every character a controller accepts.
fn is_not_a_move(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with('$') || is_assignment(t) {
        return true;
    }
    let upper = t.to_ascii_uppercase();
    ["M117", "M118"].iter().any(|m| {
        upper
            .strip_prefix(m)
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.'))
    })
}

/// Whether the line is one parameter assignment and nothing else (`#1=5`, `#<depth> = [#1*2] (note)`): a move written after it would pass without its correction.
fn is_assignment(line: &str) -> bool {
    let code = line.split([';', '(']).next().unwrap_or("").trim();
    let Some((name, value)) = code.strip_prefix('#').and_then(|c| c.split_once('=')) else {
        return false;
    };
    let named = |n: &str| {
        let number = !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
        number || (n.len() > 2 && n.starts_with('<') && n.find('>') == Some(n.len() - 1))
    };
    let value = value.trim();
    named(name.trim())
        && (value.parse::<f64>().is_ok()
            || value.strip_prefix('#').is_some_and(named)
            || is_one_expression(value))
}

/// Whether the text is one expression in square brackets, with nothing after it.
fn is_one_expression(text: &str) -> bool {
    let mut depth = 0usize;
    for (at, c) in text.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    return at + 1 == text.len();
                }
            }
            _ if depth == 0 => return false,
            _ => {}
        }
    }
    false
}

/// Refuses a word written twice on a line and two codes of one group: a controller refuses such a line, and rewriting it would keep one of them without saying so.
fn refuse_repeated(words: &[(char, f64)], n: usize) -> Result<(), String> {
    for (i, &(letter, _)) in words.iter().enumerate() {
        if !matches!(letter, 'G' | 'M') && words[..i].iter().any(|&(l, _)| l == letter) {
            return Err(format!("line {n}: the word {letter} is written twice"));
        }
    }
    let count = |letter: char, group: &[f64]| {
        words
            .iter()
            .filter(|&&(l, v)| l == letter && group.contains(&v))
            .count()
    };
    if count('G', &[0.0, 1.0, 2.0, 3.0]) > 1 || count('M', &[3.0, 4.0, 5.0]) > 1 {
        return Err(format!(
            "line {n}: two codes of the same group on one line: a controller refuses it"
        ));
    }
    Ok(())
}

/// The `$` commands of GRBL that only ask: the help, the settings, the offsets, the state of the parser, the build and the startup lines. Every other one moves the machine
/// (`$J=` jogs, `$H` homes) or changes how it behaves (`$X`, `$C`, `$100=...`, `$RST=`...).
const GRBL_QUESTIONS: [&str; 6] = ["$", "$$", "$#", "$G", "$I", "$N"];

/// Refuses what would reach the machine without having been read: a `$` command that is not a question, and a line with a carriage return in the middle, which a controller
/// reads as two lines.
fn refuse_hidden_commands(line: &str, n: usize) -> Result<(), String> {
    let line = line.trim();
    if line.contains('\r') {
        return Err(format!(
            "line {n}: a carriage return in the middle of the line: a controller would read what follows it as another line"
        ));
    }
    // a control character is obeyed by the controller wherever it stands, also in a comment or in the text of a message
    if line.chars().any(|c| c.is_control() && c != '\t') {
        return Err(format!("line {n}: a control character in the line"));
    }
    if !line.starts_with('$') {
        return Ok(());
    }
    // what GRBL reads of a line: no comments, no blanks, any case
    let mut command = String::new();
    let mut depth = 0;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 && !c.is_whitespace() => command.push(c.to_ascii_uppercase()),
            _ => {}
        }
    }
    if GRBL_QUESTIONS.contains(&command.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "line {n}: the command '{command}' of GRBL is refused: it can move the machine ($J= jogs, $H homes) or change it; only the ones that ask pass through ($, $$, $#, $G, $I, $N)"
        ))
    }
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
    let mut leveller = Leveller::new(map, opts)?;
    let mut out = String::new();
    for (i, raw) in text.lines().enumerate() {
        leveller.feed(i + 1, raw, &mut out)?;
    }
    Ok((out, leveller.finish()))
}

/// Levels a program one line at a time: the same work as [`correct`], with what it follows from line to line (the modal move, where the tool stands, which axes were set, where the
/// rotary axes stand) kept between the lines. [`correct`] feeds it a whole program; the streaming emitter feeds it the lines as they are sent.
pub struct Leveller<'a> {
    map: &'a HeightMap,
    opts: &'a Options,
    /// The height of the map at the reference point.
    zero: f64,
    /// The modal move: 0 and 1 are the straight moves, 2 and 3 the arcs.
    mode: Option<u32>,
    pos: (f64, f64, f64),
    /// Which of X, Y and Z the program has set so far.
    known: [bool; 3],
    /// Where A, B and C stand (the program is assumed to start at 0).
    rot: [f64; 3],
    /// Whether the centre of an arc is given as a place (G90.1) and not as an offset (G91.1, the default).
    arc_absolute: bool,
    report: Report,
    /// The points that lie outside the measured area: how many, the largest distance and the distance of the reference point.
    outside: (usize, f64, f64),
    /// The correction of Z of the last piece written.
    last_shift: f64,
}

impl<'a> Leveller<'a> {
    /// A leveller for `map` and `opts`; the options and the map are checked here, so a program is never half levelled for a wrong setting. The tool is taken to be at X0 Y0 Z0, or at
    /// [`Options::start`] when it is given.
    pub fn new(map: &'a HeightMap, opts: &'a Options) -> Result<Leveller<'a>, String> {
        map.require_complete().map_err(|e| e.to_string())?;
        if !(MIN_DECIMALS..=MAX_DECIMALS).contains(&opts.decimals) {
            return Err(format!(
                "decimals is {}: it must be a whole number from {MIN_DECIMALS} to {MAX_DECIMALS}",
                opts.decimals
            ));
        }
        if opts.a_compensation == AAxisCompensation::Tilt
            && !(opts.a_radius_mm.is_finite()
                && opts.a_radius_mm != 0.0
                && opts.a_radius_mm.abs() <= MAX_A_RADIUS_MM)
        {
            return Err(format!(
                "a-axis-radius-mm is {}: with a-axis-compensation = tilt it must be a number other than 0, from -{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}",
                opts.a_radius_mm
            ));
        }
        if opts.b_compensation == AAxisCompensation::Tilt
            && !(opts.b_radius_mm.is_finite()
                && opts.b_radius_mm != 0.0
                && opts.b_radius_mm.abs() <= MAX_A_RADIUS_MM)
        {
            return Err(format!(
                "b-axis-radius-mm is {}: with b-axis-compensation = tilt it must be a number other than 0, from -{MAX_A_RADIUS_MM} to {MAX_A_RADIUS_MM}",
                opts.b_radius_mm
            ));
        }
        if !(opts.max_segment_mm.is_finite() && opts.max_segment_mm > 0.0) {
            return Err(format!(
                "max_segment_mm is {}: it must be more than 0",
                opts.max_segment_mm
            ));
        }
        if !(opts.arc_tolerance_mm.is_finite()
            && (MIN_ARC_TOLERANCE_MM..=MAX_ARC_TOLERANCE_MM).contains(&opts.arc_tolerance_mm))
        {
            return Err(format!(
                "arc-tolerance-mm is {}: it must be a number from {MIN_ARC_TOLERANCE_MM} to {MAX_ARC_TOLERANCE_MM}",
                opts.arc_tolerance_mm
            ));
        }
        let mut me = Leveller {
            map,
            opts,
            zero: 0.0,
            mode: None,
            pos: (0.0, 0.0, 0.0),
            known: [false; 3],
            rot: [0.0; 3],
            arc_absolute: false,
            report: Report {
                shift_min: f64::INFINITY,
                shift_max: f64::NEG_INFINITY,
                ..Report::default()
            },
            outside: (0, 0.0, 0.0),
            last_shift: 0.0,
        };
        me.zero = me.at(opts.reference.0, opts.reference.1, 0)?;
        if let Some(start) = opts.start {
            if !start
                .iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE)
            {
                return Err(format!(
                    "the start position {start:?} is not valid: each axis must be a number within {MAX_COORDINATE} mm"
                ));
            }
            // the place is where the tool stands, which already carries the correction of the table: the program's own Z is what is left without it
            let shift = height(map, start[0], start[1], opts.method)
                .filter(|h| h.is_finite())
                .ok_or_else(|| {
                    format!(
                        "the map has no height at the start position ({:.3}, {:.3})",
                        start[0], start[1]
                    )
                })?
                - me.zero;
            me.pos = (start[0], start[1], start[2] - shift);
            me.known = [true; 3];
        }
        Ok(me)
    }

    /// The height of the map at `(x, y)`; the points outside the measured area are counted (line 0 is the reference point).
    fn at(&mut self, x: f64, y: f64, line: usize) -> Result<f64, String> {
        let away = distance_outside(&self.map.grid, x, y);
        let (count, worst, reference) = self.outside;
        if line == 0 {
            self.outside = (count, worst, away);
        } else if away > 0.0 {
            self.outside = (count + 1, worst.max(away), reference);
        }
        height(self.map, x, y, self.opts.method)
            .filter(|h| h.is_finite())
            .ok_or_else(|| format!("line {line}: the map has no height at ({x:.3}, {y:.3})"))
    }

    /// Where the tool stands after the lines fed so far, in the terms of the program (without the correction).
    pub fn position(&self) -> (f64, f64, f64) {
        self.pos
    }

    /// The correction of Z of the last piece written (0 before any).
    pub fn last_shift(&self) -> f64 {
        self.last_shift
    }

    /// What was counted so far.
    pub fn report(&self) -> Report {
        let mut report = self.report;
        if report.moves == 0 {
            report.shift_min = 0.0;
            report.shift_max = 0.0;
        }
        report.outside_points = self.outside.0;
        report.outside_max_mm = self.outside.1;
        report.reference_outside_mm = self.outside.2;
        report
    }

    /// The report of the whole program.
    pub fn finish(self) -> Report {
        self.report()
    }

    /// Levels the line number `n` (from 1) and appends what it becomes to `out`, each line ending in a line feed. On an error nothing is appended for this line.
    pub fn feed(&mut self, n: usize, raw: &str, out: &mut String) -> Result<(), String> {
        let opts = self.opts;
        let d = opts.decimals;
        let tilt = opts.a_compensation == AAxisCompensation::Tilt;
        let tilt_b = opts.b_compensation == AAxisCompensation::Tilt;
        refuse_hidden_commands(raw, n)?;
        // the '%' that some postprocessors put at the start and at the end of the program is not a word
        if raw.trim() == "%" {
            out.push_str(raw);
            out.push('\n');
            return Ok(());
        }
        // lines that are not moves and have no words to read: a `$` command of GRBL that only asks (`$G`), a parameter assignment (`#1=5`) and the text of a message (`M117 Hello`)
        if is_not_a_move(raw) {
            out.push_str(raw);
            out.push('\n');
            return Ok(());
        }
        let w = words(raw, n)?;
        refuse_repeated(&w, n)?;
        let mut moves_here = false;
        let mut dwell = false;
        // the modal G words that share the line with a move (G90, G54...) go in front of the rewritten move
        let mut kept: Vec<String> = Vec::new();
        let mut mode = self.mode;
        // whether the line itself says how it moves, and whether it cancels the modal move (G80)
        let has_motion = w
            .iter()
            .any(|&(l, v)| l == 'G' && (v == 0.0 || v == 1.0 || v == 2.0 || v == 3.0));
        let mut cancels = false;
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
                        || ((54.0..=59.0).contains(&v) && v.fract() == 0.0)
                        || v == 90.1
                        || v == 91.1 =>
                {
                    cancels |= v == 80.0;
                    if v == 90.1 {
                        self.arc_absolute = true;
                    } else if v == 91.1 {
                        self.arc_absolute = false;
                    }
                    kept.push(format!("G{}", fmt(v)));
                }
                // a dwell takes its time in P (or X on some controllers): the line is not a move
                ('G', 4.0) => dwell = true,
                ('G', 91.0) => {
                    return Err(format!(
                        "line {n}: relative distances (G91) are not supported"
                    ))
                }
                ('G', 20.0) => return Err(format!("line {n}: inches (G20) are not supported")),
                ('G', 2.0 | 3.0) if opts.arcs != ArcMode::Cut => {
                    return Err(format!(
                    "line {n}: arcs (G2, G3) are not supported: they cannot be cut in pieces here (arcs = cut in the settings file cuts them)"
                ))
                }
                ('G', v) if v == 0.0 || v == 1.0 || v == 2.0 || v == 3.0 => mode = Some(v as u32),
                // every other code changes the origin, the offsets or the way the machine moves (G10, G28, G30, G38, G43, G53, G92...):
                // turning it into a plain move would be wrong, and dropping it would change the program
                ('G', v) => {
                    return Err(format!(
                        "line {n}: G{} is not supported (only G0, G1, G2 and G3 with arcs = cut, G4, G17, G21, G40, G49, G54-G59, G61, G64, G80, G90, G90.1, G91.1 and G94 are)",
                        fmt(v)
                    ))
                }
                ('X' | 'Y' | 'Z', _) => moves_here = true,
                _ => {}
            }
        }
        if dwell {
            // a controller dwells and then makes the move of the same line: passing the line on would leave that move without its correction
            // X alone with a G4 is the time on some controllers; with a P the time is there already, and Y or Z are never a time: those words move
            // the machine in the modal move even when the line names none
            let has = |c: char| w.iter().any(|&(l, _)| l == c);
            let axes_move = has('Y') || has('Z') || (has('X') && has('P'));
            if moves_here && (has_motion || axes_move) {
                return Err(format!(
                    "line {n}: a dwell (G4) and a move on the same line are not supported: write the dwell on a line of its own"
                ));
            }
            moves_here = false;
        }
        // after G80 there is no modal move: axis words alone must not become a cutting move
        if cancels && !has_motion {
            mode = None;
        }
        // a full circle has no X or Y: its centre or radius is what says it is a move
        if !dwell
            && matches!(mode, Some(2 | 3))
            && w.iter().any(|(l, _)| matches!(l, 'I' | 'J' | 'R'))
        {
            moves_here = true;
        }
        if !moves_here {
            // a line with only rotary words (`G1 A30`) moves the rotary axes: the next cut starts where they stand
            if !dwell {
                for (k, &c) in ROTARY.iter().enumerate() {
                    if let Some((_, v)) = w.iter().rev().find(|(l, _)| *l == c) {
                        self.rot[k] = *v;
                    }
                }
            }
            self.mode = mode;
            out.push_str(raw);
            out.push('\n');
            return Ok(());
        }
        let m = mode.ok_or_else(|| format!("line {n}: a move before any G0 or G1"))?;
        let pos = self.pos;
        let get = |c: char, default: f64| {
            w.iter()
                .rev()
                .find(|(l, _)| *l == c)
                .map(|(_, v)| *v)
                .unwrap_or(default)
        };
        if opts.unknown_axis == UnknownAxis::Refuse {
            for (k, c) in ['X', 'Y', 'Z'].into_iter().enumerate() {
                if !self.known[k] && !w.iter().any(|(l, _)| *l == c) {
                    return Err(format!(
                        "line {n}: the move leaves out {c} and the program has not set it yet, so its place is not known (unknown-axis = refuse)"
                    ));
                }
            }
        }
        let target = (get('X', pos.0), get('Y', pos.1), get('Z', pos.2));
        for v in [target.0, target.1, target.2] {
            if v.abs() > MAX_COORDINATE {
                return Err(format!(
                    "line {n}: the coordinate {v} is beyond {MAX_COORDINATE} mm"
                ));
            }
        }
        // the arc this line draws (the centre and the turn), when it is one
        let arc: Option<Arc> = if m >= 2 {
            let find = |c: char| w.iter().rev().find(|(l, _)| *l == c).map(|(_, v)| *v);
            if find('P').is_some() {
                return Err(format!(
                    "line {n}: the word P of an arc (several turns) is not supported"
                ));
            }
            Some(
                Arc::from_words(
                    (pos.0, pos.1),
                    (target.0, target.1),
                    (find('I'), find('J')),
                    find('R'),
                    m == 3,
                    self.arc_absolute,
                )
                .map_err(|e| format!("line {n}: {e}"))?,
            )
        } else {
            None
        };
        // the line is accepted from here on: what it sets is remembered
        for (k, c) in ['X', 'Y', 'Z'].into_iter().enumerate() {
            if w.iter().any(|(l, _)| *l == c) {
                self.known[k] = true;
            }
        }
        // the controller does the spindle and coolant words (M3 to M9) of a block BEFORE its motion, and the words that end or pause the program (M0, M1, M2, M30, ...) after it:
        // in a cut move the first go with the first piece and the second with the last piece
        let (m_words, others): (Vec<(char, f64)>, Vec<String>) = {
            let rest: Vec<(char, f64)> = w
                .iter()
                .filter(|(l, _)| !matches!(l, 'G' | 'X' | 'Y' | 'Z'))
                .filter(|(l, _)| arc.is_none() || !matches!(l, 'I' | 'J' | 'K' | 'R' | 'P'))
                .copied()
                .collect();
            let text = |&(l, v): &(char, f64)| format!("{l}{}", fmt(v));
            (
                rest.iter().filter(|(l, _)| *l == 'M').copied().collect(),
                rest.iter().filter(|(l, _)| *l != 'M').map(text).collect(),
            )
        };
        let (m_first, m_last): (Vec<String>, Vec<String>) = {
            let before_motion = |&&(_, v): &&(char, f64)| (3.0..10.0).contains(&v);
            let text = |&(l, v): &(char, f64)| format!("{l}{}", fmt(v));
            (
                m_words.iter().filter(before_motion).map(text).collect(),
                m_words
                    .iter()
                    .filter(|m| !before_motion(m))
                    .map(text)
                    .collect(),
            )
        };
        let has_z = w.iter().any(|(l, _)| *l == 'Z');
        // the rotary axes this line moves: (letter, where it stands, where it goes)
        let rot = self.rot;
        let rotary: Vec<(char, f64, f64)> = ROTARY
            .iter()
            .enumerate()
            .filter_map(|(k, &c)| {
                w.iter()
                    .rev()
                    .find(|(l, _)| *l == c)
                    .map(|(_, v)| (c, rot[k], *v))
            })
            .collect();
        // the correction of Z of the turn of A at the end of a piece (0 when it is off), written down in the report
        let a_from = rotary
            .iter()
            .find(|(c, _, _)| *c == 'A')
            .map(|&(_, f, t)| (f, t));
        let b_from = rotary
            .iter()
            .find(|(c, _, _)| *c == 'B')
            .map(|&(_, f, t)| (f, t));
        let b_shift_of = |b: f64| {
            if tilt_b {
                a_axis_shift(opts.b_radius_mm, b)
            } else {
                0.0
            }
        };
        let a_shift_of = |a: f64| {
            if tilt {
                a_axis_shift(opts.a_radius_mm, a)
            } else {
                0.0
            }
        };
        if m == 0 {
            let mut line = kept.clone();
            line.push("G0".to_string());
            for (l, v) in w.iter().filter(|(l, _)| matches!(l, 'X' | 'Y')) {
                line.push(format!("{l}{}", coord(*v, d)));
            }
            if has_z {
                let a_shift = a_shift_of(a_from.map_or(rot[0], |(_, to)| to));
                let b_shift = b_shift_of(b_from.map_or(rot[1], |(_, to)| to));
                let shift = self.at(target.0, target.1, n)? - self.zero + a_shift + b_shift;
                self.track(shift, a_shift, b_shift);
                line.push(format!("Z{}", coord(target.2 + shift, d)));
                self.report.moves += 1;
            }
            line.extend(others);
            line.extend(m_first);
            line.extend(m_last);
            out.push_str(&line.join(" "));
            out.push('\n');
        } else {
            let pieces = match &arc {
                Some(a) => a.pieces(opts.arc_tolerance_mm, opts.max_segment_mm),
                None => {
                    let length = (target.0 - pos.0).hypot(target.1 - pos.1);
                    ((length / opts.max_segment_mm).ceil().max(1.0)) as usize
                }
            };
            if pieces > MAX_PIECES {
                return Err(format!("line {n}: the move would be cut in {pieces} pieces (more than {MAX_PIECES}); use a longer piece"));
            }
            if n - 1 + self.report.pieces_added + pieces > opts.max_output_lines {
                return Err(format!("line {n}: the corrected program would have more than {} lines; use a longer piece (--segment) or raise the limit (--max-lines)", opts.max_output_lines));
            }
            // a move that is cut in pieces writes the rotary words itself; a move in one piece keeps its words as they are
            let others: Vec<String> = if pieces > 1 && !rotary.is_empty() {
                w.iter()
                    .filter(|(l, _)| !matches!(l, 'G' | 'X' | 'Y' | 'Z' | 'M' | 'A' | 'B' | 'C'))
                    .filter(|(l, _)| arc.is_none() || !matches!(l, 'I' | 'J' | 'K' | 'R' | 'P'))
                    .map(|&(l, v)| format!("{l}{}", fmt(v)))
                    .collect()
            } else {
                others
            };
            for k in 1..=pieces {
                let t = k as f64 / pieces as f64;
                let (x, y) = match &arc {
                    // the last piece ends exactly where the arc says
                    Some(a) if k < pieces => a.point(t),
                    Some(_) => (target.0, target.1),
                    None => (
                        pos.0 + (target.0 - pos.0) * t,
                        pos.1 + (target.1 - pos.1) * t,
                    ),
                };
                let z = pos.2 + (target.2 - pos.2) * t;
                // the piece ends with A where the move puts it (interpolated when it is cut in pieces)
                let a_here = match a_from {
                    Some((_, to)) if k == pieces => to,
                    Some((from, to)) => from + (to - from) * t,
                    None => rot[0],
                };
                let a_shift = a_shift_of(a_here);
                let b_here = match b_from {
                    Some((_, to)) if k == pieces => to,
                    Some((from, to)) => from + (to - from) * t,
                    None => rot[1],
                };
                let b_shift = b_shift_of(b_here);
                let shift = self.at(x, y, n)? - self.zero + a_shift + b_shift;
                self.track(shift, a_shift, b_shift);
                let mut line = format!(
                    "G1 X{} Y{} Z{}",
                    coord(x, d),
                    coord(y, d),
                    coord(z + shift, d)
                );
                if pieces > 1 {
                    for &(c, from, to) in &rotary {
                        // the last piece ends exactly where the move says
                        let v = if k == pieces {
                            to
                        } else {
                            from + (to - from) * t
                        };
                        line.push_str(&format!(" {c}{}", coord(v, d)));
                    }
                }
                if k == 1 && !kept.is_empty() {
                    line = format!("{} {line}", kept.join(" "));
                }
                if k == 1 && !others.is_empty() {
                    line.push(' ');
                    line.push_str(&others.join(" "));
                }
                if k == 1 && !m_first.is_empty() {
                    line.push(' ');
                    line.push_str(&m_first.join(" "));
                }
                if k == pieces && !m_last.is_empty() {
                    line.push(' ');
                    line.push_str(&m_last.join(" "));
                }
                out.push_str(&line);
                out.push('\n');
            }
            self.report.moves += 1;
            self.report.pieces_added += pieces - 1;
            if arc.is_some() {
                self.report.arcs_cut += 1;
            }
        }
        self.pos = target;
        self.mode = mode;
        for &(c, _, to) in &rotary {
            if let Some(k) = ROTARY.iter().position(|&r| r == c) {
                self.rot[k] = to;
            }
        }
        Ok(())
    }

    /// Writes down the correction of one piece in the report.
    fn track(&mut self, shift: f64, a_shift: f64, b_shift: f64) {
        let report = &mut self.report;
        report.shift_min = report.shift_min.min(shift);
        report.shift_max = report.shift_max.max(shift);
        if a_shift != 0.0 {
            report.a_corrected += 1;
            if a_shift.abs() > report.a_shift_max.abs() {
                report.a_shift_max = a_shift;
            }
        }
        if b_shift != 0.0 {
            report.b_corrected += 1;
            if b_shift.abs() > report.b_shift_max.abs() {
                report.b_shift_max = b_shift;
            }
        }
        self.last_shift = shift;
    }
}

/// What `apply --json` prints: one JSON object with what the correction did (the fields of [`Report`]), whether it was a dry run and how many points of the map read
/// above or below their neighbours by more than the limit.
pub fn report_json(report: &Report, dry_run: bool, map_spikes: usize) -> String {
    use crate::surface::json_number as n;
    format!(
        "{{\"dry_run\":{},\"moves\":{},\"pieces_added\":{},\"shift_min\":{},\"shift_max\":{},\"a_corrected\":{},\"a_shift_max\":{},\"b_corrected\":{},\"b_shift_max\":{},\"outside_points\":{},\"outside_max_mm\":{},\"reference_outside_mm\":{},\"map_spikes\":{}}}",
        dry_run,
        report.moves,
        report.pieces_added,
        n(report.shift_min),
        n(report.shift_max),
        report.a_corrected,
        n(report.a_shift_max),
        report.b_corrected,
        n(report.b_shift_max),
        report.outside_points,
        n(report.outside_max_mm),
        n(report.reference_outside_mm),
        map_spikes
    )
}
