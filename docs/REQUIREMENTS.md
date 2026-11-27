# EH3D-AUTOLEVEL: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because nothing of this program has run on a real machine.

The register has 57 requirements: 49 implemented, 6 prototype, 2 planned and 0 validated. By progress: 49 done, 6 partial, 0 pending and 2 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

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

## Arcs, streaming and the start position

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-033 | Arcs are cut into pieces that follow the table. | implemented | done | host-test | `arcs::an_arc_is_cut_into_pieces_that_follow_the_table` | Off by default (`arcs = refuse`); no arc has been cut on a machine. |
| ALVL-034 | The program is levelled while it is sent to the machine. | prototype | partial | simulator | `streaming::the_program_is_levelled_while_it_is_sent` | An emitter model and a simulated machine; the sending program of the suite does not call it yet. |
| ALVL-035 | The start position of the program is the position the machine reports. | prototype | partial | simulator | `start_pos::the_start_of_the_program_is_the_position_the_machine_reports` | Read from the simulated GRBL of the suite and from saved reports; no real machine has reported its position. |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-036 | A map from a real probe on a real table is read and a real program is levelled and cut. | planned | blocked | none | `planned:real_table::a_real_table_is_probed_and_a_real_program_is_levelled` | Needs a machine, a probe and a table. |
| ALVL-037 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |

## Checks of arcs, of the stream and of the start position

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-038 | The stream stops, with the spindle off, the tool lifted and nothing more sent, when a move goes outside the measured area by more than its margin, and the margin is a setting. | implemented | done | simulator | `streaming::a_move_outside_the_map_stops_the_stream_in_a_safe_state` | Tried against a simulated machine. |
| ALVL-039 | The stream stops when a corrected Z would go below the floor or the correction of Z passes its limit, and 0 is no limit for the correction. | implemented | done | simulator | `streaming::the_limits_of_depth_and_of_correction_stop_the_stream` | - |
| ALVL-040 | A line that cannot be levelled (an arc while arcs are refused, a code that moves the origin, a program that is levelled already) is never sent. | implemented | done | host-test | `streaming::a_line_that_cannot_be_levelled_is_never_sent` | - |
| ALVL-041 | Random programs are sent or stopped without a panic, what is sent keeps the limits, and a program that is not stopped is the one the file correction writes. | implemented | done | simulator | `streaming::random_programs_are_sent_or_stopped_without_a_panic` | Seeded generation, not a continuous fuzzer. |
| ALVL-042 | The limits of the stream are settings with ranges, and the `stream` command stops with the exit code 2 and keeps the lines that were sent. | implemented | done | host-test | `streaming::the_settings_hold_the_limits_and_the_command_stops_with_the_code_2` | - |
| ALVL-043 | No chord of a cut arc strays from the arc by more than the tolerance that is a setting. | implemented | done | host-test | `arcs::the_pieces_never_stray_from_the_arc_by_more_than_the_tolerance` | Seeded generation of arcs. |
| ALVL-044 | A full circle, an arc given by its radius (positive or negative) and a helix are cut right. | implemented | done | host-test | `arcs::a_full_circle_the_radius_form_and_a_helix_are_cut_right` | - |
| ALVL-045 | An arc with no centre, a radius that is too small, an end off the circle, R together with I or J, or several turns is refused with its line, and arcs stay refused unless the setting says cut. | implemented | done | host-test | `arcs::what_cannot_be_told_is_refused_with_its_line` | - |
| ALVL-046 | The settings `arcs` and `arc-tolerance-mm` and the options `--arcs` and `--arc-tolerance` switch the cutting of arcs on, with their ranges. | implemented | done | host-test | `arcs::the_settings_and_the_command_line_switch_arcs_on` | - |
| ALVL-047 | Random programs with arcs are cut or refused without a panic, and so is a program cut at any byte. | implemented | done | host-test | `arcs::random_programs_with_arcs_are_cut_or_refused_without_a_panic` | Seeded generation, not a continuous fuzzer. |
| ALVL-048 | A machine that is moving, in alarm or silent gives no place for the start. | implemented | done | simulator | `start_pos::a_machine_in_alarm_or_silent_gives_no_place` | Tried against the simulated GRBL. |
| ALVL-049 | A saved report of GRBL or the answer to M114 of Marlin gives the place, and one that cannot be trusted (no work offset, moving, in alarm, not a number) is refused. | implemented | done | host-test | `start_pos::a_saved_report_gives_the_place_and_one_that_cannot_be_trusted_is_refused` | - |
| ALVL-050 | A damaged report never panics and gives only places within the limits. | implemented | done | host-test | `start_pos::damaged_reports_never_panic_and_give_only_places_within_the_limits` | Seeded generation, not a continuous fuzzer. |
| ALVL-051 | `--start` and `--start-report` set where the program starts, are refused together, and name the option when they are wrong. | implemented | done | host-test | `start_pos::the_command_line_takes_the_start_from_a_place_or_from_a_saved_report` | - |
| ALVL-052 | An arc is modal like the other moves, and a straight move ends it. | implemented | done | host-test | `arcs::an_arc_is_modal_and_a_straight_move_ends_it` | - |
| ALVL-053 | A piece of an arc is never longer than the segment of the settings either. | implemented | done | host-test | `arcs::a_piece_is_never_longer_than_the_segment_either` | - |
| ALVL-054 | The place read through a cable that cuts the answers and adds noise is the right one or none. | implemented | done | simulator | `start_pos::the_place_is_never_wrong_on_a_cable_that_cuts_and_adds_noise` | Seeded cable of the simulated GRBL. |

## What reaches the machine and the files that are read

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-055 | A `$` command of GRBL that can move or change the machine is refused with its line, and only the ones that ask pass through. | implemented | done | host-test | `more::a_command_of_grbl_that_can_move_the_machine_is_refused_and_one_that_asks_passes` | Checked against the commands of GRBL on a computer; the commands of other controllers that do not start with `$` are not looked at. |
| ALVL-056 | The output of apply and of stream is never the program or the map that are read. | implemented | done | host-test | `streaming::the_stream_never_writes_over_what_it_reads_and_stops_at_a_jog` | - |

## Comments the controller obeys

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| ALVL-057 | A comment with a character GRBL obeys at once stops the stream, and a corrected program says which line has it. | implemented | done | host-test | `more::a_comment_grbl_would_obey_is_warned_about_and_stops_the_stream` | The text of M117 and M118 is not looked at; no controller was used. |
