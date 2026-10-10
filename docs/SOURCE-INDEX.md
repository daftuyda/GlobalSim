# Source reference

Modules and functions for navigating the implementation. Function lists also
include test helpers; see [testing](TESTING.md) for test status.

## [activation_conditions.rs](../src/activation_conditions.rs)

Static and dynamic skill-condition evaluation.

Functions: `not_supported`, `noop_filter`, `new`, `eq`, `neq`, `lt`, `lte`, `gt`, `gte`, `immediate`, `retail_phase_condition`, `filter`, `retail_distance_rate_condition`, `filter`, `retail_remaining_distance_condition`, `filter`, `retail_final_corner_half_condition`, `filter`, `retail_corner_condition`, `filter`, `retail_slope_condition`, `filter`, `retail_furlong_condition`, `filter`, `random_cond`, `noop_all`, `noop_immediate`, `noop_random`, `noop_erlang_random`, `value_filter`, `mk`, `roster_condition`, `mk`, `same_style_rate_condition`, `mk`, `identity_transform`, `passthrough_static`, `live_condition`, `mk`, `live_simple`, `lane_type_value`, `behind_in_condition`, `mk`, `live_with_static`, `order_of_value`, `order_filter`, `order_get_pos`, `order_rate_get_pos`, `order_in_filter`, `order_out_filter`, `band_track`, `continuous_band_check`, `track_order_bands`, `section_random`, `is_sorted_by_start`, `accumulatetime_condition`, `mk`, `all_corner_random_filter_eq`, `corner_filter_eq`, `corner_filter_neq`, `corner_random_filter_eq`, `distance_rate_filter_lte`, `distance_rate_filter_gte`, `distance_rate_after_random_filter_eq`, `distance_type_filter_eq`, `distance_type_filter_neq`, `furlong_filter_eq`, `down_slope_random_filter_eq`, `hp_per_filter_lte`, `hp_per_filter_gte`, `hp_condition_percent`, `is_activate_heal_skill_filter_eq`, `take_anyskill_nodes`, `is_activate_any_skill_filter_eq`, `is_activate_other_skill_detail_filter_eq`, `is_basis_distance_filter_eq`, `is_badstart_filter_eq`, `is_dirtgrade_filter_eq`, `is_dirtgrade_filter_neq`, `is_tight_track_filter_eq`, `is_finalcorner_filter_eq`, `is_finalcorner_laterhalf_filter_eq`, `is_finalcorner_random_filter_eq`, `is_hp_empty_onetime_filter_eq`, `is_lastspurt_filter_eq`, `is_last_straight_filter_eq`, `is_last_straight_onetime_filter_eq`, `is_temptation_filter_eq`, `is_used_skill_id_filter_eq`, `lastspurt_condition`, `mk`, `phase_filter_eq`, `phase_filter_lt`, `phase_filter_lte`, `phase_filter_gt`, `phase_filter_gte`, `phase_corner_random_filter_eq`, `phase_firsthalf_filter_eq`, `phase_firstquarter_filter_eq`, `phase_laterhalf_filter_eq`, `phase_laterhalf_random_filter_eq`, `phase_random_filter_eq`, `phase_straight_random_filter_eq`, `phase_first_half_straight_random_filter_eq`, `phase_latter_half_straight_random_filter_eq`, `gate_block`, `or_default_num_umas`, `post_number_filter_eq`, `post_number_filter_lte`, `post_number_filter_gte`, `random_lot_filter_eq`, `remain_distance_filter_eq`, `remain_distance_filter_lte`, `remain_distance_filter_gte`, `run_at_full_speed_random_filter_eq`, `running_style_filter_eq`, `slope_filter_eq`, `straight_front_type_filter_eq`, `straight_random_filter_eq`, `last_straight_random_filter_eq`, `temptation_count_filter_eq`, `up_slope_random_filter_eq`, `up_slope_random_later_half_filter_eq`, `compete_fight_count_filter_gt`, `is_overtaking_value`, `change_order_up_end_after_static`, `change_order_up_finalcorner_after_static`, `change_order_up_middle_static`, `activate_count_all_filter_lte`, `activate_count_all_filter_gte`, `activate_count_end_after_filter_gte`, `activate_count_heal_filter_gte`, `activate_count_later_half_filter_gte`, `activate_count_middle_filter_gte`, `activate_count_start_filter_gte`, `v_base_power`, `v_base_speed`, `v_base_stamina`, `v_base_guts`, `v_base_wiz`, `v_corner_count`, `v_course_distance`, `v_grade`, `v_ground_condition`, `v_ground_type`, `v_motivation`, `v_popularity`, `v_rotation`, `v_season`, `v_time`, `v_track_id`, `v_weather`, `lv_visiblehorse`, `bashin_condition`, `mk`, `lv_behind_near_lane_time`, `lv_behind_near_lane_time_set1`, `lv_blocked_all_continuetime`, `lv_blocked_front`, `lv_blocked_front_continuetime`, `lv_blocked_side_continuetime`, `lv_distance_diff_rate`, `distance_spread_percentage`, `leader_gap`, `lv_distance_diff_top`, `lv_distance_diff_top_float`, `lv_infront_near_lane_time`, `lv_is_move_lane`, `lv_is_surrounded`, `lv_near_count`, `lv_near_infront_count`, `lv_temptation_count_behind`, `lv_temptation_count_infront`, `lv_temptation_opponent_count_behind`, `lv_temptation_opponent_count_infront`, `running_style_temptation_count_live`, `nige`, `senko`, `sashi`, `oikomi`, `cell_order_band_in_200`, `cell_order_band_in_400`, `cell_order_band_in_500`, `cell_order_band_in_800`, `cell_order_band_out_200`, `cell_order_band_out_400`, `cell_order_band_out_500`, `cell_order_band_out_700`, `lookup_condition`, `sample_policy`, `apply`, `make_and`, `make_or`, `build_conditions`, `activate_count_all_random_filter_gte`, `activate_count_all_random_filter_lte`, `acr_activate_count_end_after_filter_gte`, `acr_activate_count_later_half_filter_gte`, `acr_activate_count_middle_filter_gte`, `acr_activate_count_start_filter_gte`, `build_conditions_with_activate_counts_as_random`, `distance_percentage_uses_field_spread_and_floors`, `large_field_extends_balanced_outer_post_grouping`, `ordinary_eighteen_runner_posts_match_extracted_client_table`.

