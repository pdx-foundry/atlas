# Simplification capture: 2026-10-02

The M451-hotfix snapshot establishes **16,341 / 58,032 (28.158602%)** Atlas-owned config claims.
Compared with the [shared-reader capture](shared-readers.md), 362 claims gain coverage and none
lose it: 359 `Replace` facts answer maximum questions of 1, and three `Accumulate` facts answer
unbounded maxima. Coverage remains independent of agreement with CWT. No engine maximum is
published, and no finite fixture result establishes a required field.

The ledger is byte-identical to the preceding capture: 175 files, 61,146 claim occurrences and
the same 19 source diagnostics. Percentages describe inventoried claims. The new snapshot has
28,187 rules and 14,027 gaps. It adds 580 repeat rules (574 Replace, six Accumulate), plus one
parser-storage rule for category `tree_template`. It removes 1,605 default gaps, 580 maximum gaps
and one runtime gap. Twenty-two unresolved-value-form gaps become reference gaps; one storage
and one parser-outcome gap disappear. Two field-answer completeness gaps are new.

## Capture and checks

- Native: `92b27b3eaf2f7021ad9e8105ad9818edd0b7ec19` on the review branch; squash-merged to `main` as
  `2a23b2bacb345baab416d1c2cb5a50b0ddb569b4` with the review repairs, which Atlas pins.
- Atlas: the simplification migration based on `3b1dc763723050b2f090327d6e61e789d6551cfb`.
- Exact build: `29fa877366040a528098da39ec7e70b7baac76782a2a6bd161616d691f86fa38`.
- Config: `85747602a614ad7daa8cc66453777ecb023463a8`.
- Snapshot SHA-256: `de168c9538457a8433ceca17c776b3618d41137ca3403ab0188bd27d82f9671f`.
- Ledger SHA-256: `ba2bb263812b54f4f3b6d28a7b47f56b92a0be5f3f7f8da41aa6c375f2f67947`.
- Coverage SHA-256: `a00f142a2b256b33a56472e273d3771841572f96cb0c8c836cc4c864420f7888`.
- Comparison SHA-256: `cb49627950ce9630e50e9e3d6f41605aeba557b038367f21067ab407d8e701b6`.

Config name-list comparisons, all script-doc comparisons, define comparisons and loaded-modifier
tag comparisons are identical to the previous capture.

The live CLI exited 0 and confirmed disposal of both fixture games and the loaded-modifier game.
Full live and recorded snapshots match after source basis, snapshot identity and operation
availability are normalized. Recorded support now marks the three unrecorded operations
(`check_script`, `command_grammar`, `dynamic_names`) unsupported. Recorded sources earn no
current-engine coverage. The v2 schema accepts Native's numeric-conversion and nested-key gaps.

Full answers and reports remain in `.scratch/simplification-2026-10-02/`. Tests use fresh
`tests/fixtures/native/m451-hotfix` answers, with the same four-entry loaded-modifier reduction
as before; their README states that reduction. The full-config gate pins the fresh input and
report digests. Ledger and comparison exit 2 for the retained source diagnostics.

## SDK-626 failure shapes

Ticket a shape only when closing it changes what a compiler accepts, rejects, types or completes
and a config claim depends on it. These measured counts link raw gaps to config claim occurrences;
they overlap, include ambiguous directory joins, and must not be added together.

| Shape | Raw gaps | Linked claims | Compiler consequence |
| --- | ---: | ---: | --- |
| Command argument grammar | 2,170 | 12,212 | Accepted arguments and types, e.g. `abort_situation` |
| Required-field evidence absent | 1,605 | 1,237 | Missing-field diagnostics |
| Unconditional repeat behavior unknown | 1,025 | 818 | Duplicate-field warnings and acceptance |
| Command entry scope | 2,170 | 567 | Scope checking and completion |
| Unresolved reader value form | 508 | 450 | Scalar, block and type acceptance |
| Nested grammar | 422 | 394 | Accepted child keys |
| Reference target | 284 | 260 | Name completion and invalid-reference diagnostics |
| Field block scope | 422 | 226 | `this`, `root` and `from` types |
| Callback entry scopes | 517 | 497 | Scope completion; 264 claims involve self-link uncertainty |
| Conditional value form | 50 | 48 | Acceptance under an established condition |

The 571 runtime modifier-application gaps are **out of scope, vision 2026-10-02**. The 32 category
`supported_scopes` gaps touch 153 claims, but justify only SDK-547's agreed static node/container
work. They do not justify runtime application proofs. Removed default/runtime placeholders get
no tickets. Other shapes require a named dependent claim and compiler consequence before triage.

The new `fields_complete` gaps for `common/resource_regions` and `common/leader_tiers` contain
numeric-conversion uncertainty. Their existing root-search label does not establish missing
fields; classify them by the attached Native gaps, not as discovery failures. This measurement
does not create tickets from counts alone. The [unowned gap triage](gap-triage.md) gives each gap
without an owner its disposition.

## Reproduce

```sh
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/simplification-answers .scratch/simplification-rules.json
cargo run --locked -- ledger --config "$PDX_CONFIG_PATH" --snapshot .scratch/simplification-rules.json --output .scratch/simplification-ledger
ATLAS_RULE_SNAPSHOT=.scratch/simplification-rules.json cargo test --locked --test full_config -- --ignored
```
