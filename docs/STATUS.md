# EH3D-AUTOLEVEL: status

## Current state

EH3D-AUTOLEVEL is a working program on a computer: a library (`eh3d_autolevel`), a command line (`eh3d-autolevel`) and a window (built with `--features gui`). It levels programs, describes maps and runs a simulated probing session against a simulated probe node. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and nothing has moved a real machine or been compared with a collection of real files.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 37 requirements: 28 implemented, 4 prototype, 5 planned and 0 validated. By progress: 28 done, 4 partial, 1 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* How arcs should be cut: into straight pieces that follow the table, or kept as arcs with a corrected plane.
* Whether the start position of a program is read from the machine or given by the person; today it is taken to be 0 on every axis.
* Whether the correction should be made while the program is sent instead of before.

## What it needs from other repositories

* `EH3D-COMMON`: the height map and the client of the server, as a sibling folder at the version named in the workflows.
* `EH3D-CORE-LIBS`: the G-code reader.
* The probe node and the program that sends a job to a machine, for anything that is not simulated.

## Known technical debt and omitted tests

* The map used in every test is simulated or written by hand; no map from a real probe has been read.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a machine or a screen are not written: the register names each as `planned:` with the test that will show it.