## [activation_sample_policy.rs](../src/activation_sample_policy.rs)

Selection of activation points within eligible distance regions.

Functions: `kind`, `reconcile`, `sample`, `sample_distance_window`, `sample_random`, `sample_straight_random`, `sample_distribution`, `uniform_distribution`, `log_normal_distribution`, `erlang_distribution`, `place_first_corner_trigger`, `place_all_corner_triggers`, `union_regions`.

## [condition_parser.rs](../src/condition_parser.rs)

Parsing and composition of skill-condition expressions.

Functions: `is_id_char`, `tokenize`, `peek`, `advance`, `parse_or`, `parse_and`, `parse_cmp`, `parse`, `parse_with_retail`, `parse_with_retail_runtime`.

## [conserve_power.rs](../src/conserve_power.rs)

Conserve Power accumulation, cooldowns and release duration.

Functions: `default`, `enable_trace`, `status`, `observation`, `record`, `record_release`, `diagnostics`, `supported`, `update`, `duration`.

## [course_data.rs](../src/course_data.rs)

Course definitions, race phases and stat modifiers.

Functions: `is_sorted_by_start_f`, `phase_start`, `phase_end`, `course_speed_modifier`, `init_courses`, `get_course`.

## [horse_types.rs](../src/horse_types.rs)

Runner attributes, strategies and aptitude types.

Functions: `base_running_style`, `from_i32`, `parse_str`, `from_i32`, `parse_str`, `strategy_matches`.

## [hp_policy.rs](../src/hp_policy.rs)

HP consumption, recovery and last-spurt policy.

