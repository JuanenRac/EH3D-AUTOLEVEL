# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **The validation of the CI is the reusable one of the documentation hub:** the job `validate` calls `validate.yml` of EH3D-DOCS (pinned by commit) and the check of the CHANGELOG is a job of its own.
- **The pins follow the new releases:** the CI and the release build take `EH3D-COMMON` at `v0.2.1` and `EH3D-CORE-LIBS` at `v0.0.9`, and the manifest asks for `min_common` 0.2.1. The picture of the window in the README is taken again with the program of today.

## [0.0.8] - Rotary axes A, B and C and the same size limit on the command line

- **Fuzz tests with a fixed seed:** 800 random programs (with the compensation of A and B on) are levelled or refused without a panic and without a number that is not finite, the same program gives the same levelling every time, and a program cut at any byte is levelled or refused.

- **The axes B and C complete the rotary axes:** the axis B has the same compensation of Z as A with its own radius (`b-axis-compensation`, `b-axis-radius-mm`, `--b-compensation`, `--b-radius`; the two add up and the notice names B), and C, which turns about Z, changes no height and is carried and interpolated like the others. `tests/rotary_axes.rs` walks the levelled program with a simulated machine and checks the depth below the table with A, B and C turning inside moves cut in pieces; nothing has run on a machine with rotary axes.

- A screenshot of the window in the README, taken with the published program: an example program over the height map of an example table.
- **The command line reads files with the same size limit as the window:** the height map, the settings file and the G-code program given to `apply`, `info` and the other commands are refused with their name when they are larger than 256 MiB, instead of being loaded into memory whole; two more tests (174).
- **Z correction for the turn of the axis A:** `a-axis-compensation = tilt` and `a-axis-radius-mm = r` in the settings file (or `--a-compensation tilt --a-radius r`, which win over the file) add `r * (1 - cos A)` to the Z of every written move after the height of the table, with A in degrees as the move leaves it (interpolated piece by piece, modal like the other axes). It is `off` by default, which changes no output; `tilt` with a radius of 0, or a radius beyond -1000 to 1000 mm, is an error that says its line (file) or names the option. `apply`, and `apply --dry-run`, show a notice on the error output with the number of moves and the largest correction when it was applied, and the shift is part of the `Z shifted from ... to ...` summary and of `warn-shift-mm`. The formula and worked values are in `docs/a-axis-compensation.md`; `examples/autolevel.cfg` carries the new keys and is the same file as `eh3d-autolevel config`.

## [0.0.7] - Rotary axes kept between moves, a warning when the correction of Z passes a limit and the picture mode

- **A warning when the correction of Z is large:** `warn-shift-mm` in the settings file (or `--warn-shift MM`, which wins over the file; 0 by default: no warning, otherwise from 0.001 to 100 mm) makes `apply`, and `apply --dry-run`, show on the error output the largest correction of Z, with its sign, when it passes the limit up or down; the corrected program is written all the same and the exit code does not change. `examples/autolevel.cfg` carries the new key and is the same file as `eh3d-autolevel config`. The tests use a table that rises 0.001 mm for every mm (0.05 mm at X = 50) and the range of the setting.
- **The rotary axes A, B and C are interpolated:** when a cutting move (G1) is cut in pieces, its `A`, `B` and `C` words go in a straight line from the value they had to the one the move gives, like X and Y, written with the configured decimals; the last piece ends exactly on the target and the Z of the table still comes from X and Y only. The axes are modal (a move with no `A` writes none, a line with only rotary words or a rapid move sets where they stand) and start at 0. A move that is not cut keeps its words as they were written, and a program with no A, B or C is written exactly as before.
- **`max-lines` in the settings file:** the most lines the levelled program may have (5000000 by default, from 1 to 100000000), a new strict key with the error of its line (not known, not valid, given twice); `--max-lines` on the command line wins over the file and `eh3d-autolevel config` prints it with the rest.
- **A picture mode for the window:** `--shot FILE.png` (built with `--features gui`) opens the window at the size of `--size WxH` (320 to 8192, 1180x760 by default), with `--theme forge|dark|light`, `--lang CODE` and the files of `--open FILE`, waits `--wait N` frames (0 to 3600, 6 by default), saves what it drew as a PNG file and closes by itself. An existing file is never replaced unless `--replace` is given, the window keeps no settings in this mode, the exit code is 0 when the file was written, 1 when it was not (also when the picture does not come back within 600 frames) and 2 for wrong arguments, and a build without the window says so. The arguments are read by plain code with its tests.

## [0.0.6] - The window with the height map, the standard header of the levelled program and the Windows icon

