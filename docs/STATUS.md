# EH3D-AUTOLEVEL: status

## Current state

EH3D-AUTOLEVEL is a working program on a computer: a library (`eh3d_autolevel`), a command line (`eh3d-autolevel`) and a window (built with `--features gui`). It levels programs (arcs too, when the setting says cut), describes maps, runs a simulated probing session against a simulated probe node, levels a program line by line through an emitter model into a simulated machine that stops in a safe state, and takes the start position from a report that a machine gives. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and nothing has moved a real machine or been compared with a collection of real files.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 57 requirements: 49 implemented, 6 prototype, 2 planned and 0 validated. By progress: 49 done, 6 partial, 0 pending and 2 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether an arc should also be kept as an arc with a corrected plane, instead of being cut into straight pieces (today it is cut, when the setting says so).
* How the sending program of the suite will call the emitter, and whether the correction made while sending replaces the correction of the file or sits beside it.
* Whether the start position read from a machine should be asked again at the start of every job or kept from the last one.

## What it needs from other repositories

* `EH3D-COMMON`: the height map, the client of the server and the readers and simulated machines of the controllers, as a sibling folder at the version named in the workflows.
* `EH3D-CORE-LIBS`: the G-code reader.
* The probe node and the program that sends a job to a machine, for anything that is not simulated; the emitter is not called by the sending program yet.

## Known technical debt and omitted tests

* The map used in every test is simulated or written by hand; no map from a real probe has been read.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a machine or a screen are not written: the register names each as `planned:` with the test that will show it.
