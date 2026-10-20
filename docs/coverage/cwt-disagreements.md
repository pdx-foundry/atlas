# CWT disagreement review

Milestone 5 closes only after each covered claim where Atlas and CWT disagree is reviewed: it is a
CWT error, or an Atlas or Native defect to repair (Native `docs/roadmap.md`, "Milestone 5
acceptance"). This page holds that review. Each entry names the config claim, Atlas's answer, the
evidence, and the verdict.

## Renamed directories

### `type[lawsuit]` (`common/lawsuits.cwt`)

| Claim | CWT | Engine | Verdict |
| --- | --- | --- | --- |
| `types/type[lawsuit]/path` (`loader_path`) | `game/common/lawsuit_type` | `common/lawsuits` | CWT error: the executable names only `common/lawsuits`, the game ships `common/lawsuits/00_example.txt` (whose guide documents `effect`, `cost` and `duration`), and Native's `common/lawsuits` registry reads those three fields. |

## Covered disagreements at Native `bb0cc04`

**Status: in progress.** Group 4 (41 claims) is open until its conditional checks are read; every
other group has a verdict. The review completes when group 4 does.

Every covered claim in `common/` and `events/` whose comparison status is `different`: **1,942
claims in 19 groups**. The membership, one line per claim with its group number, is
[`cwt-disagreements.tsv`](cwt-disagreements.tsv). A claim's group is the triple (property, the
first 30 characters of the config answer, Atlas's answer form), as listed in the table.

**Inputs.**

- Atlas `2fc635c` (main with #48 and #49, and #52, which compares every repeat behavior as
  `inf`), config revision
  `8574760` (`config_sha256` `5c79cabb…039cc`).
- The live M452 capture at Native `bb0cc04`: `rules.json` SHA-256 `8a6389f9…5804b`.
- `ledger --config … --snapshot rules.json --game-content $STELLARIS_PATH --output R`, then
  `compare … rules.json R --script-docs …/script-docs/v4.5.0 --defines
  $STELLARIS_PATH/common/defines/00_defines.txt --answers <capture>/m452`. Outputs: `ledger.json`
  `0c03b5a9…aca00`, `coverage.json` `89df3f9a…2ffe3`, `comparison.json` `411e17a5…45855`.
- Native methods behind the answers: `registry-fields/v64` (fields: repeat, requirement, value
  form, read scope), `loader-rules/v8` (extension, definition names) and
  `registry-directories/v8` (registry discovery: the content directory of `common/lawsuits`).

Only the comparable properties are compared (presence, loader lines, basic value form, numeric
cardinality, enum members, event-kind `push_scope`). Verdicts: **CWT error** — the config states a
fact that the engine contradicts (a wrong value form, a key or file the engine never reads);
**modelling difference** — the config states a cwtools modelling decision, not engine knowledge
(soft cardinality, subtypes: Native `docs/roadmap.md`, "The target, measured"), so neither side is
wrong; **Atlas defect** or **Native defect** — Atlas's answer is wrong or incomplete, with the
cause; **open** — not decided yet.

A repeat behavior says what the engine does with a repeated key; it is not an occurrence limit.
`Replace`, `Accumulate` and `Merges` all process a second occurrence and reject none (`Replace`
keeps the later value), so Atlas compares each as `inf` ([ledger](ledger.md), field subjects). A
finite config maximum against a repeat behavior is soft cardinality: the engine accepts the repeat,
and a compiler may warn about it.

| # | Property | CWT | Atlas | Claims | Verdict |
| ---: | --- | --- | --- | ---: | --- |
| 1 | `cardinality_maximum` | `1` | repeat `Merges` | 201 | Modelling difference: soft cardinality (a repeated block merges into the first; the engine accepts repeats, a compiler may warn) |
| 2 | `loader_path` (`path_extension`) | `.txt` | no suffix filter | 167 | CWT error: a template loader reads every file of any extension (Native `loader-rules.md`, "The file enumeration") |
| 3 | `cardinality_maximum` | `1` | repeat `Accumulate` | 76 | Modelling difference: soft cardinality (each repeat appends; the engine accepts repeats, a compiler may warn) |
| 4 | `cardinality_minimum` | `1` | `0` (Optional) | 41 | **Open.** The census loads a definition without the field and logs nothing, but a check that runs only when another field is set is not observed ([pin move B](pin-move-b.md): an `Optional` difference is a finding to investigate). Route: read each field's post-read hooks for a conditional absence check, or a fixture probe with the other fields set |
| 5 | `value_form` | `int` | `number` | 32 | CWT error: the reader is fixed point (`FixedPoint`) and accepts a fraction |
| 6 | `cardinality_maximum` | `1` | repeat `Replace` | 1,377 | Modelling difference: soft cardinality (a repeated key replaces the earlier value; the engine accepts repeats, a compiler may warn) |
| 7 | `value_form` | `block` | `reference` | 12 | Native defect, likely: lists of keys (`allowed_archetypes = { … }`) that Native reports as one `Scalar` reference per occurrence (`Accumulate`) |
| 8 | `value_form` | `int` | `block` | 9 | Atlas defect: Native reports the weight block's bare value (`members.WeightBlock.scalar`, a `FixedPoint` reader) for all 17 fields of groups 8–9, but Atlas's `assemble_field_value_form` answers `block` for every `Block` reader without reading it |
| 9 | `value_form` | `float` | `block` | 8 | Atlas defect, as #8 |
| 10 | `cardinality_minimum` | `0` | `1` (Required) | 6 | CWT error: the engine logs the field's absence (census) |
| 11 | `value_form` | `float` | `integer` | 4 | CWT error: the reader is an `Integer` reader |
| 12 | `loader_path` (`type_per_file`, `name_from_file`) | `yes` | names from the root key | 2 | CWT error: `common/diplomacy_economy` is a template loader (`loader-rules/v8`, `Known`), which names each root statement (`once`, …) by its key, not one definition per file |
| 13 | `cardinality_maximum` | `20` | repeat `Accumulate` | 1 | Modelling difference: soft cardinality (Native reads `policy/option` with repeat `Accumulate` into an array of `CPolicyOption` and states no limit, and the executable's strings hold no option-count diagnostic) |
| 14 | `loader_path` (`name_field`) | `localization` | names none | 1 | CWT error: `common/start_screen_messages` holds anonymous `part = { … }` statements, and Native states that the loader names none of its definitions (`Known` empty) |
| 15 | `scope_context` (`push_scope`) | `country` | `observer` | 1 | CWT error: observer events are read in the observer scope |
| 16 | `loader_path` (`skip_root_key`) | `collection` | names from the root key | 1 | CWT error: `common/country_container` is a template loader that names the one root statement `collection`; the `…_unit` blocks are its members, which no other shipped file names |
| 17 | `value_form` | `int` | `scoped_number` | 1 | CWT error: `scripted_action/cooldown` accepts a scripted value |
| 18 | `loader_path` (`path`) | `game/common/lawsuit_type` | `common/lawsuits` | 1 | CWT error ([renamed directories](#renamed-directories); evidence `registry-directories/v8`) |
| 19 | `cardinality_maximum` | `0` | repeat `Replace` | 1 | Modelling difference: subtypes (`deposit/subtype[!null]/is_null` is read on every path, `conditions` `Always`; CWT's `0..0` describes the subtype that the key selects, not a rule the engine applies) |

Totals: CWT error 10 groups (216 claims); modelling difference 5 groups (1,656 claims: soft
cardinality 1,655, subtypes 1); Atlas defect 2 groups (17 claims); Native defect 1 group (12
claims); open 1 group (41 claims). Group 6 was 14 `inf` claims against `Replace`, an artifact of
comparing `Replace` as a maximum of 1; #52 removed it, and the 1,377 `1` claims are the new group
6.

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
6. `common/governments.cwt:184` `authority/localization_postfix`;
   `common/graphical_cultures.cwt:61` `graphical_culture/selectable`; most in `ship_sizes` (92),
   `country_types` (80), `governments` (68), `scripted_actions` (66) and `megastructures` (64).
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

- **Weight blocks' bare value (groups 8–9; 17 claims, Atlas).** Repaired by #51: the
  `value_form` rule of a block with a known `WeightBlock.scalar` carries the bare value's form
  (`scalar`), and the comparison accepts either form. With a snapshot assembled from the
  capture's recording, group 9's 8 `float` claims agree; group 8's 9 `int` claims stay different
  against a fixed-point bare value (as group 5: a CWT error). The live numbers wait for a capture.
- **Key-reference lists (group 7; 12 claims, Native).** Native reports a `Reference` reader with
  shape `Scalar` and repeat `Accumulate`; vanilla writes a block of keys
  (`allowed_archetypes = { BIOLOGICAL LITHOID }`). Suspected: the key-reference list reader
  (`ReadKeyReferenceUniform`, in the repeat table of Native `registry-fields.md`) is classified as
  one scalar per occurrence.
