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

## Covered disagreements at Native `5143009`

Every covered claim in `common/` and `events/` whose comparison status is `different`, from the
live M452 capture at Native `5143009` read by Atlas `main` with the line-bound shipped comments
of #48 (the ledger with `--game-content`, then `compare` with the script docs, the shipped defines
and the capture's recording). Only the comparable properties are compared (presence, loader
lines, basic value form, numeric cardinality, enum members, event-kind `push_scope`), so
documentation and scope blocks do not appear. Total: **575 claims** in 19 groups. The grouping
script is `.local/m5/disagreements.py` (`python3 -I disagreements.py REPORTS_DIR`).

Verdicts: **CWT error** — the engine fact contradicts the config, and the config should change;
**Atlas/Native defect** — Atlas's answer is wrong or incomplete, with the suspected cause;
**closer look** — the evidence does not decide yet. The engine facts come from Native's
`registry-fields/v61` and `loader-rules/v7` answers in the capture.

| # | Property | CWT | Atlas | Claims | Verdict |
| ---: | --- | --- | --- | ---: | --- |
| 1 | `cardinality_maximum` | `1` | repeat `Merges` | 199 | CWT error: a repeated block merges into the first, so the engine accepts more than one |
| 2 | `loader_path` (`path_extension`) | `.txt` | no suffix filter | 167 | CWT error: a template loader reads every file of any extension (Native `loader-rules.md`, "The file enumeration") |
| 3 | `cardinality_maximum` | `1` | repeat `Accumulate` | 76 | CWT error: each repeat appends, so the engine accepts more than one |
| 4 | `cardinality_minimum` | `1` | `0` (Optional) | 40 | CWT error, likely: the census loads a definition without the field with no diagnostic; a check that runs only when another field is set is not observed |
| 5 | `value_form` | `int` | `number` | 32 | CWT error: the reader is fixed point or float and accepts a fraction |
| 6 | `cardinality_maximum` | `inf` | repeat `Replace` | 13 | CWT error: a repeat replaces the earlier value, so only one occurrence counts |
| 7 | `value_form` | `block` | `reference` | 12 | Native defect, likely: these are lists of keys (`allowed_archetypes = { … }`), but Native reports one `Scalar` reference per occurrence (`Accumulate`) |
| 8 | `value_form` | `int` | `block` | 9 | Native defect, likely: the reader is the `Weight` family block reader, and vanilla also writes a plain number (`trait/cost = 1`); the value form names only the block |
| 9 | `value_form` | `float` | `block` | 8 | Atlas/Native defect, as #8 |
| 10 | `cardinality_minimum` | `0` | `1` (Required) | 6 | CWT error: the engine logs the field's absence (census) |
| 11 | `value_form` | `float` | `integer` | 4 | CWT error, likely: the reader is an integer reader |
| 12 | `loader_path` (`type_per_file`, `name_from_file`) | `yes` | names from the root key | 2 | Closer look: `common/diplomacy_economy` holds root keys (`once`, …) that CWT reads as fields of one per-file definition |
| 13 | `cardinality_maximum` | `20` | repeat `Accumulate` | 1 | Closer look: `policy/option`; the reader appends without a limit, but a use-time limit may exist |
| 14 | `loader_path` (`name_field`) | `localization` | no name field | 1 | Closer look: `common/start_screen_messages` |
| 15 | `scope_context` (`push_scope`) | `country` | `observer` | 1 | CWT error, likely: observer events are read in the observer scope |
| 16 | `loader_path` (`skip_root_key`) | `collection` | names from the root key | 1 | Closer look, likely Native defect: the shipped files wrap definitions in `collection = { … }` |
| 17 | `value_form` | `int` | `scoped_number` | 1 | CWT error: `scripted_action/cooldown` accepts a scripted value |
| 18 | `loader_path` (`path`) | `game/common/lawsuit_type` | `common/lawsuits` | 1 | CWT error (below) |
| 19 | `cardinality_maximum` | `0` | repeat `Replace` | 1 | Closer look: `deposit/subtype[!null]/is_null` uses `0..0` to describe the subtype, not an engine limit |

### Examples

1. **Merges.** `common/districts.cwt:159` `district/planet_modifier` (evidence
   `field:common/districts/planet_modifier#repeat_behavior`); `common/ascension_perks.cwt:65`
   `ascension_perk/modifier`; `common/strategic_resources.cwt:87` `resource/ai_wants`.
