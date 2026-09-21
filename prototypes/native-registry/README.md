# Atlas Native consumer

This tracked caller freezes the SDK-534 tradition and tradition-category question flow. It uses
Native's public API only. Atlas owns the question list and the mapping to
`config/common/traditions.cwt` in `src/frozen/questions.rs`; Native owns the engine facts.
The older `describe`, `live`, and `recorded` commands and `request-contract.json` retain
the bounded SDK-519 item-list experiment.

Native is pinned to merged commit
`483f57db1d3b8965bf0af4a903ddfc74e30cbefb` in `Cargo.toml` and `Cargo.lock`.
The M45 answers in `../../tests/fixtures/native/m45` were captured from the exact supported installation on
2026-09-21. The previous prototype and synthetic captures are preserved in
`~/Documents/PDX/evidence/native-2026-09-18/atlas-native-consumer-before-simplification.tar.gz`.

## Frozen flow

```sh
cargo run --release --locked -- frozen /path/to/Stellaris ../../tests/fixtures/native/m45
cargo run --release --locked -- frozen-recorded ../../tests/fixtures/native/m45
```

`frozen` records every Native answer and prints a JSON report after closing four independent
sessions: baseline registry item names, tradition field outcomes, category field outcomes, and
category read entries. The tradition fixture includes valid, omitted, repeated, malformed, and
unknown-field input. Only the baseline session asks for item names, because Native's recorded
item key is a registry name, not a fixture-session key. `frozen-recorded` asks the same questions
without a game or supervisor process.

The report separates `native_support`, whole `answers`, and per-question `coverage`.
Coverage is `Observed`, `Gap` with an owning SDK ticket, or `Unanswered` with the Native
error. A partial storage window is never called observed. No field or category is declared wholly
valid. The saved M45 recording resolves 89 questions: 48 observed, 41 owned gaps, 0 unanswered.
The category storage and diagnostic gaps remain visible; parser diagnostics do not establish
later validation or runtime behavior. Tree-template registry discovery is an SDK-551 gap.

Recorded answers have `Basis::Recorded`; live answers have their original basis. The release
live/recorded parity test compares all answers after normalizing only that basis and excluding
session readiness and disposal. Live disposal was confirmed in each of the four sessions; recorded
disposal is `NotApplicable`.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
STELLARIS_PATH=/path/to/Stellaris cargo test --release --locked --test frozen_live -- --ignored
```

Run the ignored boundary check from the Native repository:

```sh
ATLAS_CALLER_PATH=/path/to/pdx-atlas/prototypes/native-registry cargo test --test consumer_boundary -- --ignored
```

The live test must use a release build. A debug-build attempt on this host timed out before any
worker trace was produced; Native's release controls and this caller's release flow passed. The
game-free tests include a missing recorded answer and an authored Native error, and verify that
each blocks only its dependent questions.
