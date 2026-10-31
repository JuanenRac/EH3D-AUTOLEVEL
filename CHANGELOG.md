# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - A G-code program corrected with the height map of the table, bilinear or bicubic

- **The program:** cutting moves are cut in pieces that follow the surface; rapid moves are corrected only when they have a Z; the reference point is the zero of the correction.
- **The surface:** bicubic (Catmull-Rom) next to the bilinear one of the map, with the nearest edge used outside the grid.
- **What is refused with its line:** G91, G20, arcs, a move before G0 or G1, bad numbers, huge coordinates, too many pieces and a map that is not complete.
- **The command line** `eh3d-autolevel` with `apply`, `info` and `demo-map`, and an example table and program.
- **13 tests** run on a computer, with damaged input.
