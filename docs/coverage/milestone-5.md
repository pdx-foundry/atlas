# Milestone 5 capture: 2026-10-13 (SDK-798)

The Milestone 5 sweep is one live run on M452 (Stellaris 4.5.2) at Native `ee01bf3`, with the
method set unchanged during the run. It is a baseline (decision D2): repairs that landed before
`ee01bf3` are in it, and the rest are [labelled deficits](#labelled-deficits).

- **D1 gate.** The 218 config types under `common/` and `events/` at config `8574760` have
  **4,316 / 18,228 (23.68%)** Atlas-owned claims covered. 179 have a Native registry; the other 39
  have the missing-join reason of the [registry comparison](registries.md). In `events/`,
  `type[event]` has 34 / 533 and `type[event_namespace]` 3 / 6; [each of the 506 exceptions](#events)
  has a reason.
- **Headline.** **24,458 / 58,032 (42.145713%)** Atlas-owned config claims, the same as the
  [pin move A capture](pin-move-a.md): the run is at the same Native commit, and its snapshot is
  byte-identical.
- **Native rates (SDK-780).** Over the 185 listed registries and their 1,855 root fields,
  3,985 of 7,511 relationship units (53.06%) are complete. By relationship: fields 52.97%, reader
  kinds 76.12%, shapes and conditions 58.92%, references 5.76%, numerics 34.45%, weights 0%, naming
  rules 0% and scope context 62.10% ([rates](#native-rates-sdk-780)).

Coverage comes from the live snapshot. The recording repeats it without a game; recorded answers
earn no coverage.

## The recorded commit

Native `main` was `ee01bf338872290eaaab2ca3c0503fc33244686e` before and after the run, the
commit that `Cargo.lock` pins (SDK-796). The pin did not move, so the fixture trim, the baseline
refresh and the SDK-781 rerun of [pin move A](pin-move-a.md) stand: the run's `ledger.json`,
`coverage.json`, `comparison.json`, `registries.json` and `covered-claims.txt` are byte-identical
to that capture's.

The run used no local path override: `cargo tree -p pdx-native` showed the Git revision. A
checkout inside the main Atlas checkout reads the main checkout's ignored `.cargo/config.toml`,
so the run was made from a checkout outside it.

Native stamps no commit on an answer. Each answer of the recording carries its method:

| Operation | Method | Answers |
| --- | --- | ---: |
| `registries` | `registry-directories/v6` | 1 |
| `registry_fields` | `registry-fields/v33` | 179 |
| `derived_names` | `derived-names/v1` | 179 |
| `modifier_families` | `modifier-families/v3` | 179 |
| `loader_rules` | `loader-rules/v5` | 185 |
| `declarations` | `command-declarations/v3` | 2 |
| `command_grammar` | `command-grammar/v18` | 2,178 |
| `on_actions`, `game_rules` | `callbacks/v4` | 2 |
| `observe_fixture` | `observe-fixture/v9` | 2 |
| loaded modifiers | `loaded-modifiers/v1` | 1 |
| `modifiers` | `modifier-declarations/v1` | 1 |
| `modifier_categories` | `modifier-categories/v1` | 1 |
| `scopes` | `scope-declarations/v1` | 1 |
| `scope_links` | `scope-links/v2` | 1 |
| `localization_declarations` | `localization-declarations/v1` | 1 |
| `defines` | `defines/v1` | 1 |

The other 6 of the 185 registries answer `registry_fields`, `derived_names` and
`modifier_families` with `Unsupported` ([pin move A](pin-move-a.md#unsupported-registry-answers)).

## D1 gate

The gate population is every config type whose `path` is under `common/` or `events/` at config
`8574760`: 218 types (the [registry comparison](registries.md#gate-population-at-config-8574760)
says why 218 and not 216). A type's rate is its Atlas-owned claims that the live snapshot covers,
from `coverage.json` `by_type`.

By registry join:

| Join | Types | Covered | Atlas-owned | Share |
| --- | ---: | ---: | ---: | ---: |
| `named` | 159 | 3,875 | 13,519 | 28.66% |
| `no_named_registry` | 39 | 0 | 2,260 | 0.00% |
| `several_types_in_directory` | 20 | 441 | 2,449 | 18.01% |

A `no_named_registry` type has no Native registry, so no claim of it is covered. Its reason is the
join: 18 directories, listed in [the rerun at `ee01bf3`](registries.md#rerun-at-native-ee01bf3-sdk-796).
A `several_types_in_directory` type is credited only through its config selector.

By share:

| Share | Types |
| --- | ---: |
| 0% | 43 |
| under 25% | 48 |
| 25–50% | 74 |
| 50–75% | 50 |
| 75–100% | 2 |
| 100% | 1 |
| no Atlas-owned claim | 0 |

### Events

`events` is one Native registry, with one root field per event kind (SDK-702); `type[event]` and
`type[event_namespace]` load from it (`several_types_in_directory`).

| Type | File | Covered | Atlas-owned | Share |
| --- | --- | ---: | ---: | ---: |
| `type[event]` | `events/events.cwt` | 34 | 533 | 6.38% |
| `type[event_namespace]` | `events/event_namespaces.cwt` | 3 | 6 | 50.00% |

The two files have 543 Atlas-owned claims: the two types and 4 `enum[event_window_type]` items
outside them. 37 are covered: the read scopes of the 21 kinds, the `pre_triggers` and
`difficulty` fields of four kinds (9; SDK-793), six loader lines of the two types (SDK-797) and
`type[event]`'s existence. Each of the other 506 is an exception with a reason:

| Exception | Claims | Properties | Deficit |
| --- | ---: | --- | --- |
| Field-predicate subtype (`!hidden`) | 270 | `value_form` 69, `field_existence` 69, `cardinality_minimum` 63, `cardinality_maximum` 63, `naming_rule` 4, `naming_requirement` 2 | SDK-716 |
| `CEvent` member has no reader identity | 78 | `field_existence` 20, `value_form` 20, `cardinality_minimum` 19, `cardinality_maximum` 19 | SDK-710 |
| Field-predicate subtype (`inherited`) | 23 | `field_existence` 6, `value_form` 6, `cardinality_maximum` 5, `cardinality_minimum` 5, `conditional_constraint` 1 | SDK-716 |
| Field-predicate subtype (`!hidden`, `diplomatic`) | 22 | `value_form` 6, `field_existence` 6, `cardinality_minimum` 5, `cardinality_maximum` 5 | SDK-716 |
| Field-predicate subtype (`hidden`) | 21 | `cardinality_maximum` 5, `value_form` 5, `cardinality_minimum` 5, `field_existence` 5, `conditional_constraint` 1 | SDK-716 |
| Field-predicate subtype (`!triggered`) | 18 | `cardinality_maximum` 5, `cardinality_minimum` 5, `field_existence` 4, `value_form` 4 | SDK-716 |
| Member of a kind subtype: `CEvent` member has no reader | 16 | `cardinality_maximum` 4, `value_form` 4, `cardinality_minimum` 4, `field_existence` 4 | SDK-710 |
| Field-predicate subtype (`!diplomatic`, `!hidden`) | 14 | `value_form` 4, `field_existence` 4, `cardinality_maximum` 3, `cardinality_minimum` 3 | SDK-716 |
| Field-predicate subtype (`diplomatic`) | 13 | `cardinality_maximum` 3, `cardinality_minimum` 3, `field_existence` 3, `value_form` 3, `conditional_constraint` 1 | SDK-716 |
| Field-predicate subtype (`triggered`) | 9 | `cardinality_minimum` 2, `cardinality_maximum` 2, `field_existence` 2, `value_form` 2, `conditional_constraint` 1 | SDK-716 |
| Recorded gap: occurrence of `pre_triggers` or `difficulty` in a kind (repeat behavior or requirement not established) | 7 | `cardinality_minimum` 4, `cardinality_maximum` 3 | SDK-627 |
| Field-predicate subtype (`major`) | 5 | `cardinality_maximum` 1, `cardinality_minimum` 1, `conditional_constraint` 1, `field_existence` 1, `value_form` 1 | SDK-716 |
| `enum[event_window_type]` items: no enum domain | 4 | `enum_member` 4 | SDK-627 |
| `event_namespace`: `namespace` is not a field and no registry names the type | 3 | `field_existence` 1, `value_form` 1, `type_existence` 1 | none (SDK-553 files it) |
| Root `event = {` outside any kind | 2 | `value_form` 1, `field_existence` 1 | none (SDK-553 files it) |
| Field-predicate subtype (`mean_time_to_happen`) | 1 | `conditional_constraint` 1 | SDK-716 |
| **Total** | **506** | | |

A field-predicate subtype (`hidden`, `triggered`, `diplomatic`, and the others) tests a field, not
the root key, so the `type_key_filter` rule of the [ledger contract](ledger.md#snapshot-input)
does not credit it; SDK-716 owns those claims. The `CEvent` members (`id`, `trigger`, `immediate`,
`option` and the others) are fields of every kind, and none has one shared reader:
`registry_fields("events")` keeps an `UnresolvedReader` gap on each.

### Every gate type

| Type | File | Directory | Join | Covered | Atlas-owned | Share |
| --- | --- | --- | --- | ---: | ---: | ---: |
| `type[achievement]` | `common/zzz_common_folder_elements.cwt` | `common` | `no_named_registry` | 0 | 15 | 0.00% |
| `type[alert]` | `common/zzz_common_folder_elements.cwt` | `common` | `no_named_registry` | 0 | 43 | 0.00% |
| `type[alert_icon]` | `common/zzz_common_folder_elements.cwt` | `common` | `no_named_registry` | 0 | 7 | 0.00% |
| `type[agreement_preset]` | `common/agreements.cwt` | `common/agreement_presets` | `named` | 20 | 102 | 19.61% |
| `type[agreement_resource]` | `common/agreements.cwt` | `common/agreement_resources` | `named` | 4 | 14 | 28.57% |
| `type[agreement_term_value]` | `common/agreements.cwt` | `common/agreement_term_values` | `named` | 47 | 161 | 29.19% |
| `type[agreement_term]` | `common/agreements.cwt` | `common/agreement_terms` | `named` | 6 | 24 | 25.00% |
| `type[ai_budget]` | `common/ai_budget.cwt` | `common/ai_budget` | `named` | 16 | 31 | 51.61% |
| `type[espionage_operation_policy]` | `common/ai_espionage.cwt` | `common/ai_espionage/operations` | `named` | 3 | 8 | 37.50% |
| `type[espionage_spynetwork]` | `common/ai_espionage.cwt` | `common/ai_espionage/spynetworks` | `named` | 7 | 23 | 30.43% |
| `type[ai_force_species]` | `common/ai_force_species.cwt` | `common/ai_force_species` | `no_named_registry` | 0 | 16 | 0.00% |
| `type[ambient_object]` | `common/ambient_objects.cwt` | `common/ambient_objects` | `named` | 14 | 37 | 37.84% |
| `type[anomaly_category]` | `common/anomalies.cwt` | `common/anomalies` | `named` | 37 | 108 | 34.26% |
| `type[archaeological_site_type]` | `common/archaeology.cwt` | `common/archaeological_site_types` | `named` | 35 | 104 | 33.65% |
| `type[army]` | `common/armies.cwt` | `common/armies` | `named` | 82 | 149 | 55.03% |
| `type[artifact_action]` | `common/archaeology.cwt` | `common/artifact_actions` | `named` | 19 | 34 | 55.88% |
| `type[ascension_perk_category]` | `common/ascension_perk_categories.cwt` | `common/ascension_perk_categories` | `named` | 5 | 12 | 41.67% |
| `type[ascension_perk]` | `common/ascension_perks.cwt` | `common/ascension_perks` | `several_types_in_directory` | 61 | 96 | 63.54% |
| `type[swapped_ascension_perk]` | `common/ascension_perks.cwt` | `common/ascension_perks` | `several_types_in_directory` | 5 | 20 | 25.00% |
| `type[asteroid_belt_type]` | `common/asteroid_belts.cwt` | `common/asteroid_belts` | `named` | 13 | 21 | 61.90% |
| `type[astral_action]` | `common/astral_actions.cwt` | `common/astral_actions` | `named` | 42 | 87 | 48.28% |
| `type[astral_rift]` | `common/astral_rifts.cwt` | `common/astral_rifts` | `named` | 16 | 32 | 50.00% |
| `type[attitude]` | `common/attributes.cwt` | `common/attitudes` | `named` | 24 | 34 | 70.59% |
| `type[bombardment_stance]` | `common/bombardment_stances.cwt` | `common/bombardment_stances` | `named` | 40 | 69 | 57.97% |
| `type[building]` | `common/buildings.cwt` | `common/buildings` | `named` | 130 | 343 | 37.90% |
| `type[button_effect]` | `common/button_effects.cwt` | `common/button_effects` | `named` | 10 | 16 | 62.50% |
| `type[bypass]` | `common/bypass.cwt` | `common/bypass` | `named` | 57 | 118 | 48.31% |
| `type[casus_belli]` | `common/casus_belli_and_war_goals.cwt` | `common/casus_belli` | `named` | 24 | 52 | 46.15% |
| `type[cloaking_strength_level]` | `common/cloaking_strength_levels.cwt` | `common/cloaking_strength_levels` | `named` | 11 | 21 | 52.38% |
| `type[colony_automation]` | `common/colony.cwt` | `common/colony_automation` | `named` | 10 | 41 | 24.39% |
| `type[colony_automation_category]` | `common/colony.cwt` | `common/colony_automation_categories` | `named` | 6 | 10 | 60.00% |
| `type[colony_automation_exception]` | `common/colony.cwt` | `common/colony_automation_exceptions` | `named` | 9 | 68 | 13.24% |
| `type[colony_type]` | `common/colony.cwt` | `common/colony_types` | `named` | 21 | 62 | 33.87% |
| `type[country_color]` | `common/colors.cwt` | `common/colors` | `named` | 3 | 6 | 50.00% |
| `type[component_set]` | `common/components.cwt` | `common/component_sets` | `no_named_registry` | 0 | 59 | 0.00% |
| `type[component_slot_template]` | `common/components.cwt` | `common/component_slot_templates` | `named` | 3 | 23 | 13.04% |
| `type[component_template]` | `common/components.cwt` | `common/component_templates` | `no_named_registry` | 0 | 600 | 0.00% |
| `type[agenda]` | `common/council_agendas.cwt` | `common/council_agendas` | `named` | 34 | 49 | 69.39% |
| `type[country_container]` | `common/country_container.cwt` | `common/country_container` | `named` | 4 | 10 | 40.00% |
| `type[country_customization]` | `common/country_customization.cwt` | `common/country_customization` | `named` | 11 | 48 | 22.92% |
| `type[country_card_categories]` | `common/country_focus.cwt` | `common/country_focus/card_categories` | `named` | 23 | 63 | 36.51% |
| `type[country_focus_cards]` | `common/country_focus.cwt` | `common/country_focus/focus_cards` | `named` | 3 | 50 | 6.00% |
| `type[country_focus_rewards]` | `common/country_focus.cwt` | `common/country_focus/focus_rewards` | `named` | 9 | 18 | 50.00% |
| `type[country_ownership_limit]` | `common/country_limits.cwt` | `common/country_limits/ownership_limits` | `named` | 4 | 10 | 40.00% |
| `type[country_ship_of_size_limit]` | `common/country_limits.cwt` | `common/country_limits/ship_of_size_limits` | `named` | 14 | 27 | 51.85% |
| `type[country_type]` | `common/country_types.cwt` | `common/country_types` | `named` | 181 | 628 | 28.82% |
| `type[crisis_level]` | `common/crisis.cwt` | `common/crisis_levels` | `named` | 11 | 24 | 45.83% |
| `type[crisis_objective]` | `common/crisis.cwt` | `common/crisis_objectives` | `named` | 13 | 18 | 72.22% |
| `type[crisis_path]` | `common/crisis_paths.cwt` | `common/crisis_paths` | `named` | 7 | 19 | 36.84% |
| `type[decision]` | `common/decisions.cwt` | `common/decisions` | `named` | 49 | 115 | 42.61% |
| `type[deposit_category]` | `common/deposits_and_planetary_features.cwt` | `common/deposit_categories` | `named` | 9 | 16 | 56.25% |
| `type[deposit]` | `common/deposits_and_planetary_features.cwt` | `common/deposits` | `named` | 33 | 152 | 21.71% |
| `type[diplo_phrase]` | `common/diplo_phrases.cwt` | `common/diplo_phrases` | `several_types_in_directory` | 3 | 24 | 12.50% |
| `type[diplo_phrase_greetings]` | `common/diplo_phrases.cwt` | `common/diplo_phrases` | `several_types_in_directory` | 0 | 14 | 0.00% |
| `type[diplomacy_economy]` | `common/diplomacy_economy.cwt` | `common/diplomacy_economy` | `named` | 5 | 37 | 13.51% |
| `type[diplomatic_action]` | `common/diplomatic_actions.cwt` | `common/diplomatic_actions` | `named` | 75 | 187 | 40.11% |
| `type[district]` | `common/districts.cwt` | `common/districts` | `named` | 6 | 251 | 2.39% |
| `type[dust_clouds]` | `common/dust_clouds.cwt` | `common/dust_clouds` | `named` | 62 | 80 | 77.50% |
| `type[economic_category]` | `common/economic_categories.cwt` | `common/economic_categories` | `named` | 22 | 72 | 30.56% |
| `type[economic_plan]` | `common/economic_plans.cwt` | `common/economic_plans` | `named` | 3 | 64 | 4.69% |
| `type[edict]` | `common/edicts.cwt` | `common/edicts` | `named` | 43 | 87 | 49.43% |
| `type[espionage_asset]` | `common/espionage.cwt` | `common/espionage_assets` | `named` | 18 | 36 | 50.00% |
| `type[espionage_category]` | `common/espionage.cwt` | `common/espionage_operation_categories` | `named` | 3 | 8 | 37.50% |
| `type[espionage_operation]` | `common/espionage.cwt` | `common/espionage_operation_types` | `named` | 19 | 167 | 11.38% |
| `type[ethic_category]` | `common/ethics.cwt` | `common/ethic_categories` | `named` | 3 | 5 | 60.00% |
| `type[ethic]` | `common/ethics.cwt` | `common/ethics` | `named` | 47 | 93 | 50.54% |
| `type[event_chain]` | `common/event_chains.cwt` | `common/event_chains` | `named` | 17 | 34 | 50.00% |
| `type[fallen_empires]` | `common/fallen_empires.cwt` | `common/fallen_empires` | `named` | 16 | 24 | 66.67% |
| `type[federation_law_category]` | `common/federations.cwt` | `common/federation_law_categories` | `named` | 14 | 25 | 56.00% |
| `type[federation_law]` | `common/federations.cwt` | `common/federation_laws` | `named` | 27 | 46 | 58.70% |
| `type[federation_perk]` | `common/federations.cwt` | `common/federation_perks` | `named` | 26 | 58 | 44.83% |
| `type[federation_type]` | `common/federations.cwt` | `common/federation_types` | `named` | 25 | 46 | 54.35% |
| `type[first_contact_stage]` | `common/first_cntact.cwt` | `common/first_contact` | `named` | 12 | 45 | 26.67% |
| `type[rontend_background]` | `common/frontend_backgrounds.cwt` | `common/frontend_backgrounds` | `named` | 7 | 12 | 58.33% |
| `type[galactic_community_action]` | `common/galactic_community_actions.cwt` | `common/galactic_community_actions` | `named` | 4 | 13 | 30.77% |
| `type[galactic_focus]` | `common/galactic_community.cwt` | `common/galactic_focuses` | `named` | 9 | 29 | 31.03% |
| `type[game_concepts_categories]` | `common/game_concepts_categories.cwt` | `common/game_concept_categories` | `named` | 5 | 12 | 41.67% |
| `type[game_concept]` | `common/game_concepts.cwt` | `common/game_concepts` | `named` | 15 | 58 | 25.86% |
| `type[game_rule]` | `common/game_rules.cwt` | `common/game_rules` | `no_named_registry` | 0 | 5 | 0.00% |
| `type[gamesetup_setting_category]` | `common/gamesetup_setting.cwt` | `common/gamesetup_settings` | `no_named_registry` | 0 | 32 | 0.00% |
| `type[global_ship_design]` | `common/global_ship_designs.cwt` | `common/global_ship_designs` | `named` | 28 | 146 | 19.18% |
| `type[government]` | `common/governments.cwt` | `common/governments` | `named` | 27 | 51 | 52.94% |
| `type[authority]` | `common/governments.cwt` | `common/governments/authorities` | `several_types_in_directory` | 102 | 260 | 39.23% |
| `type[swapped_authority]` | `common/governments.cwt` | `common/governments/authorities` | `several_types_in_directory` | 5 | 22 | 22.73% |
| `type[civic_or_origin]` | `common/governments.cwt` | `common/governments/civics` | `several_types_in_directory` | 44 | 354 | 12.43% |
| `type[swapped_civic]` | `common/governments.cwt` | `common/governments/civics` | `several_types_in_directory` | 3 | 15 | 20.00% |
| `type[councilor]` | `common/governments.cwt` | `common/governments/councilors` | `named` | 43 | 119 | 36.13% |
| `type[graphical_culture]` | `common/graphical_cultures.cwt` | `common/graphical_culture` | `no_named_registry` | 0 | 109 | 0.00% |
| `type[greeting_overlay_sound]` | `common/greeting_overlay_sounds.cwt` | `common/greeting_overlay_sounds` | `named` | 8 | 12 | 66.67% |
| `type[intel_category]` | `common/intel.cwt` | `common/intel_categories` | `named` | 4 | 12 | 33.33% |
| `type[intel_level]` | `common/intel.cwt` | `common/intel_levels` | `named` | 6 | 16 | 37.50% |
| `type[lawsuit]` | `common/lawsuits.cwt` | `common/lawsuit_type` | `no_named_registry` | 0 | 33 | 0.00% |
| `type[leader_class]` | `common/leader_classes.cwt` | `common/leader_classes` | `named` | 69 | 175 | 39.43% |
| `type[leader_tier]` | `common/leader_tier.cwt` | `common/leader_tiers` | `named` | 7 | 11 | 63.64% |
| `type[mandate]` | `common/mandates.cwt` | `common/mandates` | `no_named_registry` | 0 | 40 | 0.00% |
| `type[map_mode]` | `common/map_modes.cwt` | `common/map_modes` | `named` | 23 | 94 | 24.47% |
| `type[megastructure_overclock_type]` | `common/megastructures.cwt` | `common/megastructure_overclock_types` | `named` | 43 | 82 | 52.44% |
| `type[megastructure]` | `common/megastructures.cwt` | `common/megastructures` | `named` | 195 | 403 | 48.39% |
| `type[menace_perk]` | `common/crisis.cwt` | `common/menace_perks` | `named` | 16 | 35 | 45.71% |
| `type[message_type]` | `common/message_types.cwt` | `common/message_types` | `no_named_registry` | 0 | 85 | 0.00% |
| `type[mission_category]` | `common/missions.cwt` | `common/missions/mission_categories` | `named` | 13 | 27 | 48.15% |
| `type[mission]` | `common/missions.cwt` | `common/missions/missions` | `named` | 77 | 248 | 31.05% |
| `type[mutations]` | `common/mutations.cwt` | `common/mutations` | `named` | 19 | 78 | 24.36% |
| `type[name_list]` | `common/names_consolidated.cwt` | `common/name_lists` | `no_named_registry` | 0 | 479 | 0.00% |
| `type[named_color]` | `common/named_color.cwt` | `common/named_colors` | `named` | 4 | 11 | 36.36% |
| `type[notification_modifier]` | `common/modifiers.cwt` | `common/notification_modifiers` | `named` | 8 | 13 | 61.54% |
| `type[observation_station_mission]` | `common/observation_station_missions.cwt` | `common/observation_station_missions` | `named` | 23 | 44 | 52.27% |
| `type[on_action]` | `common/on_actions.cwt` | `common/on_actions` | `named` | 4 | 23 | 17.39% |
| `type[opinion_modifier]` | `common/modifiers.cwt` | `common/opinion_modifiers` | `named` | 8 | 82 | 9.76% |
| `type[patron_type]` | `common/patrons.cwt` | `common/patrons` | `named` | 30 | 225 | 13.33% |
| `type[calling]` | `common/patrons.cwt` | `common/patrons/callings` | `named` | 21 | 56 | 37.50% |
| `type[deed]` | `common/patrons.cwt` | `common/patrons/deeds` | `named` | 11 | 32 | 34.38% |
| `type[psionic_aura]` | `common/patrons.cwt` | `common/patrons/psionic_auras` | `named` | 69 | 143 | 48.25% |
| `type[ai_personality]` | `common/personalities.cwt` | `common/personalities` | `named` | 157 | 271 | 57.93% |
| `type[planet_class]` | `common/planet_classes.cwt` | `common/planet_classes` | `several_types_in_directory` | 3 | 375 | 0.80% |
| `type[planet_class_random_list]` | `common/planet_classes.cwt` | `common/planet_classes` | `several_types_in_directory` | 0 | 12 | 0.00% |
| `type[planet_modifier]` | `common/modifiers.cwt` | `common/planet_modifiers` | `named` | 11 | 18 | 61.11% |
| `type[policy]` | `common/policies.cwt` | `common/policies` | `named` | 19 | 92 | 20.65% |
| `type[policy_category]` | `common/policy_categories.cwt` | `common/policy_categories` | `named` | 9 | 17 | 52.94% |
| `type[pop_category]` | `common/pop_categories.cwt` | `common/pop_categories` | `named` | 42 | 128 | 32.81% |
| `type[pop_faction]` | `common/pop_faction_types.cwt` | `common/pop_faction_types` | `named` | 61 | 229 | 26.64% |
| `type[job]` | `common/pop_jobs.cwt` | `common/pop_jobs` | `several_types_in_directory` | 61 | 266 | 22.93% |
| `type[swapped_job]` | `common/pop_jobs.cwt` | `common/pop_jobs` | `several_types_in_directory` | 0 | 21 | 0.00% |
| `type[portrait_categories]` | `common/portrait_categories.cwt` | `common/portrait_categories` | `named` | 8 | 16 | 50.00% |
| `type[portrait_sets]` | `common/portrait_set.cwt` | `common/portrait_sets` | `named` | 9 | 53 | 16.98% |
| `type[precursors]` | `common/precursor_civilizations.cwt` | `common/precursor_civilizations` | `named` | 10 | 18 | 55.56% |
| `type[prescripted_flags]` | `common/prescripted_flags.cwt` | `common/prescripted_flags` | `named` | 4 | 10 | 40.00% |
| `type[empire_name_format]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 28 | 0.00% |
| `type[empire_name_parts_list]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 24 | 0.00% |
| `type[federation_name_format]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 14 | 0.00% |
| `type[federation_name_parts_list]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 24 | 0.00% |
| `type[pop_faction_name_format]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 13 | 0.00% |
| `type[pop_faction_name_parts_list]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 23 | 0.00% |
| `type[pre_communications_name_format]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 13 | 0.00% |
| `type[pre_communications_name_parts_list]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 23 | 0.00% |
| `type[war_name_format]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 13 | 0.00% |
| `type[war_name_parts_list]` | `common/random_names.cwt` | `common/random_names` | `no_named_registry` | 0 | 23 | 0.00% |
| `type[asteroid_postfix]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 11 | 0.00% |
| `type[asteroid_prefix]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[black_hole_names]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[cosmic_storm_name_prefixes]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[custom_planet_names]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 12 | 0.00% |
| `type[custom_star_names]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 12 | 0.00% |
| `type[nebula_names]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[species_modification_postfix]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[species_modification_prefix]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[star_names]` | `common/random_names.cwt` | `common/random_names/base` | `no_named_registry` | 0 | 8 | 0.00% |
| `type[relic]` | `common/archaeology.cwt` | `common/relics` | `named` | 31 | 52 | 59.62% |
| `type[resolution_category]` | `common/galactic_community.cwt` | `common/resolution_categories` | `named` | 9 | 21 | 42.86% |
| `type[resolution_group]` | `common/galactic_community.cwt` | `common/resolution_groups` | `named` | 6 | 9 | 66.67% |
| `type[resolution]` | `common/galactic_community.cwt` | `common/resolutions` | `named` | 29 | 81 | 35.80% |
| `type[resource_converter]` | `common/resource_converters.cwt` | `common/resource_converters` | `named` | 3 | 78 | 3.85% |
| `type[resource_region]` | `common/resource_regions.cwt` | `common/resource_regions` | `named` | 3 | 3 | 100.00% |
| `type[script_value]` | `common/script_values.cwt` | `common/script_values` | `named` | 3 | 5 | 60.00% |
| `type[scripted_action]` | `common/scripted_actions.cwt` | `common/scripted_actions` | `named` | 3 | 545 | 0.55% |
| `type[scripted_effect]` | `common/scripted_effects_and_triggers.cwt` | `common/scripted_effects` | `named` | 3 | 10 | 30.00% |
| `type[scripted_loc]` | `common/scripted_loc.cwt` | `common/scripted_loc` | `no_named_registry` | 0 | 50 | 0.00% |
| `type[scripted_modifier]` | `common/scripted_modifiers.cwt` | `common/scripted_modifiers` | `named` | 25 | 61 | 40.98% |
| `type[scripted_trigger]` | `common/scripted_effects_and_triggers.cwt` | `common/scripted_triggers` | `named` | 3 | 10 | 30.00% |
| `type[section_template]` | `common/section_templates.cwt` | `common/section_templates` | `no_named_registry` | 0 | 124 | 0.00% |
| `type[sector_focus]` | `common/sector.cwt` | `common/sector_focuses` | `no_named_registry` | 0 | 47 | 0.00% |
| `type[sector_type]` | `common/sector.cwt` | `common/sector_types` | `named` | 10 | 27 | 37.04% |
| `type[ship_behavior]` | `common/ship_behaviors.cwt` | `common/ship_behaviors` | `named` | 4 | 94 | 4.26% |
| `type[ship_categories]` | `common/ship_categories.cwt` | `common/ship_categories` | `named` | 15 | 45 | 33.33% |
| `type[ship_set]` | `common/ship_sets.cwt` | `common/ship_sets` | `named` | 9 | 13 | 69.23% |
| `type[ship_size]` | `common/ship_sizes.cwt` | `common/ship_sizes` | `named` | 181 | 701 | 25.82% |
| `type[situation_log_category]` | `common/situation_logs.cwt` | `common/situation_log/categories` | `named` | 16 | 36 | 44.44% |
| `type[situation_type]` | `common/situations.cwt` | `common/situations` | `named` | 91 | 379 | 24.01% |
| `type[solar_system_initializer]` | `common/solar_system_initializers.cwt` | `common/solar_system_initializers` | `no_named_registry` | 0 | 198 | 0.00% |
| `type[solar_system_initializer_random_list]` | `common/solar_system_initializers.cwt` | `common/solar_system_initializers` | `no_named_registry` | 0 | 11 | 0.00% |
| `type[special_project]` | `common/special_projects.cwt` | `common/special_projects` | `named` | 90 | 244 | 36.89% |
| `type[specialist_subject_perk]` | `common/specialist_subject_perks.cwt` | `common/specialist_subject_perks` | `named` | 17 | 56 | 30.36% |
| `type[specialist_subject_type]` | `common/specialist_subject_types.cwt` | `common/specialist_subject_types` | `named` | 16 | 40 | 40.00% |
| `type[species_archetype]` | `common/species_consolidated.cwt` | `common/species_archetypes` | `named` | 19 | 47 | 40.43% |
| `type[species_class]` | `common/species_consolidated.cwt` | `common/species_classes` | `named` | 42 | 249 | 16.87% |
| `type[species_name]` | `common/names_consolidated.cwt` | `common/species_names` | `several_types_in_directory` | 3 | 34 | 8.82% |
| `type[species_named_list]` | `common/names_consolidated.cwt` | `common/species_names` | `several_types_in_directory` | 0 | 18 | 0.00% |
| `type[citizenship_type]` | `common/species_consolidated.cwt` | `common/species_rights/citizenship_types` | `named` | 24 | 96 | 25.00% |
| `type[colonization_control]` | `common/species_consolidated.cwt` | `common/species_rights/colonization_controls` | `named` | 6 | 72 | 8.33% |
| `type[living_standard]` | `common/species_consolidated.cwt` | `common/species_rights/living_standards` | `named` | 3 | 68 | 4.41% |
| `type[migration_control]` | `common/species_consolidated.cwt` | `common/species_rights/migration_controls` | `named` | 6 | 72 | 8.33% |
| `type[military_service_type]` | `common/species_consolidated.cwt` | `common/species_rights/military_service_types` | `named` | 3 | 68 | 4.41% |
| `type[population_control]` | `common/species_consolidated.cwt` | `common/species_rights/population_controls` | `named` | 6 | 72 | 8.33% |
| `type[purge_type]` | `common/species_consolidated.cwt` | `common/species_rights/purge_types` | `named` | 12 | 77 | 15.58% |
| `type[slavery_type]` | `common/species_consolidated.cwt` | `common/species_rights/slavery_types` | `named` | 11 | 73 | 15.07% |
| `type[subspecies_integration_type]` | `common/species_consolidated.cwt` | `common/species_rights/subspecies_integration_types` | `named` | 6 | 24 | 25.00% |
| `type[specimen]` | `common/specimens.cwt` | `common/specimens` | `named` | 17 | 43 | 39.53% |
| `type[star_class]` | `common/star_classes.cwt` | `common/star_classes` | `named` | 28 | 72 | 38.89% |
| `type[star_class_random_list]` | `common/star_classes.cwt` | `common/star_classes/randomizers` | `named` | 5 | 10 | 50.00% |
| `type[starbase_building]` | `common/starbases_consolidated.cwt` | `common/starbase_buildings` | `named` | 11 | 234 | 4.70% |
| `type[starbase_level]` | `common/starbases_consolidated.cwt` | `common/starbase_levels` | `named` | 44 | 125 | 35.20% |
| `type[starbase_module]` | `common/starbases_consolidated.cwt` | `common/starbase_modules` | `named` | 13 | 318 | 4.09% |
| `type[starbase_type]` | `common/starbases_consolidated.cwt` | `common/starbase_types` | `named` | 22 | 48 | 45.83% |
| `type[start_screen_message]` | `common/start_screen_messenges.cwt` | `common/start_screen_messages` | `named` | 8 | 13 | 61.54% |
| `type[static_modifier]` | `common/modifiers.cwt` | `common/static_modifiers` | `named` | 5 | 45 | 11.11% |
| `type[storm_types]` | `common/storm_types.cwt` | `common/storm_types` | `named` | 70 | 141 | 49.65% |
| `type[resource]` | `common/strategic_resources.cwt` | `common/strategic_resources` | `named` | 53 | 107 | 49.53% |
| `type[system_tooltip]` | `common/system_tooltips.cwt` | `common/system_tooltips` | `named` | 5 | 9 | 55.56% |
| `type[system_type]` | `common/system_types.cwt` | `common/system_types` | `named` | 11 | 14 | 78.57% |
| `type[target_type]` | `common/target_types.cwt` | `common/target_types` | `named` | 4 | 8 | 50.00% |
| `type[swapped_technology]` | `common/technologies_consolidated.cwt` | `common/technology` | `several_types_in_directory` | 5 | 22 | 22.73% |
| `type[technology]` | `common/technologies_consolidated.cwt` | `common/technology` | `several_types_in_directory` | 39 | 226 | 17.26% |
| `type[technology_category]` | `common/technologies_consolidated.cwt` | `common/technology/category` | `named` | 9 | 16 | 56.25% |
| `type[technology_tier]` | `common/technologies_consolidated.cwt` | `common/technology/tier` | `named` | 9 | 15 | 60.00% |
| `type[pre_ftl_age]` | `common/technology_ages.cwt` | `common/technology_ages/preftl` | `named` | 5 | 13 | 38.46% |
| `type[terraform_link]` | `common/terraform.cwt` | `common/terraform` | `named` | 24 | 51 | 47.06% |
| `type[timeline_events]` | `common/timeline_events.cwt` | `common/timeline_events` | `named` | 8 | 37 | 21.62% |
| `type[tradable_action]` | `common/tradable_actions.cwt` | `common/tradable_actions` | `named` | 18 | 46 | 39.13% |
| `type[trade_conversion]` | `common/trade_conversions.cwt` | `common/trade_conversions` | `no_named_registry` | 0 | 18 | 0.00% |
| `type[tradition_category]` | `common/traditions.cwt` | `common/tradition_categories` | `named` | 17 | 32 | 53.12% |
| `type[swapped_tradition]` | `common/traditions.cwt` | `common/traditions` | `several_types_in_directory` | 5 | 20 | 25.00% |
| `type[tradition]` | `common/traditions.cwt` | `common/traditions` | `several_types_in_directory` | 65 | 111 | 58.56% |
| `type[trait]` | `common/traits.cwt` | `common/traits` | `named` | 21 | 751 | 2.80% |
| `type[war_goal]` | `common/casus_belli_and_war_goals.cwt` | `common/war_goals` | `named` | 67 | 149 | 44.97% |
| `type[zone_slots]` | `common/zone_slots.cwt` | `common/zone_slots` | `named` | 3 | 50 | 6.00% |
| `type[zone]` | `common/zones.cwt` | `common/zones` | `named` | 5 | 208 | 2.40% |
| `type[event]` | `events/events.cwt` | `events` | `several_types_in_directory` | 34 | 533 | 6.38% |
| `type[event_namespace]` | `events/event_namespaces.cwt` | `events` | `several_types_in_directory` | 3 | 6 | 50.00% |
| **218 types** | | | | **4,316** | **18,228** | **23.68%** |

## Headline and per-area figures

Identical to [pin move A](pin-move-a.md#per-area-coverage):

| Area | Covered | Atlas-owned | Share |
| --- | ---: | ---: | ---: |
| Effects | 7,564 | 13,085 | 57.81% |
| Triggers | 4,847 | 6,708 | 72.26% |
| Modifiers | 1,559 | 2,243 | 69.51% |
| Scopes and links | 754 | 825 | 91.39% |
| Localisation | 626 | 922 | 67.90% |
| On_actions | 349 | 1,514 | 23.05% |
| Game rules | 421 | 884 | 47.62% |
| Defines | 3,904 | 5,570 | 70.09% |
| Registry types and other files | 4,434 | 26,281 | 16.87% |
| **Total** | **24,458** | **58,032** | **42.15%** |

The areas are those of the [Milestone 4 capture](milestone-4.md#per-area-coverage). The ledger is
unchanged: 175 files, 61,146 claim occurrences and 19 source diagnostics.

## Native rates (SDK-780)

Native's `examples/registry-sweep.rs` at `ee01bf3` reads the run's recording through
`Native::from_recorded_answers`, with no installation. The classification rules are on Native's
[method-authoring page](https://github.com/pdx-foundry/native/blob/ee01bf338872290eaaab2ca3c0503fc33244686e/docs/native/method-authoring.md#report-the-registry-sweep).
Two runs of the report over the recording gave byte-identical full reports (`relationships`,
`registries`, `readers`, `fields` and `failure_shapes`) and byte-identical `--baseline` output.
No answer is missing.

| Relationship | Unit | Complete | Partial | Failed | Undetermined | Unsupported | Denominator | Rate |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Fields | registry | 98 | 81 | 0 | 0 | 6 | 185 | 52.97% |
| Reader kinds | root field | 1,412 | 66 | 377 | 0 | 0 | 1,855 | 76.12% |
| Shapes and conditions | root field | 1,093 | 396 | 366 | 0 | 0 | 1,855 | 58.92% |
| References | root field | 24 | 11 | 0 | 382 | 0 | 417 | 5.76% |
| Numerics | root field | 206 | 10 | 0 | 382 | 0 | 598 | 34.45% |
| Weights | root field | 0 | 87 | 0 | 474 | 0 | 561 | 0.00% |
| Naming rules | registry | 0 | 179 | 0 | 0 | 6 | 185 | 0.00% |
| Scope context | root field | 1,152 | 255 | 448 | 0 | 0 | 1,855 | 62.10% |
| **All units** | | | | | | | **7,511** | **53.06%** |

"All units" sums the eight rows: 3,985 / 7,511 (53.06%). An undetermined field has reader kind
`Unknown`, so the method did not establish whether the relationship applies; it counts against
the rate. Two relationships have no complete unit:

- **Naming rules.** Every one of the 179 `modifier_families` answers keeps one
  `UnnamedDeclaration` gap: sites that compose modifier names at run time and are not joined to a
  registry, some of which may generate this registry's modifiers. So no registry's naming rules are
  complete.
- **Weights.** No weight field's `WeightBlock` is complete: 87 are partial and 474 undetermined.

Against Native's tracked baseline (`tests/population/m452/registry-sweep.json`, recorded at
`48e6d9e` over 164 registries), `--diff` lists 30 registries: the 21 that SDK-783 and SDK-786
added, and 9 whose statuses moved with later Native work (`common/pop_categories` fields became
complete; small moves in reader kinds, shapes, scope context, weights and references of
`common/agreement_term_values`, `common/country_customization`, `common/country_types`,
`common/espionage_assets`, `common/megastructures`, `common/pop_jobs`, `common/ship_sets` and
`common/ship_sizes`). Native's record of the sweep (SDK-553) updates that baseline.

### Failure shapes

Gaps that are not `OutsideMethod`, by operation and kind (260 distinct shapes):

| Operation | Gap kind | Gaps | Registries |
| --- | --- | ---: | ---: |
| `registry_fields` | `UnresolvedPath` | 1,838 | 163 |
| `registry_fields` | `UnresolvedStorage` | 1,625 | 119 |
| `registry_fields` | `UnresolvedReader` | 1,344 | 149 |
| `registry_fields` | `NumericConversion` | 387 | 59 |
| `derived_names` | `UnresolvedPath` | 328 | 116 |
| `derived_names` | `UnresolvedCondition` | 326 | 58 |
| `registry_fields` | `UnresolvedCondition` | 252 | 45 |
| `modifier_families` | `UnnamedDeclaration` | 179 | 179 |
| `modifier_families` | `UnresolvedPath` | 74 | 19 |
| `registry_fields` | `ReaderSemantics` | 16 | 9 |
| `registry_fields` | `UnnamedField` | 16 | 16 |
| `registry_fields` | `UnreadableInput` | 12 | 12 |

The largest shapes:

| Gaps | Registries | Shape |
| ---: | ---: | --- |
| 1,346 | 116 | registry_fields | UnresolvedPath: read-scope: scope-argument |
| 1,216 | 114 | registry_fields | UnresolvedStorage: Repeat behavior and nested fields remain unresolved. |
| 1,209 | 113 | registry_fields | UnresolvedReader: The field's alternatives do not establish one shared reader. |
| 399 | 47 | registry_fields | UnresolvedStorage: Repeat behavior remains unresolved. |
| 370 | 59 | registry_fields | NumericConversion: Numeric conversion is incomplete: a storage property, the accepted range or an engine conversion path is not established. |
| 207 | 34 | registry_fields | UnresolvedCondition: A loader condition remains unresolved; its outcome is retained separately. |
| 179 | 179 | modifier_families | UnnamedDeclaration: N sites that compose modifier names at run time are not joined to a registry (N inside the registration function; N reached from the post-read code of content objects that are not items of a named registry; N reached by no chain of calls from a registry's code); some may generate this registry's modifiers |
| 94 | 94 | derived_names | UnresolvedPath: N of the registry's methods have paths that go on after text that the method could not follow; their later lookups establish no condition or miss behavior |
| 90 | 82 | registry_fields | UnresolvedReader: The field's reader could not be established on at least one path. |
| 83 | 75 | registry_fields | UnresolvedPath: A path through the registry's reader could not be followed to its end. |
| 75 | 42 | registry_fields | UnresolvedPath: no evaluation that the method attributes evaluates this block |
| 72 | 20 | registry_fields | UnresolvedPath: read-scope: stored-scope |
| 53 | 27 | registry_fields | UnresolvedPath: a call site has more paths than the method follows (path-limit) |
| 50 | 50 | derived_names | UnresolvedPath: N lookup or check calls receive a name with a part that the method could not follow |
| 47 | 47 | derived_names | UnresolvedPath: N of the registry's methods have paths that could not be followed at path-limit |
| 45 | 45 | derived_names | UnresolvedPath: N of the registry's methods have paths that could not be followed at string-object |
| 45 | 14 | registry_fields | UnresolvedPath: Whether the field reads a modifier container, and its categories, are not established. |
| 45 | 18 | registry_fields | UnresolvedPath: some entry scopes of a call site could not be established |
| 42 | 42 | derived_names | UnresolvedPath: N lookup or check calls of the registry's methods were reached by no run |
| 32 | 28 | derived_names | UnresolvedCondition: the field values under which the engine uses {key} are not established |
| 32 | 25 | derived_names | UnresolvedCondition: the field values under which the engine uses {key}_desc are not established |
| 28 | 16 | registry_fields | UnresolvedPath: read-scope: zero-mask |
| 25 | 25 | derived_names | UnresolvedCondition: that the engine initializes every item after reading it is not established, so no name of the initialization is always used: a path of a database function keeps an item that it constructed without running its post-read code |
| 24 | 2 | registry_fields | UnresolvedCondition: The local use-time flag test is established; the enclosing selection context is unresolved. |
| 21 | 21 | registry_fields | UnresolvedCondition: Use-time analysis covers local Boolean selections only; enclosing context, other callers and bounded or unsupported paths remain unresolved. |

The six `Unsupported` registries are in [pin move A](pin-move-a.md#unsupported-registry-answers);
the report counts no other error.

### Per registry

`complete/denominator` for each field relationship; the registry relationships give their status.

| Registry | Fields | Naming | Reader kinds | Shapes | References | Numerics | Weights | Scope |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `common/agreement_presets` | complete | partial | 9/10 | 0/10 | 1/2 | 0/1 | 0/3 | 7/10 |
| `common/agreement_resources` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/agreement_term_values` | partial | partial | 15/21 | 15/21 | 1/6 | 4/9 | 0/8 | 11/21 |
| `common/agreement_terms` | partial | partial | 2/2 | 2/2 | — | — | — | 2/2 |
| `common/ai_budget` | partial | partial | 6/6 | 6/6 | — | — | 0/3 | 2/6 |
| `common/ai_espionage/operations` | partial | partial | — | — | — | — | — | — |
| `common/ai_espionage/spynetworks` | partial | partial | 0/4 | 0/4 | 0/2 | 0/2 | 0/4 | 0/4 |
| `common/ai_espionage/targets` | partial | partial | — | — | — | — | — | — |
| `common/ambient_objects` | complete | partial | 6/7 | 4/7 | 0/1 | 0/1 | 0/1 | 6/7 |
| `common/anomalies` | complete | partial | 12/14 | 11/14 | 1/3 | 2/4 | 0/4 | 10/14 |
| `common/archaeological_site_types` | partial | partial | 9/11 | 9/11 | 1/3 | 3/5 | 0/3 | 7/11 |
| `common/armies` | complete | partial | 28/30 | 28/30 | 0/2 | 7/9 | 0/4 | 26/30 |
| `common/artifact_actions` | complete | partial | 5/6 | 5/6 | 0/1 | 0/1 | 0/2 | 4/6 |
| `common/ascension_perk_categories` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/ascension_perks` | complete | partial | 10/11 | 11/11 | — | — | 0/2 | 7/11 |
| `common/asteroid_belts` | complete | partial | 5/5 | 5/5 | — | 2/2 | — | 5/5 |
| `common/astral_actions` | complete | partial | 13/15 | 13/15 | 0/2 | 1/5 | 0/3 | 9/15 |
| `common/astral_rifts` | complete | partial | 5/7 | 5/7 | 1/3 | 0/2 | 0/3 | 4/7 |
| `common/attitudes` | partial | partial | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `common/bombardment_stances` | complete | partial | 11/12 | 11/12 | 0/1 | 2/3 | 0/4 | 9/12 |
| `common/buildings` | partial | partial | 41/59 | 32/59 | 0/18 | 6/24 | 0/24 | 26/59 |
| `common/button_effects` | complete | partial | 3/3 | 3/3 | — | — | — | 0/3 |
| `common/bypass` | partial | partial | 22/26 | 22/26 | 0/4 | 3/7 | 0/4 | 22/26 |
| `common/casus_belli` | complete | partial | 7/8 | 7/8 | 0/1 | 0/1 | 0/1 | 4/8 |
| `common/cloaking_strength_levels` | complete | partial | 4/4 | 4/4 | — | 1/1 | — | 4/4 |
| `common/colony_automation` | partial | partial | 2/7 | 2/7 | 0/5 | 0/5 | 0/5 | 1/7 |
| `common/colony_automation_categories` | complete | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/colony_automation_exceptions` | partial | partial | 3/3 | 3/3 | — | 1/1 | — | 3/3 |
| `common/colony_types` | complete | partial | 8/11 | 8/11 | 0/3 | 0/3 | 0/4 | 3/11 |
| `common/colors` | unsupported: the loader builds no class with a member reader | unsupported: the loader builds no class with a member reader | — | — | — | — | — | — |
| `common/component_slot_templates` | partial | partial | — | — | — | — | — | — |
| `common/component_tags` | unsupported: the loader builds no class with a member reader | unsupported: the loader builds no class with a member reader | — | — | — | — | — | — |
| `common/council_agendas` | complete | partial | 10/10 | 10/10 | 1/1 | 3/3 | 0/1 | 9/10 |
| `common/country_container` | partial | partial | — | — | — | — | — | — |
| `common/country_customization` | complete | partial | 2/3 | 1/3 | — | — | 0/3 | 1/3 |
| `common/country_focus/card_categories` | complete | partial | 8/10 | 8/10 | 0/2 | 0/2 | 0/3 | 8/10 |
| `common/country_focus/focus_cards` | partial | partial | — | — | — | — | — | — |
| `common/country_focus/focus_rewards` | complete | partial | 4/6 | 3/6 | 1/3 | 1/3 | 0/2 | 4/6 |
| `common/country_limits/ownership_limits` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/country_limits/ship_of_size_limits` | complete | partial | 4/5 | 4/5 | 0/1 | 3/4 | 0/1 | 4/5 |
| `common/country_types` | partial | partial | 58/67 | 8/67 | 0/8 | 1/9 | 0/10 | 57/67 |
| `common/crisis_levels` | partial | partial | 3/4 | 3/4 | 0/1 | 2/3 | 0/1 | 2/4 |
| `common/crisis_objectives` | complete | partial | 3/3 | 3/3 | — | — | 0/1 | 2/3 |
| `common/crisis_paths` | complete | partial | 1/3 | 1/3 | 0/2 | 0/2 | 0/2 | 1/3 |
| `common/decisions` | partial | partial | 14/15 | 14/15 | 0/1 | 1/2 | 0/2 | 4/15 |
| `common/deposit_categories` | complete | partial | 2/2 | 2/2 | — | — | — | 2/2 |
| `common/deposits` | partial | partial | 23/26 | 23/26 | 0/3 | 1/4 | 0/5 | 15/26 |
| `common/diplo_phrases` | complete | partial | 4/5 | 3/5 | — | — | 0/1 | 3/5 |
| `common/diplomacy_economy` | partial | partial | — | — | — | — | — | — |
| `common/diplomatic_actions` | partial | partial | 30/33 | 9/33 | 0/1 | 1/2 | 0/3 | 30/33 |
| `common/districts` | partial | partial | — | — | — | — | — | — |
| `common/dust_clouds` | complete | partial | 6/6 | 3/6 | — | — | 0/3 | 3/6 |
| `common/dynamic_text` | complete | partial | 1/8 | 1/8 | 0/7 | 0/7 | 0/7 | 0/8 |
| `common/economic_categories` | complete | partial | 6/16 | 3/16 | 1/11 | 0/10 | 0/10 | 6/16 |
| `common/economic_plans` | partial | partial | — | — | — | — | — | — |
| `common/edicts` | complete | partial | 16/19 | 12/19 | 0/3 | 4/7 | 0/4 | 15/19 |
| `common/espionage_assets` | complete | partial | 4/6 | 4/6 | 0/1 | 0/1 | 0/2 | 2/6 |
| `common/espionage_operation_categories` | partial | partial | — | — | — | — | — | — |
| `common/espionage_operation_types` | partial | partial | 5/11 | 5/11 | 0/6 | 2/8 | 0/6 | 3/11 |
| `common/ethic_categories` | complete | partial | — | — | — | — | — | — |
| `common/ethics` | complete | partial | 16/19 | 16/19 | 0/3 | 2/5 | 0/6 | 11/19 |
| `common/event_chains` | complete | partial | 4/5 | 4/5 | 1/2 | 0/1 | 0/1 | 3/5 |
| `common/fallen_empires` | complete | partial | 6/6 | 3/6 | — | — | 0/1 | 4/6 |
| `common/federation_law_categories` | complete | partial | 3/4 | 1/4 | 0/1 | 0/1 | 0/1 | 3/4 |
| `common/federation_laws` | partial | partial | 6/9 | 6/9 | — | 3/3 | 0/3 | 5/9 |
| `common/federation_perks` | partial | partial | 7/12 | 7/12 | 0/1 | 0/1 | 0/5 | 4/12 |
| `common/federation_types` | partial | partial | 7/10 | 7/10 | 0/2 | 1/3 | 0/3 | 7/10 |
| `common/first_contact` | partial | partial | 5/7 | 5/7 | 0/2 | 0/2 | 0/2 | 2/7 |
| `common/frontend_backgrounds` | complete | partial | 2/2 | 2/2 | — | — | — | 2/2 |
| `common/galactic_community_actions` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/galactic_focuses` | partial | partial | 2/3 | 2/3 | — | — | 0/1 | 1/3 |
| `common/game_concept_categories` | complete | partial | 2/2 | 2/2 | — | — | — | 2/2 |
| `common/game_concepts` | partial | partial | 4/5 | 4/5 | 0/1 | 0/1 | 0/1 | 4/5 |
| `common/game_scenarios` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/global_ship_designs` | partial | partial | 14/32 | 4/32 | 0/17 | 0/17 | 0/18 | 14/32 |
| `common/governments` | complete | partial | 9/9 | 9/9 | — | — | 0/1 | 8/9 |
| `common/governments/authorities` | complete | partial | 19/25 | 18/25 | 1/7 | 3/9 | 0/8 | 16/25 |
| `common/governments/civics` | partial | partial | 20/40 | 12/40 | 0/17 | 1/18 | 0/20 | 18/40 |
| `common/governments/councilors` | complete | partial | 12/14 | 12/14 | 0/2 | 0/2 | 0/4 | 8/14 |
| `common/greeting_overlay_sounds` | complete | partial | 3/3 | 3/3 | — | — | 0/1 | 1/3 |
| `common/intel_categories` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/intel_levels` | partial | partial | 1/2 | 1/2 | 0/1 | 1/2 | 0/1 | 1/2 |
| `common/job_tags` | partial | partial | — | — | — | — | — | — |
| `common/lawsuits` | complete | partial | 2/3 | 2/3 | 0/1 | 1/2 | 0/1 | 1/3 |
| `common/leader_classes` | partial | partial | 26/33 | 9/33 | 0/5 | 5/10 | 0/7 | 26/33 |
| `common/leader_tiers` | complete | partial | 2/2 | 2/2 | — | 1/1 | — | 2/2 |
| `common/map_modes` | partial | partial | 9/10 | 4/10 | 0/1 | 0/1 | 0/1 | 8/10 |
| `common/megastructure_overclock_types` | complete | partial | 13/14 | 13/14 | 0/1 | 0/1 | 0/2 | 13/14 |
| `common/megastructures` | partial | partial | 75/83 | 51/83 | 1/10 | 12/18 | 0/11 | 68/83 |
| `common/menace_perks` | complete | partial | 4/5 | 4/5 | 0/1 | 0/1 | 0/1 | 3/5 |
| `common/missions/mission_categories` | complete | partial | 4/4 | 4/4 | — | — | — | 4/4 |
| `common/missions/missions` | partial | partial | 32/36 | 29/36 | 2/6 | 3/7 | 0/5 | 21/36 |
| `common/mutations` | partial | partial | 6/8 | 6/8 | 0/2 | 2/4 | 0/2 | 6/8 |
| `common/named_colors` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/notification_modifiers` | complete | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/observation_station_missions` | complete | partial | 7/11 | 7/11 | 0/4 | 2/6 | 0/5 | 4/11 |
| `common/on_actions` | partial | partial | 0/2 | 0/2 | 0/2 | 0/2 | 0/2 | 0/2 |
| `common/opinion_modifiers` | partial | partial | 9/10 | 8/10 | 0/1 | 4/5 | 0/2 | 8/10 |
| `common/patrons` | partial | partial | 6/14 | 6/14 | 0/7 | 1/8 | 0/8 | 6/14 |
| `common/patrons/callings` | complete | partial | 4/5 | 4/5 | 0/1 | 1/2 | 0/1 | 4/5 |
| `common/patrons/deeds` | partial | partial | 2/2 | 2/2 | — | — | — | 2/2 |
| `common/patrons/psionic_auras` | complete | partial | 22/23 | 22/23 | 0/1 | 0/1 | 0/4 | 12/23 |
| `common/personalities` | complete | partial | 26/28 | 24/28 | 0/2 | 21/23 | 0/5 | 24/28 |
| `common/planet_classes` | unsupported: the loader builds several classes with a member reader: CPlanetClass, CPlanetClassRandomList | unsupported: the loader builds several classes with a member reader: CPlanetClass, CPlanetClassRandomList | — | — | — | — | — | — |
| `common/planet_modifiers` | complete | partial | 4/4 | 4/4 | 1/1 | — | 0/1 | 4/4 |
| `common/policies` | complete | partial | 3/6 | 3/6 | 0/3 | 0/3 | 0/3 | 3/6 |
| `common/policy_categories` | complete | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/pop_categories` | complete | partial | 16/22 | 16/22 | 0/6 | 3/9 | 0/7 | 10/22 |
| `common/pop_faction_types` | complete | partial | 28/34 | 26/34 | 0/6 | 4/10 | 0/10 | 20/34 |
| `common/pop_jobs` | partial | partial | 15/30 | 10/30 | 1/12 | 0/10 | 0/12 | 12/30 |
| `common/portrait_categories` | partial | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/portrait_sets` | complete | partial | 0/6 | 0/6 | 0/6 | 0/6 | 0/6 | 0/6 |
| `common/precursor_civilizations` | complete | partial | 3/3 | 3/3 | — | 1/1 | — | 2/3 |
| `common/prescripted_flags` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/relics` | complete | partial | 9/10 | 9/10 | 0/1 | 2/3 | 0/2 | 8/10 |
| `common/resolution_categories` | complete | partial | 3/4 | 3/4 | 0/1 | 0/1 | 0/1 | 3/4 |
| `common/resolution_groups` | complete | partial | 1/2 | 1/2 | 0/1 | 1/2 | 0/1 | 1/2 |
| `common/resolutions` | partial | partial | 9/12 | 5/12 | 0/1 | 1/2 | 0/3 | 8/12 |
| `common/resource_converters` | partial | partial | — | — | — | — | — | — |
| `common/resource_regions` | complete | partial | 1/1 | 1/1 | — | 1/1 | — | 1/1 |
| `common/script_values` | partial | partial | — | — | — | — | — | — |
| `common/scripted_actions` | partial | partial | — | — | — | — | — | — |
| `common/scripted_effects` | partial | partial | — | — | — | — | — | — |
| `common/scripted_modifiers` | complete | partial | 10/11 | 3/11 | 0/1 | 2/3 | 0/1 | 10/11 |
| `common/scripted_triggers` | partial | partial | — | — | — | — | — | — |
| `common/sector_types` | partial | partial | 5/9 | 5/9 | 0/2 | 0/7 | 0/4 | 5/9 |
| `common/ship_behaviors` | unsupported: the loader builds no class with a member reader | unsupported: the loader builds no class with a member reader | — | — | — | — | — | — |
| `common/ship_categories` | complete | partial | 8/9 | 8/9 | 0/1 | 0/1 | 0/1 | 8/9 |
| `common/ship_sets` | complete | partial | 1/2 | 1/2 | — | — | 0/1 | 1/2 |
| `common/ship_sizes` | partial | partial | 79/100 | 41/100 | 1/21 | 22/40 | 0/21 | 78/100 |
| `common/situation_log/categories` | complete | partial | 6/7 | 3/7 | 0/1 | 1/2 | 0/1 | 6/7 |
| `common/situations` | complete | partial | 28/39 | 27/39 | 1/11 | 2/12 | 0/13 | 23/39 |
| `common/special_projects` | partial | partial | 16/26 | 10/26 | 2/11 | 3/12 | 0/11 | 11/26 |
| `common/specialist_subject_perks` | partial | partial | 4/8 | 4/8 | 0/1 | 0/1 | 0/4 | 2/8 |
| `common/specialist_subject_types` | complete | partial | 5/6 | 4/6 | 1/2 | 1/2 | 0/1 | 5/6 |
| `common/species_archetypes` | complete | partial | 5/7 | 5/7 | 0/2 | 2/4 | 0/2 | 5/7 |
| `common/species_classes` | partial | partial | 12/22 | 7/22 | 0/11 | 2/12 | 0/12 | 12/22 |
| `common/species_names` | complete | partial | 1/5 | 1/5 | — | — | 0/4 | 1/5 |
| `common/species_rights/citizenship_types` | partial | partial | 7/7 | 7/7 | — | 1/1 | — | 7/7 |
| `common/species_rights/colonization_controls` | partial | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/species_rights/living_standards` | partial | partial | — | — | — | — | — | — |
| `common/species_rights/migration_controls` | partial | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/species_rights/military_service_types` | partial | partial | — | — | — | — | — | — |
| `common/species_rights/population_controls` | partial | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/species_rights/purge_types` | partial | partial | 4/4 | 4/4 | — | 3/3 | — | 4/4 |
| `common/species_rights/slavery_types` | partial | partial | 3/3 | 3/3 | — | 1/1 | — | 3/3 |
| `common/species_rights/subspecies_integration_types` | partial | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/specimens` | complete | partial | 5/7 | 3/7 | 0/2 | 0/2 | 0/2 | 4/7 |
| `common/star_classes` | partial | partial | 8/11 | 8/11 | 0/2 | 1/4 | 0/3 | 8/11 |
| `common/star_classes/randomizers` | complete | partial | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `common/starbase_buildings` | partial | partial | 3/4 | 3/4 | 0/1 | 0/1 | 0/1 | 3/4 |
| `common/starbase_levels` | complete | partial | 14/17 | 8/17 | 2/5 | 2/5 | 0/4 | 14/17 |
| `common/starbase_modules` | partial | partial | 2/7 | 2/7 | 0/5 | 0/5 | 0/5 | 2/7 |
| `common/starbase_types` | complete | partial | 5/5 | 4/5 | — | — | 0/2 | 4/5 |
| `common/start_screen_messages` | complete | partial | 2/3 | 2/3 | 0/1 | 1/2 | 0/1 | 2/3 |
| `common/static_modifiers` | partial | partial | 3/3 | 1/3 | — | — | — | 3/3 |
| `common/storm_types` | complete | partial | 24/33 | 20/33 | 0/1 | 0/3 | 0/9 | 17/33 |
| `common/strategic_resources` | partial | partial | 21/22 | 20/22 | 2/3 | 8/9 | 0/4 | 18/22 |
| `common/system_tooltips` | complete | partial | 1/1 | 1/1 | — | — | — | 0/1 |
| `common/system_types` | complete | partial | 2/2 | 2/2 | — | — | 0/1 | 2/2 |
| `common/target_types` | complete | partial | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 | 0/1 |
| `common/technology` | partial | partial | 21/29 | 12/29 | 0/8 | 4/12 | 0/11 | 20/29 |
| `common/technology/category` | complete | partial | 3/3 | 3/3 | — | 1/1 | 0/1 | 2/3 |
| `common/technology/tier` | complete | partial | 2/2 | 2/2 | — | 1/1 | 0/1 | 1/2 |
| `common/technology_ages/preftl` | complete | partial | 1/1 | 1/1 | — | — | — | 1/1 |
| `common/terraform` | complete | partial | 5/10 | 5/10 | 0/5 | 1/6 | 0/6 | 3/10 |
| `common/timeline_events` | complete | partial | 3/7 | 3/7 | 0/4 | 0/4 | 0/4 | 3/7 |
| `common/tradable_actions` | partial | partial | 15/15 | 15/15 | — | 7/7 | 0/2 | 10/15 |
| `common/tradition_categories` | complete | partial | 3/7 | 3/7 | 0/4 | 0/4 | 0/5 | 3/7 |
| `common/traditions` | complete | partial | 10/11 | 11/11 | — | — | 0/2 | 7/11 |
| `common/trait_tags` | partial | partial | — | — | — | — | — | — |
| `common/traits` | partial | partial | 75/105 | 56/105 | 0/33 | 4/34 | 0/35 | 30/105 |
| `common/war_goals` | partial | partial | 19/21 | 19/21 | 0/2 | 3/5 | 0/3 | 19/21 |
| `common/zone_slots` | partial | partial | — | — | — | — | — | — |
| `common/zones` | partial | partial | — | — | — | — | — | — |
| `dlc_metadata/dlc_recommendations` | partial | partial | — | — | — | — | — | — |
| `events` | partial | partial | 22/22 | 0/22 | — | — | 0/22 | 22/22 |
| `flags` | complete | partial | 1/4 | 1/4 | 0/3 | 0/3 | 0/3 | 1/4 |
| `gfx/portraits/sprite_configurations` | complete | partial | 3/3 | 3/3 | — | — | — | 2/3 |
| `gfx/projectiles/planet_destruction` | partial | partial | 7/15 | 7/15 | 0/7 | 0/7 | 0/8 | 7/15 |
| `gfx/worldgfx` | unsupported: the loader builds no class with a member reader | unsupported: the loader builds no class with a member reader | — | — | — | — | — | — |
| `interface/resource_groups` | complete | partial | 4/10 | 4/10 | 0/6 | 1/7 | 0/6 | 4/10 |
| `map/galaxy` | complete | partial | 7/11 | 7/11 | 0/4 | 4/8 | 0/4 | 7/11 |
| `prescripted_countries` | unsupported: 2 databases load this directory | unsupported: 2 databases load this directory | — | — | — | — | — | — |
| `sound/advisor_voice_types` | complete | partial | 4/4 | 4/4 | — | — | 0/1 | 2/4 |

### Per reader identity

`complete/denominator`; `none` is the fields with no reader identity.

| Reader identity | Reader kinds | Shapes | References | Numerics | Weights | Scope |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `none` | 0/419 | 0/419 | 0/377 | 0/377 | 0/419 | 0/419 |
| `b894933b2b2853a5` | 391/391 | 148/391 | — | — | — | 391/391 |
| `325efaa17499c32d` | 268/268 | 261/268 | — | — | — | 268/268 |
| `e044b50825a1841d` | 138/138 | 137/138 | — | — | — | 83/138 |
| `6e28a363faf61c52` | 105/105 | 105/105 | — | — | — | 60/105 |
| `d7a95ab8c8d44489` | 100/100 | 99/100 | — | 100/100 | — | 100/100 |
| `a9818fec780f8313` | 99/99 | 97/99 | — | 99/99 | — | 99/99 |
| `f08cb83d92484a89` | 85/85 | 82/85 | — | — | 0/85 | 26/85 |
| `88f77b58ddf77488` | 60/60 | 60/60 | — | — | — | 0/60 |
| `ba5f8cddeba0d833` | 28/28 | 28/28 | — | — | — | 28/28 |
| `9416e3bf7f623b9c` | 22/22 | 0/22 | — | — | 0/22 | 22/22 |
| `e327ea91dfe75d65` | 16/16 | 16/16 | — | — | — | 2/16 |
| `1c2988588f7e8eaa` | 7/7 | 7/7 | — | — | — | 7/7 |
| `5d6a4255a3ebd82d` | 7/7 | 7/7 | — | 0/7 | — | 7/7 |
| `85bd4238680bd5b9` | 7/7 | 0/7 | — | — | — | 4/7 |
| `9cc4a8b09a789cf4` | 0/7 | 0/7 | — | — | 0/7 | 0/7 |
| `c61abc171edc30aa` | 7/7 | 7/7 | — | 7/7 | — | 7/7 |
| `d2aff87f9b4b84b1` | 6/6 | 6/6 | 6/6 | — | — | 6/6 |
| `d3a866b65e16b749` | 0/5 | 5/5 | — | — | — | 0/5 |
| `074177ea8f47c362` | 4/4 | 0/4 | — | — | — | 2/4 |
| `5a74c67fe5a4adf9` | 3/3 | 3/3 | — | — | — | 3/3 |
| `87b0c16089f35350` | 3/3 | 3/3 | — | 0/3 | — | 3/3 |
| `99b26ea3b826bc63` | 3/3 | 3/3 | — | — | — | 0/3 |
| `a2ff84da2cff0bb5` | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 | 0/3 |
| `a430a4b92eb2c8f6` | 3/3 | 3/3 | — | — | — | 2/3 |
| `ae2bd2a2d5591e04` | 3/3 | 3/3 | 3/3 | — | — | 3/3 |
| `27a18d4c719aa3f9` | 2/2 | 0/2 | 2/2 | — | — | 2/2 |
| `3e4a6b47b20070ea` | 2/2 | 2/2 | — | — | 0/2 | 0/2 |
| `784ec4ab3c469836` | 0/2 | 0/2 | 0/2 | 0/2 | 0/2 | 0/2 |
| `87346602eebc8fd3` | 2/2 | 0/2 | — | — | 0/2 | 0/2 |
| `c81625955ad8e679` | 2/2 | 2/2 | — | — | — | 0/2 |
| `f9fb8c8f5f2ca714` | 0/2 | 2/2 | — | — | — | 0/2 |
| `fd8c6ad9ff94a8f2` | 2/2 | 2/2 | — | — | 0/2 | 2/2 |
| `0100641a6783ef9a` | 0/1 | 0/1 | — | — | 0/1 | 0/1 |
| `076519e5cb6d978f` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `084d56cb19b1c12c` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `0c856b72f66c4164` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `0fc054af78594069` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `14a90b8d8c823a11` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `16b0cf79a4a26271` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `231abbf59ab285b4` | 1/1 | 1/1 | 1/1 | — | — | 1/1 |
| `260d326aa7a2fa31` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `2b8fd05026f4a58e` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `3007f5611cf13042` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `5342ee29807f32f1` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `59adcac8003a5697` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `5acdd80c3e3b21f4` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `5c598b137a53d15a` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `5efd8c15a1a0eca4` | 1/1 | 0/1 | — | — | — | 1/1 |
| `61bb19374ae40dbf` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `6e16a88f362f7c3d` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `6eb1725098a74756` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `7133036cc58cb45c` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `716290887881acb1` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `782b36da9ac3ef37` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `7ae78f17d9719167` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `8bf2338978b0bf4a` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `8c701ec68cce3654` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `8e4f28f41bf3b3dc` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `935c3b680732b559` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `9649e7622f7efce3` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `98c58513111b8c6e` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `a9e879e470ef5a19` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `b14cfbac48ff43a3` | 1/1 | 1/1 | 1/1 | — | — | 1/1 |
| `b2509d1cb943496a` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |
| `b32697a8a2f30541` | 0/1 | 0/1 | — | — | 0/1 | 0/1 |
| `bf399ef5e381c23d` | 1/1 | 0/1 | 0/1 | — | — | 1/1 |
| `c0c46ddf764006e1` | 0/1 | 0/1 | — | — | 0/1 | 0/1 |
| `cdf887af22570d73` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `cfc2d0b440083587` | 1/1 | 0/1 | 1/1 | — | — | 1/1 |
| `e7571cd65529a619` | 1/1 | 1/1 | 1/1 | — | — | 1/1 |
| `ea301d40f2b4cb92` | 0/1 | 1/1 | — | — | 0/1 | 0/1 |
| `f769f8f424d134f6` | 1/1 | 1/1 | — | — | 0/1 | 0/1 |
| `faa4fe90d7f075fc` | 0/1 | 0/1 | — | — | 0/1 | 0/1 |
| `fea242baa1840603` | 1/1 | 0/1 | — | — | 0/1 | 0/1 |

## Labelled deficits

None of the failure-shape tickets had landed by `ee01bf3`. Each is a deficit of this baseline:

| Ticket | Deficit in this run |
| --- | --- |
| SDK-710 | Fields with no shared reader (1,209 `UnresolvedReader` gaps in 113 registries), reader paths that stop (83 `UnresolvedPath` in 75), unnamed keys (16) and unreadable input (12). Reader kinds fail for 377 fields; their references and numerics are undetermined (382). The `CEvent` members of `events` (94 event claims). |
| SDK-627 | Repeat behavior and required fields that stay unresolved (1,615 `UnresolvedStorage` gaps in 119 registries); enum domains (`enum[event_window_type]`); the occurrence of `pre_triggers` and `difficulty` in an event kind. |
| SDK-635 | References: 24 of 417 fields complete. |
| SDK-713 | Naming rules: the unjoined generation sites on every `modifier_families` answer, and the 74 `UnresolvedPath` gaps of 19 registries' families. |
| SDK-676 | Modifier containers of unjoined and nested fields (45 `UnresolvedPath` gaps on modifier containers in 14 registries; 36 stops of the modifier block analysis). |
| SDK-707 | Derived names composed outside the owner's methods (the 328 `UnresolvedPath` and 326 `UnresolvedCondition` `derived_names` gaps include them). |
| SDK-609 | Localisation argument forms and the link output contexts (Localisation area). |
| SDK-610 | The 80 custom define read helpers (Defines area). |
| SDK-711 | The 41 commands whose supported scopes stay unresolved (Effects and Triggers areas). |
| SDK-716 | Claims under a field-predicate subtype (396 event claims, and those of other types). |

The shapes with no ticket above are for SDK-553 to file: the read scope of a field passed as a scope
argument (1,346 gaps in 116 registries), a stored scope (72) or a zero mask (28); blocks that no
attributed evaluation evaluates (75); path and loop limits; numeric conversion (387 gaps in 59
registries); `event_namespace`, whose `namespace` key is not a field; and the root `event = {`
claims.

## `modifier_families` cost (SDK-591)

185 `modifier_families` calls take about 30 s of the 218 s run, 0.17 s each, nearly all in
rebuilding the modifier input. The measurement and the decision are on Native's
[performance page](https://github.com/pdx-foundry/native/blob/main/docs/native/performance.md#modifier-families) (pdx-foundry/native#235).

## Capture and checks

| Input or result | Identity |
| --- | --- |
| Executable SHA-256 (Native's M452, Stellaris 4.5.2 (9776), Apple Silicon) | `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b` |
| Native revision (`main`, pinned in `Cargo.lock`) | `ee01bf338872290eaaab2ca3c0503fc33244686e` |
| Atlas revision (clean tree) | `3375af745d85ca70c4da756a1d5ceea2b24c0e61` |
| Config revision | `85747602a614ad7daa8cc66453777ecb023463a8` |
| Config content SHA-256 | `5c79cabb8d1b25e40994c0d134f5623bc2aef2a6b964059aabfc4834e6d039cc` |
| Live rule snapshot SHA-256 | `d2208992c5d7f0b6d281907ffa76ef01cebe539ba3ff3e8176170b2416672728` |
| Snapshot ID | `stellaris-rules/28616cddeab35d7e7154188f2b6f10d186f063aab20aeb9fbd853dadedbaf2f0@1` |
| Recording: 2,933 files, 69 MB; SHA-256 of its `shasum -a 256` manifest | `bf434d3d6be5fa16293b1126855126c44c28ba94e3d51ef9aa3fcd7e63139cd2` |
| `ledger.json` / `coverage.json` | `ba2bb263…f67947` / `92c137bb…4264cf` (pinned in full in the gate fixture) |
| `comparison.json` | `f402d95377464ab3609beafa09b708af3f69174086d3cf6a5c6bc1d719e31335` |
| `registries.json` | `3e8aa79aa6cab310e27a0dd9cebbe81a44e2a251ef2f512e6c3f151601a15b71` |
| Sweep report / `--baseline` | `d34e695e9d2b157cbfc4097235a54b68eb13a1ee8f224d5a39e285ee19f0aae5` / `5493f93f3075598a4f27a8cece9179a9da62ea2c1a957a9ee666ada7a6ab366c` |

The live CLI exited 0 in 218 s with 4.05 GB peak resident. Ledger, comparison and registries exit
2 for the retained source diagnostics; registries also for Native's partial registry list (13 of
198 scheduled databases are unnamed). The sweep report took 0.2 s. The full-config gate passed
with the live snapshot.

The full recording is kept on the capture host, outside Git, at `.local/sdk-798/m452/` of the main
Atlas checkout, with the snapshot and the manifest (`m452.manifest`). Tests keep using the trimmed
sample in `tests/fixtures/native/m452`, recorded at the same commit by SDK-796. The manifest
digest identifies the recording: run the manifest command below in the directory and compare.

## Reproduce

```sh
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/m452 .scratch/rules.json
(cd .scratch/m452 && find . -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256)
cargo run --release --locked -- ledger --config "$PDX_CONFIG_PATH" --snapshot .scratch/rules.json --output .scratch/reports
cargo run --release --locked -- compare "$PDX_CONFIG_PATH" .scratch/rules.json .scratch/reports \
  --script-docs "$PDX_CONFIG_PATH/../script-docs/v4.5.0" \
  --defines "$STELLARIS_PATH/common/defines/00_defines.txt" \
  --defines "$STELLARIS_PATH/common/defines/00_defines_additional_content.txt" \
  --answers .scratch/m452
cargo run --release --locked -- registries "$PDX_CONFIG_PATH" .scratch/m452 .scratch/reports
ATLAS_RULE_SNAPSHOT=.scratch/rules.json cargo test --locked --test full_config -- --ignored
```

In a Native checkout at `ee01bf3`:

```sh
cargo run --release --example registry-sweep -- "$ATLAS/.scratch/m452" > .local/population/sweep.json
cargo run --release --example registry-sweep -- --baseline "$ATLAS/.scratch/m452"
cargo run --release --example registry-sweep -- --diff tests/population/m452/registry-sweep.json .local/population/sweep.json
```

Run the live command alone on the host: Native allows one game at a time.
