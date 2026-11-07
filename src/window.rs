//! The state of the window without a screen: the program and the height map that are open, the settings, the levelled program, the paths and colours to draw, the files to write and the
//! status line. The window (`gui`) only draws this and passes on what the person does, so everything that can be decided is decided here, in code that is tested. The program is levelled
//! with `gcode::correct` and written with `gcode::mark_with_header`, the same functions as the command line.

use crate::gcode::{correct, mark_with_header, words, Options, Report, DEFAULT_MAX_OUTPUT_LINES};
use crate::lang::{fill, Lang, Msg};
use crate::surface::{height, Method};
use crate::viewport::Viewport;
use eh3d_levelling::{simulated_table, Grid, HeightMap};
use std::path::{Path, PathBuf};

/// The largest file the window reads.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// The most segments of a path that are kept to be drawn.
pub const MAX_SEGMENTS: usize = 300_000;

/// What a file chosen in the window is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    OpenProgram,
    OpenMap,
    ExportProgram,
    ExportMap,
}

impl Job {
    pub fn is_export(self) -> bool {
        matches!(self, Job::ExportProgram | Job::ExportMap)
    }

    /// The extensions the list of files shows.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Job::OpenProgram | Job::ExportProgram => {
                &["nc", "ngc", "gcode", "gc", "tap", "cnc", "txt"]
            }
            Job::OpenMap | Job::ExportMap => &["txt"],
        }
    }

    pub fn title(self) -> Msg {
        match self {
            Job::OpenProgram => Msg::TitleOpenProgram,
            Job::OpenMap => Msg::TitleOpenMap,
            Job::ExportProgram => Msg::TitleExportProgram,
            Job::ExportMap => Msg::TitleExportMap,
        }
    }
}

/// A setting that is typed in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Segment,
    RefX,
    RefY,
    MaxLines,
    SimX0,
    SimY0,
    SimX1,
    SimY1,
    SimCols,
    SimRows,
}

/// The group of the side panel a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Parameters,
    Simulation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: Key,
    pub label: Msg,
    pub section: Section,
    pub text: String,
}

fn default_fields() -> Vec<Field> {
    let f = |key, label, section, text: &str| Field {
        key,
        label,
        section,
        text: text.to_string(),
    };
    vec![
        f(Key::Segment, Msg::Segment, Section::Parameters, "1"),
        f(Key::RefX, Msg::RefX, Section::Parameters, "0"),
        f(Key::RefY, Msg::RefY, Section::Parameters, "0"),
        f(
            Key::MaxLines,
            Msg::MaxLines,
            Section::Parameters,
            &DEFAULT_MAX_OUTPUT_LINES.to_string(),
        ),
        f(Key::SimX0, Msg::SimX0, Section::Simulation, "0"),
        f(Key::SimY0, Msg::SimY0, Section::Simulation, "0"),
        f(Key::SimX1, Msg::SimX1, Section::Simulation, "200"),
        f(Key::SimY1, Msg::SimY1, Section::Simulation, "150"),
        f(Key::SimCols, Msg::SimCols, Section::Simulation, "9"),
        f(Key::SimRows, Msg::SimRows, Section::Simulation, "7"),
    ]
}

/// A straight piece of a path, seen from above.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// A cutting move (G1); a rapid move (G0) is drawn thin.
    pub cut: bool,
    pub rgb: [u8; 3],
}

/// The pieces of a program to draw, as seen from above, and whether the list was cut short at [`MAX_SEGMENTS`]. Only the moves of X and Y count; a line that cannot be read is left
/// out (the picture is not the place that refuses a program), and the reading stops at relative distances because the picture would be wrong from there on.
pub fn toolpath(text: &str) -> (Vec<Seg>, bool) {
    let mut segs = Vec::new();
    let mut mode: Option<bool> = None;
    let (mut x, mut y) = (0.0_f64, 0.0_f64);
    for (i, raw) in text.lines().enumerate() {
        let Ok(w) = words(raw, i + 1) else {
            continue;
        };
        let mut moved = false;
        let (mut nx, mut ny) = (x, y);
        for &(letter, value) in &w {
            match letter {
                'G' if value == 0.0 => mode = Some(false),
                'G' if value == 1.0 || value == 2.0 || value == 3.0 => mode = Some(true),
                'G' if value == 91.0 => return (segs, false),
                'X' => {
                    nx = value;
                    moved = true;
                }
                'Y' => {
                    ny = value;
                    moved = true;
                }
                _ => {}
            }
        }
        if !moved {
            continue;
        }
        if let Some(cut) = mode {
            if (nx - x).hypot(ny - y) > 0.0 {
                if segs.len() >= MAX_SEGMENTS {
                    return (segs, true);
                }
                segs.push(Seg {
                    x0: x,
                    y0: y,
                    x1: nx,
                    y1: ny,
                    cut,
                    rgb: [0, 0, 0],
                });
            }
        }
        x = nx;
        y = ny;
    }
    (segs, false)
}

