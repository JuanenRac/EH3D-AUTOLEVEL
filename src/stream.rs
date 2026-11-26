//! The correction made while the program is sent, line by line, in the place of the correction of the whole file before it: a [`Streamer`] takes the lines of the original program one at a
//! time and gives back the lines to send, with the height of the map for each piece, exactly as [`crate::gcode::correct`] would have written them. It is a model of the emitter of a
//! sending program: it never reads a machine and it sends nothing by itself; [`SimMachine`] is the machine that the lines are tried on.
//!
//! Between the program and the machine it keeps a safe state. The stream STOPS, and nothing more is given out (not even the line that broke the rule), when a line
//! * goes outside the measured area by more than `stream-outside-mm` (the height there is a guess),
//! * would take the tool below `stream-min-z-mm`,
//! * gets a correction of Z beyond `stream-max-shift-mm` (a map that is wrong, or a probe that slipped), or
//! * cannot be levelled at all (an arc when arcs are refused, a code that moves the origin, a program that is levelled already).
//!
//! A stopped stream stays stopped. [`Streamer::safe_lines`] is what a sender should do next: the spindle off and a lift to `stream-safe-z-mm` (plus the correction of the place, and never lower
//! than where the tool stands). Its limits are settings of the file, with conservative defaults.

use crate::gcode::{coord, words, Leveller, Options, Report, MARK};
use crate::surface::{height, Method};
use eh3d_levelling::HeightMap;
use std::fmt;

/// The highest Z that the stream may be asked to lift the tool to when it stops, in mm.
pub const MAX_STREAM_SAFE_Z_MM: f64 = 1000.0;

/// The limits of the streaming emitter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamLimits {
    /// The deepest Z the corrected program may reach, in mm (from -1000 to 0).
    pub min_z_mm: f64,
    /// The largest correction of Z, either way, in mm; 0 for no limit.
    pub max_shift_mm: f64,
    /// How far outside the measured area a move may go, in mm (0: not at all).
    pub outside_mm: f64,
    /// The Z the tool is lifted to when the stream stops, before the correction, in mm.
    pub safe_z_mm: f64,
}

impl Default for StreamLimits {
    fn default() -> Self {
        StreamLimits {
            min_z_mm: -10.0,
            max_shift_mm: 2.0,
            outside_mm: 0.0,
            safe_z_mm: 5.0,
        }
    }
}

/// Why the stream stopped.
#[derive(Debug, Clone, PartialEq)]
pub enum HaltReason {
    /// A move went outside the measured area by more than the margin.
    OutsideMap { distance_mm: f64 },
    /// A corrected Z went below the floor.
    TooDeep { z_mm: f64 },
    /// The correction of Z passed the limit.
    ShiftTooLarge { shift_mm: f64 },
    /// The line cannot be levelled (the reason is the text of the refusal).
    Refused(String),
    /// The program carries the mark of a levelled one.
    AlreadyLevelled,
}

impl HaltReason {
    /// A short word for the reason, for the programs that read the result.
    pub fn code(&self) -> &'static str {
        match self {
            HaltReason::OutsideMap { .. } => "outside-map",
            HaltReason::TooDeep { .. } => "too-deep",
            HaltReason::ShiftTooLarge { .. } => "shift-too-large",
            HaltReason::Refused(_) => "refused",
            HaltReason::AlreadyLevelled => "already-levelled",
        }
    }
}

/// The stop of the stream: at which line of the original program, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Halt {
    pub line: usize,
    pub reason: HaltReason,
}

impl fmt::Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.line;
        match &self.reason {
            HaltReason::OutsideMap { distance_mm } => write!(
                f,
                "line {n}: the move goes {distance_mm:.3} mm outside the measured area, beyond stream-outside-mm"
            ),
            HaltReason::TooDeep { z_mm } => write!(
                f,
                "line {n}: the corrected Z would be {z_mm:.3} mm, below stream-min-z-mm"
            ),
            HaltReason::ShiftTooLarge { shift_mm } => write!(
                f,
                "line {n}: the correction of Z reaches {shift_mm:+.4} mm, beyond stream-max-shift-mm"
            ),
            HaltReason::Refused(why) => write!(f, "{why}"),
            HaltReason::AlreadyLevelled => write!(
                f,
                "line {n}: this program was already levelled by eh3d-autolevel; send the original program"
            ),
        }
    }
}

