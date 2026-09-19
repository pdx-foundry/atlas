# SDK-523 baseline

The retained registry capture gives **0 / 57,282 (0%)** coverage of Atlas-owned questions.
It observes 234 traditions and 33 tradition categories. These item names establish no field,
value-form, cardinality, scope, or other rule answers. Agreement with CWT is never scored.

| Owner | Covered | Total |
| --- | ---: | ---: |
| Engine fact | 0 | 54,196 |
| Content-derived | 0 | 3,086 |
| Consumer policy | 0 | 2,672 |
| Authored text | 0 | 192 |
| All claims | 0 | 60,146 |

All 172 config files were read. All 42,973 meaningful lines have claim or diagnostic links;
the files contain 49,201 physical lines, including five final lines without a newline. Every
engine-fact claim has an expected method and roadmap ticket. Documentation ownership remains
provisional pending SDK-525. An expected method is not qualified evidence.
The denominator includes independent minimum and maximum questions for 6,584 cardinality ranges.
In the 20 mixed soft/hard ranges, policy coverage cannot substitute for the hard engine bound.

## Inputs

- Config revision: `76be5782683d09a9dbe304a512690eed6a2fa425`.
- Sorted config manifest SHA-256: `ad0637833d99e28c8f8eb1a4cd5f835d6b92d65d1183b1a16a062ce76f98b8df`.
- Retained SDK-519 registry capture: Native's `.local/sdk-519/live/normal-serial.json`.
- Capture SHA-256: `b87a98b809620b0fa1ec1b532f540ed1824d0967c4c30393fb55f7fbbb233b93`.
- Parser Git revision: `a5458bea619f3e7620848d079a9c28b180546dd6`.

The capture hash matches Native's `docs/native/atlas-consumer-verification.json`. The test uses
the retained capture; it does not launch the game or refresh evidence.

## Unresolved source

Eight diagnostics remain visible. Both reports are written, then the CLI exits with status 2.
`inventory_complete` is false: complete line accounting does not imply complete interpretation.
Reported fractions cover the inventoried claims and must not conceal these gaps.

| File | Lines | Diagnostic |
| --- | --- | --- |
| `common/country_types.cwt` | 323, 324 | Prose using semantic `##` annotation syntax |
| `game_rules.cwt` | 14, 15 | Uninterpreted `Root` and `This` annotations |
| `gfx/particles.cwt` | 533, 542, 551 | `cardinality = 2` is not a supported range |
| `triggers.cwt` | 5160 | Uninterpreted capitalized `Scopes` annotation |

All source text is retained. No config correction is inferred. The three scalar cardinalities
also fail the pinned SDK reader's range grammar; inspecting those added diagnostics justified
updating the baseline from five to eight diagnostics. This is a syntax-accounting check, not a
judgment about the config's game facts.

## Reproduce

Set `PDX_CONFIG_PATH` to the fork's `config` directory and `ATLAS_REGISTRY_SNAPSHOT` to the retained
capture, then run:

```sh
cargo run --locked -- ledger --config "$PDX_CONFIG_PATH" \
  --snapshot "$ATLAS_REGISTRY_SNAPSHOT" --output .scratch/sdk-523
cargo test --locked --test full_config -- --ignored
```

The first command's exit 2 is expected for this baseline. The acceptance test must pass and fails
if either required input is absent or changed. The fixture pins input hashes, counts, diagnostics,
observations, and both complete output digests:

- `ledger.json`: `b17ab1da853cae63fb6efeb8093a54b16696ed0e8dec86cc3c67757b9a61dda8`.
- `coverage.json`: `a54dc3d5ece6be42b1db5c0f4d11be8c3d9be90c5dc0cdaebf14d22e6cbc7686`.

Repeated runs and relocated copies of the full input must produce those same bytes. Game-free CI
runs the focused inventory, evidence, and command tests; it does not substitute synthetic data for
the required full-config acceptance run.
