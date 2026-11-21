# EH3D-AUTOLEVEL: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Measuring the table from the window | The probe belongs to another node and program; the window opens a map from a file or simulates one | the probe node exists and the suite has a way to read it live |
| A map from a real probe | No real probe has been read; the format is tested with simulated and hand-written maps | a person probes a real table and gives the map |
| Relative distances (G91) and inches (G20) | Levelling needs absolute millimetres; converting would change what the program means | a conversion with a test that proves it exact is wanted |
| Canned cycles and tool length offsets | They move the origin; they are refused with their line instead of being turned into moves | a safe rule for them is agreed |
| Arcs in the planes of X and Z and of Y and Z (G18, G19) | The cutting is written for the plane of X and Y, where the table is followed; the other planes are refused with their line | a design for what the height of the table means for a cut in a vertical plane |
| Arcs kept as arcs with a corrected plane | An arc is cut into straight pieces so that each one gets its Z; keeping the arc needs a plane that follows the table, which is not an arc | a surface and a test that show the plane is as good as the pieces |
| Several turns in one arc (the word P) | Refused with its line; a program that needs it can write one turn after another | a test that shows the turns follow the table the same way |
| The emitter called by the sending program of the suite | The emitter is a model with a simulated machine; the sending program does not call it | the sending program can call the library and a person asks for it |
| The start position asked of a real machine for every job | It is read from the simulated controllers and from saved reports; no real machine has reported its position | a machine reports its position to the suite |
