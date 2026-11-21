# EH3D-AUTOLEVEL: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Arcs (G2, G3) cut into pieces | The correction of an arc has to follow the table along a curve; today an arc is refused with its line so that nothing is levelled wrongly | the cutting of an arc into straight pieces has a design and a test against a plane |
| Start position taken from the machine | The program is assumed to start at 0 on every axis, or refused with `unknown-axis = refuse`; reading the real position needs a machine | a machine reports its position to the suite |
| Correcting the program while it is sent | It needs the program that sends a job; today the correction is made before, on a file | the sending program of the suite can call the library |
| Measuring the table from the window | The probe belongs to another node and program; the window opens a map from a file or simulates one | the probe node exists and the suite has a way to read it live |
| A map from a real probe | No real probe has been read; the format is tested with simulated and hand-written maps | a person probes a real table and gives the map |
| Relative distances (G91) and inches (G20) | Levelling needs absolute millimetres; converting would change what the program means | a conversion with a test that proves it exact is wanted |
| Canned cycles and tool length offsets | They move the origin; they are refused with their line instead of being turned into moves | a safe rule for them is agreed |
