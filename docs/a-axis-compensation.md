# Z correction for the turn of the axis A

The axis A turns the work (or the head) around a line parallel to X. When it turns, the point that is cut moves in Z even if the program does not say so. `eh3d-autolevel` can add that
movement to the Z of the levelled program.

## Settings

| Key of the file | Option of `apply` | Values | Default |
|---|---|---|---|
| `a-axis-compensation` | `--a-compensation` | `off` or `tilt` | `off` |
| `a-axis-radius-mm` | `--a-radius` | a number from -1000 to 1000 mm; not 0 when the compensation is `tilt` | `0` |

The option wins over the file. With `off` nothing changes: the program is written exactly as it is without these keys, whatever the radius. A value that makes no sense is an error: in the
file it says its line (`line 2: a-axis-compensation = tilt needs a-axis-radius-mm other than 0`), on the command line it names the option.

## Formula

With `r` the distance in mm from the axis A to the point that is cut (`a-axis-radius-mm`) and `A` the angle of the axis in degrees:

```text
shift = r * (1 - cos(A))
Z written = Z of the program + (height of the table at X, Y - height at the reference point) + shift
```

The shift is 0 at A = 0 (the program is assumed to start with A at 0, like the other axes) and has the same sign as `r`: a negative radius turns the correction round. The height of the
table still comes from X and Y only.

Worked values for r = 50 mm:

| A (degrees) | shift (mm) |
|---|---|
| 0 | 0 |
| 45 | 14.6447 |
| 60 | 25 |
| 90 | 50 |
| 180 | 100 |
| 360 | 0 |

## Where A is taken

- A cutting move (G1) that is cut in pieces: the A of each piece, interpolated in a straight line from the value it had to the value the move gives; the last piece uses the value of the
  move.
- A cutting move in one piece and a rapid move (G0) with a Z word: the A where the move ends.
- A move that has no `A` word: the A where the axis stands (it is modal; a line with only an `A` word sets it for the next move).

Moves that are not corrected for the table (a rapid move with no Z) get no shift either. The shift enters the report of the correction of Z (`Z shifted from ... to ...`), so
`warn-shift-mm` also counts it.

## Notice

When at least one move got a shift, `apply` (and `apply --dry-run`) writes one line on the error output; the program is written all the same:

```text
warning: Z corrected for the turn of axis A on 2 moves, up to +50.0000 mm (a-axis-radius-mm = 50)
```

Nothing is written when the compensation is `off` or when A stayed at 0.

## Limits

The correction is the one of a point on a circle around the axis: it does not model an offset of the axis in Y or Z, a tool length, or a turn of B or C. The radius is a setting of the
file because it depends on the machine.