/// A colour for a place between the lowest (0) and the highest (1): blue, cyan, green, yellow, red.
pub fn heat(t: f64) -> [u8; 3] {
    const STOPS: [[f64; 3]; 5] = [
        [0.10, 0.20, 0.80],
        [0.10, 0.75, 0.85],
        [0.35, 0.80, 0.30],
        [0.95, 0.85, 0.20],
        [0.90, 0.20, 0.15],
    ];
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let pos = t * 4.0;
    let i = (pos.floor() as usize).min(3);
    let f = pos - i as f64;
    std::array::from_fn(|k| {
        let v = STOPS[i][k] + (STOPS[i + 1][k] - STOPS[i][k]) * f;
        (v * 255.0).round() as u8
    })
}

/// Where a height falls between the lowest and the highest of the table (0.5 when the table is flat).
pub fn normalise(z: f64, lo: f64, hi: f64) -> f64 {
    if hi - lo < 1e-12 {
        0.5
    } else {
        ((z - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

/// The colour of every point of the map, in the order of the points; `None` for a point that was not measured.
pub fn map_colours(map: &HeightMap) -> Vec<Option<[u8; 3]>> {
    let (lo, hi) = map.spread().map_or((0.0, 0.0), |(lo, hi, _)| (lo, hi));
    (0..map.grid.len())
        .map(|i| map.get(i).map(|z| heat(normalise(z, lo, hi))))
        .collect()
}

/// Paints the pieces with the colour of the table under their middle.
fn colour_by_table(segs: &mut [Seg], map: &HeightMap, method: Method) {
    let (lo, hi) = map.spread().map_or((0.0, 0.0), |(lo, hi, _)| (lo, hi));
    for s in segs {
        let z = height(map, (s.x0 + s.x1) / 2.0, (s.y0 + s.y1) / 2.0, method).unwrap_or(lo);
        s.rgb = heat(normalise(z, lo, hi));
    }
}

/// The text of a file when it is not larger than `limit` bytes: the command line reads with it, so that a file by mistake does not fill the memory.
pub fn read_limited(path: &Path, limit: u64) -> Result<String, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > limit {
        return Err(format!("{shown}: the file is larger than {limit} bytes"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{shown}: {e}"))
}

/// The text of a file, if it is not too big.
fn read_text(path: &Path) -> Result<String, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{shown}: the file is larger than the window reads"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{shown}: {e}"))
}

/// Whether the text is a height map (the first line of the file of the probe is `levelling 1`) and not a program.
pub fn looks_like_map(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .is_some_and(|l| l.starts_with("levelling"))
}

/// Written beside and renamed, so that a file is never left half written (the command line does the same).
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

/// True when two paths name the same file (compared once resolved; by their text when one does not exist yet).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The program that is open.
#[derive(Debug, Clone)]
pub struct Program {
    pub path: PathBuf,
    pub text: String,
    pub lines: usize,
}

/// The height map that is open: read from a file, or simulated (no path).
#[derive(Debug, Clone)]
pub struct MapFile {
    pub path: Option<PathBuf>,
    pub map: HeightMap,
}

/// The levelled program and what it was made with.
#[derive(Debug, Clone)]
pub struct Done {
    pub text: String,
    pub report: Report,
    pub options: Options,
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    ProgramOpened {
        name: String,
        lines: usize,
    },
    MapOpened {
        name: String,
        measured: usize,
        total: usize,
    },
    Simulated {
        points: usize,
    },
    Computed(Report),
    Written(String),
    NeedProgram,
    NeedMap,
    BadValue(Msg),
    SameFile(String),
    Failed(String),
}

impl Status {
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::NeedProgram
                | Status::NeedMap
                | Status::BadValue(_)
                | Status::SameFile(_)
                | Status::Failed(_)
        )
    }

    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::ProgramOpened { name, lines } => {
                fill(Msg::ProgramOpened.tr(lang), &[name, &lines.to_string()])
            }
            Status::MapOpened {
                name,
                measured,
                total,
            } => fill(
                Msg::MapOpened.tr(lang),
                &[name, &measured.to_string(), &total.to_string()],
            ),
            Status::Simulated { points } => fill(Msg::Simulated.tr(lang), &[&points.to_string()]),
            Status::Computed(r) => fill(
                Msg::Computed.tr(lang),
                &[
                    &r.moves.to_string(),
                    &r.pieces_added.to_string(),
                    &format!("{:+.4}", r.shift_min),
                    &format!("{:+.4}", r.shift_max),
                ],
            ),
            Status::Written(name) => fill(Msg::Written.tr(lang), &[name]),
            Status::NeedProgram => Msg::NeedProgram.tr(lang).to_string(),
            Status::NeedMap => Msg::NeedMap.tr(lang).to_string(),
            Status::BadValue(label) => fill(Msg::BadValue.tr(lang), &[label.tr(lang)]),
            Status::SameFile(name) => fill(Msg::SameFile.tr(lang), &[name]),
            Status::Failed(e) => fill(Msg::Failed.tr(lang), &[e]),
        }
    }
}

