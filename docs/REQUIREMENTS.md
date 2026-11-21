# EH3D-AUTOLEVEL: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because nothing of this program has run on a real machine.

The register has 37 requirements: 28 implemented, 4 prototype, 5 planned and 0 validated. By progress: 28 done, 4 partial, 1 pending and 4 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `ALVL-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated probe node or a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Levelling the program

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-001 | A cutting move is cut into pieces and each piece gets the Z that the map gives for its end. | implemented | done | host-test | `autolevel::the_z_follows_a_tilted_table` | - |
| ALVL-002 | The reference point is where the correction is zero. | implemented | done | host-test | `autolevel::the_reference_point_is_where_the_correction_is_zero` | - |
| ALVL-003 | A rapid move is corrected only when it has a Z word. | implemented | done | host-test | `autolevel::a_rapid_move_corrects_its_z_only_when_it_has_one` | - |
| ALVL-004 | The surface is bilinear or bicubic, and the bicubic one goes through every measured point. | implemented | done | host-test | `autolevel::bicubic_goes_through_the_measured_points_and_is_smooth_between_them` | - |
| ALVL-005 | Outside the grid the height of the nearest edge is used, and the moves outside are counted and warned about. | implemented | done | host-test | `outside::moves_outside_the_area_are_counted_with_their_distance` | - |
| ALVL-006 | A move is cut into exactly as many pieces as its length asks for, and a move that would make too many is refused. | implemented | done | host-test | `autolevel::a_move_is_cut_in_exactly_as_many_pieces_as_its_length_asks_for` | - |
| ALVL-007 | Lines that are not moves pass through, and a checksum is not a word. | implemented | done | host-test | `more::lines_that_are_not_moves_pass_through_and_a_checksum_is_not_a_word` | - |
| ALVL-008 | The spindle words of a cut move go with its first piece and the ending words with its last. | implemented | done | host-test | `more::the_spindle_words_of_a_cut_move_go_with_its_first_piece_and_the_ending_words_with_the_last` | - |
| ALVL-009 | The coordinates are written with a number of decimals that is a setting. | implemented | done | host-test | `decimals::the_decimals_follow_the_option_from_one_to_six` | - |
| ALVL-010 | The levelled program starts with the standard header and reads back with no warning. | implemented | done | host-test | `generated_gcode::the_levelled_program_has_no_alert_and_the_header_does_not_change_what_it_cuts` | Read back with the reader of the suite, not run on a machine. |

## What is refused

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-011 | Arcs, relative distances, inches, a move before any G0 or G1 and a coordinate beyond the limit are refused with the number of the line. | implemented | done | host-test | `autolevel::what_cannot_be_cut_in_pieces_is_refused_with_its_line` | - |
| ALVL-012 | Codes that move the origin or the offsets are refused and never become moves. | implemented | done | host-test | `autolevel::codes_that_move_the_origin_or_the_offsets_are_refused_and_never_become_moves` | - |
| ALVL-013 | A map without a usable height is refused instead of writing a Z that is not a number. | implemented | done | host-test | `autolevel::a_map_without_a_usable_height_is_refused_instead_of_writing_nan` | - |
| ALVL-014 | A program that carries the mark is refused, so that it is never levelled twice. | implemented | done | host-test | `strict::a_levelled_program_carries_a_mark_and_is_refused_the_second_time` | - |
| ALVL-015 | A move that leaves out an axis not set yet is refused with its line when the setting says so. | implemented | done | host-test | `unknown_axis::with_refuse_a_move_that_leaves_out_an_axis_not_set_yet_is_refused_with_its_line` | Off by default; the default starts every axis at 0. |
| ALVL-016 | An option written wrong or a word too many is an error that names it. | implemented | done | host-test | `strict::the_command_line_refuses_options_and_words_it_does_not_know` | - |
| ALVL-017 | A program cut at any byte or fed random text is levelled or refused and never panics. | implemented | done | host-test | `fuzz::a_program_cut_at_any_byte_is_levelled_or_refused_without_a_panic` | Seeded generation, not a continuous fuzzer. |
| ALVL-018 | A file over the limit of size is refused and a missing one is an error that names it. | implemented | done | host-test | `read_limit::a_file_within_the_limit_is_read_and_a_bigger_one_is_refused` | - |

## Rotary axes and checks of the map

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-019 | The words A, B and C are interpolated along each piece and the last piece ends where the move says. | implemented | done | host-test | `rotary::a_cut_move_interpolates_a_between_where_it_was_and_where_it_goes` | - |
| ALVL-020 | The turn of the axis A adds a Z compensation of radius times one minus the cosine of the angle. | implemented | done | host-test | `a_axis::the_correction_is_radius_times_one_minus_cosine_of_the_angle` | Off by default; the formula is not compared with a real machine. |
| ALVL-021 | The axis B has the same compensation with its own radius, and the two add up. | implemented | done | host-test | `rotary_axes::b_gives_the_same_correction_as_a_with_its_own_radius` | - |
| ALVL-022 | A simulated machine walks the levelled program and the depth below the table is the one asked for. | prototype | partial | simulator | `rotary_axes::on_a_simulated_machine_the_depth_below_the_table_is_the_one_the_program_asked_for` | Nothing has run on a machine with rotary axes. |
| ALVL-023 | A doubtful reading of the map is listed and `apply` still writes the program. | implemented | done | host-test | `map_checks::one_bad_reading_is_the_only_point_listed_and_its_neighbours_are_not` | - |
| ALVL-024 | `apply` warns when the largest correction of Z passes a limit that is a setting. | implemented | done | host-test | `warn_shift::apply_warns_on_the_error_output_and_writes_the_program_all_the_same` | - |
| ALVL-025 | `--dry-run` counts the result and writes nothing. | implemented | done | host-test | `dry_run::a_dry_run_counts_the_same_and_writes_nothing` | - |
| ALVL-026 | `--json` prints the result of `info` and `apply` as one object for other programs. | implemented | done | host-test | `map_checks::info_json_is_one_object_and_apply_json_replaces_the_sentence` | - |

## Probing, server and window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-027 | A probing session against a simulated probe node measures the table, tries again where the probe does not touch and resumes after a cut. | prototype | partial | simulator | `probe_session::a_session_that_was_cut_is_saved_and_resumed_to_the_same_map` | Tried against a simulated probe node; no real probe has been read. |
| ALVL-028 | The corrected program and the map are kept on the server with the figures of the run, and nothing is sent without an address. | prototype | partial | simulator | `map_checks::the_corrected_program_is_kept_on_the_server_with_the_map_and_the_figures_of_the_run` | Tried against a server in memory, not against a real server. |
| ALVL-029 | The token of the server is never shown and a refusal comes back in the language of the person. | implemented | done | host-test | `src/server::the_status_report_hides_the_token_and_tells_when_the_mode_is_off` | - |
| ALVL-030 | The window levels like the command line and a setting that changes after levelling makes the result stale. | prototype | partial | host-test | `src/window::the_window_levels_like_the_command_line` | The logic is tested without a screen; no person has looked at the drawing. |
| ALVL-031 | The window never writes the program over the map or the map over the program. | implemented | done | host-test | `src/window::the_program_is_never_written_over_the_map` | - |
| ALVL-032 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-033 | Arcs are cut into pieces that follow the table. | planned | pending | none | `planned:arcs::an_arc_is_cut_into_pieces_that_follow_the_table` | Arcs are refused with their line today. |
| ALVL-034 | The program is levelled while it is sent to the machine. | planned | blocked | none | `planned:streaming::the_program_is_levelled_while_it_is_sent` | Needs the sending program of the suite. |
| ALVL-035 | The start position of the program is read from the machine. | planned | blocked | none | `planned:start_position::the_start_of_the_program_is_the_position_the_machine_reports` | Needs a machine that reports its position. |
| ALVL-036 | A map from a real probe on a real table is read and a real program is levelled and cut. | planned | blocked | none | `planned:real_table::a_real_table_is_probed_and_a_real_program_is_levelled` | Needs a machine, a probe and a table. |
| ALVL-037 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |
