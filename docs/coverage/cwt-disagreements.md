# CWT disagreement review

Milestone 5 closes only after each covered claim where Atlas and CWT disagree is reviewed: it is a
CWT error, or an Atlas or Native defect to repair (Native `docs/roadmap.md`, "Milestone 5
acceptance"). This page holds that review. Each entry names the config claim, Atlas's answer, the
evidence, and the verdict.

## Keys the engine never reads

The [not-a-field rule](ledger.md#keys-the-engine-never-reads-decision-of-2026-10-20) answers these
claims once the snapshot carries Native's `keeps_definition_text` (Native `bb0cc04`, `Known(false)`
here): Native's root-field answer for the registry is `Complete` and lists no field with the key,
and the loader keeps no definition text.
Snapshot: live M452 capture at Native `bb0cc04` (`registry-fields/v64`, `loader-rules/v8`).

### `common/timeline_events` (`common/timeline_events.cwt`)

Native lists seven root fields: `fire_only_once`, `gui`, `tooltip`, `tooltip_delayed`, `types`,
`override_text` and `override_texture`. The answer is `Complete`, with only the two standard
`OutsideMethod` boundaries.

| CWT key (line) | Claims | Atlas answer | Engine key | Verdict |
| --- | --- | --- | --- | --- |
| `type` (17-21) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` and item cardinality (7) | absent; `not_a_field` | `types` | CWT error: the shipped guide (`00_timeline_defines.txt`) and the reader both use `types` |
| `text` (26-28) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` (5) | absent; `not_a_field` | `override_text` | CWT error, to confirm: the shipped guide writes `text = { }` in its database example, but the reader reads only `override_text`; no shipped entry uses either |
| `texture` (30-32) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` (5) | absent; `not_a_field` | `override_texture` | As for `text` |

The loader keeps no definition text (Native `bb0cc04`, `keeps_definition_text` `Known(false)` for
`common/timeline_events`), so no later code reads `text` or `texture` from a definition; the
`add_timeline_event` effect's `override_text` and `override_texture` are command arguments, not
definition keys.

## Renamed directories

### `type[lawsuit]` (`common/lawsuits.cwt`)

| Claim | CWT | Engine | Verdict |
| --- | --- | --- | --- |
| `types/type[lawsuit]/path` (`loader_path`) | `game/common/lawsuit_type` | `common/lawsuits` | CWT error: the executable names only `common/lawsuits`, the game ships `common/lawsuits/00_example.txt` (whose guide documents `effect`, `cost` and `duration`), and Native's `common/lawsuits` registry reads those three fields. |

## Covered disagreements at Native `bb0cc04`

**Status: in progress.** Group 4 (41 claims) is open until its conditional checks are read; every
other group has a verdict. The review completes when group 4 does.

Every covered claim in `common/` and `events/` whose comparison status is `different`: **579
claims in 19 groups**. The membership, one line per claim with its group number, is
[`cwt-disagreements.tsv`](cwt-disagreements.tsv). A claim's group is the triple (property, the
first 30 characters of the config answer, Atlas's answer form), as listed in the table.

**Inputs.**

- Atlas `740acce` (main `5e9a2d4` with the line-bound shipped comments of #48), config revision
  `8574760` (`config_sha256` `5c79cabb…039cc`).
- The live M452 capture at Native `bb0cc04`: `rules.json` SHA-256 `8a6389f9…5804b`.
- `ledger --config … --snapshot rules.json --game-content $STELLARIS_PATH --output R`, then
  `compare … rules.json R --script-docs …/script-docs/v4.5.0 --defines
  $STELLARIS_PATH/common/defines/00_defines.txt --answers <capture>/m452`. Outputs: `ledger.json`
  `0c03b5a9…aca00`, `coverage.json` `89df3f9a…2ffe3`, `comparison.json` `106b1676…a5652`.
- Native methods behind the answers: `registry-fields/v64` (fields: repeat, requirement, value
  form, read scope), `loader-rules/v8` (extension, definition names) and
  `registry-directories/v8` (registry discovery: the content directory of `common/lawsuits`).

Only the comparable properties are compared (presence, loader lines, basic value form, numeric
cardinality, enum members, event-kind `push_scope`). Verdicts: **CWT error** — the engine fact
contradicts the config; **Atlas defect** or **Native defect** — Atlas's answer is wrong or
incomplete, with the cause; **open** — not decided yet.

A repeat behavior says what the engine does with a repeated key; it is not an occurrence limit.
`Merges` and `Accumulate` process a second occurrence, so a config maximum of 1 is stricter than
the engine. `Replace` also processes it (the later value wins): Atlas compares it as a maximum of
1 ([ledger](ledger.md), field subjects), which is not an engine limit, so a disagreement that rests
only on that mapping is an Atlas comparison artifact.

| # | Property | CWT | Atlas | Claims | Verdict |
| ---: | --- | --- | --- | ---: | --- |
| 1 | `cardinality_maximum` | `1` | repeat `Merges` | 201 | CWT error: a repeated block merges into the first; the engine processes it |
| 2 | `loader_path` (`path_extension`) | `.txt` | no suffix filter | 167 | CWT error: a template loader reads every file of any extension (Native `loader-rules.md`, "The file enumeration") |
| 3 | `cardinality_maximum` | `1` | repeat `Accumulate` | 76 | CWT error: each repeat appends |
| 4 | `cardinality_minimum` | `1` | `0` (Optional) | 41 | **Open.** The census loads a definition without the field and logs nothing, but a check that runs only when another field is set is not observed ([pin move B](pin-move-b.md): an `Optional` difference is a finding to investigate). Route: read each field's post-read hooks for a conditional absence check, or a fixture probe with the other fields set |
| 5 | `value_form` | `int` | `number` | 32 | CWT error: the reader is fixed point (`FixedPoint`) and accepts a fraction |
| 6 | `cardinality_maximum` | `inf` | repeat `Replace` | 14 | Atlas defect (comparison): `Replace` is compared as a maximum of 1; the engine processes repeats, so CWT's `inf` does not contradict it |
| 7 | `value_form` | `block` | `reference` | 12 | Native defect, likely: lists of keys (`allowed_archetypes = { … }`) that Native reports as one `Scalar` reference per occurrence (`Accumulate`) |
| 8 | `value_form` | `int` | `block` | 9 | Atlas defect: Native reports the weight block's bare value (`members.WeightBlock.scalar`, a `FixedPoint` reader) for all 17 fields of groups 8–9, but Atlas's `assemble_field_value_form` answers `block` for every `Block` reader without reading it |
| 9 | `value_form` | `float` | `block` | 8 | Atlas defect, as #8 |
| 10 | `cardinality_minimum` | `0` | `1` (Required) | 6 | CWT error: the engine logs the field's absence (census) |
| 11 | `value_form` | `float` | `integer` | 4 | CWT error: the reader is an `Integer` reader |
| 12 | `loader_path` (`type_per_file`, `name_from_file`) | `yes` | names from the root key | 2 | CWT error: `common/diplomacy_economy` is a template loader (`loader-rules/v8`, `Known`), which names each root statement (`once`, …) by its key, not one definition per file |
| 13 | `cardinality_maximum` | `20` | repeat `Accumulate` | 1 | CWT error: Native reads `policy/option` with repeat `Accumulate` into an array of `CPolicyOption` and states no limit, and the executable's strings hold no option-count diagnostic; a display limit would not be a load rule |
| 14 | `loader_path` (`name_field`) | `localization` | names none | 1 | CWT error: `common/start_screen_messages` holds anonymous `part = { … }` statements, and Native states that the loader names none of its definitions (`Known` empty) |
| 15 | `scope_context` (`push_scope`) | `country` | `observer` | 1 | CWT error: observer events are read in the observer scope |
| 16 | `loader_path` (`skip_root_key`) | `collection` | names from the root key | 1 | CWT error: `common/country_container` is a template loader that names the one root statement `collection`; the `…_unit` blocks are its members, which no other shipped file names |
| 17 | `value_form` | `int` | `scoped_number` | 1 | CWT error: `scripted_action/cooldown` accepts a scripted value |
| 18 | `loader_path` (`path`) | `game/common/lawsuit_type` | `common/lawsuits` | 1 | CWT error ([renamed directories](#renamed-directories); evidence `registry-directories/v8`) |
| 19 | `cardinality_maximum` | `0` | repeat `Replace` | 1 | CWT error: `deposit/subtype[!null]/is_null` is read on every path (`conditions` `Always`); CWT's `0..0` describes the subtype that the key selects, not a rule the engine applies. The verdict rests on the unconditional read, not on `Replace` |

Totals: CWT error 14 groups (495 claims); Atlas defect 3 groups (31 claims); Native defect 1 group
(12 claims); open 1 group (41 claims).

### Examples

Each with its `.cwt` location, subject and the Native evidence of Atlas's answer
(`registry-fields/v64` unless stated).

1. `common/districts.cwt:159` `district/planet_modifier` (`#repeat_behavior`);
   `common/ascension_perks.cwt:65` `ascension_perk/modifier`; `common/ascension_perks.cwt:111`
   `tradition_swap/weight`.
2. `common/agreements.cwt:55` `type[agreement_preset]/path_extension`
   (`registry:common/agreement_presets#extension`, `loader-rules/v8`); `common/star_classes.cwt:18`;
   `common/ai_espionage.cwt:4`.
3. `common/species_consolidated.cwt:311` `species_class/removed_planet_types`;
   `common/zone_slots.cwt:26` `zone_slots/excluded_zone_sets`; `common/colony.cwt:81`
   `colony_automation/prio_districts`.
4. `common/situation_logs.cwt:24` `situation_log_category/localization`
   (`#occurrences.minimum`); `common/storm_types.cwt:101`
   `storm_types/cosmic_storm_texture_color_path`; `common/asteroid_belts.cwt:9`
   `asteroid_belt_type/mesh`.
5. `common/ship_sizes.cwt:84` `ship_size/max_hitpoints` (`#value_form`);
   `common/strategic_resources.cwt:42` `resource/subtype[tradable]/market_price`;
   `common/technologies_consolidated.cwt:192` `technology/subtype[repeatable]/cost_per_level`.
6. `common/traits.cwt:254` `trait/subtype[leader_trait]/custom_tooltip_with_modifiers`;
   `common/ascension_perks.cwt:95` `tradition_swap/custom_tooltip`; `common/traits.cwt:251`
   `custom_tooltip`.
7. `common/megastructures.cwt:312` `megastructure/scripted_action`; `common/components.cwt:142`
   `component_template/size_restriction`; `common/megastructures.cwt:253`
   `megastructure/overclock_types`.
8. `common/technologies_consolidated.cwt:115` `technology/subtype[!start]/cost`;
   `common/governments.cwt:623` `councilor/ai_priority`; `common/traits.cwt:43` `trait/cost`.
9. `common/ship_sizes.cwt:344` `ship_size/ai_ship_data/fraction`; `common/buildings.cwt:62`
   `building/district_limit`; `common/bombardment_stances.cwt:33` `bombardment_stance/planet_damage`.
10. `common/situations.cwt:224` `situation_type/stages` ("No stages detected");
    `common/country_focus.cwt:43` `country_card_categories/desc`; `common/patrons.cwt:272`
    `calling/tier`.
11. `common/planet_classes.cwt:118` `planet_class/extra_planet_count`; `common/components.cwt:354`
    `strike_craft_component_template/count`; `common/bypass.cwt:37` `bypass/ftl_multiplier`.
12. `common/diplomacy_economy.cwt:5-6` (`registry:common/diplomacy_economy#definition_names`,
    `loader-rules/v8`). 13. `common/policies.cwt:28`. 14. `common/start_screen_messenges.cwt:6`
    (`loader-rules/v8`). 15. `events/events.cwt:43` (`field:events/observer_event#read_scope`).
    16. `common/country_container.cwt:5` (`loader-rules/v8`). 17. `common/scripted_actions.cwt:205`.
    18. `common/lawsuits.cwt:3` (`registry:common/lawsuits#loader_path`, `registry-directories/v8`).
    19. `common/deposits_and_planetary_features.cwt:48`.

### Defects to repair

- **Weight blocks' bare value (groups 8–9; 17 claims, Atlas).** Repaired by the next PR: the
  `value_form` rule of a block with a known `WeightBlock.scalar` carries the bare value's form
  (`scalar`), and the comparison accepts either form. With a snapshot assembled from the
  capture's recording, group 9's 8 `float` claims agree; group 8's 9 `int` claims stay different
  against a fixed-point bare value (as group 5: a CWT error).
- **`Replace` in the maximum comparison (group 6; 14 claims, Atlas).** The comparison should not
  read `Replace` as a maximum of 1: the engine processes repeats. Its own PR, with the ledger
  contract.
- **Key-reference lists (group 7; 12 claims, Native).** Native reports a `Reference` reader with
  shape `Scalar` and repeat `Accumulate`; vanilla writes a block of keys
  (`allowed_archetypes = { BIOLOGICAL LITHOID }`). Suspected: the key-reference list reader
  (`ReadKeyReferenceUniform`, in the repeat table of Native `registry-fields.md`) is classified as
  one scalar per occurrence.