Functions: `parameters`, `course`, `horse`, `integrated_hp_drain_and_heals_match_python_bits`, `spurt_policy_uses_shared_stream_and_current_hp`, `exhausted_hp_has_no_debt_against_later_recovery`, `new`, `with_retail`, `init`, `get_status_modifier`, `hp_per_second`, `tick`, `has_remaining_hp`, `hp_ratio_remaining`, `recover`, `spend`, `tick_start_dash`, `get_last_spurt_pair`, `init`, `tick`, `has_remaining_hp`, `hp_ratio_remaining`, `recover`, `get_last_spurt_pair`, `spend`, `hp_value`.

## [json.rs](../src/json.rs)

JSON parsing and serialization.

Functions: `as_f64`, `as_i64`, `as_str`, `as_bool`, `as_array`, `as_object`, `get`, `is_null`, `get_f64`, `get_i64`, `get_str`, `get_bool`, `get_array`, `get_object`, `parse`, `skip_ws`, `parse_value`, `expect_literal`, `parse_object`, `parse_array`, `parse_string`, `parse_number`, `to_string`, `write_value`, `write_number`, `write_string`, `obj`, `num`, `arr`.

## [lane_selection.rs](../src/lane_selection.rs)

Lane preferences and passing corridors.

Functions: `visible`, `collect_targets`, `passing_lane`.

## [main.rs](../src/main.rs)

Request handling, simulation configuration and result serialization.

Functions: `opt_array`, `load_course_table`, `load_skill_table`, `skill_id_and_level`, `experimental_grid_enabled`, `starting_coordinates`, `parse_race_spec`, `strategy_id_for_output`, `new_builder`, `is_usable`, `run_race`, `downsample_result`, `gate_order`, `restore_request_order`, `run`, `main`, `grid_requires_exact_contract_and_hundred_horses`, `grid_has_five_separate_rows_and_twenty_ordinary_stalls`.

## [overtaking.rs](../src/overtaking.rs)

Overtaking targets and event tracking.

Functions: `is_target`.

## [race_field.rs](../src/race_field.rs)

Field coordination, ranking, nearby runners and effect dispatch.

Functions: `is_advantage_ability`, `is_skill_demerit`, `new`, `update`, `step_all`, `retail_overtake_times_for`, `next_overtake_times`, `has_character`, `record_heal`, `record_ability_activation`, `cache_skill_activations`, `other_advantage_active`, `viewer_remaining`, `visible`, `skill_targets`, `queue_skill_effect`, `flush_skill_effects`, `order_of`, `previous_order_of`, `distance_to_leader`, `ranked_distance_bounds`, `distance_to_ahead`, `distance_to_behind`, `lane_gap_to_ahead`, `lane_gap_to_behind`, `did_overtake`, `front_blocker`, `side_room`, `is_surrounded`, `is_solo_front_runner`, `ordinary_front_runner_count`, `later_style_ahead`, `pacemaker_is_later_style_ahead`, `pacemaker`, `frontless_pacemaker`, `frontless_keep`, `leading_eligible`, `leading_eligible_in`, `gap_to_pacemaker`, `near_count`, `near_infront_count`, `is_opponent`, `temptation_count_behind`, `temptation_count_infront`, `running_style_temptation_count`.

## [race_field_retail.rs](../src/race_field_retail.rs)

Ordered field updates for the detailed runtime.

Functions: `step_all_retail`.

## [race_parameters.rs](../src/race_parameters.rs)

Race-condition types and configuration parsing.

Functions: `parse_ground_condition`, `parse_weather`, `parse_season`, `parse_time`, `parse_grade`, `default`.

## [race_solver.rs](../src/race_solver.rs)

Runner state, movement, skills and competition.