/// The emitter: the lines of the original program in, the lines to send out.
pub struct Streamer<'a> {
    leveller: Leveller<'a>,
    limits: StreamLimits,
    decimals: usize,
    line: usize,
    sent: usize,
    halted: Option<Halt>,
    /// The Z the tool stands at after the lines that were given out (with the correction), and the correction that went with it. They are kept
    /// here and not asked to the leveller, which has already taken in a line that the limits then refuse.
    standing_z: f64,
    standing_shift: f64,
}

impl<'a> Streamer<'a> {
    /// An emitter for `map`; the options and the limits are checked here.
    pub fn new(
        map: &'a HeightMap,
        opts: &'a Options,
        limits: StreamLimits,
    ) -> Result<Streamer<'a>, String> {
        if !(limits.min_z_mm.is_finite()
            && limits.max_shift_mm.is_finite()
            && limits.outside_mm.is_finite()
            && limits.safe_z_mm.is_finite()
            && limits.max_shift_mm >= 0.0
            && limits.outside_mm >= 0.0
            && (0.0..=MAX_STREAM_SAFE_Z_MM).contains(&limits.safe_z_mm))
        {
            return Err(format!(
                "the limits of the stream are not valid: {limits:?}"
            ));
        }
        let leveller = Leveller::new(map, opts)?;
        let (standing_z, standing_shift) = match opts.start {
            // the place given is where the tool stands, correction included
            Some(start) => (start[2], start[2] - leveller.position().2),
            None => (0.0, 0.0),
        };
        Ok(Streamer {
            leveller,
            limits,
            decimals: opts.decimals,
            line: 0,
            sent: 0,
            halted: None,
            standing_z,
            standing_shift,
        })
    }

    /// Takes the next line of the original program and gives the lines to send for it (none for a line that has nothing to send, such as a blank). When it stops, it says why and gives
    /// nothing; every line after that is refused with the same stop.
    pub fn line(&mut self, raw: &str) -> Result<Vec<String>, Halt> {
        if let Some(h) = &self.halted {
            return Err(h.clone());
        }
        let n = self.line + 1;
        let stop = |me: &mut Self, reason: HaltReason| {
            let halt = Halt { line: n, reason };
            me.halted = Some(halt.clone());
            Err(halt)
        };
        if raw.trim_start().starts_with(MARK) {
            return stop(self, HaltReason::AlreadyLevelled);
        }
        let mut out = String::new();
        if let Err(e) = self.leveller.feed(n, raw, &mut out) {
            return stop(self, HaltReason::Refused(e));
        }
        let report = self.leveller.report();
        let lim = self.limits;
        if report.outside_max_mm > lim.outside_mm {
            return stop(
                self,
                HaltReason::OutsideMap {
                    distance_mm: report.outside_max_mm,
                },
            );
        }
        if lim.max_shift_mm > 0.0
            && (report.shift_max > lim.max_shift_mm || -report.shift_min > lim.max_shift_mm)
        {
            let worst = if report.shift_max.abs() >= report.shift_min.abs() {
                report.shift_max
            } else {
                report.shift_min
            };
            return stop(self, HaltReason::ShiftTooLarge { shift_mm: worst });
        }
        let lines: Vec<String> = out.lines().map(str::to_string).collect();
        let mut last_z = None;
        for l in &lines {
            let z = words(l, n)
                .ok()
                .and_then(|w| w.iter().rev().find(|(c, _)| *c == 'Z').map(|(_, v)| *v));
            if let Some(z) = z.filter(|z| *z < lim.min_z_mm) {
                return stop(self, HaltReason::TooDeep { z_mm: z });
            }
            last_z = z.or(last_z);
        }
        // only now is the line given out: what it says about Z is where the tool will stand
        if let Some(z) = last_z {
            self.standing_z = z;
            self.standing_shift = self.leveller.last_shift();
        }
        self.line = n;
        self.sent += lines.len();
        Ok(lines)
    }

    /// Why the stream stopped, when it did.
    pub fn halted(&self) -> Option<&Halt> {
        self.halted.as_ref()
    }

    /// How many lines were given out.
    pub fn lines_sent(&self) -> usize {
        self.sent
    }

    /// What was counted so far.
    pub fn report(&self) -> Report {
        self.leveller.report()
    }

    /// What a sender should do after a stop: the spindle off, and a lift to the safe Z (with the correction of the place), never lower than where the tool stands.
    /// The place and the correction are those of the last line that was given out: nothing of a refused line has a say in them.
    pub fn safe_lines(&self) -> Vec<String> {
        let z = (self.limits.safe_z_mm + self.standing_shift).max(self.standing_z);
        vec!["M5".to_string(), format!("G0 Z{}", coord(z, self.decimals))]
    }
}

