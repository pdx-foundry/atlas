# Recorded Native answers

`m452/` holds a sample of the Native answers that live `snapshot` runs recorded on M452
(Stellaris 4.5.2, build `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b`).
Every answer was recorded on 2026-10-20 with Native
`bb0cc0468fc3240150ff7dd077e2bdcc0691ef5c`, pinned in `Cargo.toml`, with no local path override
(commands of the [pin move B capture](../../../docs/coverage/pin-move-b.md#reproduce)). The callback answers use `callbacks/v5`;
the [entry-scope checks](../../../docs/coverage/entry-scope-checks.json) cover that method
(SDK-818). To refresh the recording, record a full run into an
empty directory, copy the answers, then trim the sample:

```sh
rm -rf .scratch/m452
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/m452 .scratch/rules.json
rm -rf tests/fixtures/native/m452
cp -R .scratch/m452 tests/fixtures/native/m452
cargo run --release --locked --example trim_recording -- tests/fixtures/native/m452
```

A change of the callback method needs the entry-scope checks again before the callback answers
are refreshed. Registries use `registry-directories/v8`, registry fields `registry-fields/v64`,
derived names `derived-names/v5`, modifier families `modifier-families/v5`, loader rules
`loader-rules/v8`, command declarations `command-declarations/v3`, command grammars
`command-grammar/v28`, callbacks `callbacks/v5`, fixtures `observe-fixture/v9` and the loaded
modifier table `loaded-modifiers/v1`; the other declarations use `v1`, except scope links
(`scope-links/v2`).
Every required answer property is explicit; tests read them with `Native::from_recorded_answers`
without a game. The M451-hotfix recordings remain at Atlas commit
`3c3472fb143eed1c28a9c2f50640559c8d9e4ea8`. `examples/record_callbacks.rs` records only the
static callback answers, without starting a game:

```sh
cargo run --release --locked --example record_callbacks -- "$STELLARIS_PATH" .scratch/callbacks
```

## The sample

The full recording is about 88 MB; the tests read a small part of it. `examples/trim_recording.rs`
holds the lists of what the sample keeps and is their only authority:

- The registries that tests read (traditions, tradition categories, council agendas, relics,
  buildings, technology, economic categories, pop jobs, `map/galaxy` and
  `interface/resource_groups`): their entries in `registries.json`, and their `registry_fields`,
  `derived_names`, `modifier_families` and `loader_rules` answers.
- The commands that tests read (`add_age`, `add_building`, `set_variable`, `has_modifier`,
  `is_mercenary`): their entries in the effect and trigger declarations, and their
  `command_grammar` answers.
- Four of the 45,583 loaded modifiers: `pop_happiness` (declared, loaded tags equal the static
  tags), `gdf_ship_alloys_cost_mult` (declared, loaded tags differ), `job_miner_add` (generated
  by the `common/pop_jobs` family for `miner`) and `shipclass_military_build_cost_mult`
  (unexplained). `registry_items` keeps only `miner`.

Every other answer is the unmodified recording. A trimmed answer keeps its source, completeness
and gaps, so its gaps can name registries, commands or modifiers that the sample left out.

The ignored `snapshot_live` test records every answer from a live game, checks the build's whole
population, and compares the live and recorded snapshots.