/// The state of the whole window.
#[derive(Debug)]
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    pub method: Method,
    pub program: Option<Program>,
    pub map: Option<MapFile>,
    pub map_colours: Vec<Option<[u8; 3]>>,
    pub original: Vec<Seg>,
    pub corrected: Vec<Seg>,
    /// The pictures were cut short at [`MAX_SEGMENTS`].
    pub truncated: bool,
    pub done: Option<Done>,
    /// A setting changed after the program was levelled.
    pub stale: bool,
    pub status: Status,
    pub viewport: Viewport,
    pub show_map: bool,
    pub show_original: bool,
    pub show_corrected: bool,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        Window {
            lang,
            fields: default_fields(),
            method: Method::Bilinear,
            program: None,
            map: None,
            map_colours: Vec::new(),
            original: Vec::new(),
            corrected: Vec::new(),
            truncated: false,
            done: None,
            stale: false,
            status: Status::Ready,
            viewport: Viewport::default(),
            show_map: true,
            show_original: true,
            show_corrected: true,
        }
    }

    pub fn has_work(&self) -> bool {
        self.program.is_some() || self.map.is_some()
    }

    fn mark_stale(&mut self) {
        if self.done.is_some() {
            self.stale = true;
        }
    }

    /// Writes a setting that the person typed.
    pub fn set_field(&mut self, i: usize, text: String) {
        if let Some(f) = self.fields.get_mut(i) {
            if f.text != text {
                f.text = text;
                if f.section == Section::Parameters {
                    self.mark_stale();
                }
            }
        }
    }

    pub fn set_method(&mut self, method: Method) {
        if self.method != method {
            self.method = method;
            self.mark_stale();
        }
    }

    fn field(&self, key: Key) -> &Field {
        // every key has its field: `default_fields` makes one for each
        self.fields
            .iter()
            .find(|f| f.key == key)
            .unwrap_or(&self.fields[0])
    }

    fn number(&self, key: Key) -> Result<f64, Status> {
        let f = self.field(key);
        f.text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(Status::BadValue(f.label))
    }

    fn whole(&self, key: Key, lo: f64, hi: f64) -> Result<usize, Status> {
        let v = self.number(key)?;
        if v.fract() == 0.0 && (lo..=hi).contains(&v) {
            Ok(v as usize)
        } else {
            Err(Status::BadValue(self.field(key).label))
        }
    }

    /// The options of `correct` from the settings, checked the way the command line checks them.
    pub fn options(&self) -> Result<Options, Status> {
        Ok(Options {
            max_segment_mm: self.number(Key::Segment)?,
            reference: (self.number(Key::RefX)?, self.number(Key::RefY)?),
            method: self.method,
            max_output_lines: self.whole(Key::MaxLines, 1.0, 100_000_000.0)?,
            ..Options::default()
        })
    }

    /// The name of the program that is open.
    pub fn program_name(&self) -> Option<String> {
        self.program.as_ref().map(|p| name_of(&p.path))
    }

    /// The name of the map that is open.
    pub fn map_name(&self) -> Option<String> {
        self.map.as_ref().map(|m| match &m.path {
            Some(p) => name_of(p),
            None => Msg::SimulatedName.tr(self.lang).to_string(),
        })
    }

    /// The name of the file that is open for a job of opening.
    pub fn open_name(&self, job: Job) -> Option<String> {
        match job {
            Job::OpenProgram => self.program_name(),
            Job::OpenMap => self.map_name(),
            _ => None,
        }
    }

    /// The folder of the files that are open, where the window of files starts.
    pub fn work_folder(&self) -> Option<PathBuf> {
        let from_program = self.program.as_ref().map(|p| p.path.clone());
        let from_map = self.map.as_ref().and_then(|m| m.path.clone());
        from_program
            .or(from_map)
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// The name offered for a file to write.
    pub fn suggested_output_for(&self, job: Job) -> Option<PathBuf> {
        match job {
            Job::ExportProgram => {
                let p = &self.program.as_ref()?.path;
                let stem = p.file_stem()?.to_string_lossy().into_owned();
                let ext = p
                    .extension()
                    .map_or_else(|| "nc".to_string(), |e| e.to_string_lossy().into_owned());
                Some(p.with_file_name(format!("{stem}_levelled.{ext}")))
            }
            Job::ExportMap => Some(PathBuf::from("map.txt")),
            _ => None,
        }
    }

    fn set_program(&mut self, path: &Path, text: String) {
        let (segs, cut) = toolpath(&text);
        self.original = segs;
        self.corrected.clear();
        self.truncated = cut;
        self.done = None;
        self.stale = false;
        let lines = text.lines().count();
        self.status = Status::ProgramOpened {
            name: name_of(path),
            lines,
        };
        self.program = Some(Program {
            path: path.to_path_buf(),
            text,
            lines,
        });
    }

    fn set_map(&mut self, path: Option<&Path>, map: HeightMap, status: Status) {
        self.map_colours = map_colours(&map);
        self.map = Some(MapFile {
            path: path.map(Path::to_path_buf),
            map,
        });
        // the program that was levelled with another map is not valid any more
        self.done = None;
        self.corrected.clear();
        self.stale = false;
        self.status = status;
    }

    /// Opens a G-code program.
    pub fn open_program(&mut self, path: &Path) -> bool {
        match read_text(path) {
            Ok(text) => {
                self.set_program(path, text);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    fn load_map(&mut self, path: &Path, text: &str) -> bool {
        match HeightMap::from_text(text) {
            Ok(map) => {
                let status = Status::MapOpened {
                    name: name_of(path),
                    measured: map.measured(),
                    total: map.grid.len(),
                };
                self.set_map(Some(path), map, status);
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", path.display()));
                false
            }
        }
    }

    /// Opens a height map.
    pub fn open_map(&mut self, path: &Path) -> bool {
        match read_text(path) {
            Ok(text) => self.load_map(path, &text),
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Opens a file dropped on the window: a map when it starts as one, a program otherwise.
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        match read_text(path) {
            Ok(text) if looks_like_map(&text) => self.load_map(path, &text),
            Ok(text) => {
                self.set_program(path, text);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Makes the map of a simulated table (a tilted one with a bump) over the area and the number of points of the settings, like `demo-map`.
    pub fn simulate(&mut self) -> bool {
        match self.simulated_map() {
            Ok(map) => {
                let points = map.grid.len();
                self.set_map(None, map, Status::Simulated { points });
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    fn simulated_map(&self) -> Result<HeightMap, Status> {
        let area = (
            self.number(Key::SimX0)?,
            self.number(Key::SimY0)?,
            self.number(Key::SimX1)?,
            self.number(Key::SimY1)?,
        );
        if area.0 == area.2 {
            return Err(Status::BadValue(Msg::SimX1));
        }
        if area.1 == area.3 {
            return Err(Status::BadValue(Msg::SimY1));
        }
        let cols = self.whole(Key::SimCols, 2.0, 200.0)?;
        let rows = self.whole(Key::SimRows, 2.0, 200.0)?;
        let mut map = HeightMap::new(Grid::new(area, cols, rows));
        for i in 0..map.grid.len() {
            let (x, y) = map.grid.point(i);
            map.set(i, simulated_table(x, y))
                .map_err(|e| Status::Failed(e.to_string()))?;
        }
        Ok(map)
    }

    /// Levels the program with the map.
    pub fn compute(&mut self) -> bool {
        let Some(program) = &self.program else {
            self.status = Status::NeedProgram;
            return false;
        };
        let Some(file) = &self.map else {
            self.status = Status::NeedMap;
            return false;
        };
        let options = match self.options() {
            Ok(o) => o,
            Err(s) => {
                self.status = s;
                return false;
            }
        };
        match correct(&program.text, &file.map, &options) {
            Ok((text, report)) => {
                let (mut segs, cut) = toolpath(&text);
                colour_by_table(&mut segs, &file.map, options.method);
                self.corrected = segs;
                self.truncated = self.truncated || cut;
                self.done = Some(Done {
                    text,
                    report,
                    options,
                });
                self.stale = false;
                self.status = Status::Computed(report);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Writes the levelled program with the mark that says it was levelled (the program is levelled again first when a setting changed). It is never written over the map.
    pub fn export_program(&mut self, path: &Path) -> bool {
        if (self.done.is_none() || self.stale) && !self.compute() {
            return false;
        }
        let (Some(done), Some(file)) = (&self.done, &self.map) else {
            self.status = Status::NeedMap;
            return false;
        };
        if file.path.as_deref().is_some_and(|m| same_file(m, path)) {
            self.status = Status::SameFile(path.display().to_string());
            return false;
        }
        let source = self
            .program
            .as_ref()
            .map(|p| (name_of(&p.path), p.text.as_str()));
        let text = mark_with_header(
            &done.text,
            &file.map,
            &done.options,
            source.as_ref().map(|(n, t)| (n.as_str(), *t)),
        );
        match write_atomic(path, &text) {
            Ok(()) => {
                self.status = Status::Written(name_of(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Writes the height map as the probe of the suite writes it. It is never written over the program.
    pub fn export_map(&mut self, path: &Path) -> bool {
        let Some(file) = &self.map else {
            self.status = Status::NeedMap;
            return false;
        };
        if self
            .program
            .as_ref()
            .is_some_and(|p| same_file(&p.path, path))
        {
            self.status = Status::SameFile(path.display().to_string());
            return false;
        }
        match write_atomic(path, &file.map.to_text()) {
            Ok(()) => {
                self.status = Status::Written(name_of(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Does what a chosen file is for.
    pub fn run_job(&mut self, job: Job, path: &Path) -> bool {
        match job {
            Job::OpenProgram => self.open_program(path),
            Job::OpenMap => self.open_map(path),
            Job::ExportProgram => self.export_program(path),
            Job::ExportMap => self.export_map(path),
        }
    }

    /// The part of the table that the picture has to show: the map and the paths.
    fn bounds(&self) -> Option<((f64, f64), (f64, f64))> {
        let mut min = (f64::INFINITY, f64::INFINITY);
        let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut grow = |x: f64, y: f64| {
            min = (min.0.min(x), min.1.min(y));
            max = (max.0.max(x), max.1.max(y));
        };
        if let Some(f) = &self.map {
            let g = &f.map.grid;
            grow(g.x0, g.y0);
            grow(g.x1, g.y1);
        }
        for s in self.original.iter().chain(&self.corrected) {
            grow(s.x0, s.y0);
            grow(s.x1, s.y1);
        }
        (min.0 <= max.0 && min.1 <= max.1).then_some((min, max))
    }

    /// Shows everything in a picture of the size given.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((min, max)) = self.bounds() {
            self.viewport = Viewport::fit(min, max, width_px, height_px, 24.0);
        }
    }

    /// The height of the table at a place, with the surface that is chosen.
    pub fn table_height(&self, x: f64, y: f64) -> Option<f64> {
        let f = self.map.as_ref()?;
        height(&f.map, x, y, self.method)
    }

    /// What the pointer is over: the place and, when there is a map, the height of the table.
    pub fn cursor_text(&self, x: f64, y: f64) -> String {
        let (xs, ys) = (format!("{x:.2}"), format!("{y:.2}"));
        match self.table_height(x, y) {
            Some(z) => fill(
                Msg::CursorTable.tr(self.lang),
                &[&xs, &ys, &format!("{z:+.4}")],
            ),
            None => fill(Msg::CursorPlain.tr(self.lang), &[&xs, &ys]),
        }
    }

    /// The facts about the map: how many points were measured, the spread and the tilt (what the `info` command prints).
    pub fn info_lines(&self) -> Vec<String> {
        let Some(f) = &self.map else {
            return Vec::new();
        };
        let lang = self.lang;
        let mut lines = vec![fill(
            Msg::InfoPoints.tr(lang),
            &[&f.map.measured().to_string(), &f.map.grid.len().to_string()],
        )];
        if let Some((lo, hi, spread)) = f.map.spread() {
            lines.push(fill(
                Msg::InfoSpread.tr(lang),
                &[
                    &format!("{lo:+.4}"),
                    &format!("{hi:+.4}"),
                    &format!("{spread:.4}"),
                ],
            ));
        }
        if let Some((tx, ty)) = f.map.tilt_mm_per_m() {
            lines.push(fill(
                Msg::InfoTilt.tr(lang),
                &[&format!("{tx:+.3}"), &format!("{ty:+.3}")],
            ));
        }
        lines
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PROGRAM: &str = "(a track)\nG90 G21\nG0 Z2\nG0 X10 Y10\nG1 Z-0.1 F60\nG1 X50 Y10 F200\nG1 X50 Y40\nG0 Z2\nM2\n";

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!(
                "eh3d-autolevel-window-{}-{tag}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::write(&p, text).unwrap();
            p
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn ready(tag: &str) -> (Scratch, Window) {
        let s = Scratch::new(tag);
        let p = s.write("job.nc", PROGRAM);
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert!(w.simulate());
        (s, w)
    }

    fn set(w: &mut Window, key: Key, text: &str) {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    }

    #[test]
    fn the_path_of_a_program_is_read_move_by_move() {
        let (segs, cut) = toolpath(PROGRAM);
        assert!(!cut);
        // G0 to (10, 10), then two cutting moves
        assert_eq!(segs.len(), 3);
        assert!(!segs[0].cut && segs[1].cut && segs[2].cut);
        assert_eq!(
            (segs[0].x0, segs[0].y0, segs[0].x1, segs[0].y1),
            (0.0, 0.0, 10.0, 10.0)
        );
        assert_eq!((segs[2].x1, segs[2].y1), (50.0, 40.0));
    }

    #[test]
    fn lines_that_cannot_be_read_and_moves_of_z_alone_draw_nothing() {
        let (segs, _) = toolpath("G0 Z5\n@@@\nG1 X1 Y1\nM117 hello\nG1 Z-1\n");
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn a_move_before_any_g0_or_g1_is_not_drawn_and_relative_distances_stop_the_reading() {
        let (segs, _) = toolpath("X5 Y5\nG1 X10 Y0\nG91\nG1 X5\n");
        assert_eq!(segs.len(), 1);
        assert_eq!((segs[0].x0, segs[0].x1), (5.0, 10.0));
    }

    #[test]
    fn a_huge_program_is_cut_short_for_the_picture() {
        let mut text = String::from("G1 X0 Y0\n");
        for k in 1..=(MAX_SEGMENTS + 5) {
            text.push_str(&format!("G1 X{k} Y0\n"));
        }
        let (segs, cut) = toolpath(&text);
        assert!(cut);
        assert_eq!(segs.len(), MAX_SEGMENTS);
    }

    #[test]
    fn the_colours_go_from_blue_to_red() {
        let (low, high) = (heat(0.0), heat(1.0));
        assert!(low[2] > low[0] && low[2] > low[1], "the lowest is blue");
        assert!(high[0] > high[1] && high[0] > high[2], "the highest is red");
        assert_eq!(heat(f64::NAN), heat(0.0));
        assert_eq!(heat(-3.0), heat(0.0));
        assert_eq!(heat(9.0), heat(1.0));
    }

    #[test]
    fn a_flat_table_is_painted_in_the_middle_of_the_colours() {
        assert_eq!(normalise(0.05, 0.05, 0.05), 0.5);
        assert_eq!(normalise(1.0, 0.0, 2.0), 0.5);
        assert_eq!(normalise(-1.0, 0.0, 2.0), 0.0);
    }

    #[test]
    fn every_point_of_the_map_has_a_colour_unless_it_was_not_measured() {
        let mut m = HeightMap::new(Grid::new((0.0, 0.0, 10.0, 10.0), 2, 2));
        m.set(0, 0.0).unwrap();
        m.set(1, 0.1).unwrap();
        let c = map_colours(&m);
        assert_eq!(c.len(), 4);
        assert!(c[0].is_some() && c[1].is_some());
        assert!(c[2].is_none() && c[3].is_none());
        assert_ne!(c[0], c[1]);
    }

    #[test]
    fn a_map_is_told_from_a_program_by_its_first_line() {
        assert!(looks_like_map("levelling 1\ngrid 0 0 1 1 2 2\n"));
        assert!(looks_like_map("\n  levelling 1\n"));
        assert!(!looks_like_map(PROGRAM));
        assert!(!looks_like_map(""));
    }

    #[test]
    fn opening_a_program_keeps_its_text_and_draws_its_path() {
        let s = Scratch::new("open");
        let p = s.write("job.nc", PROGRAM);
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert_eq!(w.program_name().as_deref(), Some("job.nc"));
        assert_eq!(w.program.as_ref().unwrap().lines, 9);
        assert_eq!(w.original.len(), 3);
        assert!(w.has_work());
        assert!(w.status.text(Lang::En).contains("job.nc: 9 lines"));
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error_with_its_name() {
        let mut w = Window::new(Lang::En);
        assert!(!w.open_program(Path::new("/no/such/folder/job.nc")));
        assert!(w.status.is_problem());
        assert!(w.status.text(Lang::En).contains("job.nc"));
        assert!(!w.open_map(Path::new("/no/such/folder/map.txt")));
        assert!(w.program.is_none() && w.map.is_none());
    }

    #[test]
    fn a_map_file_that_is_damaged_is_refused() {
        let s = Scratch::new("badmap");
        let p = s.write("map.txt", "levelling 1\ngrid nonsense\n");
        let mut w = Window::new(Lang::En);
        assert!(!w.open_map(&p));
        assert!(w.map.is_none());
        assert!(w.status.is_problem());
    }

    #[test]
    fn a_dropped_file_is_a_map_or_a_program_by_what_it_holds() {
        let (s, mut w) = ready("drop");
        assert!(w.export_map(&s.0.join("saved.txt")));
        let mut other = Window::new(Lang::En);
        assert!(other.open_dropped(&s.0.join("saved.txt")));
        assert!(other.map.is_some() && other.program.is_none());
        assert!(other.open_dropped(&s.0.join("job.nc")));
        assert!(other.program.is_some());
    }

    #[test]
    fn the_simulated_table_has_the_points_asked_for() {
        let mut w = Window::new(Lang::En);
        assert!(w.simulate());
        assert_eq!(w.map.as_ref().unwrap().map.grid.len(), 63);
        assert!(w.map.as_ref().unwrap().map.is_complete());
        assert_eq!(w.map_colours.len(), 63);
        assert_eq!(w.map_name().as_deref(), Some("(simulated table)"));
        assert_eq!(w.info_lines().len(), 3);
    }

    #[test]
    fn a_simulation_with_nonsense_settings_says_which_one() {
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::SimCols, "1");
        assert!(!w.simulate());
        assert_eq!(w.status, Status::BadValue(Msg::SimCols));
        set(&mut w, Key::SimCols, "9");
        set(&mut w, Key::SimRows, "2.5");
        assert!(!w.simulate());
        assert_eq!(w.status, Status::BadValue(Msg::SimRows));
        set(&mut w, Key::SimRows, "7");
        set(&mut w, Key::SimX1, "0");
        assert!(!w.simulate());
        assert_eq!(w.status, Status::BadValue(Msg::SimX1));
        set(&mut w, Key::SimX1, "abc");
        assert!(!w.simulate());
        assert!(w.map.is_none());
    }

    #[test]
    fn levelling_needs_a_program_and_a_map() {
        let mut w = Window::new(Lang::En);
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedProgram);
        let s = Scratch::new("need");
        assert!(w.open_program(&s.write("job.nc", PROGRAM)));
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedMap);
    }

    #[test]
    fn the_window_levels_like_the_command_line() {
        let (_s, mut w) = ready("same");
        assert!(w.compute());
        let done = w.done.as_ref().unwrap();
        let expected = correct(PROGRAM, &w.map.as_ref().unwrap().map, &Options::default()).unwrap();
        assert_eq!(done.text, expected.0);
        assert_eq!(done.report, expected.1);
        assert!(w.corrected.len() > w.original.len());
        assert!(!w.stale);
    }

    #[test]
    fn a_setting_that_changes_after_levelling_makes_the_result_stale() {
        let (_s, mut w) = ready("stale");
        assert!(w.compute());
        set(&mut w, Key::Segment, "2");
        assert!(w.stale);
        assert!(w.compute());
        assert!(!w.stale);
        w.set_method(Method::Bicubic);
        assert!(w.stale);
        // the settings of the simulation do not matter to the result
        assert!(w.compute());
        set(&mut w, Key::SimCols, "5");
        assert!(!w.stale);
    }

    #[test]
    fn a_new_map_drops_the_result_made_with_the_old_one() {
        let (_s, mut w) = ready("newmap");
        assert!(w.compute());
        assert!(w.simulate());
        assert!(w.done.is_none() && w.corrected.is_empty());
    }

    #[test]
    fn settings_are_checked_like_the_command_line_checks_them() {
        let mut w = Window::new(Lang::En);
        assert!(w.options().is_ok());
        set(&mut w, Key::MaxLines, "0");
        assert_eq!(w.options().unwrap_err(), Status::BadValue(Msg::MaxLines));
        set(&mut w, Key::MaxLines, "5000000");
        set(&mut w, Key::RefX, "x");
        assert_eq!(w.options().unwrap_err(), Status::BadValue(Msg::RefX));
        set(&mut w, Key::RefX, "5");
        set(&mut w, Key::Segment, "0.5");
        let o = w.options().unwrap();
        assert_eq!((o.max_segment_mm, o.reference), (0.5, (5.0, 0.0)));
    }

    #[test]
    fn a_segment_that_is_not_more_than_zero_is_refused_by_the_levelling() {
        let (_s, mut w) = ready("zero");
        set(&mut w, Key::Segment, "0");
        assert!(!w.compute());
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn the_exported_program_carries_the_mark_and_cannot_be_levelled_again() {
        let (s, mut w) = ready("export");
        let out = s.0.join("job_levelled.nc");
        assert!(w.export_program(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("; generator: eh3d-autolevel "), "{text}");
        assert!(text.contains("\n(eh3d-autolevel"), "{text}");
        assert!(text.contains("; source: job.nc sha256:"), "{text}");
        assert!(!s.0.join("job_levelled.nc.tmp").exists());
        let mut again = Window::new(Lang::En);
        assert!(again.open_program(&out));
        assert!(again.simulate());
        assert!(!again.compute());
        assert!(matches!(again.status, Status::Failed(_)));
    }

    #[test]
    fn exporting_levels_again_when_a_setting_changed() {
        let (s, mut w) = ready("again");
        assert!(w.compute());
        set(&mut w, Key::Segment, "5");
        assert!(w.export_program(&s.0.join("out.nc")));
        assert!(!w.stale);
        assert_eq!(w.done.as_ref().unwrap().options.max_segment_mm, 5.0);
    }

    #[test]
    fn the_program_is_never_written_over_the_map() {
        let s = Scratch::new("overmap");
        let p = s.write("job.nc", PROGRAM);
        let mut w = Window::new(Lang::En);
        assert!(w.open_program(&p));
        assert!(w.simulate());
        let map_path = s.0.join("map.txt");
        assert!(w.export_map(&map_path));
        assert!(w.open_map(&map_path));
        assert!(!w.export_program(&map_path));
        assert!(matches!(w.status, Status::SameFile(_)));
        let kept = fs::read_to_string(&map_path).unwrap();
        assert!(kept.starts_with("levelling 1"));
    }

    #[test]
    fn the_map_is_never_written_over_the_program() {
        let (s, mut w) = ready("overprog");
        assert!(!w.export_map(&s.0.join("job.nc")));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(fs::read_to_string(s.0.join("job.nc")).unwrap(), PROGRAM);
    }

    #[test]
    fn a_map_written_and_read_back_is_the_same_map() {
        let (s, mut w) = ready("roundtrip");
        let p = s.0.join("map.txt");
        assert!(w.export_map(&p));
        let before = w.map.as_ref().unwrap().map.to_text();
        assert!(w.open_map(&p));
        assert_eq!(w.map.as_ref().unwrap().map.to_text(), before);
        assert_eq!(w.map_name().as_deref(), Some("map.txt"));
    }

    #[test]
    fn nothing_is_written_when_there_is_nothing_to_write() {
        let mut w = Window::new(Lang::En);
        let s = Scratch::new("nothing");
        assert!(!w.export_map(&s.0.join("m.txt")));
        assert!(!w.export_program(&s.0.join("p.nc")));
        assert!(!s.0.join("m.txt").exists() && !s.0.join("p.nc").exists());
    }

    #[test]
    fn the_name_offered_for_the_program_keeps_its_folder_and_extension() {
        let (s, w) = ready("name");
        assert_eq!(
            w.suggested_output_for(Job::ExportProgram),
            Some(s.0.join("job_levelled.nc"))
        );
        assert_eq!(
            w.suggested_output_for(Job::ExportMap),
            Some(PathBuf::from("map.txt"))
        );
        assert_eq!(w.suggested_output_for(Job::OpenMap), None);
        assert_eq!(w.work_folder(), Some(s.0.clone()));
    }

    #[test]
    fn the_picture_fits_the_map_and_the_paths() {
        let (_s, mut w) = ready("fit");
        w.fit(800.0, 600.0);
        let (x, y) = w.viewport.to_screen(100.0, 75.0, 800.0, 600.0);
        assert!((0.0..=800.0).contains(&x) && (0.0..=600.0).contains(&y));
        let empty = Window::new(Lang::En);
        assert!(empty.bounds().is_none());
    }

    #[test]
    fn the_height_under_the_pointer_comes_from_the_chosen_surface() {
        let (_s, mut w) = ready("cursor");
        let z = w.table_height(120.0, 90.0).unwrap();
        assert!(w.cursor_text(120.0, 90.0).contains(&format!("{z:+.4}")));
        w.set_method(Method::Bicubic);
        assert!(w.table_height(120.0, 90.0).is_some());
        let bare = Window::new(Lang::En);
        assert_eq!(bare.cursor_text(1.0, 2.0), "X 1.00  Y 2.00");
    }

    #[test]
    fn the_status_speaks_the_language_chosen() {
        let s = Status::Written("a.nc".into());
        assert_eq!(s.text(Lang::En), "Written: a.nc");
        assert_eq!(s.text(Lang::Es), "Escrito: a.nc");
        assert_eq!(s.text(Lang::Pt), "Escrito: a.nc");
        assert!(Status::BadValue(Msg::Segment)
            .text(Lang::De)
            .contains("Längstes Stück"));
    }

    #[test]
    fn each_job_lists_the_files_it_can_open() {
        assert!(Job::OpenMap.extensions().contains(&"txt"));
        assert!(Job::OpenProgram.extensions().contains(&"nc"));
        assert!(Job::ExportMap.is_export() && !Job::OpenMap.is_export());
    }
}
