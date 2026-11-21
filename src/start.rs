//! Where the tool stands when the program starts, taken from what a machine reports instead of being assumed to be 0 on every axis. Nothing here talks to a real machine: the place comes
//! from a controller of `eh3d-controller` (the shared simulated GRBL and Marlin are what the tests ask) or from the text of a report that was saved (a status report of GRBL, `<Idle|WPos:...>`,
//! or the answer to `M114` of Marlin), and it is read with the readers of that crate, not with new ones.
//!
//! The place is the one of the WORK coordinates, in mm, as the machine reports it, so its Z already carries the correction of the table: [`crate::gcode::Options::start`] takes it as it comes and the
//! leveller works the Z of the program out of it. A machine that is moving, in alarm or locked, a report with only the position of the machine and no work offset, and a place that is not a
//! number within the limits are refused with the reason.

use eh3d_controller::grbl::parse_status;
use eh3d_controller::marlin::parse_position;
use eh3d_controller::{Controller, State, AXES};

/// The farthest an axis may be from the origin, in mm.
const LIMIT_MM: f64 = 1.0e6;

fn checked(place: [f64; 3]) -> Result<[f64; 3], String> {
    if place.iter().all(|v| v.is_finite() && v.abs() <= LIMIT_MM) {
        Ok(place)
    } else {
        Err(format!(
            "the machine gave a place that is not valid: {place:?} (each axis must be a number within {LIMIT_MM} mm)"
        ))
    }
}

fn steady(state: State) -> Result<(), String> {
    match state {
        State::Idle | State::Hold => Ok(()),
        other => Err(format!(
            "the machine is {other:?}: its place is only taken when it is idle or on hold"
        )),
    }
}

/// The place of the tool, in work coordinates, asked of a controller.
pub fn from_controller<C: Controller + ?Sized>(controller: &mut C) -> Result<[f64; 3], String> {
    let status = controller
        .status()
        .map_err(|e| format!("the machine did not give its place: {e}"))?;
    steady(status.state)?;
    checked([status.work_mm[0], status.work_mm[1], status.work_mm[2]])
}

/// The place of the tool in a saved report: the first line that is a status report of GRBL (with the position of the work, or of the machine with the work offset) or an answer to `M114`.
pub fn from_report(text: &str) -> Result<[f64; 3], String> {
    for line in text.lines() {
        // the offset is not known: a report that needs it and does not carry it gives a number that is not finite, which is refused
        let mut wco = [f64::NAN; AXES];
        if let Some(status) = parse_status(line, &mut wco) {
            steady(status.state)?;
            let w = status.work_mm;
            if !w[..3].iter().all(|v| v.is_finite()) {
                return Err("the report gives the position of the machine and no work offset (WCO), so the place of the work is not known".into());
            }
            return checked([w[0], w[1], w[2]]);
        }
        if let Some(p) = parse_position(line) {
            return checked(p);
        }
    }
    Err("the text has no status report of GRBL and no answer to M114".into())
}

/// The place written on the command line, `X,Y,Z`.
pub fn parse_place(text: &str) -> Result<[f64; 3], String> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 3 {
        return Err(format!("'{text}' must be X,Y,Z"));
    }
    let mut place = [0.0; 3];
    for (slot, part) in place.iter_mut().zip(&parts) {
        *slot = part
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("'{}' is not a number", part.trim()))?;
    }
    checked(place)
}