Functions: `base_speed`, `base_target_speed`, `last_spurt_speed`, `calc_base_accel`, `compete_dist_threshold`, `compete_strategy_coef`, `compete_stamina_strategy_coef`, `secure_lead_strategy_coef`, `secure_lead_stamina_strategy_coef`, `competition_distance_coefficient`, `position_keep_course_factor`, `position_keep_min_threshold`, `position_keep_max_threshold`, `new_timer_value`, `new`, `add`, `duration_coefficient`, `activation_modifier`, `noop_hook`, `add_stat_modifier`, `final_corner_half_value`, `corner_value`, `slope_type`, `has_active_skill`, `remaining_skill_duration`, `set_starting_lane`, `is_spot_struggling`, `new`, `rng_uniform`, `call_activate_hook`, `call_deactivate_hook`, `new_skill_timer`, `next_modifier_token`, `new_timer`, `release_timer`, `init_hills`, `get_max_speed`, `step`, `step_forward`, `step_lateral`, `step_field_conditions`, `update_blocking`, `update_lane_target`, `lane_preference_flags`, `visible_distance`, `apply_external_effect`, `refresh_stat_effects`, `update_lane`, `update_near_lane_timers`, `update_kakari`, `update_position_keep`, `clear_position_keep`, `position_keep_wisdom_check`, `roll_competition_wisdom`, `update_position_keep_non_nige`, `update_position_keep_front_runner`, `update_last_spurt_state`, `update_target_speed`, `apply_forces`, `downhill_check`, `update_hills`, `update_phase`, `record_overtakes`, `process_skill_history`, `extend_running_skills_on_overtake`, `additional_skill_event`, `observe_anyskill`, `apply_unapplied_additional`, `process_skill_activations`, `expire_skill_effects`, `process_skill_priority`, `process_pending_armed`, `process_pending_full`, `conserve_release_duration`, `conserve_release_reaches_threshold`, `release_conserve_power_now`, `skill_cooldown_ready`, `start_expired_skill_cooldowns`, `start_skill_cooldown`, `activate_skill`, `is_debuff_immune`, `apply_activation`, `do_activate_random_gold`, `select_random_gold`, `update_competition_mechanics`, `update_stamina_keep`, `latch_active_spot_cutoff`, `update_spot_struggle`, `start_spot_struggle`, `update_compete_before_spurt`, `update_secure_lead`, `prepare_retail_duel_start`, `update_dueling`, `set_on_skill_activate`, `set_on_skill_deactivate`, `cleanup`, `solver`, `random_target_selection_precedes_random_ability_value`, `force_lane`, `null_lane_controller_keeps_target_and_ignores_fixed_lane_skill`, `extra_pacing_checks_all_ahead_and_exits_after_passing_later_style`, `rushing_mode_precedes_extra_catchup_and_short_course_factor_is_bounded`, `cornerless_course_cannot_open_a_duel_on_its_last_straight`, `experimental_duel_groups_do_not_sustain_unrelated_participants`, `duel_lane_and_distance_boundaries`, `duel_warmup_same_target_speed_and_count`, `duel_warmup_checks_order_and_retains_eligible_rival`, `finished_runner_is_not_a_duel_target`, `duel_proximity_survives_recovery_and_tracks_multiple_partners`, `duel_partner_hp_and_former_participant_exit_rules`, `spot_struggle_group_start_and_expiration_are_once_per_style`, `sixth_can_duel_with_fifth_but_seventh_cannot_duel_with_sixth`, `duel_condition_checks_accumulated_real_count_not_random_position`, `spot_struggle_lane_and_entry_cutoff`, `boxed_runner_cancels_stale_target`, `forced_lane_step_sweeps_side_envelope_in_both_directions`, `overlapping_runners_can_separate_and_finished_runners_do_not_block`, `outer_boundary_does_not_report_phantom_movement`, `lateral_guard_compares_positions_from_the_same_tick`, `partial_start_delay_preserves_effective_lateral_and_near_timer_dt`, `surrounded_uses_single_precision_cached_gaps_at_inclusive_limits`, `special_following_timer_is_independent_and_resets_outside_its_corridor`, `finished_runner_is_not_a_front_or_side_blocker`.

## [race_solver_builder.rs](../src/race_solver_builder.rs)

Stat adjustment and runner/skill construction.

