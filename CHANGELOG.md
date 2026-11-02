# Changelog

All notable changes to this project are documented here.

## [Unreleased]
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
