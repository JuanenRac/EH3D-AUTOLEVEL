# EH3D-AUTOLEVEL: scope

Status: **host-test**. EH3D-AUTOLEVEL is a library, a command line and a window that correct the Z of a G-code program with the height map of the table. It has been tried on a computer only, with a simulated table and a simulated probing session: it has not corrected a program that a machine ran. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The program of the suite that takes a G-code program in absolute millimetres and a height map of the table and writes the same program with its cutting moves cut in pieces whose Z follows the surface, so that a job cuts at the same depth over a table that is not flat. It also reads a height map, says how good it looks (tilt, roughness, doubtful readings, points not measured) and can make a simulated one to try the rest.

## What it does

* Cuts each cutting move (G1) into pieces no longer than a chosen length and gives each piece the Z of the table at its end, with a bilinear or a bicubic surface; a rapid move (G0) is corrected only when it has a Z word.
* Refuses, with the number of the line, what it cannot level without guessing: relative distances, inches, arcs unless the setting says to cut them, codes that move the origin, a move before any G0 or G1 and a number that is not a number.
* Marks the program it writes, so that a program that carries the mark is refused when it is levelled again, and writes the standard header of the suite before the mark.
* Interpolates the rotary words A, B and C along each piece and, when asked, adds the Z compensation that the turn of A or B brings.
* Describes a map (area, spacing, tilt, roughness, doubtful readings, points not measured) and makes a simulated one with noise and a bad point; `--json` gives the result as one object for other programs.
* Cuts an arc (G2, G3 in the plane of X and Y) into straight pieces within a tolerance of the settings, each piece with the Z of the table, when `arcs = cut` is set.
* Levels a program line by line through an emitter model into a simulated machine, with limits of depth, of correction and of the measured area that stop it in a safe state, and takes the start position of the program from a report that a machine gives.
* Shows the same work in a window (built with `--features gui`) in eight languages and keeps the corrected program and the map on the EH3D server when it is asked to.

## What it will not do

* No decision about the limits of a machine, its soft limits or its emergency stop (see below).
* Arcs are refused unless `arcs = cut` is set; arcs in other planes than X and Y are not cut (see `NOT_YET.md`).
* No measuring of a table: the map comes from the probe of the suite or from a simulated table.
* No sending of the program to a real machine: the emitter is a model that is tried on a simulated machine, and the sending program of the suite does not call it yet.
* No claim that anything was proven on a machine: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** a G-code program and a height map as plain files, its settings file and, when asked, the server of the suite.
* **It writes:** the levelled program (never over the map, and over the program only when asked and with no temporary file left), a map from `demo-map`, the JSON of a result and, when asked, artifacts on the server.
* **It is read by:** the other programs of the suite through those files (ISOCAM reads the same map format) or through the server; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-COMMON` (the map, the client of the server and the readers and simulated machines of the controllers) and `EH3D-CORE-LIBS` (the G-code reader), built next to it as sibling folders.

## What it never decides

The limits of a machine, its interlocks and its emergency stop belong to the machine and to its controller, not to this program. A levelled program is a program written cleanly, not a verified one: nothing here knows where the machine stops, so a person must check what it writes before anything runs. The rules here can only refuse a program or add a warning, and no setting makes the program write something it would otherwise refuse.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