Functions: `init_skills`, `skills_table`, `random_activation_is_general`, `asitame_release_modifier`, `asitame_release_duration`, `asitame_calc_approximate_modifier`, `stamina_syoubu_distance_factor`, `stamina_syoubu_calc_approximate_modifier`, `odd_raw_stats_retain_half_points_before_mood_and_caps`, `adjust_overcap`, `base_stat`, `build_base_stats`, `build_adjusted_stats`, `build_adjusted_stats_for_mode`, `is_target`, `valid_skill_type`, `level_multiplier`, `retail_skill_value`, `build_skill_effects`, `skill_rarity_from_i32`, `build_skill_data`, `wisdom_seed`, `race_seed_hi`, `new`, `seed`, `wisdom_checks`, `num_umas`, `roster_conditions`, `course`, `mood`, `ground`, `weather`, `season`, `time`, `grade`, `dirt_grade`, `horse`, `single_mode`, `gate_index`, `add_skill`, `with_asiwotameru`, `with_stamina_syoubu`, `prepare`, `validate_skills`, `build`, `master_and_level_conversion_round_separately`.

## [race_solver_retail.rs](../src/race_solver_retail.rs)

Detailed runtime integration and runner updates.

Functions: `course`, `config`, `solver`, `check`, `zero_time_preserves_course_event_readiness`, `complete_isolated_races_match_python_tick_bits`, `retail_phase_uses_cached_enum_and_preserves_boundary_window`, `whole_field_races_match_python_tick_bits`, `new`, `collect_retail_duel_proximity`, `enable_retail_runtime`, `pace_cancel_current_speed`, `retail_current_speed`, `retail_target_speed_with_current_skills`, `retail_lane_context`, `retail_other_horses`, `prepare_retail_tick`, `update_retail_pace`, `update_retail_spurt`, `step_forward_retail`.

## [random_gen.rs](../src/random_gen.rs)

Seeded random-number generation for the conventional runtime.

Functions: `new`, `step`, `pair`, `int32`, `random`, `uniform`.

## [region.rs](../src/region.rs)

Distance intervals and region operations.

Functions: `new`, `intersect`, `fully_contains`, `rmap_one`, `rmap_multi`, `union`.

## [retail_geometry.rs](../src/retail_geometry.rs)

Course transforms, lane width and world-space movement.

Functions: `finite`, `number`, `quaternion`, `backend_distance_to_world`, `lerp`, `dot`, `quaternion_lerp`, `quaternion_multiply`, `from_value`, `bundled`, `load`, `from_value`, `corner_state_at`, `lane_max_at`, `position`, `advance`, `directory`, `data`, `object`, `captured_gates_and_per_runner_width_changes`, `invalid_identity_samples_and_widths_are_rejected`, `corner_ids_and_final_flag_follow_explicit_events`, `final_half_uses_last_eligible_event_start`, `short_quaternion_arc`, `python_port_vectors_match_bits_for_all_captured_courses`.

## [retail_geometry_assets.rs](../src/retail_geometry_assets.rs)

Embedded geometry lookup by course ID.


## [retail_initialization.rs](../src/retail_initialization.rs)

Seeded initialization, opening values and skill lotteries.

Functions: `initial_base_wisdom`, `official_export_values_and_constructor_bounds`, `number`, `skill_id`, `field`, `terms`, `distance_window`, `sample_trigger`, `initialize`, `initialize_sections_and_rushing`, `absent_following_straight_keeps_empty_trigger_and_rng_state`, `all_ninety_official_delays_are_bit_exact_from_inputs`, `all_official_rushing_eligibility_and_sections_follow_shared_stream`.

## [retail_lane.rs](../src/retail_lane.rs)

Lane targeting, lateral movement and overlap resolution.

Functions: `from_value`, `collect`, `update_targets`, `target`, `reachable`, `is_visible`, `front_index`, `side_room`, `from_value`, `approximately`, `step`, `target`, `resolve_overlaps`, `lane_continuity_preserves_blocked_time_and_skips_direction_changes`, `row`, `indices`, `fixture`, `cached_and_live_passing_search_matches_python_bits`, `overlap_order_and_flags_match_python_on_small_and_large_fields`, `lateral_acceleration_and_permissions_match_python_bits`, `target_state_retention_cooldown_and_shared_draws_match_python`.

