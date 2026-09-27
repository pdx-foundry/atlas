# Shared-reader snapshot: SDK-597 and SDK-577

The first Milestone 4 Atlas capture establishes **15,979 / 58,032 (27.534808%)** Atlas-owned
claims. All 175 config files and 61,146 claim occurrences were inventoried. The same 19 source
diagnostics remain; these percentages describe inventoried claims, not a complete config model.

Compared with the preceding language snapshot, 701 additional claims receive coverage: 592 field
existence claims and 109 value-form claims. No formerly covered claim loses credit. The largest
increases are in ship sizes (103), megastructures (56), country types (55), personalities (48),
and situations (46). These are established reader facts, not agreement with CWT.

Ambiguous directory joins now leave explicit gaps on **586 config claim occurrences**: 12 type-existence, 12 loader-path, 134 field-existence,
134 value-form, 133 minimum-cardinality, 133 maximum-cardinality and 28 scope-context claims. Their directory alone
cannot select the applicable CWT type. These gaps remain uncovered in the coverage report and
retain their mapping reason in comparison. They are not silently dropped as Atlas-only facts.

The snapshot has 27,606 rules and 16,213 gaps. Paired read alternatives, shapes, block families,
nested members, and stored-value selections remain attached to typed field subjects. Unknown
branches remain gaps. CWT subtype selectors are not assumed equivalent to Native field predicates.
Defaults, exhaustive domains and occurrence bounds remain unestablished. Later shared-reader
claims and the final Milestone 4 measurement remain SDK-625; unassigned-gap triage remains SDK-626.

## Evidence and identities

The live CLI completed successfully, including Native-confirmed disposal of both fixture sessions
and the loaded-modifier session. Offline assembly from its unmodified recorded answers matches
the live snapshot after normalizing source basis and snapshot identity. Recorded sources retain
`Recorded` basis and receive no current-engine coverage credit.

- Capture date: 2026-09-26 (local).
- Atlas source revision: `cd40bd4` (full revision in the full-config baseline).
- Native source revision: `c33a3fc1bc5387adf14c0d30e6d8e30de6d03cd1`.
- Parser source revision: `ccb681ac10af4a4efb21ec42450b1a04d2d8500a`.
- Exact game build: `07988b4f1b865623becd7a61af1cae92e111be6515d341754af70f02107822cd`.
- Config revision: `85747602a614ad7daa8cc66453777ecb023463a8`.
- Config SHA-256: `5c79cabb8d1b25e40994c0d134f5623bc2aef2a6b964059aabfc4834e6d039cc`.
- Live snapshot SHA-256: `7e74dbc868c5e350a46407344872926e4b66a8ae03dbe77ab778a29162cfee23`.
- Ledger SHA-256: `ba2bb263812b54f4f3b6d28a7b47f56b92a0be5f3f7f8da41aa6c375f2f67947` (unchanged).
- Coverage SHA-256: `d24fe4990955c603b017aa53e934b4ddeb4a1dc94ca5528e36b58b5d07f6722c`.
- Comparison SHA-256: `90bf6e20c1a5c1017892496184c6557f8528d9b7b788bfc01652de6d2693e0a6`.

The full live snapshot, raw answers and reports are retained at
`/Users/jackson/Developer/pdx-foundry/atlas/.scratch/sdk-597/review`. The tracked fixtures contain the
fresh Native recordings, with only the loaded modifier table reduced as described in their README.

SDK-577 adds explicit `display_name` rules. The config name lists, all five script-doc comparisons,
define comparisons, and loaded-modifier tag comparisons are identical to the retained SDK-570
report. The full comparison digest necessarily changes because it includes snapshot identity and
new Atlas-only field rules. No display names or subject kinds are reconstructed from identity text.

## Reproduction

From the Atlas checkout, with the exact installed game and config fork:

```sh
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/sdk-597/answers .scratch/sdk-597/snapshot.json
cargo run --release --locked -- ledger --config "$PDX_CONFIG_PATH" \
  --snapshot .scratch/sdk-597/snapshot.json --output .scratch/sdk-597/reports
cargo run --release --locked -- compare "$PDX_CONFIG_PATH" .scratch/sdk-597/snapshot.json \
  .scratch/sdk-597/cmp --script-docs "$PDX_CONFIG_PATH/../script-docs/v4.5.0" \
  --defines "$STELLARIS_PATH/common/defines/00_defines.txt" \
  --defines "$STELLARIS_PATH/common/defines/00_defines_additional_content.txt" \
  --answers .scratch/sdk-597/answers
ATLAS_RULE_SNAPSHOT=.scratch/sdk-597/snapshot.json \
  cargo test --locked --test full_config -- --ignored
```

Ledger and comparison exit 2 for the 19 retained config diagnostics. Snapshot creation exits 0.
The pinned gate checks the exact inputs, coverage count and deterministic report digests.

The review correction retains three registry-level `field_gap_subjects` gaps for Native leaf
names that cannot select one nested path. Field evidence no longer inherits unrelated registry
gaps. Older version-2 ticket owners remain readable, while new publications use categories.
