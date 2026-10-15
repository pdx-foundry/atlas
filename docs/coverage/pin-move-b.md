# Pin move B capture: 2026-10-15 (SDK-818)

One live run on M452 (Stellaris 4.5.2) at Native `2c94b30`, the `main` commit after every sweep
repair that SDK-818 waited for, gives these results. The baseline is the [Milestone 5 sweep](milestone-5.md) (SDK-798, Native
`ee01bf3`) with the SDK-815 join.

- **D1 gate.** The 218 config types under `common/` and `events/` have **6,095 / 18,228
  (33.44%)** Atlas-owned claims covered, from 4,316 (23.68%).
- **Headline.** **27,097 / 58,032 (46.693204%)** Atlas-owned config claims, from 24,460
  (42.149159%). The pin adds 1,347 claims and the new mappings add 1,290. No claim loses
  coverage. The ledger is byte-identical: 175 files, 61,146 claim occurrences and the same 19
  source diagnostics.
- **Native rates.** 5,060 of 7,477 relationship units (67.67%) are complete, from 3,985 / 7,511
  (53.06%). 3,291 gaps are open and 1,185 are accepted limits.

Coverage comes from the live snapshot and stays independent of agreement with CWT. Recorded answers
earn no coverage.

## Per-area coverage

| Area | Covered | Atlas-owned | Share | Pin only | SDK-798 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Effects | 7,981 | 13,085 | 60.99% | 7,981 | 7,564 |
| Triggers | 4,995 | 6,708 | 74.46% | 4,995 | 4,847 |
| Modifiers | 1,559 | 2,243 | 69.51% | 1,559 | 1,559 |
| Scopes and links | 754 | 825 | 91.39% | 754 | 754 |
| Localisation | 626 | 922 | 67.90% | 626 | 626 |
| On_actions | 431 | 1,514 | 28.47% | 431 | 349 |
| Game rules | 424 | 884 | 47.96% | 424 | 421 |
| Defines | 4,052 | 5,570 | 72.75% | 4,052 | 3,904 |
| Registry types and other files | 6,275 | 26,281 | 23.88% | 4,985 | 4,436 |
| **Total** | **27,097** | **58,032** | **46.69%** | **25,807** | **24,460** |

