# PDX Atlas

Atlas inventories the questions in CWTools config and measures which ones an Atlas snapshot can
answer. **Coverage is not agreement with CWT.** An evidence-backed contradictory answer earns the
same coverage as an agreeing answer. The config is a migration/discovery input, not the game oracle.

Atlas also asks its pinned Native dependency for registry and tradition observations and for the
script language declarations: effects, triggers, modifiers and their categories and generated
families, scopes and scope links, localization, on_actions, game rules and defines. It assembles
them into one offline rule snapshot. Native handles the game and platform details. The ledger and
coverage commands still run without a game installation.

```sh
cargo run --release --locked -- snapshot /path/to/Stellaris /path/to/recorded-answers /path/to/rules.json
cargo run --release --locked -- snapshot --recorded tests/fixtures/native/m45 /path/to/rules.json
```

Both modes write deterministic JSON and a `.sha256` sidecar. The snapshot name includes a digest
of its contents, so different builds and incomplete extractions have distinct identities. The live
mode starts the game for the tradition fixtures and once more to read the loaded modifier table;
it exits 2 if an answer is missing or a game session did not confirm disposal.
Recorded answers keep their recorded basis and earn no current-engine coverage credit. The [version-2 schema](docs/contract/rule-snapshot-v2.schema.json)
describes the snapshot; [the language snapshot measurement](docs/coverage/language-snapshot.md)
states what it establishes and what stays a gap. The historical caller's findings and original source remain in the
preserved Native evidence bundle; the production tests cover its recorded-answer and fixture cases.

```sh
cargo run --locked -- ledger \
  --config /path/to/cwtools-stellaris-config/config \
  --snapshot /path/to/rules.json \
  --output /path/to/reports
```

Omit `--snapshot` to inventory with zero established coverage. The command writes `ledger.json`
and `coverage.json`. It returns 0 for a complete inventory, 2 after writing reports with source
diagnostics, and 1 for invalid arguments, unreadable input, or an invalid snapshot contract.
Output JSON contains input hashes and relative config paths, with no run timestamp.

To compare a rule-bearing Atlas snapshot with config assertions, run the separate test command:

```sh
cargo run --locked -- compare \
  /path/to/cwtools-stellaris-config/config \
  /path/to/rules.json \
  /path/to/reports \
  --script-docs /path/to/cwtools-stellaris-config/script-docs/v4.5.0 \
  --defines /path/to/Stellaris/common/defines/00_defines.txt \
  --answers /path/to/recorded-answers
```

It writes `comparison.json` with one entry per config claim plus Atlas-only questions. Entries
contain both answers where available and report `same`, `different`, `missing_from_atlas`, or
`atlas_only`. Gaps keep their reasons and count as missing answers. Only the directly comparable
properties (presence, loader path, basic value form, and numeric cardinality bounds) can be marked
the same or different; an unmapped property is reported as missing a comparable answer with its
raw Atlas answer retained. This report does not change coverage. A difference calls for review; it
does not establish which source is correct. The command exits 2 if config diagnostics remain.
Generic CWT `scalar` is compatible with a concrete scalar reader; CWT aliases and other modeled
forms remain unclassified until a semantic mapping exists.

The report also lists, for each config name list (effects, triggers, modifiers, modifier
categories, scope keywords, scope links, localisation commands and links, on_actions, game rules,
defines), the names that agree, the engine-only names and the config-only names. The options add
the same three lists for each `script-docs` log (names and, where both sides answer, descriptions,
usage, scopes and categories), for the installation's define files, and, from the recorded loaded
modifier answer, the loaded names and the declared modifiers whose loaded tags differ from their
static tags. The loaded table is a content observation, so the snapshot keeps only its counts.

The headline is supported engine-fact plus content-derived claims divided by the total in those
classes. Reports also give all-claims totals, every owner class, each file, and every claim's
assessment. Consumer policy and authored text are included in the ledger but excluded from the
headline. Empty denominators produce `null`, not 100%. If source diagnostics remain,
`inventory_complete` is false: percentages describe inventoried claims, and the omissions remain
visible. They are not quietly treated as covered.

The standalone Rust parser lives in [pdxscript-rs](https://github.com/pdx-foundry/pdxscript-rs).
Atlas pins its Git revision and uses its `cwt` module. Atlas owns classification and scoring;
the parser crate owns syntax.

See [the ledger contract](docs/coverage/ledger.md) for identities, source accounting, ownership,
snapshot input, and reproduction of the [initial measurement](docs/coverage/baseline.md).

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings

# Required local acceptance gate; the inputs must exist and match the recorded identities.
PDX_CONFIG_PATH=/path/to/cwtools-stellaris-config/config \
ATLAS_RULE_SNAPSHOT=/path/to/live-rules.json \
cargo test --locked --test full_config -- --ignored
```

The finished documentation-source measurement and its input identities remain in the
[historical report](docs/coverage/documentation.md). It is outside the product build.

Ordinary CI uses self-contained fixtures. The explicit full-config gate checks the pinned fork and
retained current registry capture, including exact deterministic report digests; it never launches
a game and fails if the required inputs are absent.

`Cargo.toml` pins Native by Git revision, and the checks above validate that revision. To try a
Native change before you push it, create `.cargo/config.toml` in this directory. Git ignores it.

```toml
[patch."https://github.com/pdx-foundry/native.git"]
pdx-native = { path = "../native" }
```

With the override, `cargo tree -p pdx-native` shows the local path. The override also changes the
`pdx-native` entry in `Cargo.lock`, so `--locked` commands fail. Build and test without
`--locked`, and run `git checkout Cargo.lock` before you commit. To check the pinned revision
again, move the override away, restore `Cargo.lock` and run the checks.
