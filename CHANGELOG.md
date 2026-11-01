# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **Codes that change the origin or the offsets are refused:** G10, G28, G30, G38, G43, G53, G92 and any other G code that is not a plain move or a setting that leaves the position alone stop the program with the number of the line, instead of being turned into a move to Z0. A dwell (G4) is kept as it is, the `%` line of the start and the end of a program passes through, and a map that gives no finite height is refused instead of writing `Z NaN`.

## [0.0.1] - A G-code program corrected with the height map of the table, bilinear or bicubic

- **The program:** cutting moves are cut in pieces that follow the surface; rapid moves are corrected only when they have a Z; the reference point is the zero of the correction.
- **The surface:** bicubic (Catmull-Rom) next to the bilinear one of the map, with the nearest edge used outside the grid.
- **What is refused with its line:** G91, G20, arcs, a move before G0 or G1, bad numbers, huge coordinates, too many pieces and a map that is not complete.
- **The command line** `eh3d-autolevel` with `apply`, `info` and `demo-map`, and an example table and program.
- **13 tests** run on a computer, with damaged input.
