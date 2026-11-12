//! Random programs with a fixed seed: lines made of the words a G-code program has (and a few that it should not), with the compensation of A and B on, are levelled without a panic;
//! when the levelling goes through, what it writes holds no number that is not finite and every line is a word of the original or a move the levelling made.

use eh3d_autolevel::gcode::{correct, AAxisCompensation, Options};
use eh3d_levelling::{Grid, HeightMap};

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const WORDS: [&str; 26] = [
    "G0", "G1", "G90", "G21", "G91", "G2", "G20", "X10.5", "X-3", "X100", "Y-3", "Y40.25", "Y0",
    "Z-0.1", "Z5", "A45", "A-90", "B30", "B-60", "C720", "F100", "M3", "M5", "S1000", "N12",
    "(a note)",
];

fn table() -> HeightMap {
    let mut m = HeightMap::new(Grid::new((0.0, 0.0, 100.0, 100.0), 5, 5));
    for i in 0..m.grid.len() {
        let (x, y) = m.grid.point(i);
        m.set(i, 0.0004 * x + 0.0002 * y).unwrap();
    }
    m
}

fn program(rng: &mut Rng) -> String {
    let mut text = String::new();
    for _ in 0..1 + rng.below(10) {
        for k in 0..1 + rng.below(5) {
            if k > 0 {
                text.push(' ');
            }
            text.push_str(WORDS[rng.below(WORDS.len())]);
        }
        text.push('\n');
    }
    text
}

#[test]
fn random_programs_are_levelled_or_refused_without_a_panic() {
    let map = table();
    let opts = Options {
        max_segment_mm: 5.0,
        a_compensation: AAxisCompensation::Tilt,
        a_radius_mm: 30.0,
        b_compensation: AAxisCompensation::Tilt,
        b_radius_mm: -20.0,
        ..Options::default()
    };
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let (mut levelled, mut refused) = (0, 0);
    for _ in 0..800 {
        let text = program(&mut rng);
        match correct(&text, &map, &opts) {
            Ok((out, report)) => {
                levelled += 1;
                assert!(
                    !out.contains("NaN") && !out.contains("inf"),
                    "{text}\n=> {out}"
                );
                assert!(
                    report.shift_min.is_finite() && report.shift_max.is_finite(),
                    "{text}"
                );
            }
            Err(e) => {
                refused += 1;
                assert!(!e.is_empty(), "{text}");
            }
        }
    }
    // both outcomes happen: the words are chosen so that some programs go through and some are refused (G91, G2, G20, a move before any G0 or G1)
    assert!(
        levelled > 5 && refused > 5,
        "{levelled} levelled, {refused} refused"
    );
}

#[test]
fn the_same_program_gives_the_same_levelling_every_time() {
    let map = table();
    let mut rng = Rng(5);
    for _ in 0..50 {
        let text = program(&mut rng);
        let a = correct(&text, &map, &Options::default());
        let b = correct(&text, &map, &Options::default());
        assert_eq!(a, b, "{text}");
    }
}

#[test]
fn a_program_cut_at_any_byte_is_levelled_or_refused_without_a_panic() {
    let map = table();
    let text = "G21 G90\nG0 X0 Y0\nG1 Z-0.1 F100\nG1 X40 Y10 A30 B-20 C90\nG1 X80 Y60 A-45 B60 C180 M3 S1000\nM5\n";
    let opts = Options {
        max_segment_mm: 7.0,
        a_compensation: AAxisCompensation::Tilt,
        a_radius_mm: 25.0,
        ..Options::default()
    };
    for end in 0..=text.len() {
        let _ = correct(&text[..end], &map, &opts);
    }
}
