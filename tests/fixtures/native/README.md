# Recorded Native answers

`m452/` holds a sample of the Native answers that live `snapshot` runs recorded on M452
(Stellaris 4.5.2, build `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b`).
`registry_fields/` was recorded on 2026-10-09 with Native
`0167bc5e4c163270ea17139cbffb0e94d8a41cde`, pinned in `Cargo.toml` (SDK-738). The other answers
were recorded on 2026-10-06 with Native `573e35f29fea8f54d32d48c90ff40c074e34b5f4`: the
[entry-scope checks](../../../docs/coverage/entry-scope-checks.json) apply only to
`callbacks/v2`, and the pinned Native answers `callbacks/v4`. To record it again, record a full
run into an empty directory, then trim it to the sample:

```sh
rm -rf tests/fixtures/native/m452
cargo run --release --locked -- snapshot "$STELLARIS_PATH" tests/fixtures/native/m452 .scratch/rules.json
cargo run --release --locked --example trim_recording -- tests/fixtures/native/m452
```

Registry fields use `registry-fields/v24`, command grammars `command-grammar/v15`, derived names
`derived-names/v1`, callbacks `callbacks/v2` and fixtures `observe-fixture/v7`. Every required
answer property is explicit; tests read them with `Native::from_recorded_answers` without a
game. The M451-hotfix recordings remain at Atlas commit
`3c3472fb143eed1c28a9c2f50640559c8d9e4ea8`. To refresh only the static callback answers, without
starting a game:

```sh
cargo run --release --locked --example record_callbacks -- "$STELLARIS_PATH" tests/fixtures/native/m452
```

## The sample

The full recording is about 40 MB; the tests read a small part of it. `examples/trim_recording.rs`
holds the lists of what the sample keeps and is their only authority:

- The registries that tests read (traditions, tradition categories, council agendas, relics,
  buildings, technology, economic categories, pop jobs, `map/galaxy` and
  `interface/resource_groups`): their entries in `registries.json`, and their `registry_fields`,
  `derived_names` and `modifier_families` answers.
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