2. **Extension.** `common/agreements.cwt:55` `type[agreement_preset]/path_extension` (evidence
   `registry:common/agreement_presets#extension`); `common/star_classes.cwt:18`;
   `common/names_consolidated.cwt:21`.
3. **Accumulate.** `common/species_consolidated.cwt:311` `species_class/removed_planet_types`;
   `common/zone_slots.cwt:26` `zone_slots/excluded_zone_sets`; `common/buildings.cwt:249`
   `building/upgrades`.
4. **Optional, CWT requires.** `common/storm_types.cwt:101`
   `storm_types/cosmic_storm_texture_color_path` (11 storm-type fields); `common/asteroid_belts.cwt:9`
   `asteroid_belt_type/mesh`; `common/first_cntact.cwt:25` `first_contact_stage/stage_event`.
5. **Fixed point.** `common/ship_sizes.cwt:84` `ship_size/max_hitpoints`;
   `common/strategic_resources.cwt:42` `resource/subtype[tradable]/market_price`;
   `common/technologies_consolidated.cwt:192` `technology/subtype[repeatable]/cost_per_level`.
6. **Replace.** `common/traits.cwt:254` `trait/subtype[leader_trait]/custom_tooltip_with_modifiers`;
   `common/traits.cwt:251` `custom_tooltip`; `common/ascension_perks.cwt:95`
   `tradition_swap/custom_tooltip`.
7. **Reference lists.** `common/traits.cwt:350` `trait/subtype[species_trait]/allowed_archetypes`;
   `common/components.cwt:142` `component_template/size_restriction`; `common/megastructures.cwt:253`
   `megastructure/overclock_types`.
8. **Weight readers.** `common/traits.cwt:43` `trait/cost`; `common/traits.cwt:577`
   `trait/ai_weight`; `common/technologies_consolidated.cwt:115` `technology/subtype[!start]/cost`.
9. `common/buildings.cwt:62` `building/district_limit`; `common/bombardment_stances.cwt:33`
   `bombardment_stance/planet_damage`; `common/modifiers.cwt:85` `opinion_modifier/opinion`.
10. **Required, CWT optional.** `common/situations.cwt:224` `situation_type/stages` ("No stages
    detected"); `common/species_consolidated.cwt:250` `species_class/graphical_culture`;
    `common/patrons.cwt:272` `calling/tier`.
11. `common/planet_classes.cwt:118` `planet_class/extra_planet_count`; `common/components.cwt:354`
    `strike_craft_component_template/count`; `common/bypass.cwt:37` `bypass/ftl_multiplier`.
12. `common/diplomacy_economy.cwt:5-6`. 13. `common/policies.cwt:28`.
14. `common/start_screen_messenges.cwt:6`. 15. `events/events.cwt:43` (evidence
    `field:events/observer_event#read_scope`). 16. `common/country_container.cwt:5`.
17. `common/scripted_actions.cwt:205`. 19. `common/deposits_and_planetary_features.cwt:48`.

### Suspected defects to repair

- **Key-reference lists (group 7; 12 claims).** Native gives these fields a `Reference` reader
  with shape `Scalar` and repeat `Accumulate`, so Atlas answers the value form `reference`. Vanilla
  writes a block of keys (`allowed_archetypes = { BIOLOGICAL LITHOID }`). Suspected: the
  key-reference list reader (`ReadKeyReferenceUniform`, in the repeat table of Native
  `registry-fields.md`) is classified as one scalar per occurrence. Native work; with a
  uniform-list members state (queue row 8) Atlas would answer `block` and the `$item` claims.
- **Weight readers (groups 8–9; 17 claims).** Native gives these fields the `Weight` family block
  reader (`WeightBlock` members, repeat `Merges`), and Atlas answers `block`. Vanilla also writes a
  plain number (`trait/cost = 1`), so the value form should have a scalar alternative. Native
  work, or an Atlas mapping of the `Weight` family to "number or block" once Native states that the
  reader accepts a number.
- **`skip_root_key` (group 16).** Native names `common/country_container` definitions from the
  root key, but the shipped files wrap them in `collection = { … }`; Native may miss the skipped
  root key.
- **Closer looks (groups 12–14, 19; 5 claims).** `diplomacy_economy` (per-file definition or root
  statements), `policy/option` (a use-time limit of 20), `start_screen_messages` (`name_field`),
  and the `0..0` subtype convention of `deposit/is_null`.
