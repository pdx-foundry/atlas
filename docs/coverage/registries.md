# Registry comparison (SDK-781)

The `registries` command puts Native's registry list beside the directories of the config types'
`path` values. It is the "exception recorded" half of the Milestone 5 gate (decision D1): each
config type in the gate population has a Native registry or a stated reason. The command reads
Native's `registries` answer from a recording; a live `snapshot` run writes that recording. The
[README](../../README.md) gives the command.

It writes `registries.json`: one row for each config type with a `path`, with its directory, the
Native registry that names the directory, a `join`, and whether the type is in the gate
population. The report also gives Native's source, completeness and gaps, whether the config inventory is
complete, the count of types by join, the count
of directories in both, and the engine-only registries. The command exits 2 after it writes the report when config
diagnostics remain or Native's answer is partial, as `ledger` and `compare` do; the pinned config
has 19 diagnostics.

| `join` | Meaning |
| --- | --- |
| `named` | A Native registry names the directory, and only this type loads from it. |
| `several_types_in_directory` | A Native registry names the directory, but several types load from it. Coverage credits each type only through its config selector ([ledger contract](ledger.md#snapshot-input), SDK-785). |
| `no_named_registry` | No Native registry names the directory. |
| `deferred_format_family` | No Native registry names the directory, and it belongs to a Milestone 6 format family: `gfx`, `interface`, `sound`, `music`, `flags`, `fonts`, `map/setup_scenarios` or `prescripted_countries`. |

Native's `UnnamedRegistries` gap is a count with no names, so it cannot say which config type has
an unnamed engine candidate. The report keeps the gap once in `native_gaps`, beside the table.

`coverage.json` gives the matching rates: `by_type` splits each file's totals by config type,
including types with no covered claim, and puts the file's other claims in `outside_types`.

## Gate population at config `8574760`

The gate population is every config type whose `path` directory is `common`, `events` or below
one of them: 218 types in 187 directories. Two of them, `type[precursors]` and
`type[prescripted_flags]`, write their path without `game/`; Atlas reads the same directory. (The
216 of decision D1 counted only paths with `game/`.) `type[mod_descriptor]` and
`type[dlc_descriptor]` have no `path` and are not in the report.

Over the full M452 recording of Native `0167bc5` (`registry-directories/v3`, 164 registries, no
unnamed-registry gap):

| | Types | Directories |
| --- | --- | --- |
| Gate population | 218 | 187 |
| `named` | 146 | 146 |
| `several_types_in_directory` | 12 | 6 |
| `no_named_registry` | 60 | 35 |

Over every row, 157 directories are in both, 51 directories are config only (35 in the gate,
16 deferred format families), and 7 registries are engine only: `common/ai_espionage/targets`,
`common/dynamic_text`, `common/game_scenarios`, `common/job_tags`, `common/lawsuits`,
`common/trait_tags` and `dlc_metadata/dlc_recommendations`. Five named directories are outside the
gate: `gfx/portraits/sprite_configurations`, `gfx/projectiles/planet_destruction`,
`interface/resource_groups`, `map/galaxy` and `sound/advisor_voice_types`.

The `several_types_in_directory` directories are `common/ascension_perks`,
`common/governments/authorities`, `common/governments/civics`, `common/pop_jobs`,
`common/technology` and `common/traditions`, each a type and its `swapped_` type. `events` holds
`type[event]` and `type[event_namespace]` and has no named registry.

### Types in `game/common` itself

`type[alert]`, `type[alert_icon]` (`path_file = "alerts.txt"`) and `type[achievement]`
(`path_file = "achievements.txt"`) have `path = "game/common"`
(`common/zzz_common_folder_elements.cwt`). They are in the gate population, in the directory
`common`, with the reason `no_named_registry`: Native names registries by content directory, and
these load one named file directly in `common/`. `path_file` is file selection, which SDK-788 owns.

## Rerun at Native `ee01bf3` (SDK-796)

Over the full M452 recording of Native `ee01bf3` (`registry-directories/v6`, 185 registries), the
answer is partial: its `UnnamedRegistries` gap says 13 of 198 scheduled databases have no template
loader and enumerate no single content directory. The [pin move A capture](pin-move-a.md) gives the
run's identities.

| | Types | Directories | At `0167bc5` |
| --- | --- | --- | --- |
| Gate population | 218 | 187 | 218 / 187 |
| `named` | 159 | 159 | 146 / 146 |
| `several_types_in_directory` | 20 | 10 | 12 / 6 |
| `no_named_registry` | 39 | 18 | 60 / 35 |

Over every row, 177 directories are in both (from 157), 31 directories are config only (18 in the
gate, 13 deferred format families) and 8 registries are engine only: the seven above and
`common/component_tags`, which no config type's `path` names. Three more named directories are
outside the gate: `flags`, `gfx/worldgfx` and `prescripted_countries`.

25 types moved. Each is a registry that SDK-783 or SDK-786 names:

| From | To | Types (directory) |
| --- | --- | --- |
| `no_named_registry` | `named` | `type[ambient_object]` (`common/ambient_objects`), `type[country_color]` (`common/colors`), `type[fallen_empires]` (`common/fallen_empires`), `type[global_ship_design]` (`common/global_ship_designs`), `type[on_action]` (`common/on_actions`), `type[opinion_modifier]` (`common/opinion_modifiers`), `type[ship_behavior]` (`common/ship_behaviors`), `type[special_project]` (`common/special_projects`), `type[start_screen_message]` (`common/start_screen_messages`), `type[static_modifier]` (`common/static_modifiers`), `type[resource]` (`common/strategic_resources`), `type[terraform_link]` (`common/terraform`), `type[trait]` (`common/traits`) |
| `no_named_registry` | `several_types_in_directory` | `type[diplo_phrase]` and `type[diplo_phrase_greetings]` (`common/diplo_phrases`), `type[planet_class]` and `type[planet_class_random_list]` (`common/planet_classes`), `type[species_name]` and `type[species_named_list]` (`common/species_names`), `type[event]` and `type[event_namespace]` (`events`) |
| `deferred_format_family` | `named` | `type[worldgfx]` (`gfx/worldgfx`), `type[prescripted_country]` (`prescripted_countries`) |
| `deferred_format_family` | `several_types_in_directory` | `type[color_define]` and `type[randomizable_combo]` (`flags`) |

The four deferred-family moves are outside the gate population. The 18 gate directories that no
registry names are `common` (the three types of [`game/common` itself](#types-in-gamecommon-itself)),
`common/ai_force_species`, `common/component_sets`, `common/component_templates`,
`common/game_rules`, `common/gamesetup_settings`, `common/graphical_culture`,
`common/lawsuit_type`, `common/mandates`, `common/message_types`, `common/name_lists`,
`common/random_names`, `common/random_names/base`, `common/scripted_loc`,
`common/section_templates`, `common/sector_focuses`, `common/solar_system_initializers` and
`common/trade_conversions`.

The [Milestone 5 sweep](milestone-5.md) (SDK-798) ran at the same commit; its `registries.json`
is byte-identical to this rerun's.

## Rerunning

Rerun the command after each Native pin move, and for the sweep (SDK-798). A directory that a new
Native registry names moves from `no_named_registry` to `named` or `several_types_in_directory`,
and an engine-only registry that matches a config directory leaves `engine_only`.

Reading paths without `game/` added 11 covered claims of `type[precursors]` and
`type[prescripted_flags]` to the Milestone 4 snapshot's coverage: 22,679 / 58,032 instead of the
22,668 that [the Milestone 4 capture](milestone-4.md) records.
