# Registry comparison (SDK-781)

The `registries` command puts Native's registry list beside the directories of the config types'
`path` values. It is the "exception recorded" half of the Milestone 5 gate (decision D1): each
config type in the gate population has a Native registry or a stated reason. The command reads
Native's `registries` answer from a recording; a live `snapshot` run writes that recording. The
[README](../../README.md) gives the command.

It writes `registries.json`: one row for each config type with a `path`, with its directory, the
Native registry that names the directory, a `join`, and whether the type is in the gate
population. The report also gives Native's source and gaps, the count of types by join, the count
of directories in both, and the engine-only registries.

| `join` | Meaning |
| --- | --- |
| `named` | A Native registry names the directory, and only this type loads from it. |
| `several_types_in_directory` | A Native registry names the directory, but several types load from it. Coverage records a mapping gap until SDK-785 joins them. |
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

## Rerunning

Rerun the command after the Native pin moves (SDK-796) and for the sweep (SDK-798). A directory
that a new Native registry names moves from `no_named_registry` to `named` or
`several_types_in_directory`, and an engine-only registry that matches a config directory leaves
`engine_only`.

Reading paths without `game/` added 11 covered claims of `type[precursors]` and
`type[prescripted_flags]` to the Milestone 4 snapshot's coverage: 22,679 / 58,032 instead of the
22,668 that [the Milestone 4 capture](milestone-4.md) records.
