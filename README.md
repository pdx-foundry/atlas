# PDX Atlas

Atlas inventories the questions in CWTools config and measures which ones an Atlas snapshot can
answer. **Coverage is not agreement with CWT.** An evidence-backed contradictory answer earns the
same coverage as an agreeing answer. The config is a migration/discovery input, not the game oracle.

Atlas also asks its pinned Native dependency for tradition and tradition-category observations and
assembles the first offline rule snapshot. Native handles the game and platform details. The ledger
and coverage commands still run without a game installation.

```sh
cargo run --release --locked -- snapshot /path/to/Stellaris /path/to/recorded-answers /path/to/traditions.json
cargo run --release --locked -- snapshot --recorded tests/fixtures/native/m45 /path/to/traditions.json
```

Both modes write deterministic JSON and a `.sha256` sidecar. The snapshot name includes a digest
of its contents, so different builds and incomplete extractions have distinct identities.
Recorded answers keep their recorded basis and earn no current-engine coverage credit. The [version-1 schema](docs/contract/rule-snapshot-v1.schema.json)
describes the snapshot. The historical caller's findings and original source remain in the
preserved Native evidence bundle; the production tests cover its recorded-answer and fixture cases.

```sh
cargo run --locked -- ledger \
  --config /path/to/cwtools-stellaris-config/config \
  --snapshot /path/to/atlas-registry-result.json \
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
  /path/to/traditions.json \
  /path/to/reports
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
ATLAS_REGISTRY_SNAPSHOT=/path/to/normal-serial.json \
cargo test --locked --test full_config -- --ignored
```

The finished documentation-source measurement and its input identities remain in the
[historical report](docs/coverage/documentation.md). It is outside the product build.

Ordinary CI uses self-contained fixtures. The explicit full-config gate checks the pinned fork and
retained current registry capture, including exact deterministic report digests; it never launches
a game and fails if the required inputs are absent.