- **The coordinates of the levelled program have three decimals:** X, Y and Z are always written with the same number of decimals (`G1 X1.000 Y0.000 Z-0.100`) instead of as many as were needed up to four; the other words (`F`, `S`, `M`...) are written as before.
- **A settings file with the decimals:** `decimals` (3 by default, from 1 to 6) is read from a `key = value` file given with `--config FILE` or given on the command line with `--decimals N`, which wins over the file; `eh3d-autolevel config` prints the file with its defaults (`examples/autolevel.cfg`). A value that is not valid, a key that is not known or a key given twice is an error with the number of the line.
- **Binaries for every system:** `release-binaries.yml` builds the program with its window for Windows (64 and 32 bits), Linux and macOS (Apple silicon and Intel), packs each one with the licence, the readme, the icon and the examples, writes `SHA256SUMS` and a build manifest, attests their provenance, verifies the artifact as a stranger would, prepares a draft release only after the owner approves it (a dry run is the default) and opens an issue `build-failed` when anything does not finish.
- **The dependencies on the shared readers name the version that has what is used:** the G-code crate (and for ISOCAM the Excellon one) is required from 0.0.7, the version that brought the way of writing a number, the header of a program and the units given by the caller.
- **The levelled program starts with the standard header:** the generator and its version, the units, the program it came from with its SHA-256 and the tools, as comment lines before the mark (`mark_with_header`); the mark is found where it is, so a levelled program is still refused the second time.
- **The G-code is read back in the tests:** the levelled program and the file of the command line are read with the reader of the suite and walked the way the strict mode of the G-code debugger walks them; there is no warning, no rapid move into the material and no cut with the spindle off.
- **A window:** built with `--features gui`, `eh3d-autolevel` with no arguments (or with files, or `gui [FILE...]`) opens a window in the look of the other programs of the suite (amber on charcoal, the icon of the program). A G-code program and a height map are opened from the list of files of the window or dropped on it (the first line tells a map from a program); the table is drawn as a coloured mesh, the original path and the levelled path over it (coloured by the table under each piece), with zoom (wheel), pan (drag) and the height of the table under the pointer; the longest piece, the reference point, the surface (bilinear or bicubic) and the most output lines can be edited, a simulated table can be made, and the levelled program (with the mark that says it was levelled) and the map are exported. The command line works as before, and a build without the feature is only a command line.
- **The window speaks the eight languages:** English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal, chosen from the system and changeable in the window.
- **The logic of the window needs no screen:** `window` (program, map, settings, levelling, paths, colours, export, status), `viewport` (zoom and pan), `browse` (the list of files) and `lang` (the words) are plain code with their own tests; `gui` only draws them. `gcode::words` is public for the picture of the paths.
- **The Rust version needed is 1.88,** the one the window needs.
- **Lines that are not moves pass through:** `$` commands, `#` assignments and the messages `M117` and `M118` are written as they are, and the checksum of a numbered line is not read as a word.
- **The M words of a cut move go with its last piece:** `G1 X50 M5` stops the spindle at the end of the move and not at its first piece.
- **The size of the output is limited:** `--max-lines` (5000000 by default) refuses a program that would have more lines, before the pieces are made.

## [0.0.5] - Audit fixes, the new reading limits, tests and the application icon

- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **The original is not overwritten:** `apply` refuses an output file that is also the map and writes the corrected program through a temporary file (levelling in place still works), and the level mark goes after the leading `%` of programs with Windows line ends too.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **Modal words kept:** a move line that also carries G90, G54 or another allowed G word keeps it when the move is rewritten.
- **More tests:** the mark of a levelled program, the bicubic surface at the edges and on a plane, the methods side by side and the lines of a postprocessor.

## [0.0.4] - CI hardening and shared validator

- **Faster, bounded CI:** the Cargo build is cached, a run that a newer push supersedes is cancelled, every job has a time limit, and the Rust jobs live in their own workflow that is skipped when only documents or images change.
- **A fixed Rust toolchain:** `rust-toolchain.toml` names the compiler version (with rustfmt and clippy) that the CI installs, so a new stable release cannot turn the lints red without a change here.
- **Monthly dependency updates:** Dependabot opens one grouped pull request a month for the GitHub Actions.
- **A line for every change:** a pull request that changes code must add a line under `[Unreleased]` of this file (`tools/changelog_pr_check.py`; the label `no-changelog` skips it).
- **Shared checks updated:** the validator of the suite checks the text of the banner against the manifest and reads the files that git tracks.
- **More tests:** comments that name a forbidden code, tables of any level, the exact number of pieces of a move, modal moves, Windows line ends, numbers with signs and dots, the limits of the coordinates, the marks of a levelled program and every error of the command line.

## [0.0.3] - Dry run and shared argument helper
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge.

- **`apply --dry-run`:** corrects and counts what it would do, without writing the output file (which may then be left out).
- **One shared helper for the command-line arguments** (`src/cli_args.rs`: flag, switch, num and a strict finish), the same file in every small utility of the suite; the copies this program had behaved the same, so nothing changes for a person.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.2] - A program is never levelled twice, a strict command line and the versions of the sibling repositories

- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Never levelled twice:** the corrected program starts with a comment line that says it was levelled and with which map; a program that carries it is refused with its line, because the correction of the table would be added twice.
- **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it; before, it was ignored and the output was made with the default values.
- **Codes that change the origin or the offsets are refused:** G10, G28, G30, G38, G43, G53, G92 and any other G code that is not a plain move or a setting that leaves the position alone stop the program with the number of the line, instead of being turned into a move to Z0. A dwell (G4) is kept as it is, the `%` line of the start and the end of a program passes through, and a map that gives no finite height is refused instead of writing `Z NaN`.

## [0.0.1] - A G-code program corrected with the height map of the table, bilinear or bicubic

- **The program:** cutting moves are cut in pieces that follow the surface; rapid moves are corrected only when they have a Z; the reference point is the zero of the correction.
- **The surface:** bicubic (Catmull-Rom) next to the bilinear one of the map, with the nearest edge used outside the grid.
- **What is refused with its line:** G91, G20, arcs, a move before G0 or G1, bad numbers, huge coordinates, too many pieces and a map that is not complete.
- **The command line** `eh3d-autolevel` with `apply`, `info` and `demo-map`, and an example table and program.
- **13 tests** run on a computer, with damaged input.
