# Historical documentation provenance measurement

SDK-525 measured every documentation claim in the config ledger against explicit source inputs.
This is text attribution, not proof that the documented behavior is correct. No match creates an
Atlas answer or raises verified rule coverage. The measurement is complete and its code was
removed from the product build at milestone 2. Its source and tests remain in Git revision
`1521c2a44c1139fcd1a8f8c0b8c720a60c7dcb56`. The tracked
`tests/fixtures/documentation-baseline.json` retains the input and report hashes.

To reproduce it, check out that revision in a separate worktree and use the original
source inputs named by the baseline fixture:

```sh
cargo run --locked -- ledger --config "$PDX_CONFIG_PATH" \
  --snapshot "$ATLAS_REGISTRY_SNAPSHOT" \
  --game-content "$PDX_GAME_CONTENT" --engine-docs "$PDX_ENGINE_DOCS" \
  --output .scratch/sdk-525
```

`PDX_GAME_CONTENT` is the installation root. Atlas reads its base-game `common/**/*.txt`, including
commented guides. It does not mount DLC archives or mods. `PDX_ENGINE_DOCS` contains `effects.log`
and `triggers.log`. The two source flags must be supplied together; without them the original
inventory remains unmeasured and its provisional owners are unchanged. Inputs must be readable
UTF-8 files, not symlinks. Empty engine dumps are rejected.

## Output and interpretation

Every documentation claim receives `provenance`, with an `origin` of `engine_text`,
`shipped_comment`, or `authored`; a `comparison` of `exact`, `rewritten`, or `none`; and the best
source locations. Each location carries a relative path, original file SHA-256, inclusive physical
line range, key path, association kind, and extracted source text. Equal best locations are all
retained. Ledger identities do not change. Ownership follows the found source; rewritten candidates
retain `provisional_owner=true`.

`documentation.json` contains counts by file and family, the complete input-hash manifest, parser
diagnostics, and explicit lists of authored and rewritten claim IDs. Join those IDs to `ledger.json`
for config paths, lines, subjects, and full text. Adjacent `###` and `####` lines form one entry,
as in the original ledger. Both entry and physical documentation-line counts are reported.

**Authored means no matching source was found in this corpus.** It does not prove historical
human authorship or that no source exists elsewhere. Version drift, absent content, unrecognized
comment layouts, and parsing limits can all contribute. Authored entries are outside the headline
coverage denominator and are not game evidence. Rewritten matches are candidate attributions,
not confirmed derivations or semantic equivalence.

## Association and comparison rules

Atlas uses the standalone script parser for active key paths and spans. It binds leading comments
to the next key, inline comments to the key on that line, and comments inside empty blocks to that
block. Define groups inherit a leading comment across adjacent sibling entries until a blank line
or another leading comment. Namespace boundaries remain significant.

Shipped guides also use `key: prose`, `key -> prose`, `key - prose`, and commented assignments with
inline or following prose. Atlas reads those as named documentation. Files named for examples,
README, or documentation also get a byte-position-preserving view of commented assignments for
structural associations. These examples are not required to be valid game definitions. Parsing
problems remain in the report; explicit named comments can still supply a text source. No game file
is changed. Quoted hashes, escaped quotes, multiline quoted strings, repeated keys, and CRLF line
endings retain their source meaning.

Engine entries are identified by command name; descriptions and usage are kept, scope metadata is
excluded. Nested config documentation can match text within its enclosing engine command. Content
candidates must share a key and be inside a content directory selected from the config's loader
paths (or its relative file stem), including descendant directories. Defines additionally require
their namespace path. This is a text-source join, not a qualified mapping between a CWT field and
an engine reader.

An exact match removes comment markers and folds whitespace only. It can select a consecutive
range of source lines, allowing a description to match without copying the following usage example.
It does not remove punctuation, change case, or delete CWT continuation backslashes. Rewritten
candidates require at least eight words on each side and at least 85% similarity by case-insensitive
word edit distance, divided by the longer word count. Short generic text is never matched fuzzily.
Only equally best candidates are retained, with exact matches taking precedence.

A source parse problem sets `source_parse_complete=false` and the CLI exits 2 after writing all
reports. Config diagnostics also retain exit 2. An unreadable input or invalid argument exits 1.
Full attribution is therefore separate from full source interpretation.

## Full measurement, 2026-09-19

The config is revision `76be5782683d09a9dbe304a512690eed6a2fa425`, manifest
`ad0637833d99e28c8f8eb1a4cd5f835d6b92d65d1183b1a16a062ce76f98b8df`.
The source corpus contains 2,060 installed content files and the fork's two `script-docs/v4.4.1`
logs. Its sorted input-manifest JSON hash is
`3d744becf7976e9ddd52e57f2d92cbaaf0b56f60b57af61852e45052519d3007`.
These historical engine logs and installed content are separate inputs, not one qualified target.

| Family | Entries | Doc lines | Exact | Rewritten candidates | Authored remainder |
| --- | ---: | ---: | ---: | ---: | ---: |
| Effects | 1,520 | 1,553 | 967 | 79 | 474 |
| Triggers | 1,152 | 1,175 | 939 | 90 | 123 |
| Defines | 1,329 | 1,348 | 1,286 | 7 | 36 |
| On_actions | 342 | 916 | 269 | 38 | 35 |
| Game rules | 195 | 382 | 152 | 13 | 30 |
| Type schemas | 1,054 | 1,703 | 401 | 68 | 585 |
| Other | 192 | 205 | 0 | 0 | 192 |
| **Total** | **5,784** | **7,282** | **4,014** | **295** | **1,475** |

All type-schema documentation was measured, not sampled. `type_schemas` includes documentation in
all `common/` CWT files except defines; the separate root on_actions/game_rules inventories have
separate rows. Repeated declarations and nested field prose are included, so these entry counts are
not the earlier roadmap's count of documented command names. The earlier “about 150 authored”
estimate is superseded by this bounded full measurement, not confirmed by it.

There are eight existing config diagnostics and 36 source parse diagnostics. The latter include
incomplete example fragments and three active-script repairs/failures; the exact list is pinned in
[the acceptance fixture](../../tests/fixtures/documentation-baseline.json). These limitations must
travel with the result. Text attribution moves the current headline denominator to 55,999 claims;
the retained registry snapshot still supports **0 / 55,999**, because it supplies no rule answers.

## Verification

The measurement's tests exist only at revision `1521c2a`. Its game-free `provenance` suite tests
source associations, quoted hashes, namespace/family separation, repeated locations,
exact/rewritten/authored separation, source identities, relocation, failure reporting, and zero
evidence credit. Its full gate pins all source hashes through the manifest and all three complete
output digests. It also verifies exact parser text and source locations for a define, an
on_action, a game rule, and a type-schema field. At that revision, run:

```sh
cargo test --locked --test provenance
cargo test --locked --test full_provenance -- --ignored
```

The full gate requires the four environment variables above. It fails if an input is absent or
changes. Raw installed content and complete local reports stay outside Git; the tracked fixture
retains reproducible identities and results.
