# Pin move A capture: 2026-10-12 (SDK-796)

One live run on M452 (Stellaris 4.5.2) at Native `ee01bf3` establishes **24,458 / 58,032
(42.145713%)** Atlas-owned config claims. The committed baseline before it was the
[Milestone 4](milestone-4.md) live snapshot under the same Atlas code: 22,995 (39.624690%). The
run adds 1,463 claims and no claim loses coverage. The ledger is byte-identical: 175 files, 61,146
claim occurrences and the same 19 source diagnostics. Coverage stays independent of agreement with
CWT, and recorded answers earn none.

The snapshot has 69,254 rules, 29,693 gaps and 12,609 subjects (Milestone 4: 56,061, 24,276 and
11,171). Native lists 185 registries, from 164: the custom-loader registries of SDK-783 and
SDK-786. This capture is an intermediate pin move. SDK-798 records the Milestone 5 sweep.

## Per-area coverage

| Area | Covered | Atlas-owned | Share | Previous |
| --- | ---: | ---: | ---: | ---: |
| Effects | 7,564 | 13,085 | 57.81% | 7,430 |
| Triggers | 4,847 | 6,708 | 72.26% | 4,829 |
| Modifiers | 1,559 | 2,243 | 69.51% | 1,559 |
| Scopes and links | 754 | 825 | 91.39% | 754 |
| Localisation | 626 | 922 | 67.90% | 626 |
| On_actions | 349 | 1,514 | 23.05% | 336 |
| Game rules | 421 | 884 | 47.62% | 406 |
| Defines | 3,904 | 5,570 | 70.09% | 3,904 |
| Registry types and other files | 4,434 | 26,281 | 16.87% | 3,151 |
| **Total** | **24,458** | **58,032** | **42.15%** | **22,995** |

The areas are those of the [Milestone 4 capture](milestone-4.md#per-area-coverage).

## Where the 1,463 claims come from

| Claims | Property | Main sources |
| ---: | --- | --- |
| 490 | `cardinality_maximum` | Registry types 373, effects 101, triggers 15, `events/events.cwt` 1 |
| 402 | `value_form` | Registry types 353, effects 28, `overrides.cwt` 16, `events/events.cwt` 4, triggers 1 |
| 227 | `field_existence` | Registry types 200, `overrides.cwt` 16, effects 5, `events/events.cwt` 4, triggers 2 |
| 221 | `loader_path` | Registry types 218, `events/events.cwt` 3 |
| 101 | `scope_context` | Registry types 52, `events/events.cwt` 21, game rules 15, on_actions 13 |
| 19 | `type_existence` | Registry types 18, `events/events.cwt` 1 |
| 3 | `naming_rule` | Registry types |
| **1,463** | | |

The largest files are `effects.cwt` (134), `common/country_types.cwt` (101),
`common/special_projects.cwt` (90), `common/personalities.cwt` (81),
`common/megastructures.cwt` (62), `common/strategic_resources.cwt` (53) and
`common/dust_clouds.cwt` (51). The sources are the fields of the new registries (SDK-787), the
loader-rule joins (SDK-797, first on a live run; the `overrides.cwt` lines and most `loader_path`
lines), event kinds and their `pre_trigger` keys (SDK-793, `events/events.cwt`), and the newer
`registry-fields` and `command-grammar` methods.

## Registry comparison

The [registry comparison](registries.md#rerun-at-native-ee01bf3-sdk-796) gives the counts and
each moved type. In the gate population, `named` grows from 146 to 159 types and
`several_types_in_directory` from 12 to 20; `no_named_registry` falls from 60 types in 35
directories to 39 types in 18 directories.

## Unsupported registry answers

Six listed registries have no answer for `registry_fields`, `derived_names` and
`modifier_families`. Native answers each with `Unsupported` and a reason:

| Registry | Reason |
| --- | --- |
| `common/colors`, `common/component_tags`, `common/ship_behaviors`, `gfx/worldgfx` | The loader builds no class with a member reader. |
| `common/planet_classes` | The loader builds several classes with a member reader (`CPlanetClass`, `CPlanetClassRandomList`). |
| `prescripted_countries` | Two databases load the directory. |

This is Native's stated answer for a listed registry (SDK-783: an answer, a partial answer or
`Unsupported`, never `UnknownRegistry`). Atlas keeps each one as a gap on the registry and counts it
as an arrived answer, so the live CLI exits 0. Any other error still makes the extraction
incomplete.

## Capture and checks

| Input or result | Identity |
| --- | --- |
| Executable SHA-256 (Native's M452, Stellaris 4.5.2 (9776), Apple Silicon) | `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b` |
| Native revision (`main`) | `ee01bf338872290eaaab2ca3c0503fc33244686e` |
| Atlas revision (clean tree, branch of SDK-796) | `3e24c680dd8f493ca13d42d6d60ac7ce654b216e` |
| Config revision | `85747602a614ad7daa8cc66453777ecb023463a8` |
| Config content SHA-256 | `5c79cabb8d1b25e40994c0d134f5623bc2aef2a6b964059aabfc4834e6d039cc` |
| Live rule snapshot SHA-256 | `d2208992c5d7f0b6d281907ffa76ef01cebe539ba3ff3e8176170b2416672728` |
| `ledger.json` / `coverage.json` | `ba2bb263…f67947` / `92c137bb…4264cf` (pinned in full in the gate fixture) |
| `comparison.json` | `f402d95377464ab3609beafa09b708af3f69174086d3cf6a5c6bc1d719e31335` |
| `registries.json` | `3e8aa79aa6cab310e27a0dd9cebbe81a44e2a251ef2f512e6c3f151601a15b71` |

The methods of the run are `registry-directories/v6`, `registry-fields/v33`, `derived-names/v1`,
`modifier-families/v3`, `loader-rules/v5`, `command-declarations/v3`, `command-grammar/v18`,
`callbacks/v4`, `observe-fixture/v9`, `loaded-modifiers/v1`, `modifier-declarations/v1`,
`modifier-categories/v1`, `scope-declarations/v1`, `scope-links/v2`,
`localization-declarations/v1` and `defines/v1`.

The live CLI exited 0 in 3 minutes 29 seconds, with no local path override (`cargo tree -p
pdx-native` shows the Git revision). An earlier run of the same commit, before the change that
counts `Unsupported` answers as arrived, exited 2; its recording and snapshot are byte-identical to
this run. Ledger, comparison and registries exit 2 for the retained source diagnostics; registries
also for Native's partial answer (13 of 198 scheduled databases are unnamed). The ignored
`snapshot_live` test ran a third live recording: 185 registries, every disposal confirmed, and the
live and recorded snapshots match after normalization.

Tests use `tests/fixtures/native/m452`, recorded by this run and trimmed to the sample that its
README states.

## Reproduce

```sh
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/m452 .scratch/rules.json
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

Run the live commands alone on the host: Native allows one game at a time.