/// A machine that is only a model: it takes the lines that are sent, follows the tool and tells how deep it went below the surface of the table. It is told the surface of the table
/// (a map, which may be another one than the map the program was levelled with) and the reference point where the Z of the job was set.
pub struct SimMachine<'a> {
    table: &'a HeightMap,
    method: Method,
    zero: f64,
    pos: (f64, f64, f64),
    rapid: bool,
    /// The lines taken.
    pub lines: Vec<String>,
    /// The lowest the tool went in relation to the table where it was, in mm (0 before anything is cut; negative: into the material).
    pub deepest_mm: f64,
    /// Whether the spindle is on.
    pub spindle_on: bool,
    /// How many lines were taken that the model does not read (a `$` command, a comment-only line, a message).
    pub unread: usize,
    /// How many rapid moves ended below the surface of the table.
    pub rapid_into_table: usize,
}

impl<'a> SimMachine<'a> {
    /// A machine over the `table`, whose reference point is `reference`.
    pub fn new(table: &'a HeightMap, reference: (f64, f64), method: Method) -> Self {
        let zero = height(table, reference.0, reference.1, method).unwrap_or(0.0);
        SimMachine {
            table,
            method,
            zero,
            pos: (0.0, 0.0, 0.0),
            rapid: true,
            lines: Vec::new(),
            deepest_mm: 0.0,
            spindle_on: false,
            unread: 0,
            rapid_into_table: 0,
        }
    }

    /// Where the tool stands.
    pub fn position(&self) -> (f64, f64, f64) {
        self.pos
    }

    /// The depth of the tool below the surface of the table where it stands, in mm (the Z of the job, which is 0 at the surface; negative: into the material).
    pub fn depth_mm(&self) -> f64 {
        let h = height(self.table, self.pos.0, self.pos.1, self.method).unwrap_or(self.zero);
        self.pos.2 - (h - self.zero)
    }

    /// Takes a line: absolute millimetres, G0 and G1, and the spindle words. A line it does not read (a `$` command, the `%` of a program) is kept and counted in `unread`.
    pub fn send(&mut self, line: &str) -> Result<(), String> {
        self.lines.push(line.to_string());
        let Ok(w) = words(line, self.lines.len()) else {
            self.unread += 1;
            return Ok(());
        };
        let mut moved = false;
        for &(c, v) in &w {
            match (c, v) {
                ('G', 0.0) => self.rapid = true,
                ('G', 1.0) => self.rapid = false,
                ('M', m) if m == 3.0 || m == 4.0 => self.spindle_on = true,
                ('M', m) if m == 5.0 || m == 2.0 || m == 30.0 => self.spindle_on = false,
                ('X', x) => {
                    self.pos.0 = x;
                    moved = true;
                }
                ('Y', y) => {
                    self.pos.1 = y;
                    moved = true;
                }
                ('Z', z) => {
                    self.pos.2 = z;
                    moved = true;
                }
                _ => {}
            }
        }
        if moved {
            let depth = self.depth_mm();
            if self.rapid {
                if depth < -1.0e-6 {
                    self.rapid_into_table += 1;
                }
            } else {
                self.deepest_mm = self.deepest_mm.min(depth);
            }
        }
        Ok(())
    }
}

/// What came of sending a program through the emitter to a machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The lines that were sent, in order.
    pub sent: Vec<String>,
    /// The lines sent after the stop (empty when the stream did not stop).
    pub safe: Vec<String>,
    /// Why it stopped, when it did.
    pub halt: Option<Halt>,
    /// What was counted.
    pub report: Report,
}

/// Sends `program` to `machine` through an emitter, line by line. The first line that cannot be sent stops it: the safe lines go to the machine and the program is not sent further.
pub fn send_program(
    program: &str,
    map: &HeightMap,
    opts: &Options,
    limits: StreamLimits,
    machine: &mut SimMachine<'_>,
) -> Result<Outcome, String> {
    let mut emitter = Streamer::new(map, opts, limits)?;
    let mut sent = Vec::new();
    let mut safe = Vec::new();
    let mut halt = None;
    for raw in program.lines() {
        match emitter.line(raw) {
            Ok(lines) => {
                for l in lines {
                    machine.send(&l)?;
                    sent.push(l);
                }
            }
            Err(h) => {
                safe = emitter.safe_lines();
                for l in &safe {
                    machine.send(l)?;
                }
                halt = Some(h);
                break;
            }
        }
    }
    Ok(Outcome {
        sent,
        safe,
        halt,
        report: emitter.report(),
    })
}
