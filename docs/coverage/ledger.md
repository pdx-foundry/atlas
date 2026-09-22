# Config claim ledger contract

SDK-523 produces an inventory of source questions. A source assertion says what CWT claims; it is
not proof of game behavior. Atlas coverage asks whether the snapshot has established an answer to
the corresponding question, within its stated context. Agreement is not part of the calculation.

## Source and identity

`ledger::inventory` accepts a sorted map of relative filenames to decoded source text. The CLI reads
every `.cwt` below the supplied config root. It rejects empty inputs, unreadable/non-UTF-8 files,
and symlinks rather than silently omitting them. UTF-8 byte hashes identify the actual inputs.

Each claim records a subject path, property, structural conditions, the CWT assertion, exact source
text/span, owner and rationale, and an expected roadmap method for engine-fact questions. Source
spans are half-open UTF-8 byte offsets with one-based physical line numbers. Type and field presence
are separate from value forms, occurrence bounds, scopes, and documentation. Adjacent documentation
lines attached to one entry form one documentation claim. Unknown annotations retain their text
and a diagnostic; expected method assignment does not prove that the method already works.
Minimum and maximum cardinality are separate questions. Each soft bound belongs to consumer policy;
the other bound can still be an engine fact. Define-derived bounds retain a separate reference question.
The block around `type[...]` is CWT metadata, not a claim that game definitions must be blocks.
Value-form questions come from the separate schema entries. Subtype selectors inside type metadata
express field predicates; subtype arms inside a schema retain the base field, alias, or content
questions and their ownership, with the subtype carried in their conditions.

The question ID hashes relative file, subject path, property, and structural conditions. Occurrence
IDs append an ordinal to distinguish repeated declarations of the same question. CWT answer text
and line positions do not affect those IDs. Bare values use their position among bare siblings as
the subject segment. Annotation subjects include the annotation name, so distinct scope operations
cannot share credit merely because both concern scope context. Moving a file, renaming a subject,
or reordering bare items can change identity;
this first format does not claim identity across those changes.

The denominator counts source claim occurrences, not unique engine facts. One qualified question
answer can cover repeated source occurrences. Alias expansion does not duplicate the shared grammar,
and source-line links and closing braces do not create extra claims. CWT grouping, subtype naming,
and alias factoring are explicit consumer-policy claims. A complete negative answer can cover a
presence question without establishing any of its value/cardinality questions. Atlas-only questions
are listed separately and do not change the config denominator.

`files[].lines` accounts for every meaningful physical line with claim IDs, diagnostic IDs, or both.
Blank lines, ordinary single-hash comments, and decorative comments with more than four hashes are
excluded; quoted multiline text remains meaningful even when a line starts with `#`. Semantic `##`
annotations and `###`/`####` documentation are included. Delimiters link to the enclosing claim.
Malformed syntax and unresolved annotations set `inventory_complete=false` and produce exit 2 after
writing both reports. Claims under known structural subtype contexts carry those context labels;
this does not qualify the underlying game condition.

## Ownership and routes

| Class | Examples | Authority |
| --- | --- | --- |
| Engine fact | Fields, accepted forms, hard cardinality, references, scopes | Qualified engine evidence |
| Content-derived | Dynamic content queries and shipped-comment documentation | Content observations |
| Consumer policy | Severity, soft bounds, subtype naming, aliases, scope groups | Consumer modeling choices |
| Authored text | Unattributed documentation outside expected engine/content families | Its author |

Documentation assignments are provisional; the historical [provenance measurement](documentation.md)
compared every entry against explicit engine dumps and installed comments. It tagged all
entries, recorded matching source locations, separated rewritten candidates, and listed the authored
remainder. Current ledger output keeps the expected-owner assignments provisional.
Neither expected ownership nor a text match supplies qualified rule coverage.
Unknown metadata has a provisional policy assignment plus a diagnostic; it is not declared
inherently manual.

Classification and expected method families are centralized in `src/ledger/classify.rs`.
Registry discovery, field discovery, reader binding, language declarations, shapes,
nested grammar, references, numerics, weights, names, loader selection, and separate
formats each have a route. A route is not a qualification or a game-rule answer.

## Snapshot input

The production `pdx-atlas snapshot` command writes `atlas_rule_snapshot` contract version 1.
The ledger accepts that file directly and projects its registry and field rules onto matching
config questions. The join comes from the ledger's own `loader_path` claims; the rule producer
does not read CWT. Each rule keeps Native's source stamp, completeness and typed gaps. Directories
with more than one config type remain unmapped: a registry fact cannot establish which type it
describes. The comparison reports those type claims without an Atlas answer. Recorded sources
are unqualified for current-engine coverage. A matching gap blocks credit for its
question. Unknown contract versions or broken subject, source or schema references are errors.
The full format is [rule-snapshot-v1.schema.json](../contract/rule-snapshot-v1.schema.json).
Snapshot assembly refuses a failed registry-discovery answer. It runs authored fixture recipes
only for registries in the discovery answer, so an unrelated static listing does not require
tradition fixture recordings.

The ledger accepts only the published `atlas_rule_snapshot` input. Its private scoring
projection joins rule subjects to ledger questions and preserves exact Native source stamps,
conditions and gaps. There is no second published coverage snapshot contract and no replay-era
registry observation input.

Credit requires a matching question and conditions, a supported answer, and applicable qualified
engine evidence. Recorded sources cannot give current-engine credit. Another target, narrower
conditions, an applicable gap, or incompatible qualified answers prevents full credit. Assessments
retain incomplete states in `answer_states`, including when none earns credit. Gap-only questions
absent from the ledger appear in `atlas_only_questions`. Conflicts are between applicable Atlas
answers, not between Atlas and CWT. Source answer equality is never used. Conditions match exactly;
there is no inferred implication or nearest-target fallback.

## Determinism and checks

File paths, source hashes, claims, diagnostics, and report breakdowns have deterministic ordering.
Coverage uses integer counts; percentages are rounded to six decimal places. Identical input bytes
produce identical report bytes, independent of filesystem enumeration and checkout location.

The game-free tests cover line accounting, ownership/method assignment, stable IDs across answer and
whitespace edits, exact and contradictory answers, unsupported/synthetic/partial evidence, target
and condition mismatch, conflict/gap handling, duplicate evidence, denominator reconciliation,
unknown formats, and relocated CLI inputs. The full-config gate separately pins both input hashes
and both output digests. Baseline changes require inspecting the changed findings before updating
the fixture; a changed digest is not a reason to rebaseline automatically.

Direct snapshot-to-claim comparison is implemented by SDK-524 as a separate test report.
Documentation source matching is implemented by SDK-525. Neither report promotes Native evidence
or publishes a rule snapshot.