## [retail_physics.rs](../src/retail_physics.rs)

Speed, acceleration, HP and numerical integration helpers.

Functions: `phase_boundaries`, `exported_thresholds_and_adjacent_float`, `target_speed_indices`, `advance_start_delay`, `finish_time`, `group`, `number`, `numbers`, `phases`, `from_value`, `course_speed`, `base_target_speed`, `start_dash_speed`, `minimum_speed`, `blocking_speed_cap`, `maximum_spurt_speed`, `target_speed`, `target_speed_with_late_terms`, `initial_hp`, `hp_state_rate`, `hp_per_second`, `acceleration`, `bounded_stat`, `root`, `apply_additive_modifiers`, `values`, `values_except_tokens`, `values_except`, `value`, `register`, `remove`, `apply`, `zero_time_suppresses_ai_without_changing_countdown`, `gate_countdown_exact_zero_and_repeated_subtraction`, `parameters`, `dash_threshold_uses_captured_float32_rate`, `python_port_vectors_match_every_float32_bit`, `receiver_keeps_order_and_recomputes_after_expiry`, `missing_parameters_are_rejected_instead_of_defaulted`, `finish_interpolation_matches_official_adjacent_crossings`.

## [retail_position_keep.rs](../src/retail_position_keep.rs)

Pacing states, thresholds and transitions.

Functions: `start_section`, `end_section`, `from_value`, `thresholds`, `default`, `base_style`, `clear`, `choose`, `update`, `sequential_field_contexts_match_python_timers_draws_and_modes`.

## [retail_random.rs](../src/retail_random.rs)

Shared random stream for the detailed runtime.

Functions: `integer`, `real`, `integer`, `real`, `new`, `next_int`, `next_double`, `integer`, `real_distance`, `real`, `independent_dotnet_reference_all_signed_seed_boundaries`, `zero_width_still_consumes_a_draw`.

## [retail_sort.rs](../src/retail_sort.rs)

Comparison sorting and tie behavior.

Functions: `comparison_sort`, `swap_if_greater`, `down_heap`, `sort`, `comparison_sort_port_matches_python_partition_vectors`.

## [retail_spurt.rs](../src/retail_spurt.rs)

Last-spurt feasibility search and planning.

Functions: `default`, `sort_candidates`, `choose`, `default`, `update`, `search_matches_python_plans_and_shared_draw_counts`, `recovery_retry_preserves_a_valid_start_and_failed_retry_keeps_plan`, `real_first_pass_retry_limit_rejects_a_distant_feasible_speed_without_rolls`, `comparator_near_ties_and_large_equal_partitions_keep_framework_behavior`, `second_pass_lottery_is_strict_and_has_its_own_retry_limit`, `integer`, `real`.

## [retail_states.rs](../src/retail_states.rs)

Rushing, downhill and related runtime states.

Functions: `number`, `from_value`, `new`, `choose_mode`, `set_forced_end`, `step`, `from_value`, `update`, `new`, `take`, `integer`, `real`, `parameters`, `rushing_boundaries_consumption_and_single_use`, `rushing_float_tick_and_forced_timer_replacement`, `downhill_zero_crossing_and_level_terrain_freeze`, `shared_stream_matches_python_state_transitions_and_float_timers`.

## [rushing.rs](../src/rushing.rs)

Rushing-mode selection and pacing strategy.

Functions: `select_mode`, `pacing_style`, `extracted_mode_lottery_boundaries_and_original_strategy`.

## [stadium_score.rs](../src/stadium_score.rs)

Positioning and rushing observations.

Functions: `style_leaders`, `new`, `update`, `new`, `update`, `leaders_use_exact_styles_and_cached_ranks`, `positioning_is_checked_only_at_phase_change`, `countdown_pauses_and_discards_overshoot`.

## [starting_grid.rs](../src/starting_grid.rs)

Starting-stall coordinates and grid configuration.

Functions: `starting_lane`, `retail_starting_lane`, `lane`, `track_specific_dividers_and_half_gate`.