The areas are those of the [Milestone 4 capture](milestone-4.md#per-area-coverage). "Pin only" is
the same recording under Atlas `9547b99`: the new pin, without the mappings below.

## Where the 2,637 claims come from

The pin alone adds 1,347:

| Claims | Property | Areas |
| ---: | --- | --- |
| 837 | `cardinality_maximum` | Effects 363, registry types 344, triggers 130 |
| 239 | `value_form` | Registry types 95, defines 74, effects 52, triggers 18 |
| 143 | `field_existence` | Defines 74, registry types 67, effects 2 |
| 122 | `scope_context` | On_actions 82, registry types 37, game rules 3 |
| 6 | `naming_rule` | Registry types |

The largest files are `effects.cwt` (417), `triggers.cwt` (148), `common/defines/00_defines.cwt`
(144) and `common/planet_classes.cwt` (134). The sources are repeat behavior (SDK-627), the
planet-class answer (SDK-813), the define readers and on_action call sites that SDK-823's path
joins reach, and the other repairs listed in the [labelled deficits](#labelled-deficits).

The new mappings add 1,290, every one in a registry type:

| Claims | Property | Mapping |
| ---: | --- | --- |
| 1,259 | `cardinality_minimum` | [Required fields](#required-fields-sdk-829): 1,215 of minimum 0, 44 of minimum 1 |
| 22 | `enum_member` | [Enum domains](#enum-domains-sdk-627-sdk-828): `enum[scripted_modifier_category]` |
| 4 | `value_form` | The three keyword fields with a domain; the `planet_class_random_list` definition |
| 3 | `field_existence` | `planet_class_random_list` and its `name` and `planets` |
| 1 | `type_existence` | `type[planet_class_random_list]` |
| 1 | `loader_path` | `type[planet_class_random_list]` `path` |

## The mappings

The [ledger contract](ledger.md) states each rule.

### Planet classes (SDK-813)

Native answers `common/planet_classes` with owner `CPlanetClass` and one more root field,
`random_list`, a root statement whose members are the fields of `CPlanetClassRandomList`. The
config's `type[planet_class]` (`type_key_filter <> random_list`) is the root type and gets the
planet-class fields: 3 / 375 before, 195 now. `type[planet_class_random_list]` (`type_key_filter =
random_list`) is now answered through the `random_list` statement
([rule](ledger.md#root-statements-of-their-own-type-sdk-818)): 6 / 12. The other six stay open:
`name_field` (Native leaves `definition_names` unresolved), the value forms of `name` and `planets`
(their read conditions are unresolved), and the `planets` item's value form and cardinality.

### Enum domains (SDK-627, SDK-828)

Three fields have a `Listed` domain on M452: buildings `category` (9 names), and scripted_modifiers
`category` and economic_categories `modifier_category` (24 names each). Each one's value form is
now the keyword with its names, and the domain answers the members of the enum that the field's
config value names ([rule](ledger.md#enum-domains-sdk-818)). All 22 members of
`enum[scripted_modifier_category]` are credited; the comparison report finds 21 accepted and
`pop_job` rejected. The 12 members of `enum[building_category]` keep a gap: `num_buildings`
`category` also names that enum, and Native gives that command key no domain.

The weight-entry `calc` and `mode` domains (SDK-725) are inside weight blocks, for which Atlas
publishes no subject; SDK-740 owns them.

### Required fields (SDK-829)

`Field.requirement` is `Required` for 30 root fields, `Optional` for 1,736 and `Unknown` for the
others, which include 164 root fields. `Required` answers a config minimum of 1 and `Optional` a
minimum of 0 ([rule](ledger.md#required-fields-sdk-818)); a minimum above 1 keeps a gap. The
comparison report finds 1,211 of the 1,259 credited minimums the same as the config and 48
different. An `Optional` field can still be required when another field has a value (Native's
accepted limit), so a difference is a finding to read, not an error.

### `description_parameters` (SDK-741): kept as a gap

Native gives `description_parameters` the new `FieldMembers::AnyKey` form, with the written value
forms `Number`, `Text` and the keywords `long_float`, `num64` and `unum64`. The key is a fixed key of
the custom-description modifier block. The config's 27 `description_parameters` claims are under
`single_alias[modifier_clause]` and `triggered_modifier_clause_base` in `aliases.cwt`, and under
`custom_tooltip_with_params`, whose block keys Native leaves unresolved. Atlas publishes no subject
for a modifier block's fixed keys and has no `single_alias_right[X]` join, so these claims, and the
other modifier-clause claims, have no answer. Crediting them needs both joins.

### `common/colors`, `common/component_tags`, `common/ship_behaviors`

These three registries, and `gfx/worldgfx`, still answer `Unsupported`, each with its own reason,
which replaces the general one from [pin move A](pin-move-a.md#unsupported-registry-answers):

| Registry | Reason |
| --- | --- |
| `common/colors` | The loader reads fixed root keys of unnamed colours into its database and builds no item |
| `common/component_tags` | The loader registers each token of its files as a tag and builds no item |
| `common/ship_behaviors` | The loader reads each item in its own statement loop, not through a member reader |
| `gfx/worldgfx` | The loader reads the database itself, whose member reader builds and reads each item |
| `prescripted_countries` | Two databases load this directory |

Atlas keeps each one as a gap on the registry with Native's reason. `type[ship_behavior]` (94
claims) and `type[country_color]` stay uncovered; the config reads component tags as
`enum[component_tag]`.

## Callback entry scopes at `callbacks/v5`

The callback method moved from `callbacks/v4` to `callbacks/v5` (SDK-812). SDK-823 then gave 15
on_actions contexts that the path bound had cut. 16 on_action answers changed and no game rule
answer changed, so the [independent-check ledger](entry-scope-checks.json) was checked again for
those 16 ([details](ledger.md#callback-entry-scopes-sdk-704)) and now names `callbacks/v5`. 238
of 294 on_actions agree with the vanilla scope comments, from 234; 201 of 223 game rules agree, as
before. 35 names have no independent source, from 34.

## D1 gate

The gate population and its joins are those of the [Milestone 5 sweep](milestone-5.md#d1-gate):
the registry comparison is byte-identical (`registries.json` `3e8aa79a…`).

| Join | Types | Covered | Atlas-owned | Share | SDK-798 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `named` | 159 | 5,318 | 13,519 | 39.34% | 3,875 |
| `no_named_registry` | 39 | 0 | 2,260 | 0.00% | 0 |
| `several_types_in_directory` | 20 | 777 | 2,449 | 31.73% | 441 |
| **Total** | **218** | **6,095** | **18,228** | **33.44%** | **4,316** |

| Share | Types | SDK-798 |
| --- | ---: | ---: |
| 0% | 42 | 43 |
| under 25% | 35 | 48 |
| 25–50% | 55 | 74 |
| 50–75% | 74 | 50 |
| 75–100% | 11 | 2 |
| 100% | 1 | 1 |

128 types gain claims and none loses one. The largest gains:

| Type | Covered | Atlas-owned | SDK-798 |
| --- | ---: | ---: | ---: |
| `type[planet_class]` | 195 | 375 | 3 |
| `type[country_type]` | 318 | 628 | 181 |
| `type[ship_size]` | 306 | 701 | 181 |
| `type[megastructure]` | 293 | 403 | 195 |
| `type[building]` | 204 | 343 | 130 |
| `type[leader_class]` | 122 | 175 | 69 |
| `type[situation_type]` | 135 | 379 | 91 |
| `type[job]` | 100 | 266 | 61 |
| `type[civic_or_origin]` | 80 | 354 | 44 |
| `type[authority]` | 136 | 260 | 102 |

In `events/`, `type[event]` has 39 / 533: the 36 of the SDK-815 join and the `pre_triggers`
cardinality maximum of three kinds (repeat behavior, SDK-627). `type[event_namespace]` keeps 3 / 6.
The [event exceptions](milestone-5.md#events) are otherwise unchanged: the field-predicate subtypes
(SDK-716) and the `CEvent` members that have no one shared reader.

After the capture, SDK-860 credits the existence of 164 definition lines over the same snapshot
([rule](ledger.md#definition-lines-sdk-860)), 158 of them in the gate: 152 of `named` types and 6
of `several_types_in_directory`. No claim loses coverage. The gate is 6,253 / 18,228 (34.30%)
and the headline 27,261 / 58,032 (46.975806%). The definition lines' block forms and their 28
definition-wide scopes keep a gap with the reason that no Native answer states them.

## Native rates

Native's `examples/registry-sweep.rs` at `2c94b30` reads the run's recording through
`Native::from_recorded_answers`. Before is the [Milestone 5 sweep](milestone-5.md#native-rates-sdk-780).

| Relationship | Unit | Complete | Partial | Failed | Undetermined | Unsupported | Denominator | Rate | SDK-798 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Fields | registry | 104 | 76 | 0 | 0 | 5 | 185 | 56.22% | 52.97% |
| Reader kinds | root field | 1,605 | 37 | 288 | 0 | 0 | 1,930 | 83.16% | 76.12% |
| Shapes and conditions | root field | 1,505 | 151 | 274 | 0 | 0 | 1,930 | 77.98% | 58.92% |
| References | root field | 36 | 0 | 0 | 293 | 0 | 329 | 10.94% | 5.76% |
| Numerics | root field | 233 | 0 | 0 | 293 | 0 | 526 | 44.30% | 34.45% |
| Weights | root field | 98 | 0 | 0 | 364 | 0 | 462 | 21.21% | 0.00% |
| Naming rules | registry | 0 | 180 | 0 | 0 | 5 | 185 | 0.00% | 0.00% |
| Scope context | root field | 1,479 | 127 | 324 | 0 | 0 | 1,930 | 76.63% | 62.10% |
| **All units** | | **5,060** | | | | | **7,477** | **67.67%** | **53.06%** |

`common/planet_classes` adds 71 root fields (1,855 to 1,930). Fewer fields are undetermined, so
the reference, numeric and weight denominators shrink. Naming rules still have no complete unit:
every `modifier_families` answer keeps the unjoined generation sites.

`--diff` against the SDK-798 sweep baseline (Native's `tests/population/m452/registry-sweep.json`
at `15d230aa`, the `--baseline` output of the SDK-798 run, `5493f93f…`) names 184 registries. That
baseline predates SDK-819's split of open and accepted gaps, so each registry's gap rows show as
new; the relationship changes are fields 11, reader kinds 57, shapes and conditions 71, scope
context 82, references 46, numerics 48, weights 92 and naming rules 5. Six registries' fields
became complete (`common/ai_espionage/spynetworks`, `common/federation_perks`,
`common/federation_types`, `common/sector_types`, `common/specialist_subject_perks`,
`gfx/projectiles/planet_destruction`), `common/planet_classes` became partial, and the other four
`Unsupported` registries changed reason.

## Labelled deficits

Each failure-shape ticket of the [Milestone 5 deficits](milestone-5.md#labelled-deficits) except
SDK-609, SDK-610, SDK-711 and SDK-716 has merged in Native. What each changed, and what remains:

| Ticket | Before (SDK-798) | Now |
| --- | --- | --- |
| SDK-710 | 1,209 `UnresolvedReader` "no shared reader" gaps; reader kinds fail for 377 fields | Reader kinds fail for 288 fields. The `CEvent` members of `events` still have no shared reader. |
| SDK-627 | Repeat behavior and required fields unresolved; enum domains | Repeat behavior adds most of the 837 maximums above. Required fields (SDK-829) and three enum domains (SDK-828) are mapped. `enum[event_window_type]` has no domain. |
| SDK-635 | References: 24 of 417 fields complete | 36 of 329. |
| SDK-713 | Naming rules: unjoined generation sites on every family | Unchanged: no registry's naming rules are complete. |
| SDK-676 | Modifier containers of unjoined and nested fields | Carried to nested owners and unjoined fields. |
| SDK-707 | Derived names composed outside the owner's methods | Followed through interface callers of name getters. |
| SDK-609, SDK-610, SDK-711 | Localisation forms, custom define readers, command scopes | Open. SDK-823's path joins resolve every define read helper on M452; the Defines area gains 148. |
| SDK-716 | Claims under a field-predicate subtype | Open. |

SDK-741 and SDK-813 are the [mappings](#the-mappings) above. SDK-814 (`namespace`) and SDK-811
(loader conditions) are accepted limits, and SDK-812's entry-context gaps on blocks with no
`replace_scopes` claim are an accepted limit too.

## Capture and checks

| Input or result | Identity |
| --- | --- |
| Executable SHA-256 (Native's M452, Stellaris 4.5.2 (9776), Apple Silicon) | `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b` |
| Native revision (`main`, pinned in `Cargo.lock`) | `2c94b304052b6b1859f8bab475241f58b8d49226` |
| Atlas revision (clean tree) | `b5dd4f690860c4e9197828aada3f3b8b403a657e` |
| Config revision | `85747602a614ad7daa8cc66453777ecb023463a8` |
| Config content SHA-256 | `5c79cabb8d1b25e40994c0d134f5623bc2aef2a6b964059aabfc4834e6d039cc` |
| Live rule snapshot SHA-256 | `48e5323aa6bbf2bc9c367acbb1d780a6e3d42d1e1682971a82d15ebb0ab3b16c` |
| Snapshot ID | `stellaris-rules/7b05e36319e3913eb7ba6bd9fbd71cb2363623d9ad898d50eec8b2ba776c1ef3@1` |
| Recording: 2,933 files, 73 MB; SHA-256 of its `shasum -a 256` manifest | `2a42ed85158c2d5b97b4a33b3fbdda94e9839478a83d780ca5ec15d000f1b032` |
| `ledger.json` / `coverage.json` | `ba2bb263…f67947` / `217faa52…63503d` (pinned in full in the gate fixture) |
| `comparison.json` | `d85da838edf52d771ed1f546f9a0d88d31aa5021063794ae81822d1ada04f696` |
| `registries.json` | `3e8aa79aa6cab310e27a0dd9cebbe81a44e2a251ef2f512e6c3f151601a15b71` |
| Sweep report / `--diff` | `72633cd265bc408bec93e6c4db95526a99a284dd988384e3c4a5b1dddf93026a` / `5ca73defca054a562e4e59d674af45510402ee09117ef15841cae9865d9fb302` |

The methods of the run are `registry-directories/v6`, `registry-fields/v45`, `derived-names/v5`,
`modifier-families/v4`, `loader-rules/v5`, `command-declarations/v3`, `command-grammar/v22`,
`callbacks/v5`, `observe-fixture/v9`, `loaded-modifiers/v1`, `modifier-declarations/v1`,
`modifier-categories/v1`, `scope-declarations/v1`, `scope-links/v2`,
`localization-declarations/v1` and `defines/v1`.

The live CLI exited 0 in 4 minutes 24 seconds, with no local path override: `cargo tree -p
pdx-native` showed the Git revision. The main Atlas checkout's ignored `.cargo/config.toml`
patches Native to a local path, so every command ran from a directory outside it. An earlier live
run at the pin, before the mappings, recorded byte-identical answers. Ledger, comparison and
registries exit 2 for the retained source diagnostics, and registries also for Native's partial
registry list. The full-config gate passed with the live snapshot, and the ignored
`snapshot_live` test passed with a third live recording. Two review fixes after the run (a root
statement with no type of its own, and an enum named only by an unidentified type) change no
report of this config: `coverage.json` is byte-identical.

The full recording, the snapshot, the manifest and the sweep report and diff are on the capture
host, outside Git, at `.local/sdk-818/` of the main Atlas checkout. Tests use the trimmed sample in
`tests/fixtures/native/m452`, recorded by this run.

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
cargo test --release --locked --test snapshot_live -- --ignored
```

In a Native checkout at `2c94b30`:

```sh
git show 15d230aa:tests/population/m452/registry-sweep.json > .local/population/sdk-798.json
cargo run --release --example registry-sweep -- "$ATLAS/.scratch/m452" > .local/population/sweep.json
cargo run --release --example registry-sweep -- --diff .local/population/sdk-798.json .local/population/sweep.json
```

Run the live commands alone on the host: Native allows one game at a time.
